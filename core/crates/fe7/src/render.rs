//! Output stage: colour correction, optional pixel-art upscaling (Scale2x/4x),
//! sharp-bilinear or nearest scaling to the window, scanline/LCD masks at
//! screen resolution, blurred borders, and compositing of the menu overlay.
//! Rows are rendered in parallel.
use crate::overlay::{CH, CW};
use crate::settings::Settings;

pub struct Renderer {
    lut: Vec<u32>,
    base: Vec<u32>,
    up1: Vec<u32>,
    up2: Vec<u32>,
    hcache: Vec<u32>,
    threads: usize,
}

/// GBA LCD colour correction (gamma 2.2, colour matrix after Pokefan531's gba-color).
fn color_lut() -> Vec<u32> {
    let mut lut = vec![0u32; 32768];
    for i in 0..32768u32 {
        let r = ((i >> 10) & 31) as f32 / 31.0;
        let g = ((i >> 5) & 31) as f32 / 31.0;
        let b = (i & 31) as f32 / 31.0;
        let (r, g, b) = (r.powf(2.2) * 0.93, g.powf(2.2) * 0.93, b.powf(2.2) * 0.93);
        let o = [0.80 * r + 0.275 * g - 0.075 * b, 0.135 * r + 0.64 * g + 0.225 * b, 0.195 * r + 0.155 * g + 0.65 * b];
        let c = |v: f32| ((v.clamp(0.0, 1.0).powf(1.0 / 2.2)) * 255.0).round() as u32;
        lut[i as usize] = c(o[0]) << 16 | c(o[1]) << 8 | c(o[2]);
    }
    lut
}

/// Scale2x / AdvMAME2x.
fn scale2x(src: &[u32], w: usize, h: usize, dst: &mut Vec<u32>) {
    dst.resize(w * h * 4, 0);
    let dw = w * 2;
    for y in 0..h {
        for x in 0..w {
            let e = src[y * w + x];
            let b = if y > 0 { src[(y - 1) * w + x] } else { e };
            let hh = if y + 1 < h { src[(y + 1) * w + x] } else { e };
            let d = if x > 0 { src[y * w + x - 1] } else { e };
            let f = if x + 1 < w { src[y * w + x + 1] } else { e };
            let (e0, e1, e2, e3) = if b != hh && d != f {
                (
                    if d == b { d } else { e },
                    if b == f { f } else { e },
                    if d == hh { d } else { e },
                    if hh == f { f } else { e },
                )
            } else {
                (e, e, e, e)
            };
            let o = y * 2 * dw + x * 2;
            dst[o] = e0;
            dst[o + 1] = e1;
            dst[o + dw] = e2;
            dst[o + dw + 1] = e3;
        }
    }
}

/// Destination rectangle of the game picture (may exceed the window when zooming).
pub fn game_rect(ww: usize, wh: usize, st: &Settings, src_w: usize) -> (i32, i32, i32, i32) {
    let (fw, fh) = (ww as f32, wh as f32);
    let sw = src_w as f32;
    let (dw, dh) = match st.aspect {
        2 => (fw, fh),
        3 => {
            let s = (fw / sw).max(fh / 160.0);
            (sw * s, 160.0 * s)
        }
        _ => {
            let mut s = (fw / sw).min(fh / 160.0);
            if st.integer_scale && s >= 1.0 {
                s = s.floor();
            }
            (sw * s, 160.0 * s)
        }
    };
    let (dw, dh) = (dw.round() as i32, dh.round() as i32);
    ((ww as i32 - dw) / 2, (wh as i32 - dh) / 2, dw, dh)
}

/// Rectangle of the menu overlay: always fully visible, 3:2.
pub fn overlay_rect(ww: usize, wh: usize) -> (i32, i32, i32, i32) {
    let s = (ww as f32 / 240.0).min(wh as f32 / 160.0);
    let (dw, dh) = ((240.0 * s) as i32, (160.0 * s) as i32);
    ((ww as i32 - dw) / 2, (wh as i32 - dh) / 2, dw, dh)
}

pub fn map_point(r: (i32, i32, i32, i32), p: (f32, f32), w: i32, h: i32) -> Option<(i32, i32)> {
    let (ox, oy, dw, dh) = r;
    if dw <= 0 || dh <= 0 {
        return None;
    }
    let x = ((p.0 - ox as f32) * w as f32 / dw as f32).floor() as i32;
    let y = ((p.1 - oy as f32) * h as f32 / dh as f32).floor() as i32;
    ((0..w).contains(&x) && (0..h).contains(&y)).then_some((x, y))
}

/// Per-axis sampling plan for one output coordinate.
#[derive(Clone, Copy, Default, PartialEq)]
struct Tap {
    i0: u32,
    i1: u32,
    w: u32,    // weight of i1, 0..=256
    mask: u32, // scanline/LCD brightness 0..=256
    gi: i32,   // index inside the game image, -1 = outside (border)
    ov: i32,   // overlay index, -1 = outside
    bl: (u32, u32, u32), // blur background taps (i0, i1, weight)
}

#[allow(clippy::too_many_arguments)]
fn plan_axis(n_out: usize, (o, d): (i32, i32), src: usize, gba_n: usize, smooth: bool, mask: u8, (oo, od): (i32, i32), ov_n: usize, blur_n: usize, (bo, bs): (f32, f32)) -> Vec<Tap> {
    let scale = d as f32 / src as f32; // output pixels per source pixel
    let gba_px = d as f32 / gba_n as f32; // output pixels per GBA pixel
    (0..n_out)
        .map(|x| {
            let mut t = Tap { gi: -1, ov: -1, mask: 256, ..Default::default() };
            let rel = x as i32 - o;
            if rel >= 0 && rel < d {
                let p = (rel as f32 + 0.5) / scale;
                if smooth {
                    let tt = p - 0.5;
                    let i = tt.floor();
                    let f2 = ((tt - i - 0.5) * scale + 0.5).clamp(0.0, 1.0);
                    let i = i as i32;
                    t.i0 = i.clamp(0, src as i32 - 1) as u32;
                    t.i1 = (i + 1).clamp(0, src as i32 - 1) as u32;
                    t.w = (f2 * 256.0) as u32;
                } else {
                    t.i0 = (p as u32).min(src as u32 - 1);
                    t.i1 = t.i0;
                }
                t.gi = 0;
                if gba_px >= 2.0 && mask != 0 {
                    let g = (rel as f32 + 0.5) / gba_px;
                    let dist = ((g - g.floor()) - 0.5).abs() * 2.0; // 0 centre .. 1 edge
                    t.mask = match mask {
                        1 => (256.0 * (1.0 - 0.5 * dist * dist)) as u32,
                        _ => (256.0 * (1.0 - 0.28 * dist.powi(4))) as u32,
                    };
                }
            }
            let orel = x as i32 - oo;
            if orel >= 0 && orel < od {
                t.ov = ((orel as i64 * ov_n as i64) / od as i64).min(ov_n as i64 - 1) as i32;
            }
            let fb = ((x as f32 - bo) / bs - 0.5).clamp(0.0, (blur_n - 1) as f32);
            let b0 = fb as u32;
            t.bl = (b0, (b0 + 1).min(blur_n as u32 - 1), ((fb - b0 as f32) * 256.0) as u32);
            t
        })
        .collect()
}

#[inline]
fn lerp(a: u32, b: u32, w: u32) -> u32 {
    if w == 0 || a == b {
        return a;
    }
    let iw = 256 - w;
    let rb = ((a & 0xFF00FF) * iw + (b & 0xFF00FF) * w) >> 8 & 0xFF00FF;
    let g = ((a & 0x00FF00) * iw + (b & 0x00FF00) * w) >> 8 & 0x00FF00;
    rb | g
}

#[inline]
fn scale_rgb(c: u32, m: u32) -> u32 {
    if m >= 256 {
        return c;
    }
    let rb = ((c & 0xFF00FF) * m) >> 8 & 0xFF00FF;
    let g = ((c & 0x00FF00) * m) >> 8 & 0x00FF00;
    rb | g
}

const BW: usize = 24;
const BH: usize = 16;

impl Renderer {
    pub fn new() -> Renderer {
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).clamp(1, 8);
        Renderer { lut: color_lut(), base: Vec::new(), up1: Vec::new(), up2: Vec::new(), hcache: Vec::new(), threads }
    }

    /// `frame` is `fw` x 160 (fw = 240 + 2 * widescreen columns). Pixels with
    /// alpha 0 carry no picture and are filled with a blurred backdrop.
    pub fn render(&mut self, frame: &[u32], fw: usize, el: usize, overlay: &[u32], out: &mut [u32], ww: usize, wh: usize, st: &Settings) {
        let e = el;
        // 1. blur source (24x16 block average of the 240x160 centre)
        let mut blur = [0u32; BW * BH];
        let need_blur = st.aspect == 1 || frame[..fw * 160].iter().step_by(37).any(|&c| c == 0);
        if need_blur {
            for by in 0..BH {
                for bx in 0..BW {
                    let mut acc = [0u32; 3];
                    for y in by * 10..by * 10 + 10 {
                        for x in bx * 10..bx * 10 + 10 {
                            let c = frame[y * fw + e + x];
                            acc[0] += (c >> 16) & 255;
                            acc[1] += (c >> 8) & 255;
                            acc[2] += c & 255;
                        }
                    }
                    let f = |v: u32| (v * 45 / 100 / 100).min(255);
                    blur[by * BW + bx] = f(acc[0]) << 16 | f(acc[1]) << 8 | f(acc[2]);
                }
            }
        }
        // 2. colour (+ fill picture-less side columns)
        self.base.resize(fw * 160, 0);
        for y in 0..160 {
            for x in 0..fw {
                let c = frame[y * fw + x];
                let c = if c == 0 {
                    let fx = (x as f32 + 0.5) * BW as f32 / fw as f32 - 0.5;
                    let fy = (y as f32 + 0.5) * BH as f32 / 160.0 - 0.5;
                    let (x0, y0) = (fx.clamp(0.0, (BW - 1) as f32) as usize, fy.clamp(0.0, (BH - 1) as f32) as usize);
                    let (x1, y1) = ((x0 + 1).min(BW - 1), (y0 + 1).min(BH - 1));
                    let (wx, wy) = (((fx - x0 as f32).clamp(0.0, 1.0) * 256.0) as u32, ((fy - y0 as f32).clamp(0.0, 1.0) * 256.0) as u32);
                    let t = lerp(blur[y0 * BW + x0], blur[y0 * BW + x1], wx);
                    let b = lerp(blur[y1 * BW + x0], blur[y1 * BW + x1], wx);
                    lerp(t, b, wy)
                } else {
                    c & 0x00FF_FFFF
                };
                self.base[y * fw + x] = if st.color_correct {
                    self.lut[(((c >> 19) & 31) << 10 | ((c >> 11) & 31) << 5 | ((c >> 3) & 31)) as usize]
                } else {
                    c
                };
            }
        }
        // 3. upscaler
        let (img, gw, gh): (&[u32], usize, usize) = match st.upscale {
            1 => {
                scale2x(&self.base, fw, 160, &mut self.up1);
                (&self.up1, fw * 2, 320)
            }
            2 => {
                scale2x(&self.base, fw, 160, &mut self.up1);
                let up1 = std::mem::take(&mut self.up1);
                scale2x(&up1, fw * 2, 320, &mut self.up2);
                self.up1 = up1;
                (&self.up2, fw * 4, 640)
            }
            _ => (&self.base, fw, 160),
        };
        // 4. plans
        let (gx, gy, gdw, gdh) = game_rect(ww, wh, st, fw);
        let (ox, oy, odw, odh) = overlay_rect(ww, wh);
        let bs = (ww as f32 / BW as f32).max(wh as f32 / BH as f32);
        let bcx = (ww as f32 - BW as f32 * bs) / 2.0;
        let bcy = (wh as f32 - BH as f32 * bs) / 2.0;
        let smooth = st.smooth;
        let mask_v = if st.filter == 1 || st.filter == 2 { st.filter } else { 0 };
        let mask_h = if st.filter == 2 { 2 } else { 0 };
        let cols = plan_axis(ww, (gx, gdw), gw, fw, smooth, mask_h, (ox, odw), CW, BW, (bcx, bs));
        let rows = plan_axis(wh, (gy, gdh), gh, 160, smooth, mask_v, (oy, odh), CH, BH, (bcy, bs));
        let blur_on = st.aspect == 1;
        let ov_any = overlay.iter().any(|&p| p != 0);

        // 5. horizontal pass: every source row scaled to the output width once
        let x_lo = gx.max(0) as usize;
        let x_hi = ((gx + gdw).max(0) as usize).min(ww);
        let span = x_hi.saturating_sub(x_lo);
        self.hcache.resize(gh * span.max(1), 0);
        let threads = self.threads;
        {
            let per = gh.div_ceil(threads).max(1);
            let cols = &cols;
            std::thread::scope(|sc| {
                for (ci, chunk) in self.hcache.chunks_mut(per * span.max(1)).enumerate() {
                    sc.spawn(move || {
                        if span == 0 {
                            return;
                        }
                        for (k, line) in chunk.chunks_mut(span).enumerate() {
                            let j = ci * per + k;
                            let src = &img[j * gw..(j + 1) * gw];
                            for (d, c) in line.iter_mut().zip(cols[x_lo..x_hi].iter()) {
                                *d = scale_rgb(lerp(src[c.i0 as usize], src[c.i1 as usize], c.w), c.mask);
                            }
                        }
                    });
                }
            });
        }
        let hcache = &self.hcache;
        // blurred border, horizontal pass (16 rows x window width)
        let mut bcache = Vec::new();
        if blur_on {
            bcache = vec![0u32; BH * ww];
            for by in 0..BH {
                for (x, c) in cols.iter().enumerate() {
                    bcache[by * ww + x] = lerp(blur[by * BW + c.bl.0 as usize], blur[by * BW + c.bl.1 as usize], c.bl.2);
                }
            }
        }
        let bcache = &bcache;

        // 6. vertical pass + borders + overlay, rows in parallel
        let chunk_rows = wh.div_ceil(threads).max(1);
        std::thread::scope(|sc| {
            for (ci, chunk) in out.chunks_mut(chunk_rows * ww).enumerate() {
                let cols = &cols;
                let rows = &rows;
                sc.spawn(move || {
                    let mut prev: Option<Tap> = None;
                    let nrows = chunk.len() / ww;
                    for ry in 0..nrows {
                        let r = rows[ci * chunk_rows + ry];
                        if prev == Some(r) {
                            chunk.copy_within((ry - 1) * ww..ry * ww, ry * ww);
                            continue;
                        }
                        prev = Some(r);
                        let line = &mut chunk[ry * ww..(ry + 1) * ww];
                        let in_game = r.gi >= 0 && span > 0;
                        // borders (whole line if outside the picture)
                        let border = |x: usize| -> u32 {
                            if blur_on {
                                lerp(bcache[r.bl.0 as usize * ww + x], bcache[r.bl.1 as usize * ww + x], r.bl.2)
                            } else {
                                0
                            }
                        };
                        if in_game {
                            for x in 0..x_lo {
                                line[x] = border(x);
                            }
                            for x in x_hi..ww {
                                line[x] = border(x);
                            }
                            let a = &hcache[r.i0 as usize * span..(r.i0 as usize + 1) * span];
                            let dst = &mut line[x_lo..x_hi];
                            if r.w == 0 || r.i0 == r.i1 {
                                if r.mask >= 256 {
                                    dst.copy_from_slice(a);
                                } else {
                                    for (d, &s) in dst.iter_mut().zip(a) {
                                        *d = scale_rgb(s, r.mask);
                                    }
                                }
                            } else {
                                let b = &hcache[r.i1 as usize * span..(r.i1 as usize + 1) * span];
                                for ((d, &s0), &s1) in dst.iter_mut().zip(a).zip(b) {
                                    *d = scale_rgb(lerp(s0, s1, r.w), r.mask);
                                }
                            }
                        } else {
                            for (x, d) in line.iter_mut().enumerate() {
                                *d = border(x);
                            }
                        }
                        if ov_any && r.ov >= 0 {
                            let orow = &overlay[r.ov as usize * CW..(r.ov as usize + 1) * CW];
                            for (x, d) in line.iter_mut().enumerate() {
                                let c = cols[x];
                                if c.ov < 0 {
                                    continue;
                                }
                                let o = orow[c.ov as usize];
                                let a = o >> 24;
                                if a == 255 {
                                    *d = o & 0xFF_FFFF;
                                } else if a > 0 {
                                    *d = lerp(*d, o & 0xFF_FFFF, a + 1);
                                }
                            }
                        }
                    }
                });
            }
        });
    }
}
