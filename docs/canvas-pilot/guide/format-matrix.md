# Choosing a file or clipboard format

[Guide contents](overview.md) · [Import and export instructions](files.md) · [Starter files](../../templates/canvas-pilot/catalog.md)

Use **Save project** to continue editing later. Use **SVG** for straightforward
vector-editor exchange, **draw.io** for diagrams.net, **PPTX** for slides, and
**FIG / FIGCRAFT** for Figma / FigCraft files. Prefer **SVG over AI** for Illustrator
unless you specifically need a native AI file. **DXF / DWG** suit CAD geometry;
**JSON / CSV** suit scripts and tables. HTML provides a browser-viewable layout.

These tables describe what **Canvas Pilot implements**, not everything each file
format can theoretically contain. ✅ means supported; ❌ means unavailable in this
workflow. Linked numbers refer to the notes below each table. These emoji are
a reference, not interactive controls.

## File exchange matrix

All eleven exchange formats import and export layer positions and sizes. Geometry
import maps boxes to **existing destination layers**: it keeps their names, source
settings and device links, and does not change the output resolution. File names
and layer labels can help identify the boxes; they do not rename destinations.

| File | Import | Export | Layer names in export | Background picture [1](#file-notes) | Crop/mask preview pictures [1](#file-notes) | Extra settings snapshot [4](#file-notes) |
| --- | :---: | :---: | :---: | :---: | :---: | :---: |
| Canvas Pilot project (`.pilot.json`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| SVG (`.svg`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| draw.io (`.drawio`, XML) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Geometry JSON (`.json`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Geometry CSV (`.csv`) | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ |
| DXF (`.dxf`) | ✅ | ✅ | ✅ [2](#file-notes) | ❌ | ❌ | ✅ |
| DWG (`.dwg`) | ✅ [3](#file-notes) | ✅ [3](#file-notes) | ✅ [2](#file-notes) | ❌ | ❌ | ❌ |
| HTML / CSS (`.html`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| PowerPoint (`.pptx`) | ✅ [3](#file-notes) | ✅ [3](#file-notes) | ✅ | ✅ | ✅ | ✅ |
| Figma file (`.fig`) | ✅ [3](#file-notes) | ✅ [3](#file-notes) | ✅ | ✅ | ✅ | ❌ |
| FigCraft file (`.figcraft`) | ✅ [3](#file-notes) | ✅ [3](#file-notes) | ✅ | ✅ | ✅ | ❌ |
| Illustrator file (`.ai`) | ✅ [3](#file-notes) | ✅ [3](#file-notes) | ✅ [2](#file-notes) | ✅ | ✅ | ❌ |

### File notes

1. **Pictures:** enable **Include background layer** and/or **Include placeholder pictures** when exporting.
2. **Names:** exported as format-specific labels/metadata. CAD uses stable layer
   IDs with human names in descriptions; AI goes through an SVG conversion.
   Another application's layer panel may display these differently.
3. **Native codecs:** DWG, PPTX, FIG, FIGCRAFT and AI require a build with vendor
   formats enabled (the default). AI additionally requires the separately installed
   [VectorCraft converter](files.md#illustrator-ai-optional-converter-required).
4. **Extra settings:** experimental and opt-in. Restore the original snapshot with
   **Project → Restore parameters from export…**; ordinary geometry import never
   applies it. External applications may strip it or leave it stale after edits.

Exports use one screen-sized canvas, slide, frame or artboard. PPTX import offers
a slide picker; FIG / FIGCRAFT offer page/frame choices; AI offers artboards.
Draw.io supports single-page files, including compressed files. These are supported
layout profiles, not general-purpose artwork importers: arbitrary HTML, SVG effects,
CAD entities or complex native document features may be rejected or omitted.
See [file import limits](files.md) before choosing a workflow for complex artwork.

## Clipboard matrix

Figma file import and Figma copy/paste are different paths. The same distinction
applies to Illustrator and draw.io. “Paste” below means **into Canvas Pilot**;
“Copy out” means a clipboard format Canvas Pilot itself generates.

| Clipboard path | Paste positions and sizes | Copy out in this format | Rename destination layers | Crop zoom | Crop edges / offsets, fit and masks |
| --- | :---: | :---: | :---: | :---: | :---: |
| Canvas Pilot **Copy layer values**, normal crop | ✅ | ✅ | ❌ | ✅ | ❌ |
| Figma **SVG text** copy/paste | ✅ | ❌ | ❌ | ❌ | ❌ |
| Illustrator **SVG text** copy/paste | ✅ | ❌ | ❌ | ❌ | ❌ |
| draw.io diagram XML copy/paste | ✅ | ❌ | ❌ | ❌ | ❌ |
| SVG file contents pasted as text | ✅ | ❌ | ❌ | ❌ | ❌ |
| Geometry JSON / CSV file contents pasted as text | ✅ | ❌ | ❌ | ❌ | ❌ |
| Canvas Pilot HTML / ASCII DXF file contents pasted as text | ✅ | ❌ | ❌ | ❌ | ❌ |
| PowerPoint native clipboard | ❌ | ❌ | ❌ | ❌ | ❌ |
| QCAD native clipboard | ❌ | ❌ | ❌ | ❌ | ❌ |

Figma SVG, Illustrator SVG and draw.io XML clipboard paths have been confirmed in
user testing. This does not imply support for every application's native binary
or HTML clipboard payload. Use the corresponding file path if no supported text
is available. PowerPoint and QCAD use file exchange in this workflow.

Paste opens a mapping review with separate **Position**, **Format** (width/height)
and, when present, **Crop zoom** choices. Names in copied values identify rows;
they do not rename destination layers. Zoom alone does not reproduce the original
crop offsets or source settings. Older Canvas Pilot values may omit zoom.
**Advanced-fit container values are not the normal-crop paste format**; use a
project or supported snapshot to retain those modes.

SVG and draw.io imports can optionally include supported outline widths in the
bounds. Colors, text styling and artwork are not applied to the destination.
Pasting exported text with extra parameters still imports geometry; use the
separate restore action to recover the embedded settings.

## Which editing features survive?

This table separates editable state from a picture of its appearance. “Snapshot”
means one of the seven checked exchange formats above, exported with extra
parameters and reopened through **Restore parameters from export**.

| Feature | Project Open/Save | Plain geometry file import | Snapshot restore |
| --- | :---: | :---: | :---: |
| Layer position and size | ✅ | ✅ | ✅ |
| Full-precision saved coordinates | ✅ | ❌ [5](#editing-notes) | ✅ |
| Output screen resolution | ✅ | ❌ | ✅ |
| Layer names and stacking order | ✅ | ❌ | ✅ |
| Folders, collapsed folders, visibility and locks | ✅ | ❌ | ✅ [6](#editing-notes) |
| Source formats, assignments and shared input relationships | ✅ | ❌ | ✅ [6](#editing-notes) |
| Input crop edges | ✅ | ❌ | ✅ |
| Normal layer crop zoom and offsets | ✅ | ❌ | ✅ |
| Advanced fit modes and their crop edges | ✅ | ❌ | ✅ |
| Editable blanking masks | ✅ | ❌ | ✅ |
| Resize pivots and group resize settings | ✅ | ❌ | ✅ |
| Saved device screen/layer links | ✅ | ❌ | ✅ |
| Reference images and their individual settings | ✅ | ❌ | ❌ |
| Capture assignments and saved preview stills | ✅ | ❌ | ❌ |
| Selection, camera, guides, snapping and workspace settings | ✅ | ❌ | ✅ [6](#editing-notes) |
| Undo history, pending edits and active device operations | ❌ | ❌ | ❌ |
| Application theme, connection history and Program permission | ❌ [7](#editing-notes) | ❌ | ❌ |

### Editing notes

5. Geometry **exports round boxes to whole pixels**; import can retain fractional
   coordinates where the reader supports them. Use a project or snapshot for
   full-precision recovery. Geometry import never reorders destination layers.
6. Snapshot restoration includes only exported layers and their referenced inputs;
   selection and folders are filtered accordingly. It replaces the document with
   the saved snapshot, not a merge of the external artwork's latest edits.
7. Application preferences are saved separately from projects. Opening a project
   or restoring a snapshot does not connect or send to a switcher.

PNG, JPEG and BMP are **reference-image imports**, not layer-geometry formats.
  PDF, PSD, VSDX and ODG are not supported geometry-exchange choices.

For a complete editable backup, **Save project** remains the recommended choice.
