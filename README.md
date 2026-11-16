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

Download the latest version from **Releases**:

| | |
|---|---|
| `FE7-HD2D-…-x86_64.AppImage` | Linux: make it executable (`chmod +x`, or Properties → "Allow executing") and double-click it |
| `FE7-HD2D-…-windows-x86_64.zip` | Windows: unpack and start `FE7-HD2D.exe`. It is not signed, so SmartScreen may warn: "More info" → "Run anyway" |
| `FE7-HD2D-…-linux-x86_64.tar.gz` | Linux as a plain folder: unpack and start `FE7-HD2D.x86_64` |

Put your ROM next to the game and it is found automatically; otherwise the launcher asks
for it (choose it or drop it onto the window). Only the unmodified US ROM is accepted
(game code `AE7E`, CRC32 `2A524221`).

The save file (`.sav`) is stored next to the ROM. Settings: `~/.local/share/godot/app_userdata/FE7-HD2D/`
(Linux), `%APPDATA%\Godot\app_userdata\FE7-HD2D\` (Windows). To open the launcher again,
start the game with the environment variable `FE7HD_LAUNCHER=1`.

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

Building the release files (AppImage, Windows zip, Linux tar.gz):

```sh
tools/build.sh                     # Linux core
tools/build.sh --windows           # Windows core, cross-compiled (rustup target + MinGW)
python3 tools/fetch_templates.py   # only the needed Godot export templates (~300 MB)
tools/package.sh /path/to/Godot_v4.5.1-stable_linux.x86_64 1.0.0   # -> dist/
```

Pushing a git tag `v*` does all of this on GitHub Actions (the Windows core is built there
with MSVC) and attaches the three files to a release.

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
tools/           build, export and package scripts, AppImage files, release readme
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
