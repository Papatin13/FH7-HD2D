//! Widescreen for the Fire Emblem 7 (USA) battle map.
//!
//! The game only draws 240 pixels: its BG tilemaps cover little more than the
//! visible area. In widescreen mode the extra columns left and right are
//! rebuilt from the game's own map data in RAM:
//! * BG3 (terrain): map metatile indices (`gBmMapBaseTiles`, row pointers at
//!   0x02032E90) looked up in the metatile table at 0x02030A90 (4 tilemap
//!   entries per metatile, palette +6), drawn with the real tile graphics from
//!   VRAM, so tile animations (water etc.) keep working.
//! * BG2 (blue movement / red attack squares): from the movement map
//!   (0xFF = unreachable) and the range map, but only while the visible part of
//!   BG2 agrees with that rule, so other overlay modes are never guessed.
//! Unit sprites outside the 240 px view are made visible by a patch in the
//! recompiled sprite code (see `post_hook`).
use crate::Gba;

const MAP_SIZE: u32 = 0x0202_E3D8; // s16 w, s16 h
const MAP_MOVE: u32 = 0x0202_E3E4; // u8** movement map
const MAP_RANGE: u32 = 0x0202_E3E8; // u8** range map
const BASE_TILE_ROWS: u32 = 0x0203_2E90; // u16* rows
const MAP_TERRAIN: u32 = 0x0202_E3E0; // u8** terrain map
const TERRAIN_TABLE: u32 = 0x0203_2A90; // u8 terrain per metatile (index = tile value / 4)
const BMST_CAMERA_MAX: u32 = 0x0202_BBB8 + 0x28; // s16 x, y
const METATILES: u32 = 0x0203_0A90;
const BMST_ORIGIN: u32 = 0x0202_BBB8 + 0x24; // s16 x, y (in tiles of 16 px)
const TRANSPARENT: u32 = 0x8000_0000;
/// Plain dark backdrop beyond the edge of the battle map (BGR555).
const OUTSIDE_COLOR: u32 = 3 | 3 << 5 | 4 << 10;


/// Per-map statistics for generating scenery beyond the map edge.
#[derive(Default)]
pub struct MapGen {
    sig: u64,
    w: i32,
    h: i32,
    terrain: Vec<u8>,
    /// most frequent metatiles per terrain type (up to 3)
    variants: Vec<(u8, [u16; 3], u8)>,
    base: u8,
    forest: bool,
}

/// Terrain that continues naturally past the edge: area-like terrain whose
/// tiles repeat seamlessly (plains, forest, thicket, sand, desert, sea, lake,
/// floor). Mountains, walls, rivers, roads and buildings are made of edge and
/// corner pieces and are replaced by the map's base terrain instead.
fn natural(t: u8) -> bool {
    matches!(t, 0x01 | 0x0C | 0x0D | 0x0E | 0x0F | 0x15 | 0x16 | 0x17 | 0x18)
}

fn hash(a: i32, b: i32, c: u64) -> u32 {
    let mut h = (a as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ c;
    h ^= h >> 31;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    (h >> 32) as u32
}

impl Gba {
    /// Rebuilds the scenery statistics when the map changed.
    pub(crate) fn mapgen_update(&mut self, w: i32, h: i32) {
        let mut sig: u64 = (w as u64) << 32 | h as u64;
        let mut terrain = Vec::with_capacity((w * h) as usize);
        let mut tiles = Vec::with_capacity((w * h) as usize);
        let trows = self.read32(MAP_TERRAIN);
        for y in 0..h {
            let brow = self.read32(BASE_TILE_ROWS + 4 * y as u32);
            let trow = self.read32(trows + 4 * y as u32);
            for x in 0..w {
                let v = self.read16(brow + 2 * x as u32);
                let t = self.read8(trow + x as u32);
                sig = sig.wrapping_mul(0x100_0000_01B3) ^ (v as u64) ^ (t as u64) << 16;
                terrain.push(t);
                tiles.push(v);
            }
        }
        if sig == self.mapgen.sig && self.mapgen.w == w {
            return;
        }
        let mut counts: std::collections::HashMap<(u8, u16), u32> = Default::default();
        let mut tcount = [0u32; 256];
        for (&t, &v) in terrain.iter().zip(tiles.iter()) {
            *counts.entry((t, v)).or_insert(0) += 1;
            tcount[t as usize] += 1;
        }
        let mut variants = Vec::new();
        for t in 0..=255u8 {
            if tcount[t as usize] == 0 {
                continue;
            }
            let mut vs: Vec<(u32, u16)> = counts.iter().filter(|((tt, _), _)| *tt == t).map(|((_, v), &n)| (n, *v)).collect();
            vs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let mut arr = [vs[0].1; 3];
            for (i, (_, v)) in vs.iter().take(3).enumerate() {
                arr[i] = *v;
            }
            variants.push((t, arr, vs.len().min(3) as u8));
        }
        let base = (0..=255u8).filter(|&t| natural(t)).max_by_key(|&t| tcount[t as usize]).filter(|&t| tcount[t as usize] > 0).unwrap_or(1);
        self.mapgen = MapGen { sig, w, h, terrain, variants, base, forest: tcount[0x0C] > 0 && base == 0x01 };
    }

    /// Metatile for a cell outside the map (scenery, not playable).
    pub(crate) fn scenery_metatile(&self, mx: i32, my: i32) -> Option<u16> {
        let g = &self.mapgen;
        if g.w == 0 {
            return None;
        }
        let ex = mx.clamp(0, g.w - 1);
        let ey = my.clamp(0, g.h - 1);
        let d = (ex - mx).abs().max((ey - my).abs());
        let edge = g.terrain[(ey * g.w + ex) as usize];
        let r = hash(mx, my, g.sig);
        // keep the edge terrain with decreasing probability, else base terrain
        let keep = natural(edge) && (r % 1000) < (1000.0 * 0.8f32.powi(d - 1)) as u32;
        let mut t = if keep { edge } else { g.base };
        if t == 0x01 && g.forest && (r >> 10) % 100 < 9 {
            t = 0x0C;
        }
        let (_, vars, n) = g.variants.iter().find(|(tt, _, _)| *tt == t).or_else(|| g.variants.iter().find(|(tt, _, _)| *tt == g.base))?;
        let pick = (r >> 20) % 100;
        let i = if pick < 90 || *n < 2 { 0 } else if pick < 98 || *n < 3 { 1 } else { 2 };
        Some(vars[i])
    }

    pub(crate) fn map_size(&mut self) -> (i32, i32) {
        (self.read16(MAP_SIZE) as i16 as i32, self.read16(MAP_SIZE + 2) as i16 as i32)
    }

    /// Map pixel of the BG3 tilemap's left/top edge (BG scroll + tile origin).
    pub(crate) fn map_origin(&mut self) -> (i32, i32) {
        // the scroll registers are 9 bits; the game also scrolls a little past the
        // map's top/left edge (negative values), so read them signed
        let s9 = |v: u16| {
            let v = (v & 0x1FF) as i32;
            if v >= 256 { v - 512 } else { v }
        };
        let hofs = s9(self.io16(0x1C));
        let vofs = s9(self.io16(0x1E));
        let ox = self.read16(BMST_ORIGIN) as i16 as i32;
        let oy = self.read16(BMST_ORIGIN + 2) as i16 as i32;
        (hofs + ox * 16, vofs + oy * 16)
    }

    pub(crate) fn cell_status(&mut self, mx: i32, my: i32) -> u8 {
        // 0 none, 1 blue, 2 red
        let mv = self.read32(MAP_MOVE);
        let rg = self.read32(MAP_RANGE);
        let mrow = self.read32(mv + 4 * my as u32);
        let rrow = self.read32(rg + 4 * my as u32);
        if self.read8(mrow + mx as u32) != 0xFF {
            1
        } else if self.read8(rrow + mx as u32) != 0 {
            2
        } else {
            0
        }
    }

    /// Called at the start of every frame in widescreen mode.
    pub fn wide_frame_setup(&mut self) {
        let dispcnt = self.io16(0);
        let bg3 = self.io16(0x0E);
        let (w, h) = self.map_size();
        self.ppu.wide_map = dispcnt & 7 == 0
            && dispcnt & 0x800 != 0
            && bg3 & 0xFFFC == 0x0F08
            && (1..=64).contains(&w)
            && (1..=64).contains(&h)
            && self.read32(BASE_TILE_ROWS) >> 24 == 2;
        self.ppu.wide_blue = 0;
        self.ppu.wide_red = 0;
        if self.ppu.wide_map {
            self.ppu.wide_map = self.bg3_shows_map(w, h);
        }
        if self.ppu.wide_map {
            self.mapgen_update(w, h);
            // Shift the picture at map edges so both sides show map, not void.
            let e = self.ppu.ext as i32;
            let (bx, _) = self.map_origin();
            let mw = w * 16;
            let lo = (2 * e - (mw - bx - 240)).max(0);
            let hi = (2 * e).min(bx);
            self.ppu.ext_left = if lo <= hi { e.clamp(lo, hi) as usize } else { e as usize };
            self.ui_layout();
        }
        if !self.ppu.wide_map || dispcnt & 0x400 == 0 || self.io16(0x0C) & 0xFFFC != 0x0E00 {
            return;
        }
        // Does the visible BG2 overlay follow the movement/range rule?
        let (bx, by) = self.map_origin();
        let (ox, oy) = (self.read16(BMST_ORIGIN) as i16 as i32, self.read16(BMST_ORIGIN + 2) as i16 as i32);
        let sb = ((self.io16(0x0C) >> 8) & 31) as usize * 0x800;
        let (mut blue, mut red) = (0u16, 0u16);
        let mut ok = true;
        let mut any = false;
        let mx0 = bx.div_euclid(16).max(0);
        let my0 = by.div_euclid(16).max(0);
        for my in my0..(my0 + 10).min(h) {
            for mx in mx0..(mx0 + 15).min(w) {
                let tx = ((mx - ox) * 2).rem_euclid(32) as usize;
                let ty = ((my - oy) * 2).rem_euclid(32) as usize;
                let a = sb + (ty * 32 + tx) * 2;
                let ent = self.vram[a] as u16 | (self.vram[a + 1] as u16) << 8;
                let shown = if ent & 0x3FF == 0x280 { Some(ent & 0xF000) } else { None };
                let want = self.cell_status(mx, my);
                match (want, shown) {
                    (0, None) => {}
                    (1, Some(p)) if blue == 0 || blue == p => {
                        blue = p;
                        any = true;
                    }
                    (2, Some(p)) if red == 0 || red == p => {
                        red = p;
                        any = true;
                    }
                    _ => ok = false,
                }
            }
        }
        if ok && any {
            self.ppu.wide_blue = if blue != 0 { blue | 0x280 } else { 0 };
            self.ppu.wide_red = if red != 0 { red | 0x280 } else { 0 };
        }
    }

    /// Widescreen HUD: finds the game's windows on BG0/BG1 (connected blocks
    /// of tiles) and anchors those at the left/right edge of the 240 px view
    /// to the real screen edges. Centred elements (text boxes, banners) stay.
    pub fn ui_layout(&mut self) {
        self.ppu.ui_shift.iter_mut().for_each(|v| *v = 0);
        self.ppu.ui_boxes.clear();
        let dispcnt = self.io16(0);
        let el = self.ppu.ext_left as i32;
        let er = (self.ppu.width - 240) as i32 - el;
        let mut occ = [false; 30 * 20];
        for b in 0..2 {
            if dispcnt & (0x100 << b) == 0 {
                continue;
            }
            let cnt = self.io16(8 + b * 2) as usize;
            let sb = ((cnt >> 8) & 31) * 0x800;
            let hofs = (self.io16(0x10 + b * 4) & 0x1FF) as usize;
            let vofs = (self.io16(0x12 + b * 4) & 0x1FF) as usize;
            for ty in 0..20 {
                for tx in 0..30 {
                    let gx = ((tx * 8 + hofs) / 8) % 32;
                    let gy = ((ty * 8 + vofs) / 8) % 32;
                    let a = sb + (gy * 32 + gx) * 2;
                    let ent = self.vram[a] as u16 | (self.vram[a + 1] as u16) << 8;
                    if ent & 0x3FF != 0 {
                        occ[ty * 30 + tx] = true;
                    }
                }
            }
        }
        let mut seen = [false; 30 * 20];
        let mut stack = Vec::new();
        for start in 0..600 {
            if !occ[start] || seen[start] {
                continue;
            }
            let (mut x0, mut y0, mut x1, mut y1) = (30, 20, 0, 0);
            let mut cells = Vec::new();
            stack.push(start);
            seen[start] = true;
            while let Some(i) = stack.pop() {
                cells.push(i);
                let (x, y) = (i % 30, i / 30);
                x0 = x0.min(x);
                x1 = x1.max(x + 1);
                y0 = y0.min(y);
                y1 = y1.max(y + 1);
                let mut push = |j: usize| {
                    if occ[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                };
                if x > 0 {
                    push(i - 1);
                }
                if x < 29 {
                    push(i + 1);
                }
                if y > 0 {
                    push(i - 30);
                }
                if y < 19 {
                    push(i + 30);
                }
            }
            let w = x1 - x0;
            let shift = if x0 <= 1 && x1 <= 16 && w <= 16 {
                -el
            } else if x1 >= 29 && x0 >= 14 && w <= 16 {
                er
            } else {
                0
            };
            if shift != 0 && y1 - y0 < 20 {
                for &i in &cells {
                    self.ppu.ui_shift[i] = shift as i16;
                }
                self.ppu.ui_boxes.push((x0 as i32 * 8, y0 as i32 * 8, x1 as i32 * 8, y1 as i32 * 8, shift));
            }
        }
    }

    /// Compares the visible BG3 tilemap with the map data (>= 90 % must match).
    pub(crate) fn bg3_shows_map(&mut self, w: i32, h: i32) -> bool {
        let (bx, by) = self.map_origin();
        let (ox, oy) = (self.read16(BMST_ORIGIN) as i16 as i32, self.read16(BMST_ORIGIN + 2) as i16 as i32);
        let sb = ((self.io16(0x0E) >> 8) & 31) as usize * 0x800;
        let (mut good, mut total) = (0, 0);
        let mx0 = (bx + 15).div_euclid(16);
        let my0 = (by + 15).div_euclid(16);
        for my in my0..(my0 + 9).min(h) {
            if my < 0 {
                continue;
            }
            let row = self.read32(BASE_TILE_ROWS + 4 * my as u32);
            for mx in mx0..(mx0 + 14).min(w) {
                if mx < 0 {
                    continue;
                }
                let v = self.read16(row + 2 * mx as u32) as u32;
                let want = self.read16(METATILES + 2 * v) & 0x0FFF;
                let tx = ((mx - ox) * 2).rem_euclid(32) as usize;
                let ty = ((my - oy) * 2).rem_euclid(32) as usize;
                let a = sb + (ty * 32 + tx) * 2;
                let ent = self.vram[a] as u16 | (self.vram[a + 1] as u16) << 8;
                total += 1;
                if ent & 0x0FFF == want {
                    good += 1;
                }
            }
        }
        if std::env::var("FE7_MAPDBG").is_ok() {
            eprintln!("bg3_shows_map good {} total {} origin {:?} bmst {} {}", good, total, (bx, by), ox, oy);
        }
        total >= 20 && good * 10 >= total * 9
    }

    pub(crate) fn tile_px(&self, char_base: usize, ent: u32, px: u32, py: u32) -> u32 {
        let tile = (ent & 0x3FF) as usize;
        let px = if ent & 0x400 != 0 { 7 - px } else { px };
        let py = if ent & 0x800 != 0 { 7 - py } else { py };
        let a = char_base + tile * 32 + (py * 4 + px / 2) as usize;
        if a >= 0x10000 {
            return TRANSPARENT;
        }
        let n = (self.vram[a] >> ((px & 1) * 4)) & 15;
        if n == 0 {
            return TRANSPARENT;
        }
        let i = ((ent >> 12) * 16 + n as u32) as usize;
        (self.pal[i * 2] as u32 | (self.pal[i * 2 + 1] as u32) << 8) & 0x7FFF
    }

    /// Fills the side columns of one line of BG3 (terrain) or BG2 (overlay).
    /// Beyond the map edge, BG3 shows generated scenery (see `scenery_metatile`),
    /// slightly darker, with a thin line marking the playable border.
    pub fn wide_sides(&mut self, y: u32, out: &mut [u32], e: usize, layer: usize) {
        let (w, h) = self.map_size();
        let (bx, by) = self.map_origin();
        let cnt = self.io16(0x08 + layer * 2) as u32;
        let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
        let py = by + y as i32;
        let my = py.div_euclid(16);
        let lw = out.len();
        let in_y = (0..h).contains(&my);
        let row = if in_y { self.read32(BASE_TILE_ROWS + 4 * my as u32) } else { 0 };
        let mut cache_mx = i32::MIN;
        let mut ents = [0u32; 4];
        let mut status = 0u8;
        let mut scenery = false;
        let (mw, mh) = (w * 16, h * 16);
        for x in (0..e).chain(e + 240..lw) {
            let px = bx + x as i32 - e as i32;
            let mx = px.div_euclid(16);
            let inside = in_y && (0..w).contains(&mx);
            if layer == 2 && !inside {
                out[x] = TRANSPARENT;
                continue;
            }
            if mx != cache_mx {
                cache_mx = mx;
                if layer == 3 {
                    let v = if inside && row >> 24 == 2 {
                        Some(self.read16(row + 2 * mx as u32))
                    } else {
                        self.scenery_metatile(mx, my)
                    };
                    scenery = !inside;
                    match v {
                        Some(v) => {
                            for (k, ent) in ents.iter_mut().enumerate() {
                                let t = self.read16(METATILES + 2 * (v as u32 + k as u32)) as u32;
                                *ent = (t & 0x0FFF) | ((((t >> 12) + 6) & 15) << 12);
                            }
                        }
                        None => ents = [u32::MAX; 4],
                    }
                } else {
                    status = self.cell_status(mx, my);
                    let base = match status {
                        1 => self.ppu.wide_blue as u32,
                        2 => self.ppu.wide_red as u32,
                        _ => 0,
                    };
                    for (k, ent) in ents.iter_mut().enumerate() {
                        *ent = base + k as u32;
                    }
                }
            }
            if layer == 2 && (status == 0 || ents[0] == 0) {
                out[x] = TRANSPARENT;
                continue;
            }
            if ents[0] == u32::MAX {
                out[x] = OUTSIDE_COLOR;
                continue;
            }
            let sub = (((py >> 3) & 1) * 2 + ((px >> 3) & 1)) as usize;
            let c = self.tile_px(char_base, ents[sub], (px & 7) as u32, (py & 7) as u32);
            let _ = (scenery, mw, mh);
            out[x] = c;
        }
    }

    /// Widescreen: makes narrow maps playable over the full screen width by
    /// appending columns on the right side of the map while it is loaded.
    /// Only the right side grows, so every coordinate used by the chapter's
    /// events (units, villages, chests, doors, reinforcements, goals) stays
    /// valid. New cells continue the area-like terrain at the right edge
    /// (plains, forest, sand, sea ...), otherwise the map's base terrain.
    fn widen_loaded_map(&mut self) {
        let buf = self.map_load_buf;
        if self.hd.on {
            return;
        }
        let target = if self.wide_cols > 0 { self.wide_cols } else { (240 + 2 * self.wide_ext).div_ceil(16) };
        if buf >> 24 != 2 || (self.wide_cols == 0 && self.wide_ext == 0) {
            return;
        }
        let w = self.read8(buf) as u32;
        let h = self.read8(buf + 1) as u32;
        if w == 0 || h == 0 || w >= target || target > 64 {
            return;
        }
        let nw = target;
        let tile = |g: &mut Gba, x: u32, y: u32, stride: u32| g.read16(buf + 2 + 2 * (y * stride + x));
        // statistics: tile values per terrain
        let mut counts: std::collections::HashMap<(u8, u16), u32> = Default::default();
        let mut tcount = [0u32; 256];
        for y in 0..h {
            for x in 0..w {
                let v = tile(self, x, y, w);
                let t = self.read8(TERRAIN_TABLE + (v as u32 >> 2));
                *counts.entry((t, v)).or_insert(0) += 1;
                tcount[t as usize] += 1;
            }
        }
        let best = |t: u8, rank: usize| -> Option<u16> {
            let mut vs: Vec<(u32, u16)> = counts.iter().filter(|((tt, _), _)| *tt == t).map(|((_, v), &n)| (n, *v)).collect();
            vs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            vs.get(rank.min(vs.len().saturating_sub(1))).map(|x| x.1)
        };
        let base = (0..=255u8).filter(|&t| natural(t) && tcount[t as usize] > 0).max_by_key(|&t| tcount[t as usize]).unwrap_or(1);
        let forest = tcount[0x0C] > 0 && base == 0x01;
        // move rows to the new stride (from the last row backwards, in place)
        for y in (0..h).rev() {
            for x in (0..w).rev() {
                let v = tile(self, x, y, w);
                self.write16(buf + 2 + 2 * (y * nw + x), v);
            }
        }
        let seed = (w as u64) << 8 | h as u64;
        for y in 0..h {
            let edge = self.read16(buf + 2 + 2 * (y * nw + w - 1));
            let te = self.read8(TERRAIN_TABLE + (edge as u32 >> 2));
            for x in w..nw {
                let d = (x - w + 1) as i32;
                let r = hash(x as i32, y as i32, seed);
                let keep = natural(te) && (r % 1000) < (1000.0 * 0.8f32.powi(d - 1)) as u32;
                let mut t = if keep { te } else { base };
                if t == 0x01 && forest && (r >> 10) % 100 < 9 {
                    t = 0x0C;
                }
                let pick = (r >> 20) % 100;
                let rank = if pick < 90 { 0 } else if pick < 98 { 1 } else { 2 };
                let v = best(t, rank).or_else(|| best(base, 0)).unwrap_or(edge);
                self.write16(buf + 2 + 2 * (y * nw + x), v);
            }
        }
        self.write8(buf, nw as u8);
        self.write16(MAP_SIZE, nw as u16);
        self.write16(BMST_CAMERA_MAX, (nw * 16 - 240) as u16);
        self.map_widened = Some((w, nw));
    }

    /// Hooks run after specific recompiled instructions (game patches).
    #[inline]
    pub fn post_hook(&mut self, addr: u32) {
        let e = self.wide_ext;
        if e == 0 {
            return;
        }
        match addr {
            // chapter map loader: remember the decompressed map buffer ...
            0x0801_9186 => self.map_load_buf = self.cpu.r[4],
            // ... and widen the map once the tileset is loaded
            0x0801_91BA => self.widen_loaded_map(),
            // map unit sprites: x+16 compared against 256 -> widen both bounds
            0x0802_593A => self.cpu.r[1] = self.cpu.r[1].wrapping_add(e),
            0x0802_593E => self.cpu.r[0] = self.cpu.r[0].wrapping_add(2 * e),
            _ => {}
        }
    }
}

pub const HOOKS: &[u32] = &[0x0802_593A, 0x0802_593E, 0x0801_9186, 0x0801_91BA];

