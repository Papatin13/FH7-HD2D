# FE7-HD2D

An unofficial HD-2D remake of *Fire Emblem* (Game Boy Advance, US version) for PC:
the maps become a 3D diorama on a war-room table, the menus are redrawn sharp in the
style of the original, and everything can be played with **mouse and keyboard**.

> **This repository contains no game data.** Graphics, text, music and game logic are
> read at runtime from the player's own ROM. You need your own ROM of the US version
> (for example a backup of your cartridge).
>
> Not affiliated with Nintendo or Intelligent Systems. "Fire Emblem" is a trademark of Nintendo.

## Playing

1. Download the latest version from **Releases** and unpack it (Linux x86-64).
2. Run `./start.sh`.
3. In the launcher, choose the ROM or drop it onto the window. Only the unmodified
   US ROM is accepted (game code `AE7E`, CRC32 `2A524221`). After that the game starts directly.

The save file (`.sav`) is stored next to the ROM. Settings: `~/.local/share/godot/app_userdata/FE7-HD2D/`.
To open the launcher again: `FE7HD_LAUNCHER=1 ./start.sh`.

### Controls

| | |
|---|---|
| Mouse | click everything (map, menus, lists), right click = back |
| Mouse wheel | scroll in menus, zoom on the map |
| Drag with the middle button | turn the camera (with Shift: move it) |
| Arrows / WASD | move |
| Enter / Space | confirm |
| Esc / Backspace | back, pause on the free map |
| Q / Tab | next unit |
| E / I | unit info |
| F5 / F8 | quick save / quick load |
| F11 | fullscreen |
| Hold Ctrl | fast forward |

### Debug mode: load any chapter

Turn on "Debug mode (choose chapter)" in the settings. The main menu and the pause menu then
offer **"Choose Chapter (Debug)"** with every chapter of Lyn's, Eliwood's and Hector's tale.
Choose whether the units come from a save file ("Restart Chapter") or a new game starts with
only the chapter's own units. The chapter starts with its own scenes. The chosen save file is
overwritten when the chapter starts; the story mode stays that of the save file.

## Building from source

Requirements: [Rust](https://rustup.rs) (stable) and [Godot 4.5.1](https://godotengine.org/download/archive/4.5.1-stable/).

```sh
tools/build.sh                     # builds the game core into godot/bin/
godot --path godot                 # runs the game from source
```

Optionally faster: `tools/build.sh <your-rom.gba>` translates the code of your ROM to Rust
(`core/generated/`) and links it in. **Such builds contain game code and must not be
shared** – `core/generated/` is therefore ignored by git, and `tools/package.sh` refuses
such builds. Without ROM code everything runs on the interpreter with identical game
behaviour (> 500 frames/s on an ordinary PC).

Building a release package:

```sh
tools/build.sh
tools/package.sh /path/to/Godot_v4.5.1-stable_linux.x86_64 1.0.0   # -> dist/
```

Pushing a git tag `v*` also builds the package with GitHub Actions and attaches it to a release.

## Layout

```
core/            game core in Rust (Cargo workspace)
  crates/gba     GBA platform: CPU (ARM/Thumb), graphics, sound, DMA, timers, BIOS replacement
  crates/fe7hd   GDExtension (class Fe7Core) - the interface to the Godot project
  crates/recomp  static recompiler ROM -> Rust (local builds only)
  crates/fe7gen  placeholder for the recompiled code
  crates/fe7     standalone 2D frontend for testing
  traces/        block addresses for the recompiler (no game data)
godot/           Godot project: scripts, shaders, fonts
  ui/frames.json blueprint of the menu frames - only addresses in the ROM, the art comes from the ROM
tools/           build and package scripts, release templates
third_party/     licenses of the third-party components (Godot, godot-rust, fonts ...)
```

More on how it works: [core/README.md](core/README.md).

### Development

Automated tests without a screen (e.g. with `xvfb-run`):

```sh
FE7HD_SHOTS=/tmp/shots FE7HD_TEST="20:launchset:/path/rom.gba;26:launchplay;300:shot:menu;301:quit" \
  godot --path godot --resolution 1280x720
```

`FE7HD_TEST` is a list of `frame:command:argument`, `FE7HD_SHOTS` the folder for screenshots,
`FE7HD_LAUNCHER=1` forces the launcher. `chtour:0:69` loads every chapter in turn and saves an
overview of each map (with `FE7HD_TOUR_TYPES=1`: close-ups of every terrain type).

The in-game texts are written in German in the scripts and translated to English through
the `EN` table in `godot/scripts/main.gd`; English is the default language.

## Contributing

Never commit ROMs, save files, screenshots with game graphics, code generated from the ROM
or any other data taken from the game.

## License

The project's own code is under the [MIT license](LICENSE). Third-party components are under
their own licenses, see [third_party/OVERVIEW.txt](third_party/OVERVIEW.txt).
