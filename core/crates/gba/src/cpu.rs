pub const MODE_USR: u32 = 0x10;
pub const MODE_FIQ: u32 = 0x11;
pub const MODE_IRQ: u32 = 0x12;
pub const MODE_SVC: u32 = 0x13;
pub const MODE_ABT: u32 = 0x17;
pub const MODE_UND: u32 = 0x1B;
pub const MODE_SYS: u32 = 0x1F;

#[derive(Clone)]
pub struct Cpu {
    pub r: [u32; 16],
    pub n: bool,
    pub z: bool,
    pub c: bool,
    pub v: bool,
    pub i: bool,
    pub f: bool,
    pub t: bool,
    pub mode: u32,
    pub spsr: u32,
    // banks: index 0 usr/sys, 1 fiq, 2 irq, 3 svc, 4 abt, 5 und
    bank_sp: [u32; 6],
    bank_lr: [u32; 6],
    bank_spsr: [u32; 6],
    fiq_r8: [u32; 5],
    usr_r8: [u32; 5],
}

fn bank_of(mode: u32) -> usize {
    match mode {
        MODE_FIQ => 1,
        MODE_IRQ => 2,
        MODE_SVC => 3,
        MODE_ABT => 4,
        MODE_UND => 5,
        _ => 0,
    }
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            r: [0; 16],
            n: false,
            z: false,
            c: false,
            v: false,
            i: true,
            f: true,
            t: false,
            mode: MODE_SYS,
            spsr: 0,
            bank_sp: [0; 6],
            bank_lr: [0; 6],
            bank_spsr: [0; 6],
            fiq_r8: [0; 5],
            usr_r8: [0; 5],
        }
    }

    pub fn cpsr(&self) -> u32 {
        (self.n as u32) << 31
            | (self.z as u32) << 30
            | (self.c as u32) << 29
            | (self.v as u32) << 28
            | (self.i as u32) << 7
            | (self.f as u32) << 6
            | (self.t as u32) << 5
            | self.mode
    }

    pub fn set_flags_from(&mut self, v: u32) {
        self.n = v & (1 << 31) != 0;
        self.z = v & (1 << 30) != 0;
        self.c = v & (1 << 29) != 0;
        self.v = v & (1 << 28) != 0;
    }

    /// Full CPSR write (mode switch included).
    pub fn set_cpsr(&mut self, v: u32) {
        self.set_flags_from(v);
        self.i = v & 0x80 != 0;
        self.f = v & 0x40 != 0;
        self.t = v & 0x20 != 0;
        let m = v & 0x1F;
        let m = if matches!(m, MODE_USR | MODE_FIQ | MODE_IRQ | MODE_SVC | MODE_ABT | MODE_UND | MODE_SYS) { m } else { MODE_SYS };
        self.set_mode(m);
    }

    pub fn set_mode(&mut self, new: u32) {
        let old = self.mode;
        if old == new {
            return;
        }
        let ob = bank_of(old);
        let nb = bank_of(new);
        if ob != nb {
            self.bank_sp[ob] = self.r[13];
            self.bank_lr[ob] = self.r[14];
            self.bank_spsr[ob] = self.spsr;
            self.r[13] = self.bank_sp[nb];
            self.r[14] = self.bank_lr[nb];
            self.spsr = self.bank_spsr[nb];
            if ob == 1 {
                self.fiq_r8.copy_from_slice(&self.r[8..13]);
                let u = self.usr_r8;
                self.r[8..13].copy_from_slice(&u);
            } else if nb == 1 {
                self.usr_r8.copy_from_slice(&self.r[8..13]);
                let f = self.fiq_r8;
                self.r[8..13].copy_from_slice(&f);
            }
        }
        self.mode = new;
    }

    /// Reads a user-mode register (for LDM/STM with S bit).
    pub fn user_reg(&self, i: usize) -> u32 {
        let b = bank_of(self.mode);
        if b == 0 {
            return self.r[i];
        }
        match i {
            13 => self.bank_sp[0],
            14 => self.bank_lr[0],
            8..=12 if b == 1 => self.usr_r8[i - 8],
            _ => self.r[i],
        }
    }

    pub fn set_user_reg(&mut self, i: usize, v: u32) {
        let b = bank_of(self.mode);
        if b == 0 {
            self.r[i] = v;
            return;
        }
        match i {
            13 => self.bank_sp[0] = v,
            14 => self.bank_lr[0] = v,
            8..=12 if b == 1 => self.usr_r8[i - 8] = v,
            _ => self.r[i] = v,
        }
    }

    #[inline]
    pub fn cond(&self, c: u32) -> bool {
        match c {
            0x0 => self.z,
            0x1 => !self.z,
            0x2 => self.c,
            0x3 => !self.c,
            0x4 => self.n,
            0x5 => !self.n,
            0x6 => self.v,
            0x7 => !self.v,
            0x8 => self.c && !self.z,
            0x9 => !self.c || self.z,
            0xA => self.n == self.v,
            0xB => self.n != self.v,
            0xC => !self.z && self.n == self.v,
            0xD => self.z || self.n != self.v,
            0xE => true,
            _ => false,
        }
    }

    #[inline]
    pub fn set_nz(&mut self, v: u32) {
        self.n = (v as i32) < 0;
        self.z = v == 0;
    }

    #[inline]
    pub fn add_flags(&mut self, a: u32, b: u32, carry: bool, setf: bool) -> u32 {
        let (r1, c1) = a.overflowing_add(b);
        let (r, c2) = r1.overflowing_add(carry as u32);
        if setf {
            self.set_nz(r);
            self.c = c1 || c2;
            self.v = ((a ^ r) & (b ^ r)) >> 31 != 0;
        }
        r
    }

    /// a - b - !carry  (SUB uses carry=true)
    #[inline]
    pub fn sub_flags(&mut self, a: u32, b: u32, carry: bool, setf: bool) -> u32 {
        let r = a.wrapping_sub(b).wrapping_sub(!carry as u32);
        if setf {
            self.set_nz(r);
            self.c = (a as u64) >= (b as u64) + (!carry as u64);
            self.v = ((a ^ b) & (a ^ r)) >> 31 != 0;
        }
        r
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

/// Barrel shifter. `ty`: 0 LSL, 1 LSR, 2 ASR, 3 ROR. `imm` form treats amount 0 specially.
#[inline]
pub fn shift_imm(cpu: &Cpu, ty: u32, v: u32, amt: u32) -> (u32, bool) {
    match ty {
        0 => {
            if amt == 0 {
                (v, cpu.c)
            } else {
                (v << amt, (v >> (32 - amt)) & 1 != 0)
            }
        }
        1 => {
            if amt == 0 {
                (0, v >> 31 != 0)
            } else {
                (v >> amt, (v >> (amt - 1)) & 1 != 0)
            }
        }
        2 => {
            if amt == 0 {
                let c = v >> 31 != 0;
                (if c { 0xFFFF_FFFF } else { 0 }, c)
            } else {
                (((v as i32) >> amt) as u32, (v >> (amt - 1)) & 1 != 0)
            }
        }
        _ => {
            if amt == 0 {
                // RRX
                (((cpu.c as u32) << 31) | (v >> 1), v & 1 != 0)
            } else {
                let r = v.rotate_right(amt);
                (r, r >> 31 != 0)
            }
        }
    }
}

/// Shift by register amount (only low byte used).
#[inline]
pub fn shift_reg(cpu: &Cpu, ty: u32, v: u32, amt: u32) -> (u32, bool) {
    let amt = amt & 0xFF;
    if amt == 0 {
        return (v, cpu.c);
    }
    match ty {
        0 => {
            if amt < 32 {
                (v << amt, (v >> (32 - amt)) & 1 != 0)
            } else if amt == 32 {
                (0, v & 1 != 0)
            } else {
                (0, false)
            }
        }
        1 => {
            if amt < 32 {
                (v >> amt, (v >> (amt - 1)) & 1 != 0)
            } else if amt == 32 {
                (0, v >> 31 != 0)
            } else {
                (0, false)
            }
        }
        2 => {
            if amt < 32 {
                (((v as i32) >> amt) as u32, (v >> (amt - 1)) & 1 != 0)
            } else {
                let c = v >> 31 != 0;
                (if c { 0xFFFF_FFFF } else { 0 }, c)
            }
        }
        _ => {
            let a = amt & 31;
            if a == 0 {
                (v, v >> 31 != 0)
            } else {
                let r = v.rotate_right(a);
                (r, r >> 31 != 0)
            }
        }
    }
}

impl Cpu {
    pub fn to_words(&self) -> Vec<u32> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.r);
        v.push(self.cpsr());
        v.push(self.spsr);
        v.extend_from_slice(&self.bank_sp);
        v.extend_from_slice(&self.bank_lr);
        v.extend_from_slice(&self.bank_spsr);
        v.extend_from_slice(&self.fiq_r8);
        v.extend_from_slice(&self.usr_r8);
        v
    }
    pub fn from_words(w: &[u32]) -> Cpu {
        let mut c = Cpu::new();
        c.r.copy_from_slice(&w[0..16]);
        let cpsr = w[16];
        c.set_flags_from(cpsr);
        c.i = cpsr & 0x80 != 0;
        c.f = cpsr & 0x40 != 0;
        c.t = cpsr & 0x20 != 0;
        c.mode = cpsr & 0x1F;
        c.spsr = w[17];
        c.bank_sp.copy_from_slice(&w[18..24]);
        c.bank_lr.copy_from_slice(&w[24..30]);
        c.bank_spsr.copy_from_slice(&w[30..36]);
        c.fiq_r8.copy_from_slice(&w[36..41]);
        c.usr_r8.copy_from_slice(&w[41..46]);
        c
    }
}
