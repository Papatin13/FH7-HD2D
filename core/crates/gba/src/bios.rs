//! BIOS replacement: a tiny ARM IRQ trampoline plus high-level SWI emulation.
use crate::Gba;
use std::f64::consts::PI;

pub fn bios_image() -> Vec<u8> {
    let mut b = vec![0u8; 0x4000];
    let code: [u32; 6] = [
        0xE92D500F, // 0x18 stmfd sp!,{r0-r3,r12,lr}
        0xE3A00301, // mov r0,#0x04000000
        0xE28FE000, // add lr,pc,#0
        0xE510F004, // ldr pc,[r0,#-4]
        0xE8BD500F, // ldmfd sp!,{r0-r3,r12,lr}
        0xE25EF004, // subs pc,lr,#4
    ];
    for (i, w) in code.iter().enumerate() {
        b[0x18 + i * 4..0x1C + i * 4].copy_from_slice(&w.to_le_bytes());
    }
    b
}

impl Gba {
    pub fn swi(&mut self, n: u32) {
        let r = self.cpu.r;
        self.cycles += 10;
        match n {
            0x00 => {
                self.reset();
            }
            0x01 => self.register_ram_reset(r[0]),
            0x02 | 0x03 => self.halted = true,
            0x04 => self.intr_wait(r[0], r[1] as u16),
            0x05 => self.intr_wait(1, 1),
            0x06 => self.div(r[0] as i32, r[1] as i32),
            0x07 => self.div(r[1] as i32, r[0] as i32),
            0x08 => self.cpu.r[0] = (r[0] as f64).sqrt() as u32,
            0x09 => {
                self.cpu.r[0] = arctan(r[0] as i32) as u32;
            }
            0x0A => {
                self.cpu.r[0] = arctan2(r[0] as i32 as i16 as i32, r[1] as i32 as i16 as i32) as u32;
            }
            0x0B => {
                if self.watch_w == 1 && (0x0600_0000..0x0601_8000).contains(&r[1]) {
                    eprintln!("cpuset {:08x} -> {:08x} {:x} lr {:08x}", r[0], r[1], r[2], self.cpu.r[14]);
                }
                self.cpu_set(r[0], r[1], r[2])
            }
            0x0C => {
                if self.watch_w == 1 && (0x0600_0000..0x0601_8000).contains(&r[1]) {
                    eprintln!("fastset {:08x} -> {:08x} {:x} lr {:08x}", r[0], r[1], r[2], self.cpu.r[14]);
                }
                self.cpu_fast_set(r[0], r[1], r[2])
            }
            0x0D => self.cpu.r[0] = 0xBAAE_187F,
            0x0E => self.bg_affine_set(r[0], r[1], r[2]),
            0x0F => self.obj_affine_set(r[0], r[1], r[2], r[3]),
            0x10 => self.bit_unpack(r[0], r[1], r[2]),
            0x11 => self.lz77(r[0], r[1], false),
            0x12 => self.lz77(r[0], r[1], true),
            0x13 => self.huff(r[0], r[1]),
            0x14 => self.rl(r[0], r[1], false),
            0x15 => self.rl(r[0], r[1], true),
            0x16 | 0x17 => self.diff_unfilter(r[0], r[1], false, n == 0x17),
            0x18 => self.diff_unfilter(r[0], r[1], true, true),
            0x19 => {}
            0x1F => {
                let freq = self.read32(r[0].wrapping_add(4)) as f64;
                let e = (180.0 - r[1] as f64 - r[2] as f64 / 256.0) / 12.0;
                self.cpu.r[0] = (freq / 2f64.powf(e)) as u32;
            }
            _ => eprintln!("unhandled SWI {:02X} at {:08X}", n, self.pc),
        }
    }

    fn intr_wait(&mut self, discard: u32, mask: u16) {
        if discard != 0 {
            let f = self.read16(0x0300_7FF8);
            self.write16(0x0300_7FF8, f & !mask);
        }
        self.set_io16(0x208, 1);
        self.intr_wait = mask;
        self.intr_wait_pc = self.pc;
    }

    fn div(&mut self, a: i32, b: i32) {
        if b == 0 {
            self.cpu.r[0] = if a < 0 { 0xFFFF_FFFF } else { 1 };
            self.cpu.r[1] = a as u32;
            self.cpu.r[3] = 1;
            return;
        }
        let q = a.wrapping_div(b);
        let m = a.wrapping_rem(b);
        self.cpu.r[0] = q as u32;
        self.cpu.r[1] = m as u32;
        self.cpu.r[3] = q.unsigned_abs();
    }

    fn register_ram_reset(&mut self, f: u32) {
        if f & 1 != 0 {
            self.ewram.fill(0);
        }
        if f & 2 != 0 {
            self.iwram[..0x7E00].fill(0);
        }
        if f & 4 != 0 {
            self.pal.fill(0);
        }
        if f & 8 != 0 {
            self.vram.fill(0);
        }
        if f & 16 != 0 {
            self.oam.fill(0);
        }
    }

    fn cpu_set(&mut self, mut src: u32, mut dst: u32, ctl: u32) {
        let count = ctl & 0x1F_FFFF;
        let fill = ctl & (1 << 24) != 0;
        if ctl & (1 << 26) != 0 {
            let v = self.read32(src);
            for _ in 0..count {
                let x = if fill { v } else { self.read32(src) };
                self.write32(dst, x);
                if !fill {
                    src = src.wrapping_add(4);
                }
                dst = dst.wrapping_add(4);
            }
        } else {
            let v = self.read16(src);
            for _ in 0..count {
                let x = if fill { v } else { self.read16(src) };
                self.write16(dst, x);
                if !fill {
                    src = src.wrapping_add(2);
                }
                dst = dst.wrapping_add(2);
            }
        }
        self.cycles += count as u64;
    }

    fn cpu_fast_set(&mut self, mut src: u32, mut dst: u32, ctl: u32) {
        let count = ((ctl & 0x1F_FFFF) + 7) & !7;
        let fill = ctl & (1 << 24) != 0;
        let v = self.read32(src);
        for _ in 0..count {
            let x = if fill { v } else { self.read32(src) };
            self.write32(dst, x);
            if !fill {
                src = src.wrapping_add(4);
            }
            dst = dst.wrapping_add(4);
        }
        self.cycles += count as u64 / 2;
    }

    fn bg_affine_set(&mut self, mut src: u32, mut dst: u32, count: u32) {
        for _ in 0..count {
            let ox = self.read32(src) as i32 as f64 / 256.0;
            let oy = self.read32(src + 4) as i32 as f64 / 256.0;
            let cx = self.read16(src + 8) as i16 as f64;
            let cy = self.read16(src + 10) as i16 as f64;
            let sx = self.read16(src + 12) as i16 as f64 / 256.0;
            let sy = self.read16(src + 14) as i16 as f64 / 256.0;
            let th = (self.read16(src + 16) >> 8) as f64 / 128.0 * PI;
            src = src.wrapping_add(20);
            let (s, c) = (q14(th.sin()), q14(th.cos()));
            let a = c * sx;
            let b = -s * sx;
            let cc = s * sy;
            let d = c * sy;
            let rx = ox - (a * cx + b * cy);
            let ry = oy - (cc * cx + d * cy);
            self.write16(dst, (a * 256.0) as i32 as u16);
            self.write16(dst + 2, (b * 256.0) as i32 as u16);
            self.write16(dst + 4, (cc * 256.0) as i32 as u16);
            self.write16(dst + 6, (d * 256.0) as i32 as u16);
            self.write32(dst + 8, (rx * 256.0) as i32 as u32);
            self.write32(dst + 12, (ry * 256.0) as i32 as u32);
            dst = dst.wrapping_add(16);
        }
    }

    fn obj_affine_set(&mut self, mut src: u32, mut dst: u32, count: u32, stride: u32) {
        for _ in 0..count {
            let sx = self.read16(src) as i16 as f64 / 256.0;
            let sy = self.read16(src + 2) as i16 as f64 / 256.0;
            let th = (self.read16(src + 4) >> 8) as f64 / 128.0 * PI;
            src = src.wrapping_add(8);
            let (s, c) = (q14(th.sin()), q14(th.cos()));
            let vals = [c * sx, -s * sx, s * sy, c * sy];
            for v in vals {
                self.write16(dst, (v * 256.0) as i32 as u16);
                dst = dst.wrapping_add(stride);
            }
        }
    }

    fn bit_unpack(&mut self, mut src: u32, mut dst: u32, info: u32) {
        let len = self.read16(info) as u32;
        let sw = self.read8(info + 2) as u32;
        let dw = self.read8(info + 3) as u32;
        let ofs = self.read32(info + 4);
        let zero_flag = ofs & 0x8000_0000 != 0;
        let ofs = ofs & 0x7FFF_FFFF;
        if sw == 0 || dw == 0 {
            return;
        }
        let mut out: u32 = 0;
        let mut out_bits = 0;
        for _ in 0..len {
            let b = self.read8(src) as u32;
            src = src.wrapping_add(1);
            let mut bit = 0;
            while bit < 8 {
                let mut v = (b >> bit) & ((1 << sw) - 1);
                if v != 0 || zero_flag {
                    v = v.wrapping_add(ofs);
                }
                out |= (v & if dw == 32 { 0xFFFF_FFFF } else { (1 << dw) - 1 }) << out_bits;
                out_bits += dw;
                if out_bits >= 32 {
                    self.write32(dst, out);
                    dst = dst.wrapping_add(4);
                    out = 0;
                    out_bits = 0;
                }
                bit += sw;
            }
        }
    }

    fn write_out(&mut self, dst: u32, data: &[u8], vram: bool) {
        if vram {
            let mut i = 0;
            while i < data.len() {
                let lo = data[i] as u16;
                let hi = *data.get(i + 1).unwrap_or(&0) as u16;
                self.write16(dst + i as u32, lo | hi << 8);
                i += 2;
            }
        } else {
            for (i, &b) in data.iter().enumerate() {
                self.write8(dst + i as u32, b);
            }
        }
        self.cycles += data.len() as u64 / 2;
    }

    fn lz77(&mut self, mut src: u32, dst: u32, vram: bool) {
        let hdr = self.read32(src);
        src += 4;
        let size = (hdr >> 8) as usize;
        let mut out: Vec<u8> = Vec::with_capacity(size);
        while out.len() < size {
            let flags = self.read8(src);
            src += 1;
            for i in 0..8 {
                if out.len() >= size {
                    break;
                }
                if flags & (0x80 >> i) != 0 {
                    let b1 = self.read8(src) as usize;
                    let b2 = self.read8(src + 1) as usize;
                    src += 2;
                    let len = (b1 >> 4) + 3;
                    let disp = ((b1 & 15) << 8 | b2) + 1;
                    for _ in 0..len {
                        let v = if disp <= out.len() { out[out.len() - disp] } else { 0 };
                        out.push(v);
                    }
                } else {
                    out.push(self.read8(src));
                    src += 1;
                }
            }
        }
        out.truncate(size);
        self.write_out(dst, &out, vram);
    }

    fn rl(&mut self, mut src: u32, dst: u32, vram: bool) {
        let hdr = self.read32(src);
        src += 4;
        let size = (hdr >> 8) as usize;
        let mut out = Vec::with_capacity(size);
        while out.len() < size {
            let f = self.read8(src);
            src += 1;
            if f & 0x80 != 0 {
                let n = (f & 0x7F) as usize + 3;
                let v = self.read8(src);
                src += 1;
                for _ in 0..n {
                    out.push(v);
                }
            } else {
                let n = (f & 0x7F) as usize + 1;
                for _ in 0..n {
                    out.push(self.read8(src));
                    src += 1;
                }
            }
        }
        out.truncate(size);
        self.write_out(dst, &out, vram);
    }

    fn huff(&mut self, src: u32, dst: u32) {
        let hdr = self.read32(src);
        let bits = hdr & 15;
        let size = (hdr >> 8) as usize;
        let tree_size = self.read8(src + 4) as u32;
        let tree = src + 5;
        let mut data = src + 4 + (tree_size + 1) * 2;
        let mut out: Vec<u8> = Vec::with_capacity(size);
        let mut acc: u32 = 0;
        let mut acc_bits = 0;
        let mut node = tree;
        let mut nodev = self.read8(node);
        while out.len() < size {
            let word = self.read32(data);
            data += 4;
            for b in (0..32).rev() {
                let bit = (word >> b) & 1;
                let off = (nodev & 0x3F) as u32;
                let next = (node & !1) + off * 2 + 2 + bit;
                let is_leaf = if bit == 0 { nodev & 0x80 != 0 } else { nodev & 0x40 != 0 };
                if is_leaf {
                    let v = self.read8(next) as u32;
                    acc |= v << acc_bits;
                    acc_bits += bits;
                    node = tree;
                    nodev = self.read8(node);
                    if acc_bits == 32 {
                        out.extend_from_slice(&acc.to_le_bytes());
                        acc = 0;
                        acc_bits = 0;
                        if out.len() >= size {
                            break;
                        }
                    }
                } else {
                    node = next;
                    nodev = self.read8(node);
                }
            }
        }
        out.truncate(size);
        self.write_out(dst, &out, false);
    }

    fn diff_unfilter(&mut self, src: u32, dst: u32, sixteen: bool, vram: bool) {
        let hdr = self.read32(src);
        let size = (hdr >> 8) as usize;
        let mut out = Vec::with_capacity(size);
        if sixteen {
            let mut acc: u16 = 0;
            let mut i = 0;
            while i < size {
                acc = acc.wrapping_add(self.read16(src + 4 + i as u32));
                out.extend_from_slice(&acc.to_le_bytes());
                i += 2;
            }
        } else {
            let mut acc: u8 = 0;
            for i in 0..size {
                acc = acc.wrapping_add(self.read8(src + 4 + i as u32));
                out.push(acc);
            }
        }
        self.write_out(dst, &out, vram);
    }
}

/// Quantize to the BIOS's 1.14 fixed point sine table precision.
fn q14(v: f64) -> f64 {
    (v * 16384.0).round() / 16384.0
}

fn arctan(i: i32) -> i32 {
    let a = -((i as i64 * i as i64) >> 14) as i32;
    let mut b = ((0xA9 * a) >> 14) + 0x390;
    b = ((b * a) >> 14) + 0x91C;
    b = ((b * a) >> 14) + 0xFB6;
    b = ((b * a) >> 14) + 0x16AA;
    b = ((b * a) >> 14) + 0x2081;
    b = ((b * a) >> 14) + 0x3651;
    b = ((b * a) >> 14) + 0xA2F9;
    ((i as i64 * b as i64) >> 16) as i32
}

fn arctan2(x: i32, y: i32) -> u16 {
    if y == 0 {
        return if x >= 0 { 0 } else { 0x8000 };
    }
    if x == 0 {
        return if y >= 0 { 0x4000 } else { 0xC000 };
    }
    let r = if y >= 0 {
        if x >= 0 {
            if x >= y {
                arctan((y << 14) / x)
            } else {
                0x4000 - arctan((x << 14) / y)
            }
        } else if -x >= y {
            arctan((y << 14) / x) + 0x8000
        } else {
            0x4000 - arctan((x << 14) / y)
        }
    } else if x <= 0 {
        if -x > -y {
            arctan((y << 14) / x) + 0x8000
        } else {
            0xC000 - arctan((x << 14) / y)
        }
    } else if x >= -y {
        arctan((y << 14) / x) + 0x10000
    } else {
        0xC000 - arctan((x << 14) / y)
    };
    r as u16
}
