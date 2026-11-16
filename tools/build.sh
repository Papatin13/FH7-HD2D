#!/bin/sh
# Builds the game core (Rust GDExtension) and puts it into godot/bin/.
#
#   tools/build.sh                 interpreter only - this build may be shared
#   tools/build.sh <your-rom.gba>  also translates the game code of YOUR ROM to
#                                  Rust (core/generated/, ignored by git) for
#                                  more speed - such a build contains game code
#                                  and must NOT be shared
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/core"
mkdir -p "$ROOT/godot/bin"
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
echo "fertig: godot/bin/libfe7hd.so"
