#!/bin/sh
# Exports the game with Godot and builds the release files in dist/:
#   FE7-HD2D-<version>-linux-x86_64.tar.gz
#   FE7-HD2D-<version>-x86_64.AppImage
#   FE7-HD2D-<version>-windows-x86_64.zip   (if godot/bin/fe7hd.dll exists)
#
#   tools/package.sh <path-to-godot-4.5.1-editor> [version]
#
# Needs the Godot export templates (tools/fetch_templates.py) and refuses
# builds that contain game code (tools/build.sh <rom>).
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="$(cd "$(dirname "$1")" 2>/dev/null && pwd)/$(basename "$1")"
VER="${2:-dev}"
[ -x "$GODOT" ] || { echo "usage: tools/package.sh <godot-binary> [version]"; exit 1; }
[ -f "$ROOT/godot/bin/libfe7hd.so" ] || { echo "first run tools/build.sh"; exit 1; }
if [ -f "$ROOT/godot/bin/.contains-rom-code" ]; then
    echo "godot/bin/libfe7hd.so contains code from your ROM - rebuild with plain tools/build.sh"
    exit 1
fi
DIST="$ROOT/dist"
rm -rf "$DIST/linux" "$DIST/windows" "$DIST/AppDir"
mkdir -p "$DIST/linux" "$DIST/windows"
"$GODOT" --headless --path "$ROOT/godot" --import >/dev/null 2>&1 || true

# common files of every package: readme, licenses
extras() {
    cp "$ROOT/tools/release/README.txt" "$1/"
    cp "$ROOT/LICENSE" "$1/LICENSE.txt"
    mkdir -p "$1/licenses"
    cp "$ROOT"/third_party/* "$1/licenses/"
}

# --- Linux: folder + tar.gz ---
NAME="FE7-HD2D-$VER-linux-x86_64"
"$GODOT" --headless --path "$ROOT/godot" --export-release "Linux" "$DIST/linux/FE7-HD2D.x86_64"
strip "$DIST/linux/libfe7hd.so" 2>/dev/null || true
rm -rf "$DIST/$NAME"
mkdir -p "$DIST/$NAME"
cp "$DIST/linux/FE7-HD2D.x86_64" "$DIST/linux/libfe7hd.so" "$DIST/$NAME/"
extras "$DIST/$NAME"
(cd "$DIST" && tar czf "$NAME.tar.gz" "$NAME")
echo "done: dist/$NAME.tar.gz"

# --- Linux: AppImage ---
APPDIR="$DIST/AppDir"
mkdir -p "$APPDIR/usr/bin"
cp "$DIST/linux/FE7-HD2D.x86_64" "$DIST/linux/libfe7hd.so" "$APPDIR/usr/bin/"
extras "$APPDIR/usr/bin"
cp "$ROOT/tools/appimage/AppRun" "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"
cp "$ROOT/tools/appimage/fe7-hd2d.desktop" "$APPDIR/"
cp "$ROOT/tools/appimage/fe7-hd2d.png" "$APPDIR/"
cp "$ROOT/tools/appimage/fe7-hd2d.png" "$APPDIR/.DirIcon"
TOOL="$DIST/appimagetool"
if [ ! -x "$TOOL" ]; then
    curl -sSL -o "$TOOL" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
fi
ARCH=x86_64 "$TOOL" --appimage-extract-and-run --no-appstream "$APPDIR" "$DIST/FE7-HD2D-$VER-x86_64.AppImage" >/dev/null
echo "done: dist/FE7-HD2D-$VER-x86_64.AppImage"

# --- Windows: zip ---
if [ -f "$ROOT/godot/bin/fe7hd.dll" ]; then
    WNAME="FE7-HD2D-$VER-windows-x86_64"
    "$GODOT" --headless --path "$ROOT/godot" --export-release "Windows" "$DIST/windows/FE7-HD2D.exe"
    rm -rf "$DIST/$WNAME"
    mkdir -p "$DIST/$WNAME"
    cp "$DIST/windows/FE7-HD2D.exe" "$DIST/windows/fe7hd.dll" "$DIST/$WNAME/"
    extras "$DIST/$WNAME"
    (cd "$DIST" && rm -f "$WNAME.zip" && python3 -c "import shutil; shutil.make_archive('$WNAME', 'zip', '.', '$WNAME')")
    echo "done: dist/$WNAME.zip"
else
    echo "skipped Windows: no godot/bin/fe7hd.dll (tools/build.sh --windows)"
fi
