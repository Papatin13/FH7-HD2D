FE7-HD2D - unofficial HD-2D remake
==================================

FE7-HD2D plays Fire Emblem (GBA) as an HD-2D remake with mouse and keyboard:
the maps as a 3D diorama on a war-room table, menus redrawn sharp.

This package contains NO game data. You need your own ROM of Fire Emblem
(Game Boy Advance, US version), for example a backup of your own cartridge.
Downloading ROMs from the internet is not legal in many countries.

Not affiliated with Nintendo or Intelligent Systems. "Fire Emblem" is a
trademark of Nintendo.

Starting
--------
./start.sh   (or double-click -> "Run as program")

On the first start the launcher appears:
  * Click "Choose ROM ..." or drop the .gba file onto the window.
  * The ROM is checked (only the unmodified US ROM fits, CRC32 2A524221).
  * "Play" - the ROM is remembered, from then on the game starts directly.
If the ROM lies in the folder above this one, it is suggested already.

The save file (.sav) is stored next to the ROM.
Settings: ~/.local/share/godot/app_userdata/FE7-HD2D/
To choose another ROM: delete settings.cfg there, or start with
FE7HD_LAUNCHER=1 ./start.sh

Controls (mouse and keyboard)
-----------------------------
Mouse: click everything (map, menus, lists), right click = back,
       wheel = scroll / camera zoom, drag with the middle button = turn the camera.
Keyboard: arrows/WASD move, Enter/Space confirm, Esc/Backspace back,
       Esc on the free map = pause, Q/Tab next unit, E/I unit info,
       F5/F8 quick save/load, F11 fullscreen, hold Ctrl = fast forward.

Debug mode: Settings -> "Debug mode (choose chapter)", then
"Choose Chapter (Debug)" in the main menu or the pause menu loads any chapter.

Contents
--------
engine/godot          Godot 4.5.1 (Linux x86-64)
game/                 Godot project (scripts, shaders, fonts)
game/bin/libfe7hd.so  game core (interpreter, contains no game code)
licenses/             licenses of the third-party components
LICENSE.txt           MIT license of the project's own code
Source code:          https://github.com/Papatin13/FH7-HD2D
