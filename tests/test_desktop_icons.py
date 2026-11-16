import importlib.util
import os
from pathlib import Path
import plistlib
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from desktop_icons import ARTWORK

spec = importlib.util.spec_from_file_location("desktop_launcher", ROOT / "scripts/create-desktop-launcher.py")
launcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launcher)


class DesktopIconTests(unittest.TestCase):
    def test_all_platform_artwork(self):
        for tool, relative in ARTWORK.items():
            with self.subTest(tool=tool):
                folder = ROOT / relative
                png = (folder / "app-icon.png").read_bytes()
                self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
                self.assertEqual(struct.unpack(">II", png[16:24]), (256, 256))
                ico = (folder / "app-icon.ico").read_bytes()
                self.assertEqual(struct.unpack("<HHH", ico[:6]), (0, 1, 7))
                sizes = set()
                for index in range(7):
                    width, height, _, _, _, _, length, offset = struct.unpack(
                        "<BBBBHHII", ico[6 + index * 16:22 + index * 16])
                    self.assertEqual(width, height)
                    self.assertLessEqual(offset + length, len(ico))
                    sizes.add(width or 256)
                self.assertEqual(sizes, {16, 24, 32, 48, 64, 128, 256})
                icns = (folder / "app-icon.icns").read_bytes()
                self.assertEqual(icns[:4], b"icns")
                self.assertEqual(struct.unpack(">I", icns[4:8])[0], len(icns))
                tags = []
                offset = 8
                while offset < len(icns):
                    tag, length = struct.unpack(">4sI", icns[offset:offset + 8])
                    self.assertGreater(length, 8)
                    self.assertEqual(icns[offset + 8:offset + 16], png[:8])
                    tags.append(tag)
                    offset += length
                self.assertEqual(offset, len(icns))
                self.assertEqual(tags, [b"ic07", b"ic08", b"ic09", b"ic10"])

    def test_launchers_and_no_overwrite(self):
        with tempfile.TemporaryDirectory(prefix="icons é ") as temporary:
            root = Path(temporary)
            binary = root / 'player " $ ` % \\ test'
            binary.write_text('#!/bin/sh\nprintf "%s" "$1"\n')
            binary.chmod(0o755)
            for tool in launcher.APPS:
                with self.subTest(tool=tool):
                    app = launcher.create(tool, binary, root / "mac", "macos")
                    with (app / "Contents/Info.plist").open("rb") as stream:
                        metadata = plistlib.load(stream)
                    self.assertTrue((app / "Contents/Resources" / metadata["CFBundleIconFile"]).is_file())
                    command = app / "Contents/MacOS" / metadata["CFBundleExecutable"]
                    self.assertFalse(command.samefile(binary))
                    self.assertEqual(command.read_bytes(), binary.read_bytes())
                    result = subprocess.check_output([str(command), "argument with spaces"])
                    self.assertEqual(result, b"argument with spaces")
                    desktop = launcher.create(tool, binary, root / "linux", "linux")
                    self.assertIn("%%", desktop.read_text())
                    if shutil.which("desktop-file-validate"):
                        subprocess.run(["desktop-file-validate", str(desktop)], check=True)
                    for platform in ("macos", "linux"):
                        with self.assertRaises(FileExistsError):
                            launcher.create(tool, binary, root / ("mac" if platform == "macos" else "linux"), platform)
                    self.assertTrue(os.access(command, os.X_OK))

    def test_missing_binary_does_not_create_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaises(FileNotFoundError):
                launcher.create("canvas-pilot", root / "missing", root / "out", "linux")
            self.assertFalse((root / "out").exists())
