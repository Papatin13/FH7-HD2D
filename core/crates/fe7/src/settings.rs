//! PC-specific settings (persisted in fe7.cfg next to the executable).
use gba::keys;
use minifb::Key;
use std::path::PathBuf;

pub const BUTTONS: [(&str, u16); 10] = [
    ("A", keys::A),
    ("B", keys::B),
    ("Start", keys::START),
    ("Select", keys::SELECT),
    ("Hoch", keys::UP),
    ("Runter", keys::DOWN),
    ("Links", keys::LEFT),
    ("Rechts", keys::RIGHT),
    ("L", keys::L),
    ("R", keys::R),
];

pub const FF_SPEEDS: [u32; 5] = [2, 3, 4, 8, 0];

#[derive(Clone)]
pub struct Settings {
    pub scale: usize,
    pub integer_scale: bool,
    /// sharp-bilinear output scaling
    pub smooth: bool,
    /// 0 off, 1 Scale2x, 2 Scale4x
    pub upscale: u8,
    pub fullscreen: bool,
    /// 0 off, 1 auto (window shape), 2 16:9, 3 16:10, 4 21:9
    pub widescreen: u8,
    /// 0 original (borders), 1 original + blurred background, 2 stretch, 3 zoom (crop)
    pub aspect: u8,
    pub filter: u8, // 0 none, 1 scanlines, 2 lcd grid
    pub color_correct: bool,
    pub show_fps: bool,
    pub volume: u8, // 0..=10
    pub ff_sound: bool,
    pub ff_speed: usize, // index into FF_SPEEDS
    pub ff_toggle: bool,
    pub pause_unfocused: bool,
    pub mouse: bool,
    pub hide_cursor: bool,
    pub slot: u8, // 1..=5
    pub recomp: bool,
    pub binds: [[Option<Key>; 2]; 10],
}

pub fn default_binds() -> [[Option<Key>; 2]; 10] {
    [
        [Some(Key::X), Some(Key::J)],
        [Some(Key::Z), Some(Key::Y)],
        [Some(Key::Enter), None],
        [Some(Key::Backspace), Some(Key::RightShift)],
        [Some(Key::Up), Some(Key::W)],
        [Some(Key::Down), Some(Key::S)],
        [Some(Key::Left), Some(Key::A)],
        [Some(Key::Right), Some(Key::D)],
        [Some(Key::Q), None],
        [Some(Key::E), None],
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            scale: 4,
            integer_scale: false,
            smooth: true,
            upscale: 0,
            fullscreen: false,
            widescreen: 1,
            aspect: 2,
            filter: 0,
            color_correct: false,
            show_fps: false,
            volume: 8,
            ff_sound: false,
            ff_speed: 2,
            ff_toggle: false,
            pause_unfocused: true,
            mouse: true,
            hide_cursor: false,
            slot: 1,
            recomp: true,
            binds: default_binds(),
        }
    }
}

/// Keys that can be bound (F1, F5, F8, F12, Esc, Space and Tab are reserved).
pub const BINDABLE: &[Key] = &[
    Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M,
    Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z,
    Key::Key0, Key::Key1, Key::Key2, Key::Key3, Key::Key4, Key::Key5, Key::Key6, Key::Key7, Key::Key8, Key::Key9,
    Key::Up, Key::Down, Key::Left, Key::Right, Key::Enter, Key::Backspace, Key::LeftShift, Key::RightShift,
    Key::LeftCtrl, Key::RightCtrl, Key::LeftAlt, Key::RightAlt, Key::Insert, Key::Delete, Key::Home, Key::End,
    Key::PageUp, Key::PageDown, Key::NumPad0, Key::NumPad1, Key::NumPad2, Key::NumPad3, Key::NumPad4,
    Key::NumPad5, Key::NumPad6, Key::NumPad7, Key::NumPad8, Key::NumPad9, Key::NumPadEnter, Key::NumPadPlus,
    Key::NumPadMinus, Key::Comma, Key::Period, Key::Minus, Key::Semicolon, Key::Apostrophe, Key::Slash,
    Key::Backslash, Key::LeftBracket, Key::RightBracket, Key::F2, Key::F3, Key::F4, Key::F6, Key::F7, Key::F9,
    Key::F10, Key::F11,
];

pub fn key_name(k: Key) -> String {
    let s = format!("{:?}", k);
    match k {
        Key::Up => "Pfeil oben".into(),
        Key::Down => "Pfeil unten".into(),
        Key::Left => "Pfeil li.".into(),
        Key::Right => "Pfeil re.".into(),
        Key::Enter => "Enter".into(),
        Key::Backspace => "Ruecktaste".into(),
        Key::LeftShift => "Shift li.".into(),
        Key::RightShift => "Shift re.".into(),
        Key::LeftCtrl => "Strg li.".into(),
        Key::RightCtrl => "Strg re.".into(),
        _ => s.strip_prefix("Key").unwrap_or(&s).to_string(),
    }
}

fn key_from(s: &str) -> Option<Key> {
    BINDABLE.iter().copied().find(|k| format!("{:?}", k) == s)
}

pub fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("fe7.cfg")))
        .unwrap_or_else(|| PathBuf::from("fe7.cfg"))
}

impl Settings {
    pub fn load() -> Settings {
        let mut s = Settings::default();
        let Ok(txt) = std::fs::read_to_string(config_path()) else { return s };
        for line in txt.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let b = v == "1" || v == "true";
            let n: usize = v.parse().unwrap_or(0);
            match k {
                "scale" => s.scale = n.clamp(2, 8),
                "integer_scale" => s.integer_scale = b,
                "fullscreen" => s.fullscreen = b,
                "smooth" => s.smooth = b,
                "widescreen" => s.widescreen = (n as u8).min(4),
                "upscale" => s.upscale = (n as u8).min(2),
                "aspect" => s.aspect = (n as u8).min(3),
                "filter" => s.filter = (n as u8).min(2),
                "color_correct" => s.color_correct = b,
                "show_fps" => s.show_fps = b,
                "volume" => s.volume = (n as u8).min(10),
                "ff_sound" => s.ff_sound = b,
                "ff_speed" => s.ff_speed = n.min(FF_SPEEDS.len() - 1),
                "ff_toggle" => s.ff_toggle = b,
                "pause_unfocused" => s.pause_unfocused = b,
                "mouse" => s.mouse = b,
                "hide_cursor" => s.hide_cursor = b,
                "slot" => s.slot = (n as u8).clamp(1, 5),
                "recomp" => s.recomp = b,
                _ => {
                    if let Some(name) = k.strip_prefix("bind.") {
                        if let Some(i) = BUTTONS.iter().position(|(n, _)| *n == name) {
                            let mut it = v.split(',');
                            for slot in 0..2 {
                                s.binds[i][slot] = it.next().and_then(|x| key_from(x.trim()));
                            }
                        }
                    }
                }
            }
        }
        s
    }

    pub fn save(&self) {
        let mut o = String::from("# Fire Emblem 7 Rust-Port – Einstellungen (F1 im Spiel)\n");
        let b = |v: bool| if v { 1 } else { 0 };
        o += &format!("scale={}\ninteger_scale={}\nfilter={}\ncolor_correct={}\nshow_fps={}\n", self.scale, b(self.integer_scale), self.filter, b(self.color_correct), b(self.show_fps));
        o += &format!("widescreen={}\n", self.widescreen);
        o += &format!("fullscreen={}\naspect={}\nsmooth={}\nupscale={}\n", b(self.fullscreen), self.aspect, b(self.smooth), self.upscale);
        o += &format!("volume={}\nff_sound={}\nff_speed={}\nff_toggle={}\npause_unfocused={}\n", self.volume, b(self.ff_sound), self.ff_speed, b(self.ff_toggle), b(self.pause_unfocused));
        o += &format!("mouse={}\nhide_cursor={}\nslot={}\nrecomp={}\n", b(self.mouse), b(self.hide_cursor), self.slot, b(self.recomp));
        for (i, (name, _)) in BUTTONS.iter().enumerate() {
            let ks: Vec<String> = self.binds[i].iter().map(|k| k.map(|k| format!("{:?}", k)).unwrap_or_default()).collect();
            o += &format!("bind.{}={}\n", name, ks.join(","));
        }
        if let Err(e) = std::fs::write(config_path(), o) {
            eprintln!("cannot write settings: {e}");
        }
    }

    pub fn keyboard(&self, w: &crate::win::Win) -> u16 {
        let mut k = 0;
        for (i, (_, bit)) in BUTTONS.iter().enumerate() {
            if self.binds[i].iter().flatten().any(|&key| w.is_key_down(key)) {
                k |= bit;
            }
        }
        k
    }
}

impl Settings {
    /// Extra map columns per side for the widescreen setting and window shape.
    pub fn wide_ext(&self, ww: usize, wh: usize) -> u32 {
        let aspect = match self.widescreen {
            0 => return 0,
            1 => ww as f32 / wh.max(1) as f32,
            2 => 16.0 / 9.0,
            3 => 16.0 / 10.0,
            _ => 21.0 / 9.0,
        };
        let e = ((160.0 * aspect - 240.0) / 2.0).round();
        (e.max(0.0) as u32).min(gba::ppu::MAXE as u32) & !1
    }
}
