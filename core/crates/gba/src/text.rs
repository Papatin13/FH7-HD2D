//! The game's text: Huffman-compressed strings in the ROM (FE7 US).
const TREE: u32 = 0x08B7_D71C;
const ROOT_PTR: u32 = 0x08B8_08A8;
const TABLE: u32 = 0x08B8_08AC;

fn rd16(rom: &[u8], a: u32) -> u32 {
    let o = (a & 0x01FF_FFFF) as usize;
    if o + 1 < rom.len() { u16::from_le_bytes([rom[o], rom[o + 1]]) as u32 } else { 0 }
}
fn rd32(rom: &[u8], a: u32) -> u32 {
    rd16(rom, a) | rd16(rom, a + 2) << 16
}

/// Decodes text `id` to plain text (control codes become spaces/new lines).
pub fn decode(rom: &[u8], id: u32) -> String {
    let p = rd32(rom, TABLE + 4 * id);
    let mut out: Vec<u8> = Vec::new();
    if p & 0x8000_0000 != 0 {
        let mut a = p & 0x7FFF_FFFF;
        while out.len() < 1024 {
            let b = rom.get((a & 0x01FF_FFFF) as usize).copied().unwrap_or(0);
            if b == 0 {
                break;
            }
            out.push(b);
            a += 1;
        }
    } else if p >> 24 == 8 {
        let root = rd32(rom, ROOT_PTR);
        let mut node = root;
        let mut src = p;
        let mut bit = 0;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 20000 {
                break;
            }
            let l = rd16(rom, node);
            let r = rd16(rom, node + 2);
            if r == 0xFFFF {
                if l & 0xFF == 0 {
                    break;
                }
                out.push(l as u8);
                if l >> 8 != 0 {
                    out.push((l >> 8) as u8);
                }
                node = root;
                continue;
            }
            let byte = rom.get((src & 0x01FF_FFFF) as usize).copied().unwrap_or(0);
            let b = (byte >> bit) & 1;
            bit += 1;
            if bit == 8 {
                bit = 0;
                src += 1;
            }
            node = TREE + 4 * if b != 0 { r } else { l };
        }
    }
    let mut s = String::new();
    for b in out {
        match b {
            0x01 => s.push('\n'),
            0x20..=0x7E => s.push(b as char),
            _ => {}
        }
    }
    s.trim().to_string()
}
