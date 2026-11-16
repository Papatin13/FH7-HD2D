//! Sound: Direct Sound FIFO A/B plus the four PSG channels, mixed at 32768 Hz.
use crate::Gba;
use std::collections::VecDeque;

pub const SAMPLE_RATE: u32 = 32768;
const CYCLES_PER_SAMPLE: u64 = 512;
const DUTY: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

#[derive(Default, Clone)]
pub struct Psg {
    pub on: bool,
    pub length: u32,
    pub length_en: bool,
    pub vol: u8,
    pub env_dir: bool,
    pub env_period: u8,
    pub env_timer: u8,
    pub freq: u32,
    pub phase: u32,
    pub timer: i64,
    pub duty: usize,
    pub sweep_timer: u8,
    pub lfsr: u16,
}

pub struct Apu {
    pub fifo: [VecDeque<i8>; 2],
    pub cur: [i8; 2],
    pub ch: [Psg; 4],
    pub sample_acc: u64,
    pub seq_acc: u64,
    pub seq_step: u32,
    pub out: Vec<f32>,
    pub wave: [u8; 32],
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            fifo: [VecDeque::with_capacity(32), VecDeque::with_capacity(32)],
            cur: [0; 2],
            ch: Default::default(),
            sample_acc: 0,
            seq_acc: 0,
            seq_step: 0,
            out: Vec::with_capacity(4096),
            wave: [0; 32],
        }
    }
    pub fn fifo_push(&mut self, f: usize, v: i8) {
        if self.fifo[f].len() < 32 {
            self.fifo[f].push_back(v);
        }
    }
    /// Pops a sample; returns true when the FIFO wants a refill.
    pub fn fifo_pop(&mut self, f: usize) -> bool {
        if let Some(v) = self.fifo[f].pop_front() {
            self.cur[f] = v;
        }
        self.fifo[f].len() <= 16
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Gba {
    pub fn apu_reg_write(&mut self, off: usize, v: u16) {
        match off {
            0x62 | 0x68 => {
                let c = &mut self.apu.ch[if off == 0x62 { 0 } else { 1 }];
                c.length = 64 - (v & 63) as u32;
                c.duty = ((v >> 6) & 3) as usize;
            }
            0x64 | 0x6C => {
                let i = if off == 0x64 { 0 } else { 1 };
                let env = self.io16(if i == 0 { 0x62 } else { 0x68 });
                let sweep = ((self.io16(0x60) >> 4) & 7) as u8;
                let c = &mut self.apu.ch[i];
                c.freq = (v & 0x7FF) as u32;
                c.length_en = v & 0x4000 != 0;
                if v & 0x8000 != 0 {
                    c.on = true;
                    if c.length == 0 {
                        c.length = 64;
                    }
                    c.vol = (env >> 12) as u8;
                    c.env_dir = env & 0x800 != 0;
                    c.env_period = ((env >> 8) & 7) as u8;
                    c.env_timer = c.env_period;
                    c.sweep_timer = sweep;
                    if env & 0xF800 == 0 {
                        c.on = false;
                    }
                }
            }
            0x70 => {
                if v & 0x80 == 0 {
                    self.apu.ch[2].on = false;
                }
            }
            0x72 => {
                self.apu.ch[2].length = 256 - (v & 0xFF) as u32;
            }
            0x74 => {
                let c = &mut self.apu.ch[2];
                c.freq = (v & 0x7FF) as u32;
                c.length_en = v & 0x4000 != 0;
                if v & 0x8000 != 0 {
                    c.on = self.io[0x70] & 0x80 != 0;
                    if c.length == 0 {
                        c.length = 256;
                    }
                    c.phase = 0;
                    // latch the playing bank
                    let bank = ((self.io[0x70] >> 6) & 1) as usize;
                    let _ = bank;
                }
            }
            0x78 => {
                self.apu.ch[3].length = 64 - (v & 63) as u32;
            }
            0x7C => {
                let env = self.io16(0x78);
                let c = &mut self.apu.ch[3];
                c.length_en = v & 0x4000 != 0;
                if v & 0x8000 != 0 {
                    c.on = env & 0xF800 != 0;
                    if c.length == 0 {
                        c.length = 64;
                    }
                    c.vol = (env >> 12) as u8;
                    c.env_dir = env & 0x800 != 0;
                    c.env_period = ((env >> 8) & 7) as u8;
                    c.env_timer = c.env_period;
                    c.lfsr = 0x7FFF;
                }
            }
            0x82 => {
                if v & 0x0800 != 0 {
                    self.apu.fifo[0].clear();
                    self.apu.cur[0] = 0;
                    self.set_io16(0x82, v & !0x0800);
                }
                if v & 0x8000 != 0 {
                    self.apu.fifo[1].clear();
                    self.apu.cur[1] = 0;
                    let nv = self.io16(0x82) & !0x8000;
                    self.set_io16(0x82, nv);
                }
            }
            0x90..=0x9F => {
                // wave RAM: CPU accesses the bank not selected for playback
                let bank = ((self.io[0x70] >> 6) & 1) as usize;
                let o = (1 - bank) * 16 + (off - 0x90);
                self.apu.wave[o] = v as u8;
                self.apu.wave[o + 1] = (v >> 8) as u8;
            }
            _ => {}
        }
    }

    fn frame_seq(&mut self) {
        let step = self.apu.seq_step;
        self.apu.seq_step = (step + 1) & 7;
        if step % 2 == 0 {
            for c in self.apu.ch.iter_mut() {
                if c.length_en && c.length > 0 {
                    c.length -= 1;
                    if c.length == 0 {
                        c.on = false;
                    }
                }
            }
        }
        if step == 2 || step == 6 {
            let sw = self.io16(0x60);
            let period = ((sw >> 4) & 7) as u8;
            let c = &mut self.apu.ch[0];
            if period != 0 && c.on {
                if c.sweep_timer > 0 {
                    c.sweep_timer -= 1;
                }
                if c.sweep_timer == 0 {
                    c.sweep_timer = period;
                    let sh = sw & 7;
                    let d = c.freq >> sh;
                    let nf = if sw & 8 != 0 { c.freq.saturating_sub(d) } else { c.freq + d };
                    if nf > 2047 {
                        c.on = false;
                    } else if sh != 0 {
                        c.freq = nf;
                    }
                }
            }
        }
        if step == 7 {
            for i in [0usize, 1, 3] {
                let c = &mut self.apu.ch[i];
                if c.env_period != 0 {
                    if c.env_timer > 0 {
                        c.env_timer -= 1;
                    }
                    if c.env_timer == 0 {
                        c.env_timer = c.env_period;
                        if c.env_dir && c.vol < 15 {
                            c.vol += 1;
                        } else if !c.env_dir && c.vol > 0 {
                            c.vol -= 1;
                        }
                    }
                }
            }
        }
    }

    fn psg_sample(&mut self, i: usize, cycles: i64) -> f32 {
        let wave_ctl = self.io[0x70];
        let wave_vol = self.io[0x73] >> 5;
        let noise_reg = self.io16(0x7C);
        let wave = self.apu.wave;
        let c = &mut self.apu.ch[i];
        if !c.on {
            return 0.0;
        }
        match i {
            0 | 1 => {
                let period = 16 * (2048 - c.freq as i64).max(1);
                c.timer -= cycles;
                while c.timer <= 0 {
                    c.timer += period;
                    c.phase = (c.phase + 1) & 7;
                }
                let hi = DUTY[c.duty][c.phase as usize] != 0;
                (if hi { 1.0 } else { -1.0 }) * c.vol as f32 / 15.0
            }
            2 => {
                let period = 8 * (2048 - c.freq as i64).max(1);
                c.timer -= cycles;
                let two = wave_ctl & 0x20 != 0;
                let len = if two { 64 } else { 32 };
                while c.timer <= 0 {
                    c.timer += period;
                    c.phase = (c.phase + 1) % len;
                }
                let bank = ((wave_ctl >> 6) & 1) as usize;
                let idx = if two { c.phase as usize } else { bank * 32 + c.phase as usize } % 64;
                let byte = wave[idx / 2];
                let s = if idx % 2 == 0 { byte >> 4 } else { byte & 15 } as f32;
                let v = match wave_vol {
                    0 => 0.0,
                    1 => 1.0,
                    2 => 0.5,
                    3 => 0.25,
                    _ => 0.75,
                };
                let v = if wave_vol & 4 != 0 { 0.75 } else { v };
                (s / 7.5 - 1.0) * v
            }
            _ => {
                let r = (noise_reg & 7) as f64;
                let s = ((noise_reg >> 4) & 15) as u32;
                let div = if r == 0.0 { 0.5 } else { r };
                let freq = 524288.0 / div / (1u64 << (s + 1)) as f64;
                let period = (16777216.0 / freq).max(1.0) as i64;
                c.timer -= cycles;
                let narrow = noise_reg & 8 != 0;
                while c.timer <= 0 {
                    c.timer += period;
                    let bit = (c.lfsr ^ (c.lfsr >> 1)) & 1;
                    c.lfsr = (c.lfsr >> 1) | (bit << 14);
                    if narrow {
                        c.lfsr = (c.lfsr & !(1 << 6)) | (bit << 6);
                    }
                }
                (if c.lfsr & 1 == 0 { 1.0 } else { -1.0 }) * c.vol as f32 / 15.0
            }
        }
    }

    pub fn apu_tick(&mut self, delta: u64) {
        self.apu.seq_acc += delta;
        while self.apu.seq_acc >= 32768 {
            self.apu.seq_acc -= 32768;
            self.frame_seq();
        }
        self.apu.sample_acc += delta;
        while self.apu.sample_acc >= CYCLES_PER_SAMPLE {
            self.apu.sample_acc -= CYCLES_PER_SAMPLE;
            let master = self.io[0x84] & 0x80 != 0;
            let cl = self.io16(0x80);
            let ch_ = self.io16(0x82);
            let mut l = 0.0f32;
            let mut r = 0.0f32;
            let mut psg = [0.0f32; 4];
            for i in 0..4 {
                psg[i] = self.psg_sample(i, CYCLES_PER_SAMPLE as i64);
            }
            if master {
                let vr = ((cl & 7) + 1) as f32 / 8.0;
                let vl = (((cl >> 4) & 7) + 1) as f32 / 8.0;
                let pv = match ch_ & 3 {
                    0 => 0.25,
                    1 => 0.5,
                    _ => 1.0,
                };
                for (i, s) in psg.iter().enumerate() {
                    if cl & (0x100 << i) != 0 {
                        r += s * vr * pv * 0.25;
                    }
                    if cl & (0x1000 << i) != 0 {
                        l += s * vl * pv * 0.25;
                    }
                }
                for f in 0..2 {
                    let full = ch_ & (4 << f) != 0;
                    let s = self.apu.cur[f] as f32 / 128.0 * if full { 1.0 } else { 0.5 };
                    if ch_ & (0x100 << (f * 4)) != 0 {
                        r += s;
                    }
                    if ch_ & (0x200 << (f * 4)) != 0 {
                        l += s;
                    }
                }
            }
            if self.apu.out.len() < SAMPLE_RATE as usize {
                self.apu.out.push((l * 0.5).clamp(-1.0, 1.0));
                self.apu.out.push((r * 0.5).clamp(-1.0, 1.0));
            }
        }
    }
}
