# Game core (Rust)

The core runs the game from the player's ROM and hands the game state to the Godot
project. It is built with `tools/build.sh` in the main folder.

## How it works

```
crates/
  gba/      platform layer: memory, CPU semantics (ARM/Thumb), software PPU (modes 0-5, affine,
            sprites, windows, blending, mosaic), DMA, timers, IRQ, BIOS replacement (HLE),
            sound (Direct Sound FIFO + 4 PSG channels), save states
  recomp/   static recompiler: translates the ROM's machine code into Rust source
  fe7gen/   placeholder; with tools/build.sh <rom> it links the code generated from YOUR ROM
            (core/generated/: ~60,000 basic blocks / ~317,000 instructions, never upload it)
  fe7hd/    GDExtension for the Godot project (class Fe7Core): game state, maps, texts, frames
  fe7/      standalone 2D frontend (window, keyboard, mouse, audio) - useful for testing
traces/     block addresses from play sessions that help the recompiler find code (no game data)
```

The recompiler finds code by recursive descent from the reset vector, through function
pointers in the ROM and through traces of real play. Every basic block becomes a Rust
function that calls the same instruction helpers as the reference interpreter (operands
already decoded, literal pools folded into constants). Code the game copies into IWRAM at
runtime (sound mixer, IRQ handler) is recompiled too and checked against the RAM contents
before each run. Whatever is not recompiled runs on the interpreter, so the game always works.

Verification: `--verify` runs every recompiled block, undoes it, runs the same instructions
on the interpreter and compares registers and every memory write. Test status: more than
240 million block executions (intro, title, game start, prologue, battle) without a mismatch.

Graphics, music and text are read from the ROM at runtime. The generated Rust code also comes
from the ROM and therefore only lives locally in `core/generated/` (ignored by git).

## Developer options of the 2D frontend

```
cargo run --release -p fe7 -- <rom.gba>
```

`--headless --frames N --script "120:START,200:A" --shot-every N --out dir`, `--verify`, `--interp`,
`--trace file.txt` (collects block addresses for the recompiler), `--save/--load`.
