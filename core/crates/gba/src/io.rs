//! I/O registers, DMA, timers, PPU timeline.
use crate::{Gba, CYCLES_PER_LINE, HBLANK_START};

const DMA_BASE: [usize; 4] = [0xB0, 0xBC, 0xC8, 0xD4];
const PRESCALE: [u64; 4] = [1, 64, 256, 1024];

impl Gba {
    pub fn io_read16(&mut self, addr: u32) -> u16 {
        let off = (addr.wrapping_sub(0x0400_0000)) as usize;
        if off >= 0x3FE {
            return 0;
        }
        match off {
            0x004 => {
                let mut v = self.io16(4) & 0xFF38;
                let vc = self.ppu.vcount;
                if (160..227).contains(&vc) {
                    v |= 1;
                }
                if self.ppu.in_hblank {
                    v |= 2;
                }
                if vc == (self.io16(4) >> 8) as u32 {
                    v |= 4;
                }
                v
            }
            0x006 => self.ppu.vcount as u16,
            0x100 | 0x104 | 0x108 | 0x10C => self.timers[(off - 0x100) / 4].counter as u16,
            0x130 => !self.keys & 0x3FF,
            _ => self.io16(off),
        }
    }

    pub fn io_write8(&mut self, addr: u32, v: u8) {
        let off = (addr.wrapping_sub(0x0400_0000)) as usize;
        if off >= 0x400 {
            return;
        }
        match off {
            0x202 => self.io[0x202] &= !v,
            0x203 => self.io[0x203] &= !v,
            0x301 => {
                self.io[0x301] = v;
                self.halted = true;
            }
            0xA0..=0xA7 => self.apu.fifo_push((off - 0xA0) / 4, v as i8),
            _ => {
                let a = off & !1;
                let old = self.io16(a);
                let nv = if off & 1 != 0 { (old & 0x00FF) | ((v as u16) << 8) } else { (old & 0xFF00) | v as u16 };
                self.io_write16(addr & !1, nv);
            }
        }
    }

    pub fn io_write32(&mut self, addr: u32, v: u32) {
        let off = (addr.wrapping_sub(0x0400_0000)) as usize;
        if off == 0xA0 || off == 0xA4 {
            let f = (off - 0xA0) / 4;
            for i in 0..4 {
                self.apu.fifo_push(f, (v >> (i * 8)) as u8 as i8);
            }
            return;
        }
        self.io_write16(addr, v as u16);
        self.io_write16(addr + 2, (v >> 16) as u16);
    }

    pub fn io_write16(&mut self, addr: u32, v: u16) {
        let off = (addr.wrapping_sub(0x0400_0000)) as usize;
        if off >= 0x3FE {
            return;
        }
        match off {
            0x004 => {
                let old = self.io16(4);
                self.set_io16(4, (old & 7) | (v & !7));
            }
            0x006 => {}
            0x028..=0x02F | 0x038..=0x03F => {
                self.set_io16(off, v);
                let base = if off < 0x38 { 0x28 } else { 0x38 };
                let bg = if off < 0x38 { 0 } else { 1 };
                let x = (self.io16(base) as u32 | (self.io16(base + 2) as u32) << 16) << 4;
                let y = (self.io16(base + 4) as u32 | (self.io16(base + 6) as u32) << 16) << 4;
                self.ppu.ref_x[bg] = (x as i32) >> 4;
                self.ppu.ref_y[bg] = (y as i32) >> 4;
            }
            0x0A0..=0x0A7 => {
                let f = (off - 0xA0) / 4;
                self.apu.fifo_push(f, v as u8 as i8);
                self.apu.fifo_push(f, (v >> 8) as u8 as i8);
            }
            0x060..=0x09F => {
                self.set_io16(off, v);
                self.apu_reg_write(off, v);
            }
            0x0BA | 0x0C6 | 0x0D2 | 0x0DE => {
                let ch = (off - 0xBA) / 12;
                self.dma_write_ctrl(ch, v);
            }
            0x100 | 0x104 | 0x108 | 0x10C => {
                self.timers[(off - 0x100) / 4].reload = v;
                self.set_io16(off, v);
            }
            0x102 | 0x106 | 0x10A | 0x10E => {
                let i = (off - 0x102) / 4;
                let t = &mut self.timers[i];
                if t.ctrl & 0x80 == 0 && v & 0x80 != 0 {
                    t.counter = t.reload as u32;
                    t.sub = 0;
                }
                t.ctrl = v;
                self.set_io16(off, v);
            }
            0x202 => {
                let old = self.io16(0x202);
                self.set_io16(0x202, old & !v);
            }
            0x300 => {
                self.io[0x300] = v as u8;
                self.io[0x301] = (v >> 8) as u8;
                self.halted = true;
            }
            _ => self.set_io16(off, v),
        }
    }

    fn io32(&self, off: usize) -> u32 {
        self.io16(off) as u32 | (self.io16(off + 2) as u32) << 16
    }

    fn dma_write_ctrl(&mut self, ch: usize, v: u16) {
        let base = DMA_BASE[ch];
        let old = self.dma[ch].ctrl;
        self.dma[ch].ctrl = v;
        self.set_io16(base + 10, v);
        if old & 0x8000 == 0 && v & 0x8000 != 0 {
            let src_mask = if ch == 0 { 0x07FF_FFFF } else { 0x0FFF_FFFF };
            let dst_mask = if ch == 3 { 0x0FFF_FFFF } else { 0x07FF_FFFF };
            self.dma[ch].src = self.io32(base) & src_mask;
            self.dma[ch].dst = self.io32(base + 4) & dst_mask;
            self.dma[ch].count = self.dma_count(ch);
            if (v >> 12) & 3 == 0 {
                self.run_dma(ch);
            }
        }
    }

    fn dma_count(&self, ch: usize) -> u32 {
        let c = self.io16(DMA_BASE[ch] + 8) as u32;
        if ch == 3 {
            if c == 0 {
                0x10000
            } else {
                c
            }
        } else {
            let c = c & 0x3FFF;
            if c == 0 {
                0x4000
            } else {
                c
            }
        }
    }

    pub fn run_dma(&mut self, ch: usize) {
        let ctrl = self.dma[ch].ctrl;
        let timing = (ctrl >> 12) & 3;
        let fifo = timing == 3 && (ch == 1 || ch == 2);
        let word = ctrl & (1 << 10) != 0 || fifo;
        let dctl = if fifo { 2 } else { (ctrl >> 5) & 3 };
        let sctl = (ctrl >> 7) & 3;
        let count = if fifo { 4 } else { self.dma[ch].count };
        let step: u32 = if word { 4 } else { 2 };
        let mut src = self.dma[ch].src;
        let mut dst = self.dma[ch].dst;
        if self.watch_w == 1 && (0x0600_0000..0x0601_0000).contains(&dst) {
            eprintln!("dma{} {:08x} -> {:08x} n {} word {}", ch, src, dst, count, word);
        }
        for _ in 0..count {
            if word {
                let v = self.read32(src & !3);
                self.write32(dst & !3, v);
            } else {
                let v = self.read16(src & !1);
                self.write16(dst & !1, v);
            }
            match sctl {
                0 | 3 => src = src.wrapping_add(step),
                1 => src = src.wrapping_sub(step),
                _ => {}
            }
            match dctl {
                0 | 3 => dst = dst.wrapping_add(step),
                1 => dst = dst.wrapping_sub(step),
                _ => {}
            }
        }
        self.cycles += count as u64 * 2;
        self.dma[ch].src = src;
        self.dma[ch].dst = dst;
        if ctrl & (1 << 14) != 0 {
            self.request_irq(1 << (8 + ch));
        }
        if ctrl & (1 << 9) == 0 || timing == 0 {
            self.dma[ch].ctrl &= !0x8000;
            let c = self.dma[ch].ctrl;
            self.set_io16(DMA_BASE[ch] + 10, c);
        } else {
            if dctl == 3 {
                self.dma[ch].dst = self.io32(DMA_BASE[ch] + 4) & 0x0FFF_FFFF;
            }
            self.dma[ch].count = self.dma_count(ch);
        }
    }

    pub fn trigger_dma(&mut self, timing: u16) {
        for ch in 0..4 {
            let c = self.dma[ch].ctrl;
            if c & 0x8000 != 0 && (c >> 12) & 3 == timing {
                if timing == 3 && (ch == 1 || ch == 2) {
                    continue;
                }
                self.run_dma(ch);
            }
        }
    }

    pub fn trigger_fifo_dma(&mut self, fifo: usize) {
        let target = 0x0400_00A0 + fifo as u32 * 4;
        for ch in 1..3 {
            let c = self.dma[ch].ctrl;
            if c & 0x8000 != 0 && (c >> 12) & 3 == 3 && self.dma[ch].dst == target {
                self.run_dma(ch);
            }
        }
    }

    pub fn tick_timers(&mut self, delta: u64) {
        self.apu_tick(delta);
        for i in 0..4 {
            let ctrl = self.timers[i].ctrl;
            if ctrl & 0x80 == 0 || (i > 0 && ctrl & 4 != 0) {
                continue;
            }
            let pre = PRESCALE[(ctrl & 3) as usize];
            let t = &mut self.timers[i];
            t.sub += delta;
            let ticks = t.sub / pre;
            t.sub %= pre;
            if ticks == 0 {
                continue;
            }
            self.timer_add(i, ticks);
        }
    }

    fn timer_add(&mut self, i: usize, ticks: u64) {
        let mut c = self.timers[i].counter as u64 + ticks;
        let reload = self.timers[i].reload as u64;
        let mut overflows = 0u64;
        while c >= 0x10000 {
            let period = 0x10000 - reload;
            c -= period;
            overflows += 1;
        }
        self.timers[i].counter = c as u32;
        for _ in 0..overflows {
            self.timer_overflow(i);
        }
    }

    fn timer_overflow(&mut self, i: usize) {
        let ctrl = self.timers[i].ctrl;
        if ctrl & 0x40 != 0 {
            self.request_irq(1 << (3 + i));
        }
        if i < 2 {
            let cnt_h = self.io16(0x82);
            for f in 0..2 {
                let sel = (cnt_h >> (10 + f * 4)) & 1;
                if sel as usize == i && self.apu.fifo_pop(f) {
                    self.trigger_fifo_dma(f);
                }
            }
        }
        if i < 3 {
            let n = self.timers[i + 1].ctrl;
            if n & 0x80 != 0 && n & 4 != 0 {
                self.timer_add(i + 1, 1);
            }
        }
    }

    pub fn ppu_next_event(&self) -> u64 {
        self.ppu.next_event
    }

    pub fn ppu_update(&mut self) {
        while self.cycles >= self.ppu.next_event {
            if !self.ppu.in_hblank {
                self.ppu.in_hblank = true;
                self.ppu.next_event += CYCLES_PER_LINE - HBLANK_START;
                let vc = self.ppu.vcount;
                if vc < 160 {
                    if !self.no_render {
                        self.render_line(vc);
                    }
                    self.trigger_dma(2);
                }
                if self.io16(4) & 0x10 != 0 {
                    self.request_irq(2);
                }
            } else {
                self.ppu.in_hblank = false;
                self.ppu.next_event += HBLANK_START;
                self.ppu.vcount = (self.ppu.vcount + 1) % crate::LINES;
                let vc = self.ppu.vcount;
                if vc == 160 {
                    if self.io16(4) & 0x8 != 0 {
                        self.request_irq(1);
                    }
                    self.trigger_dma(1);
                    self.ppu.reload_affine(&self.io);
                    self.frame += 1;
                }
                if vc == (self.io16(4) >> 8) as u32 && self.io16(4) & 0x20 != 0 {
                    self.request_irq(4);
                }
            }
        }
    }
}
