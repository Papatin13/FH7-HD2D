#!/bin/sh
# Assembles a release folder + archive in dist/ from the current build.
#
#   tools/package.sh <path-to-godot-4.5.1-binary> [version]
#
# Refuses builds that contain game code (tools/build.sh <rom>).
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="$1"
VER="${2:-dev}"
[ -x "$GODOT" ] || { echo "usage: tools/package.sh <godot-binary> [version]"; exit 1; }
[ -f "$ROOT/godot/bin/libfe7hd.so" ] || { echo "first run tools/build.sh"; exit 1; }
if [ -f "$ROOT/godot/bin/.contains-rom-code" ]; then
    echo "godot/bin/libfe7hd.so contains code from your ROM - rebuild with plain tools/build.sh"
    exit 1
fi
NAME="FE7-HD2D-$VER-linux-x86_64"
OUT="$ROOT/dist/$NAME"
rm -rf "$OUT"
mkdir -p "$OUT/engine" "$OUT/game/bin" "$OUT/game/.godot" "$OUT/game/ui" "$OUT/licenses"
cp "$GODOT" "$OUT/engine/godot"
chmod +x "$OUT/engine/godot"
cd "$ROOT/godot"
cp -r scripts shaders fonts main.tscn project.godot fe7.gdextension "$OUT/game/"
cp ui/frames.json "$OUT/game/ui/"
cp .godot/extension_list.cfg "$OUT/game/.godot/"
cp bin/libfe7hd.so "$OUT/game/bin/"
strip "$OUT/game/bin/libfe7hd.so" 2>/dev/null || true
find "$OUT/game" \( -name "*.import" -o -name "*.png" \) -delete
cp "$ROOT"/third_party/* "$OUT/licenses/"
cp "$ROOT/LICENSE" "$OUT/LICENSE.txt"
cp "$ROOT/tools/release/README.txt" "$ROOT/tools/release/start.sh" "$OUT/"
chmod +x "$OUT/start.sh"
cd "$ROOT/dist"
tar czf "$NAME.tar.gz" "$NAME"
echo "done: dist/$NAME.tar.gz"
