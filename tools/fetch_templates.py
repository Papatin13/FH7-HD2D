#!/usr/bin/env python3
"""Fetches only the needed Godot export templates (not the whole 1.3 GB archive).

    tools/fetch_templates.py [linux] [windows]

Reads the zip's directory over HTTP range requests and downloads just the
templates for the given platforms into Godot's template folder.
"""
import io, os, sys, zipfile, urllib.request

VER = "4.5.1"
URL = f"https://github.com/godotengine/godot/releases/download/{VER}-stable/Godot_v{VER}-stable_export_templates.tpz"
FILES = {
    "linux": ["templates/linux_release.x86_64", "templates/linux_debug.x86_64"],
    "windows": ["templates/windows_release_x86_64.exe", "templates/windows_debug_x86_64.exe"],
}


class RangeFile(io.RawIOBase):
    def __init__(self, url):
        self.url = urllib.request.urlopen(urllib.request.Request(url, method="HEAD")).geturl()
        with urllib.request.urlopen(urllib.request.Request(self.url, method="HEAD")) as r:
            self.size = int(r.headers["Content-Length"])
        self.pos = 0

    def seekable(self):
        return True

    def readable(self):
        return True

    def tell(self):
        return self.pos

    def seek(self, off, whence=0):
        self.pos = off if whence == 0 else self.pos + off if whence == 1 else self.size + off
        return self.pos

    def read(self, n=-1):
        if n < 0:
            n = self.size - self.pos
        if n == 0 or self.pos >= self.size:
            return b""
        end = min(self.size, self.pos + n) - 1
        req = urllib.request.Request(self.url, headers={"Range": f"bytes={self.pos}-{end}"})
        with urllib.request.urlopen(req) as r:
            data = r.read()
        self.pos += len(data)
        return data

    def readinto(self, b):
        data = self.read(len(b))
        b[: len(data)] = data
        return len(data)


def template_dir():
    if sys.platform.startswith("win"):
        base = os.path.join(os.environ["APPDATA"], "Godot")
    elif sys.platform == "darwin":
        base = os.path.expanduser("~/Library/Application Support/Godot")
    else:
        base = os.path.join(os.environ.get("XDG_DATA_HOME", os.path.expanduser("~/.local/share")), "godot")
    return os.path.join(base, "export_templates", f"{VER}.stable")


def main():
    plats = sys.argv[1:] or ["linux", "windows"]
    out = template_dir()
    os.makedirs(out, exist_ok=True)
    z = zipfile.ZipFile(io.BufferedReader(RangeFile(URL), buffer_size=1 << 20))
    for name in ["templates/version.txt"] + [f for p in plats for f in FILES[p]]:
        dst = os.path.join(out, os.path.basename(name))
        if os.path.exists(dst) and os.path.getsize(dst) == z.getinfo(name).file_size:
            continue
        print("fetching", name, z.getinfo(name).file_size // 1048576, "MB")
        with z.open(name) as src, open(dst, "wb") as f:
            while True:
                chunk = src.read(1 << 22)
                if not chunk:
                    break
                f.write(chunk)
        if not name.endswith(".exe") and not name.endswith(".txt"):
            os.chmod(dst, 0o755)
    print("templates in", out)


if __name__ == "__main__":
    main()
