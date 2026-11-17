#!/usr/bin/env python3
"""Record isolated Canvas Pilot feature clips. See docs/canvas-pilot/feature-demo-checklist.md.

Requires a dedicated 1440x940 X11 display, ffmpeg/ffprobe, ImageMagick, tesseract,
xdotool and xclip. Never point --display at your working desktop. Generated files
and fresh per-shot profiles stay under the ignored showcase-videos directory.
"""

import argparse
import base64
import csv
import html
import io
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
import zipfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "showcase-videos"


def run(args, **kwargs):
    kwargs.setdefault("timeout", 60)
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def baseline():
    p = json.loads((ROOT / "docs/interfaces/canvas-pilot-v18.example.json").read_text())
    p["folders"] = []
    p["input_proposals"] = []
    p["editor"].update(
        backdrop_title="Canvas Pilot · feature showcase",
        selected_layer_ids=[1],
        position_point="top_left",
        position_layer_point=None,
        viewport_zoom=1,
        viewport_pan=[0, 0],
        outline_layers=False,
        show_source_pattern=False,
    )
    for i, layer in enumerate(p["layers"]):
        layer.update(
            folder="",
            name=["Slides", "Presenter"][i],
            device_link=None,
            layer_fit=None,
            visible=True,
            locked=False,
            pivot=[0.5, 0.5],
        )
        layer["advanced_crop"] = dict.fromkeys(["left", "right", "top", "bottom"], 0.0)
        layer["mask"] = dict(origin="layer", left=0.0, right=0.0, top=0.0, bottom=0.0)
    return p


def prepare_assets():
    assets = OUT / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    svg = """<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080">
<rect width="1920" height="1080" fill="#142c42"/>
<rect x="60" y="60" width="1800" height="960" rx="30" fill="#21536a" stroke="#7fe2e6" stroke-width="8"/>
<g fill="#f2f5fb" font-family="DejaVu Sans" text-anchor="middle">
<text x="960" y="180" font-size="64">CANVAS PILOT</text><text x="960" y="260" font-size="35">Offline demonstration image · 1920 × 1080</text></g>
<rect x="150" y="350" width="460" height="450" rx="25" fill="#ecb550"/>
<rect x="730" y="350" width="460" height="450" rx="25" fill="#8274df"/>
<rect x="1310" y="350" width="460" height="450" rx="25" fill="#4ebbac"/>
<g fill="#142c42" font-family="DejaVu Sans" font-size="110" text-anchor="middle"><text x="380" y="620">1</text><text x="960" y="620">2</text><text x="1540" y="620">3</text></g>
<path d="M960 20V1060M20 540H1900" stroke="#f2f5fb" opacity=".35" stroke-width="4"/>
<text x="960" y="930" text-anchor="middle" font-family="DejaVu Sans" font-size="38" fill="white">Crop • Mask • Fit • Frame</text></svg>"""
    (assets / "source.svg").write_text(svg)
    run(
        ["convert", "-background", "none", assets / "source.svg", assets / "source.png"]
    )
    (assets / "geometry.svg").write_text(
        """<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080"><rect id="Main" x="80" y="100" width="1000" height="562.5" fill="#425a7c"/><rect id="Speaker" x="1200" y="250" width="420" height="600" fill="#96783a"/></svg>"""
    )
    with zipfile.ZipFile(ROOT / "docs/language-packs/language-pack.zip") as source:
        with zipfile.ZipFile(
            assets / "language-pack-revision-2.zip", "w", zipfile.ZIP_DEFLATED
        ) as output:
            for name in source.namelist():
                data = source.read(name)
                if name == "manifest.json":
                    manifest = json.loads(data)
                    manifest["revision"] = 2
                    data = json.dumps(manifest, ensure_ascii=False).encode()
                output.writestr(name, data)
    return assets


class Studio:
    def __init__(self, args):
        self.args = args
        self.env = os.environ.copy()
        self.env.pop("WAYLAND_DISPLAY", None)
        self.env.update(
            DISPLAY=args.display,
            WINIT_UNIX_BACKEND="x11",
            WINIT_X11_SCALE_FACTOR="1",
            XDG_CURRENT_DESKTOP="GNOME",
            XDG_SESSION_TYPE="x11",
            GTK_USE_PORTAL="0",
            GDK_BACKEND="x11",
        )
        if args.tools:
            self.env["PATH"] = (
                str(Path(args.tools) / "usr/bin") + ":" + self.env["PATH"]
            )
            self.env["LD_LIBRARY_PATH"] = str(Path(args.tools) / "usr/lib")
            self.env["TESSDATA_PREFIX"] = str(Path(args.tools) / "usr/share/tessdata")
        self.pos = (1000, 800)
        self.app = None
        self.recorder = None
        self.events = []
        self.started = 0
        self.folder = OUT
        self.assets = prepare_assets()
        self.bus = None
        self.portal_processes = []

    def cmd(self, *args, **kw):
        if kw.pop("capture_output", False):
            kw.setdefault("stdout", subprocess.PIPE)
            kw.setdefault("stderr", subprocess.PIPE)
        else:
            kw.setdefault("stdout", subprocess.DEVNULL)
            kw.setdefault("stderr", subprocess.DEVNULL)
        return run(args, env=self.env, **kw)

    def portals(self):
        result = self.cmd(
            "dbus-daemon",
            "--session",
            "--fork",
            "--print-address=1",
            "--print-pid=1",
            capture_output=True,
            text=True,
        ).stdout.splitlines()
        self.env["DBUS_SESSION_BUS_ADDRESS"] = result[0]
        self.bus = int(result[1])
        config = OUT / "portal-config"
        (config / "xdg-desktop-portal").mkdir(parents=True, exist_ok=True)
        (config / "xdg-desktop-portal/portals.conf").write_text(
            "[preferred]\ndefault=gtk\n"
        )
        penv = dict(
            self.env,
            XDG_CONFIG_HOME=str(config),
            XDG_DATA_HOME=str(OUT / "portal-data"),
            XDG_CACHE_HOME=str(OUT / "portal-cache"),
        )
        for executable in [
            "/usr/lib/xdg-desktop-portal-gtk",
            "/usr/lib/xdg-desktop-portal",
        ]:
            log = open(OUT / (Path(executable).name + ".log"), "w")
            self.portal_processes.append(
                subprocess.Popen(
                    [executable], env=penv, stdout=log, stderr=subprocess.STDOUT
                )
            )
        time.sleep(2)

    def stop_app(self):
        if self.app and self.app.poll() is None:
            self.app.terminate()
            try:
                self.app.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.app.kill()
                self.app.wait()
        self.app = None

    def start(self, slug, variant="default"):
        self.stop_app()
        self.folder = OUT / slug
        self.folder.mkdir(parents=True, exist_ok=True)
        (self.folder / "shot.json").unlink(missing_ok=True)
        profile = self.folder / "profile"
        if profile.exists():
            shutil.rmtree(profile)
        config = profile / "config/midra-layout"
        config.mkdir(parents=True)
        settings = json.loads(
            (
                ROOT / "docs/interfaces/canvas-pilot-settings-v14.example.json"
            ).read_text()
        )
        settings.update(
            language="en",
            autoconnect_at_startup=False,
            midra_host="",
            recent_midra_hosts=[],
        )
        p = baseline()
        if variant in ["crop", "mask", "fits", "preview", "mask-snap"]:
            image = base64.b64encode((self.assets / "source.png").read_bytes()).decode()
            p["workspace"]["previews"] = [
                dict(
                    layer_id=l["id"],
                    source_id=l["source_id"],
                    input=i + 1,
                    host="",
                    enabled=True,
                    automatic=False,
                    guide_opacity=0.5,
                    name="Offline demonstration image",
                    image_base64=image,
                )
                for i, l in enumerate(p["layers"])
            ]
        if variant in ["crop", "mask", "preview"]:
            p["layers"][1]["advanced_crop"] = dict(
                left=480.0, right=480.0, top=270.0, bottom=270.0
            )
        if variant == "mask":
            p["layers"][1]["mask"].update(left=0.15, right=0.15, top=0.1, bottom=0.1)
        if variant == "fits":
            p["layers"][1]["bounds"].update(width=500.0, height=500.0)
            p["layers"][1]["layer_fit"] = "centered"
        if variant == "mask-snap":
            p["layers"][0]["mask"].update(right=0.25)
            p["workspace"]["snapping"].update(
                enabled=True,
                pixels=False,
                screen_edges=False,
                layer_edges=False,
                mask_edges=True,
            )
        if variant == "reference":
            p["references"] = [
                dict(
                    name="Stage plan — offline reference",
                    image_base64=base64.b64encode(
                        (self.assets / "source.png").read_bytes()
                    ).decode(),
                    visible=True,
                    opacity=0.65,
                    preserve_ratio=True,
                    native_size=False,
                    offset=[0.0, 0.0],
                )
            ]
            p["editor"]["outline_layers"] = True
        if variant == "organize":
            p["folders"] = ["Presentation"]
            p["layers"][0]["folder"] = "Presentation"
        if variant == "import":
            p["editor"]["selected_layer_ids"] = [1, 2]
        if variant == "language":
            packs = config / "language-packs"
            packs.mkdir()
            shutil.copyfile(
                ROOT / "docs/language-packs/language-pack.zip",
                packs / "example-uk-bho.zip",
            )
        if variant == "device":
            settings["midra_host"] = "127.0.0.1:3000"
        (config / "settings.json").write_text(json.dumps(settings))
        (config / "last_project.json").write_text(
            json.dumps(
                dict(version=1, project=p, project_path=None, manual_saved_at=None)
            )
        )
        (self.folder / "starting-project.json").write_text(
            json.dumps(p, indent=2) + "\n"
        )
        self.env.update(
            XDG_CONFIG_HOME=str(profile / "config"), XDG_DATA_HOME=str(profile / "data")
        )
        self.app = subprocess.Popen(
            [str(ROOT / "target/debug/canvas-pilot")],
            env=self.env,
            stdout=open(self.folder / "app.log", "w"),
            stderr=subprocess.STDOUT,
        )
        time.sleep(3)
        ids = self.cmd(
            "xdotool",
            "search",
            "--name",
            "^Canvas Pilot$",
            capture_output=True,
            text=True,
        ).stdout.split()
        self.window = ids[-1]
        self.cmd("xdotool", "windowmove", self.window, "0", "0")
        self.cmd("xdotool", "windowsize", self.window, "1440", "940")
        self.cmd("xdotool", "windowfocus", self.window)
        self.move(1000, 800, 0.1)
        self.capture("start.png")

    def move(self, x, y, duration=0.45):
        start = self.pos
        begun = time.monotonic()
        while True:
            f = min(1, (time.monotonic() - begun) / duration)
            e = (1 - math.cos(math.pi * f)) / 2
            self.cmd(
                "xdotool",
                "mousemove",
                str(round(start[0] + (x - start[0]) * e)),
                str(round(start[1] + (y - start[1]) * e)),
                stdout=subprocess.DEVNULL,
            )
            if f == 1:
                break
            time.sleep(1 / 40)
        self.pos = (x, y)

    def click(self, x, y, button=1, pause=0.65):
        self.move(x, y)
        self.cmd("xdotool", "mousedown", str(button))
        time.sleep(0.1)
        self.cmd("xdotool", "mouseup", str(button))
        time.sleep(pause)

    def drag(self, x, y, tx, ty):
        self.move(x, y)
        self.cmd("xdotool", "mousedown", "1")
        self.move(tx, ty, 1.4)
        self.cmd("xdotool", "mouseup", "1")
        time.sleep(0.8)

    def key(self, key):
        self.cmd("xdotool", "keydown", key)
        time.sleep(0.12)
        self.cmd("xdotool", "keyup", key)
        time.sleep(0.45)

    def type(self, text):
        self.clipboard(text)
        self.key("ctrl+v")

    def field(self, x, y, value):
        self.click(x, y)
        self.key("ctrl+a")
        self.type(str(value))
        self.key("Return")

    def capture(self, name):
        path = self.folder / name
        self.cmd("import", "-window", "root", path)
        return path

    def find(self, label, region=None, last=False):
        image = self.capture("probe.png")
        raw = self.cmd(
            "tesseract",
            image,
            "stdout",
            "--psm",
            "11",
            "-c",
            "tessedit_create_tsv=1",
            capture_output=True,
            text=True,
            stderr=subprocess.DEVNULL,
        ).stdout
        groups = {}
        for row in csv.DictReader(io.StringIO(raw), delimiter="\t"):
            if not row["text"].strip():
                continue
            x, y, w, h = [int(row[k]) for k in ["left", "top", "width", "height"]]
            if region and not (
                region[0] <= x < region[2] and region[1] <= y < region[3]
            ):
                continue
            groups.setdefault(
                tuple(row[k] for k in ["block_num", "par_num", "line_num"]), []
            ).append((row["text"], x, y, w, h))
        norm = lambda s: re.sub(r"[^a-z0-9]", "", s.lower())
        query = norm(label)
        matches = []
        for words in groups.values():
            for i in range(len(words)):
                text = ""
                for j in range(i, len(words)):
                    text += norm(words[j][0])
                    if text == query:
                        segment = words[i : j + 1]
                        x = min(w[1] for w in segment)
                        y = min(w[2] for w in segment)
                        right = max(w[1] + w[3] for w in segment)
                        bottom = max(w[2] + w[4] for w in segment)
                        matches.append(
                            (
                                i == 0 and j == len(words) - 1,
                                ((x + right) / 2, (y + bottom) / 2),
                            )
                        )
                    if len(text) > len(query):
                        break
        if not matches:
            raise RuntimeError(f"Cannot find {label!r}; inspect {image}")
        exact = [point for whole, point in matches if whole]
        points = exact or [point for _, point in matches]
        return points[-1 if last else 0]

    def label(self, text, region=None, last=False, pause=0.65):
        self.click(*self.find(text, region, last), pause=pause)

    def expect_absent(self, text):
        try:
            self.find(text)
        except RuntimeError:
            return
        raise RuntimeError(f"{text!r} is still visible; action did not finish")

    def hover(self, text):
        self.move(*self.find(text))
        time.sleep(1)

    def note(self, text, pause=1):
        self.events.append((time.monotonic() - self.started, text))
        time.sleep(pause)

    def clipboard(self, text):
        self.cmd("xclip", "-selection", "clipboard", "-i", input=text.encode())

    def dialog(self, path, save=False):
        time.sleep(1)
        windows = self.cmd(
            "xdotool",
            "search",
            "--onlyvisible",
            "--class",
            ".",
            capture_output=True,
            text=True,
        ).stdout.split()
        picker = None
        for window in windows:
            if window == self.window:
                continue
            geometry = self.cmd(
                "xdotool",
                "getwindowgeometry",
                "--shell",
                window,
                capture_output=True,
                text=True,
            ).stdout
            dimensions = dict(re.findall(r"(WIDTH|HEIGHT)=(\d+)", geometry))
            if (
                int(dimensions.get("WIDTH", 0)) > 500
                and int(dimensions.get("HEIGHT", 0)) > 300
            ):
                picker = window
                break
        if picker is None:
            raise RuntimeError("Native file picker did not appear")
        if picker:
            self.cmd("xdotool", "windowmove", picker, "100", "100")
            self.cmd("xdotool", "windowsize", picker, "1100", "740")
            self.cmd("xdotool", "windowfocus", picker)
        self.key("ctrl+l")
        self.key("ctrl+a")
        self.type(str(path))
        self.key("Return")
        time.sleep(1)
        if save:
            self.key("alt+s")
        self.capture("file-dialog-result.png")
        time.sleep(1)
        self.cmd("xdotool", "windowfocus", self.window)

    def properties(self, tab):
        self.click(1240, 300)
        self.click(
            {
                "Source": 48,
                "Layer": 106,
                "Crop": 156,
                "Mask": 208,
                "Mapping": 269,
                "Preview": 343,
            }[tab],
            71,
        )
        self.click(
            {
                "Source": 48,
                "Layer": 106,
                "Crop": 156,
                "Mask": 208,
                "Mapping": 269,
                "Preview": 343,
            }[tab],
            71,
        )

    def close_window(self):
        self.click(537, 35)

    def record(self, slug, title, variant, actions):
        self.start(slug, variant)
        (self.folder / "shot.json").unlink(missing_ok=True)
        self.events = []
        self.started = time.monotonic()
        video = self.folder / "video.mp4"
        for name in ["layout.svg", "showcase.pilot.json", "showcase.pilot.json.json"]:
            (self.folder / name).unlink(missing_ok=True)
        self.recorder = subprocess.Popen(
            [
                "ffmpeg",
                "-hide_banner",
                "-y",
                "-f",
                "x11grab",
                "-draw_mouse",
                "1",
                "-framerate",
                "30",
                "-video_size",
                "1440x940",
                "-i",
                self.args.display + ".0",
                "-an",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "18",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
                str(video),
            ],
            env=self.env,
            stdin=subprocess.PIPE,
            stdout=open(self.folder / "record.log", "w"),
            stderr=subprocess.STDOUT,
        )
        ok = False
        try:
            self.note(title, 2)
            actions(self)
            time.sleep(1.5)
            self.capture("end.png")
            ok = True
        finally:
            if not ok:
                try:
                    self.capture("failure.png")
                except subprocess.SubprocessError:
                    pass
            try:
                self.recorder.communicate(b"q\n", timeout=20)
            except subprocess.TimeoutExpired:
                self.recorder.kill()
                self.recorder.wait()
            self.recorder = None
            self.stop_app()
            recovery = self.folder / "profile/config/midra-layout/last_project.json"
            if recovery.exists():
                (self.folder / "ending-project.json").write_text(
                    json.dumps(json.loads(recovery.read_text())["project"], indent=2)
                    + "\n"
                )
            if not ok:
                video.rename(self.folder / "FAILED-video.mp4")
        try:
            verify_result(slug, self.folder)
        except (AssertionError, ValueError) as error:
            video.rename(self.folder / "FAILED-video.mp4")
            raise RuntimeError(f"Result verification failed: {error}") from error
        run(["ffmpeg", "-v", "error", "-i", video, "-f", "null", "-"])
        info = json.loads(
            run(
                [
                    "ffprobe",
                    "-v",
                    "error",
                    "-show_entries",
                    "format=duration,size",
                    "-of",
                    "json",
                    video,
                ],
                capture_output=True,
                text=True,
            ).stdout
        )["format"]
        duration = float(info["duration"])
        stamp = (
            lambda s: f"{int(s)//3600:02}:{int(s)//60%60:02}:{int(s)%60:02}.{int(s%1*1000):03}"
        )
        captions = "WEBVTT\n\n"
        for i, (start, text) in enumerate(self.events):
            end = self.events[i + 1][0] if i + 1 < len(self.events) else duration
            captions += f"{stamp(start)} --> {stamp(end)}\n{text}\n\n"
        (self.folder / "captions.vtt").write_text(captions)
        (self.folder / "shot.json").write_text(
            json.dumps(
                dict(
                    id=slug,
                    title=title,
                    variant=variant,
                    duration=duration,
                    bytes=int(info["size"]),
                    events=self.events,
                    status="recorded; awaiting visual review",
                ),
                indent=2,
            )
            + "\n"
        )
        run(
            [
                "ffmpeg",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                str(duration / 2),
                "-i",
                video,
                "-frames:v",
                "1",
                self.folder / "poster.jpg",
            ]
        )
        run(
            [
                "ffmpeg",
                "-loglevel",
                "error",
                "-y",
                "-i",
                video,
                "-vf",
                f"fps=4/{duration},scale=720:-1,tile=2x2",
                "-frames:v",
                "1",
                self.folder / "contact.jpg",
            ]
        )
        print(f"RECORDED {slug}: {duration:.1f}s", flush=True)

    def close(self):
        self.stop_app()
        for process in self.portal_processes:
            process.terminate()
        if self.bus:
            try:
                os.kill(self.bus, 15)
            except ProcessLookupError:
                pass


def verify_result(slug, folder):
    """Check persistent outcomes as well as the screenshots reviewed by a person."""
    start = json.loads((folder / "starting-project.json").read_text())
    end = json.loads((folder / "ending-project.json").read_text())
    before = {layer["id"]: layer for layer in start["layers"]}
    after = {layer["id"]: layer for layer in end["layers"]}
    if slug in {
        "01-project-save-reopen",
        "03-alignment-pivots-measurement",
        "04-group-resizing",
        "12-clipboard-import",
        "13-file-import",
        "19-mask-snapping",
    }:
        assert {key: value["bounds"] for key, value in before.items()} == {
            key: value["bounds"] for key, value in after.items()
        }, "Reopen/undo did not restore the original geometry"
    if slug == "02-move-resize":
        assert after[1]["bounds"]["width"] == 720, "Exact width did not apply"
    if slug == "08-crop":
        assert not any(after[1]["advanced_crop"].values()), "Crop reset did not apply"
    if slug == "09-mask":
        assert all(
            after[1]["mask"][edge] == 0 for edge in ("left", "right", "top", "bottom")
        ), "Mask reset did not apply"
    if slug == "11-fit-modes":
        assert after[1]["layer_fit"] == "one_to_one", "Final 1:1 fit did not apply"
    if slug == "06-layers-folders":
        assert len(after) == 2 and all(
            layer["folder"] == "Presentation" for layer in after.values()
        ), "Folder organization/removal did not finish"
    if slug == "20-language-pack-lifecycle":
        assert not list(
            (folder / "profile/config/midra-layout/language-packs").glob("*.zip")
        ), "Pack removal did not finish"


# Each shot starts from its own documented seed; no shot depends on another clip.
def movement(s):
    s.note("Move the selected layer; the inspector follows its position.")
    s.drag(795, 390, 705, 460)
    s.note("Resize with a corner handle while preserving aspect ratio.")
    s.drag(876, 553, 935, 588)
    s.note("Undo and redo are single editing steps.")
    s.key("ctrl+z")
    s.key("ctrl+shift+z")
    s.note("Enter an exact width, then nudge with arrow keys.")
    s.field(1150, 650, "720")
    s.click(735, 470)
    s.key("Right")
    s.key("shift+Down")


def crop(s):
    s.note("A prepared 200% crop starts this clip; the layer rectangle remains fixed.")
    s.label("Adjust layer crop")
    s.drag(795, 390, 865, 420)
    s.note("Faded content outside the rectangle helps with reframing.")
    s.key("Left")
    s.key("shift+Up")
    s.note("Crop properties offer precise zoom and offset controls.")
    s.properties("Crop")
    s.capture("crop-properties.png")
    s.note("Recenter preserves zoom; Reset crop returns to the minimum framing.")
    s.label("Recenter")
    s.label("Reset crop")
    s.close_window()


def mask(s):
    s.note("The prepared mask hides picture edges; double-click enters crop editing.")
    s.click(795, 390)
    s.key("Escape")
    s.click(795, 390)
    s.cmd("xdotool", "click", "--repeat", "2", "--delay", "120", "1")
    time.sleep(0.8)
    s.note(
        "Tab switches from crop to mask editing. Hatching reveals the hidden picture."
    )
    s.key("Tab")
    s.drag(680, 390, 725, 390)
    s.note("Move the reveal window without changing the layer rectangle.")
    s.drag(795, 390, 825, 410)
    s.note("Numeric Mask controls include origin, centering and reset.")
    s.properties("Mask")
    s.label("Center mask")
    s.label("Reset mask")
    s.close_window()


def language(s):
    s.click(50, 81)
    s.label("Open all settings")
    s.note(
        "Language packs have one management section: installed languages and revision."
    )
    s.label("Language packs")
    s.label("Remove pack")
    s.note("Removal names every bundled language. Cancel leaves the pack installed.", 2)
    s.label("Cancel")
    s.label("Appearance")
    s.note("Appearance controls resize the interface and choose a theme.")
    s.label("Browse all themes")
    s.label("Corporate Light")
    s.capture("light-theme.png")
    s.note("Preview another language; the confirmation lets you return to English.")
    s.click(220, 71)
    s.label("French")
    s.capture("language-preview.png")
    time.sleep(3)
    s.click(630, 531)


def navigation(s):
    s.note("Zoom around the pointer; middle-drag pans without moving geometry.")
    s.move(790, 390)
    s.cmd("xdotool", "click", "--repeat", "3", "--delay", "150", "4")
    time.sleep(1)
    s.move(650, 500)
    s.cmd("xdotool", "mousedown", "2")
    s.move(760, 560, 1)
    s.cmd("xdotool", "mouseup", "2")
    time.sleep(1)
    s.note(
        "Fit screen restores the complete output. Zoom to selection frames the selected layer."
    )
    s.click(496, 81)
    s.click(532, 81)
    s.click(496, 81)
    s.note("Output format exposes standard and custom resolutions.")
    s.click(776, 81)
    s.capture("output-dialog.png")


def clipboard(s):
    s.note("Paste SVG geometry from an external application; review before applying.")
    s.key("ctrl+a")
    s.clipboard((s.assets / "geometry.svg").read_text())
    s.key("ctrl+v")
    time.sleep(1)
    s.label("Selected layers")
    s.label("All layers")
    s.click(650, 723)
    s.click(560, 779)
    s.capture("import-review.png")
    s.note("Choose mapped geometry and import it as a single undoable edit.", 2)
    s.label("Import mapped values")
    s.expect_absent("Import layer values")
    s.capture("imported.png")
    time.sleep(2)
    s.note("Undo restores the previous geometry.")
    s.key("ctrl+z")


def export(s):
    s.click(50, 81)
    s.label("Export")
    s.label("Export layers")
    s.note(
        "One export window selects format, layer scope and optional editor metadata.", 2
    )
    s.capture("export-window.png")
    s.label("SVG vector editors")
    s.capture("formats.png")
    time.sleep(2)
    s.key("Escape")
    s.label("Selected layers")
    s.label("All layers")
    s.label("Canvas Pilot settings not included")
    s.capture("metadata-options.png")
    s.note("Export the complete layout as an SVG file.")
    s.label("Export", region=(15, 100, 360, 850), last=True)
    s.dialog(s.folder / "layout.svg", save=True)
    if not (s.folder / "layout.svg").exists():
        raise RuntimeError("SVG export was not saved")


def device(s):
    s.note("Aquilon simulator at 127.0.0.1:3000 — not physical-output acceptance.")
    s.click(50, 81)
    s.label("Device connection")
    s.capture("connection.png")
    s.label("Connect", last=True)
    time.sleep(4)
    try:
        s.find("Online")
    except RuntimeError:
        s.label("Connect", last=True)
        time.sleep(4)
        s.find("Online")
    s.capture("identity.png")
    s.click(417, 35)
    s.note(
        "Inspect the detected screen, Preview bank and explicit destination mapping."
    )
    s.click(170, 923)
    s.capture("device-mapping.png")
    time.sleep(3)


def fits(s):
    s.note("A square layer makes the source-to-container fit differences visible.")
    s.properties("Crop")
    s.capture("advanced-fit.png")
    for old, new in [
        ("Centered (fit)", "Fullscreen (stretch)"),
        ("Fullscreen (stretch)", "Cropped (fill)"),
        ("Cropped (fill)", "1:1 pixels"),
    ]:
        s.note(new + ": inspect the picture preview and unchanged container.")
        s.click(95, 180)
        s.click(
            85,
            {"Fullscreen (stretch)": 267, "Cropped (fill)": 297, "1:1 pixels": 207}[
                new
            ],
        )
        time.sleep(1)
    s.capture("fit-modes.png")
    s.close_window()


def snap(s):
    s.note("Enable proximity snapping and choose its targets.")
    s.click(646, 81)
    s.label("Snap movement & resizing")
    s.label("Mask edges")
    s.click(1000, 800)
    s.note(
        "Move the layer close to another edge; the guide marks the snapped position."
    )
    s.drag(795, 390, 588, 390)
    s.note("Overlay-grid visibility and snapping are independent.")
    s.click(682, 81)
    s.label("Show overlay grid")
    s.label("Snap to overlay grid")
    s.capture("grid-options.png")
    s.click(1000, 800)
    s.click(580, 390)
    s.key("shift+Right")
    s.key("ctrl+z")


def mask_snap(s):
    s.note(
        "Only mask-edge targets are enabled in this prepared scene. The left picture ends before its layer box."
    )
    s.click(646, 81)
    s.capture("mask-targets.png")
    s.click(1000, 800)
    s.note("Drag near the visible picture edge, rather than the original layer edge.")
    s.drag(795, 390, 503, 400)
    s.note("Undo restores the original placement.")
    s.key("ctrl+z")


def groups(s):
    s.note("Shift-click selects both layers; they move together.")
    s.cmd("xdotool", "keydown", "Shift_L")
    s.click(250, 390)
    s.cmd("xdotool", "keyup", "Shift_L")
    s.drag(795, 390, 755, 430)
    s.key("ctrl+z")
    for mode in ["Same handles", "Mirror horizontally", "Opposite handles"]:
        s.note(mode + ": resize the pair, then undo before the next example.")
        s.click(795, 390, 3)
        s.label("Resize together")
        s.click(
            1080,
            {"Same handles": 606, "Mirror horizontally": 634, "Opposite handles": 662}[
                mode
            ],
        )
        s.drag(965, 483, 1000, 510)
        s.key("ctrl+z")


def alignment(s):
    s.note("Alignment targets the output when one layer is selected.")
    s.click(585, 81)
    s.capture("alignment-menu.png")
    # The icon grid follows the target selector in this fixed-size menu.
    x, y = s.find("Screen (one selected layer)")
    s.click(x, y + 74)
    time.sleep(1)
    s.note("Undo alignment, then reposition the resize pivot.")
    s.key("ctrl+z")
    s.click(722, 81)
    s.drag(795, 389, 629, 294)
    s.note("A corner resize now keeps the relocated pivot fixed.")
    s.drag(965, 483, 1000, 510)
    s.key("ctrl+z")
    s.click(722, 81)
    s.note(
        "Measurement references change the displayed coordinates without moving geometry."
    )
    s.label("Top left to Top left")
    s.capture("measurement-points.png")
    s.click(1130, 569)
    time.sleep(2)
    s.key("Escape")


def organize(s):
    s.note(
        "A prepared folder contains Slides. Move Presenter into it from the layer menu."
    )
    s.click(1185, 116, 3)
    s.label("Move to folder")
    s.label("Presentation")
    s.note(
        "Collapse and expand the folder; visibility and lock controls apply to its members."
    )
    s.click(1083, 114)
    s.click(1083, 114)
    s.click(1395, 114)
    s.click(1395, 114)
    s.click(1357, 114)
    s.click(1357, 114)
    s.note(
        "The layer manager provides bulk names, folders, ordering and add/remove actions."
    )
    s.click(1403, 76)
    s.capture("layer-manager.png")
    s.note("Add a layer, give it a name and folder, then change its stacking order.")
    s.label("Add layer")
    s.field(250, 163, "Logo")
    s.field(420, 163, "Branding")
    s.click(70, 166)
    s.capture("reordered.png")
    s.note("Remove the temporary layer; existing layers keep their IDs.")
    s.label("Remove selected")
    s.capture("removed.png")


def reference(s):
    s.note(
        "A prepared offline reference image guides placement; outlines keep it visible."
    )
    s.click(50, 81)
    s.label("View")
    s.label("Reference image settings")
    s.note(
        "Reference opacity and native-pixel placement are independent of layer geometry."
    )
    s.label("Show reference")
    s.label("Show reference")
    s.label("Native pixels (1 image pixel = 1 screen pixel)")
    s.capture("reference-settings.png")
    time.sleep(2)


def preview(s):
    s.note(
        "This is a cached, labelled offline demonstration image, not a live capture."
    )
    s.properties("Preview")
    s.capture("preview-settings.png")
    s.note("Preview controls show capture setup, refresh and crop-guide opacity.", 2)
    s.close_window()
    s.label("Adjust layer crop")
    s.drag(795, 390, 830, 405)


def source(s):
    s.note(
        "Source dimensions, usable input and final layer size are different concepts."
    )
    s.properties("Source")
    s.capture("source-settings.png")
    s.note(
        "Advanced shared-input controls are opt-in; opening this tab sends nothing.", 2
    )
    s.label("Edit advanced input settings and include my edits when sending")
    s.capture("advanced-source.png")
    time.sleep(2)
    s.close_window()


def file_import(s):
    s.note("Import a real SVG file from disk; geometry is reviewed before application.")
    s.click(184, 81)
    s.label("Geometry file")
    s.dialog(s.assets / "geometry.svg")
    time.sleep(1)
    s.click(700, 125)
    s.capture("file-review.png")
    s.note("Both selected layers are mapped; apply position and size together.", 2)
    s.label("Import mapped values")
    s.expect_absent("Import layer values")
    s.capture("imported.png")
    time.sleep(2)
    s.note("The imported edit can be undone.")
    s.key("ctrl+z")


def project(s):
    destination = s.folder / "showcase.pilot.json"
    s.note(
        "Save a named project including editable settings and local workspace state."
    )
    s.click(123, 81)
    s.label("Save project")
    s.dialog(destination, save=True)
    if not destination.exists():
        raise RuntimeError("Project file was not saved")
    s.label("OK")
    s.note(
        "Make a change, then reopen the saved project with an explicit replacement review."
    )
    s.drag(795, 390, 745, 430)
    s.click(123, 81)
    s.label("Open project")
    s.dialog(destination)
    s.label("Open project", last=True)
    time.sleep(2)
    s.capture("reopened.png")


def pack_lifecycle(s):
    s.click(50, 81)
    s.label("Open all settings")
    s.label("Language packs")
    s.note(
        "Install the example ZIP: Ukrainian and Bhojpuri become available immediately."
    )
    s.click(270, 228)
    s.dialog(ROOT / "docs/language-packs/language-pack.zip")
    s.capture("installed.png")
    time.sleep(2)
    s.note("A higher revision updates the existing pack without an overwrite warning.")
    s.click(270, 228)
    s.dialog(s.assets / "language-pack-revision-2.zip")
    s.capture("updated.png")
    time.sleep(2)
    s.note(
        "An older revision requires an explicit overwrite decision. Cancel keeps revision 2."
    )
    s.click(270, 228)
    s.dialog(ROOT / "docs/language-packs/language-pack.zip")
    s.find("Overwrite pack")
    s.capture("downgrade-warning.png")
    time.sleep(2)
    s.label("Cancel")
    s.note("Remove the pack after reviewing the two languages affected.")
    s.label("Remove pack")
    s.capture("remove-warning.png")
    time.sleep(2)
    s.click(550, 524)
    s.capture("removed-pack.png")


SHOTS = {
    "20-language-pack-lifecycle": (
        "Install, update and remove a language pack",
        "default",
        pack_lifecycle,
    ),
    "01-project-save-reopen": ("Project save and reopen", "default", project),
    "03-alignment-pivots-measurement": (
        "Alignment, pivots and measurement references",
        "default",
        alignment,
    ),
    "04-group-resizing": ("Group movement and coordinated resizing", "default", groups),
    "05-snapping-grid": ("Snapping targets and overlay grid", "default", snap),
    "06-layers-folders": (
        "Layers, folders, visibility and locking",
        "organize",
        organize,
    ),
    "10-source-and-fit": ("Source model and advanced input opt-in", "default", source),
    "11-fit-modes": ("Centered, stretched, filled and 1:1 fitting", "fits", fits),
    "13-file-import": ("Geometry file import and review", "import", file_import),
    "15-reference-images": (
        "Reference images and outline placement",
        "reference",
        reference,
    ),
    "18-cached-preview": (
        "Cached input preview and crop guides — offline",
        "preview",
        preview,
    ),
    "19-mask-snapping": ("Snap to the visible masked picture", "mask-snap", mask_snap),
    "02-move-resize": ("Move, resize, precise values and undo", "default", movement),
    "07-navigation-output": (
        "Camera navigation and output resolution",
        "default",
        navigation,
    ),
    "08-crop": ("Crop and reframe inside a fixed layer", "crop", crop),
    "09-mask": ("Mask edges, reveal guides and reset", "mask", mask),
    "12-clipboard-import": (
        "Clipboard geometry import and review",
        "default",
        clipboard,
    ),
    "14-export": ("Export formats, scope and metadata", "default", export),
    "16-appearance-language-packs": (
        "Appearance and language-pack management",
        "language",
        language,
    ),
    "17-aquilon-simulator": (
        "Aquilon simulator connection and review",
        "device",
        device,
    ),
}


def gallery():
    cards = []
    duration = 0.0
    for shot in sorted(OUT.glob("*/shot.json")):
        info = json.loads(shot.read_text())
        slug = shot.parent.name
        if not (shot.parent / "video.mp4").is_file():
            continue
        duration += info["duration"]
        cards.append(
            f'<section><h2>{html.escape(info["title"])}</h2><p>{slug} · {info["duration"]:.1f} seconds · {html.escape(info["status"])}</p><video controls preload="none" poster="{slug}/poster.jpg"><source src="{slug}/video.mp4" type="video/mp4"><track default kind="captions" srclang="en" label="English" src="{slug}/captions.vtt"></video><p><a href="{slug}/video.mp4">MP4</a> · <a href="{slug}/captions.vtt">Captions</a></p></section>'
        )
    (OUT / "index.html").write_text(
        '<!doctype html><html lang="en"><meta charset="utf-8"><title>Canvas Pilot showcase</title><style>body{background:#111720;color:#edf3fc;font:18px system-ui;max-width:1100px;margin:40px auto;padding:20px}video{width:100%}section{margin:50px 0}a{color:#66dbef}</style><h1>Canvas Pilot feature showcase</h1><p>Independent clips · actual application capture · silent with optional captions. Simulator footage is labelled explicitly.</p>'
        + f"<p>{len(cards)} clips · {duration / 60:.1f} minutes total</p>"
        + "".join(cards)
        + "</html>"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--display", default=":97")
    parser.add_argument("--tools", type=Path)
    parser.add_argument("--only", nargs="+", choices=SHOTS)
    parser.add_argument("--list", action="store_true")
    parser.add_argument(
        "--inspect",
        choices=SHOTS,
        help="Start one seed without recording; leave it open for manual rehearsal",
    )
    args = parser.parse_args()
    if args.list:
        for slug, (title, variant, _) in sorted(SHOTS.items()):
            print(slug, ":", title, "[seed:", variant + "]")
        return
    OUT.mkdir(exist_ok=True)
    studio = Studio(args)
    studio.portals()
    if args.inspect:
        studio.start(args.inspect, SHOTS[args.inspect][1])
        print("Ready for rehearsal", flush=True)
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            studio.close()
        return
    errors = []
    try:
        for slug in args.only or sorted(SHOTS):
            title, variant, actions = SHOTS[slug]
            try:
                studio.record(slug, title, variant, actions)
            except Exception as error:
                errors.append(f"{slug}: {error}")
                print("FAILED", errors[-1], flush=True)
    finally:
        studio.close()
        gallery()
    if errors:
        raise SystemExit("\n".join(errors))


if __name__ == "__main__":
    main()
