//! HD-2D support: splits a battle-map frame into the 3D world (terrain,
//! unit sprites, movement/attack ranges, cursor) and a 2D interface layer
//! (windows, menus, dialogues, portraits) with transparency.
//!
//! The frontend draws the world itself from the data collected here and puts
//! the interface layer (`ppu.frame`, ARGB with alpha) on top. Outside the
//! battle map (`hd.map == false`) the frame is the normal, opaque picture.
use crate::Gba;

const MAP_TERRAIN: u32 = 0x0202_E3E0;
const MAP_FOG: u32 = 0x0202_E3EC;
const BASE_TILE_ROWS: u32 = 0x0203_2E90;
const METATILES: u32 = 0x0203_0A90;
const TRANSPARENT: u32 = 0x8000_0000;

/// One unit sprite of the current frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct HdSprite {
    /// Position in map pixels (top-left) and size.
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Rectangle in the sprite atlas.
    pub ax: i32,
    pub ay: i32,
    /// Semi-transparent (OBJ mode 1).
    pub semi: bool,
    /// Lies flat on the ground (movement arrow) instead of standing.
    pub flat: bool,
}

pub const ATLAS: usize = 256;

#[derive(Default)]
pub struct Hd {
    /// HD-2D presentation requested by the frontend.
    pub on: bool,
    /// The battle map is on screen this frame (world + interface layer).
    pub map: bool,
    pub sprites: Vec<HdSprite>,
    /// ATLAS x ATLAS, 0xAARRGGBB.
    pub atlas: Vec<u32>,
    /// Map pixel at the top-left of the original 240x160 view.
    pub view: (i32, i32),
    /// Map cursor (map pixels of the cursor tile's top-left), if shown.
    pub cursor: Option<(i32, i32)>,
    /// Screen brightness effect on the world: -16 (black) .. 0 .. 16 (white).
    pub fade: i32,
    /// Movement/attack overlay per map cell (0 none, 1 blue, 2 red), row-major.
    pub cells: Vec<u8>,
    pub cells_w: i32,
    /// Speech bubbles are drawn by the frontend (the game's bubble is removed).
    pub talk_native: bool,
    /// The speech bubble on screen this frame.
    pub talk: Option<TalkBox>,
    /// 30x20 screen cells of BG0/BG1 that belong to the bubble or a menu (hidden).
    pub talk_mask: Vec<bool>,
    /// Menu windows drawn by the frontend.
    pub menu_native: bool,
    /// Cells hidden last frame; kept hidden after a window closes until the game
    /// has really erased its tiles (it closes the menu first, the tiles go later).
    pub linger: Vec<bool>,
    /// BG0/BG1 entries of those cells when they were last hidden.
    pub linger_ent: Vec<(u32, u32)>,
    pub linger_t: u32,
    /// Battle forecast drawn by the frontend.
    pub forecast: Option<Forecast>,
    /// Info windows of the free map (goal, terrain, unit under the cursor).
    pub mapinfo: MapInfo,
    /// Full-width banner: (kind 0 = phase, 1 = chapter title, text, frames shown).
    pub banner: Option<(i32, String, i32)>,
    /// OBJs with their centre inside one of these rects are not drawn (game pixels).
    pub obj_hide: Vec<(i32, i32, i32, i32)>,
    /// Hide the game's pointing hand (16x16, priority 0) everywhere.
    pub hide_hand: bool,
    /// The map overview (START on the map) is open; drawn by the frontend.
    pub minimap: bool,
    /// The mask only applies to BG0 (battle scene: BG1 holds spell effects).
    pub mask_bg0_only: bool,
    /// BG1 screen cells hidden in the battle scene (pop-ups like "You can now use").
    pub bg1_mask: Vec<bool>,
    /// EXP gauge of a battle on the map (no animation): rect in game pixels.
    pub mapexp: Option<(i32, i32, i32, i32)>,
    /// Any other window of the game (pop-ups like "Got an Iron sword", item help,
    /// stat boxes): (x0, y0, x1, y1, shift) in game pixels. Its frame and
    /// background are hidden; the frontend draws a native window and puts the
    /// game's text layer (BG0) of that area on it.
    pub generic: Vec<(i32, i32, i32, i32, i32)>,
    /// Height of a name tab on top of each generic window (pixels, 0 none).
    pub generic_tab: Vec<i32>,
    /// Per generic window: recognised strings (x, y, colour, text) in game pixels
    /// and the BG0 cells (x, y) that are not text (icons and the like).
    pub generic_text: Vec<(Vec<(i32, i32, u8, String)>, Vec<(i32, i32)>)>,
    /// Strings the game drew into text tiles: (first tile, tiles, pixel x in the
    /// first tile, colour id, text), newest last.
    pub texts: Vec<(u32, u32, u32, u8, String)>,
    pub text_dbg: bool,
    /// Strings being printed letter by letter: text handle -> (record index key
    /// = first tile, next cursor x).
    pub live: std::collections::HashMap<u32, (u32, u32)>,
    /// UI digit glyphs: (tilemap entry address, digit) waiting to be resolved,
    /// and the resolved tile -> digit cache.
    pub pending_chars: Vec<(u32, u8)>,
    pub char_tiles: std::collections::HashMap<u32, u8>,
    /// Help bubble (R button, "There's no need for that." ...): x, y, w, h, text.
    pub rhelp: Option<(i32, i32, i32, i32, String)>,
    /// Map view of the preparations ("Check Map"): 0 off, 1 its menu is open, 2 free map.
    pub prepmap: i32,
    /// Trade screen: (left window rect, right window rect, shift, hands [(x, y)]).
    /// Shop screen shown (everything of the game's UI hidden, portrait rect in BG0).
    pub shop: Option<(i32, i32, i32, i32)>,
    /// World map narration box: text lines (the box itself is hidden).
    pub worldmap: Option<Vec<String>>,
    pub trade: Option<((i32, i32, i32, i32), (i32, i32, i32, i32), i32, Vec<(i32, i32)>)>,
    /// Last unit shown in the unit window (kept while it slides out).
    pub last_unit: Option<UnitWin>,
    pub unit_hash: u64,
    /// Text of the last tutorial box (shown while the box stays after printing).
    pub tut_last: Vec<String>,
    pub tut_full: Vec<String>,
    /// Show each page of a bubble at once instead of letter by letter.
    pub instant_text: bool,
    /// Something is hidden this frame (bubble, menu, help or leftovers).
    pub hiding: bool,
    pub menus: Vec<MenuBox>,
    /// Help box under an item list: rect (game px) and text.
    pub help: Option<HelpBox>,
    /// BG0 (text and icons) of the previous frame before hiding, 240x160 ARGB.
    pub bg0: Vec<u32>,
    /// All four BG layers of the last frame (240x160 each), when `cap_layers` is set.
    pub layers: Vec<u32>,
    pub cap_layers: bool,
}

/// A menu window (action menu, item list, map menu ...), in game pixels.
#[derive(Clone, Default, Debug)]
pub struct MenuBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub items: Vec<MenuEntry>,
    pub cur: i32,
    /// The newest menu (takes input).
    pub active: bool,
    pub shift: i32,
}

#[derive(Clone, Default, Debug)]
pub struct MenuEntry {
    /// Label position (game pixels).
    pub x: i32,
    pub y: i32,
    pub label: String,
    pub enabled: bool,
    /// Item list entry: icon at (x - 16, y), remaining uses (-1 none).
    pub icon: bool,
    pub uses: i32,
}

/// Box next to an item list: item description, or the weapon's battle stats.
#[derive(Clone, Default, Debug)]
pub struct HelpBox {
    pub rect: (i32, i32, i32, i32),
    pub shift: i32,
    pub text: String,
    /// Atk, Hit, Crit, Avoid (weapon selection).
    pub stats: Option<[i32; 4]>,
    /// Affinity icon position in game pixels (stats box).
    pub icon: Option<(i32, i32)>,
}

/// One side of the battle forecast.
#[derive(Clone, Default, Debug)]
pub struct FcSide {
    pub name: String,
    pub weapon: String,
    pub hp: i32,
    /// -1 = cannot attack ("--").
    pub mt: i32,
    pub hit: i32,
    pub crit: i32,
    pub double: bool,
    /// Weapon triangle: 1 advantage, -1 disadvantage.
    pub tri: i32,
    /// Weapon icon (BG0 pixel position), if found.
    pub icon: Option<(i32, i32)>,
}

/// Battle forecast window (choosing the target of an attack).
#[derive(Clone, Default, Debug)]
pub struct Forecast {
    pub rect: (i32, i32, i32, i32),
    pub shift: i32,
    pub player: FcSide,
    pub enemy: FcSide,
}

/// Windows of the free map. Rects in game pixels (x0, y0, x1, y1).
#[derive(Clone, Default, Debug)]
pub struct MapInfo {
    pub goal: Option<((i32, i32, i32, i32), i32, String, String)>,
    pub terrain: Option<((i32, i32, i32, i32), i32, String, i32, i32)>,
    pub unit: Option<((i32, i32, i32, i32), i32, UnitWin)>,
    /// HP boxes of a battle shown on the map (no animation): rect, shift, name, hp, max, side.
    pub fights: Vec<((i32, i32, i32, i32), i32, String, i32, i32, i32)>,
}

#[derive(Clone, Default, Debug)]
pub struct UnitWin {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    /// 0 player, 1 other (green), 2 enemy
    pub side: i32,
    /// Mini portrait in the BG0 picture (game pixels).
    pub face: Option<(i32, i32, i32, i32)>,
}

/// A speech bubble, in game pixels of the original 240x160 picture.
#[derive(Clone, Default, Debug)]
pub struct TalkBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    /// Bubble tail cells (bounding box), if any.
    pub tail: Option<(i32, i32, i32, i32)>,
    /// Visible lines (BBCode: red words as [color=red]).
    pub lines: Vec<String>,
    /// Name plate (opening scene).
    pub name: String,
    /// Waiting for a button press (arrow).
    pub waiting: bool,
    /// Horizontal HUD shift (widescreen battle map).
    pub shift: i32,
    /// Yes/No question: selected answer (0 yes, 1 no), -1 if none.
    pub choice: i32,
    /// The whole current page (for sizing the text before it is fully printed).
    pub full: Vec<String>,
    /// 0 speech bubble, 1 yellow tutorial box.
    pub style: u8,
}

const TALK_BUF: u32 = 0x0202_A5B4;
const TALK_STATE: u32 = 0x0300_0040;
const SCR_TALK: u32 = 0x08B9_0D88;
const SCR_OPENING_TALK: u32 = 0x08CC_306C;

#[derive(PartialEq, Clone, Copy)]
pub enum ObjKind {
    Ui,
    /// Unit sprite: stands upright in the world.
    Unit,
    /// Movement arrow: lies on the ground.
    Flat,
    /// Map cursor corners (the frontend draws its own cursor).
    Cursor,
}

impl Hd {
    /// What an OBJ on the battle map belongs to.
    #[inline]
    pub fn obj_kind(a0: u32, a1: u32, a2: u32) -> ObjKind {
        let prio = (a2 >> 10) & 3;
        let pal = a2 >> 12;
        let tile = a2 & 0x3FF;
        let small = a0 >> 14 == 0;
        if prio >= 2 && pal >= 12 {
            ObjKind::Unit
        } else if prio >= 2 && pal == 3 && a0 >> 14 == 0 && a1 >> 14 == 1 {
            // 16x16 pieces of the movement arrow (portraits use the same palette but bigger pieces)
            ObjKind::Flat
        } else if (prio >= 2 && pal == 2 && (0x20..0x30).contains(&tile)) || (pal == 0 && tile == 2 && small) {
            ObjKind::Cursor
        } else {
            ObjKind::Ui
        }
    }
    #[inline]
    pub fn world_obj(a0: u32, a1: u32, a2: u32) -> bool {
        let small = a1 >> 14 == 0;
        let k = Self::obj_kind(a0, a1, a2);
        k != ObjKind::Ui && (k != ObjKind::Cursor || small || a2 >> 12 == 2)
    }
}

const OBJ_SIZE: [[(i32, i32); 4]; 3] = [
    [(8, 8), (16, 16), (32, 32), (64, 64)],
    [(16, 8), (32, 8), (32, 16), (64, 32)],
    [(8, 16), (8, 32), (16, 32), (32, 64)],
];

impl Gba {
    fn rgb(&self, c: u32) -> u32 {
        self.ppu.rgb_lut[(c & 0x7FFF) as usize]
    }

    /// Start of a frame in HD mode (after `wide_frame_setup`).
    pub fn proc_find(&mut self, script: u32) -> Option<u32> {
        (0..64).map(|i| 0x0202_4E28 + i * 0x6C).find(|&p| self.read32(p) == script)
    }

    /// Finds the game's speech bubble (BG0/BG1 tiles 0x10-0x13 frame/fill and
    /// 0x14-0x19 tail, palette 3) and the text it shows.
    fn hd_talk_detect(&mut self) {
        self.hd.talk = None;
        if self.hd.talk_mask.len() != 600 {
            self.hd.talk_mask = vec![false; 600];
        }
        self.hd.talk_mask.iter_mut().for_each(|v| *v = false);
        if !self.hd.talk_native {
            return;
        }
        // yellow tutorial box (frame made of sprites, own text pointer). The game ends
        // the text process when the text is complete but keeps the box: keep the text.
        if let Some((x0, y0, x1, y1)) = self.tutorial_frame() {
            let tp = self.proc_find(0x08CC_2ACC);
            if tp.is_some() || !self.hd.tut_last.is_empty() {
                for cy in (y0 / 8).max(0)..(y1 / 8).min(20) {
                    for cx in (x0 / 8).max(0)..(x1 / 8).min(30) {
                        self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                    }
                }
                let lines = match tp {
                    Some(tp) => {
                        let ptr = self.read32(tp + 0x2C);
                        let max_lines = ((y1 - y0 - 8) / 16).clamp(1, 6) as usize;
                        let (l, _, w, f) = self.talk_text(ptr, false, max_lines);
                        self.hd.tut_last = l.clone();
                        self.hd.tut_full = f.clone();
                        (l, w, f)
                    }
                    None => (self.hd.tut_last.clone(), false, self.hd.tut_full.clone()),
                };
                let (lines, waiting, full) = (lines.0, lines.1, lines.2);
                self.hd.talk = Some(TalkBox { x0, y0, x1, y1, lines, waiting, choice: -1, style: 1, full, ..Default::default() });
                return;
            }
        } else {
            self.hd.tut_last.clear();
        }
        let opening = self.proc_find(SCR_OPENING_TALK);
        if opening.is_none() && self.proc_find(SCR_TALK).is_none() {
            return;
        }
        let ptr = match opening {
            Some(p) => self.read32(p + 0x2C),
            None => self.read32(TALK_STATE),
        };
        // yes/no choice inside the bubble (0x18); the opening scene keeps the game's own one
        let choosing = ptr > TALK_BUF && self.read8(ptr - 1) == 0x18;
        if choosing && opening.is_some() {
            return;
        }
        let choice = if choosing { self.read8(TALK_STATE + 0x12) as i32 & 1 } else { -1 };
        let dispcnt = self.io16(0);
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        let (mut tx0, mut ty0, mut tx1, mut ty1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        let mut n = 0;
        let mut cells: Vec<(i32, i32)> = Vec::new();
        for bg in 0..2usize {
            if dispcnt & (0x100 << bg) == 0 {
                continue;
            }
            let cnt = self.io16(8 + bg * 2) as usize;
            if cnt & 0xC000 != 0 {
                continue;
            }
            let sb = ((cnt >> 8) & 31) * 0x800;
            let hofs = (self.io16(0x10 + bg * 4) & 0x1FF) as i32;
            let vofs = (self.io16(0x12 + bg * 4) & 0x1FF) as i32;
            for my in 0..32i32 {
                for mx in 0..32i32 {
                    let a = sb + ((my * 32 + mx) * 2) as usize;
                    let ent = self.vram[a] as u32 | (self.vram[a + 1] as u32) << 8;
                    let t = ent & 0x3FF;
                    if ent >> 12 != 3 || !(0x10..=0x19).contains(&t) {
                        continue;
                    }
                    let sx = (mx * 8 - hofs).rem_euclid(256);
                    let sy = (my * 8 - vofs).rem_euclid(256);
                    if sx >= 240 || sy >= 160 {
                        continue;
                    }
                    if t <= 0x13 {
                        x0 = x0.min(sx);
                        y0 = y0.min(sy);
                        x1 = x1.max(sx + 8);
                        y1 = y1.max(sy + 8);
                        n += 1;
                    } else {
                        tx0 = tx0.min(sx);
                        ty0 = ty0.min(sy);
                        tx1 = tx1.max(sx + 8);
                        ty1 = ty1.max(sy + 8);
                    }
                    cells.push((sx / 8, sy / 8));
                }
            }
        }
        if n < 12 || x1 - x0 < 48 || y1 - y0 < 24 {
            return;
        }
        // everything inside the box (text glyphs) and the tail
        for cy in y0 / 8..y1 / 8 {
            for cx in x0 / 8..x1 / 8 {
                self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
            }
        }
        for (cx, cy) in cells {
            if (0..30).contains(&cx) && (0..20).contains(&cy) {
                self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
            }
        }
        let (lines, name, waiting, full) = self.talk_text(ptr, opening.is_some(), 2);
        let shift = if self.ppu.wide_map && !self.ppu.ui_boxes.is_empty() {
            let c = (((y0 + y1) / 16).min(19) * 30 + ((x0 + x1) / 16).min(29)) as usize;
            self.ppu.ui_shift[c] as i32
        } else {
            0
        };
        self.hd.talk = Some(TalkBox {
            x0,
            y0,
            x1,
            y1,
            tail: if tx1 > tx0 { Some((tx0, ty0, tx1, ty1)) } else { None },
            lines,
            name,
            waiting: waiting && !choosing,
            shift,
            choice,
            style: 0,
            full,
        });
    }

    /// The text of the current page up to the print position.
    /// Bounding box of the tutorial box frame sprites (palette 5, tiles 0x190-0x1C0).
    fn tutorial_frame(&self) -> Option<(i32, i32, i32, i32)> {
        let (mut x0, mut y0, mut x1, mut y1, mut n) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN, 0);
        for i in 0..128 {
            let o = i * 8;
            let a0 = (self.oam[o] as u32) | (self.oam[o + 1] as u32) << 8;
            let a1 = (self.oam[o + 2] as u32) | (self.oam[o + 3] as u32) << 8;
            let a2 = (self.oam[o + 4] as u32) | (self.oam[o + 5] as u32) << 8;
            if a0 & 0x300 == 0x200 || a0 >> 14 == 3 {
                continue;
            }
            let t = a2 & 0x3FF;
            if a2 >> 12 != 5 || !(0x190..=0x1C0).contains(&t) {
                continue;
            }
            let (w, h) = OBJ_SIZE[(a0 >> 14) as usize][(a1 >> 14) as usize];
            let mut y = (a0 & 0xFF) as i32;
            if y >= 160 {
                y -= 256;
            }
            let x = (a1 & 0x1FF) as i32;
            let x = if x >= 256 { x - 512 } else { x };
            if y + h <= 0 || x + w <= 0 || x >= 240 {
                continue; // parked off screen
            }
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + w);
            y1 = y1.max(y + h);
            n += 1;
        }
        (n >= 6 && x1 - x0 >= 48 && y1 - y0 >= 24).then_some((x0, y0, x1, y1))
    }

    /// Text of the bubble: (printed lines, name, waiting, whole page).
    /// With `hd.instant_text` the printed lines are the whole page.
    fn talk_text(&mut self, ptr: u32, with_name: bool, max_lines: usize) -> (Vec<String>, String, bool, Vec<String>) {
        if ptr < TALK_BUF || ptr > TALK_BUF + 0xC00 {
            return (Vec::new(), String::new(), false, Vec::new());
        }
        let waiting = self.read8(ptr - 1) == 0x03;
        // end of the page: next wait / clear / end / question
        let mut end = ptr;
        if !waiting {
            while end < TALK_BUF + 0xC00 {
                match self.read8(end) {
                    0x00 | 0x02 | 0x03 | 0x18 => break,
                    0x10 => end += 3,
                    0x80 => {
                        end += 2;
                        if self.read8(end - 1) == 0x23 {
                            while self.read8(end) >= 0x20 {
                                end += 1;
                            }
                        }
                    }
                    _ => end += 1,
                }
            }
        }
        let (l, name, w) = self.talk_text_to(ptr, ptr, with_name, max_lines);
        let full = if end > ptr { self.talk_text_to(ptr, end, false, max_lines).0 } else { l.clone() };
        let lines = if self.hd.instant_text { full.clone() } else { l };
        (lines, name, w, full)
    }

    fn talk_text_to(&mut self, ptr: u32, stop: u32, with_name: bool, max_lines: usize) -> (Vec<String>, String, bool) {
        let waiting = self.read8(ptr - 1) == 0x03;
        // 0x02 clears the box (new page); 0x03 only waits for a button
        let mut s = ptr - 1;
        while s >= TALK_BUF {
            let b = self.read8(s);
            // a 0x00 can also be a parameter of "load portrait" (0x10 xx xx)
            let param = s >= TALK_BUF + 2 && (self.read8(s - 1) == 0x10 || self.read8(s - 2) == 0x10);
            // 0x08-0x0F move the bubble to another speaker: a new bubble starts there
            let speaker = (0x08..=0x0F).contains(&b) && !param && self.read8(s - 1) != 0x80;
            if b == 0x02 || (b == 0x00 && !param) || speaker {
                break;
            }
            s -= 1;
        }
        let start = s + 1;
        let mut name = String::new();
        if with_name {
            // last name tag (0x80 0x23 ... 0x01) before the print position
            let mut a = TALK_BUF;
            while a + 1 < ptr {
                if self.read8(a) == 0x80 && self.read8(a + 1) == 0x23 {
                    name.clear();
                    let mut b = a + 2;
                    while b < ptr && self.read8(b) >= 0x20 {
                        name.push(self.read8(b) as char);
                        b += 1;
                    }
                    a = b;
                } else {
                    a += 1;
                }
            }
        }
        let mut lines: Vec<String> = vec![String::new()];
        let mut red = false;
        let mut a = start;
        while a < stop {
            let b = self.read8(a);
            match b {
                0x00 => break,
                0x01 => {
                    if red {
                        lines.last_mut().unwrap().push_str("[/color]");
                    }
                    lines.push(String::new());
                    if red {
                        lines.last_mut().unwrap().push_str("[color=#c8202a]");
                    }
                }
                // load portrait: two parameter bytes
                0x10 => a += 2,
                0x80 => {
                    let c = self.read8(a + 1);
                    a += 1;
                    if c == 0x20 {
                        // tactician's name
                        for i in 0..11u32 {
                            let ch = self.read8(0x0202_BC18 + i);
                            if ch < 0x20 {
                                break;
                            }
                            lines.last_mut().unwrap().push(ch as char);
                        }
                    } else if c == 0x21 {
                        red = !red;
                        lines.last_mut().unwrap().push_str(if red { "[color=#c8202a]" } else { "[/color]" });
                    } else if c == 0x23 {
                        while a + 1 < stop && self.read8(a + 1) >= 0x20 {
                            a += 1;
                        }
                    }
                }
                0x02..=0x1F => {}
                b'[' => lines.last_mut().unwrap().push_str("[lb]"),
                0x81..=0xFF => {}
                _ => lines.last_mut().unwrap().push(b as char),
            }
            a += 1;
        }
        if red {
            lines.last_mut().unwrap().push_str("[/color]");
        }
        // drop leading empty lines, show the last two (the box scrolls)
        while lines.len() > 1 && lines[0].is_empty() {
            lines.remove(0);
        }
        let k = lines.len().saturating_sub(max_lines);
        (lines[k..].to_vec(), name, waiting)
    }

    /// OBJ that is part of the bubble (text sprites, name plate, arrow) and gets hidden.
    pub(crate) fn talk_hides_obj(&self, ox: i32, oy: i32, w: i32, h: i32, prio: u32) -> bool {
        if self.hd.hide_hand && prio == 0 && w == 16 && h == 16 {
            return true;
        }
        for &(x0, y0, x1, y1) in &self.hd.obj_hide {
            let (cx, cy) = (ox + w / 2, oy + h / 2);
            if cx >= x0 && cx < x1 && cy >= y0 && cy < y1 {
                return true;
            }
        }
        // arrows, x2 and markers on the forecast window
        if let Some(f) = &self.hd.forecast {
            let (x0, y0, x1, y1) = f.rect;
            let (cx, cy) = (ox + w / 2, oy + h / 2);
            if prio <= 1 && cx >= x0 - 4 && cx < x1 + 8 && cy >= y0 - 4 && cy < y1 + 4 {
                return true;
            }
        }
        // the menu hand over a window that is closing
        if prio == 0 && w == 16 && h == 16 && self.hd.menus.is_empty() && self.hd.linger.len() == 600 {
            let (cx, cy) = (((ox + 8) / 8).clamp(0, 29), ((oy + 8) / 8).clamp(0, 19));
            let near = |dx: i32| {
                let x = (cx + dx).clamp(0, 29);
                self.hd.talk_mask[(cy * 30 + x) as usize]
            };
            if near(0) || near(1) || near(2) {
                return true;
            }
        }
        // the menu hand
        if prio == 0 && w == 16 && h == 16 {
            for m in &self.hd.menus {
                if ox + 8 >= m.x0 - 16 && ox < m.x1 && oy + 8 >= m.y0 && oy < m.y1 {
                    return true;
                }
            }
        }
        match &self.hd.talk {
            Some(t) if prio == 0 => {
                let (cx, cy) = (ox + w / 2, oy + h / 2);
                cx >= t.x0 - 8 && cx < t.x1 + 8 && cy >= t.y0 - 24 && cy < t.y1 + 8
            }
            _ => false,
        }
    }

    /// Window rectangles on BG1 (frame corner tiles 0x01 / 0x03 / 0x07, palette 1), in cells.
    fn bg1_windows(&mut self) -> Vec<(i32, i32, i32, i32)> {
        let cnt = self.io16(0x0A) as usize;
        let sb = ((cnt >> 8) & 31) * 0x800;
        let at = |g: &Self, x: i32, y: i32| -> u32 {
            let a = sb + ((y * 32 + x) * 2) as usize;
            (g.vram[a] as u32 | (g.vram[a + 1] as u32) << 8) & 0xF3FF
        };
        let mut out = Vec::new();
        for cy in 0..19 {
            for cx in 0..29 {
                if at(self, cx, cy) != 0x1001 {
                    continue;
                }
                let mut x1 = cx + 1;
                while x1 < 30 && at(self, x1, cy) != 0x1003 {
                    x1 += 1;
                }
                let mut y1 = cy + 1;
                while y1 < 20 && at(self, cx, y1) != 0x1007 {
                    y1 += 1;
                }
                if x1 < 30 && y1 < 20 {
                    out.push((cx, cy, x1 + 1, y1 + 1));
                }
            }
        }
        out
    }

    /// Like `bg1_windows`, but also finds windows whose top-left corner is covered
    /// by another window (found from the bottom-right corner).
    fn bg1_windows_all(&mut self) -> Vec<(i32, i32, i32, i32)> {
        let mut out = self.bg1_windows();
        let cnt = self.io16(0x0A) as usize;
        let sb = ((cnt >> 8) & 31) * 0x800;
        let at = |g: &Self, x: i32, y: i32| -> u32 {
            let a = sb + ((y * 32 + x) * 2) as usize;
            (g.vram[a] as u32 | (g.vram[a + 1] as u32) << 8) & 0xF3FF
        };
        for cy in 1..20 {
            for cx in 1..30 {
                if at(self, cx, cy) != 0x1009 {
                    continue;
                }
                let mut x0 = cx - 1;
                while x0 >= 0 && at(self, x0, cy) == 0x1008 {
                    x0 -= 1;
                }
                let mut y0 = cy - 1;
                while y0 >= 0 && matches!(at(self, cx, y0), 0x1006 | 0x100F) {
                    y0 -= 1;
                }
                // a window still opening or closing has no top / left edge yet
                if x0 < 0 || at(self, x0, cy) != 0x1007 {
                    x0 += 1;
                }
                if y0 < 0 || !matches!(at(self, cx, y0), 0x1003 | 0x100C) {
                    y0 += 1;
                }
                if cx - x0 < 2 || cy - y0 < 0 {
                    continue;
                }
                let r = (x0, y0, cx + 1, cy + 1);
                if !out.iter().any(|o| o.2 == r.2 && o.3 == r.3) {
                    out.push(r);
                }
            }
        }
        out
    }

    /// Lines of text the game printed in a BG0 cell region, top to bottom.
    pub fn region_lines(&mut self, cx0: i32, cy0: i32, cx1: i32, cy1: i32) -> Vec<(i32, i32, String)> {
        let (mut t, _) = self.generic_texts(cx0, cy0, cx1, cy1);
        t.sort_by_key(|e| (e.1, e.0));
        t.into_iter().map(|e| (e.0, e.1, e.3)).collect()
    }

    /// Strings and leftover cells of BG0 inside a window (cells).
    fn generic_texts(&mut self, cx0: i32, cy0: i32, cx1: i32, cy1: i32) -> (Vec<(i32, i32, u8, String)>, Vec<(i32, i32)>) {
        self.generic_texts_bg(0, cx0, cy0, cx1, cy1)
    }

    fn generic_texts_bg(&mut self, bg: usize, cx0: i32, cy0: i32, cx1: i32, cy1: i32) -> (Vec<(i32, i32, u8, String)>, Vec<(i32, i32)>) {
        let cnt = self.io16(0x08 + 2 * bg) as usize;
        let sb = ((cnt >> 8) & 31) * 0x800;
        let (hx, vy) = if bg == 0 { (0, 0) } else { ((self.io16(0x10 + 4 * bg) as i32 & 0x1FF) / 8, (self.io16(0x12 + 4 * bg) as i32 & 0x1FF) / 8) };
        let mut used = vec![false; 600];
        let mut texts = Vec::new();
        let mut rest = Vec::new();
        for cy in cy0..cy1 {
            for cx in cx0..cx1 {
                let a = sb + ((((cy + vy) & 31) * 32 + ((cx + hx) & 31)) * 2) as usize;
                let e = (self.vram[a] as u32 | (self.vram[a + 1] as u32) << 8) & 0x3FF;
                if e == 0 || used[(cy * 30 + cx) as usize] {
                    continue;
                }
                let hit = self.hd.texts.iter().rev().find(|t| e >= t.0 && e < t.0 + t.1).cloned();

                match hit {
                    Some((lo, n, xo, col, text)) if (e - lo) % 2 == 0 => {
                        let c0 = cx - ((e - lo) / 2) as i32;
                        // mark the whole string (top and bottom row)
                        for k in 0..(n / 2) as i32 {
                            for dy in 0..2 {
                                let (x, y) = (c0 + k, cy + dy);
                                if (0..30).contains(&x) && (0..20).contains(&y) {
                                    used[(y * 30 + x) as usize] = true;
                                }
                            }
                        }
                        if !texts.iter().any(|t: &(i32, i32, u8, String)| t.0 == c0 * 8 + xo as i32 && t.1 == cy * 8) {
                            texts.push((c0 * 8 + xo as i32, cy * 8, col, text));
                        }
                    }
                    Some(_) => {}
                    None => {
                        if let Some(&d) = self.hd.char_tiles.get(&e) {
                            // UI digit: join with a digit right before it on this row
                            let x = cx * 8;
                            match texts.iter_mut().find(|t| t.1 == cy * 8 && t.2 == 255 && t.0 + 8 * t.3.len() as i32 == x) {
                                Some(t) => t.3.push((b'0' + d) as char),
                                None => texts.push((x, cy * 8, 255, ((b'0' + d) as char).to_string())),
                            }
                            for dy in 0..2 {
                                if cy + dy < 20 {
                                    used[((cy + dy) * 30 + cx) as usize] = true;
                                }
                            }
                        } else {
                            rest.push((cx * 8, cy * 8));
                        }
                    }
                }
            }
        }
        (texts, rest)
    }

    /// Battle scene pop-ups on BG1 (frame tiles 0x100-0x104, palette 1): hidden and
    /// handed to the frontend as generic windows with their text.
    fn battle_popup_detect(&mut self) {
        let cnt = self.io16(0x0A) as usize;
        if self.io16(0) & 0x200 == 0 {
            return;
        }
        let sb = ((cnt >> 8) & 31) * 0x800;
        let hx = (self.io16(0x14) as i32 & 0x1FF) / 8;
        let vy = (self.io16(0x16) as i32 & 0x1FF) / 8;
        let at = |g: &Self, x: i32, y: i32| -> u32 {
            let a = sb + ((((y + vy) & 31) * 32 + ((x + hx) & 31)) * 2) as usize;
            g.vram[a] as u32 | (g.vram[a + 1] as u32) << 8
        };
        let mut mask = vec![false; 600];
        let (mut x0, mut y0, mut x1, mut y1) = (99, 99, -1, -1);
        for cy in 0..20 {
            for cx in 0..30 {
                let e = at(self, cx, cy);
                if e >> 12 == 1 && (0x100..0x140).contains(&(e & 0x3FF)) {
                    mask[(cy * 30 + cx) as usize] = true;
                    x0 = x0.min(cx);
                    y0 = y0.min(cy);
                    x1 = x1.max(cx + 1);
                    y1 = y1.max(cy + 1);
                }
            }
        }
        if x1 <= x0 {
            return;
        }
        self.hd.bg1_mask = mask;
        if y0 >= 13 {
            return; // the EXP gauge: drawn by the frontend's battle HUD
        }
        let (texts, _) = self.generic_texts_bg(1, x0, y0, x1, y1);
        self.hd.generic.push((x0 * 8, y0 * 8, x1 * 8, y1 * 8, 0));
        self.hd.generic_tab.push(0);
        self.hd.generic_text.push((texts, Vec::new()));
        self.hd.hiding = true;
    }

    fn hd_generic_detect(&mut self) {
        self.hd.generic.clear();
        self.hd.generic_tab.clear();
        self.hd.generic_text.clear();
        if self.hd.mask_bg0_only && self.hd.menu_native {
            self.battle_popup_detect();
            return;
        }
        if !self.hd.menu_native || self.hd.talk_mask.len() != 600 || self.io16(0) & 0x200 == 0 {
            return;
        }
        for (cx0, cy0, cx1, cy1) in self.bg1_windows_all() {
            let (cx0, mut cy0, cx1, cy1) = (cx0.max(0), cy0.max(0), cx1.min(30), cy1.min(20));
            // a name plate narrower than the window sits on its top-left: take it in
            let sbn = ((self.io16(0x0A) as usize >> 8) & 31) * 0x800;
            let entn = |g: &Self, x: i32, y: i32| -> u32 {
                let a = sbn + ((y * 32 + x) * 2) as usize;
                (g.vram[a] as u32 | (g.vram[a + 1] as u32) << 8) & 0xF3FF
            };
            let mut plate = (0, 0);
            if entn(self, cx0, cy0) == 0x100D {
                let mut ty = cy0;
                while ty > 0 && matches!(entn(self, cx0, ty - 1), 0x100D | 0x100A) {
                    ty -= 1;
                    if entn(self, cx0, ty) == 0x100A {
                        break;
                    }
                }
                if entn(self, cx0, ty) == 0x100A {
                    let mut px = cx0 + 1;
                    while px < cx1 && entn(self, px, ty) != 0x100C {
                        px += 1;
                    }
                    plate = ((cy0 + 1 - ty) * 8, (px + 1 - cx0) * 8);
                    cy0 = ty;
                }
            }
            let mut open = 0;
            let mut all = 0;
            for cy in cy0..cy1 {
                for cx in cx0..cx1 {
                    all += 1;
                    if !self.hd.talk_mask[(cy * 30 + cx) as usize] {
                        open += 1;
                    }
                }
            }
            if all == 0 || open == 0 {
                continue;
            }
            if open * 2 < all {
                // mostly covered by a native window already (e.g. a menu that is still
                // opening or closing): just hide the rest of the game's frame
                for cy in cy0..cy1 {
                    for cx in cx0..cx1 {
                        self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                    }
                }
                self.hd.hiding = true;
                continue;
            }
            for cy in cy0..cy1 {
                for cx in cx0..cx1 {
                    self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                }
            }
            let (x0, y0, x1, y1) = (cx0 * 8, cy0 * 8, cx1 * 8, cy1 * 8);
            let shift = if self.ppu.wide_map && !self.ppu.ui_boxes.is_empty() {
                let c = (((y0 + y1) / 16).min(19) * 30 + ((x0 + x1) / 16).min(29)) as usize;
                self.ppu.ui_shift[c] as i32
            } else {
                0
            };
            // name tab (corners 0x0A / 0x0C, sides 0x0D / 0x0F) above the window body
            let sb = ((self.io16(0x0A) as usize >> 8) & 31) * 0x800;
            let ent = |g: &Self, x: i32, y: i32| -> u32 {
                let a = sb + ((y * 32 + x) * 2) as usize;
                (g.vram[a] as u32 | (g.vram[a + 1] as u32) << 8) & 0xF3FF
            };
            let mut tab = 0;
            if ent(self, cx1 - 1, cy0) == 0x100C {
                let mut y = cy0 + 1;
                while y < cy1 && ent(self, cx1 - 1, y) == 0x100F {
                    y += 1;
                }
                tab = (y - cy0) * 8;
            }
            if plate.0 > 0 {
                tab = plate.0 | plate.1 << 16;
            }
            self.hd.generic.push((x0, y0, x1, y1, shift));
            self.hd.generic_tab.push(tab);
            let gt = self.generic_texts(cx0, cy0, cx1, cy1);
            self.hd.generic_text.push(gt);
            self.hd.hiding = true;
        }
    }

    fn hd_menu_detect(&mut self) {
        self.hd.menus.clear();
        self.hd.help = None;
        if !self.hd.menu_native {
            return;
        }
        let mut list = Vec::new();
        for i in 0..64u32 {
            let p = 0x0202_4E28 + i * 0x6C;
            if self.read32(p) == 0x08B9_A8A0 {
                let n = self.read8(p + 0x60) as u32;
                if (1..=11).contains(&n) {
                    list.push(p);
                }
            }
        }
        // menu of the "Check Map" view during the preparations (View Map / Formation /
        // Options / Save): the prep menu proc, laid out in a BG1 window
        if let Some(m) = self.proc_find(0x08CC_416C) {
            let parent = self.read32(m + 0x14);
            if self.read32(parent) != 0x08CC_3BDC {
                let n = (self.read8(m + 0x2B) as u32).min(8);
                let win = self.bg1_windows().into_iter().find(|&(_, y0, _, y1)| y1 - y0 >= 2 * n as i32);
                if let Some((cx0, cy0, cx1, cy1)) = win {
                    let mut items = Vec::new();
                    for i in 0..n {
                        let e = self.read32(m + 0x38 + 4 * i);
                        if e >> 24 != 2 {
                            continue;
                        }
                        let tid = self.read32(e + 0x34);
                        let label = crate::text::decode(&self.rom, tid).trim().to_string();
                        let enabled = self.read8(e + 0x38) == 0;
                        items.push(MenuEntry { x: cx0 * 8 + 16, y: cy0 * 8 + 8 + 16 * i as i32, label, enabled, icon: false, uses: -1 });
                    }
                    for cy in cy0..cy1 {
                        for cx in cx0..cx1 {
                            self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                        }
                    }
                    let cur = self.read8(m + 0x2A) as i32;
                    self.hd.menus.push(MenuBox { x0: cx0 * 8, y0: cy0 * 8, x1: cx1 * 8, y1: cy1 * 8, items, cur, active: true, shift: 0 });
                    self.hd.hiding = true;
                }
            }
        }
        // a submenu is started from its parent menu: draw parents first, and only a
        // menu without an open submenu takes input (proc slots are reused, so the
        // slot order says nothing)
        let depth = |g: &mut Self, p: u32| {
            let mut d = 0;
            let mut q = g.read32(p + 0x14);
            for _ in 0..8 {
                if q >> 24 != 2 {
                    break;
                }
                if g.read32(q) == 0x08B9_A8A0 {
                    d += 1;
                }
                q = g.read32(q + 0x14);
            }
            d
        };
        let mut keyed: Vec<(i32, u32)> = list.iter().map(|&p| (depth(self, p), p)).collect();
        keyed.sort_by_key(|&(d, _)| d);
        let list: Vec<u32> = keyed.into_iter().map(|(_, p)| p).collect();
        let n_menus = list.len();
        for (k, p) in list.into_iter().enumerate() {
            let rx = self.read8(p + 0x2C) as i8 as i32;
            let ry = self.read8(p + 0x2D) as i8 as i32;
            let rw = self.read8(p + 0x2E) as i32;
            let rh = self.read8(p + 0x2F) as i32;
            if rw < 2 || rh < 2 || rx < 0 || ry < 0 || rx + rw > 30 || ry + rh > 20 {
                continue;
            }
            let n = self.read8(p + 0x60) as u32;
            let mut items = Vec::new();
            let unit = self.read32(0x0300_4690);
            for i in 0..n {
                let it = self.read32(p + 0x34 + 4 * i);
                if it >> 24 != 2 {
                    continue;
                }
                let x = self.read16(it + 0x2A) as i16 as i32 * 8;
                let y = self.read16(it + 0x2C) as i16 as i32 * 8;
                let def = self.read32(it + 0x30);
                let msg = self.read16(def + 4) as u32;
                let custom = self.read32(def + 0x10) != 0;
                let avail = self.read8(it + 0x3D);
                let mut e = MenuEntry { x, y, label: String::new(), enabled: avail != 2, icon: false, uses: -1 };
                if msg != 0 {
                    e.label = crate::text::decode(&self.rom, msg);
                } else if custom && unit >> 24 == 2 {
                    // inventory entry of the active unit
                    let slot = self.read8(it + 0x3C) as u32;
                    let item = self.read16(unit + 0x1E + 2 * slot.min(4)) as u32;
                    let id = item & 0xFF;
                    if id != 0 {
                        let name = self.read16(0x08BE_222C + id * 0x24) as u32;
                        e.label = crate::text::decode(&self.rom, name);
                        e.icon = true;
                        e.uses = (item >> 8) as i32;
                        e.x += 16;
                        e.enabled = true;
                    }
                }
                items.push(e);
            }
            if items.is_empty() {
                continue;
            }
            for cy in ry..ry + rh {
                for cx in rx..rx + rw {
                    self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                }
            }
            let (x0, y0, x1, y1) = (rx * 8, ry * 8, (rx + rw) * 8, (ry + rh) * 8);
            let shift = if self.ppu.wide_map && !self.ppu.ui_boxes.is_empty() {
                let c = (((y0 + y1) / 16).min(19) * 30 + ((x0 + x1) / 16).min(29)) as usize;
                self.ppu.ui_shift[c] as i32
            } else {
                0
            };
            let cur = self.read8(p + 0x61) as i32;
            self.hd.menus.push(MenuBox { x0, y0, x1, y1, items, cur, active: k + 1 == n_menus, shift });
        }
        // box next to an item list: description of the selected item, or the battle stats of the weapon
        let inv = self.hd.menus.iter().find(|m| m.items.iter().any(|e| e.icon)).cloned();
        if let Some(m) = inv {
            let unit = self.read32(0x0300_4690);
            let cur = m.cur.clamp(0, 4) as u32;
            let id = self.read16(unit + 0x1E + 2 * cur) as u32 & 0xFF;
            let desc = if id != 0 { self.read16(0x08BE_222C + id * 0x24 + 4) as u32 } else { 0 };
            // only the item list itself rules a window out (sub-menus like Use/Discard
            // or Yes/No may lie over the help box)
            let menu_cells: Vec<(i32, i32, i32, i32)> = self.hd.menus.iter().filter(|m| m.items.iter().any(|e| e.icon)).map(|m| (m.x0 / 8, m.y0 / 8, m.x1 / 8, m.y1 / 8)).collect();
            for (cx0, cy0, cx1, cy1) in self.bg1_windows_all() {
                let overlaps = menu_cells.iter().any(|&(a, b, c, d)| cx0 < c && a < cx1 && cy0 < d && b < cy1);
                if overlaps {
                    continue;
                }
                let mut hb = HelpBox { rect: (cx0 * 8, cy0 * 8, cx1 * 8, cy1 * 8), ..Default::default() };
                if desc != 0 {
                    hb.text = crate::text::decode(&self.rom, desc);
                } else {
                    // weapon: stats of the battle actor (gBattleActor 0x0203A3F0)
                    let b = 0x0203_A3F0u32;
                    let rd = |g: &mut Self, o: u32| g.read16(b + o) as i16 as i32;
                    hb.stats = Some([rd(self, 0x5A), rd(self, 0x60), rd(self, 0x66), rd(self, 0x62)]);
                    // affinity icon: first BG0 cell in the box with a non-text palette
                    let cnt = self.io16(0x08) as usize;
                    let sb = ((cnt >> 8) & 31) * 0x800;
                    'find: for cy in cy0..cy1 {
                        for cx in cx0..cx1 {
                            let a = sb + ((cy * 32 + cx) * 2) as usize;
                            let ent = self.vram[a] as u32 | (self.vram[a + 1] as u32) << 8;
                            if ent & 0x3FF != 0 && ent >> 12 >= 2 {
                                hb.icon = Some((cx * 8, cy * 8));
                                break 'find;
                            }
                        }
                    }
                }
                for cy in cy0..cy1 {
                    for cx in cx0..cx1 {
                        self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                    }
                }
                let (x0, y0, x1, y1) = hb.rect;
                hb.shift = if self.ppu.wide_map && !self.ppu.ui_boxes.is_empty() {
                    let c = (((y0 + y1) / 16).min(19) * 30 + ((x0 + x1) / 16).min(29)) as usize;
                    self.ppu.ui_shift[c] as i32
                } else {
                    0
                };
                self.hd.help = Some(hb);
                break;
            }
        }
    }

    /// BG0/BG1 entry at a screen cell (0 = empty).
    fn bg_cell(&self, bg: usize, cx: i32, cy: i32) -> u32 {
        let cnt = self.io16(8 + bg * 2) as usize;
        let sb = ((cnt >> 8) & 31) * 0x800;
        let hofs = (self.io16(0x10 + bg * 4) & 0xFF) as i32;
        let vofs = (self.io16(0x12 + bg * 4) & 0xFF) as i32;
        let mx = ((cx * 8 + hofs) / 8).rem_euclid(32);
        let my = ((cy * 8 + vofs) / 8).rem_euclid(32);
        let a = sb + ((my * 32 + mx) * 2) as usize;
        (self.vram[a] as u32 | (self.vram[a + 1] as u32) << 8)
    }

    fn hd_linger(&mut self) {
        if self.hd.linger.len() != 600 {
            self.hd.linger = vec![false; 600];
            self.hd.linger_ent = vec![(0, 0); 600];
        }
        let active = self.hd.talk.is_some() || !self.hd.menus.is_empty() || self.hd.help.is_some() || self.hd.forecast.is_some();
        if active {
            for i in 0..600 {
                let on = self.hd.talk_mask[i];
                self.hd.linger[i] = on;
                if on {
                    let (cx, cy) = ((i % 30) as i32, (i / 30) as i32);
                    self.hd.linger_ent[i] = (self.bg_cell(0, cx, cy), self.bg_cell(1, cx, cy));
                }
            }
            self.hd.linger_t = 0;
            self.hd.hiding = true;
            return;
        }
        // the window is closed: keep hiding its old tiles as long as they are still
        // there unchanged (the game erases them later, e.g. after an animation)
        self.hd.linger_t += 1;
        let mut any = false;
        for i in 0..600 {
            if !self.hd.linger[i] {
                continue;
            }
            let (cx, cy) = ((i % 30) as i32, (i / 30) as i32);
            let (e0, e1) = (self.bg_cell(0, cx, cy), self.bg_cell(1, cx, cy));
            let (o0, o1) = self.hd.linger_ent[i];
            let same = (e0 != 0 && e0 == o0) || (e1 != 0 && e1 == o1);
            if same && self.hd.linger_t < 600 {
                self.hd.talk_mask[i] = true;
                any = true;
            } else {
                self.hd.linger[i] = false;
            }
        }
        self.hd.hiding = any;
    }

    fn fc_side(&mut self, b: u32, other: u32) -> FcSide {
        let rd = |g: &mut Self, o: u32| g.read16(b + o) as i16 as i32;
        let ch = self.read32(b);
        let nid = self.read16(ch) as u32;
        let name = crate::text::decode(&self.rom, nid);
        let id = self.read8(b + 0x0B);
        // current HP from the real unit (the battle copy is already simulated)
        let mut hp = self.read8(b + 0x12) as i32;
        for i in 0..200u32 {
            let u = 0x0202_BD50 + i * 0x48;
            if self.read32(u) == ch && self.read8(u + 0x0B) == id {
                hp = self.read8(u + 0x13) as i32;
                break;
            }
        }
        let item = self.read16(b + 0x48) as u32 & 0xFF;
        let weapon = if item != 0 {
            let wid = self.read16(0x08BE_222C + item * 0x24) as u32;
            crate::text::decode(&self.rom, wid)
        } else {
            String::new()
        };
        let atk = rd(self, 0x5A);
        let odef = self.read16(other + 0x5C) as i16 as i32;
        let can = item != 0;
        let as_ = rd(self, 0x5E);
        let oas = self.read16(other + 0x5E) as i16 as i32;
        let tri = self.read8(b + 0x53) as i8;
        FcSide {
            name,
            weapon,
            hp,
            mt: if can { (atk - odef).max(0) } else { -1 },
            hit: if can { rd(self, 0x64).clamp(0, 100) } else { -1 },
            crit: if can { rd(self, 0x6A).clamp(0, 100) } else { -1 },
            double: can && as_ - oas >= 4,
            tri: (tri > 0) as i32 - (tri < 0) as i32,
            icon: None,
        }
    }

    fn hd_forecast_detect(&mut self) {
        self.hd.forecast = None;
        if !self.hd.menu_native || self.proc_find(0x08B9_6D5C).is_none() {
            return;
        }
        // window on BG1: top-left tile 0x201, bottom-right 0x20C
        let mut rect: Option<(i32, i32, i32, i32)> = None;
        'f: for cy in 0..20 {
            for cx in 0..30 {
                if self.bg_cell(1, cx, cy) & 0x3FF != 0x201 {
                    continue;
                }
                for y1 in cy + 4..20 {
                    for x1 in cx + 4..30 {
                        if self.bg_cell(1, x1, y1) & 0x3FF == 0x20C {
                            rect = Some((cx, cy, x1 + 1, y1 + 1));
                            break 'f;
                        }
                    }
                }
            }
        }
        // sliding in or out at the screen edge: only one corner is on screen; the
        // window is always 10 x 16 cells
        if rect.is_none() {
            'g: for cy in 0..20 {
                for cx in 0..30 {
                    let t = self.bg_cell(1, cx, cy) & 0x3FF;
                    if t == 0x201 {
                        rect = Some((cx, cy, cx + 10, cy + 16));
                        break 'g;
                    }
                    if t == 0x20C {
                        rect = Some((cx + 1 - 10, cy + 1 - 16, cx + 1, cy + 1));
                        break 'g;
                    }
                }
            }
        }
        let Some((x0, y0, x1, y1)) = rect else { return };
        let (pa, ea) = (0x0203_A3F0, 0x0203_A470);
        let mut player = self.fc_side(pa, ea);
        let mut enemy = self.fc_side(ea, pa);
        // weapon icons: BG0 cells with an icon palette (top = player, lower = enemy)
        let mut icons = Vec::new();
        for cy in y0.max(0)..y1.min(20) {
            for cx in x0.max(0)..x1.min(30) {
                let e = self.bg_cell(0, cx, cy);
                if e & 0x3FF != 0 && e >> 12 >= 3 && !icons.iter().any(|&(ix, iy): &(i32, i32)| (cx * 8 - ix).abs() < 16 && (cy * 8 - iy).abs() < 16) {
                    icons.push((cx * 8, cy * 8));
                }
            }
        }
        player.icon = icons.first().copied();
        enemy.icon = icons.get(1).copied();
        // (one row more on each side: the window slides in by a few pixels)
        for cy in (y0 - 1).max(0)..(y1 + 1).min(20) {
            for cx in x0.max(0)..x1.min(30) {
                self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
            }
        }
        let (px0, py0, px1, py1) = (x0 * 8, y0 * 8, x1 * 8, y1 * 8);
        let shift = if self.ppu.wide_map && !self.ppu.ui_boxes.is_empty() {
            let c = (((py0 + py1) / 16).min(19) * 30 + ((px0 + px1) / 16).min(29)) as usize;
            self.ppu.ui_shift[c] as i32
        } else {
            0
        };
        self.hd.forecast = Some(Forecast { rect: (px0, py0, px1, py1), shift, player, enemy });
    }

    pub fn hd_bg_cell(&self, bg: usize, cx: i32, cy: i32) -> u16 {
        self.bg_cell(bg, cx, cy) as u16
    }

    /// Bounding box (cells) of BG1 tiles in `lo..=hi`.
    fn bg1_box(&self, lo: u32, hi: u32) -> Option<(i32, i32, i32, i32)> {
        let (mut x0, mut y0, mut x1, mut y1, mut n) = (99, 99, -1, -1, 0);
        for cy in 0..20 {
            for cx in 0..30 {
                let t = self.bg_cell(1, cx, cy) & 0x3FF;
                if (lo..=hi).contains(&t) {
                    x0 = x0.min(cx);
                    y0 = y0.min(cy);
                    x1 = x1.max(cx + 1);
                    y1 = y1.max(cy + 1);
                    n += 1;
                }
            }
        }
        (n >= 2).then_some((x0, y0, x1, y1))
    }

    fn ui_shift_at(&self, px0: i32, py0: i32, px1: i32, py1: i32) -> i32 {
        if self.ppu.wide_map && !self.ppu.ui_boxes.is_empty() {
            let c = (((py0 + py1) / 16).clamp(0, 19) * 30 + ((px0 + px1) / 16).clamp(0, 29)) as usize;
            self.ppu.ui_shift[c] as i32
        } else {
            0
        }
    }

    fn mask_cells(&mut self, (x0, y0, x1, y1): (i32, i32, i32, i32)) {
        for cy in y0.max(0)..y1.min(20) {
            for cx in x0.max(0)..x1.min(30) {
                self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
            }
        }
    }

    /// Phase change ("Player Phase") and chapter title banners: hidden in the UI
    /// layer and redrawn natively.
    fn hd_banner_detect(&mut self) {
        self.hd.obj_hide.clear();
        self.hd.hide_hand = false;
        if !self.hd.menu_native {
            self.hd.banner = None;
            self.hd.trade = None;
            self.hd.prepmap = 0;
            return;
        }
        self.hd.rhelp = None;
        if let Some(p) = self.proc_find(0x08CC_2014) {
            let id = self.read16(p + 0x4C) as u32;
            if id != 0 && id < 0x1400 {
                let x = self.read16(p + 0x30) as i16 as i32;
                let y = self.read16(p + 0x32) as i16 as i32;
                let w = self.read16(p + 0x44) as i16 as i32;
                let h = self.read16(p + 0x46) as i16 as i32;
                let text = crate::text::decode(&self.rom, id);
                // the bubble is made of objects: hide them, the frontend draws its own
                self.hd.obj_hide.push((x - 8, y - 16, x + w + 24, y + h + 24));
                self.hd.rhelp = Some((x, y, w, h, text));
                self.hd.hiding = true;
            }
        }
        self.hd.prepmap = 0;
        if self.proc_find(0x08B9_6460).is_some() && self.proc_find(0x08CC_3BDC).is_none() {
            // the button hints of the "Check Map" view are objects: hidden, the
            // frontend shows its own buttons
            // the process stays after the preparations: only its own views count
            if self.proc_find(0x08CC_416C).is_some() {
                self.hd.prepmap = 1;
                self.hd.obj_hide.push((-64, 132, 304, 224));
                self.hd.hide_hand = true;
                self.hd.hiding = true;
            } else if self.proc_find(0x08CC_2D98).is_some() {
                self.hd.prepmap = 2;
                self.hd.obj_hide.push((140, -16, 304, 16));
                self.hd.hiding = true;
            }
        }
        // battle scene: the name plates (BG0 rows 0-3) and the HUD (rows 13-19) are
        // drawn natively by the frontend; hide the game's own
        self.hd.mask_bg0_only = false;
        if self.proc_find(0x08B9_A99C).is_some() && self.hd.talk_mask.len() == 600 {
            self.hd.mask_bg0_only = true;
            let dispcnt = self.io16(0);
            if dispcnt & 0x100 != 0 {
                // in the battle scene BG0 only holds the interface
                for v in self.hd.talk_mask.iter_mut() {
                    *v = true;
                }
                // the HUD's gauges and numbers are objects: the frontend draws its own
                self.hd.obj_hide.push((-64, 116, 304, 224));
                self.hd.hiding = true;
            }
        }
        self.hd.bg1_mask.clear();
        self.hd.mapexp = None;
        if self.proc_find(0x08C9_D820).is_some() && self.hd.talk_mask.len() == 600 {
            let (mut x0, mut y0, mut x1, mut y1) = (99, 99, -1, -1);
            for cy in 0..20 {
                for cx in 0..30 {
                    let e = self.bg_cell(0, cx, cy);
                    if e >> 12 == 5 && (0x200..0x230).contains(&(e & 0x3FF)) {
                        self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                        x0 = x0.min(cx);
                        y0 = y0.min(cy);
                        x1 = x1.max(cx + 1);
                        y1 = y1.max(cy + 1);
                    }
                }
            }
            if x1 > x0 {
                self.hd.mapexp = Some((x0 * 8, y0 * 8, x1 * 8, y1 * 8));
                self.hd.hiding = true;
            }
        }
        self.hd.minimap = false;
        if self.proc_find(0x08CE_3B6C).is_some() {
            if self.hd.talk_mask.len() == 600 {
                self.hd.talk_mask.iter_mut().for_each(|v| *v = true);
            }
            self.hd.obj_hide.push((-64, -64, 304, 224));
            self.hd.minimap = true;
            self.hd.hiding = true;
        }
        self.hd_trade_detect();
        self.hd_shop_detect();
        self.hd_worldmap_detect();
        let phase = [0x08B9_372Cu32, 0x08B9_374C, 0x08B9_376C, 0x08B9_378C, 0x08B9_37AC].iter().any(|&p| self.proc_find(p).is_some());
        let title = self.proc_find(0x08B9_391C).is_some() || self.proc_find(0x08B9_3934).is_some();
        let kind = if title { 1 } else if phase { 0 } else { -1 };
        if kind < 0 {
            self.hd.banner = None;
            return;
        }
        let text = if kind == 1 {
            let ch = self.read8(0x0202_BC06) as u32;
            crate::text::decode(&self.rom, 0x58C + ch).trim().to_string()
        } else {
            match self.read8(0x0202_BC07) {
                0x00 => "Player Phase",
                0x40 => "Other Phase",
                _ => "Enemy Phase",
            }
            .to_string()
        };
        let n = match &self.hd.banner {
            Some((k, t, n)) if *k == kind && *t == text => n + 1,
            _ => 0,
        };
        self.hd.banner = Some((kind, text, n));
        // hide the game's banner: all UI tiles and objects
        if self.hd.talk_mask.len() == 600 {
            self.hd.talk_mask.iter_mut().for_each(|v| *v = true);
        }
        self.hd.obj_hide.push((-64, -64, 304, 224));
        self.hd.hiding = true;
    }

    /// Trade screen (process 0x08B942F8): item windows and name plates are redrawn natively.
    fn hd_trade_detect(&mut self) {
        self.hd.trade = None;
        if self.proc_find(0x08B9_42F8).is_none() {
            return;
        }
        // the two item windows on BG1 (corner tiles 0x001 / 0x003)
        let mut wins = Vec::new();
        for cy in 0..20 {
            for cx in 0..30 {
                if self.bg_cell(1, cx, cy) & 0x3FF == 0x001 {
                    let mut x1 = cx + 1;
                    while x1 < 30 && self.bg_cell(1, x1, cy) & 0x3FF != 0x003 {
                        x1 += 1;
                    }
                    let mut y1 = cy + 1;
                    while y1 < 20 && self.bg_cell(1, cx, y1) & 0x3FF != 0x007 {
                        y1 += 1;
                    }
                    wins.push((cx, cy, x1 + 1, y1 + 1));
                }
            }
        }
        if wins.len() < 2 {
            return;
        }
        wins.sort();
        let (l, r) = (wins[0], wins[wins.len() - 1]);
        for b in [l, r] {
            self.mask_cells(b);
        }
        // name plates: BG0 text in the top rows and their objects
        self.mask_cells((0, 0, 30, 2));
        self.hd.obj_hide.push((-64, -16, 304, 20));
        self.hd.hide_hand = true;
        self.hd.hiding = true;
        let mut hands = Vec::new();
        for i in 0..128usize {
            let a0 = self.oam[i * 8] as u16 | (self.oam[i * 8 + 1] as u16) << 8;
            let a1 = self.oam[i * 8 + 2] as u16 | (self.oam[i * 8 + 3] as u16) << 8;
            let a2 = self.oam[i * 8 + 4] as u16 | (self.oam[i * 8 + 5] as u16) << 8;
            if (a0 >> 8) & 3 == 2 || a0 >> 14 != 0 || a1 >> 14 != 1 || (a2 >> 10) & 3 != 0 || a2 & 0x3FF != 0 {
                continue;
            }
            let y = (a0 & 255) as i32;
            if y < 160 {
                hands.push(((a1 & 511) as i32, y));
            }
        }
        let px = |b: (i32, i32, i32, i32)| (b.0 * 8, b.1 * 8, b.2 * 8, b.3 * 8);
        let sh = self.ui_shift_at(l.0 * 8, l.1 * 8, r.2 * 8, r.3 * 8);
        self.hd.trade = Some((px(l), px(r), sh, hands));
    }

    /// Shop (process 0x08CE6FC0): the whole screen is redrawn natively.
    fn hd_shop_detect(&mut self) {
        self.hd.shop = None;
        if self.proc_find(0x08CE_6FC0).is_none() && self.proc_find(0x08CE_7280).is_none() {
            return;
        }
        // portrait: BG0 cells with palette 2 in the left columns
        let (mut x0, mut y0, mut x1, mut y1) = (99, 99, -1, -1);
        for cy in 0..20 {
            for cx in 0..8 {
                let e = self.bg_cell(0, cx, cy);
                if e >> 12 == 2 && e & 0x3FF != 0 {
                    x0 = x0.min(cx);
                    y0 = y0.min(cy);
                    x1 = x1.max(cx + 1);
                    y1 = y1.max(cy + 1);
                }
            }
        }
        let arena = self.proc_find(0x08CE_6FC0).is_none();
        if self.hd.talk_mask.len() == 600 {
            if arena {
                // the arena's own box and gold at the top; other windows (the opponent)
                // stay for the generic windows
                for cy in 0..10 {
                    for cx in 0..30 {
                        if cy < 8 || cx >= 23 {
                            self.hd.talk_mask[cy * 30 + cx] = true;
                        }
                    }
                }
            } else {
                self.hd.talk_mask.iter_mut().for_each(|v| *v = true);
            }
        }
        self.hd.obj_hide.push(if arena { (-64, -64, 304, 72) } else { (-64, -64, 304, 224) });
        self.hd.hiding = true;
        self.hd.shop = Some(if x1 > x0 { (x0 * 8, y0 * 8, x1 * 8, y1 * 8) } else { (0, 0, 0, 0) });
    }

    /// World map narration (process 0x08CE7630): the text box made of objects at the bottom.
    fn hd_worldmap_detect(&mut self) {
        self.hd.worldmap = None;
        if self.proc_find(0x08CE_7630).is_none() {
            return;
        }
        // the box frame objects (palette 3 at y 112..160) show while text is up
        let mut boxed = false;
        for i in 0..128usize {
            let a0 = self.oam[i * 8] as u16 | (self.oam[i * 8 + 1] as u16) << 8;
            let a2 = self.oam[i * 8 + 4] as u16 | (self.oam[i * 8 + 5] as u16) << 8;
            if (a0 >> 8) & 3 != 2 && a2 >> 12 == 3 && (a0 & 255) == 112 {
                boxed = true;
            }
        }
        if !boxed {
            return;
        }
        let lines = self.hd_current_text();
        self.hd.obj_hide.push((-64, 113, 304, 224));
        self.hd.hiding = true;
        self.hd.worldmap = Some(lines);
    }

    pub fn hd_char_hook(&mut self, t: u32, p: u32) {
        let c = self.read8(p);
        if c < 0x20 || c >= 0x80 {
            return;
        }
        let chr = self.read16(t) as u32;
        let x = self.read8(t + 2) as u32;
        let col = self.read8(t + 3);
        let tw = self.read8(t + 4) as u32;
        let font = self.read32(0x0202_8D70);
        let base = (self.read16(font + 0x10) & 0x3FF) as u32;
        let glyphs = self.read32(font + 4);
        let gp = self.read32(glyphs + 4 * c as u32);
        let w = if gp >> 24 == 8 { self.read8(gp + 5) as u32 } else { 4 };
        // same text handle, cursor moved on (glyph widths may differ by kerning)
        let cont = self.hd.live.get(&t).filter(|l| x >= l.1.saturating_sub(2) && x <= l.1 + 8 && x > 1).map(|l| l.0);
        if cont.is_none() && x < 2 {
            // a line printed from its start again: the box was cleared, forget what
            // was in this text's tiles
            let (lo, hi) = (base + 2 * chr, base + 2 * (chr + tw.max(1)));
            self.hd.texts.retain(|e| e.0 + e.1 <= lo || hi <= e.0);
            // a new page on the first line: the other lines of the box go too
            if chr == 0 {
                // the text tiles of the box's lines follow each other
                let end = lo + 2 * tw.max(1) * 4;
                self.hd.texts.retain(|e| e.0 < lo || e.0 >= end);
            }
            if chr == 0 || self.hd.live.values().all(|l| l.0 >= lo) {
                let others: Vec<u32> = self.hd.live.iter().filter(|(k, _)| **k != t).map(|(_, l)| l.0).collect();
                self.hd.texts.retain(|e| !others.contains(&e.0));
                self.hd.live.retain(|k, _| *k == t);
            }
        }
        let start = match cont {
            Some(st) => st,
            None => base + 2 * (chr + x / 8),
        };
        let pos = self.hd.texts.iter().rposition(|e| e.0 == start);
        match (cont, pos) {
            (Some(_), Some(i)) => {
                let e = &mut self.hd.texts[i];
                e.4.push(c as char);
                let end_col = chr + (x + w + 7) / 8;
                e.1 = (2 * (end_col - (start - base) / 2)).max(e.1);
            }
            _ => {
                let n = 2 * ((x % 8 + w + 7) / 8).max(1);
                self.hd.texts.retain(|e| e.0 + e.1 <= start || start + n <= e.0);
                self.hd.texts.push((start, n, x % 8, col, (c as char).to_string()));
                if self.hd.texts.len() > 256 {
                    self.hd.texts.remove(0);
                }
            }
        }
        if self.hd.text_dbg {
            eprintln!("CHAR t {:08x} chr {} x {} w {} c {:?} cont {:?}", t, chr, x, w, c as char, cont);
        }
        self.hd.live.insert(t, (start, x + w));
    }

    pub fn hd_text_hook(&mut self, t: u32, sp: u32, number: bool) {
        let chr = self.read16(t) as u32;
        let mut x = self.read8(t + 2) as u32;
        let col = self.read8(t + 3);
        let tw = self.read8(t + 4) as u32;
        let mut s = Vec::new();
        if number {
            s = sp.to_string().into_bytes();
        } else {
            for i in 0..80 {
                let c = self.read8(sp + i);
                if c <= 1 || c == 0x1F {
                    break;
                }
                s.push(c);
            }
        }
        let text = String::from_utf8_lossy(&s).to_string();
        if self.hd.text_dbg {
            let raw: Vec<u8> = (0..8).map(|i| self.read8(t + i)).collect();
            eprintln!("TEXT t {:08x} {:02x?} {:?}", t, raw, text);
        }
        // text tiles: two per column (top, bottom) from the font's tile base
        let font = self.read32(0x0202_8D70);
        let base = (self.read16(font + 0x10) & 0x3FF) as u32;
        if !number && x < 8 {
            let (lo, hi) = (base + 2 * chr, base + 2 * (chr + tw.max(1)));
            self.hd.texts.retain(|e| e.0 + e.1 <= lo || hi <= e.0);
        }
        let start = base + 2 * (chr + x / 8);
        // width of the string in pixels from the font's glyph table (as the game does)
        let glyphs = self.read32(font + 4);
        let mut w = 0u32;
        for &c in &s {
            let gp = self.read32(glyphs + 4 * c as u32);
            if gp >> 24 == 8 {
                w += self.read8(gp + 5) as u32;
            }
        }
        if number {
            // numbers are drawn right-aligned, ending at the cursor
            x = x.saturating_sub(w);
            let start = base + 2 * (chr + x / 8);
            let cols = ((x % 8 + w + 7) / 8).max(1);
            let n = 2 * cols;
            let text = String::from_utf8_lossy(&s).to_string();
            self.hd.texts.retain(|e| e.0 + e.1 <= start || start + n <= e.0);
            self.hd.texts.push((start, n, x % 8, col, text));
            return;
        }
        let cols = ((x % 8 + w + 7) / 8).max(1).min(tw.saturating_sub(x / 8).max(1));
        let n = 2 * cols;
        self.hd.texts.retain(|e| e.0 + e.1 <= start || start + n <= e.0);
        self.hd.texts.push((start, n, x % 8, col, text));
        if self.hd.texts.len() > 256 {
            self.hd.texts.remove(0);
        }
    }

    /// Text shown in the game's text window right now (current page, lines).
    /// The whole page of text being typed out right now (not only the part
    /// already on screen): the pointer at 0x03000040 marks the typing position.
    pub fn hd_typed_text(&mut self) -> Vec<String> {
        let ptr = self.read32(0x0300_0040);
        let mut l = self.talk_text(ptr, false, 3).3;
        while l.last().map_or(false, |x| x.is_empty()) {
            l.pop();
        }
        l
    }

    pub fn hd_current_text(&mut self) -> Vec<String> {
        let ptr = self.read32(0x0300_0040);
        self.talk_text(ptr, false, 3).0
    }

    /// HP boxes of a battle on the map (battle animations off / map battles).
    fn hd_mapbattle_detect(&mut self) {
        self.hd.mapinfo.fights.clear();
        if !self.hd.menu_native {
            return;
        }
        // the map battle process; staff use, dancing and attacking walls show the
        // same HP boxes from other processes: then go by the boxes alone
        let proc = self.proc_find(0x08C9_D7B0).is_some();
        if !proc && !self.ppu.wide_map {
            return;
        }
        // map animation actors: 0x0203E0FC, 0x14 bytes each: unit, battle unit, proc, max HP, HP
        let mut actors = Vec::new();
        for k in 0..2u32 {
            let e = 0x0203_E0FC + k * 0x14;
            let u = self.read32(e);
            if u >> 24 != 2 {
                continue;
            }
            let ch = self.read32(u);
            let name = if ch >> 24 == 8 {
                let id = self.read16(ch) as u32;
                crate::text::decode(&self.rom, id).trim().to_string()
            } else {
                String::new()
            };
            let side = match self.read8(u + 0x0B) >> 6 { 0 => 0, 1 => 1, _ => 2 };
            actors.push((name, self.read8(e + 0x0D) as i32, self.read8(e + 0x0C) as i32, side));
        }
        let mut any = false;
        for (pal, side) in [(2u16, 0), (1, 2), (3, 1)] {
            let (mut x0, mut y0, mut x1, mut y1, mut n) = (99, 99, -1, -1, 0);
            for cy in 0..20 {
                for cx in 0..30 {
                    let e = self.bg_cell(1, cx, cy);
                    if (e >> 12) as u16 == pal && (0x001..=0x014).contains(&(e & 0x3FF)) {
                        x0 = x0.min(cx);
                        y0 = y0.min(cy);
                        x1 = x1.max(cx + 1);
                        y1 = y1.max(cy + 1);
                        n += 1;
                    }
                }
            }
            if n < 8 || (!proc && (y1 - y0 > 4 || x1 - x0 > 12)) {
                continue;
            }
            let Some(a) = actors.iter().find(|a| a.3 == side).or(actors.first()).cloned() else { continue };
            let r = (x0 * 8, y0 * 8, x1 * 8, y1 * 8);
            let sh = self.ui_shift_at(r.0, r.1, r.2, r.3);
            self.mask_cells((x0, y0, x1, y1));
            self.hd.mapinfo.fights.push((r, sh, a.0, a.1, a.2, side));
            any = true;
        }
        if any {
            self.hd.hiding = true;
        }
    }

    /// Goal / terrain / unit windows of the free map (map UI process 0x08CC2D38).
    fn hd_mapinfo_detect(&mut self) {
        self.hd.mapinfo = MapInfo::default();
        if !self.hd.menu_native || !self.ppu.wide_map || (self.proc_find(0x08CC_2D38).is_none() && self.proc_find(0x08CC_2D98).is_none()) {
            self.hd.last_unit = None;
            return;
        }
        // text of the info windows while they slide in or out (their frame is not on
        // screen yet): letters the game printed, outside any window frame
        if self.hd.talk_mask.len() == 600 {
            let wins = self.bg1_windows_all();
            let cnt = self.io16(0x08) as usize;
            let sb = ((cnt >> 8) & 31) * 0x800;
            let mut hid = false;
            for cy in 0..20i32 {
                for cx in 0..30i32 {
                    if wins.iter().any(|&(a, b, c, d)| cx >= a && cx < c && cy >= b && cy < d) {
                        continue;
                    }
                    let a = sb + ((cy * 32 + cx) * 2) as usize;
                    let e = (self.vram[a] as u32 | (self.vram[a + 1] as u32) << 8) & 0x3FF;
                    if e != 0 && self.hd.texts.iter().any(|t| e >= t.0 && e < t.0 + t.1) {
                        self.hd.talk_mask[(cy * 30 + cx) as usize] = true;
                        hid = true;
                    }
                }
            }
            if hid {
                self.hd.hiding = true;
            }
        }
        let (cx, cy) = (self.read16(0x0202_BBCC) as i32, self.read16(0x0202_BBCE) as i32);
        let (w, h) = self.map_size();
        let on_map = (0..w).contains(&cx) && (0..h).contains(&cy);
        let mut any = false;
        // goal; a second line counts enemies left (rout) or turns (survive)
        if let Some(b) = self.bg1_box(0x176, 0x17F) {
            let ch = self.read8(0x0202_BC06) as u32;
            let cd = 0x08C9_A200 + ch * 0x98;
            let id = self.read16(cd + 0x8E) as u32;
            let text = crate::text::decode(&self.rom, id).trim().to_string();
            let kind = self.read8(cd + 0x90);
            // like the game (0x08085E36): only rout (1) and hold-out (2) chapters
            // have a second line
            let line2 = if b.3 - b.1 <= 4 {
                Some(String::new())
            } else if kind == 1 {
                if self.read8(0x0202_BC05) != 0 {
                    // fog of war: the game hides the number
                    Some("Left ??".to_string())
                } else {
                    let mut n = 0;
                    for i in 0x81..0xC0u32 {
                        let u = self.read32(0x08B9_2EB0 + 4 * i);
                        if u >> 24 != 2 {
                            continue;
                        }
                        if self.read32(u) != 0 && self.read32(u + 0x0C) & 0x0C == 0 {
                            n += 1;
                        }
                    }
                    Some(format!("Left {}", n))
                }
            } else if kind == 2 {
                let turn = self.read16(0x0202_BC08) as i32;
                let last = self.read8(cd + 0x91) as i32 - 1;
                if turn >= last {
                    Some("Last Turn".to_string())
                } else {
                    Some(format!("Turn {}/{}", turn, last))
                }
            } else {
                Some(String::new())
            };
            if let (false, Some(l2)) = (text.is_empty(), line2) {
                let r = (b.0 * 8, b.1 * 8, b.2 * 8, b.3 * 8);
                let sh = self.ui_shift_at(r.0, r.1, r.2, r.3);
                self.mask_cells(b);
                self.hd.mapinfo.goal = Some((r, sh, text, l2));
                any = true;
            }
        }
        // terrain under the cursor (breakable walls / snags show HP: keep those original)
        if let (Some(b), true) = (self.bg1_box(0x100, 0x116), on_map) {
            let tp = self.read32(MAP_TERRAIN);
            let row = self.read32(tp + 4 * cy as u32);
            let t = self.read8(row + cx as u32) as u32;
            if t != 0x1B && t != 0x33 && t < 0x41 {
                let name = crate::text::decode(&self.rom, 0x5E1 + t).trim().to_string();
                let name = if name.chars().any(|c| c.is_ascii_alphabetic()) { name } else { "---".to_string() };
                let def = self.read8(0x08BE_453A + t) as i32;
                let avo = self.read8(0x08BE_44F9 + t) as i32;
                let r = (b.0 * 8, b.1 * 8, b.2 * 8, b.3 * 8);
                let sh = self.ui_shift_at(r.0, r.1, r.2, r.3);
                self.mask_cells(b);
                self.hd.mapinfo.terrain = Some((r, sh, name, def, avo));
                any = true;
            }
        }
        // unit under the cursor
        if let Some(b) = self.bg1_box(0x155, 0x15D) {
            let up = self.read32(0x0202_E3DC);
            // refresh the shown unit only when the game redraws the window contents
            // (the name/HP must match the portrait the game shows)
            let cnt0 = self.io16(8) as usize;
            let cb0 = ((cnt0 >> 2) & 3) * 0x4000;
            let mut hsh: u64 = 0xcbf29ce484222325;
            for y in b.1..b.3 {
                for x in b.0..b.2 {
                    let t = (self.bg_cell(0, x, y) & 0x3FF) as usize;
                    hsh = (hsh ^ t as u64).wrapping_mul(0x100000001b3);
                    for k in (0..32).step_by(3) {
                        let v = self.vram.get(cb0 + t * 32 + k).copied().unwrap_or(0);
                        hsh = (hsh ^ v as u64).wrapping_mul(0x100000001b3);
                    }
                }
            }
            let redrawn = hsh != self.hd.unit_hash || self.hd.last_unit.is_none();
            self.hd.unit_hash = hsh;
            let id = if on_map && redrawn {
                let row = self.read32(up + 4 * cy as u32);
                self.read8(row + cx as u32) as u32
            } else {
                0
            };
            if id != 0 {
                let u = self.read32(0x08B9_2EB0 + 4 * id);
                let ch = self.read32(u);
                if ch >> 24 == 8 {
                    let nid = self.read16(ch) as u32;
                    self.hd.last_unit = Some(UnitWin {
                        name: crate::text::decode(&self.rom, nid).trim().to_string(),
                        hp: self.read8(u + 0x13) as i32,
                        max_hp: self.read8(u + 0x12) as i32,
                        side: match id >> 6 { 0 => 0, 1 => 1, _ => 2 },
                        face: None,
                    });
                }
            }
            if let Some(mut uw) = self.hd.last_unit.clone() {
                // mini portrait: BG0 cells with palette 4 inside the window
                let (mut fx0, mut fy0, mut fx1, mut fy1) = (99, 99, -1, -1);
                for y in b.1..b.3 {
                    for x in b.0..b.2 {
                        let e = self.bg_cell(0, x, y);
                        if e >> 12 == 4 && e & 0x3FF != 0 {
                            fx0 = fx0.min(x);
                            fy0 = fy0.min(y);
                            fx1 = fx1.max(x + 1);
                            fy1 = fy1.max(y + 1);
                        }
                    }
                }
                if fx1 > fx0 {
                    uw.face = Some((fx0 * 8, fy0 * 8, fx1 * 8, fy1 * 8));
                }
                let r = (b.0 * 8, b.1 * 8, b.2 * 8, b.3 * 8);
                let sh = self.ui_shift_at(r.0, r.1, r.2, r.3);
                self.mask_cells(b);
                self.hd.mapinfo.unit = Some((r, sh, uw));
                any = true;
            } else {
                // unit not known (state loaded while the window was already open):
                // hide the game's window instead of showing it in its old look
                self.mask_cells(b);
                self.hd.obj_hide.push((b.0 * 8 - 8, b.1 * 8 - 8, b.2 * 8 + 8, b.3 * 8 + 8));
                any = true;
            }
        } else {
            self.hd.last_unit = None;
        }
        if any {
            self.hd.hiding = true;
        }
    }

    pub(crate) fn hd_frame_start(&mut self) {
        if !self.hd.pending_chars.is_empty() {
            let p = std::mem::take(&mut self.hd.pending_chars);
            for (a, c) in p {
                let e = self.read16(a) as u32 & 0x3FF;
                if e != 0 {
                    self.hd.char_tiles.insert(e, c);
                }
            }
        }
        self.hd_talk_detect();
        self.hd_menu_detect();
        self.hd_forecast_detect();
        self.hd_linger();
        self.hd_mapinfo_detect();
        self.hd_mapbattle_detect();
        self.hd_banner_detect();
        self.hd_generic_detect();
        self.hd.map = self.ppu.wide_map;
        self.hd.sprites.clear();
        self.hd.cursor = None;
        if !self.hd.map {
            return;
        }
        let (bx, by) = self.map_origin();
        self.hd.view = (bx, by);
        // brightness effect on the map layer
        let bldcnt = self.io16(0x50);
        let evy = (self.io16(0x54) & 31).min(16) as i32;
        // with windows on, effects only reach the map where the outside area allows them
        let dispcnt = self.io16(0);
        let effects_outside = dispcnt & 0xE000 == 0 || self.io16(0x4A) & 0x20 != 0;
        self.hd.fade = if !effects_outside { 0 } else { match (bldcnt >> 6) & 3 {
            2 if bldcnt & 8 != 0 => evy,
            3 if bldcnt & 8 != 0 => -evy,
            _ => 0,
        } };
        if self.io16(0) & 0x800 == 0 {
            self.hd.fade = -16;
        }
        self.hd_collect_sprites(bx, by);
        // movement / attack range
        let (w, h) = self.map_size();
        let n = (w * h).max(0) as usize;
        self.hd.cells.clear();
        self.hd.cells.resize(n, 0);
        self.hd.cells_w = w;
        if self.ppu.wide_blue | self.ppu.wide_red != 0 {
            for my in 0..h {
                for mx in 0..w {
                    self.hd.cells[(my * w + mx) as usize] = self.cell_status(mx, my);
                }
            }
        }
    }

    fn hd_collect_sprites(&mut self, bx: i32, by: i32) {
        if self.hd.atlas.len() != ATLAS * ATLAS {
            self.hd.atlas = vec![0; ATLAS * ATLAS];
        }
        let dispcnt = self.io16(0);
        if dispcnt & 0x1000 == 0 {
            return;
        }
        let one_d = dispcnt & 0x40 != 0;
        let (mut ax, mut ay, mut rowh) = (0i32, 0i32, 0i32);
        let mut cursor_seen = false;
        let (mut cx0, mut cy0, mut cx1, mut cy1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for i in 0..128 {
            let o = i * 8;
            let a0 = (self.oam[o] as u32) | (self.oam[o + 1] as u32) << 8;
            let a1 = (self.oam[o + 2] as u32) | (self.oam[o + 3] as u32) << 8;
            let a2 = (self.oam[o + 4] as u32) | (self.oam[o + 5] as u32) << 8;
            let affine = a0 & 0x100 != 0;
            if (!affine && a0 & 0x200 != 0) || (a0 >> 10) & 3 >= 2 || a0 >> 14 == 3 {
                continue;
            }
            if !Hd::world_obj(a0, a1, a2) {
                continue;
            }
            let kind = Hd::obj_kind(a0, a1, a2);
            let (w, h) = OBJ_SIZE[(a0 >> 14) as usize][(a1 >> 14) as usize];
            let mut sy = (a0 & 0xFF) as i32;
            if sy >= 160 {
                sy -= 256;
            }
            let raw = (a1 & 0x1FF) as i32;
            let sx = if raw >= 256 + 32 { raw - 512 } else { raw };
            if sx + w <= -64 || sx >= 304 || sy + h <= -64 || sy >= 224 {
                continue;
            }
            if kind == ObjKind::Cursor {
                cursor_seen = true;
                cx0 = cx0.min(sx);
                cy0 = cy0.min(sy);
                cx1 = cx1.max(sx + w);
                cy1 = cy1.max(sy + h);
                continue;
            }
            if ax + w > ATLAS as i32 {
                ax = 0;
                ay += rowh;
                rowh = 0;
            }
            if ay + h > ATLAS as i32 {
                break;
            }
            let bpp8 = a0 & 0x2000 != 0;
            let base_tile = (a2 & 0x3FF) as i32;
            let palbank = ((a2 >> 12) * 16) as usize;
            for ty in 0..h {
                for tx in 0..w {
                    let sxp = if !affine && a1 & 0x1000 != 0 { w - 1 - tx } else { tx };
                    let syp = if !affine && a1 & 0x2000 != 0 { h - 1 - ty } else { ty };
                    let tstep = if bpp8 { 2 } else { 1 };
                    let tile = if one_d {
                        base_tile + (syp / 8) * (w / 8) * tstep + (sxp / 8) * tstep
                    } else {
                        (base_tile & if bpp8 { !1 } else { !0 }) + (syp / 8) * 32 + (sxp / 8) * tstep
                    } & 0x3FF;
                    let ci = if bpp8 {
                        let a = 0x10000 + tile as usize * 32 + ((syp % 8) * 8 + sxp % 8) as usize;
                        let c = self.vram[a & 0x17FFF] as usize;
                        if c == 0 { 0 } else { 0x100 + c }
                    } else {
                        let a = 0x10000 + tile as usize * 32 + ((syp % 8) * 4 + (sxp % 8) / 2) as usize;
                        let n = (self.vram[a & 0x17FFF] >> ((sxp & 1) * 4)) & 15;
                        if n == 0 { 0 } else { 0x100 + palbank + n as usize }
                    };
                    let px = if ci == 0 {
                        0
                    } else {
                        let c = (self.pal[ci * 2] as u32 | (self.pal[ci * 2 + 1] as u32) << 8) & 0x7FFF;
                        self.rgb(c) | 0xFF00_0000
                    };
                    self.hd.atlas[(ay + ty) as usize * ATLAS + (ax + tx) as usize] = px;
                }
            }
            self.hd.sprites.push(HdSprite { x: bx + sx, y: by + sy, w, h, ax, ay, semi: (a0 >> 10) & 3 == 1, flat: kind == ObjKind::Flat });
            ax += w;
            rowh = rowh.max(h);
        }
        if cursor_seen {
            // the corner sprites frame the cursor tile
            let mx = (bx + (cx0 + cx1) / 2).div_euclid(16) * 16;
            let my = (by + (cy0 + cy1) / 2).div_euclid(16) * 16;
            self.hd.cursor = Some((mx, my));
        }
    }

    /// Map size in cells (0,0 if no map is loaded).
    pub fn hd_map_size(&mut self) -> (i32, i32) {
        let (w, h) = self.map_size();
        if (1..=64).contains(&w) && (1..=64).contains(&h) && self.read32(BASE_TILE_ROWS) >> 24 == 2 {
            (w, h)
        } else {
            (0, 0)
        }
    }

    /// Terrain id per cell, row-major (`margin` extra cells on every side
    /// continue the map's scenery and get the id 0xFF).
    pub fn hd_terrain(&mut self, margin: i32) -> Vec<u8> {
        let (w, h) = self.hd_map_size();
        let (tw, th) = (w + 2 * margin, h + 2 * margin);
        let mut out = vec![0xFFu8; (tw * th).max(0) as usize];
        let tp = self.read32(MAP_TERRAIN);
        for my in 0..h {
            let row = self.read32(tp + 4 * my as u32);
            for mx in 0..w {
                out[((my + margin) * tw + mx + margin) as usize] = self.read8(row + mx as u32);
            }
        }
        out
    }

    /// Metatile terrain for a scenery cell outside the map (for its height).
    pub fn hd_scenery_terrain(&mut self, mx: i32, my: i32) -> u8 {
        match self.scenery_metatile(mx, my) {
            Some(v) => self.read8(0x0203_2A90 + (v as u32 >> 2)),
            None => 0x01,
        }
    }

    /// The map as a picture: (w+2m)*16 x (h+2m)*16 pixels, 0xAARRGGBB,
    /// drawn with the game's current tiles and palettes (animated water etc.).
    /// The margin shows the generated scenery around the map.
    pub fn hd_map_image(&mut self, margin: i32) -> Vec<u32> {
        let mut img = self.hd_map_image_raw(margin);
        self.hd_apply_fog(&mut img, margin);
        img
    }

    /// Fog of war: per map cell true if hidden (only when the chapter has fog).
    pub fn hd_fog(&mut self) -> Vec<bool> {
        let (w, h) = self.hd_map_size();
        let mut out = vec![false; (w * h).max(0) as usize];
        if w == 0 || self.read8(0x0202_BC05) == 0 {
            return out;
        }
        let fp = self.read32(MAP_FOG);
        for my in 0..h {
            let row = self.read32(fp + 4 * my as u32);
            for mx in 0..w {
                out[(my * w + mx) as usize] = self.read8(row + mx as u32) == 0;
            }
        }
        out
    }

    /// Darkens the fogged cells of a map picture (like the original's dim palette).
    fn hd_apply_fog(&mut self, img: &mut [u32], margin: i32) {
        let fog = self.hd_fog();
        if !fog.iter().any(|&f| f) {
            return;
        }
        let (w, h) = self.hd_map_size();
        let iw = ((w + 2 * margin) * 16) as usize;
        // the surroundings outside the map are never in sight
        for my in -margin..h + margin {
            for mx in -margin..w + margin {
                let inside = (0..w).contains(&mx) && (0..h).contains(&my);
                if inside && !fog[(my * w + mx) as usize] {
                    continue;
                }
                for py in 0..16 {
                    let y = ((my + margin) * 16 + py) as usize;
                    for px in 0..16 {
                        let x = ((mx + margin) * 16 + px) as usize;
                        let c = img[y * iw + x];
                        let (r, g, b) = ((c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF);
                        let l = (r * 3 + g * 6 + b) / 10;
                        let f = |v: u32, t: u32| ((v * 3 + l * 2) / 5 * 9 / 20 + t).min(255);
                        img[y * iw + x] = (c & 0xFF00_0000) | f(r, 4) << 16 | f(g, 8) << 8 | f(b, 22);
                    }
                }
            }
        }
    }

    fn hd_map_image_raw(&mut self, margin: i32) -> Vec<u32> {
        let (w, h) = self.hd_map_size();
        if w == 0 {
            return Vec::new();
        }
        let (tw, th) = (w + 2 * margin, h + 2 * margin);
        let iw = (tw * 16) as usize;
        let mut img = vec![0xFF00_0000u32; iw * (th * 16) as usize];
        self.mapgen_update(w, h);
        let cnt = self.io16(0x0E) as u32;
        let char_base = ((cnt >> 2) & 3) as usize * 0x4000;
        for cy in 0..th {
            let my = cy - margin;
            let row = if (0..h).contains(&my) { self.read32(BASE_TILE_ROWS + 4 * my as u32) } else { 0 };
            for cx in 0..tw {
                let mx = cx - margin;
                let v = if row != 0 && (0..w).contains(&mx) {
                    Some(self.read16(row + 2 * mx as u32))
                } else {
                    self.scenery_metatile(mx, my)
                };
                let Some(v) = v else { continue };
                for k in 0..4u32 {
                    let t = self.read16(METATILES + 2 * (v as u32 + k)) as u32;
                    let ent = (t & 0x0FFF) | ((((t >> 12) + 6) & 15) << 12);
                    let (ox, oy) = ((k & 1) * 8, (k >> 1) * 8);
                    for py in 0..8u32 {
                        for px in 0..8u32 {
                            let c = self.tile_px(char_base, ent, px, py);
                            let c = if c & TRANSPARENT != 0 { pal_rgb(&self.pal, 0) } else { c };
                            let x = (cx * 16) as usize + (ox + px) as usize;
                            let y = (cy * 16) as usize + (oy + py) as usize;
                            img[y * iw + x] = self.rgb(c);
                        }
                    }
                }
            }
        }
        img
    }

    /// Cut-outs for upright standees: like `hd_map_image`, but only the cells whose
    /// terrain is in `kinds`, with every pixel that has a colour of plain ground
    /// (terrain 0x01 cells) made transparent. Everything else is transparent.
    pub fn hd_map_cutout(&mut self, margin: i32, kinds: &[u8]) -> Vec<u32> {
        let (w, h) = self.hd_map_size();
        let img = self.hd_map_image_raw(margin);
        if w == 0 || img.is_empty() {
            return img;
        }
        let map_terr = self.hd_terrain(0);
        let tw = (w + 2 * margin) as usize;
        let iw = tw * 16;
        let ih = img.len() / iw;
        // work on the whole picture: the generated scenery around the map gets
        // standees too (its terrain comes from the scenery metatiles)
        let th = h + 2 * margin;
        let mut terr = vec![0u8; tw * th as usize];
        for cy in 0..th {
            for cx in 0..tw as i32 {
                let (mx, my) = (cx - margin, cy - margin);
                terr[(cy * tw as i32 + cx) as usize] = if (0..w).contains(&mx) && (0..h).contains(&my) {
                    map_terr[(my * w + mx) as usize]
                } else {
                    self.hd_scenery_terrain(mx, my)
                };
            }
        }
        let fog_margin = margin;
        let (w, h, margin) = (tw as i32, th, 0i32);
        // colours of plain ground anywhere on the map
        let mut ground = std::collections::HashSet::new();
        for my in 0..h {
            for mx in 0..w {
                let t = terr[(my * w + mx) as usize];
                if t != 0x01 && t != 0x02 {
                    continue;
                }
                for py in 0..16 {
                    for px in 0..16 {
                        let x = ((mx + margin) * 16 + px) as usize;
                        let y = ((my + margin) * 16 + py) as usize;
                        ground.insert(img[y * iw + x]);
                    }
                }
            }
        }
        // regions: every tree cell alone, buildings joined into connected groups
        let mut reg = vec![-1i32; (w * h) as usize];
        let mut next = 0;
        for my in 0..h {
            for mx in 0..w {
                let i = (my * w + mx) as usize;
                let t = terr[i];
                if reg[i] != -1 || !kinds.contains(&t) {
                    continue;
                }
                let tree = t == 0x0C || t == 0x0D;
                reg[i] = next;
                let mut st = vec![(mx, my)];
                while let Some((x, y)) = st.pop() {
                    if tree {
                        break;
                    }
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h {
                            continue;
                        }
                        let j = (ny * w + nx) as usize;
                        let tj = terr[j];
                        if reg[j] == -1 && kinds.contains(&tj) && tj != 0x0C && tj != 0x0D {
                            reg[j] = next;
                            st.push((nx, ny));
                        }
                    }
                }
                next += 1;
            }
        }
        let region = |mx: i32, my: i32| -> i32 {
            if mx < 0 || my < 0 || mx >= w || my >= h { -1 } else { reg[(my * w + mx) as usize] }
        };
        // background colours of each region: frequent colours on its outer border
        let mut hist: Vec<std::collections::HashMap<u32, u32>> = vec![Default::default(); next as usize];
        let mut border_n = vec![0u32; next as usize];
        let mut out = vec![0u32; img.len()];
        let mut seeds: Vec<(i32, i32)> = Vec::new();
        for my in 0..h {
            for mx in 0..w {
                let r = region(mx, my);
                if r < 0 {
                    continue;
                }
                for py in 0..16 {
                    for px in 0..16 {
                        let x = ((mx + margin) * 16 + px) as usize;
                        let y = ((my + margin) * 16 + py) as usize;
                        out[y * iw + x] = img[y * iw + x];
                        let edge = (px == 0 && region(mx - 1, my) != r)
                            || (px == 15 && region(mx + 1, my) != r)
                            || (py == 0 && region(mx, my - 1) != r)
                            || (py == 15 && region(mx, my + 1) != r);
                        if edge {
                            *hist[r as usize].entry(img[y * iw + x]).or_insert(0) += 1;
                            border_n[r as usize] += 1;
                            seeds.push((x as i32, y as i32));
                        }
                    }
                }
            }
        }
        let bg: Vec<std::collections::HashSet<u32>> = hist
            .iter()
            .zip(&border_n)
            .map(|(hm, &n)| hm.iter().filter(|(k, &c)| c * 100 >= n * 6 && ground.contains(*k)).map(|(&k, _)| k).collect())
            .collect();
        let cell_region = |x: i32, y: i32| region(x / 16 - margin, y / 16 - margin);
        let mut clear = vec![false; img.len()];
        let mut stack: Vec<(i32, i32)> = seeds
            .into_iter()
            .filter(|&(x, y)| {
                let r = cell_region(x, y);
                r >= 0 && bg[r as usize].contains(&img[y as usize * iw + x as usize])
            })
            .collect();
        // flood through background colours inside the same region
        while let Some((x, y)) = stack.pop() {
            let i = y as usize * iw + x as usize;
            if clear[i] {
                continue;
            }
            clear[i] = true;
            let r = cell_region(x, y);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx as usize >= iw || ny as usize >= ih {
                    continue;
                }
                let j = ny as usize * iw + nx as usize;
                if !clear[j] && cell_region(nx, ny) == r && bg[r as usize].contains(&img[j]) {
                    stack.push((nx, ny));
                }
            }
        }
        for (i, c) in clear.iter().enumerate() {
            if *c {
                out[i] = 0;
            }
        }
        self.hd_apply_fog(&mut out, fog_margin);
        out
    }

    /// Cheap fingerprint of everything `hd_map_image` depends on.
    pub fn hd_map_hash(&mut self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mut feed = |b: u8| {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        };
        let cnt = self.io16(0x0E) as usize;
        let cb = ((cnt >> 2) & 3) * 0x4000;
        for &b in self.vram[cb..cb + 0x8000].iter().step_by(7) {
            feed(b);
        }
        for &b in &self.pal[..0x200] {
            feed(b);
        }
        let (w, hh) = self.hd_map_size();
        feed(w as u8);
        feed(hh as u8);
        for f in self.hd_fog() {
            feed(f as u8);
        }
        if w > 0 {
            for my in 0..hh {
                let row = self.read32(BASE_TILE_ROWS + 4 * my as u32);
                for mx in 0..w {
                    let v = self.read16(row + 2 * mx as u32);
                    feed(v as u8);
                    feed((v >> 8) as u8);
                }
            }
        }
        h
    }
}

fn pal_rgb(pal: &[u8], i: usize) -> u32 {
    (pal[i * 2] as u32 | (pal[i * 2 + 1] as u32) << 8) & 0x7FFF
}
