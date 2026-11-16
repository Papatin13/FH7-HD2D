//! Mouse control for Fire Emblem 7 (USA).
//!
//! The mouse is translated into button presses based on what the game is
//! currently doing (read from game RAM):
//! * Battle map with the cursor active: hovering moves the map cursor to the
//!   tile under the mouse, left click selects (A), right click cancels (B).
//! * Menus: hovering highlights the item under the mouse, left click chooses it.
//! * Anything else (dialogue, cutscenes): left click = A, right click = B.
//! Mouse wheel scrolls menus (Up/Down) or cycles units on the map (L).
//! Middle click = Start.
use gba::{keys, Gba};

/// gBmSt (battle map state): camera at +0x0C, cursor tile at +0x14.
const BMST: u32 = 0x0202_BBB8;
const PROC_ARRAY: u32 = 0x0202_4E28;
const PROC_SIZE: u32 = 0x6C;
const PROC_COUNT: u32 = 64;
const SCR_MENU: u32 = 0x08B9_A8A0;
const SCR_PLAYER_PHASE: u32 = 0x08B9_3374;
const SCR_TALK: u32 = 0x08B9_0D88;
/// Title-screen main menu (Resume / Restart / Copy / Erase / New Game) and save-slot selection.
const SCR_MAIN_MENU: u32 = 0x08CE_3C54;
const MAIN_MENU_LOOP: u32 = 0x080A_39F9;
const SLOT_SELECT_LOOP: u32 = 0x080A_3E99;
/// Yes/no box shown on the title screen ("Suspended data will be lost. Start / Quit").
const SCR_TITLE_CONFIRM: u32 = 0x08CC_2014;
/// Trade screen (two inventories side by side). Proc fields: +0x2C/+0x30 the
/// two units, +0x34.. valid-slot flags (column*6 + row), +0x41 cursor column,
/// +0x42 cursor row. Idle loops: choosing the item / choosing where it goes.
const SCR_TRADE: u32 = 0x08B9_42F8;
const TRADE_LOOPS: [u32; 2] = [0x0802_B135, 0x0802_B289];

/// A vertical list drawn at fixed screen positions (title menus).
#[derive(Clone, Copy)]
struct List {
    cur_addr: u32,
    count: i32,
    top: i32,
    pitch: i32,
    x0: i32,
    x1: i32,
    wrap: bool,
    /// Items laid out left-to-right (x0 + i*pitch); rows top..x1 used as y-range.
    horiz: bool,
    /// Value stored for item 0 (the confirm box stores 1/2).
    base: i32,
}

#[derive(Default)]
pub struct MouseInput {
    /// Position in GBA screen pixels (None if outside the picture).
    pub pos: Option<(i32, i32)>,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub wheel: i32,
}

#[derive(Default)]
pub struct Mouse {
    last_out: u16,
    last_pos: Option<(i32, i32)>,
    map_target: Option<(i32, i32)>,
    pending_click: bool,
    click_timeout: u32,
    pending_keys: Vec<u16>,
    pub mode: &'static str,
    /// Generic screens: drive the game's pointing hand towards the mouse.
    hand_target: Option<(i32, i32)>,
    hand_horiz: bool,
    hand_click: bool,
    hand_last: (i32, i32),
    hand_stuck: u32,
    hand_wait: u32,
}

/// The game's pointing hands on screen (16x16 OBJ, tile 0, palette 0), sorted by x.
fn hands(g: &Gba) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for i in 0..128 {
        let o = i * 8;
        let a0 = g.oam[o] as u32 | (g.oam[o + 1] as u32) << 8;
        let a1 = g.oam[o + 2] as u32 | (g.oam[o + 3] as u32) << 8;
        let a2 = g.oam[o + 4] as u32 | (g.oam[o + 5] as u32) << 8;
        if a0 & 0x300 == 0x200 || a0 >> 14 != 0 || a1 >> 14 != 1 {
            continue;
        }
        if a2 & 0x3FF != 0 || a2 >> 12 != 0 {
            continue;
        }
        let mut y = (a0 & 0xFF) as i32;
        if y >= 160 {
            y -= 256;
        }
        let x = (a1 & 0x1FF) as i32;
        let x = if x >= 256 { x - 512 } else { x };
        if x > -16 && x < 240 && y > -16 && y < 160 {
            v.push((x, y));
        }
    }
    v.sort();
    v
}

#[derive(Clone, Copy)]
enum Ctx {
    Map,
    Menu(u32),
    List(List),
    Trade(u32),
    /// Unit list (rows with a highlight bar): proc.
    Rows(u32),
    Other,
}

const SCR_UNIT_LIST: u32 = 0x08CC_3194;

fn r8(g: &mut Gba, a: u32) -> u8 {
    g.read8(a)
}
fn r16s(g: &mut Gba, a: u32) -> i32 {
    g.read16(a) as i16 as i32
}
fn r32(g: &mut Gba, a: u32) -> u32 {
    g.read32(a)
}

fn procs_with(g: &mut Gba, script: u32) -> Vec<u32> {
    (0..PROC_COUNT)
        .map(|i| PROC_ARRAY + i * PROC_SIZE)
        .filter(|&p| r32(g, p) == script)
        .collect()
}

/// Item index under the mouse for a menu proc, plus the item count/current.
fn menu_hit(g: &mut Gba, m: u32, pos: (i32, i32)) -> Option<usize> {
    let count = r8(g, m + 0x60) as usize;
    if count == 0 || count > 11 {
        return None;
    }
    let rx = r8(g, m + 0x2C) as i8 as i32;
    let rw = r8(g, m + 0x2E) as i32;
    for i in 0..count {
        let item = r32(g, m + 0x34 + 4 * i as u32);
        if item >> 24 != 2 {
            continue;
        }
        let xt = r16s(g, item + 0x2A);
        let yt = r16s(g, item + 0x2C);
        let _ = xt;
        // the whole row of the window is clickable (icons and margins too)
        let x0 = rx * 8;
        let x1 = ((rx + rw) * 8).max(x0 + 24);
        let y0 = yt * 8;
        if pos.0 >= x0 && pos.0 < x1 && pos.1 >= y0 && pos.1 < y0 + 16 {
            return Some(i);
        }
    }
    None
}

impl Mouse {
    /// Lists, option screens, shops ...: hover moves the hand to the row under the
    /// mouse; a click on a row confirms it, a click on a value to the right of the
    /// row's hand (second hand on the same row, e.g. options) moves that hand there.
    fn hand_follow(&mut self, hs: &[(i32, i32)], inp: &MouseInput, moved: bool) -> u16 {
        let row = hs[0];
        let value = hs.iter().copied().find(|h| *h != row && (h.1 - row.1).abs() < 6);
        if let Some(p) = inp.pos {
            if moved || inp.left {
                let horiz = inp.left && value.map(|v| p.0 >= v.0 - 4 && (p.1 - (row.1 + 8)).abs() <= 8).unwrap_or(false);
                self.hand_target = Some(p);
                self.hand_horiz = horiz;
                self.hand_click = inp.left && !horiz;
                self.hand_stuck = 0;
                self.hand_wait = 0;
            }
        }
        let Some((tx, ty)) = self.hand_target else { return 0 };
        let cur = if self.hand_horiz { value.unwrap_or(row) } else { row };
        if std::env::var("FE7_HANDLOG").is_ok() {
            eprintln!("hand row {:?} value {:?} target {:?} horiz {} wait {} stuck {}", row, value, (tx, ty), self.hand_horiz, self.hand_wait, self.hand_stuck);
        }
        // the game slides the hand: wait until it has stopped after the last press
        // (the hand also bobs by a pixel or two all the time)
        let moving = (cur.0 - self.hand_last.0).abs() > 2 || (cur.1 - self.hand_last.1).abs() > 2;
        if moving || self.hand_wait == 0 {
            self.hand_last = cur;
        }
        if moving {
            self.hand_wait = 3;
            self.hand_stuck = 0;
            return 0;
        }
        if self.hand_wait > 0 {
            self.hand_wait -= 1;
            return 0;
        }
        if !self.pending_keys.is_empty() {
            return 0;
        }
        let key = if self.hand_horiz {
            if tx < cur.0 {
                keys::LEFT
            } else if tx > cur.0 + 40 {
                keys::RIGHT
            } else {
                0
            }
        } else if ty < row.1 {
            keys::UP
        } else if ty >= row.1 + 16 {
            keys::DOWN
        } else {
            0
        };
        if key == 0 {
            self.hand_target = None;
            if self.hand_click {
                self.pending_keys.push(keys::A);
            }
            return 0;
        }
        self.hand_stuck += 1;
        if self.hand_stuck > 2 {
            // pressed but nothing moved: end of the list
            self.hand_target = None;
            return 0;
        }
        self.hand_wait = 8;
        self.pending_keys.push(key);
        0
    }

    fn context(&self, g: &mut Gba) -> Ctx {
        // Trade screen (opened from the map; the map procs stay alive underneath).
        if let Some(&p) = procs_with(g, SCR_TRADE).first() {
            return if TRADE_LOOPS.contains(&r32(g, p + 0x0C)) { Ctx::Trade(p) } else { Ctx::Other };
        }
        if let Some(&p) = procs_with(g, SCR_UNIT_LIST).first() {
            return Ctx::Rows(p);
        }
        // Player phase waiting for cursor input (free cursor or choosing a target tile).
        for p in procs_with(g, SCR_PLAYER_PHASE) {
            if r32(g, p + 0x0C) != 0 {
                return Ctx::Map;
            }
        }
        // Title screen menus.
        let confirm_open = !procs_with(g, SCR_TITLE_CONFIRM).is_empty();
        for p in procs_with(g, SCR_MAIN_MENU) {
            if confirm_open {
                // Start (1) at x 30..78, Quit (2) at x 78..126, y 126..158
                return Ctx::List(List { cur_addr: p + 0x36, count: 2, top: 126, pitch: 48, x0: 30, x1: 158, wrap: false, horiz: true, base: 1 });
            }
            let idle = r32(g, p + 0x0C);
            if idle == MAIN_MENU_LOOP {
                let count = r8(g, p + 0x31) as i32;
                if (1..=8).contains(&count) {
                    return Ctx::List(List {
                        cur_addr: p + 0x2B,
                        count,
                        top: 64 - (count - 1) * 12 - 3,
                        pitch: 24,
                        x0: 40,
                        x1: 200,
                        wrap: false,
                        horiz: false,
                        base: 0,
                    });
                }
            } else if idle == SLOT_SELECT_LOOP {
                return Ctx::List(List {
                    cur_addr: p + 0x2C,
                    count: 3,
                    top: 33,
                    pitch: 32,
                    x0: 8,
                    x1: 232,
                    wrap: false,
                    horiz: false,
                    base: 0,
                });
            }
        }
        // A dialogue box takes input before menus.
        if !procs_with(g, SCR_TALK).is_empty() {
            return Ctx::Other;
        }
        let menus = procs_with(g, SCR_MENU);
        // Prefer the newest menu that has items; menus deeper in the array are newer.
        if let Some(&m) = menus.iter().rev().find(|&&m| {
            let c = g.read8(m + 0x60);
            c > 0 && c <= 11
        }) {
            return Ctx::Menu(m);
        }
        Ctx::Other
    }

    /// Returns the buttons the mouse wants pressed this frame.
    pub fn update(&mut self, g: &mut Gba, inp: &MouseInput) -> u16 {
        let mut want: u16 = 0;
        let moved = inp.pos.is_some() && inp.pos != self.last_pos;
        self.last_pos = inp.pos;
        let ctx = self.context(g);
        self.mode = match ctx {
            Ctx::Map => "map",
            Ctx::Menu(_) => "menu",
            Ctx::List(_) => "title-menu",
            Ctx::Trade(_) => "trade",
            Ctx::Rows(_) => "list",
            Ctx::Other => "other",
        };

        if inp.right {
            self.pending_click = false;
            self.map_target = None;
            self.pending_keys.push(keys::B);
        }
        if inp.middle {
            self.pending_keys.push(keys::START);
        }

        match ctx {
            Ctx::Map => {
                let camx = r16s(g, BMST + 0x0C);
                let camy = r16s(g, BMST + 0x0E);
                let cx = r16s(g, BMST + 0x14);
                let cy = r16s(g, BMST + 0x16);
                if let Some(p) = inp.pos {
                    if moved || inp.left {
                        self.map_target = Some(((p.0 + camx).div_euclid(16), (p.1 + camy).div_euclid(16)));
                    }
                }
                if inp.left {
                    self.pending_click = true;
                    self.click_timeout = 90;
                }
                if inp.wheel != 0 {
                    self.pending_keys.push(keys::L);
                    self.map_target = None;
                }
                if let Some((tx, ty)) = self.map_target {
                    if tx < cx {
                        want |= keys::LEFT;
                    } else if tx > cx {
                        want |= keys::RIGHT;
                    }
                    if ty < cy {
                        want |= keys::UP;
                    } else if ty > cy {
                        want |= keys::DOWN;
                    }
                    if want == 0 {
                        self.map_target = None;
                    }
                }
                // cursor sprite still sliding towards its tile?
                let settled = r16s(g, BMST + 0x20) == cx * 16 && r16s(g, BMST + 0x22) == cy * 16;
                if self.pending_click {
                    self.click_timeout = self.click_timeout.saturating_sub(1);
                    if (want == 0 && settled) || self.click_timeout == 0 {
                        self.pending_click = false;
                        self.map_target = None;
                        self.pending_keys.push(keys::A);
                        want = 0;
                    }
                }
            }
            Ctx::Menu(_) | Ctx::List(_) => {
                self.map_target = None;
                let (count, cur, hit, wrap, horiz) = match ctx {
                    Ctx::Menu(m) => {
                        let hit = inp.pos.and_then(|p| menu_hit(g, m, p)).map(|h| h as i32);
                        (r8(g, m + 0x60) as i32, r8(g, m + 0x61) as i32, hit, true, false)
                    }
                    Ctx::List(l) => {
                        let hit = inp.pos.and_then(|(x, y)| {
                            if l.horiz {
                                let i = (x - l.x0).div_euclid(l.pitch);
                                (x >= l.x0 && y >= l.top && y < l.x1 && i < l.count).then_some(i)
                            } else {
                                let i = (y - l.top).div_euclid(l.pitch);
                                (x >= l.x0 && x < l.x1 && y >= l.top && i < l.count).then_some(i)
                            }
                        });
                        (l.count, r8(g, l.cur_addr) as i32 - l.base, hit, l.wrap, l.horiz)
                    }
                    _ => unreachable!(),
                };
                if inp.wheel != 0 {
                    self.pending_keys.push(if inp.wheel > 0 { keys::UP } else { keys::DOWN });
                }
                if let Some(t) = hit {
                    if t != cur && (moved || self.pending_click) {
                        let go_down = if wrap {
                            (t - cur).rem_euclid(count) <= (cur - t).rem_euclid(count)
                        } else {
                            t > cur
                        };
                        want |= match (go_down, horiz) {
                            (true, false) => keys::DOWN,
                            (false, false) => keys::UP,
                            (true, true) => keys::RIGHT,
                            (false, true) => keys::LEFT,
                        };
                    }
                    if inp.left {
                        self.pending_click = true;
                        self.click_timeout = 60;
                    }
                    if self.pending_click && t == cur {
                        self.pending_click = false;
                        self.pending_keys.push(keys::A);
                        want = 0;
                    }
                }
                if self.pending_click {
                    self.click_timeout = self.click_timeout.saturating_sub(1);
                    if self.click_timeout == 0 {
                        self.pending_click = false;
                    }
                }
            }
            Ctx::Trade(p) => {
                let (cc, cr) = (r8(g, p + 0x41) as i32, r8(g, p + 0x42) as i32);
                // Item rows: 16 px high from y=72; left column x 8..120, right 120..232.
                let hit = inp.pos.and_then(|(x, y)| {
                    if !(8..232).contains(&x) || !(72..152).contains(&y) {
                        return None;
                    }
                    let (c, r) = (((x - 8) / 112).min(1), (y - 72) / 16);
                    (r8(g, p + 0x34 + (c * 6 + r) as u32) != 0).then_some((c, r))
                });
                if inp.wheel != 0 {
                    self.pending_keys.push(if inp.wheel > 0 { keys::UP } else { keys::DOWN });
                }
                if moved || inp.left {
                    self.map_target = hit;
                }
                if inp.left && hit.is_some() {
                    self.pending_click = true;
                    self.click_timeout = 60;
                }
                if let Some((c, r)) = self.map_target {
                    if r8(g, p + 0x34 + (c * 6 + r) as u32) == 0 || (c, r) == (cc, cr) {
                        self.map_target = None;
                    } else {
                        want |= if c != cc {
                            if c > cc { keys::RIGHT } else { keys::LEFT }
                        } else if r > cr {
                            keys::DOWN
                        } else {
                            keys::UP
                        };
                    }
                }
                if self.pending_click {
                    self.click_timeout = self.click_timeout.saturating_sub(1);
                    if self.map_target.is_none() {
                        self.pending_click = false;
                        if hit == Some((cc, cr)) || self.click_timeout > 0 {
                            self.pending_keys.push(keys::A);
                        }
                        want = 0;
                    } else if self.click_timeout == 0 {
                        self.pending_click = false;
                        self.map_target = None;
                    }
                }
            }
            Ctx::Rows(p) => {
                // rows 16 px high from y = 48, six visible
                let vis = r8(g, p + 0x30) as i32;
                if inp.wheel != 0 {
                    self.pending_keys.push(if inp.wheel > 0 { keys::UP } else { keys::DOWN });
                }
                if let Some((x, y)) = inp.pos {
                    if (moved || inp.left) && (8..232).contains(&x) && (48..144).contains(&y) {
                        self.map_target = Some((0, (y - 48) / 16));
                        self.pending_click = inp.left;
                        self.hand_stuck = 0;
                        self.hand_wait = 0;
                    }
                }
                if let Some((_, t)) = self.map_target {
                    if self.hand_wait > 0 {
                        self.hand_wait -= 1;
                    } else if self.pending_keys.is_empty() {
                        if t == vis {
                            self.map_target = None;
                            if self.pending_click {
                                self.pending_keys.push(keys::A);
                            }
                            self.pending_click = false;
                        } else if self.hand_stuck > 1 && vis == self.hand_last.1 {
                            self.map_target = None; // end of the list
                            self.pending_click = false;
                        } else {
                            if vis == self.hand_last.1 {
                                self.hand_stuck += 1;
                            } else {
                                self.hand_stuck = 0;
                            }
                            self.hand_last = (0, vis);
                            self.pending_keys.push(if t > vis { keys::DOWN } else { keys::UP });
                            self.hand_wait = 6;
                        }
                    }
                }
            }
            Ctx::Other => {
                self.map_target = None;
                self.pending_click = false;
                if inp.wheel != 0 {
                    self.pending_keys.push(if inp.wheel > 0 { keys::UP } else { keys::DOWN });
                }
                let hs = hands(g);
                if hs.is_empty() {
                    self.hand_target = None;
                    if inp.left {
                        self.pending_keys.push(keys::A);
                    }
                } else {
                    want |= self.hand_follow(&hs, inp, moved);
                }
            }
        }

        // Queued single presses take priority.
        if !self.pending_keys.is_empty() {
            want = self.pending_keys[0];
        }
        // Emit presses as pulses: a key held last frame is released first, so
        // every request registers as a fresh press in the game.
        let out = if want & self.last_out != 0 { 0 } else { want };
        if out != 0 && !self.pending_keys.is_empty() && out == self.pending_keys[0] {
            self.pending_keys.remove(0);
        }
        self.last_out = out;
        out
    }
}

/// Screen position (game coordinates incl. widescreen columns) -> position in
/// the game's own 240 px layout, undoing the widescreen HUD shift of windows
/// that were moved to the screen edges.
pub fn unshift(g: &Gba, (x, y): (i32, i32)) -> (i32, i32) {
    for &(x0, y0, x1, y1, sh) in &g.ppu.ui_boxes {
        if y >= y0 && y < y1 && x >= x0 + sh && x < x1 + sh {
            return (x - sh, y);
        }
    }
    (x, y)
}
