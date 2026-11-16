# Canvas Pilot geometry exchange

Canvas Pilot was formerly named Midra Layout. The rename preserves existing wire-format identifiers, `.midra.json` projects, settings/recovery locations and `MIDRA_VECTORCRAFT_CLI` configuration. No schema version changes are needed. New exports use Canvas Pilot in human-readable titles; legacy files remain readable.

The Export menu writes **SVG v2**, **draw.io v1**, **geometry JSON v1**,
**geometry CSV v1**, **DXF/DWG**, **HTML/CSS v1**, **PPTX**, **Figma FIG / FigCraft** and
**Illustrator AI** (the latter via a local VectorCraft CLI). Project
JSON is version 16 and is the full-fidelity save format. These exchanges
preserve layer **visible-box geometry**, not sources, crop settings, zoom, pivots,
Midra links, locks, visibility, camera or selection. Pictures illustrate current
cropping; importing a picture does not reconstruct its crop or source settings.

The [starter-file catalog](../templates/canvas-pilot/catalog.md) centralizes ready-to-open
two-layer templates under `docs/templates/canvas-pilot/` for Full HD (1920 × 1080) and UHD 4K
(3840 × 2160), in all eleven exchange formats (including PPTX, FIG and AI). The 4K geometry is an exact
2× scale of the Full HD layout. Regenerate them with:

```sh
cargo run --locked -p canvas-pilot --example check_geometry_exchange -- templates docs/templates/canvas-pilot
# Optional native codecs (default build); AI requires VectorCraft CLI:
MIDRA_VECTORCRAFT_CLI=/path/to/vectorcraft-cli cargo run --locked -p canvas-pilot --example check_geometry_exchange -- templates docs/templates/canvas-pilot pptx fig ai
```

The generator verifies imported geometry before writing each template and replaces
existing templates in the explicitly supplied directory. Samples contain only two
editable layer boxes, with no background or placeholder images. PPTX uses one slide,
FIG one frame, and AI one artboard. Their canvas sizes are pixels (PPTX uses the
documented 96 pixels/inch conversion). These files are geometry examples, not
full-fidelity project saves. AI samples were generated and round-trip checked with
VectorCraft CLI 0.24.1; Adobe Illustrator itself was not used for validation.
Native fixtures are checked semantically rather than byte-for-byte because encoder
metadata can vary. The AI regression check is opt-in and requires the real CLI:

```sh
MIDRA_VECTORCRAFT_CLI=/path/to/vectorcraft-cli cargo test --locked -p canvas-pilot --test exchange illustrator_readme_templates -- --ignored
```

## Application and format matrix

| Workflow | Exchange format | Import/export | Background | Placeholder pictures |
| --- | --- | --- | --- | --- |
| Inkscape, Illustrator, Affinity Designer, Figma, web vector tools | SVG | Yes | Embedded PNG | Embedded PNG |
| diagrams.net / draw.io | draw.io XML | Yes | Embedded PNG | Embedded PNG |
| Scripts, automation and future adapters | Geometry JSON | Yes | Embedded PNG | Embedded PNG |
| Excel, LibreOffice Calc and data tools | Geometry CSV | Yes | Geometry row | No |
| AutoCAD, LibreCAD and QCAD | DXF | Yes | Rectangle entity | No |
| Browsers and web layout tools | HTML/CSS | Yes | Embedded PNG | Embedded PNG |
| PowerPoint, LibreOffice Impress, DeckCraft | PPTX | Yes; slide picker on import | Embedded PNG | Embedded PNG |
| Figma, FigCraft | Native FIG | Yes; page/frame picker on import | Embedded PNG | Embedded PNG |
| FigCraft | Native FIGCRAFT | Yes; page/frame picker on import | Embedded PNG | Embedded PNG |
| Illustrator, VectorCraft | Native AI via VectorCraft CLI | Yes; artboard picker on import | Embedded PNG | Embedded PNG |

PSD, VSDX and ODG remain future adapters rather than implied support.
PDF is a useful final visual but does not retain a
reliable editable-rectangle contract for import, so it is not presented as a
round-trip format.

## Scope, coordinates and artwork

All formats offer Current layer (exactly one selected), Selected layers,
Specific layer (including hidden), All layers and Visible layers only. Stable IDs
are `layer-N`; order is document stacking order, independent of selection order.
All chosen layers are exported visibly, even if hidden in the editor. No layers,
unknown/duplicate IDs, invalid project geometry or dimensions rounding below one
pixel reject export. X/Y are top-left output pixels, relative to the **whole output
screen**, including for single-layer exports. X, Y, width and height round
independently to nearest integer; halves round away from zero. Negative positions
are retained. No stroke expansion is necessary: exported boxes have no outline.

**Export layers…** is one window: choose a format, artwork options and layer
scope. All codecs share project validation and shape preparation; the byte-oriented
`interchange` registry supplies formats, extensions and capabilities to the UI.
Importing files uses the same mapping review as pasted text. Multi-page files
require a page/frame/slide choice; changing it resets mappings. File parsing and
conversion run on workers so the viewport remains responsive. Files are written
only after encoding succeeds; existing destinations require replacement approval.

Options are shared for the current session, default off;
they do not modify the project/settings schema:

- **Include background layer** adds a first, full-screen shape with ID
  `midra-background`, explicitly marked as background. SVG, draw.io, JSON, HTML,
  PPTX, FIG and AI
  embed all visible references composited in project order over a dark backdrop
  using each image's fit/stretch/native-size, offset and opacity; otherwise they embed a plain dark
  backdrop. This is a baked visual reference, not an editable copy of reference
  settings. Image raster dimensions are proportional to the screen, capped at
  1920 pixels on the longest side; the exported shape still has the exact screen
  dimensions. CSV and DXF carry only the background rectangle.
- **Include placeholder pictures** uses embedded PNG source-grid patterns instead
  of flat layer fills. The pattern reflects native input cropping and advanced
  cropping, fitted into each visible box. Each raster is capped at 512 pixels on
  its longest side. No live source image is available/captured by this exporter.
  The images contain no external URLs and need no network access. Imported picture
  layers are still geometry-only. Without this option, the layers are rectangles.
  CSV and DXF disable this option because they cannot embed self-contained raster
  pictures in the supported profile.

The background is detected and ignored by default on import and cannot be mapped
as a content layer. A legitimate full-screen layer is kept as a layer. Every
placeholder is **one** importable layer, not an image plus duplicate rectangle.

## Optional native document codecs and licensing builds

The default `vendor_formats` Cargo feature uses the published, pinned
[`deckcraft-pptx 0.6.1`](https://crates.io/crates/deckcraft-pptx/0.6.1) and
[`figcraft-format 0.5.1`](https://crates.io/crates/figcraft-format/0.5.1) crates with
their document/geometry types. Their licenses are `MIT OR Apache-2.0`; vendor file
formats do not imply a proprietary implementation license. No upstream app UI,
font bundle, media engine or online service is embedded. VectorCraft's AI readers
are unpublished and pull in rendering, font and PDF stacks, so AI uses a separate
optional executable rather than embedding that stack. Upstream licenses still
apply to separately distributed tools; Midra does not bundle VectorCraft.

```sh
# Normal build: native PPTX/FIG/DWG codecs and the optional AI converter interface.
cargo build --locked --release -p canvas-pilot
# Excludes vendor codecs, their optional dependency trees and the AI interface.
cargo build --locked --release -p canvas-pilot --no-default-features --features no_proprietary
```

`no_proprietary` takes precedence in code and UI if combined with other features.
Cargo features are additive: the `--no-default-features` part is required to remove
the dependencies themselves. An `--all-features` build disables the adapters but
still compiles optional dependencies. CI tests both configurations; production
coverage uses defaults so native-codec code remains included in the denominator.
This is a format/dependency policy switch, not a legal certification.

### PPTX

Standard OOXML ZIP, one slide matching the screen; 96 screen pixels per inch,
or 0.75 points per pixel (9525 EMU/px). Native shapes/pictures preserve editable
names and boxes. The background is named `midra-background`, a reserved marker
excluded from mapping. PNGs are embedded as media; no source URLs are used.
PPTX/FIG/FIGCRAFT export asks to rename a content layer with that reserved name so it
cannot silently become an ignored background on reimport.
Stable Midra device/source IDs are not represented in the native file.

Import lists each readable slide independently. Shapes and pictures use their
explicit transforms, recursively composing group coordinate transforms. Rotation
and flips produce axis-aligned enclosing boxes. Hidden, zero-sized, unpositioned
and unsupported opaque/connector objects are omitted with a count in the review.
Masters, inherited placeholders, text content, animations, media playback and
source/crop semantics are not imported. The note reports slide dimensions and
the fixed unit convention; importing never silently resizes the Midra output.

### Figma FIG

Import/export manually reviewed on 2026-11-15.

FigCraft writes a native `.fig` ZIP containing the Kiwi-encoded canvas and embedded
image assets. Export creates a `Canvas Pilot` frame with screen dimensions and
rectangle children (image fills when requested). Coordinates are Figma pixels.
The background uses the same reserved `midra-background` name. The upstream
codec is best effort; warnings on export are errors rather than silent losses.

Import offers top-level frames/components as separate canvases, relative to their
own origin, and pages with loose objects. Visible leaf objects are mapped by their
transformed enclosing boxes; nested containers are traversed without duplicating
their bounding rectangle. Hidden children and zero-sized objects are omitted.
Upstream approximation warnings appear in the mapping review. Constraints,
components, auto-layout rules, text and styling do not become Midra parameters.
Native round-trip tests verify geometry/artwork with the pinned upstream codec;
they do not constitute validation against every Figma service version.

### FigCraft FIGCRAFT

Import/export manually reviewed on 2026-11-15.

The separate `.figcraft` option uses the pinned `figcraft-format` crate's native
version 1 reader/writer, not a renamed `.fig`. Its ZIP contains `document.json`
(with `format: "figcraft"` and `version: 1`) and image assets under `images/`.
Import also accepts the native bare JSON representation with a `.figcraft`
extension; unsupported future versions are rejected by the upstream reader.

Geometry, page/frame selection, reserved background names and artwork follow the
FIG contract above. Export rounds positions and sizes to whole screen pixels;
import retains fractional coordinates. One output-sized frame contains the named
layer rectangles. Background and placeholder pictures are optional. Source, crop, fit, mask,
mapping, inactive input proposals and workspace settings can be stored in the
optional [extra-parameter snapshot](midra-extra-parameters.md). Plain geometry import
does not restore these settings. FigCraft history, branches and library data are not imported into
Midra or recreated on export. Reader repair warnings appear in the import review.
This option shares the FIG vendor feature and is excluded by `no_proprietary`.
Tests cover native resaving, bare JSON, geometry, artwork and both starter sizes.

### Illustrator AI

Optional compatibility target: **prefer SVG for Illustrator exchange**. AI import
and export require a separately installed VectorCraft CLI; it remains external
and is not embedded or bundled. See the [manual setup instructions](../canvas-pilot/guide/files.md#illustrator-ai-optional-converter-required).

Uses [VectorCraft CLI](https://github.com/storytold/vectorcraft), discovered through
`MIDRA_VECTORCRAFT_CLI`, beside the Midra executable, in its `bin/` subdirectory,
then on PATH. Conversion runs locally in safe mode in a temporary directory, with
a 60-second timeout and 16 MiB result limit. Nothing is installed automatically.
The CLI reads AI and converts to SVG for the shared geometry importer, or writes
PDF-compatible AI from Midra SVG. Import enumerates up to 64 artboards and presents
each separately in the same canvas selector. Converter import warnings are shown
in the mapping review; export warnings reject the result instead of silently
losing content. SVG's rectangle/path/image restrictions
still apply; this is layer-box exchange, not arbitrary illustration reconstruction.

Manual native-file generation and round-trip diagnostics (no device access):

```sh
cargo run --locked -p canvas-pilot --example check_geometry_exchange -- native /tmp/midra-native pptx fig
MIDRA_VECTORCRAFT_CLI=/path/to/vectorcraft-cli cargo run --locked -p canvas-pilot --example check_geometry_exchange -- native /tmp/midra-ai ai
```

These generate plain and illustrated documents, with backgrounds, and verify the
imported layer boxes. Existing diagnostic files are not overwritten. VectorCraft
0.24.1 native AI round trips were checked for both variants. DeckCraft 0.8.0
rendered the exported PPTX independently of the pinned 0.6.1 codec; the resulting
Full HD image was visually inspected.

## SVG v2

Example: [canvas-pilot-svg-v2.example.svg](canvas-pilot-svg-v2.example.svg).
UTF-8 SVG; SVG namespace and optional XLink namespace. Root attributes:

- `data-midra-layout-geometry="2"`
- `width`, `height`: output screen dimensions in pixels
- `viewBox="0 0 W H"`

Each layer is a `<rect>` (plain) or `<image>` (placeholder), with `id="layer-N"`,
`data-midra-role="layer"`, `x`, `y`, `width`, `height`. A `<title>` carries its name
for external viewers; Midra import does not rename destinations. Images use
`xlink:href="data:image/png;base64,..."` and `preserveAspectRatio="none"`.
The optional background is an `<image>` with `data-midra-role="background"` and
full-page bounds. No visible text geometry, transforms, clipping or CSS is needed.

The importer supports v1 and v2 and explicitly rejects other marked versions.
VectorCraft-derived SVG may omit the root version and retain `data-midra-role`
on wrapping groups. The importer inherits these role markers and their stable
IDs; marked image layers remain importable and backgrounds remain excluded.
Explicit unsupported root versions are still refused.
Legacy `Project::to_svg`/`to_svg_ids` continue producing v1 rectangle-only SVG;
the GUI uses v2. Old applications cannot import v2 placeholder images; use JSON
or a plain rectangle-only export where required. Editors should retain role
markers. If they remove them, ordinary image import remains intentionally limited
to a first full-page background. Midra never fetches image URLs during import.

## draw.io v1

Example: [canvas-pilot-drawio-v1.example.drawio](canvas-pilot-drawio-v1.example.drawio).
UTF-8 uncompressed `<mxfile compressed="false">` with one `<diagram>` containing
an `<mxGraphModel data-midra-layout-geometry="1">`. `pageWidth`, `pageHeight`,
`pageScale="1"`, and `page="1"` establish the output canvas; diagram units are pixels.
Root cells `0` and `1` provide the required graph root and default layer.

Each exported shape is an `<mxCell vertex="1" parent="1">` with stable `id`,
escaped plain-text `value` (layer name), and `data-midra-role="layer"` or
`"background"`. Its single `<mxGeometry as="geometry">` stores pixel X/Y/width/
height. Styles disable outlines and rounding. Pictures use `shape=image`,
`imageAspect=0`, and draw.io's `image=data:image/png,BASE64` style syntax (without
`;base64`, which would split the style). Background label text is empty.

Import accepts raw/once-percent-encoded mxGraphModel clipboard XML, single-page
mxfile XML, and native compressed diagram text: Base64 → raw DEFLATE → percent
decode → mxGraphModel. Native re-saving may remove the model's Midra version
attribute; role attributes and stable IDs survive in the tested version. The
reserved `midra-background` ID also identifies the exported background. Unknown
marked draw.io exchange versions are rejected. Multiple pages are refused with an
instruction to export one page, rather than silently importing the wrong page.
Groups, edges, rotation, flips, relative/alternate bounds and nonrectangular
shapes remain unsupported; ordinary unrotated image cells now import their box.

## Geometry JSON v1

UTF-8 JSON with root fields `format: "beffroi.midra-geometry"`, `version: 1`,
`units: "px"`, `canvas: {width,height}` and `layers`. Each layer has `id`, `name`,
`role`, `x`, `y`, `width` and `height`; `image_png_base64` is optional. Roles are
`layer` or `background`. Unknown fields, versions and units are rejected. IDs
must be unique and at most 255 characters; names are limited to 1024 characters
without controls. This JSON is the preferred small machine interface. It is not
the version-10 project save file and does not contain sources, crop or device links.

## Geometry CSV v1

RFC 4180 CSV with CRLF records and this fixed header:

```text
midra_geometry_version,canvas_width,canvas_height,id,name,role,x,y,width,height
```

Every row repeats version `1` and the same canvas so a selected row remains
self-describing. Names use normal CSV quoting. Roles and geometry have the same
meaning as JSON. Empty files, mismatched canvases, extra/missing fields and
non-integer geometry are rejected. CSV carries background geometry but no image
data or placeholder pictures.

## CAD: DXF and DWG profiles

Both DXF and DWG import/export were manually reviewed on 2026-11-15.

DXF uses the official shared `craft-dxf` document model and serializer, pinned to
Craft libraries revision `79e97b6febcb62de406e5bcf5a724bd3f7333065` (MIT OR Apache-2.0).
It writes UTF-8 ASCII DXF `AC1021` (AutoCAD 2007), with complete symbol tables,
model/paper-space block records, dictionaries, layouts, unique handles, owner
references and subclass markers. Handles reserve the low `0x100` range to avoid
QCAD stock-object collisions; Craft remaps definitions and pointers together. This replaces the incomplete hand-written DXF
skeleton rejected by QCAD. The rectangle/coordinate contract remains unchanged;
older Midra DXF files are still readable.

Drawings are unitless (`$INSUNITS=0`): one CAD unit equals one screen pixel.
Whole-pixel `$LIMMIN`/`$LIMMAX` describe the canvas, with matching initial extents.
Every stable ID has a layer-table entry; every box is one closed four-vertex
`LWPOLYLINE`. CAD Y increases upward, so `cad_y = canvas_height - screen_y`.
Import reverses this conversion and requires explicit canvas limits (falling
back to extents), rather than guessing scale or page origin. The special
`midra-background` layer is excluded from destination mapping. Human names are
layer descriptions; imported IDs remain CAD layer names. Pictures are omitted.
DXF import uses Craft's bounded ASCII/binary tag reader and retains the existing
axis-aligned rectangle restrictions.

DWG uses the optional `craft-dwg` byte bridge from the same pinned revision,
backed by unmodified `acadrust` 0.6.6 (MPL-2.0). It converts the same DXF model to
AutoCAD 2007 DWG and back; canvas limits, IDs, pixel rounding and background rules
are identical. A writer report of omitted records rejects export. Reader notes
appear in the import review. Both DWG input and expanded DXF are limited to
16 MiB. DWG is available with `vendor_formats`, excluded by `no_proprietary`;
DXF remains available in all builds. The bridge reports unsupported platforms
explicitly (WebAssembly and FreeBSD ARM64 at this pin).

Geometry exchange is not a general CAD editor: arbitrary drawing entities,
blocks and drafting annotations are not a supported round-trip profile. Keep
original CAD drawings; import/export the rectangular layer layout separately.

## HTML/CSS v1

The XHTML-compatible document is a directly viewable page. Root
`data-midra-layout-geometry="1"` marks the version; `#midra-canvas` stores
`data-width`/`data-height`. Each `<div>` or `<img>` stores authoritative
`data-midra-role`, `data-name`, `data-x`, `data-y`, `data-width` and
`data-height`; inline CSS renders the same box. Images use self-contained PNG data
URLs. Import reads the data attributes, not computed CSS, scripts or browser
layout. The file contains no script or external resource.

## Limits, import and round trips

**Manual review (2026-11-14):** PPTX, draw.io and SVG workflows were confirmed
working. Their reported format issues are marked solved in
the [task list](../canvas-pilot/plan.md#completed-acceptance).

Import files and clipboard text, and generated exports, are capped at **16 MiB**
(to allow self-contained images). Decompression is bounded: at most 48 MiB of
percent-encoded text and 16 MiB decoded XML. Up to 256 layer boxes plus one
background are allowed. No imported image is decoded or downloaded by the
geometry importer. Export reference decoding retains the existing 8192-pixel
per-side/256 MiB decoder bounds. Oversized exports fail before writing a file.

Use **Import → Geometry file…**, drop supported files, or paste supported text,
then review and map shapes to editor layers. Drops route case-insensitive registered
extensions (including draw.io XML and vendor formats) through the same asynchronous
byte importer as the picker; native-codec build restrictions still apply. Path-backed
and in-memory file drops are supported. Raster images retain reference-image behavior.
Several geometry files become named choices in the file/page/frame/slide selector,
with a combined 16 MiB input limit. Parsing is atomic across the geometry batch;
errors report the failed filename, and no geometry is applied before mapping confirmation. Destination count need not match current selection. The review offers All / Selected /
Folder destination scopes without changing canvas selection; hidden and locked layers
are unavailable. Every included item needs a distinct destination, while excluded
items and unmapped editor layers remain unchanged. Existing mappings survive scope
changes when still eligible. Clipboard text and per-item geometry details are
collapsed, read-only previews. These review choices are session-only and do not
change any file schema. Import with
size and position enabled restores the rounded geometry in one Undo step. Output
screen resolution is not imported. Mapped names, sources and Midra links stay with
the destination layers. File replacement retains the existing explicit overwrite
confirmation and atomic-write path.

SVG files rendered by draw.io Desktop are also accepted. The importer recognizes
the `data-cell-id` wrapper groups retained by draw.io, reads the first rectangle,
image or symbol use in each cell, and ignores generated HTML labels, raster text
fallbacks, symbols and the draw.io help link. The standard SVG 1.1 public DOCTYPE
emitted by draw.io is accepted without fetching external content; XML parsing is
limited to 200,000 nodes. The reserved `midra-background` cell remains an ignored
background.

Draw.io's SVG export normally crops the canvas to the diagram content, adds one
pixel to its bounds and rebases cell coordinates. Those rebased coordinates are
the only positions present in that SVG. Include Canvas Pilot's background layer
before exporting through draw.io when the full output-screen origin must survive
the round trip.

Tests cover each format/artwork combination, the twelve text templates and eight native PPTX/FIG/FIGCRAFT/DWG templates (plus two AI templates in the opt-in CLI check),
full-screen layers vs backgrounds,
negative/fractional coordinates, repeat round trips, compressed clipboard/files,
embedded references, invalid IDs/versions and no-overwrite file handling.
A manual native-editor check is available:

```sh
cargo run --locked -p canvas-pilot --example check_geometry_exchange -- generate /tmp/midra-exchange-check
drawio -x --size page -o /tmp/midra-exchange-check/render.png /tmp/midra-exchange-check/illustrated.drawio
drawio -x -o /tmp/midra-exchange-check/resaved.xml /tmp/midra-exchange-check/illustrated.drawio
drawio -x -f svg -o /tmp/midra-exchange-check/drawio-rendered.svg /tmp/midra-exchange-check/illustrated.drawio
cargo run --locked -p canvas-pilot --example check_geometry_exchange -- verify /tmp/midra-exchange-check/resaved.xml
cargo run --locked -p canvas-pilot --example check_geometry_exchange -- verify /tmp/midra-exchange-check/drawio-rendered.svg
```

On 2026-11-14, installed **draw.io Desktop 31.7.0** (`/usr/bin/drawio`, not Flatpak)
rendered the illustrated diagram correctly, and its re-saved XML returned both
layer boxes unchanged. Its SVG export, including `<symbol>/<use>` image cells and
HTML/raster label fallbacks, also returned both layer boxes unchanged when the
background preserved the page. Generated plain/illustrated SVG and draw.io files
passed geometry verification. Native output artifacts remain in `/tmp`, not Git.

Protocol references: [draw.io diagram format](https://www.drawio.com/docs/reference/diagram-generation/)
and [desktop CLI](https://github.com/jgraph/drawio/wiki/Desktop-App),
[SVG 2](https://www.w3.org/TR/SVG/), [CSV RFC 4180](https://datatracker.ietf.org/doc/rfc4180/),
and Autodesk's [DXF group-code reference](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-DXF/files/GUID-89CB823D-614D-4D1E-8204-568EC72DF869.htm).

Multiple references are flattened into the same single background image in
artwork-capable exports; no format version changes. Reimport reconstructs that
background as a geometry/artwork item, not the editable reference collection.
Use project JSON v15 to preserve independent images and visibility.

### QCAD interoperability check

Validated with QCAD Professional 3.28.2: both 1080p and 4K DXF/DWG templates
render two correctly placed rectangles without import/handle warnings. Saving
both Full HD samples through QCAD as R21 (2007) and reimporting preserves both
layer boxes. Example manual check (temporary output only):

```sh
QT_QPA_PLATFORM=offscreen /path/to/qcad/dwg2dwg -r R21 -o /tmp/layout.dwg docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.dwg
cargo run --locked -p canvas-pilot --example check_geometry_exchange -- verify /tmp/layout.dwg
```

The verifier now uses the unified byte importer for text and binary formats.
The `.ai` starter ZIP is only a download wrapper, not a new import format; its
two entries must remain byte-identical to the `.ai` fixtures (Python regression).

## Optional extra Canvas Pilot parameters

Export layers offers **Include extra Canvas Pilot parameters (experimental)** for SVG,
draw.io, JSON, CSV, DXF, HTML, PPTX and FigCraft. This embeds an inert snapshot separately
from the artwork. JSON/CSV use geometry version 2 only when enabled; plain exports
remain version 1. Other carrier versions remain unchanged. Existing omissions in
this document describe plain geometry export; the optional snapshot preserves
source/crop, identity, links and workspace settings, excluding image bytes and
capture bindings. See the [complete carrier contract and format audit](midra-extra-parameters.md).

Restore the original snapshot with **Project → Restore parameters from export…**.
Normal geometry import continues to use the artwork's current boxes, including any
external edits. Third-party editors may strip metadata. DWG, FIG and AI are not
enabled until their current codec paths can preserve it reliably.

Embedded project snapshots now contain project version 16 and family-aware `device_link` objects. Geometry representation versions are unchanged. Existing embedded versions 1–15 migrate using the [project contract](canvas-pilot.md); plain geometry continues to omit device destinations.

Project v14 additionally preserves `layer_fit` in embedded snapshots. Geometry
alone describes the layer container when explicit fit is enabled and omits crop
and fit behavior. Optional placeholder artwork renders the fitted image clipped
to the container, including transparent margins for Centered or small 1:1 content.

Project v15 also preserves normalized layer masks in embedded snapshots. Placeholder
artwork blanks pixels transparently without resampling. Plain rectangle geometry
continues to omit mask appearance; use project JSON for exact round trips.
