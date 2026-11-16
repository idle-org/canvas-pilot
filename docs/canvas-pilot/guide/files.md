# Import, export and project files

[Canvas Pilot](../introduction.md) · [Guide contents](overview.md) · [Starter files](../../templates/canvas-pilot/catalog.md)

Choosing an exchange format? See the [file and clipboard feature matrix](format-matrix.md)
for naming, crop, pictures and editable-settings support.

## Paste layer values and import geometry

Drop SVG, draw.io/XML, geometry JSON/CSV, DXF/DWG, HTML, PPTX, FIG, FIGCRAFT or AI files into
the window to open the same geometry review as **Import → Geometry file…**.
Native codecs still require their normal build options/dependencies. Filename
extensions are case-insensitive; PNG/JPEG/BMP drops remain reference images.
Multiple geometry files appear in the **File / page / frame / slide** chooser;
choose one document/page to map at a time. Reading runs in the background, and
nothing changes in the layout until Apply. The entire geometry drop is limited
to 16 MiB; a failed document leaves the batch unapplied and reports its filename.

1. Copy layer values or geometry and use **Import → Clipboard values** or
   **Ctrl/Cmd-V** with the viewport focused. Alternatively use **Import → Geometry
   file…** or drop a supported file. These entry points share one mapping review.
   If entering text manually, paste it and click **Review pasted values**.
2. Once geometry is detected, the review only shows the loaded data, mapping and
   import options. **Clipboard data** is a collapsed, read-only preview; there are
   no additional file/paste buttons inside the review. Multiple documents/pages
   retain their page chooser.
3. Under **Destination layers**, choose **All layers**, **Selected layers**, or
   **Folder: …**. Existing selection initially scopes and pre-maps the import;
   with no selection, choose destinations individually from All layers. Changing
   the scope never changes canvas selection and preserves mappings still in scope.
   Hidden/locked layers are excluded. Each source item has a prominent destination
   dropdown and collapsed **Geometry · before / after** details. Uncheck **Include
   item** to skip an item. Each included item must map to a different destination;
   fewer source items than destinations are fine. Duplicate or missing mappings
   block import. Other layers stay unchanged.
4. Choose independently what to paste: **Format (width / height)**, **Position**,
   and **Crop zoom**. Zoom is enabled only when present in the data (new Midra
   clipboard values include it; older copies, draw.io and SVG do not). All
   available values are initially enabled. Unchecked values retain destination
   settings; format-only paste keeps its top-left position. Choose **Include
   outline in bounds** if the layer should cover the rectangle's outline too.
   Zoom applies to the resulting layer size; existing crop offsets clamp if
   necessary. Mixed batches keep destination zoom where the source lacks it.
   At least one available value must be enabled.
5. Click **Import mapped values** to update the mapped layers in one undo step.
   Cancel leaves all layers unchanged. Invalid geometry
   blocks the whole batch, without partial changes.

The workspace supports 1–256 layers; the import review handles all selected
layers rather than choosing one rectangle from a larger clipboard selection.

### SVG bounds

SVG import accepts square-corner rectangles and simple closed rectangular
paths/polygons, including translated/scaled groups and inline styles. It applies
page dimensions and `viewBox` transforms, using CSS units at 96 px/in. Coordinates
are relative to the SVG page; exporting only a selection may lose its original
document position. The SVG is used for bounds, not rendered as a reference image.

A first full-page rectangle or image is marked as a possible background. Uncheck
**Include full-page background** to exclude it from the count and mapping. This
is explicit: a full-page rectangle can also be a real layer. Image backgrounds
must be excluded; their contents and external links are never loaded.

Midra SVG v2 placeholder images import as one layer box each; their image content
is not decoded. Other visible artwork, rotation/skew, rounded rectangles, clipping, masks,
filters, stylesheets, and unsupported SVG features are refused with an error.
Hidden objects and definitions are ignored. Export simple rectangles with inline
styles for predictable results. Files are limited to 16 MiB, 256 rectangles plus an optional background, and
64 levels of nesting. SVG imports retain the same outline and position-only
choices as draw.io imports.

### Figma SVG clipboard text

The supplied [Figma SVG sample](../../../crates/canvas-pilot/tests/fixtures/figma-rectangles.svg)
works through the existing SVG/value paste parser: two 640 × 360 rectangular
paths at (0, 0) and (1040, 0), including their matrix translations and order.
Paste SVG text and review the detected shapes in the mapping dialog.
The user confirmed actual Figma copy/paste working on 2026-11-15. This validates
the tested SVG workflow, not every native binary/HTML clipboard format.

### Illustrator SVG clipboard text

The supplied [Illustrator SVG sample](../../../crates/canvas-pilot/tests/fixtures/illustrator-rectangles.svg)
works through the existing SVG/value paste parser. Its two 1814.174 × 1020.473
rectangles retain fractional positions (0.354, 0.354) and (395.791, 461.357).
Enabling outline inclusion adds the centered 0.709-unit stroke to their bounds,
producing 1814.883 × 1021.182 boxes. Stroke colors are not imported as artwork
through this geometry workflow. The user confirmed actual Illustrator copy/paste
working on 2026-11-15; other native clipboard formats are outside this validation.

For QCAD and PowerPoint, use file exchange; no clipboard-export integration is
planned for the user's reported workflows.

### draw.io clipboard

The draw.io adapter accepts single-page `.drawio`/`.xml` files (including native
compression) and raw or once-percent-encoded `mxGraphModel` clipboard XML, like
[the supplied clipboard fixture](../../../crates/canvas-pilot/tests/fixtures/drawio-rectangle.txt).
Draw.io coordinates map **1:1 to screen pixels**, with a top-left origin. This is
an explicit import convention; no page-scale, physical-unit or viewport-size
conversion is inferred. Missing X/Y means zero. Decimal dimensions are preserved.

The supplied example imports **X 12, Y 271, width 945, height 531.58**. The center
is **484.5, 536.79**. The visible box has its exact imported ratio; it is not rounded
or coerced into a 16:9 preset. “Format” here means the visible rectangle's width,
height and aspect, not native source format or advanced cropping.

**Include outline in bounds** is off by default on each paste. For an outline of
width `s`, enabling it subtracts `s/2` from X and Y and adds `s` to width and
height, preserving the center. This follows the half-stroke expansion in
[mxGraph's bounding-box calculation](https://github.com/jgraph/mxgraph/blob/master/javascript/src/js/shape/mxShape.js).
Your sample's 5 px outline gives **950 × 536.58 at (9.5, 268.5)**. Only the bounds
are imported; no border is drawn or configured on the device.

The option is disabled for position-only paste: that mode uses the original
geometry's top-left and keeps the target layer's dimensions. Zero-width,
`strokeColor=none`/`transparent`, zero stroke opacity and zero overall opacity
produce no outline expansion. A missing width uses the standard 1 px default;
the detected width is shown in the preview. Each rectangle in a multi-object
paste uses its own width. Shadows, sketch effects and raster antialiasing fringes
are not included. Invalid thickness/opacity values or bounds beyond the
application limits are rejected without changing the document.

The adapter otherwise ignores colors, text, style aspect locks and other
appearance settings. It currently rejects grouped/nested geometry, connections,
nonrectangular shapes, relative coordinates, alternate bounds, rotated/flipped
objects, named/inherited styles, wrapped objects and compressed `.drawio` files.
Mixed selections containing unsupported objects are rejected as a whole. Invalid
input makes no document changes. Text is limited to 16 MiB; geometry must have
finite coordinates within ±1,000,000 px and positive sizes up to 1,000,000 px.
These are application limits, not verified Eikos limits.

Draw.io also exposes raw page XML through **Extras → Edit Diagram**, as described
in its [source-editing documentation](https://www.drawio.com/docs/manual/advanced/diagram-source-edit/).
Support is covered by the supplied clipboard data and regression fixtures. The
user confirmed actual draw.io copy/paste working on 2026-11-15.

## Save, open and export

Use the **Project** toolbar menu to **Save project…** or **Open project…**.
Existing `.capi.json` and `.midra.json` projects still open normally and are not renamed
automatically. **Save project…** suggests a `.pilot.json` name.

Projects use readable, versioned `.pilot.json` files and preserve applied output,
inputs, layer bounds/crops, source assignments, locks, visibility, selection,
viewport preferences, and all embedded reference images. Apply pending edits
before saving. Opening asks before replacing current work and starts a fresh
undo history. Existing files require confirmation before replacement.

The [starter-file catalog](../../templates/canvas-pilot/catalog.md) contains all eleven
exchange formats, with two layers for 1920 × 1080 and 3840 × 2160 canvases.

**Project → Export → Export layers…** opens one shared window. Choose SVG,
draw.io, JSON, CSV, DXF/DWG, HTML, PPTX, Figma FIG, FigCraft or Illustrator AI, then
**Current layer…**,
**Selected layers…**, **Specific layer…**, **All layers…** and **Visible layers
only…**. A current-layer export keeps its original screen position and full-screen
canvas. The window offers **Include background layer**. SVG, draw.io, geometry
JSON, HTML, PPTX, FIG, FIGCRAFT and AI also offer **Include placeholder pictures** with embedded source-grid
imagery; CSV and DXF explicitly disable pictures and carry rectangle geometry.

Import a supported format with **Import → Geometry file…** or paste its text, then
choose a slide/page/frame where applicable, then map detected layers to destinations.
PPTX uses 96 pixels per inch; FIG/FIGCRAFT use native pixels. AI requires a separately
installed `vectorcraft-cli`; its artboards appear in the same canvas selector.
Import/export runs in the background and reports conversion failures without
modifying the project. Backgrounds are automatically ignored;
placeholder images each count as one layer. draw.io files saved with compression
are supported. SVG rendered by draw.io Desktop is recognized through its retained
cell IDs; generated HTML labels and raster text fallbacks are ignored. Draw.io
may crop and rebase that SVG unless the exported diagram includes the background
layer. These exports round-trip whole-pixel visible-box geometry, not
source/crop settings; the separate `.pilot.json` project format preserves the full project. See the
[geometry exchange contract and native draw.io checks](../../interfaces/canvas-pilot-exchange.md)
and [project save contract](../../interfaces/canvas-pilot.md).

Visual crop mode also provides **Reset crop** beside **Done adjusting crop** in
layer properties. It restores 100% content zoom and centered offsets, keeps the
visible box and source/input crop unchanged, and leaves visual editing active.
The reset is undoable.

The toolbar’s **Viewport options** cog contains **Keep aspect ratio when resizing**
and **Show source patterns on other layers while cropping**. Both apply immediately
and are preserved in saved projects. Crop editing always previews source content
and keeps its proportions, independently of these layer/view preferences.

The toolbar’s **Import** button uses a downward arrow into a tray. Hover for its
label; its dropdown contains SVG, clipboard values, and reference-image actions.

## Choose layers inside import and export menus

You can choose layers without returning to the viewport:

- **Import → Choose destination layers…**, and the same picker inside the
  geometry review, select the destinations. Paste and file import can open
  with nothing selected: map each detected rectangle directly to an available
  layer using its destination dropdown. When using a selection, there must be at least as many chosen destinations as imported rectangles
  after ignoring a background. Map to any subset of them; other layers stay
  unchanged. Duplicate mappings and empty imports remain invalid.
- A format’s **Choose layers to export…** selects the group for
  **Selected layers…**. **Specific layer…** remains available for individual layers,
  including hidden layers.
- Midra send preparation has **Choose editor layers to send…**, followed by a
  device-destination dropdown for each chosen layer. It can open online with
  nothing selected. Sending requires explicit review and clicking **Send mapped layers**.
- Midra import retains its independent per-device-layer destinations, including
  **New layer** and **Do not import**.

The shared pickers update the viewport selection. Hidden/locked layers are labeled
and disabled in these pickers; choosing layers changes no geometry. Full project
save/load and screen reference images are document-wide operations, so they have
no layer mapping.

**Midra mapping & transfer…**, beside ONLINE/OFFLINE and in **Export**, opens the Midra export configuration without sending, even when layers already have saved links. Choose layers, destination mappings, Preview/Program and sizing, then click **Send mapped layers** to send. The action is disabled offline or during transfers. Sending writes geometry, pulses the WebRCS preset-update command, and verifies geometry readback. Only applied editor values are sent.

Layer rows use padlock and eye icon buttons for locking and visibility, with tooltips and highlighted locked/hidden states. Double-click a layer name in the right-hand list to rename it. Enter saves, Escape or Cancel discards the edit; names must contain 1–128 characters. Renaming preserves the layer ID and folder and supports Undo.

Hold **Shift** while moving or resizing layers to snap to a **10 px grid**;
**Ctrl+Shift** uses a **100 px grid**. A matching selected grid tool is temporarily disabled instead; a different shortcut temporarily switches grid size. These grids override ordinary
pixel/edge snapping, even when snapping is disabled in Settings. **Alt**
overrides both grids and temporarily inverts ordinary snapping. The modifier state on release determines the final position.
Grid coordinates are screen pixels measured from the screen's top-left, independent
of viewport zoom. Group movement snaps the selection's bounding top-left and keeps
relative spacing. Resize snapping keeps opposite anchors and group proportions;
free corners snap both axes, while ratio-locked corners snap one driving axis.
Cyan guides mark reached grid lines. Numeric entry, crop-content gestures and camera
panning are unaffected. Shift-click still toggles selection; Shift-drag on a layer
moves it on the grid, and Shift-drag on empty canvas adds a selection box.
This replaces the previous Shift-to-invert-resize-snapping shortcut.

With snapping and layer-edge snapping enabled, resizing in **Mirror** mode snaps
the two moving inner edges into exact contact, including unequal-sized layers.
**Opposite** mode also supports vertical contact. A guide marks the meeting edge;
anchors and aspect-ratio settings are preserved. Move beyond the snap radius to
continue resizing, or hold Alt to temporarily invert snapping. Shift/Ctrl+Shift grid overrides remain.

Midra layer lists omit unassigned source slots (`NONE` or empty). Assigned inputs remain listed even if the layer is off. The exporter defaults to Preview and hides Program until **Settings → Midra integration → Allow sending to Program** is checked. This preference is saved automatically; turning it off returns the exporter and quick Apply action to Preview.

Destination choices in **Midra mapping & transfer…** now save immediately on the editor layer and support Undo. Save the project JSON to keep links across sessions. Links include screen and layer IDs and survive refreshes and selection changes. Choose the empty destination on the linked screen to unlink. The direct send button is available beside Midra mapping & transfer and in Export; in **All layers** scope it includes unselected, hidden and locked linked layers, grouped by screen. Its label changes to **Send selected links** or **Send folder links** in those scopes. Midra mapping & transfer… opens mapping configuration for the current scope. Preview/Program and sizing still use the current export options.

Every explicit send now includes WebRCS’s preset apply pulse (`xUpdate` false → true). This applies staged geometry when automatic updates are disabled, including a repeat send where WebRCS already shows the desired values.

**Send all linked layers** sends immediately using saved links and the current target and sizing settings, without opening the export window. The button shows progress and its tooltip reports completion. Invalid links or send failures open the export window with the error.

**Load linked layers**, beside the connection status and in Export, reads fresh
Midra Preview geometry and crop framing into linked editor layers in the shared
transfer scope. The default **All layers** ignores canvas selection; **Only selected
layers** and **Folder: …** explicitly restrict the operation. The load button inside
**Midra mapping & transfer…** uses that same scope and the saved links. Both actions are read-only
on the device and always load Preview (independent of the send target). They do
not recall device memories or presets. Loading is one atomic Undo step; links
remain saved. Missing screens/layers, mismatched output resolutions, unsupported
geometry, and hidden or locked editor destinations reject the complete import
without changing the document. Unlock/show destinations before loading. Results
appear in the transfer window or the load button's status tooltip. No project or
settings schema changes are involved.

Midra destinations are identified by the pair **screen ID + layer ID**: Screen 1
Layer 1 and Screen 2 Layer 1 are distinct links. Both IDs are already persisted
in project version 5; no schema change is needed. Opening **Midra mapping & transfer…** or
**Midra mapping & transfer…** restores the first scoped layer's available linked
screen. Destination choices display both IDs. Browsing another screen does not
retarget existing links; choosing a destination there explicitly replaces that
layer's saved link. Direct send/load actions use each layer's saved screen,
independently of the screen currently displayed in the configuration window.


### Recovery autosave and New project

Applied edits and reference images are checkpointed to `last_project.json`
beside the application settings, at most every two seconds between gestures and
again on normal exit. The last session resumes automatically on launch without
an Open dialog. Unchanged snapshots are not rewritten. Named project files remain
explicit saves (measurement-point autosave still updates its two fields).

**Project → New project…** starts the default layout after confirmation. The warning
shows the last recorded manual-save age; autosave does not count. Choose **Save and
create new…**, **Discard and create new**, or **Cancel**. Cancelling a save or a
failed save keeps the current project. Opening another project also replaces the
recovery copy, so save a named copy when you want to keep multiple projects.

Recovery contains applied project values, not undo history or unfinished text,
Source/import drafts. A crash can lose edits since the last checkpoint; normal
exit cancels unfinished canvas gestures. Successful restoration is silent. Recovery failures still display an actionable
message. Project-menu status shows the recovery
path and failures. A corrupt/unsupported recovery file is preserved with autosave
paused; Retry recovery attempts to load it again. For a write failure, Retry
attempts to save again. New project explicitly replaces the recovery copy.

See the [recovery file contract](../../interfaces/canvas-pilot-recovery.md).

Right-click a layer in the viewport for **Rename layer…** / **Delete layer** alongside
the existing tools. Layer-list rows have a compact menu with just those two actions.
Actions target the clicked layer; deletion supports Undo and requires a visible,
unlocked layer with at least one other layer remaining.

In **Layer properties → Crop**, X/Y offsets are constrained to the valid range
for the current zoom, input dimensions and visible box. Typing or dragging beyond
an edge stops at that edge; reducing zoom clamps existing offsets to the new
bounds. An axis with no spare content stays at zero. These adjustments apply
immediately and support Undo without an offset-range error.

The tiled source pattern is hidden during normal layout editing, including when
loading saved projects/settings with source-pattern display enabled. Double-click
a layer or open its Crop properties to show its pattern and faded cropped overflow;
leaving crop editing hides them again. **Show source patterns on other layers
while cropping** optionally displays patterns on the other layers during that
mode only (except outline-only layers).

## What the project saves

Project JSON includes source formats, both crop stages, geometry, layer ordering,
folders, visibility/locking, pivots, Midra links, reference images, selection,
viewport and positioning settings. It also saves snapping/grid settings, group
resize mode, collapsed folders, capture input assignments, the last captured
stills, guide opacity and refresh settings. Saved stills work offline; automatic
refresh reopens paused until you capture or resume it explicitly.

Connection history, startup connection, theme and Program permission remain in
application preferences. Unsaved drafts, undo history, open menus, active editing
gestures and pending device operations are temporary. Transfer scope, target bank,
sizing strategy and opt-in advanced device input edits remain session choices.
See the [complete persistence audit](../../interfaces/canvas-pilot-persistence-audit.md)
and [versioned project contract](../../interfaces/canvas-pilot.md).

## Extra settings inside artwork exports

In **Export layers**, enable **Include extra Midra parameters (experimental)** to
store the exported layers' source/crop settings, names, folders, links and workspace
setup alongside the geometry. It is available for SVG, draw.io, JSON, CSV, DXF,
HTML and PPTX. Images/capture assignments are excluded from this snapshot; the
background and placeholder options remain separate. DWG, FIG and AI show why the
option is unavailable with the current codecs.

Use **Project → Restore parameters from export…** to restore that original
snapshot as an unsaved project. This replaces the current editor document after
review, and never overwrites the artwork with a project file. To import changes
made in another program, continue to use **Import → Geometry file…**: the embedded
snapshot can be older than the artwork, so it is never applied automatically.
Other programs may remove the extra data when resaving. **Save project** remains
the complete save, including reference images and captured previews.

See the [format-by-format audit and metadata contract](../../interfaces/midra-extra-parameters.md).

## Illustrator AI: optional converter required

**SVG is the preferred format for exchanging layer layouts with Illustrator.**
Midra imports and exports SVG directly, without Illustrator or VectorCraft installed.
AI remains an optional compatibility format; both AI import and AI export require
**VectorCraft CLI**, which is not bundled with Canvas Pilot. Installing the
VectorCraft graphical application alone does not guarantee that its CLI is
available to Midra.

The normal build includes all other supported exchange formats without external
applications: SVG, draw.io, geometry JSON/CSV, HTML, DXF/DWG, PPTX, FIG and FIGCRAFT.
A build made with `no_proprietary` disables DWG, PPTX, FIG, FIGCRAFT and AI support.
Installing a converter does not re-enable formats excluded from that build.

Obtain VectorCraft CLI from the [VectorCraft project](https://github.com/storytold/vectorcraft).
Midra searches in this order:

1. The executable path set in `MIDRA_VECTORCRAFT_CLI`.
2. `vectorcraft-cli` beside the Canvas Pilot executable.
3. `vectorcraft-cli` in the adjacent `bin/` folder.
4. `vectorcraft-cli` on `PATH`.

On Windows the executable is named `vectorcraft-cli.exe`. For an explicit location,
start Midra from the same terminal in which you set the variable:

```sh
MIDRA_VECTORCRAFT_CLI=/absolute/path/to/vectorcraft-cli canvas-pilot
```

```powershell
$env:MIDRA_VECTORCRAFT_CLI = 'C:\Tools\VectorCraft\vectorcraft-cli.exe'
.\canvas-pilot.exe
```

Select **Illustrator · AI (VectorCraft)** in Export layers, or import an `.ai` file
through Geometry file. Conversion is local and temporary; it does not use a cloud
service. A missing converter produces an error without changing the project.
Use SVG if you do not want to install or configure the converter.

AI exchanges layer geometry and optional artwork, not the complete editor project.
Use a `.pilot.json` project to preserve all settings. See the
[AI format contract and round-trip checks](../../interfaces/canvas-pilot-exchange.md#illustrator-ai).
