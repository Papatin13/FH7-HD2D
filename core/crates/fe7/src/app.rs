//! Desktop frontend: window, keyboard, mouse, audio, battery save, save states.
use crate::render;
use crate::mouse::{Mouse, MouseInput};
use crate::overlay::{self, Action, Menu, MenuInput};
use crate::settings::{Settings, FF_SPEEDS};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gba::Gba;
use crate::win::{KeyRepeat, MouseButton, Win as Window};
use minifb::Key;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const FRAME: Duration = Duration::from_nanos(16_742_706); // 59.7275 Hz

struct Audio {
    _stream: cpal::Stream,
    queue: Arc<Mutex<VecDeque<f32>>>,
    rate: u32,
    pos: f64,
    last: (f32, f32),
}

fn start_audio() -> Option<Audio> {
    let host = cpal::default_host();
    let dev = host.default_output_device()?;
    let cfg = dev.default_output_config().ok()?;
    let rate = cfg.sample_rate().0;
    let channels = cfg.channels() as usize;
    let queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::with_capacity(rate as usize)));
    let q = queue.clone();
    let err = |e| eprintln!("audio error: {e}");
    let sc: cpal::StreamConfig = cfg.clone().into();
    let fill = move |out: &mut dyn FnMut(usize, f32), frames: usize| {
        let mut q = q.lock().unwrap();
        for f in 0..frames {
            let (l, r) = if q.len() >= 2 { (q.pop_front().unwrap(), q.pop_front().unwrap()) } else { (0.0, 0.0) };
            for c in 0..channels {
                out(f * channels + c, if c % 2 == 0 { l } else { r });
            }
        }
    };
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => dev.build_output_stream(
            &sc,
            move |d: &mut [f32], _| {
                let n = d.len() / channels;
                fill(&mut |i, v| d[i] = v, n)
            },
            err,
            None,
        ),
        cpal::SampleFormat::I16 => dev.build_output_stream(
            &sc,
            move |d: &mut [i16], _| {
                let n = d.len() / channels;
                fill(&mut |i, v| d[i] = (v * 32767.0) as i16, n)
            },
            err,
            None,
        ),
        cpal::SampleFormat::U16 => dev.build_output_stream(
            &sc,
            move |d: &mut [u16], _| {
                let n = d.len() / channels;
                fill(&mut |i, v| d[i] = ((v * 32767.0) as i32 + 32768) as u16, n)
            },
            err,
            None,
        ),
        _ => return None,
    }
    .ok()?;
    stream.play().ok()?;
    Some(Audio { _stream: stream, queue, rate, pos: 0.0, last: (0.0, 0.0) })
}

impl Audio {
    /// Resamples 32768 Hz stereo to the device rate and queues it.
    fn push(&mut self, src: &[f32]) {
        let step = gba::apu::SAMPLE_RATE as f64 / self.rate as f64;
        let n = src.len() / 2;
        let mut out = Vec::with_capacity((n as f64 / step) as usize * 2 + 4);
        while self.pos < n as f64 {
            let i = self.pos as usize;
            let fr = (self.pos - i as f64) as f32;
            let (l0, r0) = if i == 0 { self.last } else { (src[(i - 1) * 2], src[(i - 1) * 2 + 1]) };
            let (l1, r1) = (src[i * 2], src[i * 2 + 1]);
            out.push(l0 + (l1 - l0) * fr);
            out.push(r0 + (r1 - r0) * fr);
            self.pos += step;
        }
        self.pos -= n as f64;
        if n > 0 {
            self.last = (src[(n - 1) * 2], src[(n - 1) * 2 + 1]);
        }
        let mut q = self.queue.lock().unwrap();
        // keep latency bounded (~120 ms)
        let max = (self.rate as usize / 8) * 2;
        if q.len() > max {
            let drop = q.len() - max / 2;
            q.drain(..drop & !1);
        }
        q.extend(out);
    }
}

pub fn save_path(rom: &Path, ext: &str) -> PathBuf {
    rom.with_extension(ext)
}

fn open_window(scale: usize) -> Window {
    Window::new("Fire Emblem 7 — Rust PC-Port   [F1: Einstellungen]", 240 * scale, 160 * scale)
}

fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

pub fn run(mut g: Box<Gba>, rom_path: &Path, scale: Option<usize>) {
    let mut st = Settings::load();
    if let Some(sc) = scale {
        st.scale = sc.clamp(2, 8);
    }
    g.use_recomp = st.recomp;
    let sav = save_path(rom_path, "sav");
    if let Ok(d) = std::fs::read(&sav) {
        let n = d.len().min(g.sram.len());
        g.sram[..n].copy_from_slice(&d[..n]);
        eprintln!("loaded battery save {}", sav.display());
    }
    let mut window = open_window(st.scale);
    window.set_cursor_visibility(!st.hide_cursor);
    let mut audio = start_audio();
    if audio.is_none() {
        eprintln!("no audio device; running silent");
    }
    let mut renderer = render::Renderer::new();
    let mut mouse = Mouse::default();
    let mut menu = Menu::default();
    let mut cv = vec![0u32; overlay::CW * overlay::CH];
    let mut out: Vec<u32> = Vec::new();
    let mut prev_buttons = (false, false, false);
    let mut sram_timer = 0u32;
    let mut paused = false;
    let mut ff_on = false;
    let mut fps_t = Instant::now();
    let mut fps_n = 0;
    let mut fps_shown = 0;
    let state_path = |slot: u8| save_path(rom_path, &format!("ss{}", slot));
    menu.toast("F1: Einstellungen   F11: Vollbild");
    let mut fs_pending = st.fullscreen;
    let mut last_raw: Option<(f32, f32)> = None;
    let mut game_pos: Option<(i32, i32)> = None;
    let prof = std::env::var("FE7_PROF").is_ok();
    let mut prof_acc = (0.0f64, 0.0f64, 0u32);
    let mut prof_t = Instant::now();
    let mut prof_f = 0u64;

    let mut acc = Duration::ZERO;
    let mut last_t = Instant::now();

    'main: while window.is_open() {
        window.pump();
        let pressed = window.get_keys_pressed(KeyRepeat::Yes);
        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let b = (
            window.get_mouse_down(MouseButton::Left),
            window.get_mouse_down(MouseButton::Right),
            window.get_mouse_down(MouseButton::Middle),
        );
        let wheel = window.get_scroll_wheel().map(|(_, y)| y).unwrap_or(0.0);
        let wheel = if wheel > 0.0 {
            1
        } else if wheel < 0.0 {
            -1
        } else {
            0
        };
        let raw = window.mouse_pos();
        let raw_moved = raw != last_raw;
        last_raw = raw;
        let (cur_w, cur_h) = window.get_size();
        let cpos = raw.and_then(|p| render::map_point(render::overlay_rect(cur_w, cur_h), p, overlay::CW as i32, overlay::CH as i32));
        let fw = g.ppu.width;
        let e = g.ppu.ext_left as i32;
        let game_pt = raw
            .and_then(|p| render::map_point(render::game_rect(cur_w, cur_h, &st, fw), p, fw as i32, 160))
            .map(|(x, y)| (x - e, y))
            .map(|p| crate::mouse::unshift(&g, p));
        g.wide_ext = st.wide_ext(cur_w, cur_h);
        g.wide_cols = match st.widescreen {
            0 => 0,
            3 => 16, // 16:10
            4 => 24, // 21:9
            _ => 18, // Auto, 16:9
        };
        let (left, right, middle) = (b.0 && !prev_buttons.0, b.1 && !prev_buttons.1, b.2 && !prev_buttons.2);
        prev_buttons = b;

        let mut act = Action::None;
        if menu.open {
            let fresh: Vec<Key> = window.get_keys_pressed(KeyRepeat::No);
            let mut keys = pressed.clone();
            // F1/Esc only on a fresh press
            keys.retain(|k| !matches!(k, Key::F1 | Key::Escape) || fresh.contains(k));
            act = menu.input(&mut st, &MenuInput { keys, shift, pos: cpos, left, right, wheel, moved: raw_moved }, g.use_recomp);
            if g.use_recomp != st.recomp {
                g.use_recomp = st.recomp;
            }
        } else {
            for k in window.get_keys_pressed(KeyRepeat::No) {
                match k {
                    Key::F1 | Key::Escape => menu.open = true,
                    Key::P => {
                        paused = !paused;
                        menu.toast(if paused { "Pause" } else { "Weiter" });
                    }
                    Key::F5 => act = Action::SaveState,
                    Key::F8 => act = Action::LoadState,
                    Key::F12 => act = Action::Screenshot,
                    Key::F11 => {
                        st.fullscreen = !st.fullscreen;
                        act = Action::Fullscreen;
                    }
                    Key::Enter if window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt) => {
                        st.fullscreen = !st.fullscreen;
                        act = Action::Fullscreen;
                    }
                    Key::Space | Key::Tab if st.ff_toggle => ff_on = !ff_on,
                    _ => {}
                }
            }
        }

        match act {
            Action::None => {}
            Action::Resize => {
                if st.fullscreen {
                    menu.toast("Fenstergröße gilt außerhalb des Vollbilds");
                } else {
                    window.set_size(240 * st.scale, 160 * st.scale);
                }
            }
            Action::Fullscreen => {
                window.set_fullscreen(st.fullscreen);
                st.save();
            }
            Action::Cursor => window.set_cursor_visibility(!st.hide_cursor),
            Action::SaveState => match std::fs::write(state_path(st.slot), g.save_state()) {
                Ok(_) => menu.toast(format!("Savestate {} gespeichert", st.slot)),
                Err(e) => menu.toast(format!("Fehler: {e}")),
            },
            Action::LoadState => match std::fs::read(state_path(st.slot)) {
                Ok(d) if g.load_state(&d) => menu.toast(format!("Savestate {} geladen", st.slot)),
                _ => menu.toast(format!("Slot {} ist leer", st.slot)),
            },
            Action::Screenshot => {
                let dir = exe_dir().join("screenshots");
                let _ = std::fs::create_dir_all(&dir);
                let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
                let p = dir.join(format!("fe7_{}.png", t));
                crate::headless::save_png(&g, &p);
                menu.toast(format!("Screenshot: screenshots/{}", p.file_name().unwrap().to_string_lossy()));
            }
            Action::Reset => {
                let sram = g.sram.clone();
                let rom = std::mem::take(&mut g.rom);
                g = Gba::new(rom);
                g.lookup = Some(fe7gen::lookup);
                g.use_recomp = st.recomp;
                g.sram = sram;
                menu.open = false;
                menu.toast("Spiel zurückgesetzt");
            }
            Action::Quit => break 'main,
        }

        let fast = if st.ff_toggle { ff_on } else { window.is_key_down(Key::Space) || window.is_key_down(Key::Tab) };
        let unfocused = st.pause_unfocused && !window.is_active();
        if raw_moved || left || right {
            game_pos = game_pt;
        }
        let gpos = game_pos;
        // Game time advances with real time: if presenting a frame takes longer
        // than one GBA frame, more frames are emulated per presented frame.
        let now = Instant::now();
        acc += now - last_t;
        last_t = now;
        if acc > FRAME * 6 {
            acc = FRAME * 6;
        }
        let due = (acc.as_nanos() / FRAME.as_nanos()) as u32;
        acc -= FRAME * due;
        if !paused && !menu.open && !unfocused && due > 0 {
            let t0 = Instant::now();
            let speed = if fast { FF_SPEEDS[st.ff_speed] } else { 1 };
            let total = if speed == 0 { u32::MAX } else { due * speed };
            let mut i = 0;
            loop {
                let inp = if i == 0 {
                    MouseInput { pos: gpos, left, right, middle, wheel }
                } else {
                    MouseInput { pos: gpos, ..Default::default() }
                };
                let mk = if st.mouse { mouse.update(&mut g, &inp) } else { 0 };
                g.keys = st.keyboard(&window) | mk;
                if window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt) {
                    g.keys &= !gba::keys::START; // Alt+Enter toggles fullscreen
                }
                g.run_frame();
                i += 1;
                if i >= total || (speed == 0 && t0.elapsed() > Duration::from_millis(14)) {
                    break;
                }
            }
            let mut samples = std::mem::take(&mut g.apu.out);
            if let Some(a) = audio.as_mut() {
                if !fast || st.ff_sound {
                    let v = st.volume as f32 / 10.0;
                    samples.iter_mut().for_each(|x| *x *= v);
                    a.push(&samples);
                }
            }
        } else if paused || menu.open || unfocused {
            g.apu.out.clear();
        }

        cv.fill(0);
        if st.show_fps && !menu.open {
            overlay::text_shadow(&mut cv, 6, 6, &format!("{} FPS{}", fps_shown, if fast { " >>" } else { "" }), 0xFFFF80, 1);
        }
        if paused && !menu.open {
            overlay::text_shadow(&mut cv, 6, 306, "PAUSE (P)", 0xFFFFFF, 1);
        }
        menu.draw(&mut cv, &st, g.use_recomp);

        let (ww, wh) = window.get_size();
        let (ww, wh) = (ww.max(1), wh.max(1));
        if out.len() != ww * wh {
            out = vec![0; ww * wh];
        }
        let tr = Instant::now();
        renderer.render(&g.ppu.frame, g.ppu.width, g.ppu.ext_left, &cv, &mut out, ww, wh, &st);
        let t_render = tr.elapsed();
        if fs_pending {
            fs_pending = false;
            window.set_fullscreen(st.fullscreen);
        }
        let tu = Instant::now();
        window.update_with_buffer(&out, ww, wh);
        if prof {
            prof_acc.0 += t_render.as_secs_f64();
            prof_acc.1 += tu.elapsed().as_secs_f64();
            prof_acc.2 += 1;
            if prof_acc.2 == 60 {
                let dt = prof_t.elapsed().as_secs_f64();
                eprintln!(
                    "render {:.2} ms  present {:.2} ms  ({}x{})  loop {:.1}/s  game {:.1} frames/s",
                    prof_acc.0 * 1000.0 / 60.0,
                    prof_acc.1 * 1000.0 / 60.0,
                    ww,
                    wh,
                    60.0 / dt,
                    (g.frame - prof_f) as f64 / dt
                );
                prof_acc = (0.0, 0.0, 0);
                prof_t = Instant::now();
                prof_f = g.frame;
            }
        }

        if g.sram_dirty {
            sram_timer += 1;
            if sram_timer > 30 {
                g.sram_dirty = false;
                sram_timer = 0;
                if let Err(e) = std::fs::write(&sav, &g.sram[..0x8000]) {
                    eprintln!("cannot write save: {e}");
                }
            }
        }

        fps_n += 1;
        if fps_t.elapsed() >= Duration::from_secs(1) {
            fps_shown = fps_n;
            fps_n = 0;
            fps_t = Instant::now();
        }

        let spent = acc + last_t.elapsed();
        if spent < FRAME {
            std::thread::sleep(FRAME - spent);
        }
    }
    st.save();
    let _ = std::fs::write(&sav, &g.sram[..0x8000]);
}
