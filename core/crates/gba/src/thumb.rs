//! Thumb instruction semantics. Every helper here is used both by the
//! interpreter (`thumb_exec`) and by recompiled code with constant arguments.
//! Convention: before an instruction executes, `g.pc` is already the address of
//! the following instruction and `cpu.r[15]` holds the pipeline value (addr+4)
//! when the instruction reads r15.
use crate::cpu::shift_imm;
use crate::Gba;

#[inline(always)]
fn sext(v: u32, bits: u32) -> i32 {
    ((v << (32 - bits)) as i32) >> (32 - bits)
}

impl Gba {
    #[inline(always)]
    pub fn t_shift_imm(&mut self, op: u32, rd: usize, rs: usize, amt: u32) {
        let (r, c) = shift_imm(&self.cpu, op, self.cpu.r[rs], amt);
        self.cpu.r[rd] = r;
        self.cpu.c = c;
        self.cpu.set_nz(r);
    }

    #[inline(always)]
    pub fn t_add_reg(&mut self, rd: usize, rs: usize, rn: usize) {
        let (a, b) = (self.cpu.r[rs], self.cpu.r[rn]);
        self.cpu.r[rd] = self.cpu.add_flags(a, b, false, true);
    }
    #[inline(always)]
    pub fn t_sub_reg(&mut self, rd: usize, rs: usize, rn: usize) {
        let (a, b) = (self.cpu.r[rs], self.cpu.r[rn]);
        self.cpu.r[rd] = self.cpu.sub_flags(a, b, true, true);
    }
    #[inline(always)]
    pub fn t_add_imm3(&mut self, rd: usize, rs: usize, imm: u32) {
        let a = self.cpu.r[rs];
        self.cpu.r[rd] = self.cpu.add_flags(a, imm, false, true);
    }
    #[inline(always)]
    pub fn t_sub_imm3(&mut self, rd: usize, rs: usize, imm: u32) {
        let a = self.cpu.r[rs];
        self.cpu.r[rd] = self.cpu.sub_flags(a, imm, true, true);
    }
    #[inline(always)]
    pub fn t_mov_imm(&mut self, rd: usize, imm: u32) {
        self.cpu.r[rd] = imm;
        self.cpu.set_nz(imm);
    }
    #[inline(always)]
    pub fn t_cmp_imm(&mut self, rd: usize, imm: u32) {
        let a = self.cpu.r[rd];
        self.cpu.sub_flags(a, imm, true, true);
    }
    #[inline(always)]
    pub fn t_add_imm8(&mut self, rd: usize, imm: u32) {
        let a = self.cpu.r[rd];
        self.cpu.r[rd] = self.cpu.add_flags(a, imm, false, true);
    }
    #[inline(always)]
    pub fn t_sub_imm8(&mut self, rd: usize, imm: u32) {
        let a = self.cpu.r[rd];
        self.cpu.r[rd] = self.cpu.sub_flags(a, imm, true, true);
    }

    #[inline(always)]
    pub fn t_alu(&mut self, op: u32, rd: usize, rs: usize) {
        let a = self.cpu.r[rd];
        let b = self.cpu.r[rs];
        let c = &mut self.cpu;
        match op {
            0x0 => {
                let r = a & b;
                c.r[rd] = r;
                c.set_nz(r);
            }
            0x1 => {
                let r = a ^ b;
                c.r[rd] = r;
                c.set_nz(r);
            }
            0x2 | 0x3 | 0x4 | 0x7 => {
                let ty = match op {
                    0x2 => 0,
                    0x3 => 1,
                    0x4 => 2,
                    _ => 3,
                };
                let (r, cf) = crate::cpu::shift_reg(c, ty, a, b);
                c.r[rd] = r;
                c.c = cf;
                c.set_nz(r);
            }
            0x5 => {
                let cin = c.c;
                c.r[rd] = c.add_flags(a, b, cin, true);
            }
            0x6 => {
                let cin = c.c;
                c.r[rd] = c.sub_flags(a, b, cin, true);
            }
            0x8 => {
                c.set_nz(a & b);
            }
            0x9 => {
                c.r[rd] = c.sub_flags(0, b, true, true);
            }
            0xA => {
                c.sub_flags(a, b, true, true);
            }
            0xB => {
                c.add_flags(a, b, false, true);
            }
            0xC => {
                let r = a | b;
                c.r[rd] = r;
                c.set_nz(r);
            }
            0xD => {
                let r = a.wrapping_mul(b);
                c.r[rd] = r;
                c.set_nz(r);
                self.cycles += 2;
            }
            0xE => {
                let r = a & !b;
                c.r[rd] = r;
                c.set_nz(r);
            }
            _ => {
                let r = !b;
                c.r[rd] = r;
                c.set_nz(r);
            }
        }
    }

    /// Hi register ops: 0 ADD, 1 CMP, 2 MOV, 3 BX.
    #[inline(always)]
    pub fn t_hireg(&mut self, op: u32, rd: usize, rs: usize) {
        let b = self.cpu.r[rs];
        match op {
            0 => {
                let r = self.cpu.r[rd].wrapping_add(b);
                if rd == 15 {
                    self.write_pc(r);
                } else {
                    self.cpu.r[rd] = r;
                }
            }
            1 => {
                let a = self.cpu.r[rd];
                self.cpu.sub_flags(a, b, true, true);
            }
            2 => {
                if rd == 15 {
                    self.write_pc(b);
                } else {
                    self.cpu.r[rd] = b;
                }
            }
            _ => {
                self.bx(b);
            }
        }
    }

    #[inline(always)]
    pub fn t_ldr_lit(&mut self, rd: usize, addr: u32) {
        self.cpu.r[rd] = self.read32(addr);
        self.cycles += 2;
    }

    /// op: 0 STR, 1 STRB, 2 LDR, 3 LDRB
    #[inline(always)]
    pub fn t_ldst(&mut self, op: u32, rd: usize, addr: u32) {
        self.cycles += 2;
        match op {
            0 => self.write32(addr & !3, self.cpu.r[rd]),
            1 => self.write8(addr, self.cpu.r[rd] as u8),
            2 => self.cpu.r[rd] = self.ld32(addr),
            _ => self.cpu.r[rd] = self.read8(addr) as u32,
        }
    }
    #[inline(always)]
    pub fn t_ldst_reg(&mut self, op: u32, rd: usize, rb: usize, ro: usize) {
        let a = self.cpu.r[rb].wrapping_add(self.cpu.r[ro]);
        self.t_ldst(op, rd, a);
    }
    #[inline(always)]
    pub fn t_ldst_imm(&mut self, op: u32, rd: usize, rb: usize, off: u32) {
        let a = self.cpu.r[rb].wrapping_add(off);
        self.t_ldst(op, rd, a);
    }
    /// op: 0 STRH, 1 LDRH, 2 LDSB, 3 LDSH
    #[inline(always)]
    pub fn t_ldst_h(&mut self, op: u32, rd: usize, addr: u32) {
        self.cycles += 2;
        match op {
            0 => self.write16(addr & !1, self.cpu.r[rd] as u16),
            1 => self.cpu.r[rd] = self.ld16u(addr),
            2 => self.cpu.r[rd] = self.read8(addr) as i8 as i32 as u32,
            _ => self.cpu.r[rd] = self.ld16s(addr),
        }
    }
    #[inline(always)]
    pub fn t_ldst_h_reg(&mut self, op: u32, rd: usize, rb: usize, ro: usize) {
        let a = self.cpu.r[rb].wrapping_add(self.cpu.r[ro]);
        self.t_ldst_h(op, rd, a);
    }
    #[inline(always)]
    pub fn t_ldst_h_imm(&mut self, op: u32, rd: usize, rb: usize, off: u32) {
        let a = self.cpu.r[rb].wrapping_add(off);
        self.t_ldst_h(op, rd, a);
    }

    #[inline(always)]
    pub fn t_set_reg(&mut self, rd: usize, v: u32) {
        self.cpu.r[rd] = v;
    }
    #[inline(always)]
    pub fn t_add_sp_rd(&mut self, rd: usize, imm: u32) {
        self.cpu.r[rd] = self.cpu.r[13].wrapping_add(imm);
    }
    #[inline(always)]
    pub fn t_add_sp(&mut self, imm: i32) {
        self.cpu.r[13] = self.cpu.r[13].wrapping_add(imm as u32);
    }

    #[inline(always)]
    pub fn t_push(&mut self, rlist: u32, lr: bool) {
        let n = rlist.count_ones() + lr as u32;
        let mut a = self.cpu.r[13].wrapping_sub(n * 4);
        self.cpu.r[13] = a;
        self.cycles += n as u64 + 1;
        for i in 0..8 {
            if rlist & (1 << i) != 0 {
                self.write32(a & !3, self.cpu.r[i]);
                a = a.wrapping_add(4);
            }
        }
        if lr {
            self.write32(a & !3, self.cpu.r[14]);
        }
    }
    #[inline(always)]
    pub fn t_pop(&mut self, rlist: u32, pc: bool) {
        let mut a = self.cpu.r[13];
        self.cycles += rlist.count_ones() as u64 + 2;
        for i in 0..8 {
            if rlist & (1 << i) != 0 {
                self.cpu.r[i] = self.read32(a & !3);
                a = a.wrapping_add(4);
            }
        }
        if pc {
            let v = self.read32(a & !3);
            a = a.wrapping_add(4);
            self.write_pc(v);
        }
        self.cpu.r[13] = a;
    }
    #[inline(always)]
    pub fn t_stmia(&mut self, rb: usize, rlist: u32) {
        let mut a = self.cpu.r[rb];
        let n = rlist.count_ones();
        let first = rlist.trailing_zeros() as usize;
        let final_addr = a.wrapping_add(n * 4);
        self.cycles += n as u64 + 1;
        for i in 0..8 {
            if rlist & (1 << i) != 0 {
                let v = if i == rb && i != first { final_addr } else { self.cpu.r[i] };
                self.write32(a & !3, v);
                a = a.wrapping_add(4);
            }
        }
        self.cpu.r[rb] = final_addr;
    }
    #[inline(always)]
    pub fn t_ldmia(&mut self, rb: usize, rlist: u32) {
        let mut a = self.cpu.r[rb];
        self.cycles += rlist.count_ones() as u64 + 2;
        for i in 0..8 {
            if rlist & (1 << i) != 0 {
                self.cpu.r[i] = self.read32(a & !3);
                a = a.wrapping_add(4);
            }
        }
        if rlist & (1 << rb) == 0 {
            self.cpu.r[rb] = a;
        }
    }

    /// Second half of BL: jumps to target with lr = return|1.
    #[inline(always)]
    pub fn t_bl(&mut self, target: u32, ret: u32) {
        self.cpu.r[14] = ret | 1;
        self.pc = target & !1;
        self.cycles += 3;
    }

    pub fn thumb_exec(&mut self, op: u16, a: u32) {
        let op = op as u32;
        let rd = (op & 7) as usize;
        let rs = ((op >> 3) & 7) as usize;
        match op >> 11 {
            0..=2 => self.t_shift_imm(op >> 11, rd, rs, (op >> 6) & 31),
            3 => {
                let rn = ((op >> 6) & 7) as usize;
                match (op >> 9) & 3 {
                    0 => self.t_add_reg(rd, rs, rn),
                    1 => self.t_sub_reg(rd, rs, rn),
                    2 => self.t_add_imm3(rd, rs, rn as u32),
                    _ => self.t_sub_imm3(rd, rs, rn as u32),
                }
            }
            4..=7 => {
                let rd = ((op >> 8) & 7) as usize;
                let imm = op & 0xFF;
                match (op >> 11) & 3 {
                    0 => self.t_mov_imm(rd, imm),
                    1 => self.t_cmp_imm(rd, imm),
                    2 => self.t_add_imm8(rd, imm),
                    _ => self.t_sub_imm8(rd, imm),
                }
            }
            8 => {
                if op & 0x400 == 0 {
                    self.t_alu((op >> 6) & 15, rd, rs);
                } else {
                    let rd = rd | (((op >> 7) & 1) << 3) as usize;
                    let rs = ((op >> 3) & 15) as usize;
                    self.t_hireg((op >> 8) & 3, rd, rs);
                }
            }
            9 => {
                let rd = ((op >> 8) & 7) as usize;
                let addr = (a.wrapping_add(4) & !2).wrapping_add((op & 0xFF) * 4);
                self.t_ldr_lit(rd, addr);
            }
            10 | 11 => {
                let ro = ((op >> 6) & 7) as usize;
                if op & 0x200 == 0 {
                    let l = (op >> 11) & 1;
                    let b = (op >> 10) & 1;
                    self.t_ldst_reg((l << 1) | b, rd, rs, ro);
                } else {
                    let h = (op >> 11) & 1;
                    let s = (op >> 10) & 1;
                    self.t_ldst_h_reg((s << 1) | h, rd, rs, ro);
                }
            }
            12..=15 => {
                let b = (op >> 12) & 1;
                let l = (op >> 11) & 1;
                let off5 = (op >> 6) & 31;
                let off = if b == 1 { off5 } else { off5 * 4 };
                self.t_ldst_imm((l << 1) | b, rd, rs, off);
            }
            16 | 17 => {
                let l = (op >> 11) & 1;
                let off = ((op >> 6) & 31) * 2;
                self.t_ldst_h_imm(l, rd, rs, off);
            }
            18 | 19 => {
                let l = (op >> 11) & 1;
                let rd = ((op >> 8) & 7) as usize;
                let off = (op & 0xFF) * 4;
                self.t_ldst_imm(l << 1, rd, 13, off);
            }
            20 => {
                let rd = ((op >> 8) & 7) as usize;
                let v = (a.wrapping_add(4) & !2).wrapping_add((op & 0xFF) * 4);
                self.t_set_reg(rd, v);
            }
            21 => {
                let rd = ((op >> 8) & 7) as usize;
                self.t_add_sp_rd(rd, (op & 0xFF) * 4);
            }
            22 | 23 => match (op >> 8) & 0xF {
                0 => {
                    let imm = ((op & 0x7F) * 4) as i32;
                    self.t_add_sp(if op & 0x80 != 0 { -imm } else { imm });
                }
                4 | 5 => self.t_push(op & 0xFF, op & 0x100 != 0),
                0xC | 0xD => self.t_pop(op & 0xFF, op & 0x100 != 0),
                _ => self.undefined(a),
            },
            24 => self.t_stmia(((op >> 8) & 7) as usize, op & 0xFF),
            25 => self.t_ldmia(((op >> 8) & 7) as usize, op & 0xFF),
            26 | 27 => {
                let cond = (op >> 8) & 0xF;
                if cond == 0xF {
                    self.swi(op & 0xFF);
                } else if cond == 0xE {
                    self.undefined(a);
                } else if self.cpu.cond(cond) {
                    let off = sext(op & 0xFF, 8) * 2;
                    self.pc = a.wrapping_add(4).wrapping_add(off as u32);
                    self.cycles += 2;
                }
            }
            28 => {
                let off = sext(op & 0x7FF, 11) * 2;
                self.pc = a.wrapping_add(4).wrapping_add(off as u32);
                self.cycles += 2;
            }
            29 => self.undefined(a),
            30 => {
                let off = sext(op & 0x7FF, 11) << 12;
                self.cpu.r[14] = a.wrapping_add(4).wrapping_add(off as u32);
            }
            _ => {
                let target = self.cpu.r[14].wrapping_add((op & 0x7FF) << 1);
                self.t_bl(target, a.wrapping_add(2));
            }
        }
    }

    pub fn undefined(&mut self, a: u32) {
        eprintln!("undefined instruction at {:08X} (thumb={})", a, self.cpu.t);
    }
}
