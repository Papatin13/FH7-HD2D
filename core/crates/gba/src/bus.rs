//! Memory map.
use crate::Gba;

#[inline(always)]
fn rd16(m: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([m[o], m[o + 1]])
}
#[inline(always)]
fn rd32(m: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([m[o], m[o + 1], m[o + 2], m[o + 3]])
}
#[inline(always)]
fn wr16(m: &mut [u8], o: usize, v: u16) {
    m[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
#[inline(always)]
fn wr32(m: &mut [u8], o: usize, v: u32) {
    m[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

#[inline(always)]
fn vram_off(addr: u32) -> usize {
    let o = (addr & 0x1FFFF) as usize;
    if o >= 0x18000 {
        o - 0x8000
    } else {
        o
    }
}

impl Gba {
    #[inline]
    pub fn read8(&mut self, addr: u32) -> u8 {
        if self.watch != 0 && addr & !3 == self.watch {
            *self.watch_hits.entry(self.pc).or_insert(0) += 1;
        }
        match addr >> 24 {
            0x00 => {
                if addr < 0x4000 {
                    self.bios[addr as usize]
                } else {
                    0
                }
            }
            0x02 => self.ewram[(addr & 0x3FFFF) as usize],
            0x03 => self.iwram[(addr & 0x7FFF) as usize],
            0x04 => {
                let v = self.io_read16(addr & !1);
                if addr & 1 != 0 {
                    (v >> 8) as u8
                } else {
                    v as u8
                }
            }
            0x05 => self.pal[(addr & 0x3FF) as usize],
            0x06 => self.vram[vram_off(addr)],
            0x07 => self.oam[(addr & 0x3FF) as usize],
            0x08..=0x0D => {
                let o = (addr & 0x01FF_FFFF) as usize;
                if o < self.rom.len() {
                    self.rom[o]
                } else {
                    ((addr >> 1) >> ((addr & 1) * 8)) as u8
                }
            }
            0x0E | 0x0F => self.sram[(addr & 0x7FFF) as usize],
            _ => 0,
        }
    }

    #[inline]
    pub fn read16(&mut self, addr: u32) -> u16 {
        if self.watch != 0 && addr & !3 == self.watch {
            *self.watch_hits.entry(self.pc).or_insert(0) += 1;
        }
        let addr = addr & !1;
        match addr >> 24 {
            0x00 => {
                if addr < 0x4000 {
                    rd16(&self.bios, addr as usize)
                } else {
                    0
                }
            }
            0x02 => rd16(&self.ewram, (addr & 0x3FFFF) as usize),
            0x03 => rd16(&self.iwram, (addr & 0x7FFF) as usize),
            0x04 => self.io_read16(addr),
            0x05 => rd16(&self.pal, (addr & 0x3FF) as usize),
            0x06 => rd16(&self.vram, vram_off(addr)),
            0x07 => rd16(&self.oam, (addr & 0x3FF) as usize),
            0x08..=0x0D => {
                let o = (addr & 0x01FF_FFFF) as usize;
                if o + 1 < self.rom.len() {
                    rd16(&self.rom, o)
                } else {
                    (addr >> 1) as u16
                }
            }
            0x0E | 0x0F => {
                let b = self.sram[(addr & 0x7FFF) as usize] as u16;
                b * 0x101
            }
            _ => 0,
        }
    }

    #[inline]
    pub fn read32(&mut self, addr: u32) -> u32 {
        if self.watch != 0 && addr & !3 == self.watch {
            *self.watch_hits.entry(self.pc).or_insert(0) += 1;
        }
        let addr = addr & !3;
        match addr >> 24 {
            0x00 => {
                if addr < 0x4000 {
                    rd32(&self.bios, addr as usize)
                } else {
                    0
                }
            }
            0x02 => rd32(&self.ewram, (addr & 0x3FFFF) as usize),
            0x03 => rd32(&self.iwram, (addr & 0x7FFF) as usize),
            0x04 => self.io_read16(addr) as u32 | (self.io_read16(addr + 2) as u32) << 16,
            0x05 => rd32(&self.pal, (addr & 0x3FF) as usize),
            0x06 => rd32(&self.vram, vram_off(addr)),
            0x07 => rd32(&self.oam, (addr & 0x3FF) as usize),
            0x08..=0x0D => {
                let o = (addr & 0x01FF_FFFF) as usize;
                if o + 3 < self.rom.len() {
                    rd32(&self.rom, o)
                } else {
                    let lo = (addr >> 1) & 0xFFFF;
                    lo | ((lo + 1) << 16)
                }
            }
            0x0E | 0x0F => {
                let b = self.sram[(addr & 0x7FFF) as usize] as u32;
                b * 0x0101_0101
            }
            _ => 0,
        }
    }

    #[inline]
    pub fn write8(&mut self, addr: u32, v: u8) {
        if self.watch_w != 0 && (addr & !3 == self.watch_w || (self.watch_w == 1 && (0x0600_0000..0x0601_8000).contains(&addr))) {
            *self.watch_hits.entry(self.pc | 1).or_insert(0) += 1;
        }
        if self.wlog.is_some() {
            let a = addr & !(1 - 1);
            let old = self.read8(a) as u32;
            self.wlog.as_mut().unwrap().push((a, 1, old, v as u32));
        }
        match addr >> 24 {
            0x02 => self.ewram[(addr & 0x3FFFF) as usize] = v,
            0x03 => self.iwram[(addr & 0x7FFF) as usize] = v,
            0x04 => self.io_write8(addr, v),
            0x05 => {
                let o = (addr & 0x3FE) as usize;
                self.pal[o] = v;
                self.pal[o + 1] = v;
            }
            0x06 => {
                // byte writes to BG VRAM duplicate; OBJ VRAM ignored
                let o = vram_off(addr) & !1;
                let bg_limit = if self.io16(0) & 7 >= 3 { 0x14000 } else { 0x10000 };
                if o < bg_limit {
                    self.vram[o] = v;
                    self.vram[o + 1] = v;
                }
            }
            0x07 => {}
            0x0E | 0x0F => {
                self.sram[(addr & 0x7FFF) as usize] = v;
                self.sram_dirty = true;
            }
            _ => {}
        }
    }

    #[inline]
    pub fn write16(&mut self, addr: u32, v: u16) {
        if self.watch_w != 0 && (addr & !3 == self.watch_w || (self.watch_w == 1 && (0x0600_0000..0x0601_8000).contains(&addr))) {
            *self.watch_hits.entry(self.pc | 1).or_insert(0) += 1;
        }
        if self.wlog.is_some() {
            let a = addr & !(2 - 1);
            let old = self.read16(a) as u32;
            self.wlog.as_mut().unwrap().push((a, 2, old, v as u32));
        }
        let addr = addr & !1;
        match addr >> 24 {
            0x02 => wr16(&mut self.ewram, (addr & 0x3FFFF) as usize, v),
            0x03 => wr16(&mut self.iwram, (addr & 0x7FFF) as usize, v),
            0x04 => self.io_write16(addr, v),
            0x05 => wr16(&mut self.pal, (addr & 0x3FF) as usize, v),
            0x06 => wr16(&mut self.vram, vram_off(addr), v),
            0x07 => wr16(&mut self.oam, (addr & 0x3FF) as usize, v),
            0x0E | 0x0F => {
                self.sram[(addr & 0x7FFF) as usize] = (v >> ((addr & 1) * 8)) as u8;
                self.sram_dirty = true;
            }
            _ => {}
        }
    }

    #[inline]
    pub fn write32(&mut self, addr: u32, v: u32) {
        if self.watch_w != 0 && (addr & !3 == self.watch_w || (self.watch_w == 1 && (0x0600_0000..0x0601_8000).contains(&addr))) {
            *self.watch_hits.entry(self.pc | 1).or_insert(0) += 1;
        }
        if self.wlog.is_some() {
            let a = addr & !(4 - 1);
            let old = self.read32(a) as u32;
            self.wlog.as_mut().unwrap().push((a, 4, old, v as u32));
        }
        let addr = addr & !3;
        match addr >> 24 {
            0x02 => wr32(&mut self.ewram, (addr & 0x3FFFF) as usize, v),
            0x03 => wr32(&mut self.iwram, (addr & 0x7FFF) as usize, v),
            0x04 => self.io_write32(addr, v),
            0x05 => wr32(&mut self.pal, (addr & 0x3FF) as usize, v),
            0x06 => wr32(&mut self.vram, vram_off(addr), v),
            0x07 => wr32(&mut self.oam, (addr & 0x3FF) as usize, v),
            0x0E | 0x0F => {
                self.sram[(addr & 0x7FFF) as usize] = (v >> ((addr & 3) * 8)) as u8;
                self.sram_dirty = true;
            }
            _ => {}
        }
    }

    /// LDR semantics (misaligned rotate).
    #[inline]
    pub fn ld32(&mut self, addr: u32) -> u32 {
        let v = self.read32(addr);
        v.rotate_right((addr & 3) * 8)
    }
    #[inline]
    pub fn ld16u(&mut self, addr: u32) -> u32 {
        let v = self.read16(addr) as u32;
        if addr & 1 != 0 {
            v.rotate_right(8)
        } else {
            v
        }
    }
    #[inline]
    pub fn ld16s(&mut self, addr: u32) -> u32 {
        if addr & 1 != 0 {
            self.read8(addr) as i8 as i32 as u32
        } else {
            self.read16(addr) as i16 as i32 as u32
        }
    }
}
