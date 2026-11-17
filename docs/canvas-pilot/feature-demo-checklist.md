# Recording the feature showcase

Record independent clips, with a fresh project and settings profile for each take.
This avoids carrying a mistake into the next demonstration. The reproducible shot
recipes live in [record-showcase.py](../../scripts/record-showcase.py); the
[advanced guide](guide/overview.md) remains the reference for individual controls.

## Shot list

The two-layer fixture is a 1920 × 1080 output with Slides and Presenter. Special
fixtures start with crop, mask, folder, reference or cached preview state already
prepared. Captions explicitly identify that setup instead of implying it was
created during the clip.

| Clip | Actions and result to check |
| --- | --- |
| 01 Project | Save a named project, move a layer, reopen and confirm replacement; saved geometry returns. |
| 02 Movement | Drag, corner-resize, undo/redo, enter an exact width and nudge; inspector values follow. |
| 03 Alignment | Align to the screen, relocate a pivot, resize and change measurement reference. |
| 04 Group resize | Select two layers, move together and compare same, mirrored and opposite handles with undo between examples. |
| 05 Snapping/grid | Show target choices, snap a moving edge and distinguish overlay visibility from grid snapping. |
| 06 Organization | Move a layer into a prepared folder, collapse, hide/lock, then add, name, reorder and remove a temporary layer. |
| 07 Navigation/output | Pointer-centered zoom, pan, fit screen, zoom to selection and inspect output formats. |
| 08 Crop | Start at 200%, reframe, inspect faded surroundings, recenter and reset without moving the container. |
| 09 Mask | Enter crop, Tab to mask, resize/move the reveal window, inspect hatching, center and reset. |
| 10 Source | Inspect the local source/input/layer distinction and explicit advanced-input opt-in; no input writes. |
| 11 Fit modes | Use a square container to compare Centered, Fullscreen, Cropped and 1:1 against the same image. |
| 12 Clipboard | Paste real SVG text, review destination mapping, apply geometry and undo. |
| 13 File import | Pick a real SVG file, review, import and undo. |
| 14 Export | Show formats, scope and metadata choices, then save a complete SVG layout. |
| 15 Reference | Show a prepared reference, visibility, opacity/native placement controls and layer outlines. |
| 16 Appearance/packs | Inspect installed pack revision and languages, removal confirmation, cancel, change theme and preview French with a return to English. |
| 17 Simulator | Connect to the explicitly identified local Aquilon simulator and inspect device controls. |
| 18 Cached preview | Inspect preview/guide controls using a labelled offline image; this is not live capture. |
| 19 Mask snapping | With only mask targets enabled, snap against the visible picture edge and undo. |
| 20 Pack lifecycle | Install the Ukrainian/Bhojpuri example ZIP, update to revision 2, cancel a downgrade warning and confirm removal. |
| 21 Simulator transfer | With a separately authorized fixture, load a linked Preview layer, edit geometry, review, send and verify readback; restore the simulator afterward. |

These clips group related interactions rather than making a clip for every button.
The SVG demonstrations exercise interchange without pretending to record Illustrator,
Figma or draw.io. Actual captures from those applications, live input capture, other
file codecs and physical device output require separate application/device sessions.

## Conference handoff kit

A separate local `showcase-conference-kit/` directory supplies a fictional **Lumen
Forum** event for recording the raster-only versus editable-client workflow.
It leaves the existing `showcase-videos/` recordings intact. Start with its
`index.html` preview and `START-HERE.txt` recording guide.

The kit includes a clean 1920 × 1080 background, empty PIP infodecor, a placement
guide and a complete speaker/slide composite; three editable presentation slides;
a synthetic portrait; and silent speaker-motion, presentation and background MP4s.
The speaker loop is a pan/zoom of a generated still, not a real recorded talk.
All names, event content and slide data are fictional.

`delivery-jpg-only/` contains flattened PNG/JPGs and a deliberately degraded
960 × 540 JPEG. Raster files are reference artwork, not editable geometry.
`delivery-editable/` contains native PPTX text/shapes, vector SVG slides and a
composition whose speaker/slide pictures move independently in PPTX/draw.io/SVG.
Three layout folders each contain a project plus SVG, draw.io, PPTX, JSON and CSV
geometry. The main speaker box is `(72,260,432,576)`; the slide box is
`(552,260,1296,729)`. Coordinates are top-left screen pixels. The other layouts
show a large slide and a side-by-side pair. Projects have no device links.

Rebuild into a **new** directory with the supplied 3:4 speaker PNG:

```sh
python3 scripts/prepare-conference-showcase.py \
  --speaker showcase-conference-kit/media/speaker/alex-morgan-synthetic.png \
  --output /tmp/lumen-forum-rebuilt
```

The recipe uses Python's standard library, `rsvg-convert`, FFmpeg and ffprobe,
and refuses to replace an existing output folder. DejaVu Sans is the design font.
The portrait was made with the built-in image-generation tool using
[the saved prompt](../../scripts/showcase-speaker-prompt.txt); rebuilds reuse the
image rather than make new network calls. Generated assets remain local and ignored
by Git. The recipe, prompt and checks are versioned.

Prepared-kit validation includes exact geometry imports through Canvas Pilot,
project loading, LibreOffice PDF renders of the native decks, and ffprobe checks
of dimensions and durations. PowerPoint itself and the draw.io GUI were not used;
draw.io geometry was checked by the application importer. `verification/` holds
the local evidence. This is offline artwork evidence, not device acceptance.

## Recording environment

Use a dedicated X11 display, not the working desktop. The script sends real mouse
and keyboard events, changes its own clipboard and recreates its per-shot settings.
The normal user profile is not used. Requirements:

- A built `target/debug/canvas-pilot` (`cargo build -p canvas-pilot`).
- Python 3, Xvfb, xdotool, xclip, FFmpeg/ffprobe and ImageMagick.
- Tesseract with English recognition data; OCR locates labelled controls.
- `dbus-daemon`, `xdg-desktop-portal` and the GTK portal backend for native file pickers.

Start the virtual display in its own terminal:

```sh
Xvfb :97 -screen 0 1440x940x24 -nolisten tcp -noreset
```

Then, from the repository root:

```sh
python3 scripts/record-showcase.py --list
python3 scripts/record-showcase.py --display :97 --only 02-move-resize
```

Omit `--only` to run clips 01–20. Clip 21 requires the separate simulator setup described below. To rehearse a fixture without recording:

```sh
python3 scripts/record-showcase.py --display :97 --inspect 08-crop
```

Stop rehearsal with Ctrl-C. `--tools /path/to/prefix` optionally finds tools under
`usr/bin`, shared libraries under `usr/lib` and English OCR data under
`usr/share/tessdata`. Normal distribution packages need no such prefix. The script
forces X11 for both egui and GTK, uses a private portal bus and sets fresh XDG
configuration/data directories for each shot. Keep the 1440 × 940 resolution and
English interface: icon coordinates are deliberately simple and fixed. Rehearse
again after changing interface layout, font scaling or the desktop tool versions.

## Output and review

The initial 2026-11-17 series records application v0.9.16 (`dc3e6ca`). Version
0.9.17 adds the recording tooling and guide without changing application behavior.
The local `manifest.json` records the binary fingerprint, clip durations and hashes.

Everything generated stays in the Git-ignored `showcase-videos/` folder. Each shot
has its own `video.mp4` (H.264, 30 fps, no audio), English `captions.vtt`, poster,
contact sheet, start/end captures, starting/ending project JSON and recording log.
The gallery is `showcase-videos/index.html`. For browser caption support, serve it:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory showcase-videos
```

Open `http://127.0.0.1:8765/`. There is no autoplay. Videos and source fixtures stay
local; committing the recipe does not publish or upload the recordings.

Every take must decode successfully, but that is only the first check. Watch the
clip/contact sheet and inspect the saved result: a clicked disabled button is not
a successful import. Confirm visible movement, readable menus, complete dialogs,
actual saved exports and restoration after undo. Failed takes are named
`FAILED-video.mp4` and excluded from the gallery. Fix the recipe and rerun just that
shot. Keep captions factual about what is visible and what was preloaded.

## Simulator and hardware boundaries

The device fixture uses `127.0.0.1:3000`, with startup connection disabled. Check
identity before filming. Standalone AQL Cmax 6.2.x is the currently supported
Aquilon profile; the session simulator identifies as 6.2.79. Broader Aquilon/Midra
coverage is tracked in the [project backlog](plan.md#to-do-list).

Connection and mapping inspection are read-only. For a separate send demonstration,
first capture the chosen Preview layer's baseline, use already routed sources,
review the exact geometry/crop/mask changes, send, verify readback, then restore
and verify the baseline. Never include TAKE, source routing or shared-input edits
implicitly. Do not use a saved host as authorization for another device. Simulator
readback does not establish physical-output accuracy.

For the local transfer take, the user authorized temporarily routing **only S1
Preview layer 1 to LIVE_1**. The completed transfer take confirmed application readback, followed by a successful
comparison with the original baseline (`simulator-restoration.json`, no mismatches).
The baseline records source, geometry, anchor, classic
crop and mask for all seven S1 layers in both banks. The local
`showcase-videos/simulator_session.py` captures/restores this fixture, and
`showcase-videos/record-transfer.py` records the take with restoration in `finally`.
Those session-specific helpers and their JSON evidence stay beside the recordings,
not in the reusable recorder or CI. They require the temporary `websocket-client`
package and this session's tool paths; inspect and adapt them before reuse.

To take the transfer shot manually: identify the simulator, preserve the same
baseline, route its Preview source explicitly, link Presenter to Aquilon S1/layer
1 in Mapping, connect, and click **Load linked layers**. Set width 720, X 900 and
Y 300, open **Device mapping & transfer**, expand the window, verify Preview and
the linked destination, then **Send mapped layers**. Wait for the readback success
message. Restore the original Preview source and all edited fields, apply the
preset update and compare both banks with the captured baseline. Stop if bank
mapping changes or any restoration check fails.
