# Updating the Canvas Pilot screenshots

[Canvas Pilot introduction](../introduction.md) · [Detailed user guide](../guide/overview.md)

The introduction uses real application captures, not the design mockups in
`../design/`. Each image has an adjacent HTML comment in the introduction that
points to its recipe below.

## Capture provenance

Captured on **2026-11-14** from the default-feature debug build at source commit
`ba67f21`, on Linux/X11 (XFCE). The app's client area was **1440 × 940 pixels**,
at 100% display scaling, with the default dark theme. Screenshots are unedited
PNG captures of the app client area, without desktop decorations or the mouse
pointer. The original captures used ImageMagick `import -window <window-id>`;
a temporary Python `ctypes` helper located the isolated process's X11 window and
used XTest clicks to select the controls. The helper is not required to reproduce
the images: the same steps can be performed manually.

All images use the built-in example backdrop and the default **1920 × 1080**
project. No external artwork, saved user project or device connection was used.
The footer should read **OFFLINE**. No hardware reads or writes are needed.

## Prepare a clean session

From the repository root, build the current app and start it with temporary user
settings and recovery storage. Use a new temporary directory for each capture
session so that earlier recovery data does not replace the default layout.

```sh
cargo build --locked -p canvas-pilot
MIDRA_DOC_SESSION=$(mktemp -d /tmp/canvas-pilot-docs.XXXXXX)
XDG_CONFIG_HOME="$MIDRA_DOC_SESSION/config" \
XDG_DATA_HOME="$MIDRA_DOC_SESSION/data" \
  target/debug/canvas-pilot
```

These isolation variables apply on Linux. On another platform, use an isolated
user profile or restore the same default project and theme before capturing.
Keep the default window size and 100% display scaling. The output canvas is
1920 × 1080; the two layers are **01 / Presenter** and **02 / Slides**, each
640 × 360. Select **01 / Presenter** in the Layers list. Do not move or resize it.

After each action, move the pointer into empty space below the canvas, without
clicking, and wait for the UI to settle. Clicking empty space clears selection.
Dismiss any notification or tooltip before capturing.

With ImageMagick, run the following in a second terminal and click the Midra
window to select it. Alternatively, use your desktop's “capture window” action
with the pointer and window border excluded.

```sh
import docs/canvas-pilot/images/01-workspace.png
```

For a known X11 client-window ID, the equivalent command is:

```sh
# Set MIDRA_DOC_WINDOW to the ID of this temporary Canvas Pilot window.
import -window "$MIDRA_DOC_WINDOW" docs/canvas-pilot/images/01-workspace.png
```

Capture each state below to its corresponding filename. Keep the full client
area; do not crop, add annotations, or substitute a mockup. Close the temporary
app after the last capture.

## 01-workspace.png

Start from the clean session with **01 / Presenter** selected and no menus or
dialogs open. Both layer boxes should be visible. The right inspector shows
640 × 360 dimensions and measurements from the top-right corner: 120 pixels
from the right and 180 pixels from the top. Capture the workspace.

## 02-layer-properties.png

From the workspace state, click the **640.0 × 360.0 px / 16:9** resolution field
in the right inspector. Keep the **Layer** tab selected in the Layer properties
window, with **Keep aspect when resizing** checked and the original width and
height unchanged. Do not move or resize the properties window. Capture it over
the canvas.

## 03-crop.png

From the Layer properties state, click the **Crop** tab. This activates visual
crop mode and displays the source pattern on the canvas. Leave the default
100% zoom and centered content unchanged. Capture the crop controls and preview.
Do not connect a device or import an input snapshot to reproduce this picture.

## Check the update

- Open all three PNGs and confirm the intended controls and layer labels are readable.
- Check that there are no tooltips, transient notifications, unrelated windows or private data.
- Preview the [introduction](../introduction.md) and keep its captions and image alt text consistent with the UI.
- Update the capture date, source commit and any changed steps in this file; keep the adjacent screenshot comments current.
- Commit the three PNGs and documentation changes together. Temporary settings and captures do not belong in Git.

## Advanced guide captures

Captured on **2026-11-16** from the default-feature debug binary in the standalone
Canvas Pilot checkout (documentation revision `f4584e0`). The current application
still renders `MIDRA / LAYOUT` in its header. These are fresh captures, not the
older introduction images. Full client area: **1440 × 940**, Linux/X11, default
dark theme, 100% display scaling, pointer and decorations excluded. PNGs are
unedited. Each guide placement links here with an HTML comment.

The session used separate temporary `XDG_CONFIG_HOME` and `XDG_DATA_HOME`
directories, the default two-layer 1920 × 1080 layout and built-in backdrop.
It stayed OFFLINE throughout; no project files or device data were opened.
The source binary was `target/debug/canvas-pilot`; ImageMagick captured only its
client window. XTest pointer events operated only the isolated session.

| Image | Reproduce the visible state |
| --- | --- |
| `04-guide-workspace.png` | Start a clean session; leave Presenter selected. |
| `05-guide-layer-properties.png` | Click the selected layer's resolution card and leave the Layer tab open. |
| `06-guide-crop.png` | Switch to Crop; retain normal fit, 100% zoom and zero offsets. |
| `07-guide-mask.png` | Switch to Mask, then drag the left magenta edge inward about 61 display pixels (116.19 output pixels). Keep the layer bounds unchanged. |
| `08-guide-export.png` | Reset the mask and close properties. Select Slides; open Project → Export → Export layers. Leave SVG and all optional checkboxes at their defaults. |
| `09-guide-settings.png` | Close export; open the toolbar settings cog → Open all settings. Leave Appearance selected. |
| `10-guide-grid.png` | With Slides selected and other dialogs closed, open the overlay-grid control; keep the default eight columns and six rows, visibility on and grid snapping off. |

Verify each full-size capture before replacing it. Keep its guide caption,
alt text and actual control state consistent. The original three introduction
images remain unchanged and retain their earlier provenance above.
