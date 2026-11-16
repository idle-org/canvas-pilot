#!/usr/bin/env python3
"""Create an icon-bearing Linux/BSD desktop entry or macOS app launcher."""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import sys
import tempfile

from desktop_icons import APPS, ROOT


def desktop_value(value):
    return str(value).replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")


def desktop_exec(path):
    # Exec has a second escaping layer inside double quotes, plus field codes.
    value = str(path).replace("%", "%%")
    for character in ('\\', '"', '`', '$'):
        value = value.replace(character, "\\" + character)
    return desktop_value('"' + value + '"')


def create(tool, binary, output, platform):
    title, assets, _ = APPS[tool]
    binary = binary.resolve(strict=True)
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError(f"Not an executable file: {binary}")
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    assets = ROOT / assets
    if not assets.is_dir():
        assets = ROOT / "icons" / tool
    if platform == "macos":
        destination = output / f"{title}.app"
        if destination.exists() or destination.is_symlink():
            raise FileExistsError(destination)
        with tempfile.TemporaryDirectory(dir=output) as temporary:
            app = Path(temporary) / destination.name
            contents = app / "Contents"
            (contents / "MacOS").mkdir(parents=True)
            (contents / "Resources").mkdir()
            shutil.copyfile(assets / "app-icon.icns", contents / "Resources/app-icon.icns")
            with (contents / "Info.plist").open("wb") as stream:
                plistlib.dump({
                    "CFBundleName": title, "CFBundleDisplayName": title,
                    "CFBundleIdentifier": f"com.beffroi.{tool}",
                    "CFBundlePackageType": "APPL", "CFBundleExecutable": tool,
                    "CFBundleIconFile": "app-icon.icns", "CFBundleVersion": "1",
                    "NSHighResolutionCapable": True,
                }, stream)
            executable = contents / "MacOS" / tool
            shutil.copy2(binary, executable)
            executable.chmod(0o755)
            notices = binary.parent / "LICENSES"
            if notices.is_dir():
                if notices.is_symlink() or any(path.is_symlink() for path in notices.rglob("*")):
                    raise ValueError("Refusing symlinked bundle notices")
                shutil.copytree(notices, contents / "Resources/LICENSES")
            app.rename(destination)
        return destination
    destination = output / "applications" / f"{tool}.desktop"
    icon = output / "icons/hicolor/256x256/apps" / f"{tool}.png"
    if any(path.exists() or path.is_symlink() for path in (destination, icon)):
        raise FileExistsError(f"Launcher or icon already exists for {tool}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    icon.parent.mkdir(parents=True, exist_ok=True)
    with icon.open("xb") as stream:
        stream.write((assets / "app-icon.png").read_bytes())
    with destination.open("x") as stream:
        stream.write(f"[Desktop Entry]\nType=Application\nName={title}\n"
                     f"Exec={desktop_exec(binary)}\nIcon={desktop_value(icon)}\n"
                     f"Terminal=false\nCategories=AudioVideo;Utility;\nStartupWMClass={tool}\n")
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tool", choices=APPS)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--platform", choices=("linux", "macos"),
                        default="macos" if sys.platform == "darwin" else "linux")
    parser.add_argument("--output", type=Path, required=True,
                        help="Applications folder on macOS; data prefix (e.g. ~/.local/share) on Linux/BSD")
    args = parser.parse_args()
    try:
        print(create(args.tool, args.binary, args.output, args.platform))
    except (OSError, ValueError) as error:
        parser.exit(1, f"Cannot create launcher: {error}\n")


if __name__ == "__main__":
    main()
