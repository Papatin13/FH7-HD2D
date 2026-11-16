//! Save states (whole machine snapshot, ROM excluded).
use crate::{Cpu, Gba};

const MAGIC: &[u8; 8] = b"FE7RSST1";

struct W(Vec<u8>);
impl W {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
}
struct R<'a>(&'a [u8], usize);
impl R<'_> {
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[self.1..self.1 + 4].try_into().unwrap());
        self.1 += 4;
        v
    }
    fn u64(&mut self) -> u64 {
        let v = u64::from_le_bytes(self.0[self.1..self.1 + 8].try_into().unwrap());
        self.1 += 8;
        v
    }
    fn bytes(&mut self, dst: &mut [u8]) {
        let n = self.u32() as usize;
        dst.copy_from_slice(&self.0[self.1..self.1 + n]);
        self.1 += n;
    }
}

impl Gba {
    pub fn save_state(&self) -> Vec<u8> {
        let mut w = W(MAGIC.to_vec());
        let cw = self.cpu.to_words();
        w.u32(cw.len() as u32);
        for x in cw {
            w.u32(x);
        }
        w.u32(self.pc);
        w.u64(self.cycles);
        w.u32(self.halted as u32);
        w.u32(self.intr_wait as u32);
        w.u32(self.intr_wait_pc);
        for m in [&self.ewram, &self.iwram, &self.io, &self.pal, &self.vram, &self.oam, &self.sram] {
            w.bytes(m);
        }
        for d in &self.dma {
            w.u32(d.src);
            w.u32(d.dst);
            w.u32(d.count);
            w.u32(d.ctrl as u32);
        }
        for t in &self.timers {
            w.u32(t.reload as u32);
            w.u32(t.counter);
            w.u32(t.ctrl as u32);
            w.u64(t.sub);
        }
        w.u32(self.ppu.vcount);
        w.u32(self.ppu.in_hblank as u32);
        w.u64(self.ppu.next_event);
        for i in 0..2 {
            w.u32(self.ppu.ref_x[i] as u32);
            w.u32(self.ppu.ref_y[i] as u32);
        }
        w.u64(self.frame);
        w.0
    }

    pub fn load_state(&mut self, d: &[u8]) -> bool {
        if d.len() < 8 || &d[..8] != MAGIC {
            return false;
        }
        let mut r = R(d, 8);
        let n = r.u32() as usize;
        let cw: Vec<u32> = (0..n).map(|_| r.u32()).collect();
        self.cpu = Cpu::from_words(&cw);
        self.pc = r.u32();
        self.cycles = r.u64();
        self.halted = r.u32() != 0;
        self.intr_wait = r.u32() as u16;
        self.intr_wait_pc = r.u32();
        let mut bufs = [
            std::mem::take(&mut self.ewram),
            std::mem::take(&mut self.iwram),
            std::mem::take(&mut self.io),
            std::mem::take(&mut self.pal),
            std::mem::take(&mut self.vram),
            std::mem::take(&mut self.oam),
            std::mem::take(&mut self.sram),
        ];
        for b in bufs.iter_mut() {
            r.bytes(b);
        }
        let [a, b, c, dd, e, f, g] = bufs;
        self.ewram = a;
        self.iwram = b;
        self.io = c;
        self.pal = dd;
        self.vram = e;
        self.oam = f;
        self.sram = g;
        for i in 0..4 {
            self.dma[i].src = r.u32();
            self.dma[i].dst = r.u32();
            self.dma[i].count = r.u32();
            self.dma[i].ctrl = r.u32() as u16;
        }
        for i in 0..4 {
            self.timers[i].reload = r.u32() as u16;
            self.timers[i].counter = r.u32();
            self.timers[i].ctrl = r.u32() as u16;
            self.timers[i].sub = r.u64();
        }
        self.ppu.vcount = r.u32();
        self.ppu.in_hblank = r.u32() != 0;
        self.ppu.next_event = r.u64();
        for i in 0..2 {
            self.ppu.ref_x[i] = r.u32() as i32;
            self.ppu.ref_y[i] = r.u32() as i32;
        }
        self.frame = r.u64();
        self.apu.fifo[0].clear();
        self.apu.fifo[1].clear();
        true
    }
}
