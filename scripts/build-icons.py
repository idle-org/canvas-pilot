#!/usr/bin/env python3
"""Regenerate committed PNG, ICO and ICNS icons using ImageMagick 7."""
import struct
import subprocess
import tempfile
from pathlib import Path

from desktop_icons import ARTWORK, ROOT


def build(folder):
    source = folder / "app-icon-source.png"
    subprocess.run(["magick", str(source), "-resize", "256x256", str(folder / "app-icon.png")], check=True)
    subprocess.run(["magick", str(source), "-define", "icon:auto-resize=256,128,64,48,32,24,16",
                    str(folder / "app-icon.ico")], check=True)
    chunks = []
    with tempfile.TemporaryDirectory() as temporary:
        for size, tag in [(128, b"ic07"), (256, b"ic08"), (512, b"ic09"), (1024, b"ic10")]:
            png = Path(temporary) / f"{size}.png"
            subprocess.run(["magick", str(source), "-resize", f"{size}x{size}", str(png)], check=True)
            data = png.read_bytes()
            chunks.append(tag + struct.pack(">I", len(data) + 8) + data)
    data = b"".join(chunks)
    (folder / "app-icon.icns").write_bytes(b"icns" + struct.pack(">I", len(data) + 8) + data)


if __name__ == "__main__":
    for relative in ARTWORK.values():
        build(ROOT / relative)
