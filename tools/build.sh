#!/bin/sh
# Builds the game core (Rust GDExtension) and puts it into godot/bin/.
#
#   tools/build.sh                 interpreter only - this build may be shared
#   tools/build.sh --windows       the same for Windows (fe7hd.dll), cross-compiled;
#                                  needs `rustup target add x86_64-pc-windows-gnu`
#                                  and MinGW (Debian/Ubuntu: gcc-mingw-w64-x86-64)
#   tools/build.sh <your-rom.gba>  also translates the game code of YOUR ROM to
#                                  Rust (core/generated/, ignored by git) for
#                                  more speed - such a build contains game code
#                                  and must NOT be shared
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/core"
mkdir -p "$ROOT/godot/bin"
if [ "$1" = "--windows" ]; then
    cargo build --release -p fe7hd --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/release/fe7hd.dll "$ROOT/godot/bin/"
    echo "done: godot/bin/fe7hd.dll"
    exit 0
fi
if [ -n "$1" ]; then
    ROM="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
    cargo build --release -p recomp
    mkdir -p generated
    ./target/release/recomp "$ROM" generated traces/*.txt
    cargo build --release -p fe7hd --features recomp
    touch "$ROOT/godot/bin/.contains-rom-code"
else
    cargo build --release -p fe7hd
    rm -f "$ROOT/godot/bin/.contains-rom-code"
fi
cp target/release/libfe7hd.so "$ROOT/godot/bin/"
echo "done: godot/bin/libfe7hd.so"
