//! GBA platform layer for the Fire Emblem 7 PC port.
//!
//! The game code itself runs either as statically recompiled Rust (crate `fe7gen`)
//! or, for code that was not recompiled (RAM code, undiscovered paths), through
//! the fallback interpreter in `thumb.rs` / `arm.rs`. Both paths share the exact
//! same instruction-semantics helpers.

pub mod apu;
pub mod arm;
pub mod bios;
pub mod bus;
pub mod cpu;
pub mod io;
pub mod ppu;
pub mod state;
pub mod thumb;
pub mod wide;
pub mod hd;
pub mod text;

pub use cpu::Cpu;

/// A recompiled basic block. It executes a run of instructions starting at the
/// address it was registered for and leaves `g.pc` at the next address to run.
pub type BlockFn = fn(&mut Gba);
/// Lookup of recompiled code: (address, thumb) -> block.
pub type LookupFn = fn(u32, bool) -> Option<BlockFn>;

pub const CYCLES_PER_LINE: u64 = 1232;
pub const HBLANK_START: u64 = 960;
pub const LINES: u32 = 228;

pub mod keys {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const RIGHT: u16 = 1 << 4;
    pub const LEFT: u16 = 1 << 5;
    pub const UP: u16 = 1 << 6;
    pub const DOWN: u16 = 1 << 7;
    pub const R: u16 = 1 << 8;
    pub const L: u16 = 1 << 9;
}

#[derive(Default, Clone, Copy)]
pub struct Dma {
    pub src: u32,
    pub dst: u32,
    pub count: u32,
    pub ctrl: u16,
    pub active: bool,
}

#[derive(Default, Clone, Copy)]
pub struct Timer {
    pub reload: u16,
    pub counter: u32,
    pub ctrl: u16,
    pub sub: u64, // prescaler remainder
}

pub struct Stats {
    pub blocks_recomp: u64,
    pub blocks_interp: u64,
    pub instrs_interp: u64,
}

pub struct Gba {
    pub cpu: Cpu,
    /// Address of the next instruction to execute (no pipeline offset).
    pub pc: u32,
    pub cycles: u64,
    pub halted: bool,
    pub intr_wait: u16,
    pub intr_wait_pc: u32,

    pub bios: Vec<u8>,
    pub ewram: Vec<u8>,
    pub iwram: Vec<u8>,
    pub io: Vec<u8>,
    pub pal: Vec<u8>,
    pub vram: Vec<u8>,
    pub oam: Vec<u8>,
    pub rom: Vec<u8>,
    pub sram: Vec<u8>,
    pub sram_dirty: bool,

    pub dma: [Dma; 4],
    pub timers: [Timer; 4],
    pub timer_last: u64,

    pub ppu: ppu::Ppu,
    pub apu: apu::Apu,

    /// Pressed keys, active high (keys::*).
    pub keys: u16,
    pub frame: u64,

    pub lookup: Option<LookupFn>,
    pub use_recomp: bool,
    pub stats: Stats,
    /// Called for every interpreted block start address when set (for tracing).
    pub trace: Option<std::collections::HashSet<(u32, bool)>>,
    pub last_bios_read: u32,
    /// Lockstep verification: lookup returning (block, instruction count).
    pub verify: Option<fn(u32, bool) -> Option<(BlockFn, u32)>>,
    pub verify_fail: std::collections::HashSet<u32>,
    pub verify_count: u64,
    /// Write log for verification: (addr, size, old, new).
    pub wlog: Option<Vec<(u32, u8, u32, u32)>>,
    /// Debug: word address to watch for reads (0 = off) and the PCs that read it.
    pub watch: u32,
    /// Debug: word address to watch for writes (0 = off).
    pub watch_w: u32,
    /// Widescreen: extra columns per side requested by the frontend (0 = off).
    pub wide_ext: u32,
    /// HD-2D presentation state (see hd.rs).
    pub hd: hd::Hd,
    /// Debug/profiling: skip the PPU.
    pub no_render: bool,
    /// Palette RAM decoded to 15-bit colours (refreshed before each line).
    pub pal_rgb15: [u16; 512],
    /// Widescreen scenery statistics for the current map.
    pub mapgen: wide::MapGen,
    /// Widescreen map widening: decompressed map buffer of the loader and the
    /// last widening (old width, new width).
    pub map_load_buf: u32,
    pub map_widened: Option<(u32, u32)>,
    /// Debug: chapter to start instead of the next one (applied once in StartChapter).
    pub debug_chapter: Option<u8>,
    /// Map width (in tiles) narrow maps are widened to at load (0 = off).
    /// Fixed per widescreen setting so suspended chapters stay consistent.
    pub wide_cols: u32,
    pub watch_hits: std::collections::BTreeMap<u32, u32>,
}

impl Gba {
    pub fn new(rom: Vec<u8>) -> Box<Gba> {
        let mut g = Box::new(Gba {
            cpu: Cpu::new(),
            pc: 0x0800_0000,
            cycles: 0,
            halted: false,
            intr_wait: 0,
            intr_wait_pc: 0,
            bios: bios::bios_image(),
            ewram: vec![0; 0x40000],
            iwram: vec![0; 0x8000],
            io: vec![0; 0x400],
            pal: vec![0; 0x400],
            vram: vec![0; 0x18000],
            oam: vec![0; 0x400],
            rom,
            sram: vec![0xFF; 0x10000],
            sram_dirty: false,
            dma: [Dma::default(); 4],
            timers: [Timer::default(); 4],
            timer_last: 0,
            ppu: ppu::Ppu::new(),
            apu: apu::Apu::new(),
            keys: 0,
            frame: 0,
            lookup: None,
            use_recomp: true,
            stats: Stats { blocks_recomp: 0, blocks_interp: 0, instrs_interp: 0 },
            trace: None,
            last_bios_read: 0xE129F000,
            verify: None,
            verify_fail: Default::default(),
            verify_count: 0,
            wlog: None,
            watch: 0,
            watch_w: 0,
            wide_ext: 0,
            hd: Default::default(),
            no_render: std::env::var("FE7_NORENDER").is_ok(),
            pal_rgb15: [0; 512],
            mapgen: Default::default(),
            map_load_buf: 0,
            map_widened: None,
            debug_chapter: None,
            wide_cols: 0,
            watch_hits: Default::default(),
        });
        g.reset();
        g
    }

    pub fn reset(&mut self) {
        self.cpu = Cpu::new();
        // State after the real BIOS boot sequence.
        self.cpu.set_mode(cpu::MODE_SVC);
        self.cpu.r[13] = 0x0300_7FE0;
        self.cpu.set_mode(cpu::MODE_IRQ);
        self.cpu.r[13] = 0x0300_7FA0;
        self.cpu.set_mode(cpu::MODE_SYS);
        self.cpu.r[13] = 0x0300_7F00;
        self.cpu.t = false;
        self.pc = 0x0800_0000;
        self.halted = false;
        self.intr_wait = 0;
        self.io.iter_mut().for_each(|b| *b = 0);
        // KEYINPUT idle, SOUNDBIAS default
        self.io[0x88] = 0x00;
        self.io[0x89] = 0x02;
        self.io[0x20] = 0x00;
        self.io[0x21] = 0x01; // BG2PA = 1.0
        self.io[0x26] = 0x00;
        self.io[0x27] = 0x01; // BG2PD
        self.io[0x30] = 0x00;
        self.io[0x31] = 0x01;
        self.io[0x36] = 0x00;
        self.io[0x37] = 0x01;
    }

    pub fn irq_pending(&self) -> bool {
        let ie = self.io16(0x200);
        let iff = self.io16(0x202);
        let ime = self.io16(0x208) & 1;
        ime != 0 && (ie & iff) != 0
    }

    pub fn request_irq(&mut self, bit: u16) {
        let v = self.io16(0x202) | bit;
        self.set_io16(0x202, v);
    }

    #[inline]
    pub fn io16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.io[off], self.io[off + 1]])
    }
    #[inline]
    pub fn set_io16(&mut self, off: usize, v: u16) {
        self.io[off] = v as u8;
        self.io[off + 1] = (v >> 8) as u8;
    }

    fn take_irq(&mut self) {
        let cpsr = self.cpu.cpsr();
        self.cpu.set_mode(cpu::MODE_IRQ);
        self.cpu.spsr = cpsr;
        self.cpu.r[14] = self.pc.wrapping_add(4);
        self.cpu.t = false;
        self.cpu.i = true;
        self.pc = 0x18;
    }

    /// Runs one emulated frame (until the PPU wraps to line 0).
    pub fn run_frame(&mut self) {
        let f = self.frame;
        while self.frame == f {
            self.step();
        }
    }

    pub fn step(&mut self) {
        let before = self.cycles;
        if self.irq_pending() {
            self.halted = false;
            if !self.cpu.i {
                self.take_irq();
            }
        }
        if !self.halted && self.intr_wait != 0 && self.pc == self.intr_wait_pc {
            let flags = self.read16(0x0300_7FF8);
            if flags & self.intr_wait != 0 {
                self.write16(0x0300_7FF8, flags & !self.intr_wait);
                self.intr_wait = 0;
            } else {
                self.halted = true;
                // BIOS IntrWait forces IME on.
                self.set_io16(0x208, 1);
            }
        }
        if self.halted {
            let next = self.ppu_next_event();
            self.cycles = next.max(self.cycles + 1);
        } else {
            self.exec_block();
        }
        let delta = self.cycles - before;
        self.tick_timers(delta);
        self.ppu_update();
    }

    fn exec_block(&mut self) {
        let thumb = self.cpu.t;
        if self.debug_chapter.is_some()
            && (self.pc == 0x0802_E028 || self.pc == 0x0802_E190 || self.pc == 0x0801_8D88)
        {
            // StartChapter (new / restarted chapter) or, as a fallback, the chapter map
            // loader: the debug menu's chapter is used from here on (gPlaySt.chapterIndex);
            // until then it is kept set every frame
            let c = self.debug_chapter.take().unwrap();
            self.write8(0x0202_BC06, c);
        }
        if self.hd.on && self.pc == 0x0800_5814 {
            // Text_DrawCharacter(text, str): talk boxes print one letter at a time
            self.hd_char_hook(self.cpu.r[0], self.cpu.r[1]);
        }
        if self.hd.on && self.pc == 0x0800_615C {
            // DrawSpecialUiChar(tilemap entry, colour, char): digits of UI numbers
            let (a, c) = (self.cpu.r[0], self.cpu.r[2]);
            if c < 10 && self.hd.pending_chars.len() < 256 {
                self.hd.pending_chars.push((a, c as u8));
            }
        }
        if self.hd.on && (self.pc == 0x0800_5718 || self.pc == 0x0800_57D8) {
            // Text_DrawString(text, str) / Text_DrawNumber(text, n): remember which
            // text tiles get which string
            self.hd_text_hook(self.cpu.r[0], self.cpu.r[1], self.pc == 0x0800_57D8);
        }
        if let Some(vf) = self.verify {
            if let Some((f, n)) = vf(self.pc, thumb) {
                self.verify_block(f, n);
                return;
            }
        }
        if self.use_recomp {
            if let Some(lk) = self.lookup {
                if let Some(f) = lk(self.pc, thumb) {
                    self.stats.blocks_recomp += 1;
                    f(self);
                    return;
                }
            }
        }
        self.interp_block();
    }

    /// Checks that RAM still holds the code a recompiled RAM block was built from.
    pub fn code_matches(&self, addr: u32, code: &[u8]) -> bool {
        let (mem, o) = match addr >> 24 {
            2 => (&self.ewram, (addr & 0x3FFFF) as usize),
            3 => (&self.iwram, (addr & 0x7FFF) as usize),
            _ => return false,
        };
        mem.get(o..o + code.len()) == Some(code)
    }

    /// Runs one block with the interpreter.
    pub fn interp_block(&mut self) {
        let thumb = self.cpu.t;
        self.stats.blocks_interp += 1;
        if let Some(t) = self.trace.as_mut() {
            t.insert((self.pc, thumb));
        }
        for _ in 0..64 {
            let start = self.pc;
            let size = if self.cpu.t { 2 } else { 4 };
            let t = self.cpu.t;
            self.step_instr();
            self.stats.instrs_interp += 1;
            if self.pc != start.wrapping_add(size) || self.cpu.t != t || self.halted || self.intr_wait != 0 {
                break;
            }
        }
    }

    fn verify_block(&mut self, f: BlockFn, n: u32) {
        let start = self.pc;
        let snap = (self.cpu.clone(), self.pc, self.halted, self.intr_wait, self.intr_wait_pc);
        self.wlog = Some(Vec::new());
        f(self);
        let wa = self.wlog.take().unwrap();
        let a = (self.cpu.clone(), self.pc, self.halted, self.intr_wait);
        for &(addr, size, old, _) in wa.iter().rev() {
            match size {
                1 => self.write8(addr, old as u8),
                2 => self.write16(addr, old as u16),
                _ => self.write32(addr, old),
            }
        }
        self.cpu = snap.0;
        self.pc = snap.1;
        self.halted = snap.2;
        self.intr_wait = snap.3;
        self.intr_wait_pc = snap.4;
        self.wlog = Some(Vec::new());
        for _ in 0..n {
            let s = self.pc;
            let sz = if self.cpu.t { 2 } else { 4 };
            self.step_instr();
            if self.pc != s.wrapping_add(sz) {
                break;
            }
        }
        let wb = self.wlog.take().unwrap();
        self.verify_count += 1;
        let mut diffs = Vec::new();
        for i in 0..15 {
            if a.0.r[i] != self.cpu.r[i] {
                diffs.push(format!("r{} {:08X} vs {:08X}", i, a.0.r[i], self.cpu.r[i]));
            }
        }
        if a.0.cpsr() != self.cpu.cpsr() {
            diffs.push(format!("cpsr {:08X} vs {:08X}", a.0.cpsr(), self.cpu.cpsr()));
        }
        if a.1 != self.pc {
            diffs.push(format!("pc {:08X} vs {:08X}", a.1, self.pc));
        }
        if a.2 != self.halted || a.3 != self.intr_wait {
            diffs.push("halt/intrwait".into());
        }
        let ka: Vec<_> = wa.iter().map(|w| (w.0, w.1, w.3)).collect();
        let kb: Vec<_> = wb.iter().map(|w| (w.0, w.1, w.3)).collect();
        if ka != kb {
            diffs.push(format!("writes {:X?} vs {:X?}", &ka[..ka.len().min(6)], &kb[..kb.len().min(6)]));
        }
        if !diffs.is_empty() && self.verify_fail.insert(start) {
            eprintln!("VERIFY MISMATCH block {:08X}: {}", start, diffs.join("; "));
        }
    }

    /// Executes exactly one instruction with the interpreter.
    pub fn step_instr(&mut self) {
        if self.cpu.t {
            let a = self.pc & !1;
            let op = self.read16(a);
            self.pc = a.wrapping_add(2);
            self.cpu.r[15] = a.wrapping_add(4);
            self.cycles += 1;
            self.thumb_exec(op, a);
            if self.wide_ext != 0 {
                self.post_hook(a);
            }
        } else {
            let a = self.pc & !3;
            let op = self.read32(a);
            self.pc = a.wrapping_add(4);
            self.cpu.r[15] = a.wrapping_add(8);
            self.cycles += 1;
            self.arm_exec(op, a);
        }
    }

    /// Writes PC from an instruction (data processing / load into r15).
    #[inline]
    pub fn write_pc(&mut self, v: u32) {
        if self.cpu.t {
            self.pc = v & !1;
        } else {
            self.pc = v & !3;
        }
    }

    /// BX semantics.
    #[inline]
    pub fn bx(&mut self, v: u32) {
        if v & 1 != 0 {
            self.cpu.t = true;
            self.pc = v & !1;
        } else {
            self.cpu.t = false;
            self.pc = v & !3;
        }
    }
}
