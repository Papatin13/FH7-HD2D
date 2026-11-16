//! Godot extension: runs the recompiled Fire Emblem 7 and hands the HD-2D
//! frontend everything it needs (interface layer, map, units, sound, saves).
use godot::classes::image::Format;
use godot::classes::{IRefCounted, Image, RefCounted};
use godot::prelude::*;
use gba::{keys, Gba};
use std::path::PathBuf;

#[path = "../../fe7/src/mouse.rs"]
#[allow(dead_code)]
mod mouse;

use gba::text;

struct Ext;
#[gdextension]
unsafe impl ExtensionLibrary for Ext {}

const PROC_ARRAY: u32 = 0x0202_4E28;
const SCR_MAIN_MENU: u32 = 0x08CE_3C54;
const SCR_SAVE_MENU: u32 = 0x08CE_3F24;
const MAIN_MENU_LOOP: u32 = 0x080A_39F9;
const SLOT_SELECT_LOOP: u32 = 0x080A_3E99;
const SCR_TITLE_CONFIRM: u32 = 0x08CC_2014;
/// "Personal Information" screen of a new game and the yes/no choice in its text box.
const SCR_PERSONAL: u32 = 0x08CE_45E4;
const SCR_TALK_CHOICE: u32 = 0x08CC_3174;
/// gPlaySt: tactician name (+0x20, 11 bytes), birth month (+0x2B high nibble), gender (+0x2C).
const PLAYST: u32 = 0x0202_BBF8;
/// Text of the current dialogue (decoded).
const TALK_TEXT: u32 = 0x0202_A9B4;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Drive {
    None,
    /// Skip intro and title until the main menu waits for input.
    ToMainMenu,
    /// Choose the main menu entry with this bit (stops at the save-slot screen).
    Choose(u8),
    /// On the save-slot screen: move to this slot and confirm.
    Slot(u8),
    /// Leave the save-slot screen (B).
    Back,
    /// Personal information of a new game: skip the game's own screen
    /// (answer "No" = keep defaults), then write the frontend's values.
    Personal,
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct Fe7Core {
    base: Base<RefCounted>,
    g: Option<Box<Gba>>,
    rom: PathBuf,
    mouse: mouse::Mouse,
    minp: mouse::MouseInput,
    drive: Drive,
    drive_t: u32,
    sram_timer: u32,
    frames: u64,
    hd_on: bool,
    talk_native: bool,
    menu_native: bool,
    instant_text: bool,
    slot_done: bool,
    confirm_seen: bool,
    confirm_t: u32,
    queue: std::collections::VecDeque<Drive>,
    personal: Option<(Vec<u8>, u8, u8)>,
    personal_phase: u8,
    personal_t: u32,
}

#[godot_api]
impl IRefCounted for Fe7Core {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            g: None,
            rom: PathBuf::new(),
            mouse: Default::default(),
            minp: Default::default(),
            drive: Drive::None,
            drive_t: 0,
            sram_timer: 0,
            frames: 0,
            hd_on: true,
            talk_native: true,
            menu_native: true,
            instant_text: false,
            slot_done: false,
            confirm_seen: false,
            confirm_t: 0,
            queue: Default::default(),
            personal: None,
            personal_phase: 0,
            personal_t: 0,
        }
    }
}

/// With the `recomp` feature the blocks translated from the ROM run natively;
/// the distributed build has no such code and interprets the player's ROM.
fn attach_recomp(g: &mut Gba) {
    #[cfg(feature = "recomp")]
    {
        g.lookup = Some(fe7gen::lookup);
        g.use_recomp = true;
    }
    #[cfg(not(feature = "recomp"))]
    {
        g.lookup = None;
        g.use_recomp = false;
    }
}

/// Checks a ROM file: Fire Emblem (USA), the only version the addresses fit.
/// Returns an empty string when it is right, otherwise what is wrong.
fn rom_problem(p: &std::path::Path) -> String {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(p) else { return "Datei nicht lesbar.".into() };
    let mut h = [0u8; 0xC0];
    if f.read_exact(&mut h).is_err() {
        return "Keine GBA-ROM-Datei.".into();
    }
    match &h[0xAC..0xB0] {
        b"AE7E" => {}
        b"AE7P" => return "Das ist die europäische Version. Benötigt wird Fire Emblem (USA).".into(),
        b"AE7J" => return "Das ist die japanische Version. Benötigt wird Fire Emblem (USA).".into(),
        _ => return "Das ist kein Fire-Emblem-ROM (USA).".into(),
    }
    let Ok(d) = std::fs::read(p) else { return "Datei nicht lesbar.".into() };
    if crc32(&d) != ROM_CRC {
        return "Fire Emblem (USA), aber verändert (Patch/Hack?). Benötigt wird ein unverändertes ROM.".into();
    }
    String::new()
}

/// CRC32 of the unmodified Fire Emblem (USA) ROM.
const ROM_CRC: u32 = 0x2A524221;

fn crc32(d: &[u8]) -> u32 {
    let mut t = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        t[i as usize] = c;
    }
    let mut c = !0u32;
    for &b in d {
        c = t[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

fn find_rom(extra: &str) -> Option<PathBuf> {
    let mut dirs = Vec::new();
    // a ROM file chosen in the launcher
    let direct = PathBuf::from(extra);
    if direct.is_file() && rom_problem(&direct).is_empty() {
        return Some(direct);
    }
    if !extra.is_empty() {
        let p = PathBuf::from(extra);
        dirs.push(p.clone());
        let mut q = p.as_path();
        while let Some(par) = q.parent() {
            dirs.push(par.to_path_buf());
            q = par;
            if dirs.len() > 4 {
                break;
            }
        }
    }
    if let Ok(e) = std::env::current_exe() {
        let mut q = e.as_path();
        for _ in 0..3 {
            if let Some(par) = q.parent() {
                dirs.push(par.to_path_buf());
                q = par;
            }
        }
    }
    dirs.push(PathBuf::from("."));
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        v.sort();
        for p in v {
            if p.extension().map(|x| x.eq_ignore_ascii_case("gba")).unwrap_or(false) {
                if let Ok(mut f) = std::fs::File::open(&p) {
                    use std::io::Read;
                    let mut h = [0u8; 0xB0];
                    if f.read_exact(&mut h).is_ok() && &h[0xAC..0xB0] == b"AE7E" {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

fn r32(g: &mut Gba, a: u32) -> u32 {
    g.read32(a)
}

fn proc_with(g: &mut Gba, script: u32) -> Option<u32> {
    (0..64).map(|i| PROC_ARRAY + i * 0x6C).find(|&p| r32(g, p) == script)
}

/// The save-slot list: in the main menu, or the "Save" screen during the game
/// (chapter end, preparations) which shares its layout.
fn slot_proc(g: &mut Gba) -> Option<u32> {
    proc_with(g, SCR_MAIN_MENU).or_else(|| proc_with(g, SCR_SAVE_MENU))
}

fn rgba_bytes(px: &[u32]) -> PackedByteArray {
    let mut v = Vec::with_capacity(px.len() * 4);
    for c in px {
        v.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, *c as u8, (c >> 24) as u8]);
    }
    PackedByteArray::from(v.as_slice())
}

fn image(w: usize, h: usize, px: &[u32]) -> Option<Gd<Image>> {
    Image::create_from_data(w as i32, h as i32, false, Format::RGBA8, &rgba_bytes(px))
}

impl Fe7Core {
    fn sav_path(&self) -> PathBuf {
        self.rom.with_extension("sav")
    }

    fn drive_keys(&mut self) -> u16 {
        if self.drive == Drive::None {
            if let Some(d) = self.queue.pop_front() {
                self.drive = d;
                self.drive_t = 0;
                self.slot_done = false;
                self.confirm_seen = false;
            }
        }
        let g = self.g.as_mut().unwrap();
        self.drive_t += 1;
        let t = self.drive_t;
        match self.drive {
            Drive::None => 0,
            Drive::ToMainMenu => {
                if let Some(p) = proc_with(g, SCR_MAIN_MENU) {
                    if r32(g, p + 0x0C) == MAIN_MENU_LOOP {
                        self.drive = Drive::None;
                    }
                    return 0;
                }
                if t % 12 == 0 {
                    keys::START
                } else {
                    0
                }
            }
            Drive::Choose(bit) => {
                let Some(p) = proc_with(g, SCR_MAIN_MENU) else {
                    if t > 600 {
                        self.drive = Drive::None;
                    }
                    return 0;
                };
                if r32(g, p + 0x0C) != MAIN_MENU_LOOP {
                    if r32(g, p + 0x0C) == SLOT_SELECT_LOOP || t > 600 {
                        self.drive = Drive::None;
                    }
                    return 0;
                }
                let mask = g.read8(p + 0x30) as u32;
                if mask & (1 << bit) == 0 || t > 900 {
                    self.drive = Drive::None;
                    return 0;
                }
                let want = (mask & ((1u32 << bit) - 1)).count_ones() as u8;
                let cur = g.read8(p + 0x2B);
                if t % 6 != 0 {
                    return 0;
                }
                if cur < want {
                    keys::DOWN
                } else if cur > want {
                    keys::UP
                } else {
                    keys::A
                }
            }
            Drive::Slot(slot) => {
                if std::env::var("FE7HD_DEBUG").is_ok() && t % 10 == 0 {
                    let p = slot_proc(g).unwrap_or(0);
                    eprintln!("slot t{} idle {:08x} cur {} q36 {} confirm {} done {}", t, if p != 0 { r32(g, p + 0x0C) } else { 0 }, if p != 0 { g.read8(p + 0x2C) } else { 0 }, if p != 0 { g.read8(p + 0x36) } else { 0 }, proc_with(g, SCR_TITLE_CONFIRM).is_some(), self.slot_done);
                }
                let Some(p) = slot_proc(g) else {
                    // the chapter is starting
                    self.drive = Drive::None;
                    return 0;
                };
                let idle = r32(g, p + 0x0C);
                if t > 900 || idle == MAIN_MENU_LOOP {
                    self.drive = Drive::None;
                    return 0;
                }
                // the game's own "Start / Quit" question: answer Start
                if proc_with(g, SCR_TITLE_CONFIRM).is_some() {
                    self.confirm_seen = true;
                    self.confirm_t = t;
                    if t % 8 == 0 {
                        return if g.read8(p + 0x36) != 1 { keys::LEFT } else { keys::A };
                    }
                    return 0;
                }
                // erase question ("Erase / Quit") is shown inside the slot list: answer Erase
                if idle == SLOT_SELECT_LOOP && self.slot_done && g.read8(p + 0x36) != 0 {
                    self.confirm_seen = true;
                    self.confirm_t = t;
                    if t % 8 == 0 {
                        return if g.read8(p + 0x36) != 1 { keys::LEFT } else { keys::A };
                    }
                    return 0;
                }
                if idle == SLOT_SELECT_LOOP {
                    if self.slot_done {
                        // still on the list: the question may come a moment later;
                        // after erasing we are back here for good
                        // (several questions can follow each other, e.g. "suspended data will
                        // be lost" and "overwrite?": wait until none has come for a while)
                        if (self.confirm_seen && t > self.confirm_t + 90) || t > 400 {
                            self.drive = Drive::None;
                        }
                        return 0;
                    }
                    if t % 6 != 0 {
                        return 0;
                    }
                    let cur = g.read8(p + 0x2C);
                    if t > 120 && cur != slot {
                        // the game does not allow this slot here
                        self.drive = Drive::None;
                        return 0;
                    }
                    return if cur < slot {
                        keys::DOWN
                    } else if cur > slot {
                        keys::UP
                    } else {
                        self.slot_done = true;
                        keys::A
                    };
                }
                0
            }
            Drive::Personal => {
                let active = proc_with(g, SCR_PERSONAL).is_some();
                if std::env::var("FE7HD_DEBUG").is_ok() && t % 30 == 0 {
                    eprintln!("personal t{} phase {} active {} choice {}", t, self.personal_phase, active, proc_with(g, SCR_TALK_CHOICE).is_some());
                }
                match self.personal_phase {
                    0 => {
                        if !active || t > 3000 {
                            self.personal_phase = 1;
                            self.personal_t = t;
                            return 0;
                        }
                        if proc_with(g, SCR_TALK_CHOICE).is_some() {
                            // "Enter information?" -> No (keep the defaults, we set our own);
                            // any other question ("Are these the settings ...?") -> Yes
                            let q: Vec<u8> = (0..5).map(|i| g.read8(TALK_TEXT + i)).collect();
                            let no = &q[..] == b"Enter";
                            return match t % 24 {
                                0 => if no { keys::RIGHT } else { keys::LEFT },
                                12 => keys::A,
                                _ => 0,
                            };
                        }
                        // advance the explanation texts with B (never selects anything)
                        if t % 16 == 0 { keys::B } else { 0 }
                    }
                    _ => {
                        if let Some((name, month, gender)) = self.personal.clone() {
                            for i in 0..11u32 {
                                g.write8(PLAYST + 0x20 + i, name.get(i as usize).copied().unwrap_or(0));
                            }
                            let b = g.read8(PLAYST + 0x2B);
                            g.write8(PLAYST + 0x2B, (b & 0x0F) | (month.min(11) << 4));
                            g.write8(PLAYST + 0x2C, gender & 1);
                        }
                        if t > self.personal_t + 120 {
                            self.drive = Drive::None;
                            self.personal = None;
                        }
                        0
                    }
                }
            }
            Drive::Back => {
                let Some(p) = slot_proc(g) else {
                    self.drive = Drive::None;
                    return 0;
                };
                let idle = r32(g, p + 0x0C);
                if idle == MAIN_MENU_LOOP || t > 600 {
                    self.drive = Drive::None;
                    return 0;
                }
                if idle == SLOT_SELECT_LOOP && t % 10 == 1 {
                    keys::B
                } else {
                    0
                }
            }
        }
    }
}

#[godot_api]
impl Fe7Core {
    /// Finds the ROM (next to the program, in `hint` or its parents) and starts the game.
    /// Returns an empty string on success, else an error message.
    #[func]
    fn start(&mut self, hint: GString) -> GString {
        let Some(rom) = find_rom(&hint.to_string()) else {
            return "Kein Fire-Emblem-ROM (USA) gefunden.".into();
        };
        let data = match std::fs::read(&rom) {
            Ok(d) => d,
            Err(e) => return format!("ROM nicht lesbar: {e}").as_str().into(),
        };
        let mut g = Gba::new(data);
        attach_recomp(&mut g);
        g.hd.on = self.hd_on;
        g.hd.talk_native = self.talk_native;
        g.hd.menu_native = self.menu_native;
        g.hd.instant_text = self.instant_text;
        self.rom = rom;
        if let Ok(d) = std::fs::read(self.sav_path()) {
            let n = d.len().min(g.sram.len());
            g.sram[..n].copy_from_slice(&d[..n]);
        }
        self.g = Some(g);
        self.drive = Drive::ToMainMenu;
        self.drive_t = 0;
        GString::new()
    }

    /// Bytes of the loaded ROM (offset from the ROM start). Used to put the
    /// window frames together from the player's own ROM at runtime.
    #[func]
    fn rom_bytes(&self, off: i64, len: i64) -> PackedByteArray {
        let Some(g) = self.g.as_ref() else { return PackedByteArray::new() };
        let (o, n) = (off.max(0) as usize, len.max(0) as usize);
        let e = (o + n).min(g.rom.len());
        PackedByteArray::from(g.rom.get(o..e).unwrap_or(&[]))
    }

    /// LZ77 (type 0x10) data of the loaded ROM, decompressed.
    #[func]
    fn rom_lz77(&self, off: i64) -> PackedByteArray {
        let Some(g) = self.g.as_ref() else { return PackedByteArray::new() };
        let r = &g.rom;
        let o = off.max(0) as usize;
        if o + 4 > r.len() || r[o] != 0x10 {
            return PackedByteArray::new();
        }
        let size = r[o + 1] as usize | (r[o + 2] as usize) << 8 | (r[o + 3] as usize) << 16;
        let mut out: Vec<u8> = Vec::with_capacity(size);
        let mut p = o + 4;
        'outer: while out.len() < size {
            let Some(&fl) = r.get(p) else { break };
            p += 1;
            for b in 0..8 {
                if out.len() >= size {
                    break;
                }
                if fl & (0x80 >> b) != 0 {
                    let (Some(&a), Some(&c)) = (r.get(p), r.get(p + 1)) else { break 'outer };
                    p += 2;
                    let n = (a as usize >> 4) + 3;
                    let disp = ((a as usize & 0xF) << 8 | c as usize) + 1;
                    if disp > out.len() {
                        break 'outer;
                    }
                    for _ in 0..n {
                        let v = out[out.len() - disp];
                        out.push(v);
                    }
                } else {
                    let Some(&v) = r.get(p) else { break 'outer };
                    out.push(v);
                    p += 1;
                }
            }
        }
        out.truncate(size);
        PackedByteArray::from(out.as_slice())
    }

    /// Launcher: checks a chosen ROM file. {ok, msg}.
    #[func]
    fn check_rom(&self, path: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        let msg = rom_problem(std::path::Path::new(&path.to_string()));
        d.set("ok", msg.is_empty());
        d.set("msg", msg.as_str());
        d
    }

    /// Launcher: looks for a fitting ROM near the game. Empty when none.
    #[func]
    fn find_rom(&self, hint: GString) -> GString {
        find_rom(&hint.to_string()).map(|p| p.display().to_string()).unwrap_or_default().as_str().into()
    }

    /// True when this build carries recompiled game code (must not be distributed).
    #[func]
    fn has_recomp(&self) -> bool {
        cfg!(feature = "recomp")
    }

    #[func]
    fn rom_path(&self) -> GString {
        self.rom.display().to_string().as_str().into()
    }

    /// Restart from power-on and go to the main menu (invisible).
    #[func]
    fn reset_to_menu(&mut self) {
        if let Some(g) = self.g.as_mut() {
            let sram = g.sram.clone();
            let rom = std::mem::take(&mut g.rom);
            let mut n = Gba::new(rom);
            attach_recomp(&mut n);
            n.hd.on = self.hd_on;
            n.hd.talk_native = self.talk_native;
            n.hd.menu_native = self.menu_native;
            n.sram = sram;
            n.hd.instant_text = self.instant_text;
            n.wide_ext = g.wide_ext;
            self.g = Some(n);
            self.drive = Drive::ToMainMenu;
            self.drive_t = 0;
            self.mouse = Default::default();
        }
    }

    /// HD-2D presentation on/off (off = original picture everywhere).
    #[func]
    fn set_hd(&mut self, on: bool) {
        self.hd_on = on;
        if let Some(g) = self.g.as_mut() {
            g.hd.on = on;
        }
    }

    /// Still skipping intro / choosing a menu entry automatically?
    #[func]
    fn busy(&self) -> bool {
        self.drive != Drive::None || !self.queue.is_empty()
    }

    /// Main menu entries available (bit 0 resume, 1 restart, 2 copy, 3 erase, 4 new game).
    #[func]
    fn menu_mask(&mut self) -> i32 {
        let g = self.g.as_mut().unwrap();
        match proc_with(g, SCR_MAIN_MENU) {
            Some(p) => g.read8(p + 0x30) as i32,
            None => 0,
        }
    }

    #[func]
    fn choose(&mut self, bit: i32) {
        self.drive = Drive::Choose(bit as u8);
        self.drive_t = 0;
    }

    /// The game waits on its save-slot screen (the frontend shows its own).
    #[func]
    fn in_slot_select(&mut self) -> bool {
        let g = self.g.as_mut().unwrap();
        match slot_proc(g) {
            Some(p) => r32(g, p + 0x0C) == SLOT_SELECT_LOOP,
            None => false,
        }
    }

    /// Save slots: 3 x (chapter id or -1 if empty, play time in frames).
    #[func]
    fn slot_info(&mut self) -> PackedInt32Array {
        let g = self.g.as_mut().unwrap();
        let mut v = Vec::new();
        if let Some(p) = slot_proc(g) {
            for i in 0..3u32 {
                let ch = g.read8(p + 0x37 + i);
                v.push(if ch == 0xFF { -1 } else { ch as i32 });
                v.push(g.read32(p + 0x48 + 4 * i) as i32);
            }
        }
        PackedInt32Array::from(v.as_slice())
    }

    /// Chapter title from the game's text (e.g. "Ch. 1: Footsteps of Fate").
    #[func]
    fn chapter_title(&self, id: i32) -> GString {
        let g = self.g.as_ref().unwrap();
        text::decode(&g.rom, 0x58C + id.clamp(0, 0x7F) as u32).as_str().into()
    }

    /// Any text of the game by id.
    #[func]
    fn text(&self, id: i32) -> GString {
        let g = self.g.as_ref().unwrap();
        text::decode(&g.rom, id.max(0) as u32).as_str().into()
    }

    #[func]
    fn slot_pick(&mut self, slot: i32) {
        self.drive = Drive::Slot(slot.clamp(0, 2) as u8);
        self.slot_done = false;
        self.confirm_seen = false;
        self.drive_t = 0;
    }

    /// The game shows its "Personal Information" screen (new game).
    #[func]
    fn personal_active(&mut self) -> bool {
        let g = self.g.as_mut().unwrap();
        proc_with(g, SCR_PERSONAL).is_some()
    }

    /// Current tactician data: [name, month 0-11, gender 0 male / 1 female].
    #[func]
    fn personal_get(&mut self) -> VarArray {
        let g = self.g.as_mut().unwrap();
        let mut name = String::new();
        for i in 0..11u32 {
            let c = g.read8(PLAYST + 0x20 + i);
            if c == 0 {
                break;
            }
            name.push(c as char);
        }
        let month = (g.read8(PLAYST + 0x2B) >> 4) as i64;
        let gender = (g.read8(PLAYST + 0x2C) & 1) as i64;
        varray![name.as_str(), month, gender]
    }

    /// Finish the personal information with these values.
    #[func]
    fn personal_set(&mut self, name: GString, month: i32, gender: i32) {
        let n: Vec<u8> = name.to_string().bytes().filter(|b| (0x20..0x7F).contains(b)).take(7).collect();
        self.personal = Some((n, month.clamp(0, 11) as u8, (gender & 1) as u8));
        self.personal_phase = 0;
        self.drive = Drive::Personal;
        self.drive_t = 0;
    }

    /// New game in an occupied slot: erase it first, then start the new game there.
    #[func]
    fn slot_overwrite(&mut self, slot: i32) {
        let sl = slot.clamp(0, 2) as u8;
        self.queue.clear();
        self.drive = Drive::Back;
        self.drive_t = 0;
        self.queue.extend([Drive::Choose(3), Drive::Slot(sl), Drive::Back, Drive::Choose(4), Drive::Slot(sl)]);
    }

    #[func]
    fn slot_back(&mut self) {
        self.drive = Drive::Back;
        self.drive_t = 0;
    }

    /// Extra columns per side for the interface layer (window wider than 3:2).
    #[func]
    fn set_ext(&mut self, e: i32) {
        if let Some(g) = self.g.as_mut() {
            g.wide_ext = e.clamp(0, 96) as u32;
        }
    }

    /// Runs `n` frames with these GBA buttons held. Returns false if no game.
    #[func]
    fn run(&mut self, n: i32, held: i32) -> bool {
        if self.g.is_none() {
            return false;
        }
        for i in 0..n.max(1) {
            let fast = self.drive == Drive::ToMainMenu;
            let dk = self.drive_keys();
            let g = self.g.as_mut().unwrap();
            g.no_render = fast;
            if let Some(c) = g.debug_chapter {
                // debug chapter: keep it set until StartChapter takes it, so the
                // chapter's intro scenes already belong to it
                g.write8(0x0202_BC06, c);
            }
            let mk = if i == 0 && self.drive == Drive::None {
                let inp = std::mem::take(&mut self.minp);
                self.minp.pos = inp.pos;
                self.mouse.update(g, &inp)
            } else {
                0
            };
            g.keys = if self.drive != Drive::None { dk } else { held as u16 | mk };
            g.run_frame();
            self.frames += 1;
            if g.sram_dirty {
                self.sram_timer += 1;
                if self.sram_timer > 30 {
                    self.sram_timer = 0;
                    g.sram_dirty = false;
                    let _ = std::fs::write(self.rom.with_extension("sav"), &g.sram[..0x8000]);
                }
            }
        }
        true
    }

    /// Mouse over the interface layer, in frame pixels (x in 0..frame width).
    #[func]
    fn mouse_ui(&mut self, x: f32, y: f32) {
        let g = self.g.as_ref().unwrap();
        let gx = x as i32 - g.ppu.ext_left as i32;
        self.minp.pos = Some(mouse::unshift(g, (gx, y as i32)));
    }

    /// Mouse over the map, in map pixels.
    #[func]
    fn mouse_map(&mut self, mx: f32, my: f32) {
        let g = self.g.as_mut().unwrap();
        let camx = g.read16(0x0202_BBB8 + 0x0C) as i16 as i32;
        let camy = g.read16(0x0202_BBB8 + 0x0E) as i16 as i32;
        self.minp.pos = Some((mx.floor() as i32 - camx, my.floor() as i32 - camy));
    }

    #[func]
    fn mouse_none(&mut self) {
        self.minp.pos = None;
    }

    #[func]
    fn mouse_buttons(&mut self, left: bool, right: bool, middle: bool, wheel: i32) {
        self.minp.left |= left;
        self.minp.right |= right;
        self.minp.middle |= middle;
        self.minp.wheel += wheel;
    }

    #[func]
    fn mouse_mode(&self) -> GString {
        self.mouse.mode.into()
    }

    /// Interface layer (map scenes) or full picture (everything else).
    #[func]
    fn frame(&self) -> Option<Gd<Image>> {
        let g = self.g.as_ref()?;
        let w = g.ppu.width;
        if g.hd.map {
            image(w, 160, &g.ppu.frame[..w * 160])
        } else {
            let px: Vec<u32> = g.ppu.frame[..w * 160].iter().map(|c| c | 0xFF00_0000).collect();
            image(w, 160, &px)
        }
    }

    #[func]
    fn frame_width(&self) -> i32 {
        self.g.as_ref().map(|g| g.ppu.width as i32).unwrap_or(240)
    }

    /// Columns of the interface layer left of the original 240 px view.
    #[func]
    fn ext_left(&self) -> i32 {
        self.g.as_ref().map(|g| g.ppu.ext_left as i32).unwrap_or(0)
    }

    #[func]
    fn is_map(&self) -> bool {
        self.g.as_ref().map(|g| g.hd.map).unwrap_or(false)
    }

    /// Map pixel at the top-left of the game's 240x160 view.
    #[func]
    fn view(&self) -> Vector2i {
        let g = self.g.as_ref().unwrap();
        Vector2i::new(g.hd.view.0, g.hd.view.1)
    }

    /// Map cursor (map pixels) and z = 1 if shown.
    #[func]
    fn cursor(&self) -> Vector3i {
        let g = self.g.as_ref().unwrap();
        match g.hd.cursor {
            Some((x, y)) => Vector3i::new(x, y, 1),
            None => Vector3i::new(0, 0, 0),
        }
    }

    #[func]
    fn fade(&self) -> i32 {
        self.g.as_ref().map(|g| g.hd.fade).unwrap_or(0)
    }

    /// Unit sprites: 7 ints each (x, y, w, h, atlas x, atlas y, flags: 1 semi, 2 flat) in map pixels.
    #[func]
    fn sprites(&self) -> PackedInt32Array {
        let g = self.g.as_ref().unwrap();
        let mut v = Vec::with_capacity(g.hd.sprites.len() * 7);
        for s in &g.hd.sprites {
            v.extend_from_slice(&[s.x, s.y, s.w, s.h, s.ax, s.ay, s.semi as i32 | (s.flat as i32) << 1]);
        }
        PackedInt32Array::from(v.as_slice())
    }

    #[func]
    fn atlas(&self) -> Option<Gd<Image>> {
        let g = self.g.as_ref()?;
        if g.hd.atlas.is_empty() {
            return None;
        }
        image(gba::hd::ATLAS, gba::hd::ATLAS, &g.hd.atlas)
    }

    /// Movement/attack range per map cell (0 none, 1 blue, 2 red).
    #[func]
    fn cells(&self) -> PackedByteArray {
        let g = self.g.as_ref().unwrap();
        PackedByteArray::from(g.hd.cells.as_slice())
    }

    #[func]
    fn map_size(&mut self) -> Vector2i {
        let g = self.g.as_mut().unwrap();
        let (w, h) = g.hd_map_size();
        Vector2i::new(w, h)
    }

    #[func]
    fn map_hash(&mut self) -> i64 {
        self.g.as_mut().unwrap().hd_map_hash() as i64
    }

    #[func]
    fn map_image(&mut self, margin: i32) -> Option<Gd<Image>> {
        let g = self.g.as_mut().unwrap();
        let (w, h) = g.hd_map_size();
        if w == 0 {
            return None;
        }
        let img = g.hd_map_image(margin);
        image(((w + 2 * margin) * 16) as usize, ((h + 2 * margin) * 16) as usize, &img)
    }

    /// Standee cut-outs (see gba::hd::hd_map_cutout), same size as map_image.
    #[func]
    fn map_cutout(&mut self, margin: i32, kinds: PackedByteArray) -> Option<Gd<Image>> {
        let g = self.g.as_mut().unwrap();
        let (w, h) = g.hd_map_size();
        if w == 0 {
            return None;
        }
        let img = g.hd_map_cutout(margin, kinds.as_slice());
        image(((w + 2 * margin) * 16) as usize, ((h + 2 * margin) * 16) as usize, &img)
    }

    /// Texture for the 3D buildings: the cut-out art with the ground holes of each
    /// cell filled with the nearest colour of the building (no grass on walls/roofs).
    #[func]
    fn building_tex(&mut self, margin: i32, kinds: PackedByteArray, strip: PackedByteArray) -> Option<Gd<Image>> {
        let g = self.g.as_mut().unwrap();
        let (w, h) = g.hd_map_size();
        if w == 0 {
            return None;
        }
        let mut img = g.hd_map_cutout(margin, kinds.as_slice());
        let full = g.hd_map_image(margin);
        let iw = ((w + 2 * margin) * 16) as usize;
        let ih = ((h + 2 * margin) * 16) as usize;
        let terr = self.terrain(margin);
        let strip = strip.as_slice();
        for cy in 0..ih / 16 {
            for cx in 0..iw / 16 {
                let at = |x: usize, y: usize| (cy * 16 + y) * iw + cx * 16 + x;
                let opaque = (0..256).filter(|&k| img[at(k % 16, k / 16)] >> 24 != 0).count();
                let t = terr.as_slice()[cy * iw / 16 + cx] & 0x7F;
                if opaque > 0 && strip.contains(&t) {
                    // grass and bushes in a wall's cell are not part of the wall either
                    // (their greens are not always the plain ground's)
                    let keep: Vec<u32> = (0..256).map(|k| img[at(k % 16, k / 16)]).collect();
                    let mut left = 0;
                    for k in 0..256 {
                        let c = keep[k];
                        let (r, gg, b) = ((c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF);
                        if c >> 24 != 0 && gg >= r && gg >= b && gg > r.min(b) + 30 {
                            img[at(k % 16, k / 16)] = 0;
                        } else if c >> 24 != 0 {
                            left += 1;
                        }
                    }
                    if left == 0 {
                        for k in 0..256 {
                            img[at(k % 16, k / 16)] = keep[k];
                        }
                    }
                }
                let opaque = (0..256).filter(|&k| img[at(k % 16, k / 16)] >> 24 != 0).count();
                if opaque == 0 {
                    // nothing of the building left in this cell: keep the original art
                    for k in 0..256 {
                        img[at(k % 16, k / 16)] = full[at(k % 16, k / 16)];
                    }
                    continue;
                }
                if opaque == 256 {
                    continue;
                }
                // grow the opaque pixels into the holes, one ring per pass
                for _ in 0..16 {
                    let prev: Vec<u32> = (0..256).map(|k| img[at(k % 16, k / 16)]).collect();
                    let mut changed = false;
                    for y in 0..16usize {
                        for x in 0..16usize {
                            if prev[y * 16 + x] >> 24 != 0 {
                                continue;
                            }
                            let n = [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)].iter().find_map(|&(dx, dy)| {
                                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                                if (0..16).contains(&nx) && (0..16).contains(&ny) {
                                    let c = prev[ny as usize * 16 + nx as usize];
                                    (c >> 24 != 0).then_some(c)
                                } else {
                                    None
                                }
                            });
                            if let Some(c) = n {
                                img[at(x, y)] = c;
                                changed = true;
                            }
                        }
                    }
                    if !changed {
                        break;
                    }
                }
            }
        }
        image(iw, ih, &img)
    }

    /// Terrain per cell incl. margin (scenery cells get their scenery terrain | 0x80).
    #[func]
    fn terrain(&mut self, margin: i32) -> PackedByteArray {
        let g = self.g.as_mut().unwrap();
        let (w, h) = g.hd_map_size();
        let mut t = g.hd_terrain(margin);
        let tw = w + 2 * margin;
        for cy in 0..h + 2 * margin {
            for cx in 0..tw {
                let i = (cy * tw + cx) as usize;
                if t[i] == 0xFF {
                    t[i] = g.hd_scenery_terrain(cx - margin, cy - margin) | 0x80;
                }
            }
        }
        PackedByteArray::from(t.as_slice())
    }

    /// Speech bubbles drawn by the frontend (the game's own bubble is hidden).
    #[func]
    fn set_talk_native(&mut self, on: bool) {
        self.talk_native = on;
        if let Some(g) = self.g.as_mut() {
            g.hd.talk_native = on;
        }
    }

    /// Current speech bubble in frame pixels, or an empty dictionary.
    /// Keys: rect (Rect2i), tail (Rect2i, size 0 if none), lines (PackedStringArray, BBCode),
    /// name (String), waiting (bool).
    #[func]
    fn talk(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_ref() else { return d };
        let Some(t) = g.hd.talk.as_ref() else { return d };
        let ox = g.ppu.ext_left as i32 + t.shift;
        d.set("rect", Rect2i::new(Vector2i::new(t.x0 + ox, t.y0), Vector2i::new(t.x1 - t.x0, t.y1 - t.y0)));
        let tail = match t.tail {
            Some((a, b, c, e)) => Rect2i::new(Vector2i::new(a + ox, b), Vector2i::new(c - a, e - b)),
            None => Rect2i::default(),
        };
        d.set("tail", tail);
        let lines: Vec<GString> = t.lines.iter().map(|l| GString::from(l.as_str())).collect();
        d.set("lines", &PackedStringArray::from(lines.as_slice()));
        let full: Vec<GString> = t.full.iter().map(|l| GString::from(l.as_str())).collect();
        d.set("fit", &PackedStringArray::from(full.as_slice()));
        d.set("name", &GString::from(t.name.as_str()));
        d.set("waiting", t.waiting);
        d.set("choice", t.choice);
        d.set("style", t.style as i32);
        d
    }

    /// Speech bubbles show each page at once.
    #[func]
    fn set_instant_text(&mut self, on: bool) {
        self.instant_text = on;
        if let Some(g) = self.g.as_mut() {
            g.hd.instant_text = on;
        }
    }

    #[func]
    fn set_menu_native(&mut self, on: bool) {
        self.menu_native = on;
        if let Some(g) = self.g.as_mut() {
            g.hd.menu_native = on;
        }
    }

    /// Menu windows in frame pixels: Array of Dictionaries
    /// {rect: Rect2i, cur: int, active: bool, items: Array of {pos: Vector2i, label, enabled, icon: bool, uses}}.
    #[func]
    fn menus(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(g) = self.g.as_ref() else { return out };
        for m in &g.hd.menus {
            let ox = g.ppu.ext_left as i32 + m.shift;
            let mut d = VarDictionary::new();
            d.set("rect", Rect2i::new(Vector2i::new(m.x0 + ox, m.y0), Vector2i::new(m.x1 - m.x0, m.y1 - m.y0)));
            d.set("cur", m.cur);
            d.set("active", m.active);
            d.set("shift", ox);
            let mut items = VarArray::new();
            for e in &m.items {
                let mut it = VarDictionary::new();
                it.set("pos", Vector2i::new(e.x + ox, e.y));
                it.set("game_pos", Vector2i::new(e.x, e.y));
                it.set("label", &GString::from(e.label.as_str()));
                it.set("enabled", e.enabled);
                it.set("icon", e.icon);
                it.set("uses", e.uses);
                items.push(&it.to_variant());
            }
            d.set("items", &items);
            out.push(&d.to_variant());
        }
        out
    }

    /// Battle forecast: {rect, player: {...}, enemy: {...}} or empty.
    #[func]
    fn forecast(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_ref() else { return d };
        let Some(f) = &g.hd.forecast else { return d };
        let ox = g.ppu.ext_left as i32 + f.shift;
        let (x0, y0, x1, y1) = f.rect;
        d.set("rect", Rect2i::new(Vector2i::new(x0 + ox, y0), Vector2i::new(x1 - x0, y1 - y0)));
        for (k, sd) in [("player", &f.player), ("enemy", &f.enemy)] {
            let mut s = VarDictionary::new();
            s.set("name", &GString::from(sd.name.as_str()));
            s.set("weapon", &GString::from(sd.weapon.as_str()));
            s.set("hp", sd.hp);
            s.set("mt", sd.mt);
            s.set("hit", sd.hit);
            s.set("crit", sd.crit);
            s.set("double", sd.double);
            s.set("tri", sd.tri);
            if let Some((ix, iy)) = sd.icon {
                s.set("icon", Vector2i::new(ix, iy));
            }
            d.set(k, &s);
        }
        d
    }

    /// Free-map windows: {goal: {rect, text}, terrain: {rect, name, def, avo},
    /// unit: {rect, name, hp, max_hp, side, face: Rect2i}} (missing keys = not shown).
    #[func]
    fn mapinfo(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_ref() else { return d };
        let mi = &g.hd.mapinfo;
        let el = g.ppu.ext_left as i32;
        let rect = |(x0, y0, x1, y1): (i32, i32, i32, i32), sh: i32| Rect2i::new(Vector2i::new(x0 + el + sh, y0), Vector2i::new(x1 - x0, y1 - y0));
        if let Some((r, sh, t, l2)) = &mi.goal {
            let mut e = VarDictionary::new();
            e.set("rect", rect(*r, *sh));
            e.set("text", &GString::from(t.as_str()));
            e.set("line2", &GString::from(l2.as_str()));
            d.set("goal", &e);
        }
        if let Some((r, sh, n, def, avo)) = &mi.terrain {
            let mut e = VarDictionary::new();
            e.set("rect", rect(*r, *sh));
            e.set("name", &GString::from(n.as_str()));
            e.set("def", *def);
            e.set("avo", *avo);
            d.set("terrain", &e);
        }
        if let Some((r, sh, u)) = &mi.unit {
            let mut e = VarDictionary::new();
            e.set("rect", rect(*r, *sh));
            e.set("name", &GString::from(u.name.as_str()));
            e.set("hp", u.hp);
            e.set("max_hp", u.max_hp);
            e.set("side", u.side);
            if let Some(f) = u.face {
                // the BG0 picture is not shifted: plain game pixels
                e.set("face", Rect2i::new(Vector2i::new(f.0, f.1), Vector2i::new(f.2 - f.0, f.3 - f.1)));
            }
            d.set("unit", &e);
        }
        if !mi.fights.is_empty() {
            let mut arr = VarArray::new();
            for (r, sh, n, hp, mx, side) in &mi.fights {
                let mut e = VarDictionary::new();
                e.set("rect", rect(*r, *sh));
                e.set("name", &GString::from(n.as_str()));
                e.set("hp", *hp);
                e.set("max_hp", *mx);
                e.set("side", *side);
                arr.push(&e.to_variant());
            }
            d.set("fights", &arr);
        }
        d
    }

    /// Battle scene HUD: {hud: px offset (0 = fully in), sides: [left, right] each
    /// {name, weapon, hp, max_hp, hit, dmg, crit (-1 = none), side 0/1/2, tri -1/0/1,
    /// icon: [tile, pal] or empty}, exp: {old, new, level_up} when the EXP gauge shows, ext}.
    #[func]
    fn battle(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        if g.proc_find(0x08B9_A99C).is_none() {
            return d;
        }
        const B: u32 = 0x0203_E094;
        // HUD position from the HP gauge objects (y 145 when fully shown)
        let mut hud = None;
        let mut icons = [None, None];
        for i in 0..128usize {
            let a0 = g.oam[i * 8] as u16 | (g.oam[i * 8 + 1] as u16) << 8;
            let a1 = g.oam[i * 8 + 2] as u16 | (g.oam[i * 8 + 3] as u16) << 8;
            let a2 = g.oam[i * 8 + 4] as u16 | (g.oam[i * 8 + 5] as u16) << 8;
            if (a0 >> 8) & 3 == 2 {
                continue;
            }
            let (y, x, tile, pal) = ((a0 & 255) as i32, (a1 & 511) as i32, (a2 & 0x3FF) as i32, (a2 >> 12) as i32);
            if pal == 11 && tile == 0x1A0 && hud.is_none() {
                hud = Some(if y > 160 { y - 256 } else { y } - 145);
            }
            if (pal == 13 || pal == 14) && a0 >> 14 == 0 && a1 >> 14 == 1 && y < 160 {
                icons[(x >= 120) as usize] = Some((tile, pal));
            }
        }
        // one-sided scenes (promotion, item use) have no gauge objects: the HUD stands
        d.set("single", hud.is_none());
        let hud = hud.unwrap_or(0);
        d.set("hud", hud);
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let mut sides = VarArray::new();
        for k in 0..2u32 {
            let u = g.read32(B + 4 * k);
            if u >> 24 != 2 {
                return VarDictionary::new();
            }
            let ch = g.read32(u);
            let mut e = VarDictionary::new();
            let nid = g.read16(ch) as u32;
            e.set("name", &GString::from(txt(nid).as_str()));
            let w = g.read16(u + 0x48) as u32 & 0xFF;
            let wn = if w != 0 { let id = g.read16(0x08BE_222C + w * 0x24) as u32; txt(id) } else { String::new() };
            e.set("weapon", &GString::from(wn.as_str()));
            let rs = |g: &mut gba::Gba, o: u32| g.read16(B + o + 2 * k) as i16 as i32;
            e.set("hp", rs(g, 0x24).max(0));
            e.set("max_hp", g.read8(u + 0x12) as i32);
            for (key, o) in [("hit", 0x30), ("dmg", 0x34), ("crit", 0x38)] {
                let v = rs(g, o);
                e.set(key, if v < 0 || w == 0 { -1 } else { v });
            }
            e.set("side", match g.read8(u + 0x0B) >> 6 { 0 => 0, 1 => 1, _ => 2 });
            let tri = g.read8(u + 0x53) as i8;
            e.set("tri", tri.signum() as i32);
            if let Some((t, p)) = icons[k as usize] {
                e.set("icon", &PackedInt32Array::from(&[t, p][..]));
            }
            sides.push(&e.to_variant());
        }
        d.set("sides", &sides);
        // EXP gauge (BG1 window, tiles 0x100.. palette 1, lower rows)
        let mut exp_rows = 0;
        for cy in 14..20 {
            for cx in 0..30 {
                let e = g.hd_bg_cell(1, cx, cy);
                if e >> 12 == 1 && (0x100..=0x115).contains(&(e & 0x3FF)) {
                    exp_rows += 1;
                }
            }
        }
        if exp_rows >= 8 {
            // the player's unit of the two
            for k in 0..2u32 {
                let u = g.read32(B + 4 * k);
                if g.read8(u + 0x0B) >> 6 != 0 {
                    continue;
                }
                let idx = g.read8(u + 0x0B) as u32;
                let real = g.read32(0x08B9_2EB0 + 4 * idx);
                let old = g.read8(real + 9) as i32;
                let new = g.read8(u + 9) as i32;
                let gain = g.read8(u + 0x6E) as i32;
                let mut e = VarDictionary::new();
                e.set("old", if old == 255 { 0 } else { old });
                e.set("new", if new == 255 { 100 } else { new });
                e.set("gain", gain);
                e.set("level_up", g.read8(u + 8) != g.read8(real + 8));
                d.set("exp", &e);
                break;
            }
        }
        d.set("ext", g.ppu.ext_left as i32);
        d
    }

    /// Phase or chapter title banner: {kind: 0 phase / 1 title, text, frames} or empty.
    #[func]
    fn banner(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some((k, t, n)) = self.g.as_ref().and_then(|g| g.hd.banner.clone()) {
            d.set("kind", k);
            d.set("text", &GString::from(t.as_str()));
            d.set("frames", n);
        }
        d
    }

    /// All (non-affine) objects inside a screen rect drawn on transparency, optionally only one palette (-1 = all).
    #[func]
    fn obj_region(&self, rx: i32, ry: i32, rw: i32, rh: i32, only_pal: i32) -> Option<Gd<Image>> {
        let g = self.g.as_ref()?;
        let (rw, rh) = (rw.clamp(1, 240) as usize, rh.clamp(1, 160) as usize);
        let mut px = vec![0u32; rw * rh];
        let one_d = g.io16(0) & 0x40 != 0;
        const SIZES: [[(i32, i32); 4]; 3] = [[(8, 8), (16, 16), (32, 32), (64, 64)], [(16, 8), (32, 8), (32, 16), (64, 32)], [(8, 16), (8, 32), (16, 32), (32, 64)]];
        // back to front so lower OAM indices end on top
        for i in (0..128usize).rev() {
            let a0 = g.oam[i * 8] as u16 | (g.oam[i * 8 + 1] as u16) << 8;
            let a1 = g.oam[i * 8 + 2] as u16 | (g.oam[i * 8 + 3] as u16) << 8;
            let a2 = g.oam[i * 8 + 4] as u16 | (g.oam[i * 8 + 5] as u16) << 8;
            // -3: also rotated/scaled objects, drawn unscaled (screen titles that
            // the game flips in)
            let affine = a0 & 0x100 != 0;
            if (a0 & 0x300 != 0 && !(affine && only_pal == -3)) || a0 >> 14 == 3 || a0 & 0x2000 != 0 {
                continue; // hidden, affine or 8bpp
            }
            let pal = (a2 >> 12) as i32;
            if only_pal >= 0 && pal != only_pal {
                continue;
            }
            // -2: only objects of the UI (priority 0/1), not the units on the map
            if only_pal == -2 && (a2 >> 10) & 3 > 1 {
                continue;
            }
            let (w, h) = SIZES[(a0 >> 14) as usize][(a1 >> 14) as usize];
            let mut oy = (a0 & 255) as i32;
            if oy >= 160 {
                oy -= 256;
            }
            let mut ox = (a1 & 511) as i32;
            if ox >= 256 {
                ox -= 512;
            }
            let (hf, vf) = if affine { (false, false) } else { (a1 & 0x1000 != 0, a1 & 0x2000 != 0) };
            if affine && a0 & 0x200 != 0 {
                ox += w / 2;
                oy += h / 2;
            }
            let tile = (a2 & 0x3FF) as usize;
            for yy in 0..h {
                for xx in 0..w {
                    let (sx, sy) = (ox + xx - rx, oy + yy - ry);
                    if sx < 0 || sy < 0 || sx >= rw as i32 || sy >= rh as i32 {
                        continue;
                    }
                    let tx = if hf { w - 1 - xx } else { xx };
                    let ty = if vf { h - 1 - yy } else { yy };
                    let stride = if one_d { (w / 8) as usize } else { 32 };
                    let t = tile + (ty / 8) as usize * stride + (tx / 8) as usize;
                    let a = 0x10000 + (t & 0x3FF) * 32 + (ty % 8) as usize * 4 + (tx % 8) as usize / 2;
                    let n = (g.vram[a.min(0x17FFF)] >> ((tx & 1) * 4)) & 15;
                    if n != 0 {
                        let ci = 0x100 + pal as usize * 16 + n as usize;
                        let c = (g.pal[ci * 2] as u32 | (g.pal[ci * 2 + 1] as u32) << 8) & 0x7FFF;
                        px[sy as usize * rw + sx as usize] = g.ppu.rgb_lut[c as usize] | 0xFF00_0000;
                    }
                }
            }
        }
        image(rw, rh, &px)
    }

    /// Keep pictures of all four BG layers (for cutting portraits out of screens).
    #[func]
    fn set_capture_layers(&mut self, on: bool) {
        if let Some(g) = self.g.as_mut() {
            g.hd.cap_layers = on;
        }
    }

    /// One BG layer of the last frame (240x160, transparent where empty).
    #[func]
    fn layer_image(&self, bg: i32) -> Option<Gd<Image>> {
        let g = self.g.as_ref()?;
        if g.hd.layers.len() != 4 * 240 * 160 {
            return None;
        }
        let b = bg.clamp(0, 3) as usize;
        image(240, 160, &g.hd.layers[b * 38400..(b + 1) * 38400])
    }

    /// Level-up screen: {name, class, level, stats: [[label, value, up]x9], fade}.
    #[func]
    fn levelup(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        if g.proc_find(0x08BD_B5FC).is_none() {
            return d;
        }
        // the battle unit that gained the level (player side, level differs from the real unit)
        let mut unit = None;
        for u in [0x0203_A3F0u32, 0x0203_A470] {
            let idx = g.read8(u + 0x0B) as u32;
            if idx >> 6 != 0 || g.read32(u) >> 24 != 8 {
                continue;
            }
            let real = g.read32(0x08B9_2EB0 + 4 * idx);
            if g.read8(u + 8) != g.read8(real + 8) {
                unit = Some((u, real));
            }
        }
        let Some((u, real)) = unit else { return d };
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let ch = g.read32(u);
        let cl = g.read32(u + 4);
        let n = g.read16(ch) as u32;
        d.set("name", &GString::from(txt(n).as_str()));
        let n = g.read16(cl) as u32;
        d.set("class", &GString::from(txt(n).as_str()));
        d.set("level", g.read8(u + 8) as i32);
        let mut magic = false;
        let mut phys = false;
        for k in 0..8u32 {
            if g.read8(cl + 0x2C + k) > 0 {
                if k >= 4 { magic = true } else { phys = true }
            }
        }
        let pw = if magic && !phys { "Mag" } else { "Str" };
        // stat: (label, offset in the unit, offset of the gain in the battle unit)
        let con = (g.read8(cl + 0x11) as i32) + (g.read8(ch + 0x13) as i32) + (g.read8(real + 0x1A) as i32);
        let spec: [(&str, u32, u32); 8] = [("HP", 0x12, 0x73), (pw, 0x14, 0x74), ("Skill", 0x15, 0x75), ("Spd", 0x16, 0x76),
            ("Luck", 0x19, 0x79), ("Def", 0x17, 0x77), ("Res", 0x18, 0x78), ("Con", 0, 0x7A)];
        let mut stats = VarArray::new();
        for (label, off, up) in spec {
            let gain = g.read8(u + up) as i8 as i32;
            let base = if off == 0 { con } else { g.read8(real + off) as i32 };
            let mut e = VarArray::new();
            e.push(&label.to_variant());
            e.push(&(base + gain).to_variant());
            e.push(&gain.to_variant());
            stats.push(&e.to_variant());
        }
        d.set("stats", &stats);
        d
    }

    /// Unit list screen: {page 0-5, row, sel, mode (3 = header), hcol, sort, desc, slide,
    /// units: [{name, class, lv, exp, hp, max, stats[6], pw, affinity, equip, atk, hit, avo,
    ///   move, con, aid, trv, cond, ranks[8] letters, side}], icons: [[tile, pal, y]]}.
    #[func]
    fn unitlist(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        let Some(p) = g.proc_find(0x08CC_3194) else { return d };
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let gs = |s: &str| GString::from(s);
        d.set("page", g.read8(p + 0x37) as i32);
        d.set("row", g.read8(p + 0x2C) as i32);
        d.set("sel", g.read8(p + 0x30) as i32);
        d.set("mode", g.read8(p + 0x29) as i32);
        d.set("hcol", g.read8(p + 0x2D) as i32);
        d.set("sort", g.read8(p + 0x32) as i32);
        d.set("desc", g.read8(p + 0x34) as i32);
        d.set("slide", g.read8(p + 0x38) as i32);
        let mut units = VarArray::new();
        let cond_names = ["", "Poison", "Sleep", "Silence", "Berserk", "Attack", "Defense", "Critical", "Avoid", "Sick", "Recover"];
        for k in 0..64u32 {
            let ep = g.read32(0x0200_CBF0 + 4 * k);
            if ep >> 24 != 2 {
                break;
            }
            let u = g.read32(ep);
            let ch = g.read32(u);
            let cl = g.read32(u + 4);
            if ch >> 24 != 8 || cl >> 24 != 8 {
                continue;
            }
            let r8 = |g: &mut gba::Gba, a: u32| g.read8(a) as i32;
            let mut e = VarDictionary::new();
            let id = g.read16(ch) as u32;
            e.set("name", &gs(&txt(id)));
            let id = g.read16(cl) as u32;
            e.set("class", &gs(&txt(id)));
            e.set("lv", r8(g, u + 8));
            e.set("exp", r8(g, u + 9));
            e.set("hp", r8(g, u + 0x13));
            e.set("max", r8(g, u + 0x12));
            let stats = PackedInt32Array::from(&[r8(g, u + 0x14), r8(g, u + 0x15), r8(g, u + 0x16), r8(g, u + 0x19), r8(g, u + 0x17), r8(g, u + 0x18)][..]);
            e.set("stats", &stats);
            e.set("affinity", r8(g, ch + 9));
            // equipped weapon: first usable weapon in the inventory
            let mut equip = String::new();
            let mut atk = -1;
            let mut hit = -1;
            let con = r8(g, cl + 0x11) + r8(g, ch + 0x13) + r8(g, u + 0x1A);
            let mut wt = 0;
            for i in 0..5u32 {
                let it = g.read16(u + 0x1E + 2 * i) as u32;
                if it & 0xFF == 0 {
                    break;
                }
                let ia = 0x08BE_222C + (it & 0xFF) * 0x24;
                let ty = g.read8(ia + 7) as u32;
                let attr = g.read32(ia + 8);
                if attr & 1 == 0 || ty > 7 || ty == 4 {
                    continue; // not a weapon (staves don't count)
                }
                let need = g.read8(ia + 0x1C);
                if g.read8(u + 0x28 + ty) < need.max(1) {
                    continue;
                }
                let id = g.read16(ia) as u32;
                equip = txt(id);
                let pw = r8(g, u + 0x14);
                atk = pw + r8(g, ia + 0x15);
                hit = r8(g, ia + 0x16) + r8(g, u + 0x15) * 2 + r8(g, u + 0x19) / 2;
                wt = r8(g, ia + 0x17);
                break;
            }
            let spd = (r8(g, u + 0x16) - (wt - con).max(0)).max(0);
            e.set("equip", &gs(&equip));
            e.set("atk", atk);
            e.set("hit", hit);
            e.set("avo", spd * 2 + r8(g, u + 0x19));
            e.set("move", r8(g, cl + 0x12) + r8(g, u + 0x1D));
            e.set("con", con);
            let attrs = g.read32(cl + 0x28) | g.read32(ch + 0x28);
            let aid = if attrs & 1 != 0 { if attrs & 0x4000 != 0 { 20 - con } else { 25 - con } } else { con - 1 };
            e.set("aid", aid.max(0));
            let rescue = g.read8(u + 0x1B) as u32;
            let mut trv = String::new();
            if rescue != 0 {
                let ru = g.read32(0x08B9_2EB0 + 4 * rescue);
                let rc = g.read32(ru);
                if rc >> 24 == 8 {
                    let id = g.read16(rc) as u32;
                    trv = txt(id);
                }
            }
            e.set("trv", &gs(&trv));
            let st = (g.read8(u + 0x30) & 0x0F) as usize;
            e.set("cond", &gs(cond_names.get(st).unwrap_or(&"")));
            let mut ranks = PackedStringArray::new();
            for k in 0..8u32 {
                let v = g.read8(u + 0x28 + k);
                let l = match v { 0 => "-", 1..=30 => "E", 31..=70 => "D", 71..=120 => "C", 121..=180 => "B", 181..=250 => "A", _ => "S" };
                ranks.push(l);
            }
            e.set("ranks", &ranks);
            // support partners on the map
            let mut allies = PackedStringArray::new();
            let sp = g.read32(ch + 0x2C);
            if sp >> 24 == 8 {
                for k in 0..7u32 {
                    let partner = g.read8(sp + k) as u32;
                    if partner == 0 {
                        continue;
                    }
                    let pc = 0x08BD_CE18 + partner * 0x34;
                    let id = g.read16(pc) as u32;
                    let pts = g.read8(u + 0x32 + k);
                    let lv = if pts > 240 { " A" } else if pts > 160 { " B" } else if pts > 80 { " C" } else { "" };
                    allies.push(&format!("{}{}", txt(id), lv));
                }
            }
            e.set("allies", &allies);
            units.push(&e.to_variant());
        }
        d.set("units", &units);
        // map sprites of the visible rows
        let mut icons = VarArray::new();
        for i in 0..128usize {
            let a0 = g.oam[i * 8] as u16 | (g.oam[i * 8 + 1] as u16) << 8;
            let a1 = g.oam[i * 8 + 2] as u16 | (g.oam[i * 8 + 3] as u16) << 8;
            let a2 = g.oam[i * 8 + 4] as u16 | (g.oam[i * 8 + 5] as u16) << 8;
            if (a0 >> 8) & 3 == 2 || a0 >> 14 != 0 || a1 >> 14 != 1 {
                continue;
            }
            let (y, x) = ((a0 & 255) as i32, (a1 & 511) as i32);
            if x == 8 && (48..160).contains(&y) {
                icons.push(&PackedInt32Array::from(&[(a2 & 0x3FF) as i32, (a2 >> 12) as i32, y][..]).to_variant());
            }
        }
        d.set("icons", &icons);
        d
    }

    /// A 16x16 item icon straight from the ROM (icon number from the item data +0x1D).
    #[func]
    fn item_icon(&self, icon: i32) -> Option<Gd<Image>> {
        let g = self.g.as_ref()?;
        const TILES: usize = 0x00C5_EA4;
        const PAL: usize = 0x00CB_EA4;
        let mut px = vec![0u32; 256];
        let base = TILES + (icon.clamp(0, 255) as usize) * 128;
        for t in 0..4usize {
            for y in 0..8usize {
                for x in 0..8usize {
                    let b = g.rom.get(base + t * 32 + y * 4 + x / 2).copied().unwrap_or(0);
                    let n = ((b >> ((x & 1) * 4)) & 15) as usize;
                    if n != 0 {
                        let c = (g.rom[PAL + n * 2] as u32 | (g.rom[PAL + n * 2 + 1] as u32) << 8) & 0x7FFF;
                        px[((t / 2) * 8 + y) * 16 + (t % 2) * 8 + x] = g.ppu.rgb_lut[c as usize] | 0xFF00_0000;
                    }
                }
            }
        }
        image(16, 16, &px)
    }

    /// The game's own pixel fonts straight from the ROM as a 16x16 grid of 16x16
    /// cells (character code = cell number): which 0 = menu font, 1 = dialogue font.
    /// Returns {image, adv (advance per character)}. The umlauts and the sharp s,
    /// which the US game lacks, are put together from its own letters.
    #[func]
    fn font_atlas(&self, which: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_ref() else { return d };
        let table: usize = if which == 0 { 0x00B8_96B0 } else { 0x00B8_B5B0 };
        let (body, edge) = if which == 0 { (2u8, 3u8) } else { (3u8, 2u8) };
        let rd32 = |o: usize| -> u32 {
            (0..4).fold(0u32, |a, i| a | (g.rom.get(o + i).copied().unwrap_or(0) as u32) << (8 * i))
        };
        let mut cells = vec![[[0u8; 16]; 16]; 256];
        let mut adv = vec![0u8; 256];
        for c in 0x20..0x100usize {
            let gp = rd32(table + 4 * c);
            if gp >> 24 != 8 {
                continue;
            }
            let o = (gp - 0x0800_0000) as usize;
            adv[c] = g.rom.get(o + 5).copied().unwrap_or(0);
            for y in 0..16 {
                let row = rd32(o + 8 + 4 * y);
                for x in 0..16 {
                    cells[c][y][x] = ((row >> (2 * x)) & 3) as u8;
                }
            }
        }
        // umlauts: the base letter with two dots above
        for (dst, src) in [(0xE4usize, b'a'), (0xF6, b'o'), (0xFC, b'u'), (0xC4, b'A'), (0xD6, b'O'), (0xDC, b'U')] {
            let mut cell = cells[src as usize];
            let top = (0..16).find(|&y| cell[y].iter().any(|&v| v == body)).unwrap_or(4);
            let w = adv[src as usize] as usize;
            let (x0, x1) = if which == 0 { (1usize, (w.max(5) - 1).min(4)) } else { (0usize, 3usize) };
            if which == 0 {
                let dy = if top >= 3 { top - 2 } else { 0 };
                for &x in &[x0, x1] {
                    for (ox, oy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                        let (nx, ny) = (x as i32 + ox, dy as i32 + oy);
                        if (0..16).contains(&nx) && (0..16).contains(&ny) && cell[ny as usize][nx as usize] == 0 {
                            cell[ny as usize][nx as usize] = edge;
                        }
                    }
                    cell[dy][x] = body;
                }
            } else {
                let dy = if top >= 2 { top - 2 } else { 0 };
                if top < 2 {
                    cell[0] = [0; 16];
                }
                for &x in &[x0, x1] {
                    cell[dy][x] = body;
                    if cell[dy][x + 1] == 0 {
                        cell[dy][x + 1] = edge;
                    }
                }
            }
            cells[dst] = cell;
            adv[dst] = adv[src as usize];
        }
        let sz: &[&str] = if which == 0 {
            &["", "", "", ".@@@..", "@+++@.", "@+@@+@", "@+@@+@", "@+@+@.", "@+@@+@", "@+@@+@", "@+@@+@", "@+@@+@", "@+@+@.", ".@.@.."]
        } else {
            &["", ".@@+..", "@++@+.", "@+.@+.", "@+.@+.", "@+@+..", "@+.@+.", "@+..@+", "@+..@+", "@+..@+", "@+..@+", "@+..@+", "@+.@+.", "@+@+.."]
        };
        for (y, row) in sz.iter().enumerate() {
            for (x, ch) in row.bytes().enumerate() {
                // drawn in the menu font's numbering (+ = body, @ = edge); the
                // dialogue font swaps the two
                cells[0xDF][y][x] = match (ch, which) {
                    (b'+', 0) | (b'@', 1) => body,
                    (b'@', 0) | (b'+', 1) => edge,
                    _ => 0,
                };
            }
        }
        adv[0xDF] = 6;
        let mut px = vec![0u32; 256 * 256];
        for c in 0..256usize {
            for y in 0..16 {
                for x in 0..16 {
                    let v = cells[c][y][x];
                    let col = if v == 0 {
                        0
                    } else if v == body {
                        0xFFFF_FFFF
                    } else if which == 0 {
                        // dark outline of the menu font (value 1 is a soft inner shade)
                        if v == edge { 0xE618_1820 } else { 0xFFB8_B8C8 }
                    } else if v == edge {
                        0x73FF_FFFF
                    } else {
                        0
                    };
                    px[((c / 16) * 16 + y) * 256 + (c % 16) * 16 + x] = col;
                }
            }
        }
        if let Some(img) = image(256, 256, &px) {
            d.set("image", &img);
        }
        d.set("adv", &PackedByteArray::from(adv.as_slice()));
        d
    }

    /// World map narration lines (empty array = no box).
    #[func]
    fn worldmap(&self) -> PackedStringArray {
        let mut a = PackedStringArray::new();
        if let Some(l) = self.g.as_ref().and_then(|g| g.hd.worldmap.clone()) {
            for x in l {
                a.push(x.as_str());
            }
            if a.is_empty() {
                a.push("");
            }
        }
        a
    }

    /// Shop: {kind ("Armory"/"Vendor"/"Secret Shop"), mode 0 buy / 1 sell, items: [{name, uses, price, icon}],
    /// text: [lines], gold, face: Rect2i in BG0 picture, row (hand), choice (-1 none, 0 buy, 1 sell)}.
    #[func]
    fn shop(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        let Some(face) = g.hd.shop else { return d };
        let arena = g.proc_find(0x08CE_6FC0).is_none();
        let Some(p) = g.proc_find(0x08CE_6FC0).or_else(|| g.proc_find(0x08CE_7280)) else { return d };
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let u = g.read32(p + 0x2C);
        let mode = if arena { 0 } else { g.read8(p + 0x60) as i32 };
        d.set("mode", mode);
        d.set("arena", arena);
        // kind from the terrain under the unit
        let mut kind = "Shop";
        if u >> 24 == 2 {
            let (x, y) = (g.read8(u + 0x10) as u32, g.read8(u + 0x11) as u32);
            let tp = g.read32(0x0202_E3E0);
            let row = g.read32(tp + 4 * y);
            kind = match g.read8(row + x) { 6 => "Armory", 7 => "Vendor", 0x38 => "Secret Shop", _ => "Shop" };
        }
        d.set("kind", &GString::from(kind));
        let mut items = VarArray::new();
        if arena {
            kind = "Arena";
            d.set("kind", &GString::from(kind));
        }
        let list: Vec<u32> = if arena {
            Vec::new()
        } else if mode == 1 && u >> 24 == 2 {
            (0..5).map(|i| g.read16(u + 0x1E + 2 * i) as u32).collect()
        } else {
            (0..20).map(|i| g.read16(p + 0x30 + 2 * i) as u32).collect()
        };
        for it in list {
            if it & 0xFF == 0 {
                break;
            }
            let ia = 0x08BE_222C + (it & 0xFF) * 0x24;
            let id = g.read16(ia) as u32;
            let per = g.read16(ia + 0x1A) as i32;
            let uses = (it >> 8) as i32;
            let maxu = g.read8(ia + 0x14) as i32;
            let unbreak = g.read32(ia + 8) & 8 != 0;
            let price = if mode == 1 {
                if unbreak || maxu == 0 { per / 2 } else { per * uses / 2 }
            } else if unbreak || maxu == 0 {
                per
            } else {
                per * uses
            };
            let mut x = VarDictionary::new();
            x.set("name", &GString::from(txt(id).as_str()));
            x.set("uses", if unbreak { -1 } else { uses });
            x.set("price", price);
            x.set("icon", g.read8(ia + 0x1D) as i32);
            items.push(&x.to_variant());
        }
        d.set("items", &items);
        let lines = g.hd_typed_text();
        let lines: Vec<GString> = lines.iter().map(|l| GString::from(l.as_str())).collect();
        d.set("text", &PackedStringArray::from(lines.as_slice()));
        d.set("gold", g.read32(0x0202_BC00) as i32);
        d.set("face", Rect2i::new(Vector2i::new(face.0, face.1), Vector2i::new(face.2 - face.0, face.3 - face.1)));
        // the hand: in the list (row) or on Buy/Sell in the text box
        let mut row = -1;
        let mut choice = -1;
        for i in 0..128usize {
            let a0 = g.oam[i * 8] as u16 | (g.oam[i * 8 + 1] as u16) << 8;
            let a1 = g.oam[i * 8 + 2] as u16 | (g.oam[i * 8 + 3] as u16) << 8;
            let a2 = g.oam[i * 8 + 4] as u16 | (g.oam[i * 8 + 5] as u16) << 8;
            if (a0 >> 8) & 3 == 2 || a0 >> 14 != 0 || a1 >> 14 != 1 || a2 & 0x3FF != 0 {
                continue;
            }
            let (y, x) = ((a0 & 255) as i32, (a1 & 511) as i32);
            if y >= 160 {
                continue;
            }
            if y >= 64 && !arena {
                row = (y + 8 - 72) / 16;
            } else if y < 64 {
                choice = if x < (if arena { 140 } else { 100 }) { 0 } else { 1 };
            }
        }
        d.set("row", row);
        d.set("choice", choice);
        // what the shopkeeper says, as the game printed it into its text box
        let printed = g.region_lines(6, 0, 30, 9);
        if !printed.is_empty() {
            let mut lines: Vec<(i32, String)> = Vec::new();
            let gold_s = g.read32(0x0202_BC00).to_string();
            for (_, y, t) in printed {
                if t.trim() == gold_s {
                    continue;
                }
                if choice >= 0 && matches!(t.trim(), "Yes" | "No" | "Buy" | "Sell") {
                    continue;
                }
                match lines.iter_mut().find(|l| (l.0 - y).abs() < 8) {
                    Some(l) => {
                        if !l.1.ends_with(' ') && !t.starts_with(' ') {
                            l.1.push(' ');
                        }
                        l.1.push_str(&t);
                    }
                    None => lines.push((y, t)),
                }
            }
            let lines: Vec<GString> = lines
                .iter()
                .map(|l| {
                    let mut t = l.1.trim().to_string();
                    // the choice is drawn as buttons
                    while choice >= 0 {
                        let tr = t.trim_end();
                        if let Some(r) = tr.strip_suffix("Yes").or_else(|| tr.strip_suffix("No")).or_else(|| tr.strip_suffix("Buy")).or_else(|| tr.strip_suffix("Sell")) {
                            t = r.trim_end().to_string();
                        } else {
                            break;
                        }
                    }
                    GString::from(t.as_str())
                })
                .collect();
            // the log only knows what was printed while we watched: when it is just
            // the tail of the text being typed, keep the whole text
            let typed = g.hd_typed_text().join(" ");
            let logged = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(" ");
            let _ = &logged;
            if arena || typed.trim().is_empty() {
                d.set("text", &PackedStringArray::from(lines.as_slice()));
            }
        }
        d
    }

    /// Trade screen: {left, right: {rect, name, items: [{name, uses, icon}]}, cursor: [side, row],
    /// picked: [side, row] or empty}.
    #[func]
    fn trade(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        let Some((l, r, sh, _)) = g.hd.trade.clone() else { return d };
        let mut hands = Vec::new();
        for i in 0..128usize {
            let a0 = g.oam[i * 8] as u16 | (g.oam[i * 8 + 1] as u16) << 8;
            let a1 = g.oam[i * 8 + 2] as u16 | (g.oam[i * 8 + 3] as u16) << 8;
            let a2 = g.oam[i * 8 + 4] as u16 | (g.oam[i * 8 + 5] as u16) << 8;
            if (a0 >> 8) & 3 == 2 || a0 >> 14 != 0 || a1 >> 14 != 1 || a2 & 0x3FF != 0 {
                continue;
            }
            let y = (a0 & 255) as i32;
            if y < 160 {
                hands.push(((a1 & 511) as i32, y));
            }
        }
        let Some(p) = g.proc_find(0x08B9_42F8) else { return d };
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let el = g.ppu.ext_left as i32;
        let rect = |(x0, y0, x1, y1): (i32, i32, i32, i32)| Rect2i::new(Vector2i::new(x0 + el + sh, y0), Vector2i::new(x1 - x0, y1 - y0));
        for (k, key, rr) in [(0u32, "left", l), (1, "right", r)] {
            let u = g.read32(p + 0x2C + 4 * k);
            if u >> 24 != 2 {
                return VarDictionary::new();
            }
            let ch = g.read32(u);
            let mut e = VarDictionary::new();
            e.set("rect", rect(rr));
            let id = g.read16(ch) as u32;
            e.set("name", &GString::from(txt(id).as_str()));
            let mut items = VarArray::new();
            for i in 0..5u32 {
                let it = g.read16(u + 0x1E + 2 * i) as u32;
                if it & 0xFF == 0 {
                    break;
                }
                let ia = 0x08BE_222C + (it & 0xFF) * 0x24;
                let mut x = VarDictionary::new();
                let id = g.read16(ia) as u32;
                x.set("name", &GString::from(txt(id).as_str()));
                let unbreak = g.read32(ia + 8) & 8 != 0;
                x.set("uses", if unbreak { -1 } else { (it >> 8) as i32 });
                x.set("icon", g.read8(ia + 0x1D) as i32);
                items.push(&x.to_variant());
            }
            e.set("items", &items);
            d.set(key, &e);
        }
        // hands: row = (y - window top - 9) / 16, side by x
        let pos = |(x, y): (i32, i32)| {
            let side = if x + 8 >= r.0 { 1 } else { 0 };
            let top = if side == 1 { r.1 } else { l.1 };
            PackedInt32Array::from(&[side, ((y - top - 1).max(0)) / 16][..])
        };
        if let Some(h) = hands.first() {
            d.set("cursor", &pos(*h));
        }
        if let Some(h) = hands.get(1) {
            d.set("picked", &pos(*h));
        }
        d
    }

    /// Preparations sub-screens "Pick Units" and "Trade" (choosing whom):
    /// {mode: "pick"/"trade", cursor, units: [{name, class, lv, hp, max, on, locked}],
    /// count, max, picked (trade: first unit chosen or -1), items of the unit under the cursor}.
    #[func]
    fn prepunits(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        // a story scene (world map narration) can run while the preparations wait
        if g.proc_find(0x08CE_7630).is_some() {
            return d;
        }
        let (mode, p) = if let Some(p) = g.proc_find(0x08CC_4854) {
            ("pick", p)
        } else if let Some(p) = g.proc_find(0x08CC_4448) {
            ("trade", p)
        } else {
            return d;
        };
        // the trade screen itself (two units chosen) has its own native view
        if g.proc_find(0x08B9_42F8).is_some() {
            return d;
        }
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let n = g.read32(0x0201_17DC).min(64);
        let cur = if mode == "pick" { g.read16(p + 0x2E) as u32 } else { g.read8(p + 0x29) as u32 };
        let mut units = VarArray::new();
        let mut cur_unit = 0u32;
        for i in 0..n {
            let u = g.read32(0x0201_16DC + 4 * i);
            if u >> 24 != 2 {
                continue;
            }
            let ch = g.read32(u);
            let cl = g.read32(u + 4);
            let st = g.read32(u + 0x0C);
            // the mode's lord always goes (Lyn / Eliwood / Hector)
            let cid = g.read8(ch + 4);
            let lord = matches!((g.read8(0x0202_BBF8 + 0x1B), cid), (1, 3) | (2, 1) | (3, 2));
            let mut e = VarDictionary::new();
            e.set("name", txt(g.read16(ch) as u32).as_str());
            e.set("class", txt(g.read16(cl) as u32).as_str());
            e.set("lv", g.read8(u + 8) as i32);
            e.set("hp", g.read8(u + 0x13) as i32);
            e.set("max", g.read8(u + 0x12) as i32);
            e.set("on", st & 8 == 0 || lord);
            e.set("locked", lord || st & 0x0200_0000 != 0);
            units.push(&e.to_variant());
            if i == cur {
                cur_unit = u;
            }
        }
        d.set("mode", mode);
        d.set("units", &units);
        d.set("cursor", cur as i32);
        if mode == "pick" {
            d.set("count", g.read8(p + 0x29) as i32);
            d.set("max", g.read8(p + 0x2A) as i32);
        } else {
            let pk = g.read8(p + 0x2A);
            d.set("picked", if pk == 0xFF { -1 } else { pk as i32 });
        }
        let mut items = VarArray::new();
        if cur_unit != 0 {
            for i in 0..5u32 {
                let it = g.read16(cur_unit + 0x1E + 2 * i) as u32;
                if it & 0xFF == 0 {
                    break;
                }
                let ia = 0x08BE_222C + (it & 0xFF) * 0x24;
                let mut x = VarDictionary::new();
                x.set("name", txt(g.read16(ia) as u32).as_str());
                let unbreak = g.read32(ia + 8) & 8 != 0;
                x.set("uses", if unbreak { -1 } else { (it >> 8) as i32 });
                x.set("icon", g.read8(ia + 0x1D) as i32);
                items.push(&x.to_variant());
            }
        }
        d.set("items", &items);
        let ch = g.read8(0x0202_BC06) as u32;
        d.set("goal", txt(g.read16(0x08C9_A200 + ch * 0x98 + 0x8E) as u32).as_str());
        d.set("gold", g.read32(0x0202_BC00) as i32);
        d
    }

    /// Help bubble: {rect (frame px), text} or empty.
    #[func]
    fn rhelp(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_ref() else { return d };
        if let Some((x, y, w, h, t)) = &g.hd.rhelp {
            d.set("rect", Rect2i::new(Vector2i::new(x + g.ppu.ext_left as i32, *y), Vector2i::new(*w, *h)));
            let lines: Vec<GString> = t.replace('\x01', "\n").split('\n').map(|l| GString::from(l.trim_matches(|c: char| c.is_control()).trim())).filter(|l| !l.is_empty()).collect();
            d.set("lines", &PackedStringArray::from(lines.as_slice()));
        }
        d
    }

    /// Other game windows replaced by native frames: [Rect2i] in frame pixels;
    /// their BG0 text is in `bg0_image()` at the rect minus `ext_left` + shift.
    #[func]
    fn generic_windows(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(g) = self.g.as_ref() else { return out };
        for (i, &(x0, y0, x1, y1, sh)) in g.hd.generic.iter().enumerate() {
            let mut d = VarDictionary::new();
            d.set("tab", g.hd.generic_tab.get(i).copied().unwrap_or(0));
            if let Some((texts, rest)) = g.hd.generic_text.get(i) {
                let ox = g.ppu.ext_left as i32 + sh;
                let mut ta = VarArray::new();
                for (x, y, c, t) in texts {
                    let mut e = VarDictionary::new();
                    e.set("pos", Vector2i::new(x + ox, *y));
                    e.set("color", if *c == 255 { 2 } else { *c as i32 });
                    e.set("text", t.as_str());
                    ta.push(&e.to_variant());
                }
                d.set("texts", &ta);
                let mut ra = VarArray::new();
                for (x, y) in rest {
                    ra.push(&Vector2i::new(*x, *y).to_variant());
                }
                d.set("rest", &ra);
            }
            d.set("rect", Rect2i::new(Vector2i::new(x0 + g.ppu.ext_left as i32 + sh, y0), Vector2i::new(x1 - x0, y1 - y0)));
            d.set("src", Rect2i::new(Vector2i::new(x0, y0), Vector2i::new(x1 - x0, y1 - y0)));
            out.push(&d.to_variant());
        }
        out
    }

    /// EXP gauge after a battle on the map: {rect, old, new, level_up} or empty.
    #[func]
    fn mapexp(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        let Some((x0, y0, x1, y1)) = g.hd.mapexp else { return d };
        const B: u32 = 0x0203_E094;
        for k in 0..2u32 {
            let u = g.read32(B + 4 * k);
            if u >> 24 != 2 || g.read8(u + 0x0B) >> 6 != 0 {
                continue;
            }
            let idx = g.read8(u + 0x0B) as u32;
            let real = g.read32(0x08B9_2EB0 + 4 * idx);
            let old = g.read8(real + 9) as i32;
            let new = g.read8(u + 9) as i32;
            d.set("rect", Rect2i::new(Vector2i::new(x0 + g.ppu.ext_left as i32, y0), Vector2i::new(x1 - x0, y1 - y0)));
            d.set("old", if old == 255 { 0 } else { old });
            d.set("new", if new == 255 { 100 } else { new });
            d.set("level_up", g.read8(u + 8) != g.read8(real + 8));
            break;
        }
        d
    }

    /// Map overview: {size, units: [Vector3i(x, y, side 0 blue / 1 green / 2 red)],
    /// cursor} or empty when closed.
    #[func]
    fn minimap(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        if !g.hd.minimap {
            return d;
        }
        let w = g.read16(0x0202_E3D8) as i32;
        let h = g.read16(0x0202_E3DA) as i32;
        d.set("size", Vector2i::new(w, h));
        let mut units = VarArray::new();
        for i in 1..0xC0u32 {
            let u = g.read32(0x08B9_2EB0 + 4 * i);
            if u >> 24 != 2 || g.read32(u) == 0 {
                continue;
            }
            let st = g.read32(u + 0x0C);
            if st & 0x0D != 0 {
                continue; // hidden, dead, not deployed
            }
            let side = if i < 0x40 { 0 } else if i < 0x80 { 1 } else { 2 };
            let (x, y) = (g.read8(u + 0x10) as i8 as i32, g.read8(u + 0x11) as i8 as i32);
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            units.push(&Vector3i::new(x, y, side).to_variant());
        }
        d.set("units", &units);
        d.set("cursor", Vector2i::new(g.read16(0x0202_BBCC) as i32, g.read16(0x0202_BBCE) as i32));
        d
    }

    /// "Check Map" view of the preparations: 0 off, 1 its menu is open, 2 free map.
    #[func]
    fn prepmap(&self) -> i32 {
        self.g.as_ref().map(|g| g.hd.prepmap).unwrap_or(0)
    }

    /// Preparations main menu (before a chapter): {title, goal, items: [{label, off}],
    /// cursor, help}. Empty while a sub-screen (pick units, trade, save...) is open.
    #[func]
    fn prep(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        if g.proc_find(0x08CE_7630).is_some() {
            return d;
        }
        let Some(pp) = g.proc_find(0x08CC_3BDC) else { return d };
        let Some(m) = g.proc_find(0x08CC_416C) else { return d };
        if g.read32(m + 0x14) != pp {
            return d;
        }
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let ch = g.read8(0x0202_BC06) as u32;
        d.set("title", txt(0x58C + ch).as_str());
        let cd = 0x08C9_A200 + ch * 0x98;
        let gid = g.read16(cd + 0x8E) as u32;
        d.set("goal", txt(gid).as_str());
        let n = (g.read8(m + 0x2B) as u32).min(8);
        let cur = g.read8(m + 0x2A) as u32;
        let variant = if g.read8(0x0202_BBF8 + 0x1B) == 1 { 0 } else { 4 };
        let mut items = VarArray::new();
        let mut help = String::new();
        for i in 0..n {
            let e = g.read32(m + 0x38 + 4 * i);
            if e >> 24 != 2 {
                continue;
            }
            let id = g.read8(e + 0x39) as u32;
            let off = g.read8(e + 0x38) != 0;
            let mut it = VarDictionary::new();
            it.set("label", txt(g.read32(e + 0x34)).as_str());
            it.set("off", off);
            items.push(&it.to_variant());
            if i == cur {
                let hid = if off {
                    0x3E7
                } else if ch == 0x2E && id == 7 {
                    0x3EF
                } else if id < 8 {
                    g.read32(0x08CC_3B30 + id * 12 + variant)
                } else {
                    0
                };
                if hid != 0 {
                    help = txt(hid);
                }
            }
        }
        d.set("items", &items);
        d.set("cursor", cur as i32);
        let lines: Vec<GString> = help.split('\n').map(|l| GString::from(l.trim())).collect();
        d.set("help", &PackedStringArray::from(lines.as_slice()));
        d.set("gold", g.read32(0x0202_BC00) as i32);
        d
    }

    /// Game options screen: {row, help, rows: [{label, values: [..], cur}]} or empty.
    #[func]
    fn options(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        if g.proc_find(0x08CE_5BF0).is_none() {
            return d;
        }
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        const S: u32 = 0x0202_0140;
        const ENTRIES: u32 = 0x08CE_58D4; // 0x2C bytes per option
        let mut mode = g.read16(S + 0x32) as u32;
        if g.read8(0x0202_BBF8 + 0x14) & 0x40 != 0 {
            mode += 3;
        }
        let list = g.read32(0x08CE_586C + mode * 8);
        let count = g.read32(0x08CE_5870 + mode * 8).min(16);
        // current value of an option (bits of gPlaySt +0x40..0x42)
        let cfg = |g: &mut gba::Gba, o: u32| g.read8(0x0202_BC38 + o) as u32;
        let mut rows = VarArray::new();
        let row = g.read16(S + 0x2A) as i16 as i32;
        let mut help = String::new();
        for i in 0..count {
            let id = g.read8(list + i) as u32;
            let e = ENTRIES + id * 0x2C;
            let lid = g.read16(e + 4) as u32;
            let mut values = PackedStringArray::new();
            let mut helps = Vec::new();
            for v in 0..4u32 {
                let vid = g.read16(e + 10 + v * 8) as u32;
                if vid == 0 {
                    break;
                }
                values.push(txt(vid).as_str());
                helps.push(g.read16(e + 8 + v * 8) as u32);
            }
            let cur = match id {
                0 => match (cfg(g, 2) >> 1) & 3 { 1 => 2, 2 => 3, 3 => 1, _ => 0 },
                1 => (cfg(g, 0) >> 1) & 1,
                2 => (cfg(g, 0) >> 2) & 3,
                3 => (cfg(g, 0) >> 4) & 1,
                4 => (cfg(g, 0) >> 5) & 3,
                5 => (cfg(g, 0) >> 7) & 1,
                6 => cfg(g, 1) & 1,
                7 => (cfg(g, 1) >> 1) & 1,
                8 => (cfg(g, 1) >> 2) & 3,
                9 => 0,
                10 => (cfg(g, 2) >> 3) & 3,
                11 => (cfg(g, 1) >> 7) & 1,
                12 => (cfg(g, 1) >> 6) & 1,
                13 => cfg(g, 0) & 1,
                14 => cfg(g, 2) & 1,
                _ => (cfg(g, 2) >> 5) & 1,
            } as i32;
            if i as i32 == row {
                if let Some(h) = helps.get(cur as usize) {
                    help = txt(*h);
                }
            }
            let mut r = VarDictionary::new();
            r.set("label", &GString::from(txt(lid).as_str()));
            r.set("values", &values);
            r.set("cur", cur);
            rows.push(&r.to_variant());
        }
        d.set("rows", &rows);
        d.set("row", row);
        d.set("help", &GString::from(help.as_str()));
        d
    }

    /// Unit status screen (R on a unit): everything the three pages show, or empty.
    /// {page, name, class, level, exp, hp, max_hp, pow_label, stats: [[label, value, cap]x6],
    ///  move, con, con_cap, aid, trv, affinity, cond, items: [{name, uses, max, equipped}],
    ///  equip: [atk, hit, crit, avoid], rng, ranks: [{type, letter, frac}], supports: [{name, level}],
    ///  fade, ext}
    #[func]
    fn status(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_mut() else { return d };
        if g.proc_find(0x08CC_1F0C).is_none() {
            return d;
        }
        let u = g.read32(0x0200_3118);
        if u >> 24 != 2 {
            return d;
        }
        let ch = g.read32(u);
        let cl = g.read32(u + 4);
        if ch >> 24 != 8 || cl >> 24 != 8 {
            return d;
        }
        let rom = g.rom.clone();
        let txt = |id: u32| gba::text::decode(&rom, id).trim().to_string();
        let gs = |s: String| GString::from(s.as_str());
        let r8 = |g: &mut gba::Gba, a: u32| g.read8(a) as i32;
        d.set("page", r8(g, 0x0200_310C));
        let n = g.read16(ch) as u32;
        d.set("name", &gs(txt(n)));
        let n = g.read16(cl) as u32;
        d.set("class", &gs(txt(n)));
        d.set("level", r8(g, u + 8));
        d.set("exp", r8(g, u + 9));
        d.set("hp", r8(g, u + 0x13));
        d.set("max_hp", r8(g, u + 0x12));
        // physical or magic user (class base weapon ranks)
        let mut magic = false;
        let mut phys = false;
        for k in 0..8u32 {
            let base = g.read8(cl + 0x2C + k) as u32 + g.read8(u + 0x28 + k) as u32;
            if base > 0 {
                if k >= 4 { magic = true } else { phys = true }
            }
        }
        let magic_user = magic && !phys;
        let labels = [if magic_user { "Mag" } else { "Str" }, "Skill", "Spd", "Luck", "Def", "Res"];
        let vals = [0x14u32, 0x15, 0x16, 0x19, 0x17, 0x18];
        let caps = [r8(g, cl + 0x14), r8(g, cl + 0x15), r8(g, cl + 0x16), 30, r8(g, cl + 0x17), r8(g, cl + 0x18)];
        let mut stats = VarArray::new();
        for i in 0..6 {
            let mut e = VarArray::new();
            e.push(&labels[i].to_variant());
            e.push(&r8(g, u + vals[i]).to_variant());
            e.push(&caps[i].to_variant());
            stats.push(&e.to_variant());
        }
        d.set("stats", &stats);
        let con = r8(g, cl + 0x11) + r8(g, ch + 0x13) + r8(g, u + 0x1A);
        d.set("con", con);
        d.set("con_cap", r8(g, cl + 0x19));
        d.set("move", r8(g, cl + 0x12) + r8(g, u + 0x1D));
        let attrs = g.read32(cl + 0x28) | g.read32(ch + 0x28);
        let aid = if attrs & 1 != 0 { if attrs & 0x4000 != 0 { 20 - con } else { 25 - con } } else { con - 1 };
        d.set("aid", aid.max(0));
        let rescue = g.read8(u + 0x1B) as u32;
        let mut trv = String::new();
        if rescue != 0 {
            let ru = g.read32(0x08B9_2EB0 + 4 * rescue);
            let rc = g.read32(ru);
            if rc >> 24 == 8 {
                let id = g.read16(rc) as u32;
                trv = txt(id);
            }
        }
        d.set("trv", &gs(trv));
        d.set("affinity", r8(g, ch + 9));
        let cond = ["", "Poison", "Sleep", "Silence", "Berserk", "Attack", "Defense", "Critical", "Avoid", "Sick", "Recover"];
        let st = (g.read8(u + 0x30) & 0x0F) as usize;
        d.set("cond", &gs(cond.get(st).unwrap_or(&"").to_string()));
        // items
        let weapon = g.read16(0x0203_A3F0 + 0x48) as u32 & 0xFF;
        let mut items = VarArray::new();
        let mut eq_done = false;
        let mut rng = String::new();
        for k in 0..5u32 {
            let it = g.read16(u + 0x1E + 2 * k) as u32;
            if it & 0xFF == 0 {
                break;
            }
            let ia = 0x08BE_222C + (it & 0xFF) * 0x24;
            let mut e = VarDictionary::new();
            let nid = g.read16(ia) as u32;
            e.set("name", &gs(txt(nid)));
            let unbreak = g.read32(ia + 8) & 8 != 0;
            e.set("uses", if unbreak { -1 } else { (it >> 8) as i32 });
            e.set("max", if unbreak { -1 } else { r8(g, ia + 0x14) });
            let eq = !eq_done && weapon != 0 && it & 0xFF == weapon;
            if eq {
                eq_done = true;
                let r = g.read8(ia + 0x19);
                rng = if r >> 4 == r & 15 { format!("{}", r & 15) } else { format!("{}-{}", r >> 4, r & 15) };
            }
            e.set("equipped", eq);
            items.push(&e.to_variant());
        }
        d.set("items", &items);
        let rd = |g: &mut gba::Gba, o: u32| g.read16(0x0203_A3F0 + o) as i16 as i32;
        let equip = if weapon != 0 {
            PackedInt32Array::from(&[rd(g, 0x5A), rd(g, 0x60), rd(g, 0x66), rd(g, 0x62)][..])
        } else {
            PackedInt32Array::from(&[-1, -1, -1, rd(g, 0x62)][..])
        };
        d.set("equip", &equip);
        d.set("rng", &gs(if rng.is_empty() { "--".into() } else { rng }));
        // weapon ranks: the set the game shows (physical or magic)
        let order: [u32; 4] = if magic_user { [5, 7, 6, 4] } else { [0, 1, 2, 3] };
        let mut ranks = VarArray::new();
        for k in order {
            let v = g.read8(u + 0x28 + k) as i32;
            let (letter, lo, hi) = match v {
                0 => ("-", 0, 1),
                1..=30 => ("E", 1, 31),
                31..=70 => ("D", 31, 71),
                71..=120 => ("C", 71, 121),
                121..=180 => ("B", 121, 181),
                181..=250 => ("A", 181, 251),
                _ => ("S", 251, 251),
            };
            let mut e = VarDictionary::new();
            e.set("type", k as i32);
            e.set("letter", &gs(letter.to_string()));
            e.set("frac", if hi > lo { (v - lo) as f32 / (hi - lo) as f32 } else { 1.0 });
            ranks.push(&e.to_variant());
        }
        d.set("ranks", &ranks);
        // supports (level C and above)
        let mut sups = VarArray::new();
        let sp = g.read32(ch + 0x2C);
        if sp >> 24 == 8 {
            for k in 0..7u32 {
                let partner = g.read8(sp + k) as u32;
                if partner == 0 {
                    continue;
                }
                let pts = g.read8(u + 0x32 + k) as i32;
                let lv = if pts > 240 { "A" } else if pts > 160 { "B" } else if pts > 80 { "C" } else { "" };
                if lv.is_empty() {
                    continue;
                }
                let pc = 0x08BD_CE18 + partner * 0x34;
                let id = g.read16(pc) as u32;
                let mut e = VarDictionary::new();
                e.set("name", &gs(txt(id)));
                e.set("level", &gs(lv.to_string()));
                sups.push(&e.to_variant());
            }
        }
        d.set("supports", &sups);
        // screen brightness (fade in/out of the screen)
        let bldcnt = g.io16(0x50);
        let evy = (g.io16(0x54) & 31).min(16) as i32;
        let fade = match (bldcnt >> 6) & 3 {
            2 => evy,
            3 => -evy,
            _ => 0,
        };
        d.set("fade", if g.io16(0) & 0x80 != 0 { -16 } else { fade });
        d.set("ext", g.ppu.ext_left as i32);
        d
    }

    /// Item description box: {rect: Rect2i, text: String} or empty.
    #[func]
    fn help(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = self.g.as_ref() else { return d };
        if let Some(h) = &g.hd.help {
            let (x0, y0, x1, y1) = h.rect;
            let ox = g.ppu.ext_left as i32 + h.shift;
            d.set("rect", Rect2i::new(Vector2i::new(x0 + ox, y0), Vector2i::new(x1 - x0, y1 - y0)));
            d.set("text", &GString::from(h.text.as_str()));
            if let Some(st) = h.stats {
                d.set("stats", &PackedInt32Array::from(&st[..]));
            }
            if let Some((ix, iy)) = h.icon {
                d.set("icon", Vector2i::new(ix, iy));
            }
        }
        d
    }

    /// The game's text/icon layer (BG0) of the last frame, 240x160.
    #[func]
    fn bg0_image(&self) -> Option<Gd<Image>> {
        let g = self.g.as_ref()?;
        if g.hd.bg0.len() != 240 * 160 {
            return None;
        }
        image(240, 160, &g.hd.bg0)
    }

    /// An OBJ graphic from video memory (4bpp, 1D mapping), e.g. the menu hand: tile 0, palette 0.
    #[func]
    fn obj_image(&self, tile: i32, pal: i32, w: i32, h: i32) -> Option<Gd<Image>> {
        let g = self.g.as_ref()?;
        let (w, h) = (w.clamp(8, 64) as usize, h.clamp(8, 64) as usize);
        let mut px = vec![0u32; w * h];
        for y in 0..h {
            for x in 0..w {
                // 1D or 2D OBJ tile mapping
                let stride = if g.io16(0) & 0x40 != 0 { w / 8 } else { 32 };
                let t = tile as usize + (y / 8) * stride + x / 8;
                let a = 0x10000 + (t & 0x3FF) * 32 + (y % 8) * 4 + (x % 8) / 2;
                let n = (g.vram[a.min(0x17FFF)] >> ((x & 1) * 4)) & 15;
                if n != 0 {
                    let ci = 0x100 + pal as usize * 16 + n as usize;
                    let c = (g.pal[ci * 2] as u32 | (g.pal[ci * 2 + 1] as u32) << 8) & 0x7FFF;
                    px[y * w + x] = g.ppu.rgb_lut[c as usize] | 0xFF00_0000;
                }
            }
        }
        image(w, h, &px)
    }

    /// Debug: the next chapter start loads chapter `id` instead (-1 cancels).
    #[func]
    fn debug_chapter(&mut self, id: i32) {
        let g = self.g.as_mut().unwrap();
        g.debug_chapter = if (0..=0xFF).contains(&id) { Some(id as u8) } else { None };
    }

    /// Debug: chapter still waiting to be started, or -1.
    #[func]
    fn debug_chapter_pending(&self) -> i32 {
        self.g.as_ref().unwrap().debug_chapter.map_or(-1, |c| c as i32)
    }

    /// Chapter number (gPlaySt + 0x0E).
    #[func]
    fn chapter(&mut self) -> i32 {
        self.g.as_mut().unwrap().read8(0x0202_BC06) as i32
    }

    /// Sound since the last call: stereo frames at 32768 Hz.
    #[func]
    fn audio(&mut self) -> PackedVector2Array {
        let g = self.g.as_mut().unwrap();
        let s = std::mem::take(&mut g.apu.out);
        let v: Vec<Vector2> = s.chunks_exact(2).map(|c| Vector2::new(c[0], c[1])).collect();
        PackedVector2Array::from(v.as_slice())
    }

    #[func]
    fn save_state(&self, path: GString) -> bool {
        let g = self.g.as_ref().unwrap();
        std::fs::write(path.to_string(), g.save_state()).is_ok()
    }

    #[func]
    fn load_state(&mut self, path: GString) -> bool {
        let g = self.g.as_mut().unwrap();
        match std::fs::read(path.to_string()) {
            Ok(d) => {
                let ok = g.load_state(&d);
                g.hd.on = self.hd_on;
                g.hd.talk_native = self.talk_native;
                g.hd.menu_native = self.menu_native;
                g.hd.instant_text = self.instant_text;
                // strings seen before the load belong to another game state
                g.hd.texts.clear();
                g.hd.live.clear();
                g.hd.char_tiles.clear();
                g.hd.pending_chars.clear();
                self.drive = Drive::None;
                ok
            }
            Err(_) => false,
        }
    }

    /// Writes the battery save now (on quit).
    #[func]
    fn flush(&mut self) {
        if let Some(g) = self.g.as_ref() {
            let _ = std::fs::write(self.rom.with_extension("sav"), &g.sram[..0x8000]);
        }
    }
}

// Rust's std references pidfd_spawnp/pidfd_getpid (glibc >= 2.39) weakly;
// defining them keeps the library loadable on glibc >= 2.34.
#[no_mangle]
pub extern "C" fn pidfd_spawnp(_a: *mut i32, _b: *const u8, _c: *const u8, _d: *const u8, _e: *const *const u8, _f: *const *const u8) -> i32 {
    38
}
#[no_mangle]
pub extern "C" fn pidfd_getpid(_fd: i32) -> i32 {
    -1
}
