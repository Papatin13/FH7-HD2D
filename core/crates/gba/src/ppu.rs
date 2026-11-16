//! Scanline software renderer (modes 0-5, affine, sprites, windows, blending, mosaic).
use crate::Gba;

pub const W: usize = 240;
pub const H: usize = 160;
/// Maximum extra columns per side in widescreen mode.
pub const MAXE: usize = 96;
pub const MAXW: usize = W + 2 * MAXE;
/// Outer background pixels per side that get stretched in widescreen scenes.
pub const EDGE_BAND: usize = 48;
const TRANSPARENT: u32 = 0x8000_0000;
/// Side pixel outside the battle map (filled by the frontend).
pub const OUTSIDE: u32 = 0x4000_0000;

pub struct Ppu {
    pub frame: Vec<u32>,
    pub vcount: u32,
    pub in_hblank: bool,
    pub next_event: u64,
    pub ref_x: [i32; 2],
    pub ref_y: [i32; 2],
    /// Width of the current frame (240 + 2 * ext) and extra columns per side.
    pub width: usize,
    pub ext: usize,
    /// Columns left of the original 240 px picture (== ext unless shifted at map edges).
    pub ext_left: usize,
    /// Widescreen: the battle map is on screen this frame.
    pub wide_map: bool,
    /// Widescreen: movement/attack overlay entries for side columns (0 = off).
    pub wide_blue: u16,
    pub wide_red: u16,
    /// BGR555 -> 0xFFRRGGBB
    pub rgb_lut: Vec<u32>,
    /// Widescreen HUD: horizontal shift per 8x8 screen cell (30x20) and the
    /// shifted window boxes (x0, y0, x1, y1 in game pixels, shift).
    pub ui_shift: Vec<i16>,
    pub ui_boxes: Vec<(i32, i32, i32, i32, i32)>,
}

const OBJ_SIZE: [[(i32, i32); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

fn io16(io: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([io[o], io[o + 1]])
}
fn io32(io: &[u8], o: usize) -> u32 {
    io16(io, o) as u32 | (io16(io, o + 2) as u32) << 16
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            frame: vec![0xFF00_0000; MAXW * H],
            vcount: 0,
            in_hblank: false,
            next_event: crate::HBLANK_START,
            ref_x: [0; 2],
            ref_y: [0; 2],
            width: W,
            ext: 0,
            ext_left: 0,
            wide_map: false,
            wide_blue: 0,
            wide_red: 0,
            rgb_lut: (0..32768u32).map(to_rgb).collect(),
            ui_shift: vec![0; 600],
            ui_boxes: Vec::new(),
        }
    }

    pub fn reload_affine(&mut self, io: &[u8]) {
        for bg in 0..2 {
            let b = 0x28 + bg * 0x10;
            self.ref_x[bg] = ((io32(io, b) << 4) as i32) >> 4;
            self.ref_y[bg] = ((io32(io, b + 4) << 4) as i32) >> 4;
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn pal_color(pal: &[u8], idx: usize) -> u32 {
    (pal[idx * 2] as u32 | (pal[idx * 2 + 1] as u32) << 8) & 0x7FFF
}

impl Gba {
    fn render_text_bg(&self, bg: usize, y: u32, out: &mut [u32]) {
        let io = &self.io;
        let cnt = io16(io, 8 + bg * 2) as u32;
        let hofs = io16(io, 0x10 + bg * 4) as u32 & 0x1FF;
        let vofs = io16(io, 0x12 + bg * 4) as u32 & 0x1FF;
        let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
        let screen_base = ((cnt >> 8) & 31) as usize * 0x800;
        let bpp8 = cnt & 0x80 != 0;
        let size = cnt >> 14;
        let w = if size & 1 != 0 { 512 } else { 256 };
        let h = if size & 2 != 0 { 512 } else { 256 };
        let mosaic = cnt & 0x40 != 0;
        let mos = io16(io, 0x4C) as u32;
        let (mh, mv) = if mosaic { ((mos & 15) + 1, ((mos >> 4) & 15) + 1) } else { (1, 1) };
        let ly = y - y % mv;
        let yy = (ly + vofs) % h;
        let vram = &self.vram;
        if !mosaic && !bpp8 {
            // fast path: one tilemap fetch per 8 pixels
            let mut x: u32 = 0;
            while (x as usize) < W {
                let xx = (x + hofs) % w;
                let blk = match size {
                    0 => 0,
                    1 => xx / 256,
                    2 => yy / 256,
                    _ => (xx / 256) + (yy / 256) * 2,
                } as usize;
                let sb = screen_base + blk * 0x800 + (((yy % 256) / 8) * 64 + ((xx % 256) / 8) * 2) as usize;
                let run = (8 - (xx & 7)).min(W as u32 - x);
                if sb + 1 >= vram.len() {
                    for i in 0..run {
                        out[(x + i) as usize] = TRANSPARENT;
                    }
                    x += run;
                    continue;
                }
                let e = vram[sb] as u32 | (vram[sb + 1] as u32) << 8;
                let tile = (e & 0x3FF) as usize;
                let py = if e & 0x800 != 0 { 7 - (yy & 7) } else { yy & 7 };
                let a = char_base + tile * 32 + (py * 4) as usize;
                let row = if a + 3 < 0x10000 { u32::from_le_bytes([vram[a], vram[a + 1], vram[a + 2], vram[a + 3]]) } else { 0 };
                let palbase = ((e >> 12) * 16) as usize;
                let hflip = e & 0x400 != 0;
                for i in 0..run {
                    let px = (xx & 7) + i;
                    let px = if hflip { 7 - px } else { px };
                    let n = (row >> (px * 4)) & 15;
                    out[(x + i) as usize] = if n == 0 { TRANSPARENT } else { self.pal_rgb15[palbase + n as usize] as u32 };
                }
                x += run;
            }
            return;
        }
        for x in 0..W as u32 {
            let lx = x - x % mh;
            let xx = (lx + hofs) % w;
            let blk = match size {
                0 => 0,
                1 => xx / 256,
                2 => yy / 256,
                _ => (xx / 256) + (yy / 256) * 2,
            } as usize;
            let sb = screen_base + blk * 0x800 + (((yy % 256) / 8) * 64 + ((xx % 256) / 8) * 2) as usize;
            if sb + 1 >= vram.len() {
                out[x as usize] = TRANSPARENT;
                continue;
            }
            let e = vram[sb] as u32 | (vram[sb + 1] as u32) << 8;
            let tile = (e & 0x3FF) as usize;
            let mut px = xx % 8;
            let mut py = yy % 8;
            if e & 0x400 != 0 {
                px = 7 - px;
            }
            if e & 0x800 != 0 {
                py = 7 - py;
            }
            let c = if bpp8 {
                let a = char_base + tile * 64 + (py * 8 + px) as usize;
                if a >= 0x10000 {
                    0
                } else {
                    vram[a] as usize
                }
            } else {
                let a = char_base + tile * 32 + (py * 4 + px / 2) as usize;
                if a >= 0x10000 {
                    0
                } else {
                    let n = (vram[a] >> ((px & 1) * 4)) & 15;
                    if n == 0 {
                        0
                    } else {
                        ((e >> 12) * 16) as usize + n as usize
                    }
                }
            };
            out[x as usize] = if c == 0 { TRANSPARENT } else { pal_color(&self.pal, c) };
        }
    }

    fn render_affine_bg(&self, bg: usize, out: &mut [u32]) {
        let io = &self.io;
        let cnt = io16(io, 8 + bg * 2) as u32;
        let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
        let screen_base = ((cnt >> 8) & 31) as usize * 0x800;
        let size = 128i32 << (cnt >> 14);
        let wrap = cnt & 0x2000 != 0;
        let p = 0x20 + (bg - 2) * 0x10;
        let pa = io16(io, p) as i16 as i32;
        let pc = io16(io, p + 4) as i16 as i32;
        let mut tx = self.ppu.ref_x[bg - 2];
        let mut ty = self.ppu.ref_y[bg - 2];
        for x in 0..W {
            let mut ix = tx >> 8;
            let mut iy = ty >> 8;
            tx = tx.wrapping_add(pa);
            ty = ty.wrapping_add(pc);
            if wrap {
                ix = ix.rem_euclid(size);
                iy = iy.rem_euclid(size);
            } else if ix < 0 || iy < 0 || ix >= size || iy >= size {
                out[x] = TRANSPARENT;
                continue;
            }
            let m = screen_base + ((iy / 8) * (size / 8) + ix / 8) as usize;
            let tile = self.vram[m.min(0x17FFF)] as usize;
            let a = char_base + tile * 64 + ((iy % 8) * 8 + ix % 8) as usize;
            let c = if a < 0x10000 { self.vram[a] as usize } else { 0 };
            out[x] = if c == 0 { TRANSPARENT } else { pal_color(&self.pal, c) };
        }
    }

    fn render_bitmap_bg(&self, mode: u32, out: &mut [u32]) {
        let io = &self.io;
        let pa = io16(io, 0x20) as i16 as i32;
        let pc = io16(io, 0x24) as i16 as i32;
        let mut tx = self.ppu.ref_x[0];
        let mut ty = self.ppu.ref_y[0];
        let page = if io16(io, 0) & 0x10 != 0 { 0xA000 } else { 0 };
        let (w, h) = if mode == 5 { (160, 128) } else { (240, 160) };
        for x in 0..W {
            let ix = tx >> 8;
            let iy = ty >> 8;
            tx = tx.wrapping_add(pa);
            ty = ty.wrapping_add(pc);
            if ix < 0 || iy < 0 || ix >= w || iy >= h {
                out[x] = TRANSPARENT;
                continue;
            }
            let i = (iy * w + ix) as usize;
            out[x] = match mode {
                3 => (self.vram[i * 2] as u32 | (self.vram[i * 2 + 1] as u32) << 8) & 0x7FFF,
                4 => {
                    let c = self.vram[page + i] as usize;
                    if c == 0 {
                        TRANSPARENT
                    } else {
                        pal_color(&self.pal, c)
                    }
                }
                _ => {
                    let a = page + i * 2;
                    (self.vram[a] as u32 | (self.vram[a + 1] as u32) << 8) & 0x7FFF
                }
            };
        }
    }

    fn render_objs(&self, y: i32, color: &mut [u32], prio: &mut [u8], semi: &mut [bool], win: &mut [bool], e: usize, skip_world: bool) {
        let dispcnt = io16(&self.io, 0);
        let one_d = dispcnt & 0x40 != 0;
        let bitmap = dispcnt & 7 >= 3;
        let mos = io16(&self.io, 0x4C) as i32;
        let (omh, omv) = (((mos >> 8) & 15) + 1, ((mos >> 12) & 15) + 1);
        for i in 0..128 {
            let o = i * 8;
            let a0 = (self.oam[o] as u32) | (self.oam[o + 1] as u32) << 8;
            let a1 = (self.oam[o + 2] as u32) | (self.oam[o + 3] as u32) << 8;
            let a2 = (self.oam[o + 4] as u32) | (self.oam[o + 5] as u32) << 8;
            let affine = a0 & 0x100 != 0;
            if !affine && a0 & 0x200 != 0 {
                continue;
            }
            let mode = (a0 >> 10) & 3;
            if mode == 3 {
                continue;
            }
            let shape = (a0 >> 14) as usize;
            if shape == 3 {
                continue;
            }
            if skip_world && crate::hd::Hd::world_obj(a0, a1, a2) {
                continue;
            }
            if self.hd.hiding {
                let (w, h) = OBJ_SIZE[shape][(a1 >> 14) as usize];
                let mut oy = (a0 & 0xFF) as i32;
                if oy >= 160 {
                    oy -= 256;
                }
                let raw = (a1 & 0x1FF) as i32;
                let ox = if raw >= 256 { raw - 512 } else { raw };
                if self.talk_hides_obj(ox, oy, w, h, (a2 >> 10) & 3) {
                    continue;
                }
            }
            let (w, h) = OBJ_SIZE[shape][(a1 >> 14) as usize];
            let double = affine && a0 & 0x200 != 0;
            let (bw, bh) = if double { (w * 2, h * 2) } else { (w, h) };
            let mut oy = (a0 & 0xFF) as i32;
            if oy + bh > 256 {
                oy -= 256;
            }
            let mut iy = y - oy;
            if iy < 0 || iy >= bh {
                continue;
            }
            let mosaic = a0 & 0x1000 != 0;
            if mosaic {
                iy -= iy % omv;
            }
            let raw = (a1 & 0x1FF) as i32;
            // 9-bit X: in widescreen more of the range counts as "right of the screen"
            let thr = if self.ppu.wide_map { 256 + self.ppu.ext as i32 + 32 } else { 256 };
            let mut ox = if raw >= thr { raw - 512 } else { raw } + e as i32;
            if self.ppu.wide_map && (a2 >> 10) & 3 <= 1 && !self.ppu.ui_boxes.is_empty() {
                let cx = ox - e as i32 + w / 2;
                let cy = oy + h / 2;
                if (0..240).contains(&cx) && (0..160).contains(&cy) {
                    ox += self.ppu.ui_shift[(cy / 8) as usize * 30 + (cx / 8) as usize] as i32;
                }
            }
            let lw = color.len() as i32;
            // outside the battle map, sprites stay inside the original 240 px
            let (lo, hi) = if self.ppu.wide_map { (0, lw) } else { (e as i32, (e + W) as i32) };
            let bpp8 = a0 & 0x2000 != 0;
            let base_tile = (a2 & 0x3FF) as i32;
            let pr = ((a2 >> 10) & 3) as u8;
            let palbank = ((a2 >> 12) * 16) as usize;
            let (pa, pb, pc, pd) = if affine {
                let g = ((a1 >> 9) & 31) as usize * 32;
                let r = |k: usize| (self.oam[g + k] as u16 | (self.oam[g + k + 1] as u16) << 8) as i16 as i32;
                (r(6), r(14), r(22), r(30))
            } else {
                (0, 0, 0, 0)
            };
            for ix in 0..bw {
                let sx = ox + ix;
                if !(lo..hi).contains(&sx) {
                    continue;
                }
                let sxu = sx as usize;
                let mut lix = ix;
                if mosaic {
                    lix -= lix % omh;
                }
                let (tx, ty) = if affine {
                    let dx = lix - bw / 2;
                    let dy = iy - bh / 2;
                    let tx = ((pa * dx + pb * dy) >> 8) + w / 2;
                    let ty = ((pc * dx + pd * dy) >> 8) + h / 2;
                    if tx < 0 || ty < 0 || tx >= w || ty >= h {
                        continue;
                    }
                    (tx, ty)
                } else {
                    let tx = if a1 & 0x1000 != 0 { w - 1 - lix } else { lix };
                    let ty = if a1 & 0x2000 != 0 { h - 1 - iy } else { iy };
                    (tx, ty)
                };
                let tstep = if bpp8 { 2 } else { 1 };
                let tile = if one_d {
                    base_tile + (ty / 8) * (w / 8) * tstep + (tx / 8) * tstep
                } else {
                    (base_tile & if bpp8 { !1 } else { !0 }) + (ty / 8) * 32 + (tx / 8) * tstep
                } & 0x3FF;
                if bitmap && tile < 512 {
                    continue;
                }
                let ci = if bpp8 {
                    let a = 0x10000 + tile as usize * 32 + ((ty % 8) * 8 + tx % 8) as usize;
                    let c = self.vram[a & 0x17FFF] as usize;
                    if c == 0 {
                        continue;
                    }
                    0x100 + c
                } else {
                    let a = 0x10000 + tile as usize * 32 + ((ty % 8) * 4 + (tx % 8) / 2) as usize;
                    let n = (self.vram[a & 0x17FFF] >> ((tx & 1) * 4)) & 15;
                    if n == 0 {
                        continue;
                    }
                    0x100 + palbank + n as usize
                };
                if mode == 2 {
                    win[sxu] = true;
                    continue;
                }
                if color[sxu] & TRANSPARENT != 0 || pr < prio[sxu] {
                    color[sxu] = pal_color(&self.pal, ci);
                    prio[sxu] = pr;
                    semi[sxu] = mode == 1;
                }
            }
        }
    }

    pub fn render_line(&mut self, y: u32) {
        if y == 0 {
            let e = (self.wide_ext as usize).min(MAXE);
            self.ppu.ext = e;
            self.ppu.ext_left = e;
            self.ppu.width = W + 2 * e;
            if e > 0 || self.hd.on {
                self.wide_frame_setup();
            } else {
                self.ppu.wide_map = false;
            }
            if self.hd.on {
                self.hd_frame_start();
            } else {
                self.hd.map = false;
                self.hd.hiding = false;
            }
            if !self.ppu.wide_map {
                self.ppu.ui_boxes.clear();
                self.ppu.ui_shift.iter_mut().for_each(|v| *v = 0);
            }
        }
        let e = self.ppu.ext_left;
        let lw = self.ppu.width;
        for i in 0..256 {
            self.pal_rgb15[i] = (self.pal[i * 2] as u16 | (self.pal[i * 2 + 1] as u16) << 8) & 0x7FFF;
        }
        let dispcnt = self.io16(0) as u32;
        let row = y as usize * lw;
        if dispcnt & 0x80 != 0 {
            for x in 0..lw {
                self.ppu.frame[row + x] = 0xFFFF_FFFF;
            }
            return;
        }
        let mode = dispcnt & 7;
        let mut bgbuf = [[TRANSPARENT; MAXW]; 4];
        let mut outside = [false; MAXW];
        let mut bg_on = [false; 4];
        for bg in 0..4 {
            if dispcnt & (0x100 << bg) == 0 {
                continue;
            }
            match (mode, bg) {
                (0, _) | (1, 0) | (1, 1) => {
                    self.render_text_bg(bg, y, &mut bgbuf[bg][e..e + W]);
                    bg_on[bg] = true;
                }
                (1, 2) | (2, 2) | (2, 3) => {
                    self.render_affine_bg(bg, &mut bgbuf[bg][e..e + W]);
                    bg_on[bg] = true;
                }
                (3..=5, 2) => {
                    self.render_bitmap_bg(mode, &mut bgbuf[2][e..e + W]);
                    bg_on[2] = true;
                }
                _ => {}
            }
        }
        if self.hd.on && self.hd.cap_layers {
            if self.hd.layers.len() != 4 * 240 * 160 {
                self.hd.layers = vec![0; 4 * 240 * 160];
            }
            for b in 0..4 {
                for gx in 0..W.min(240) {
                    let c = bgbuf[b][e + gx];
                    self.hd.layers[(b * 160 + y as usize) * 240 + gx] =
                        if !bg_on[b] || c & TRANSPARENT != 0 { 0 } else { self.ppu.rgb_lut[(c & 0x7FFF) as usize] | 0xFF00_0000 };
                }
            }
        }
        if self.hd.on && bg_on[0] {
            if self.hd.bg0.len() != 240 * 160 {
                self.hd.bg0 = vec![0; 240 * 160];
            }
            for gx in 0..W.min(240) {
                let c = bgbuf[0][e + gx];
                self.hd.bg0[y as usize * 240 + gx] = if c & TRANSPARENT != 0 { 0 } else { self.ppu.rgb_lut[(c & 0x7FFF) as usize] | 0xFF00_0000 };
            }
        }
        if self.hd.hiding {
            let ty = (y as usize / 8).min(19);
            let row = &self.hd.talk_mask[ty * 30..ty * 30 + 30];
            if self.hd.mask_bg0_only && self.hd.bg1_mask.len() == 600 && bg_on[1] {
                let row1 = &self.hd.bg1_mask[ty * 30..ty * 30 + 30];
                for gx in 0..W {
                    if row1[gx / 8] {
                        bgbuf[1][e + gx] = TRANSPARENT;
                    }
                }
            }
            for b in 0..(if self.hd.mask_bg0_only { 1 } else { 2 }) {
                if bg_on[b] {
                    for gx in 0..W {
                        if row[gx / 8] {
                            bgbuf[b][e + gx] = TRANSPARENT;
                        }
                    }
                }
            }
        }
        if self.ppu.ext > 0 && self.ppu.wide_map && !self.ppu.ui_boxes.is_empty() {
            let ty = (y as usize / 8).min(19);
            let row_shift = &self.ppu.ui_shift[ty * 30..ty * 30 + 30];
            if row_shift.iter().any(|&v| v != 0) {
                for b in 0..2 {
                    if !bg_on[b] {
                        continue;
                    }
                    let mut tmp = [TRANSPARENT; MAXW];
                    for gx in 0..W {
                        let sh = row_shift[gx / 8] as i32;
                        let xo = (e + gx) as i32 + sh;
                        if xo >= 0 && (xo as usize) < lw {
                            let c = bgbuf[b][e + gx];
                            if sh != 0 || tmp[xo as usize] & TRANSPARENT != 0 {
                                tmp[xo as usize] = c;
                            }
                        }
                    }
                    bgbuf[b][..lw].copy_from_slice(&tmp[..lw]);
                }
            }
        }
        let hd_map = self.hd.map;
        if hd_map {
            bg_on[2] = false;
            bg_on[3] = false;
        }
        if self.ppu.ext > 0 && self.ppu.wide_map && !hd_map {
            if bg_on[3] {
                self.wide_sides(y, &mut bgbuf[3][..lw], e, 3);
                // outside the map: let the frontend fill it (marker)
                for x in (0..e).chain(e + W..lw) {
                    if bgbuf[3][x] == OUTSIDE {
                        bgbuf[3][x] = TRANSPARENT;
                        outside[x] = true;
                    }
                }
            }
            if bg_on[2] && self.ppu.wide_blue | self.ppu.wide_red != 0 {
                self.wide_sides(y, &mut bgbuf[2][..lw], e, 2);
            }
        }
        // Widescreen outside the battle map: the background layer (lowest
        // priority BG) fills the full width. The middle stays 1:1, only the
        // outer EDGE_BAND pixels of the background are stretched outwards.
        let mut scene_wide = false;
        if self.ppu.ext > 0 && !self.ppu.wide_map {
            // candidates from lowest to highest priority
            let mut cand: [(u16, usize); 4] = [(0, 0); 4];
            let mut nc = 0;
            for b in 0..4 {
                if bg_on[b] {
                    cand[nc] = (((self.io16(8 + b * 2) & 3) << 2) | b as u16, b);
                    nc += 1;
                }
            }
            cand[..nc].sort_by(|a, b| b.0.cmp(&a.0));
            // the lowest layer that shows a real picture at the edges wins;
            // plain (single colour) or mostly transparent layers are skipped
            let mut best: Option<(u16, usize)> = cand[..nc].first().copied();
            for &(k, b) in &cand[..nc] {
                let line = &bgbuf[b][e..e + W];
                let mut opaque = 0;
                let mut first = None;
                let mut varied = false;
                for &c in line[..EDGE_BAND].iter().chain(line[W - EDGE_BAND..].iter()) {
                    if c & TRANSPARENT == 0 {
                        opaque += 1;
                        match first {
                            None => first = Some(c),
                            Some(f) if f != c => varied = true,
                            _ => {}
                        }
                    }
                }
                if opaque * 4 >= EDGE_BAND * 2 * 3 && varied {
                    best = Some((k, b));
                    break;
                }
            }
            if let Some((_, b)) = best {
                let mut tmp = [TRANSPARENT; MAXW];
                let src = &bgbuf[b][e..e + W];
                let band = EDGE_BAND;
                let left = e + band; // output pixels covering source [0, band)
                let right_start = e + W - band;
                let right = lw - right_start;
                for (x, t) in tmp[..lw].iter_mut().enumerate() {
                    let sx = if x < left {
                        x * band / left
                    } else if x >= right_start {
                        W - band + (x - right_start) * band / right
                    } else {
                        x - e
                    };
                    *t = src[sx.min(W - 1)];
                }
                bgbuf[b][..lw].copy_from_slice(&tmp[..lw]);
                scene_wide = true;
            }
        }
        let mut ocol = [TRANSPARENT; MAXW];
        let mut oprio = [4u8; MAXW];
        let mut osemi = [false; MAXW];
        let mut owin = [false; MAXW];
        if dispcnt & 0x1000 != 0 {
            self.render_objs(y as i32, &mut ocol[..lw], &mut oprio[..lw], &mut osemi[..lw], &mut owin[..lw], e, hd_map);
        }

        // advance affine reference points
        for i in 0..2 {
            let p = 0x20 + i * 0x10;
            let pb = self.io16(p + 2) as i16 as i32;
            let pd = self.io16(p + 6) as i16 as i32;
            self.ppu.ref_x[i] = self.ppu.ref_x[i].wrapping_add(pb);
            self.ppu.ref_y[i] = self.ppu.ref_y[i].wrapping_add(pd);
        }

        // BG order by priority
        let bgprio: [u8; 4] = [0, 1, 2, 3].map(|b| (self.io16(8 + b * 2) & 3) as u8);
        let mut order = [(0usize, 0u8); 4];
        let mut nb = 0;
        for p in 0..4u8 {
            for b in 0..4 {
                if bg_on[b] && bgprio[b] == p {
                    order[nb] = (b, p);
                    nb += 1;
                }
            }
        }
        let order = &order[..nb];

        let win_any = dispcnt & 0xE000 != 0;
        let winin = self.io16(0x48);
        let winout = self.io16(0x4A);
        let inside = |h: u16, v: u16, x: u32, y: u32| -> bool {
            let (x1, x2) = ((h >> 8) as u32, (h & 0xFF) as u32);
            let (y1, y2) = ((v >> 8) as u32, (v & 0xFF) as u32);
            let ix = if x1 <= x2 { x >= x1 && x < x2 } else { x >= x1 || x < x2 };
            let iy = if y1 <= y2 { y >= y1 && y < y2 } else { y >= y1 || y < y2 };
            ix && iy
        };
        let w0h = self.io16(0x40);
        let w1h = self.io16(0x42);
        let w0v = self.io16(0x44);
        let w1v = self.io16(0x46);

        let mut bldcnt = self.io16(0x50);
        // shop / arena drawn natively: no darkened panel behind the game's text box
        if self.hd.on && self.hd.shop.is_some() && (bldcnt >> 6) & 3 == 3 {
            bldcnt &= !0xC0;
        }
        let bmode = (bldcnt >> 6) & 3;
        let bldalpha = self.io16(0x52);
        let eva = (bldalpha & 31).min(16) as u32;
        let evb = ((bldalpha >> 8) & 31).min(16) as u32;
        let evy = (self.io16(0x54) & 31).min(16) as u32;
        let backdrop = pal_color(&self.pal, 0);

        let wide_map = self.ppu.wide_map;
        for x in 0..lw {
            let gx = x as i32 - e as i32;
            let side = gx < 0 || gx >= W as i32;
            if side && !wide_map && !scene_wide {
                // no picture here: the frontend fills it
                self.ppu.frame[row + x] = 0;
                continue;
            }
            let gxu = gx.max(0) as u32;
            let wmask: u16 = if !win_any {
                0x3F
            } else if !side && dispcnt & 0x2000 != 0 && inside(w0h, w0v, gxu, y) {
                winin & 0x3F
            } else if !side && dispcnt & 0x4000 != 0 && inside(w1h, w1v, gxu, y) {
                (winin >> 8) & 0x3F
            } else if dispcnt & 0x8000 != 0 && owin[x] {
                (winout >> 8) & 0x3F
            } else {
                winout & 0x3F
            };
            let bd = if hd_map { TRANSPARENT } else if side && !scene_wide { 0 } else { backdrop };
            let mut layers = [(5u8, bd); 2];
            if side && outside[x] && (ocol[x] & TRANSPARENT != 0 || wmask & 0x10 == 0) {
                self.ppu.frame[row + x] = 0;
                continue;
            }
            let mut n = 0;
            let oc = ocol[x];
            let mut obj_pending = oc & TRANSPARENT == 0 && wmask & 0x10 != 0;
            let op = oprio[x];
            for &(b, p) in order {
                if obj_pending && op <= p {
                    obj_pending = false;
                    layers[n] = (4, oc);
                    n += 1;
                    if n == 2 {
                        break;
                    }
                }
                let c = bgbuf[b][x];
                if c & TRANSPARENT == 0 && wmask & (1 << b) != 0 {
                    layers[n] = (b as u8, c);
                    n += 1;
                    if n == 2 {
                        break;
                    }
                }
            }
            if obj_pending && n < 2 {
                layers[n] = (4, oc);
            }
            let (t1, c1) = layers[0];
            let (t2, c2) = layers[1];
            if hd_map {
                // interface layer: transparent where the world shows through
                if c1 & TRANSPARENT != 0 {
                    self.ppu.frame[row + x] = 0;
                    continue;
                }
                let effects = wmask & 0x20 != 0;
                let is_t1 = bldcnt & (1 << t1) != 0;
                let world_t2 = bldcnt & (1 << 13) != 0 || bldcnt & (0xC << 8) != 0;
                let blends = (t1 == 4 && osemi[x]) || (effects && is_t1 && bmode == 1);
                let mut out = c1;
                let mut a = 255u32;
                if blends {
                    if c2 & TRANSPARENT == 0 && bldcnt & (1 << (8 + t2 as u16)) != 0 {
                        out = alpha(c1, c2, eva, evb);
                    } else if world_t2 {
                        a = (eva * 255 / 16).min(255);
                    }
                } else if effects && is_t1 {
                    match bmode {
                        2 => out = bright(c1, evy, true),
                        3 => out = bright(c1, evy, false),
                        _ => {}
                    }
                }
                self.ppu.frame[row + x] = (self.ppu.rgb_lut[out as usize & 0x7FFF] & 0xFFFFFF) | a << 24;
                continue;
            }
            let effects = wmask & 0x20 != 0;
            let is_t1 = bldcnt & (1 << t1) != 0;
            let is_t2 = bldcnt & (1 << (8 + t2 as u16)) != 0;
            let mut out = c1;
            if t1 == 4 && osemi[x] && is_t2 {
                out = alpha(c1, c2, eva, evb);
            } else if effects && is_t1 {
                match bmode {
                    1 if is_t2 => out = alpha(c1, c2, eva, evb),
                    2 => out = bright(c1, evy, true),
                    3 => out = bright(c1, evy, false),
                    _ => {}
                }
            }
            self.ppu.frame[row + x] = self.ppu.rgb_lut[out as usize & 0x7FFF];
        }
    }
}

#[inline]
fn alpha(a: u32, b: u32, eva: u32, evb: u32) -> u32 {
    let mut r = 0;
    for s in [0, 5, 10] {
        let ca = (a >> s) & 31;
        let cb = (b >> s) & 31;
        r |= ((ca * eva + cb * evb) >> 4).min(31) << s;
    }
    r
}
#[inline]
fn bright(a: u32, evy: u32, up: bool) -> u32 {
    let mut r = 0;
    for s in [0, 5, 10] {
        let c = (a >> s) & 31;
        let v = if up { c + (((31 - c) * evy) >> 4) } else { c - ((c * evy) >> 4) };
        r |= v << s;
    }
    r
}
#[inline]
fn to_rgb(c: u32) -> u32 {
    let r = c & 31;
    let g = (c >> 5) & 31;
    let b = (c >> 10) & 31;
    let e = |v: u32| (v << 3) | (v >> 2);
    0xFF00_0000 | e(r) << 16 | e(g) << 8 | e(b)
}
