mod app;
mod headless;
mod mouse;
mod overlay;
mod render;
mod settings;
mod win;

use gba::Gba;
use std::path::PathBuf;

struct Args {
    rom: PathBuf,
    frames: u64,
    shot_every: u64,
    out: PathBuf,
    interp: bool,
    script: String,
    trace: Option<PathBuf>,
    headless: bool,
    hash_every: u64,
    verify: bool,
    scale: usize,
    mouse: String,
    load: Option<PathBuf>,
    save: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut a = Args {
        rom: find_rom(),
        frames: 600,
        shot_every: 0,
        out: PathBuf::from("shots"),
        interp: false,
        script: String::new(),
        trace: None,
        headless: false,
        hash_every: 0,
        verify: false,
        scale: 0,
        mouse: String::new(),
        load: None,
        save: None,
    };
    let v: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < v.len() {
        match v[i].as_str() {
            "--rom" => {
                a.rom = PathBuf::from(&v[i + 1]);
                i += 1;
            }
            "--frames" => {
                a.frames = v[i + 1].parse().unwrap();
                i += 1;
            }
            "--shot-every" => {
                a.shot_every = v[i + 1].parse().unwrap();
                i += 1;
            }
            "--out" => {
                a.out = PathBuf::from(&v[i + 1]);
                i += 1;
            }
            "--script" => {
                a.script = v[i + 1].clone();
                i += 1;
            }
            "--trace" => {
                a.trace = Some(PathBuf::from(&v[i + 1]));
                i += 1;
            }
            "--hash-every" => {
                a.hash_every = v[i + 1].parse().unwrap();
                i += 1;
            }
            "--load" => {
                a.load = Some(PathBuf::from(&v[i + 1]));
                i += 1;
            }
            "--save" => {
                a.save = Some(PathBuf::from(&v[i + 1]));
                i += 1;
            }
            "--mouse" => {
                a.mouse = v[i + 1].clone();
                i += 1;
            }
            "--scale" => {
                a.scale = v[i + 1].parse().unwrap();
                i += 1;
            }
            "--interp" => a.interp = true,
            "--verify" => a.verify = true,
            "--headless" => a.headless = true,
            other => {
                if !other.starts_with("--") {
                    a.rom = PathBuf::from(other);
                }
            }
        }
        i += 1;
    }
    a
}

/// Looks for the Fire Emblem (USA) ROM next to the executable or in the
/// current directory (game code AE7E).
fn find_rom() -> PathBuf {
    let mut dirs = vec![PathBuf::from(".")];
    if let Ok(e) = std::env::current_exe() {
        if let Some(d) = e.parent() {
            dirs.push(d.to_path_buf());
            if let Some(p) = d.parent() {
                dirs.push(p.to_path_buf());
            }
        }
    }
    for d in dirs {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x.eq_ignore_ascii_case("gba")).unwrap_or(false) {
                    if let Ok(mut f) = std::fs::File::open(&p) {
                        use std::io::Read;
                        let mut h = [0u8; 0xB0];
                        if f.read_exact(&mut h).is_ok() && &h[0xAC..0xB0] == b"AE7E" {
                            return p;
                        }
                    }
                }
            }
        }
    }
    PathBuf::from("rom.gba")
}

fn state_hash(g: &Gba) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for chunk in [&g.ewram[..], &g.iwram[..], &g.vram[..], &g.pal[..], &g.oam[..]] {
        for &b in chunk {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

fn main() {
    if let Some(i) = std::env::args().position(|x| x == "--bench-render") {
        let res: Vec<usize> = std::env::args().nth(i + 1).unwrap().split('x').map(|v| v.parse().unwrap()).collect();
        let st = settings::Settings::load();
        let mut r = render::Renderer::new();
        let frame: Vec<u32> = (0..240 * 160u32).map(|i| i.wrapping_mul(2654435761) >> 8).collect();
        let ov = vec![0u32; overlay::CW * overlay::CH];
        let mut out = vec![0u32; res[0] * res[1]];
        let t = std::time::Instant::now();
        for _ in 0..60 {
            r.render(&frame, 240, 0, &ov, &mut out, res[0], res[1], &st);
        }
        println!("{}x{}: {:.2} ms/frame", res[0], res[1], t.elapsed().as_secs_f64() * 1000.0 / 60.0);
        return;
    }
    let a = parse_args();
    let rom = std::fs::read(&a.rom).expect("cannot read ROM");
    let mut g = Gba::new(rom);
    g.lookup = Some(fe7gen::lookup);
    g.use_recomp = !a.interp;
    if a.verify {
        g.verify = Some(fe7gen::lookup_verify);
    }
    if let Ok(w) = std::env::var("FE7_WIDE") {
        g.wide_ext = w.parse().unwrap_or(0);
    }
    if let Ok(w) = std::env::var("FE7_WATCHW") {
        g.watch_w = u32::from_str_radix(w.trim_start_matches("0x"), 16).unwrap();
    }
    if let Ok(w) = std::env::var("FE7_WATCH") {
        g.watch = u32::from_str_radix(w.trim_start_matches("0x"), 16).unwrap();
    }
    g.hd.on = std::env::var("FE7_HD").is_ok();
    g.hd.cap_layers = std::env::var("FE7_LAYERS").is_ok();
    g.hd.text_dbg = std::env::var("FE7_TEXTDBG").is_ok();
    g.hd.talk_native = g.hd.on;
    g.hd.menu_native = g.hd.on;
    if a.trace.is_some() {
        g.trace = Some(Default::default());
    }
    if !a.headless {
        if let Some(p) = &a.load {
            assert!(g.load_state(&std::fs::read(p).unwrap()), "bad state");
        }
        app::run(g, &a.rom, if a.scale > 0 { Some(a.scale) } else { None });
        return;
    }
    if let Some(p) = &a.load {
        assert!(g.load_state(&std::fs::read(p).unwrap()), "bad state");
    }
    if let Ok(p) = std::env::var("FE7_SRAM_IN") {
        let d = std::fs::read(p).unwrap();
        g.sram[..d.len()].copy_from_slice(&d);
    }
    std::fs::create_dir_all(&a.out).ok();
    let script = headless::parse_script(&a.script);
    let t0 = std::time::Instant::now();
    let mut ms = mouse::Mouse::default();
    // mouse script: frame:x,y[:L|R|M]
    let mev: Vec<(u64, i32, i32, String)> = a
        .mouse
        .split(';')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let f: Vec<&str> = p.split(':').collect();
            let xy: Vec<i32> = f[1].split(',').map(|v| v.parse().unwrap()).collect();
            (f[0].parse().unwrap(), xy[0], xy[1], f.get(2).unwrap_or(&"").to_string())
        })
        .collect();
    let mut mpos = None;
    let mut audio_rms: Vec<f64> = Vec::new();
    let mut audit_hits = 0u64;
    let mut audit_prev = false;
    let audit_all = std::env::var("FE7_AUDIT_ALL").is_ok();
    let mut last_talk = String::new();
    for f in 0..a.frames {
        let mut inp = mouse::MouseInput::default();
        for e in mev.iter().filter(|e| e.0 == f) {
            mpos = Some((e.1, e.2));
            inp.left |= e.3 == "L";
            inp.right |= e.3 == "R";
            inp.middle |= e.3 == "M";
        }
        inp.pos = mpos.map(|p| mouse::unshift(&g, p));
        let mk = if mev.is_empty() { 0 } else { ms.update(&mut g, &inp) };
        if !mev.is_empty() && (inp.left || inp.right) {
            eprintln!("frame {} mouse ctx {}", f, ms.mode);
        }
        g.keys = headless::keys_at(&script, f) | mk;
        if let Ok(pks) = std::env::var("FE7_POKE") {
            // addr=val@frames[;addr=val@frames...]
            for pk in pks.split(';') {
                let (av, until) = pk.split_once('@').unwrap_or((pk, "100000"));
                let (ad, vl) = av.split_once('=').unwrap();
                if f < until.parse::<u64>().unwrap() {
                    g.write8(u32::from_str_radix(ad, 16).unwrap(), u8::from_str_radix(vl, 16).unwrap());
                }
            }
        }
        if let Ok(c) = std::env::var("FE7_CALL") {
            // test hook: frame:func:r0:r1:r2 (hex) - calls a game function before the frame
            let v: Vec<u32> = c.split(':').map(|x| u32::from_str_radix(x.trim_start_matches("0x"), 16).unwrap()).collect();
            if f == v[0] as u64 {
                let (cpu, pc) = (g.cpu.clone(), g.pc);
                g.cpu.r[0] = v[2];
                g.cpu.r[1] = v[3];
                g.cpu.r[2] = v[4];
                const MAGIC: u32 = 0x0800_00C0;
                g.cpu.r[14] = MAGIC | 1;
                g.cpu.t = true;
                g.pc = v[1] & !1;
                while g.pc != MAGIC {
                    g.step_instr();
                }
                g.cpu = cpu;
                g.pc = pc;
            }
        }
        g.run_frame();
        if let Ok(dir) = std::env::var("FE7_AUDIT") {
            // GBA-style window frames (BG0/BG1 tiles 0x001..0x009, palette 1) that the
            // HD frontend neither hides nor covers with one of its full-screen views
            let covered = [0x08CC_1F0Cu32, 0x08CC_3194, 0x08CE_5BF0, 0x08CC_3BDC, 0x08CC_4854, 0x08CC_4448,
                0x08B9_42F8, 0x08CE_6FC0, 0x08CE_3C54, 0x08CE_3F24]
                .iter().any(|&p| g.proc_find(p).is_some());
            let dispcnt = g.io16(0);
            let mut bad = 0;
            if !covered && g.hd.fade.abs() < 12 {
                for bg in 0..2usize {
                    if dispcnt & (0x100 << bg) == 0 { continue; }
                    let cnt = g.io16(8 + bg * 2) as usize;
                    let sb = ((cnt >> 8) & 31) * 0x800;
                    let ho = (g.io16(0x10 + bg * 4) & 0x1FF) as usize / 8;
                    let vo = (g.io16(0x12 + bg * 4) & 0x1FF) as usize / 8;
                    for cy in 0..20usize {
                        for cx in 0..30usize {
                            let a = sb + ((((cy + vo) % 32) * 32 + (cx + ho) % 32) * 2);
                            let e = g.vram[a] as u32 | (g.vram[a + 1] as u32) << 8;
                            let any_ui = audit_all && g.hd.map && e & 0x3FF != 0;
                            if (any_ui || e >> 12 == 1 && (1..=9).contains(&(e & 0x3FF))) && !g.hd.talk_mask[cy * 30 + cx] {
                                bad += 1;
                            }
                        }
                    }
                }
            }
            if std::env::var("FE7_AUDITDBG").is_ok() { eprintln!("audit f{} covered {} bad {} fade {}", f, covered, bad, g.hd.fade); }
            // the audit reads VRAM after the frame (the next frame's maps are already
            // in it), the masks are from the frame's start: count what stays 2 frames
            let hit = bad >= 6 && audit_prev;
            audit_prev = bad >= 6;
            if hit {
                audit_hits += 1;
                if audit_hits % 30 == 1 {
                    let path = format!("{}/hit{:06}.png", dir, f);
                    headless::save_png(&g, std::path::Path::new(&path));
                    let mut names = Vec::new();
                    for i in 0..64u32 {
                        let p = 0x0202_4E28 + i * 0x6C;
                        let sc = g.read32(p);
                        if sc >> 24 == 8 { names.push(format!("{:08x}", sc)); }
                    }
                    eprintln!("AUDIT f{} cells {} procs {}", f, bad, names.join(","));
                }
            }
        }
        if std::env::var("FE7_TALKLOG").is_ok() {
            let cur = format!("{:?}", g.hd.talk.as_ref().map(|t| (&t.lines, t.waiting)));
            if cur != last_talk {
                let ptr = g.read32(0x0300_0040);
                eprintln!("f{} {} ptr {:08x}", f, cur, ptr);
                last_talk = cur;
            }
        }
        if a.hash_every > 0 {
            let n = g.apu.out.len().max(1) as f64;
            let rms = (g.apu.out.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / n).sqrt();
            audio_rms.push(rms);
        }
        if let Ok(w) = std::env::var("FE7_WAV") {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(w).unwrap();
            for v in &g.apu.out {
                f.write_all(&((v * 32767.0) as i16).to_le_bytes()).unwrap();
            }
        }
        g.apu.out.clear();
        if a.shot_every > 0 && (f + 1) % a.shot_every == 0 {
            headless::save_png(&g, &a.out.join(format!("f{:06}.png", f + 1)));
        }
        if a.hash_every > 0 && (f + 1) % a.hash_every == 0 {
            let k = a.hash_every as usize;
            let r = audio_rms[audio_rms.len().saturating_sub(k)..].iter().sum::<f64>() / k as f64;
            println!("frame {} hash {:016x} pc {:08x} audio_rms {:.4}", f + 1, state_hash(&g), g.pc, r);
        }
    }
    if let Ok(d) = std::env::var("FE7_DUMP") {
        // addr:len[;addr:len] hex dump at the end; '*addr' dereferences a pointer first
        for e in d.split(';') {
            let (a0, l) = e.split_once(':').unwrap();
            let mut ad = u32::from_str_radix(a0.trim_start_matches('*').trim_start_matches("0x"), 16).unwrap();
            if a0.starts_with('*') {
                ad = g.read32(ad);
            }
            let l = usize::from_str_radix(l, 16).unwrap();
            for r in 0..l.div_ceil(16) {
                let b: Vec<String> = (0..16.min(l - r * 16)).map(|i| format!("{:02x}", g.read8(ad + (r * 16 + i) as u32))).collect();
                println!("{:08x}: {}", ad + r as u32 * 16, b.join(" "));
            }
        }
    }
    let dt = t0.elapsed();
    for (pc, n) in &g.watch_hits {
        println!("watch hit pc {:08X} x{}", pc, n);
    }
    if let Ok(p) = std::env::var("FE7_SRAM_OUT") {
        std::fs::write(p, &g.sram[..0x8000]).unwrap();
    }
    if g.hd.on {
        let wpng = |path: &str, w: usize, h: usize, px: &[u32]| {
            let f = std::fs::File::create(path).unwrap();
            let mut e = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
            e.set_color(png::ColorType::Rgba);
            let mut wr = e.write_header().unwrap();
            let d: Vec<u8> = px.iter().flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8, (c >> 24) as u8]).collect();
            wr.write_image_data(&d).unwrap();
        };
        let lw = g.ppu.width;
        wpng(&format!("{}/hd_ui.png", a.out.display()), lw, 160, &g.ppu.frame[..lw * 160]);
        if g.hd.layers.len() == 4 * 38400 {
            for b in 0..4 {
                wpng(&format!("{}/hd_layer{}.png", a.out.display(), b), 240, 160, &g.hd.layers[b * 38400..(b + 1) * 38400]);
            }
        }
        if g.hd.atlas.len() == 256 * 256 {
            wpng(&format!("{}/hd_atlas.png", a.out.display()), 256, 256, &g.hd.atlas);
        }
        let (w, h) = g.hd_map_size();
        if w > 0 {
            let img = g.hd_map_image(4);
            wpng(&format!("{}/hd_map.png", a.out.display()), (w as usize + 8) * 16, (h as usize + 8) * 16, &img);
            let cut = g.hd_map_cutout(4, &[0x0C, 0x0D, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x23, 0x2C, 0x2E]);
            wpng(&format!("{}/hd_cut.png", a.out.display()), (w as usize + 8) * 16, (h as usize + 8) * 16, &cut);
        }
        eprintln!("talk {:?}", g.hd.talk);
        eprintln!("menus {:?}", g.hd.menus);
        eprintln!("generic {:?} texts {:?} rhelp {:?} fights {:?}", g.hd.generic, g.hd.generic_text, g.hd.rhelp, g.hd.mapinfo.fights);
        eprintln!("forecast {:?}", g.hd.forecast);
        eprintln!("hd map {} view {:?} cursor {:?} fade {} sprites {:?}", g.hd.map, g.hd.view, g.hd.cursor, g.hd.fade, g.hd.sprites);
        eprintln!("mapinfo {:?}", g.hd.mapinfo);
        eprintln!("typed {:?} cur {:?}", g.hd_typed_text(), g.hd_current_text());
    }
    if let Some(p) = &a.save {
        std::fs::write(p, g.save_state()).unwrap();
        headless::save_png(&g, &p.with_extension("png"));
    }
    if a.verify {
        eprintln!("verified {} block executions, {} mismatching blocks", g.verify_count, g.verify_fail.len());
    }
    eprintln!(
        "{} frames in {:.2?} ({:.1} fps); blocks recomp {} interp {} instrs interp {}",
        a.frames,
        dt,
        a.frames as f64 / dt.as_secs_f64(),
        g.stats.blocks_recomp,
        g.stats.blocks_interp,
        g.stats.instrs_interp
    );
    if let Some(p) = &a.trace {
        let dir = p.parent().unwrap_or(std::path::Path::new("."));
        std::fs::write(dir.join("iwram.bin"), &g.iwram).unwrap();
        std::fs::write(dir.join("ewram.bin"), &g.ewram).unwrap();
    }
    if let (Some(p), Some(t)) = (a.trace, g.trace.as_ref()) {
        let mut v: Vec<_> = t.iter().copied().collect();
        v.sort();
        let s: String = v.iter().map(|(a, t)| format!("{:08x} {}\n", a, *t as u8)).collect();
        std::fs::write(p, s).unwrap();
    }
}

// glibc compatibility: Rust's std references pidfd_spawnp/pidfd_getpid
// (glibc >= 2.39) weakly; they are only used for pidfd-based process
// spawning, which this program never requests. Defining them here keeps the
// binary loadable on older systems (glibc >= 2.34).
#[no_mangle]
pub extern "C" fn pidfd_spawnp(
    _pidfd: *mut i32,
    _file: *const u8,
    _fa: *const u8,
    _attr: *const u8,
    _argv: *const *const u8,
    _envp: *const *const u8,
) -> i32 {
    38 // ENOSYS
}

#[no_mangle]
pub extern "C" fn pidfd_getpid(_fd: i32) -> i32 {
    -1
}
