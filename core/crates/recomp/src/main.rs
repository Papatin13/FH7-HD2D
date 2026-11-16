//! Static recompiler: ROM machine code (Thumb + ARM) -> Rust source.
//!
//! Usage: recomp <rom.gba> <out_dir> [trace files...]
//!
//! 1. Discovery: recursive descent from the reset vector plus every block start
//!    observed in interpreter traces. Every branch target, call target and
//!    return address becomes a "leader" (block start).
//! 2. Each block (leader up to the next control-flow change or leader) is
//!    emitted as one Rust function that calls the shared instruction-semantics
//!    helpers of the `gba` crate with constant, pre-decoded operands.
//! 3. A lookup table maps (address, thumb) to the generated functions.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;

const ROM_BASE: u32 = 0x0800_0000;
const MAX_BLOCK: usize = 200;

/// Code memory view: the ROM plus snapshots of EWRAM/IWRAM (for code the game
/// copies into RAM at runtime).
struct Rom {
    regions: Vec<(u32, Vec<u8>)>,
}
impl Rom {
    fn region(&self, a: u32) -> Option<(&[u8], usize)> {
        for (base, d) in &self.regions {
            if a >= *base && ((a - base) as usize) + 4 <= d.len() {
                return Some((d, (a - base) as usize));
            }
        }
        None
    }
    fn in_rom(&self, a: u32) -> bool {
        self.region(a).is_some()
    }
    fn is_const(&self, a: u32) -> bool {
        (a >> 24 >= 8 || a < 0x4000) && self.in_rom(a)
    }
    fn r16(&self, a: u32) -> u16 {
        let (d, o) = self.region(a).unwrap();
        u16::from_le_bytes([d[o], d[o + 1]])
    }
    fn r32(&self, a: u32) -> u32 {
        let (d, o) = self.region(a).unwrap();
        u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
    }
}

fn sext(v: u32, bits: u32) -> i32 {
    ((v << (32 - bits)) as i32) >> (32 - bits)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Loc {
    addr: u32,
    thumb: bool,
}

/// Control-flow classification of one instruction.
enum Flow {
    /// Continues to the next instruction.
    Next,
    /// Ends the block. `succ`: statically known successors.
    End(Vec<Loc>),
}

fn thumb_flow(rom: &Rom, a: u32, op: u32) -> (Flow, usize) {
    // returns flow and the instruction length in halfwords (2 for fused BL)
    match op >> 11 {
        8 if op & 0x400 != 0 => {
            let h = (op >> 8) & 3;
            let rd = (op & 7) | ((op >> 4) & 8);
            let rs = (op >> 3) & 15;
            if h == 3 {
                if rs == 15 {
                    return (Flow::End(vec![Loc { addr: (a + 4) & !3, thumb: false }]), 1);
                }
                return (Flow::End(vec![]), 1);
            }
            if (h == 0 || h == 2) && rd == 15 {
                return (Flow::End(vec![]), 1);
            }
            (Flow::Next, 1)
        }
        22 | 23 => {
            if (op >> 8) & 0xF == 0xD {
                (Flow::End(vec![]), 1)
            } else {
                (Flow::Next, 1)
            }
        }
        26 | 27 => {
            let cond = (op >> 8) & 0xF;
            if cond == 0xF {
                (Flow::End(vec![Loc { addr: a + 2, thumb: true }]), 1)
            } else if cond == 0xE {
                (Flow::End(vec![]), 1)
            } else {
                let t = a.wrapping_add(4).wrapping_add((sext(op & 0xFF, 8) * 2) as u32);
                (Flow::End(vec![Loc { addr: t, thumb: true }, Loc { addr: a + 2, thumb: true }]), 1)
            }
        }
        28 => {
            let t = a.wrapping_add(4).wrapping_add((sext(op & 0x7FF, 11) * 2) as u32);
            (Flow::End(vec![Loc { addr: t, thumb: true }]), 1)
        }
        29 => (Flow::End(vec![]), 1),
        30 => {
            if rom.in_rom(a + 2) {
                let lo = rom.r16(a + 2) as u32;
                if lo >> 11 == 31 {
                    let hi = (sext(op & 0x7FF, 11) << 12) as u32;
                    let t = a.wrapping_add(4).wrapping_add(hi).wrapping_add((lo & 0x7FF) << 1);
                    return (Flow::End(vec![Loc { addr: t, thumb: true }, Loc { addr: a + 4, thumb: true }]), 2);
                }
            }
            (Flow::Next, 1)
        }
        31 => (Flow::End(vec![Loc { addr: a + 2, thumb: true }]), 1),
        _ => (Flow::Next, 1),
    }
}

fn arm_flow(a: u32, op: u32) -> Flow {
    let cond = op >> 28;
    if cond == 0xF {
        return Flow::End(vec![]);
    }
    let next = Loc { addr: a + 4, thumb: false };
    let cond_succ = |mut v: Vec<Loc>| {
        if cond != 0xE {
            v.push(next);
        }
        Flow::End(v)
    };
    if op & 0x0FFF_FFF0 == 0x012F_FF10 {
        return cond_succ(vec![]);
    }
    match (op >> 25) & 7 {
        5 => {
            let off = (((op & 0x00FF_FFFF) << 8) as i32 >> 6) as u32;
            let t = a.wrapping_add(8).wrapping_add(off);
            let mut v = vec![Loc { addr: t, thumb: false }];
            if op & (1 << 24) != 0 {
                v.push(next);
            }
            cond_succ(v)
        }
        7 if op & (1 << 24) != 0 => Flow::End(vec![next]),
        4 => {
            if op & (1 << 20) != 0 && op & 0x8000 != 0 {
                cond_succ(vec![])
            } else {
                Flow::Next
            }
        }
        2 | 3 => {
            if op & (1 << 20) != 0 && (op >> 12) & 15 == 15 {
                cond_succ(vec![])
            } else {
                Flow::Next
            }
        }
        0 | 1 => {
            // data processing / msr / mul etc writing r15
            let rd = (op >> 12) & 15;
            let is_dp = !(op & 0x0E00_0090 == 0x0000_0090 && op & 0x0200_0000 == 0);
            let opc = (op >> 21) & 15;
            let is_test = (8..=11).contains(&opc);
            // MSR to CPSR can change mode/thumb
            if op & 0x0DB0_F000 == 0x0120_F000 {
                return Flow::End(vec![next]);
            }
            if is_dp && rd == 15 && !is_test {
                cond_succ(vec![])
            } else if op & 0x0E00_0090 == 0x0000_0090 && op & 0x60 != 0 && op & (1 << 20) != 0 && rd == 15 {
                cond_succ(vec![])
            } else {
                Flow::Next
            }
        }
        _ => Flow::End(vec![]),
    }
}

struct Disc {
    leaders: BTreeSet<Loc>,
    insns: BTreeSet<Loc>,
}

fn discover(rom: &Rom, seeds: &[Loc]) -> Disc {
    let mut d = Disc { leaders: BTreeSet::new(), insns: BTreeSet::new() };
    let mut work: VecDeque<Loc> = seeds.iter().copied().collect();
    while let Some(l) = work.pop_front() {
        let l = Loc { addr: if l.thumb { l.addr & !1 } else { l.addr & !3 }, thumb: l.thumb };
        if !rom.in_rom(l.addr) {
            continue;
        }
        d.leaders.insert(l);
        let mut a = l.addr;
        loop {
            if !rom.in_rom(a) {
                break;
            }
            let here = Loc { addr: a, thumb: l.thumb };
            if !d.insns.insert(here) {
                break; // already decoded from here on
            }
            if l.thumb {
                let op = rom.r16(a) as u32;
                let (f, len) = thumb_flow(rom, a, op);
                if len == 2 {
                    d.insns.insert(Loc { addr: a + 2, thumb: true });
                }
                match f {
                    Flow::Next => a += 2 * len as u32,
                    Flow::End(succ) => {
                        for s in succ {
                            if !d.leaders.contains(&s) || !d.insns.contains(&s) {
                                work.push_back(s);
                            }
                            d.leaders.insert(s);
                        }
                        break;
                    }
                }
            } else {
                let op = rom.r32(a);
                match arm_flow(a, op) {
                    Flow::Next => a += 4,
                    Flow::End(succ) => {
                        for s in succ {
                            work.push_back(s);
                            d.leaders.insert(s);
                        }
                        break;
                    }
                }
            }
        }
    }
    d
}

/// Emits one Thumb instruction. Returns (code, ends_block, halfwords consumed).
fn emit_thumb(rom: &Rom, a: u32, op: u32, can_fuse: bool, s: &mut String) -> (bool, usize) {
    let rd = op & 7;
    let rs = (op >> 3) & 7;
    let w = |s: &mut String, t: String| {
        s.push_str("    ");
        s.push_str(&t);
        s.push('\n');
    };
    match op >> 11 {
        0..=2 => w(s, format!("g.t_shift_imm({}, {}, {}, {});", op >> 11, rd, rs, (op >> 6) & 31)),
        3 => {
            let rn = (op >> 6) & 7;
            let f = ["t_add_reg", "t_sub_reg", "t_add_imm3", "t_sub_imm3"][((op >> 9) & 3) as usize];
            w(s, format!("g.{}({}, {}, {});", f, rd, rs, rn));
        }
        4..=7 => {
            let rd = (op >> 8) & 7;
            let f = ["t_mov_imm", "t_cmp_imm", "t_add_imm8", "t_sub_imm8"][((op >> 11) & 3) as usize];
            w(s, format!("g.{}({}, {});", f, rd, op & 0xFF));
        }
        8 => {
            if op & 0x400 == 0 {
                w(s, format!("g.t_alu({}, {}, {});", (op >> 6) & 15, rd, rs));
            } else {
                let h = (op >> 8) & 3;
                let rd = rd | ((op >> 4) & 8);
                let rs = (op >> 3) & 15;
                if rs == 15 || (rd == 15 && h != 3) {
                    w(s, format!("g.cpu.r[15] = 0x{:08X};", a + 4));
                }
                let ends = h == 3 || ((h == 0 || h == 2) && rd == 15);
                if ends {
                    w(s, format!("g.pc = 0x{:08X};", a + 2));
                }
                w(s, format!("g.t_hireg({}, {}, {});", h, rd, rs));
                if ends {
                    return (true, 1);
                }
            }
        }
        9 => {
            let rd = (op >> 8) & 7;
            let addr = (a.wrapping_add(4) & !2).wrapping_add((op & 0xFF) * 4);
            if rom.is_const(addr) {
                // literal pool in ROM is constant: fold the load
                w(s, format!("g.cpu.r[{}] = 0x{:08X}; g.cycles += 2;", rd, rom.r32(addr)));
            } else {
                w(s, format!("g.t_ldr_lit({}, 0x{:08X});", rd, addr));
            }
        }
        10 | 11 => {
            let ro = (op >> 6) & 7;
            if op & 0x200 == 0 {
                let l = (op >> 11) & 1;
                let b = (op >> 10) & 1;
                w(s, format!("g.t_ldst_reg({}, {}, {}, {});", (l << 1) | b, rd, rs, ro));
            } else {
                let h = (op >> 11) & 1;
                let sb = (op >> 10) & 1;
                w(s, format!("g.t_ldst_h_reg({}, {}, {}, {});", (sb << 1) | h, rd, rs, ro));
            }
        }
        12..=15 => {
            let b = (op >> 12) & 1;
            let l = (op >> 11) & 1;
            let off5 = (op >> 6) & 31;
            let off = if b == 1 { off5 } else { off5 * 4 };
            w(s, format!("g.t_ldst_imm({}, {}, {}, {});", (l << 1) | b, rd, rs, off));
        }
        16 | 17 => {
            let l = (op >> 11) & 1;
            w(s, format!("g.t_ldst_h_imm({}, {}, {}, {});", l, rd, rs, ((op >> 6) & 31) * 2));
        }
        18 | 19 => {
            let l = (op >> 11) & 1;
            let rd = (op >> 8) & 7;
            w(s, format!("g.t_ldst_imm({}, {}, 13, {});", l << 1, rd, (op & 0xFF) * 4));
        }
        20 => {
            let rd = (op >> 8) & 7;
            let v = (a.wrapping_add(4) & !2).wrapping_add((op & 0xFF) * 4);
            w(s, format!("g.cpu.r[{}] = 0x{:08X};", rd, v));
        }
        21 => {
            let rd = (op >> 8) & 7;
            w(s, format!("g.t_add_sp_rd({}, {});", rd, (op & 0xFF) * 4));
        }
        22 | 23 => match (op >> 8) & 0xF {
            0 => {
                let imm = ((op & 0x7F) * 4) as i32;
                w(s, format!("g.t_add_sp({});", if op & 0x80 != 0 { -imm } else { imm }));
            }
            4 | 5 => w(s, format!("g.t_push(0x{:02X}, {});", op & 0xFF, op & 0x100 != 0)),
            0xC => w(s, format!("g.t_pop(0x{:02X}, false);", op & 0xFF)),
            0xD => {
                w(s, format!("g.pc = 0x{:08X};", a + 2));
                w(s, format!("g.t_pop(0x{:02X}, true);", op & 0xFF));
                return (true, 1);
            }
            _ => {
                w(s, format!("g.pc = 0x{:08X}; g.undefined(0x{:08X});", a + 2, a));
                return (true, 1);
            }
        },
        24 => w(s, format!("g.t_stmia({}, 0x{:02X});", (op >> 8) & 7, op & 0xFF)),
        25 => w(s, format!("g.t_ldmia({}, 0x{:02X});", (op >> 8) & 7, op & 0xFF)),
        26 | 27 => {
            let cond = (op >> 8) & 0xF;
            if cond == 0xF {
                w(s, format!("g.pc = 0x{:08X};", a + 2));
                w(s, format!("g.swi(0x{:02X});", op & 0xFF));
            } else if cond == 0xE {
                w(s, format!("g.pc = 0x{:08X}; g.undefined(0x{:08X});", a + 2, a));
            } else {
                let t = a.wrapping_add(4).wrapping_add((sext(op & 0xFF, 8) * 2) as u32);
                w(
                    s,
                    format!(
                        "if g.cpu.cond({}) {{ g.pc = 0x{:08X}; g.cycles += 2; }} else {{ g.pc = 0x{:08X}; }}",
                        cond,
                        t,
                        a + 2
                    ),
                );
            }
            return (true, 1);
        }
        28 => {
            let t = a.wrapping_add(4).wrapping_add((sext(op & 0x7FF, 11) * 2) as u32);
            w(s, format!("g.pc = 0x{:08X}; g.cycles += 2;", t));
            return (true, 1);
        }
        29 => {
            w(s, format!("g.pc = 0x{:08X}; g.undefined(0x{:08X});", a + 2, a));
            return (true, 1);
        }
        30 => {
            if can_fuse && rom.in_rom(a + 2) {
                let lo = rom.r16(a + 2) as u32;
                if lo >> 11 == 31 {
                    let hi = (sext(op & 0x7FF, 11) << 12) as u32;
                    let t = a.wrapping_add(4).wrapping_add(hi).wrapping_add((lo & 0x7FF) << 1);
                    w(s, format!("g.t_bl(0x{:08X}, 0x{:08X});", t, a + 4));
                    return (true, 2);
                }
            }
            let off = (sext(op & 0x7FF, 11) << 12) as u32;
            w(s, format!("g.cpu.r[14] = 0x{:08X};", a.wrapping_add(4).wrapping_add(off)));
        }
        _ => {
            w(
                s,
                format!(
                    "let t = g.cpu.r[14].wrapping_add(0x{:X}); g.t_bl(t, 0x{:08X});",
                    (op & 0x7FF) << 1,
                    a + 2
                ),
            );
            return (true, 1);
        }
    }
    (false, 1)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: recomp <rom.gba> <out_crate_src_dir> [trace files...]");
        std::process::exit(1);
    }
    let mut rom = Rom { regions: vec![(ROM_BASE, std::fs::read(&args[1]).expect("rom"))] };
    let out = std::path::PathBuf::from(&args[2]);
    rom.regions.push((0, gba::bios::bios_image()));
    if let Ok(d) = std::fs::read("traces/iwram.bin") {
        rom.regions.push((0x0300_0000, d));
    }
    if let Ok(d) = std::fs::read("traces/ewram.bin") {
        rom.regions.push((0x0200_0000, d));
    }
    let mut seeds = vec![Loc { addr: ROM_BASE, thumb: false }];
    let mut traced = 0;
    for t in &args[3..] {
        let txt = std::fs::read_to_string(t).expect("trace");
        for line in txt.lines() {
            let mut it = line.split_whitespace();
            let (Some(a), Some(th)) = (it.next(), it.next()) else { continue };
            let a = u32::from_str_radix(a, 16).unwrap();
            if (2..=9).contains(&(a >> 24)) || a < 0x4000 {
                seeds.push(Loc { addr: a, thumb: th == "1" });
                traced += 1;
            }
        }
    }
    // Function-pointer scan: aligned words pointing at a Thumb `push {..., lr}`.
    let romlen = rom.regions[0].1.len() as u32;
    let mut ptrs = 0;
    let mut a = ROM_BASE;
    while a + 4 <= ROM_BASE + romlen {
        let w = rom.r32(a);
        if w & 1 == 1 && w >= ROM_BASE && w < ROM_BASE + romlen - 4 {
            let t = w & !1;
            if rom.r16(t) & 0xFF00 == 0xB500 {
                seeds.push(Loc { addr: t, thumb: true });
                ptrs += 1;
            }
        }
        a += 4;
    }
    eprintln!("pointer-scan seeds: {}", ptrs);
    let d = discover(&rom, &seeds);
    eprintln!("seeds {} (traced {}), leaders {}, instructions {}", seeds.len(), traced, d.leaders.len(), d.insns.len());

    // Generate blocks
    let leaders: Vec<Loc> = d.leaders.iter().copied().filter(|l| rom.in_rom(l.addr)).collect();
    let mut blocks: BTreeMap<Loc, (String, u32)> = BTreeMap::new();
    for &l in &leaders {
        let mut body = String::new();
        let mut a = l.addr;
        let mut n = 0u32;
        let mut ended = false;
        while (n as usize) < MAX_BLOCK && rom.in_rom(a) {
            if n > 0 && d.leaders.contains(&Loc { addr: a, thumb: l.thumb }) {
                break;
            }
            if l.thumb {
                let op = rom.r16(a) as u32;
                if gba::wide::HOOKS.contains(&a) {
                    // patched instruction: emit it alone, then the hook
                    let (end, len) = emit_thumb(&rom, a, op, false, &mut body);
                    let _ = writeln!(body, "    g.post_hook(0x{:08X});", a);
                    n += len as u32;
                    a += 2 * len as u32;
                    if end {
                        ended = true;
                        break;
                    }
                    continue;
                }
                // do not fuse a BL pair if the second half is itself a leader
                let can_fuse = !d.leaders.contains(&Loc { addr: a + 2, thumb: true });
                let (end, len) = emit_thumb(&rom, a, op, can_fuse, &mut body);
                n += len as u32;
                a += 2 * len as u32;
                if end {
                    ended = true;
                    break;
                }
            } else {
                let op = rom.r32(a);
                let _ = writeln!(body, "    g.cpu.r[15] = 0x{:08X}; g.pc = 0x{:08X}; g.arm_exec(0x{:08X}, 0x{:08X});", a + 8, a + 4, op, a);
                n += 1;
                let flow = arm_flow(a, op);
                a += 4;
                let _ = writeln!(body, "    if g.pc != 0x{:08X} || g.cpu.t {{ return; }}", a);
                if let Flow::End(_) = flow {
                    ended = true;
                    break;
                }
            }
        }
        if !ended {
            let _ = writeln!(body, "    g.pc = 0x{:08X};", a);
        }
        if l.addr >> 24 < 8 && l.addr >= 0x4000 {
            // RAM code: only valid while the bytes still match the snapshot
            let (d, o) = rom.region(l.addr).unwrap();
            let len = (a - l.addr) as usize;
            let bytes: Vec<String> = d[o..o + len].iter().map(|b| format!("{}", b)).collect();
            body = format!(
                "    const CODE: [u8; {}] = [{}];\n    if !g.code_matches(0x{:08X}, &CODE) {{ g.interp_block(); return; }}\n{}",
                len,
                bytes.join(","),
                l.addr,
                body
            );
        }
        blocks.insert(l, (body, n));
    }

    // Write modules
    std::fs::create_dir_all(&out).unwrap();
    let per_mod = 1500;
    let list: Vec<(&Loc, &(String, u32))> = blocks.iter().collect();
    let nmods = list.len().div_ceil(per_mod);
    let mut lib = String::new();
    lib.push_str("//! AUTO-GENERATED by `recomp` from the Fire Emblem (USA) ROM. Do not edit.\n");
    lib.push_str("//! Each function is one basic block of the original game code, translated to Rust.\n");
    lib.push_str("#![allow(unused_variables, unused_imports, non_snake_case, clippy::all)]\nuse gba::{BlockFn, Gba};\n\n");
    for m in 0..nmods {
        let _ = writeln!(lib, "mod m{:03};", m);
        let mut src = String::from("#![allow(unused_variables, unused_imports, non_snake_case, clippy::all)]\nuse gba::Gba;\n\n");
        for (l, (body, n)) in &list[m * per_mod..((m + 1) * per_mod).min(list.len())] {
            let _ = writeln!(src, "pub fn {}_{:08X}(g: &mut Gba) {{", if l.thumb { "t" } else { "a" }, l.addr);
            let _ = writeln!(src, "    g.cycles += {};", n);
            src.push_str(body);
            src.push_str("}\n\n");
        }
        std::fs::write(out.join(format!("m{:03}.rs", m)), src).unwrap();
    }
    let _ = writeln!(lib, "\npub const BLOCK_COUNT: usize = {};", list.len());
    lib.push_str("/// (address, thumb, function, instruction count), sorted by address.\n");
    lib.push_str("pub static BLOCKS: &[(u32, bool, BlockFn, u32)] = &[\n");
    for (i, (l, (_, n))) in list.iter().enumerate() {
        let m = i / per_mod;
        let _ = writeln!(
            lib,
            "    (0x{:08X}, {}, m{:03}::{}_{:08X}, {}),",
            l.addr,
            l.thumb,
            m,
            if l.thumb { "t" } else { "a" },
            l.addr,
            n
        );
    }
    lib.push_str("];\n\n");
    lib.push_str(LOOKUP_SRC);
    std::fs::write(out.join("lib.rs"), lib).unwrap();
    let total_lines: usize = blocks.values().map(|(b, _)| b.lines().count()).sum();
    eprintln!("wrote {} blocks in {} modules, {} lines of block code", list.len(), nmods, total_lines);
}

const LOOKUP_SRC: &str = r#"
struct Region {
    base: u32,
    thumb: Vec<u32>, // index+1 into BLOCKS, 0 = none (per halfword)
    arm: Vec<u32>,   // per word
}

impl Region {
    fn new(base: u32, size: u32) -> Region {
        Region { base, thumb: vec![0; (size / 2) as usize], arm: vec![0; (size / 4) as usize] }
    }
    #[inline]
    fn get(&self, addr: u32, thumb: bool) -> Option<usize> {
        let off = addr.wrapping_sub(self.base);
        let i = if thumb { *self.thumb.get((off / 2) as usize)? } else { *self.arm.get((off / 4) as usize)? };
        if i == 0 { None } else { Some(i as usize - 1) }
    }
}

/// Dense lookup tables per memory region (ROM, EWRAM, IWRAM, BIOS).
struct Table {
    rom: Region,
    ewram: Region,
    iwram: Region,
    bios: Region,
}

static TABLE: std::sync::OnceLock<Table> = std::sync::OnceLock::new();

fn table() -> &'static Table {
    TABLE.get_or_init(|| {
        let base = 0x0800_0000u32;
        let max = BLOCKS.iter().map(|b| b.0).filter(|&a| a >= base).max().unwrap_or(base);
        let mut t = Table {
            rom: Region::new(base, (max - base + 4) & !3),
            ewram: Region::new(0x0200_0000, 0x4_0000),
            iwram: Region::new(0x0300_0000, 0x8000),
            bios: Region::new(0, 0x4000),
        };
        for (i, b) in BLOCKS.iter().enumerate() {
            let r = match b.0 >> 24 {
                0 => &mut t.bios,
                2 => &mut t.ewram,
                3 => &mut t.iwram,
                _ => &mut t.rom,
            };
            let off = b.0 - r.base;
            if b.1 {
                r.thumb[(off / 2) as usize] = i as u32 + 1;
            } else {
                r.arm[(off / 4) as usize] = i as u32 + 1;
            }
        }
        t
    })
}

/// Finds the recompiled block for an address.
#[inline]
pub fn lookup(addr: u32, thumb: bool) -> Option<BlockFn> {
    lookup_index(addr, thumb).map(|i| BLOCKS[i].2)
}

pub fn lookup_verify(addr: u32, thumb: bool) -> Option<(BlockFn, u32)> {
    lookup_index(addr, thumb).map(|i| (BLOCKS[i].2, BLOCKS[i].3))
}

#[inline]
pub fn lookup_index(addr: u32, thumb: bool) -> Option<usize> {
    let t = table();
    match addr >> 24 {
        0x08..=0x09 => t.rom.get(addr, thumb),
        0x03 => t.iwram.get(addr, thumb),
        0x02 => t.ewram.get(addr, thumb),
        0x00 => t.bios.get(addr, thumb),
        _ => None,
    }
}
"#;
