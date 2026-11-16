//! ARM (32-bit) instruction semantics. ARM code in FE7 lives mostly in IWRAM
//! (interrupt handler, sound mixer), so it is executed by this interpreter.
use crate::cpu::{shift_imm, shift_reg, MODE_SYS, MODE_USR};
use crate::Gba;

impl Gba {
    #[inline]
    fn arm_reg(&self, r: usize) -> u32 {
        self.cpu.r[r]
    }

    fn arm_set_reg(&mut self, r: usize, v: u32) {
        if r == 15 {
            self.write_pc(v);
        } else {
            self.cpu.r[r] = v;
        }
    }

    pub fn arm_exec(&mut self, op: u32, a: u32) {
        if !self.cpu.cond(op >> 28) {
            return;
        }
        if op & 0x0FFF_FFF0 == 0x012F_FF10 {
            let v = self.arm_reg((op & 15) as usize);
            self.bx(v);
            self.cycles += 2;
            return;
        }
        match (op >> 25) & 7 {
            0 | 1 => {
                if op & 0x0FC0_00F0 == 0x0000_0090 {
                    self.arm_mul(op)
                } else if op & 0x0F80_00F0 == 0x0080_0090 {
                    self.arm_mull(op)
                } else if op & 0x0FB0_0FF0 == 0x0100_0090 {
                    self.arm_swp(op)
                } else if op & 0x0E00_0090 == 0x0000_0090 && op & 0x60 != 0 {
                    self.arm_halfword(op)
                } else if op & 0x0FBF_0FFF == 0x010F_0000 {
                    let v = if op & (1 << 22) != 0 { self.cpu.spsr } else { self.cpu.cpsr() };
                    self.cpu.r[((op >> 12) & 15) as usize] = v;
                } else if op & 0x0DB0_F000 == 0x0120_F000 {
                    self.arm_msr(op)
                } else {
                    self.arm_dp(op)
                }
            }
            2 | 3 => {
                if op & 0x0200_0010 == 0x0200_0010 {
                    self.undefined(a);
                } else {
                    self.arm_sdt(op)
                }
            }
            4 => self.arm_block(op),
            5 => {
                let off = (((op & 0x00FF_FFFF) << 8) as i32 >> 6) as u32;
                if op & (1 << 24) != 0 {
                    self.cpu.r[14] = a.wrapping_add(4);
                }
                self.pc = a.wrapping_add(8).wrapping_add(off) & !3;
                self.cycles += 2;
            }
            7 if op & (1 << 24) != 0 => self.swi((op >> 16) & 0xFF),
            _ => self.undefined(a),
        }
    }

    fn arm_dp(&mut self, op: u32) {
        let opc = (op >> 21) & 15;
        let s = op & (1 << 20) != 0;
        let rn = ((op >> 16) & 15) as usize;
        let rd = ((op >> 12) & 15) as usize;
        let mut rn_v = self.arm_reg(rn);
        let (op2, sc) = if op & (1 << 25) != 0 {
            let imm = op & 0xFF;
            let rot = ((op >> 8) & 15) * 2;
            let v = imm.rotate_right(rot);
            (v, if rot == 0 { self.cpu.c } else { v >> 31 != 0 })
        } else {
            let rm = (op & 15) as usize;
            let ty = (op >> 5) & 3;
            if op & 0x10 != 0 {
                // register-specified shift: PC reads as +12
                let mut rm_v = self.arm_reg(rm);
                if rm == 15 {
                    rm_v = rm_v.wrapping_add(4);
                }
                if rn == 15 {
                    rn_v = rn_v.wrapping_add(4);
                }
                let amt = self.arm_reg(((op >> 8) & 15) as usize);
                self.cycles += 1;
                shift_reg(&self.cpu, ty, rm_v, amt)
            } else {
                shift_imm(&self.cpu, ty, self.arm_reg(rm), (op >> 7) & 31)
            }
        };
        let setf = s && rd != 15;
        let logical = |g: &mut Gba, r: u32| {
            if setf {
                g.cpu.set_nz(r);
                g.cpu.c = sc;
            }
            r
        };
        let res: Option<u32> = match opc {
            0x0 => Some(logical(self, rn_v & op2)),
            0x1 => Some(logical(self, rn_v ^ op2)),
            0x2 => Some(self.cpu.sub_flags(rn_v, op2, true, setf)),
            0x3 => Some(self.cpu.sub_flags(op2, rn_v, true, setf)),
            0x4 => Some(self.cpu.add_flags(rn_v, op2, false, setf)),
            0x5 => {
                let c = self.cpu.c;
                Some(self.cpu.add_flags(rn_v, op2, c, setf))
            }
            0x6 => {
                let c = self.cpu.c;
                Some(self.cpu.sub_flags(rn_v, op2, c, setf))
            }
            0x7 => {
                let c = self.cpu.c;
                Some(self.cpu.sub_flags(op2, rn_v, c, setf))
            }
            0x8 => {
                logical(self, rn_v & op2);
                None
            }
            0x9 => {
                logical(self, rn_v ^ op2);
                None
            }
            0xA => {
                self.cpu.sub_flags(rn_v, op2, true, setf);
                None
            }
            0xB => {
                self.cpu.add_flags(rn_v, op2, false, setf);
                None
            }
            0xC => Some(logical(self, rn_v | op2)),
            0xD => Some(logical(self, op2)),
            0xE => Some(logical(self, rn_v & !op2)),
            _ => Some(logical(self, !op2)),
        };
        // test ops with S and rd=15 (legacy TEQP etc.) are not used.
        if let Some(r) = res {
            if rd == 15 {
                if s {
                    let spsr = self.cpu.spsr;
                    self.cpu.set_cpsr(spsr);
                }
                if self.cpu.t {
                    self.pc = r & !1;
                } else {
                    self.pc = r & !3;
                }
            } else {
                self.cpu.r[rd] = r;
            }
        } else if (opc & 0xC) == 0x8 && rd == 15 && s {
            let spsr = self.cpu.spsr;
            self.cpu.set_cpsr(spsr);
        }
    }

    fn arm_msr(&mut self, op: u32) {
        let spsr = op & (1 << 22) != 0;
        let v = if op & (1 << 25) != 0 {
            (op & 0xFF).rotate_right(((op >> 8) & 15) * 2)
        } else {
            self.arm_reg((op & 15) as usize)
        };
        let mut mask = 0u32;
        if op & (1 << 19) != 0 {
            mask |= 0xFF00_0000;
        }
        if op & (1 << 16) != 0 && self.cpu.mode != MODE_USR {
            mask |= 0xFF;
        }
        if spsr {
            self.cpu.spsr = (self.cpu.spsr & !mask) | (v & mask);
        } else {
            let cur = self.cpu.cpsr();
            let nv = (cur & !mask) | (v & mask);
            self.cpu.set_cpsr(nv);
        }
    }

    fn arm_mul(&mut self, op: u32) {
        let rd = ((op >> 16) & 15) as usize;
        let rn = ((op >> 12) & 15) as usize;
        let rs = ((op >> 8) & 15) as usize;
        let rm = (op & 15) as usize;
        let mut r = self.cpu.r[rm].wrapping_mul(self.cpu.r[rs]);
        if op & (1 << 21) != 0 {
            r = r.wrapping_add(self.cpu.r[rn]);
        }
        self.cpu.r[rd] = r;
        if op & (1 << 20) != 0 {
            self.cpu.set_nz(r);
        }
        self.cycles += 3;
    }

    fn arm_mull(&mut self, op: u32) {
        let hi = ((op >> 16) & 15) as usize;
        let lo = ((op >> 12) & 15) as usize;
        let rs = ((op >> 8) & 15) as usize;
        let rm = (op & 15) as usize;
        let signed = op & (1 << 22) != 0;
        let mut r: u64 = if signed {
            (self.cpu.r[rm] as i32 as i64).wrapping_mul(self.cpu.r[rs] as i32 as i64) as u64
        } else {
            (self.cpu.r[rm] as u64).wrapping_mul(self.cpu.r[rs] as u64)
        };
        if op & (1 << 21) != 0 {
            r = r.wrapping_add(((self.cpu.r[hi] as u64) << 32) | self.cpu.r[lo] as u64);
        }
        self.cpu.r[lo] = r as u32;
        self.cpu.r[hi] = (r >> 32) as u32;
        if op & (1 << 20) != 0 {
            self.cpu.n = (r >> 63) != 0;
            self.cpu.z = r == 0;
        }
        self.cycles += 4;
    }

    fn arm_swp(&mut self, op: u32) {
        let rn = ((op >> 16) & 15) as usize;
        let rd = ((op >> 12) & 15) as usize;
        let rm = (op & 15) as usize;
        let addr = self.cpu.r[rn];
        if op & (1 << 22) != 0 {
            let old = self.read8(addr) as u32;
            self.write8(addr, self.cpu.r[rm] as u8);
            self.cpu.r[rd] = old;
        } else {
            let old = self.ld32(addr);
            self.write32(addr & !3, self.cpu.r[rm]);
            self.cpu.r[rd] = old;
        }
        self.cycles += 3;
    }

    fn arm_sdt(&mut self, op: u32) {
        let p = op & (1 << 24) != 0;
        let u = op & (1 << 23) != 0;
        let b = op & (1 << 22) != 0;
        let w = op & (1 << 21) != 0;
        let l = op & (1 << 20) != 0;
        let rn = ((op >> 16) & 15) as usize;
        let rd = ((op >> 12) & 15) as usize;
        let off = if op & (1 << 25) == 0 {
            op & 0xFFF
        } else {
            let rm = (op & 15) as usize;
            shift_imm(&self.cpu, (op >> 5) & 3, self.cpu.r[rm], (op >> 7) & 31).0
        };
        let base = self.arm_reg(rn);
        let offa = if u { base.wrapping_add(off) } else { base.wrapping_sub(off) };
        let addr = if p { offa } else { base };
        self.cycles += 2;
        if l {
            let v = if b { self.read8(addr) as u32 } else { self.ld32(addr) };
            if !p || w {
                self.cpu.r[rn] = offa;
            }
            self.arm_set_reg(rd, v);
        } else {
            let mut v = self.arm_reg(rd);
            if rd == 15 {
                v = v.wrapping_add(4);
            }
            if b {
                self.write8(addr, v as u8);
            } else {
                self.write32(addr & !3, v);
            }
            if !p || w {
                self.cpu.r[rn] = offa;
            }
        }
    }

    fn arm_halfword(&mut self, op: u32) {
        let p = op & (1 << 24) != 0;
        let u = op & (1 << 23) != 0;
        let i = op & (1 << 22) != 0;
        let w = op & (1 << 21) != 0;
        let l = op & (1 << 20) != 0;
        let rn = ((op >> 16) & 15) as usize;
        let rd = ((op >> 12) & 15) as usize;
        let sh = (op >> 5) & 3;
        let off = if i { ((op >> 4) & 0xF0) | (op & 0xF) } else { self.cpu.r[(op & 15) as usize] };
        let base = self.arm_reg(rn);
        let offa = if u { base.wrapping_add(off) } else { base.wrapping_sub(off) };
        let addr = if p { offa } else { base };
        self.cycles += 2;
        if l {
            let v = match sh {
                1 => self.ld16u(addr),
                2 => self.read8(addr) as i8 as i32 as u32,
                _ => self.ld16s(addr),
            };
            if !p || w {
                self.cpu.r[rn] = offa;
            }
            self.arm_set_reg(rd, v);
        } else {
            let v = self.arm_reg(rd);
            self.write16(addr & !1, v as u16);
            if !p || w {
                self.cpu.r[rn] = offa;
            }
        }
    }

    fn arm_block(&mut self, op: u32) {
        let p = op & (1 << 24) != 0;
        let u = op & (1 << 23) != 0;
        let s = op & (1 << 22) != 0;
        let w = op & (1 << 21) != 0;
        let l = op & (1 << 20) != 0;
        let rn = ((op >> 16) & 15) as usize;
        let rlist = op & 0xFFFF;
        let n = rlist.count_ones();
        let base = self.cpu.r[rn];
        let (mut addr, new_base) = if u {
            (if p { base.wrapping_add(4) } else { base }, base.wrapping_add(n * 4))
        } else {
            let start = base.wrapping_sub(n * 4);
            (if p { start } else { start.wrapping_add(4) }, start)
        };
        self.cycles += n as u64 + 1;
        let user = s && !(l && rlist & 0x8000 != 0);
        let user = user && self.cpu.mode != MODE_USR && self.cpu.mode != MODE_SYS;
        if l {
            if w {
                self.cpu.r[rn] = new_base;
            }
            for i in 0..16 {
                if rlist & (1 << i) != 0 {
                    let v = self.read32(addr & !3);
                    addr = addr.wrapping_add(4);
                    if i == 15 {
                        if s {
                            let spsr = self.cpu.spsr;
                            self.cpu.set_cpsr(spsr);
                        }
                        if self.cpu.t {
                            self.pc = v & !1;
                        } else {
                            self.pc = v & !3;
                        }
                    } else if user {
                        self.cpu.set_user_reg(i, v);
                    } else {
                        self.cpu.r[i] = v;
                    }
                }
            }
        } else {
            let first = rlist.trailing_zeros() as usize;
            for i in 0..16 {
                if rlist & (1 << i) != 0 {
                    let mut v = if user { self.cpu.user_reg(i) } else { self.cpu.r[i] };
                    if i == 15 {
                        v = v.wrapping_add(4);
                    }
                    if i == rn && i != first && w {
                        v = new_base;
                    }
                    self.write32(addr & !3, v);
                    addr = addr.wrapping_add(4);
                }
            }
            if w {
                self.cpu.r[rn] = new_base;
            }
        }
    }
}
