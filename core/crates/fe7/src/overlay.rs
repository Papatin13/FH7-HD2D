//! F1 settings menu and on-screen messages, drawn on a transparent 480x320
//! ARGB layer that is composited over the game picture.
use crate::settings::{key_name, Settings, BINDABLE, BUTTONS, FF_SPEEDS};
use font8x8::UnicodeFonts;
use minifb::Key;
use std::time::{Duration, Instant};

pub const CW: usize = 480;
pub const CH: usize = 320;

pub fn glyph(c: char) -> [u8; 8] {
    font8x8::BASIC_FONTS.get(c).or_else(|| font8x8::LATIN_FONTS.get(c)).unwrap_or([0; 8])
}

pub fn text(cv: &mut [u32], x: i32, y: i32, s: &str, color: u32, scale: i32) {
    let mut cx = x;
    for ch in s.chars() {
        let g = glyph(ch);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..8 {
                if bits & (1 << col) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = cx + col * scale + dx;
                            let py = y + row as i32 * scale + dy;
                            if px >= 0 && py >= 0 && (px as usize) < CW && (py as usize) < CH {
                                cv[py as usize * CW + px as usize] = color | 0xFF00_0000;
                            }
                        }
                    }
                }
            }
        }
        cx += 8 * scale;
    }
}

pub fn text_shadow(cv: &mut [u32], x: i32, y: i32, s: &str, color: u32, scale: i32) {
    text(cv, x + scale, y + scale, s, 0x000000, scale);
    text(cv, x, y, s, color, scale);
}

fn rect(cv: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32) {
    for yy in y.max(0)..(y + h).min(CH as i32) {
        for xx in x.max(0)..(x + w).min(CW as i32) {
            cv[yy as usize * CW + xx as usize] = color | 0xFF00_0000;
        }
    }
}

/// Draws a translucent rectangle ("over" compositing on the ARGB overlay layer).
fn shade(cv: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32, alpha: u32) {
    let sa = alpha.min(255);
    for yy in y.max(0)..(y + h).min(CH as i32) {
        for xx in x.max(0)..(x + w).min(CW as i32) {
            let p = &mut cv[yy as usize * CW + xx as usize];
            let da = *p >> 24;
            let oa = sa + da * (255 - sa) / 255;
            if oa == 0 {
                continue;
            }
            let mut o = 0;
            for sh in [0, 8, 16] {
                let s = (color >> sh) & 0xFF;
                let d = (*p >> sh) & 0xFF;
                let v = (s * sa + d * da * (255 - sa) / 255) / oa;
                o |= v.min(255) << sh;
            }
            *p = o | oa << 24;
        }
    }
}

fn frame_rect(cv: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32) {
    rect(cv, x, y, w, 1, color);
    rect(cv, x, y + h - 1, w, 1, color);
    rect(cv, x, y, 1, h, color);
    rect(cv, x + w - 1, y, 1, h, color);
}

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    None,
    Resize,
    SaveState,
    LoadState,
    Screenshot,
    Reset,
    Quit,
    Cursor,
    Fullscreen,
}

#[derive(Clone, Copy, PartialEq)]
enum Opt {
    Wide,
    Smooth,
    Upscale,
    Fullscreen,
    Aspect,
    Scale,
    IntScale,
    Filter,
    Color,
    Fps,
    Volume,
    FfSound,
    FfSpeed,
    FfToggle,
    PauseBg,
    Mouse,
    HideCursor,
    Slot,
    Recomp,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Opt(Opt),
    Act(Action),
    Bind(usize),
    DefaultBinds,
}

struct Row {
    label: &'static str,
    value: String,
    kind: Kind,
}

const PAGES: [&str; 5] = ["Bild", "Ton", "Steuerung", "Komfort", "System"];
const TAB_W: i32 = 80;
const TAB_X: i32 = 40;
const TAB_Y: i32 = 50;
const ROW_Y: i32 = 76;
const ROW_H: i32 = 14;
const VAL_X: i32 = 250;
const BIND2_X: i32 = 350;

fn onoff(b: bool) -> String {
    if b { "An".into() } else { "Aus".into() }
}

fn rows(page: usize, s: &Settings, recomp_active: bool) -> Vec<Row> {
    let r = |label, value: String, kind| Row { label, value, kind };
    match page {
        0 => vec![
            r("Vollbild  (F11)", onoff(s.fullscreen), Kind::Opt(Opt::Fullscreen)),
            r("Breitbild", ["Aus", "Auto (Karten 16:9)", "16:9", "16:10", "21:9"][s.widescreen as usize].into(), Kind::Opt(Opt::Wide)),
            r("Bildformat", [
                "3:2 mit Rändern",
                "3:2 + unscharfer Rand",
                "Strecken (füllend)",
                "Zoomen (abgeschnitten)",
            ][s.aspect as usize].into(), Kind::Opt(Opt::Aspect)),
            r("Fenstergröße", format!("{}x  ({}x{})", s.scale, 240 * s.scale, 160 * s.scale), Kind::Opt(Opt::Scale)),
            r("Pixel-Skalierung", if s.integer_scale { "Ganzzahlig (scharf)".into() } else { "Frei".into() }, Kind::Opt(Opt::IntScale)),
            r("Glättung", if s.smooth { "Scharf-weich (empfohlen)".into() } else { "Aus (harte Pixel)".into() }, Kind::Opt(Opt::Smooth)),
            r("Hochskalierung", ["Aus (Original)", "Scale2x", "Scale4x (rund)"][s.upscale as usize].into(), Kind::Opt(Opt::Upscale)),
            r("Filter", ["Aus", "Scanlines", "LCD-Raster"][s.filter as usize].into(), Kind::Opt(Opt::Filter)),
            r("Farbkorrektur", if s.color_correct { "GBA-Display".into() } else { "Aus (kräftig)".into() }, Kind::Opt(Opt::Color)),
            r("FPS anzeigen", onoff(s.show_fps), Kind::Opt(Opt::Fps)),
        ],
        1 => vec![
            r("Lautstärke", format!("{}%", s.volume as u32 * 10), Kind::Opt(Opt::Volume)),
            r("Ton beim Vorspulen", onoff(s.ff_sound), Kind::Opt(Opt::FfSound)),
        ],
        2 => {
            let mut v = vec![
                r("Maussteuerung", onoff(s.mouse), Kind::Opt(Opt::Mouse)),
                r("Mauszeiger", if s.hide_cursor { "Ausblenden".into() } else { "Anzeigen".into() }, Kind::Opt(Opt::HideCursor)),
            ];
            for (i, (name, _)) in BUTTONS.iter().enumerate() {
                v.push(r(name, String::new(), Kind::Bind(i)));
            }
            v.push(r("Standardbelegung wiederherstellen", String::new(), Kind::DefaultBinds));
            v
        }
        3 => vec![
            r("Vorspul-Tempo", match FF_SPEEDS[s.ff_speed] {
                0 => "Maximum".into(),
                n => format!("{}x", n),
            }, Kind::Opt(Opt::FfSpeed)),
            r("Vorspulen (Leertaste)", if s.ff_toggle { "Umschalten".into() } else { "Halten".into() }, Kind::Opt(Opt::FfToggle)),
            r("Im Hintergrund pausieren", onoff(s.pause_unfocused), Kind::Opt(Opt::PauseBg)),
            r("Savestate-Slot", format!("{}", s.slot), Kind::Opt(Opt::Slot)),
        ],
        _ => vec![
            r("Savestate speichern  (F5)", String::new(), Kind::Act(Action::SaveState)),
            r("Savestate laden      (F8)", String::new(), Kind::Act(Action::LoadState)),
            r("Screenshot speichern (F12)", String::new(), Kind::Act(Action::Screenshot)),
            r("Spiel zurücksetzen (Reset)", String::new(), Kind::Act(Action::Reset)),
            r("Ausführung", if recomp_active { "Rekompilierter Rust-Code".into() } else { "Interpreter".into() }, Kind::Opt(Opt::Recomp)),
            r("Beenden", String::new(), Kind::Act(Action::Quit)),
        ],
    }
}

fn change(s: &mut Settings, o: Opt, d: i32) -> Action {
    let cyc = |v: i32, n: i32| (v + d).rem_euclid(n);
    match o {
        Opt::Fullscreen => {
            s.fullscreen = !s.fullscreen;
            return Action::Fullscreen;
        }
        Opt::Smooth => s.smooth = !s.smooth,
        Opt::Wide => s.widescreen = cyc(s.widescreen as i32, 5) as u8,
        Opt::Upscale => s.upscale = cyc(s.upscale as i32, 3) as u8,
        Opt::Aspect => s.aspect = cyc(s.aspect as i32, 4) as u8,
        Opt::Scale => {
            s.scale = (s.scale as i32 + d).clamp(2, 8) as usize;
            return Action::Resize;
        }
        Opt::IntScale => s.integer_scale = !s.integer_scale,
        Opt::Filter => s.filter = cyc(s.filter as i32, 3) as u8,
        Opt::Color => s.color_correct = !s.color_correct,
        Opt::Fps => s.show_fps = !s.show_fps,
        Opt::Volume => s.volume = (s.volume as i32 + d).clamp(0, 10) as u8,
        Opt::FfSound => s.ff_sound = !s.ff_sound,
        Opt::FfSpeed => s.ff_speed = cyc(s.ff_speed as i32, FF_SPEEDS.len() as i32) as usize,
        Opt::FfToggle => s.ff_toggle = !s.ff_toggle,
        Opt::PauseBg => s.pause_unfocused = !s.pause_unfocused,
        Opt::Mouse => s.mouse = !s.mouse,
        Opt::HideCursor => {
            s.hide_cursor = !s.hide_cursor;
            return Action::Cursor;
        }
        Opt::Slot => s.slot = (cyc(s.slot as i32 - 1, 5) + 1) as u8,
        Opt::Recomp => s.recomp = !s.recomp,
    }
    Action::None
}

pub struct MenuInput {
    pub keys: Vec<Key>,
    pub shift: bool,
    pub pos: Option<(i32, i32)>, // canvas coordinates
    pub left: bool,
    pub right: bool,
    pub wheel: i32,
    /// The physical mouse moved since the last frame.
    pub moved: bool,
}

#[derive(Default)]
pub struct Menu {
    pub open: bool,
    page: usize,
    sel: usize,
    bind_col: usize,
    capture: Option<(usize, usize)>,
    toast: Option<(String, Instant)>,
    last_pos: Option<(i32, i32)>,
}

impl Menu {
    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    fn activate(&mut self, s: &mut Settings, kind: Kind, d: i32) -> Action {
        match kind {
            Kind::Opt(o) => change(s, o, d),
            Kind::Act(a) => a,
            Kind::Bind(i) => {
                self.capture = Some((i, self.bind_col));
                Action::None
            }
            Kind::DefaultBinds => {
                s.binds = crate::settings::default_binds();
                self.toast("Standardbelegung wiederhergestellt");
                Action::None
            }
        }
    }

    /// Handles input while the menu is open.
    pub fn input(&mut self, s: &mut Settings, inp: &MenuInput, recomp_active: bool) -> Action {
        let rs = rows(self.page, s, recomp_active);
        if let Some((b, slot)) = self.capture {
            for &k in &inp.keys {
                if k == Key::Escape || k == Key::F1 {
                    self.capture = None;
                    return Action::None;
                }
                if BINDABLE.contains(&k) {
                    // a key drives only one button
                    for bb in s.binds.iter_mut() {
                        for x in bb.iter_mut() {
                            if *x == Some(k) {
                                *x = None;
                            }
                        }
                    }
                    s.binds[b][slot] = Some(k);
                    self.capture = None;
                    return Action::None;
                }
            }
            if inp.right {
                self.capture = None;
            }
            return Action::None;
        }
        let mut act = Action::None;
        for &k in &inp.keys {
            let kind = rs.get(self.sel).map(|r| r.kind);
            match k {
                Key::Escape | Key::F1 => {
                    self.open = false;
                    s.save();
                }
                Key::Up => self.sel = (self.sel + rs.len() - 1) % rs.len(),
                Key::Down => self.sel = (self.sel + 1) % rs.len(),
                Key::Tab => {
                    self.page = if inp.shift { (self.page + PAGES.len() - 1) % PAGES.len() } else { (self.page + 1) % PAGES.len() };
                    self.sel = 0;
                }
                Key::Left | Key::Right => {
                    let d = if k == Key::Left { -1 } else { 1 };
                    match kind {
                        Some(Kind::Opt(o)) => act = change(s, o, d),
                        Some(Kind::Bind(_)) => self.bind_col = if d > 0 { 1 } else { 0 },
                        _ => {}
                    }
                }
                Key::Enter | Key::NumPadEnter | Key::Space => {
                    if let Some(kd) = kind {
                        act = self.activate(s, kd, 1);
                    }
                }
                Key::Delete | Key::Backspace => {
                    if let Some(Kind::Bind(i)) = kind {
                        s.binds[i][self.bind_col] = None;
                    }
                }
                _ => {}
            }
        }
        // mouse
        let moved = inp.moved && inp.pos != self.last_pos;
        self.last_pos = inp.pos;
        if inp.wheel != 0 {
            self.sel = (self.sel as i32 - inp.wheel).rem_euclid(rs.len() as i32) as usize;
        }
        if let Some((x, y)) = inp.pos {
            if y >= TAB_Y && y < TAB_Y + 16 && x >= TAB_X && x < TAB_X + TAB_W * PAGES.len() as i32 {
                if inp.left {
                    self.page = ((x - TAB_X) / TAB_W) as usize;
                    self.sel = 0;
                }
            } else if y >= ROW_Y && x >= 32 && x < 448 {
                let i = ((y - ROW_Y) / ROW_H) as usize;
                if i < rs.len() {
                    if moved {
                        self.sel = i;
                    }
                    if let Kind::Bind(_) = rs[i].kind {
                        if moved || inp.left {
                            self.bind_col = if x >= BIND2_X { 1 } else { 0 };
                        }
                    }
                    if inp.left {
                        self.sel = i;
                        act = self.activate(s, rs[i].kind, 1);
                    } else if inp.right {
                        if let Kind::Opt(o) = rs[i].kind {
                            act = change(s, o, -1);
                        }
                    }
                }
            } else if inp.right {
                self.open = false;
                s.save();
            }
        }
        if self.sel >= rs.len() {
            self.sel = 0;
        }
        act
    }

    pub fn draw(&self, cv: &mut [u32], s: &Settings, recomp_active: bool) {
        if self.open {
            shade(cv, 0, 0, CW as i32, CH as i32, 0x000000, 150);
            shade(cv, 24, 16, 432, 288, 0x0C1830, 230);
            frame_rect(cv, 24, 16, 432, 288, 0x7090D0);
            frame_rect(cv, 26, 18, 428, 284, 0x30406A);
            text_shadow(cv, 40, 26, "Einstellungen", 0xFFE8A0, 2);
            text(cv, 300, 30, "F1 / Esc: zurück", 0x8090B0, 1);
            for (i, p) in PAGES.iter().enumerate() {
                let x = TAB_X + i as i32 * TAB_W;
                let on = i == self.page;
                if on {
                    rect(cv, x, TAB_Y, TAB_W - 4, 16, 0x3050A0);
                }
                frame_rect(cv, x, TAB_Y, TAB_W - 4, 16, if on { 0xA0C0FF } else { 0x405078 });
                let tw = p.chars().count() as i32 * 8;
                text(cv, x + (TAB_W - 4 - tw) / 2, TAB_Y + 4, p, if on { 0xFFFFFF } else { 0x90A0C0 }, 1);
            }
            let rs = rows(self.page, s, recomp_active);
            for (i, r) in rs.iter().enumerate() {
                let y = ROW_Y + i as i32 * ROW_H;
                let sel = i == self.sel;
                if sel {
                    rect(cv, 32, y - 3, 416, ROW_H, 0x24407C);
                }
                let lc = if sel { 0xFFFFFF } else { 0xC8D0E0 };
                match r.kind {
                    Kind::Bind(b) => {
                        text(cv, 48, y, &format!("GBA {}", r.label), lc, 1);
                        for slot in 0..2 {
                            let x = if slot == 0 { VAL_X - 12 } else { BIND2_X };
                            let capturing = self.capture == Some((b, slot));
                            let v = if capturing {
                                "Taste drücken...".to_string()
                            } else {
                                s.binds[b][slot].map(key_name).unwrap_or_else(|| "-".into())
                            };
                            let active = sel && self.bind_col == slot;
                            if active || capturing {
                                frame_rect(cv, x - 4, y - 3, 100, ROW_H, if capturing { 0xFFD060 } else { 0x80A0E0 });
                            }
                            text(cv, x, y, &v, if capturing { 0xFFD060 } else { 0x9FE0A0 }, 1);
                        }
                    }
                    _ => {
                        text(cv, 48, y, r.label, lc, 1);
                        if !r.value.is_empty() {
                            let v = if sel { format!("< {} >", r.value) } else { r.value.clone() };
                            text(cv, VAL_X - if sel { 16 } else { 0 }, y, &v, 0x9FE0A0, 1);
                        }
                    }
                }
            }
            let help = match rs.get(self.sel).map(|r| r.kind) {
                Some(Kind::Bind(_)) => "Enter: neu belegen   Entf: löschen   </>: Spalte",
                Some(Kind::Act(_)) | Some(Kind::DefaultBinds) => "Enter/Klick: ausführen      Tab: nächste Seite",
                _ => "</> oder Klick: ändern      Tab: nächste Seite",
            };
            text(cv, 40, 286, help, 0x8090B0, 1);
        }
        if let Some((m, t)) = &self.toast {
            if t.elapsed() < Duration::from_millis(2500) {
                let w = m.chars().count() as i32 * 8 + 16;
                let x = (CW as i32 - w) / 2;
                shade(cv, x, CH as i32 - 34, w, 20, 0x000000, 180);
                text(cv, x + 8, CH as i32 - 28, m, 0xFFFFFF, 1);
            }
        }
    }
}
