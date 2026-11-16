//! Headless runner used for testing: runs N frames with scripted input and
//! writes PNG screenshots.
use gba::{keys, Gba};
use std::path::Path;

pub fn save_png(g: &Gba, path: &Path) {
    let f = std::fs::File::create(path).expect("png create");
    let w = g.ppu.width;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, 160);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().unwrap();
    let mut data = Vec::with_capacity(240 * 160 * 3);
    for &p in &g.ppu.frame[..w * 160] {
        data.push((p >> 16) as u8);
        data.push((p >> 8) as u8);
        data.push(p as u8);
    }
    wr.write_image_data(&data).unwrap();
}

pub fn parse_key(s: &str) -> u16 {
    match s.to_ascii_uppercase().as_str() {
        "A" => keys::A,
        "B" => keys::B,
        "START" | "ST" => keys::START,
        "SELECT" | "SE" => keys::SELECT,
        "UP" | "U" => keys::UP,
        "DOWN" | "D" => keys::DOWN,
        "LEFT" | "L" => keys::LEFT,
        "RIGHT" | "R" => keys::RIGHT,
        "LB" => keys::L,
        "RB" => keys::R,
        _ => 0,
    }
}

/// Script format: "frame:KEY+KEY:dur,frame:KEY..." (dur default 4 frames)
pub fn parse_script(s: &str) -> Vec<(u64, u16, u64)> {
    let mut v = Vec::new();
    for part in s.split(',').filter(|p| !p.is_empty()) {
        let f: Vec<&str> = part.split(':').collect();
        let frame: u64 = f[0].parse().unwrap_or(0);
        let k = f.get(1).map(|k| k.split('+').map(parse_key).fold(0, |a, b| a | b)).unwrap_or(0);
        let dur = f.get(2).and_then(|d| d.parse().ok()).unwrap_or(4);
        v.push((frame, k, dur));
    }
    v
}

pub fn keys_at(script: &[(u64, u16, u64)], frame: u64) -> u16 {
    script.iter().filter(|(f, _, d)| frame >= *f && frame < f + d).fold(0, |a, (_, k, _)| a | k)
}
