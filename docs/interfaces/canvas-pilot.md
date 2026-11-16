# Canvas Pilot file interfaces

Canvas Pilot was formerly named Midra Layout. The rename preserves existing wire-format identifiers, `.midra.json` projects, settings/recovery locations and `MIDRA_VECTORCRAFT_CLI` configuration. The branding rename alone did not change schemas. New exports use Canvas Pilot in human-readable titles; legacy files remain readable.

These are offline editor formats, not Analog Way REST payloads. Implementation:
[`project.rs`](../../crates/canvas-pilot/src/project.rs). Regression tests:
[`project.rs`](../../crates/canvas-pilot/tests/project.rs). Update this contract,
examples and tests in the same change as any save/export schema change.

## Project JSON, version 17

Suggested filename: `layout.pilot.json`. **Open project** accepts any `.json`
filename, including legacy `.midra.json` projects, and validates the project
representation inside it. Unrelated or invalid JSON is rejected.
The file dialog accepts `.json`, and validation uses file contents rather than the
filename. **Save project…** suggests the new name and lets the user choose the path;
existing files are not renamed automatically. This filename change requires no schema migration.

Encoding: UTF-8, pretty-printed JSON,
with a trailing newline. A project preserves applied document values at their
stored floating-point precision. No display rounding is applied. The loader
requires every field below, rejects unknown fields, and rejects unsupported
versions. Version 1 files first gain
`editor.show_toolbar_icon_names: false`. Versions 1 and 2 then migrate to version 3
with their original two layer IDs, names `Presenter` and `Slides`, empty folders,
and `next_layer_id: 3`; other values are preserved.
The [version 1 example](canvas-pilot-v1.example.json) remains a migration fixture.
Version 4 adds `editor.outline_layers: false`, and reference `native_size: false`
and `offset: [0, 0]` when migrating versions 1–3. Old fitting behavior is preserved.
Version 5 adds required `midra_link` to every layer. Versions 1–4 migrate with null links; links cannot be supplied under an older version. Version 6 renames `editor.position_corner` to `editor.position_point` and adds `center`. Versions 1–5 retain their original corner; center and the new field are rejected in those older schemas. Version 7 adds `editor.backdrop_title`; versions 1–6 gain `"Example Conference Name"` and cannot supply this field under the old schema. Version 8 adds required `editor.position_layer_point`: null keeps matched points; a point enables independent measurement. Versions 1–7 migrate to null and cannot supply this field. Version 9 adds required `layers[].pivot`; versions 1–8 migrate to `[0.5, 0.5]` and cannot supply this field. Version 10 replaces `reference` with the required ordered `references` array. Versions 1–9 migrate null to `[]` and a single image to a one-element array, preserving all image settings. Version 11 adds required `workspace` settings and embedded layer previews. Versions 1–10 gain the documented workspace defaults, with no captures or collapsed folders; supplying `workspace` in an older schema is rejected. Version 12 adds required `folders`, the explicit folder names that survive without members. Versions 1–11 gain `folders: []`, preserving their existing implicit layer grouping; supplying this field under an older version is rejected. Version 13 replaces `midra_link` with required `device_link`, adding the explicit device family. Versions 1–12 migrate numeric links to `family: "midra4k"`; null remains null, and legacy decimal IDs with leading zeros are canonicalized. Unknown legacy fields are rejected. Version 14 adds required `layers[].layer_fit`: versions 1–13 migrate to null, preserving normal crop/zoom behavior; supplying this field under an older schema is rejected. Version 15 adds required `layers[].mask`; versions 1–14 migrate with four zero edges and reject masks supplied under an older version. Version 16 adds required root `input_proposals`; versions 1–15 migrate to an empty array and reject this field if supplied under an older version. Version 17 adds required `mask.origin` (`layer` or `source`). Versions 1–16 migrate to `layer`; older versions containing `origin` are rejected. See the [version 17 example](canvas-pilot-v17.example.json); v16 remains a migration fixture. Saving always writes version 17; older applications reject it.
Incompatible files are never partially applied.

[Complete example](canvas-pilot-v17.example.json) is a valid project with an empty `Memory A` folder, a Midra link and an inactive input correction proposal,
with an unlinked Fullscreen example layer and no reference image. New projects label the lower layer `02 / Slides` and
the upper layer `01 / Presenter`; existing projects retain their saved IDs/names.
It is loaded by the regression tests.

| Root field | Representation / meaning |
| --- | --- |
| `format` | Exactly `"beffroi.midra-layout"` |
| `version` | Integer `17` |
| `input_proposals` | Required array, 0–256 inactive correction proposals; at most one per existing layer, with an exactly matching `device_link`. See below. |
| `folders` | Required array of 0–256 unique explicit folder names, each trimmed and 1–128 characters. Explicit folders remain even with no members; layer folder labels also form implicit groups. |
| `next_layer_id` | Positive integer strictly greater than every layer ID; not rewound by undo |
| `output_resolution` | `{ "width": integer, "height": integer }`, screen pixels |
| `inputs` | 1–1024 input objects, consecutive IDs starting at 1 |
| `layers` | 1–256 layer objects, unique positive stable IDs, in back-to-front stacking order (last is topmost) |
| `references` | Required array of 0–32 embedded reference-image objects, back-to-front compositing order |
| `editor` | Selection, positioning and viewport preferences |
| `workspace` | Required working setup and layer previews, described below |

### Inactive input correction proposals

Each object has exactly `layer_id` (stable layer ID), `device_link` (the layer's
Midra link), `input` (`INPUT_` plus a canonical positive decimal ID), `plug`
(canonical positive decimal ID) and `settings` (1–9 whitelisted suffix/value pairs).
IDs have at most 16 digits; only the Midra adapter currently supports proposals.
Unknown fields, paths, values, duplicates or mismatched links are rejected.

| Setting suffix | Value / units |
| --- | --- |
| `aspect/pp/signal` | `NATIVE`, `5_4`, `4_3`, `16_10`, `15_9`, `16_9`, `21_9`, `256_135`, `64_27` |
| `aspect/pp/transformTo` | `SIGNAL`, `5_4`, `4_3`, `16_10`, `15_9`, `16_9`, `21_9`, `CUSTOM` |
| `aspect/pp/customRatio` | Integer 250–10000, thousandths of width/height ratio; 1778 means 1.778:1 |
| `aspect/pp/layerFill` | `LAYER_SETTING`, `1_1`, `CENTERED`, `FULLSCREEN`, `CROPPED` |
| `cropping/control/pp/predefined` | `NONE`, `LETTERBOX_1_78`, `LETTERBOX_1_85`, `LETTERBOX_2_35`, `PILLARBOX_1_33` |
| `cropping/control/pp/left`, `right`, `top`, `bottom` | Integer 0–8191, input pixels; each edge uses the full prefix |

Proposals are explicitly saved, never inferred from connected-device defaults.
Opening, recovery and snapshot restoration do not enable or stage them. Staging
requires explicit permission, a matching mapped Preview input/plug and every
referenced field in the current snapshot; Send remains separate and preflighted.
They contain no host, authentication, source-routing command or update pulse.
A proposal is a reviewable suggestion, not proof of the connected unit's identity.
Deleting/remapping a layer filters its proposal from subsequent saves. Exports keep
only proposals for included layers. Values retain integer precision.

Input objects:

| Field | Representation / meaning |
| --- | --- |
| `id` | Consecutive input slot ID; not an external device identifier |
| `source_format` | `"four_three"`, `"sixteen_nine"`, `"sixteen_ten"`, `"square"`, or `"custom"` |
| `native_resolution` | `{ "width": integer, "height": integer }`, native input pixels |
| `input_crop` | `{ "left": number, "right": number, "top": number, "bottom": number }`, pixels removed from the native input |

The user-facing input resolution is derived by subtracting opposite input crop
edges from native dimensions. The source format remains explicit even with
custom input cropping. Non-custom source formats must match the native resolution
within the existing half-pixel rounding tolerance.

Layer objects:

| Field | Representation / meaning |
| --- | --- |
| `id` | Unique positive stable editor layer ID; independent of list position and Midra mapping |
| `name` | Nonblank display name, at most 128 characters |
| `layer_fit` | Required: null for normal visible-box crop/zoom, or `"one_to_one"`, `"centered"`, `"fullscreen"`, `"cropped"`. Explicit modes treat bounds as the container; crop is applied before fitting. Unknown modes are rejected. Full-precision crop and container values survive project, recovery and embedded-project round trips. |
| `device_link` | Required: null (unlinked) or `{ "family": "midra4k", "screen": "2", "layer": "1" }` / `{ "family": "aquilon", "screen": "S1", "layer": "1" }`. Family is required; unknown fields/families rejected. Layer IDs and Midra screen IDs are canonical positive decimal strings, at most 16 digits; Aquilon screens require `S` (screen) or `A` (auxiliary) plus that numeric ID. No host, bank, credentials or unit identity: linked Aquilon systems are unsupported. |
| `pivot` | Required `[x, y]` gravity point relative to the layer top-left, in fractions of width/height. Center is `[0.5, 0.5]`; outside points are allowed. Finite doubles within ±1,000,000, preserved without rounding. Placement/reset is undoable; follows layer movement/reordering. Omitted from SVG and device writes. |
| `folder` | Local folder name, at most 128 characters; empty means ungrouped |
| `source_id` | Existing input slot ID; multiple layers may share an input |
| `bounds` | `{ "x": number, "y": number, "width": number, "height": number }`, visible box for normal crop/zoom, container for explicit `layer_fit`, in screen pixels; x/y are its top-left position |
| `mask` | Required `{ "origin": "layer" or "source", "left": number, "right": number, "top": number, "bottom": number }`: finite fractions of the selected reference width/height, each 0–1 and opposite sums ≤1. Zero means no blanking; meeting edges blank the entire layer. Layer-relative masks are independent of crop/fit. Source-relative masks follow the usable source before layer crop/fit. Values remain proportional on resize and retain full floating-point precision. |
| `advanced_crop` | Four nonnegative edges in usable input pixels, after input cropping |
| `visible` | Boolean editor visibility |
| `locked` | Boolean editor selection/movement protection |

The visible box is the user's layer resolution. Its content zoom and offset are
**derived** from `advanced_crop`, usable input size and visible box size; they are
not duplicated in the save file. Retained source pixels must have the same ratio
as the box. Crop zoom must be 1–100 (100–10000%). Shared input assignments and
both crop stages survive round trips without rewriting the crop values.

Each reference object:

| Field | Representation / meaning |
| --- | --- |
| `name` | Display filename, not a path to read |
| `image_base64` | Standard padded RFC 4648 base64 of the original PNG/JPEG/BMP file bytes; no data-URL prefix |
| `visible` | Boolean |
| `opacity` | Number from 0 to 1 |
| `preserve_ratio` | Boolean: fit proportionally if true, stretch to screen if false; applies when native sizing is off |
| `native_size` | Boolean: image pixels map 1:1 to screen pixels; anchored at screen top-left before offset |
| `offset` | `[x, y]` finite screen-pixel translation, each within ±1,000,000; applies after fitting/native placement |

Images are embedded, so moving the project does not break the references. Each
image retains independent visibility, opacity and fitting settings. Later entries
paint over earlier ones; hidden entries do not erase earlier images. The editor
lists them topmost first with distinct reference coloring, eye/remove controls
and Show only this reference. They have no layer IDs, geometry handles, device
links or selection/alignment participation. The currently focused reference row
is session-only; saved visibility determines the reopened composite.

At most 32 images and 64 MiB of combined original image bytes are accepted.
The existing 96 MiB project JSON limit still applies. No
external files, URLs or device endpoints are accessed on load. Image dimensions
are decoded from the bytes. The existing image loader enforces a 64 MiB encoded
file limit, maximum 8192 pixels per side (or the GPU texture limit if smaller),
and a 256 MiB decoder allocation limit. Bad image data aborts opening and leaves
the current editor unchanged. Project file reading/parsing, validation and reference
decoding run in a background worker. The editor is replaced only after every
image succeeds; Cancel loading ignores late worker results. Recovery checkpoints
pause during loading. Texture upload/presentation still depends on the GUI/GPU.
Captured Midra stills use the same reference object and embedded PNG bytes, with
no device request on reload; no schema/version change is required.

Editor object:

| Field | Representation / meaning |
| --- | --- |
| `selected_layer_ids` | Array of unique IDs of visible, unlocked layers; may be empty |
| `backdrop_title` | Required string, 0–128 Unicode characters, no control characters. Example backdrop title; empty hides the text. Preserved exactly, including spaces; omitted from SVG/device output. |
| `position_point` | Screen measurement origin: `"top_left"`, `"top_right"`, `"bottom_left"`, `"bottom_right"`, `"center"` |
| `position_layer_point` | Required: null for the matching layer point (advanced toggle off), or one of the same five point strings for an independent layer endpoint (toggle on) |
| `keep_aspect_ratio` | Boolean for layer resizing |
| `viewport_zoom` | Positive fit-relative scale, 0.0001–256 |
| `viewport_pan` | `[x, y]`, viewport pan in screen-coordinate pixels; finite, absolute value at most 1e9 |
| `show_source_pattern` | Boolean: show source patterns on other layers during crop editing; normal editing always hides patterns |
| `show_toolbar_icon_names` | Boolean: show names beneath toolbar icons; false by default |
| `outline_layers` | Boolean: transparent layer fill for reference alignment; selection/handles remain visible |

Position offsets measure from `position_point` on the screen to the chosen layer point (matching when `position_layer_point` is null). Screen corners retain inward signs: right origins reverse X, bottom origins reverse Y. Screen center uses positive right/down. For screen top-left → layer center, offsets are the layer’s absolute center coordinates. Numeric resizing and manual ratio edits preserve the chosen layer endpoint’s offsets. Switching either point or toggling independent measurement never moves a layer. Stored bounds remain top-left coordinates; viewport guides use the same endpoints.

Resolution dimensions must be whole pixels from 1 to 16384. Layer x/y must be finite and within ±1,000,000 pixels; layer widths/heights
and retained input dimensions must be positive, finite, and at most 1,000,000
pixels. Crop edges must be finite and nonnegative, with a positive retained
region. Off-screen x/y are allowed. Invalid source references, duplicate layer IDs or out-of-order input IDs, invalid crops,
missing fields and non-finite values are rejected before replacing the document.
Projects larger than 96 MiB are rejected.

Save includes **applied values only**. Pending text/format/import drafts, prepared aspect-ratio targets, open
dialogs, undo/redo history, crop-edit mode, pivot/navigation tools, alignment targets, and provisional device
export calculations are not persisted. Resize mode and grid snapping setup are restored from the project. Names and folder labels are stored. Removing layers does not renumber surviving
IDs; adding a layer consumes `next_layer_id`. Unused input slots are retained
to preserve shared-source references. Loading starts a fresh history. Loading requires a
replacement confirmation; failed loading leaves current work intact. Writes use
a same-directory temporary file and atomic publication. Existing destination
files require explicit replacement confirmation for Save/exports. Measurement-point
clicks and the independent-point toggle are an exception: they reread the current
project and atomically update only `editor.position_point` and
`editor.position_layer_point`, preserving its other stored values (including external edits).
Older schemas migrate to v17 on this write. No project path is stored in settings;
new documents require a first Save. Geometry exports never change the active project path.
Missing/invalid/unwritable projects are left untouched and an error is shown;
active in-memory choices remain available for a manual Save. A failed no-overwrite write
leaves the existing file intact.

## Layer history and export scopes

Add/remove/rename, reordering and folder edits are undoable. Device mappings are not implied
by editor IDs. Selection stores stable layer IDs, not zero-based indices. At
least one layer must remain. Visibility and locks remain local protection flags and are undoable.
Geometry export supports all, visible-only, explicit specific-layer (including hidden)
or selected layers (including a single
layer); rectangles retain stable IDs and document stacking order. Names appear as titles/labels in current geometry exchanges; folders are omitted.

Reordering changes only the order of the existing `layers` array, not IDs or
per-layer values. Reordering uses the existing array order for every exchange version. Moving layer 1 above
layer 2 in the list saves IDs in `[2, 1]` order and emits `layer-2` before `layer-1` in SVG.
Loading preserves that order; selected-layer exports still follow document order,
not selection order. Source slots, `next_layer_id`, names, folders, crop geometry,
visibility/locks and Midra links remain attached to their original layer IDs.
The layer manager displays reverse document order (top row is topmost). The inspector
collects same-name folder members into one collapsible group, ordered by each group's
frontmost member; children retain reverse stacking order. Legacy interleaved groups
are displayed together without silently rewriting the saved stack. Selection does not
raise a layer; canvas drawing and hit testing follow the saved stack.

Folders remain single-level names on layer records; no schema change is required.
There are no empty-folder records or implicit path separators. Creating/renaming a
folder updates all member `folder` strings in one history step; renaming to another
existing folder name is rejected (use membership moves to combine groups).
Moving a folder reorders its members as one contiguous block, preserving member order,
IDs, source references, crops and Midra links. A layer row drop changes membership and
stacking together in one history step. Folder eye/lock actions update each member's
`visible`/`locked` flags atomically; Undo restores mixed prior states. Folder selection
respects each member's visibility/lock. Collapse state is saved in `workspace.collapsed_folders`, but omitted
from plain geometry exports. Explicitly created folders remain when their last member leaves; legacy implicit groups disappear when empty. Empty folder rows can be renamed or deleted, and accept dropped layers. Creating/removing/renaming folders is undoable. Folder stacking movement requires members; empty folders are listed after the layer-backed groups.
Geometry exports continue to omit folders.

## Legacy SVG geometry exchange, version 1

The Export menu now produces SVG v2 and draw.io v1 with optional artwork; see
[geometry exchange](canvas-pilot-exchange.md) for their current contracts and
examples. The v1 API below remains readable and available for compatibility.

Suggested filename: `layout.svg`. UTF-8 SVG with SVG namespace, `width`, `height`
and `viewBox="0 0 W H"` equal to the output screen. The root carries
`data-midra-layout-geometry="1"`. A title and one `<rect>` per exported layer follow.

```xml
<svg xmlns="http://www.w3.org/2000/svg" data-midra-layout-geometry="1"
     width="1920" height="1080" viewBox="0 0 1920 1080">
  <title>Canvas Pilot geometry</title>
  <rect id="layer-1" x="120" y="180" width="640" height="360" fill="#173b50"/>
  <rect id="layer-2" x="1160" y="180" width="640" height="360" fill="#173b50"/>
</svg>
```

- IDs are stable layer names `layer-1`, `layer-2`; rectangle order is document
  order, independent of viewport selection.
- Coordinates are the visible box top-left and size, rounded independently to
  nearest whole pixels; half values round away from zero. Negative coordinates
  are retained. Dimensions that would round to zero reject export.
- There are no strokes, transforms or groups, so stroke expansion is unnecessary.
- **All layers** includes hidden and locked layers as ordinary visible rectangles.
  **Visible layers only** omits hidden layers but includes locked visible ones.
  An empty visible selection rejects export instead of producing an unimportable file.
- The file is a geometry interchange, not a project backup. It deliberately omits
  source assignments, crops, content zoom, reference image, lock/visibility flags,
  selection and camera. Use project JSON to preserve those.
- Use **Import → Geometry file…** and review the mappings. Choose destinations
  from All / Selected / Folder scopes; each included item needs a distinct layer. A full-screen rectangle bearing this export marker
  is treated as a layer, not an automatically ignored background. Other SVGs keep
  the existing full-page background detection rule.
- Reimporting restores rounded box geometry through the regular import options;
  it does not set the screen resolution or restore omitted project state. External
  SVG editors may remove the marker; then review the background checkbox on import.

## Compatibility changes

Project version 17 and the versioned geometry exchanges are strict contracts. A breaking project field/meaning change must bump
`version` and either implement a tested migration or clearly reject older/newer
files. Changing rectangle/marker semantics requires a corresponding documented
exchange-interface revision and importer tests. Update this document and its example
with every schema or semantic change, including additive fields under the strict
unknown-field policy. Never silently interpret a future version as a supported version.

Midra links belong to layer identity and survive selection changes, refresh, rename, reorder and project round trips. Link edits support Undo/Redo. New layers and older projects start unlinked. Links refer to the currently connected device; Preview/Program and sizing remain session choices, and Program permission remains an application preference. Duplicate destinations can be stored within or across folders. Mapping warnings list the other linked layers/folders; a send scope containing duplicates is rejected before any writes, while conflict-free folders/selections remain valid. Unavailable screens/layers are reported without deleting the saved links. Geometry exports omit links.

Midra destinations are identified by the pair **screen ID + layer ID**: Screen 1
Layer 1 and Screen 2 Layer 1 are distinct links. Both IDs are already persisted
in project version 5; no schema change is needed. Opening **Midra mapping & transfer…** or
**Configure Midra export…** restores the first selected layer's available linked
screen. Destination choices display both IDs. Browsing another screen does not
retarget existing links; choosing a destination there explicitly replaces that
layer's saved link. Direct send/load actions use each layer's saved screen,
independently of the screen currently displayed in the configuration window.

Automatic session recovery wraps this project representation in a separate [recovery file](canvas-pilot-recovery.md); named project JSON is version 17.

## Folder-scoped Midra transfers

The shared Midra transfer scope is session-only and is not saved. Defaults use all
layers; Selected/Folder modes filter by the existing selection or `folder` fields.
Folders may store alternative layouts whose `device_link` values reuse the same screen
and layer; each individual send requires unique destinations within its scope.
Manual device import now saves the reviewed `device_link` with imported geometry/crops
in one Undo step. New layers inherit the chosen folder; existing folder names remain.
All these values use the existing layer fields (unchanged in version 10). See
[device transfer scopes](midra-device-send.md#shared-transfer-scopes-and-folder-layouts).


### Editor input creation

The opt-in Advanced Source section offers Inputs 1–8 independently of editor/device
layer counts. Mapping only edits Midra screen/layer links, not input identities.
**New editor input** extends this local list up to the existing 1024-slot limit.
Selecting an unused identity is staged; **Apply source changes** creates any
missing consecutive slots with the default 1920 × 1080, 16:9 source and zero input
crop, then assigns the selected layer. Creation and assignment form one undoable
edit. Multiple layers may share the new identity. Existing smaller projects remain
unchanged until Apply; saving writes the materialized slots in `inputs` and the
one-based assignment in `layers[].source_id`. Version 10 and older-version
migration rules are unchanged. These IDs do not select or route physical inputs.

## Workspace settings (version 11)

| Field | Representation / validation / migration default |
| --- | --- |
| `snapping` | Required booleans `enabled` (false), `pixels` (true), `screen_edges` (true), `layer_edges` (true), `grid` (false), `show_grid` (true), and `grid_divisions` (two integers 1–64, default `[8,6]`). These override application defaults for the opened project. |
| `resize_mode` | `individual` (default), `same`, `mirror`, or `opposite` |
| `grid_spacing` | null (default), 10 or 100, in output pixels |
| `collapsed_folders` | Unique existing nonempty folder names (including empty explicit folders); default `[]`. Deleted/renamed stale entries are omitted when saving. |
| `capture_interval_seconds` | Integer 5–300, default 10 |
| `show_empty_inputs` | Boolean, default false |
| `previews` | Array with at most one entry per existing layer; default `[]` |

Each preview has required fields `layer_id` (stable existing editor layer ID),
`source_id` (the layer's current editor input slot ID, starting at 1), `input`
(physical capture input number, 0 means unassigned; maximum 1024), `host`
(empty or numeric IP with optional port, used only to match capture assignments),
`enabled`, `automatic` (booleans), `guide_opacity` (finite 0–1), `name`
(at most 1024 UTF-8 bytes), and `image_base64` (standard padded base64 encoded
original image bytes, empty if no still has been captured). Stale assignments for
removed layers or changed editor input slots are omitted on save. Captures retain
original bytes, without recompression or display rounding.

Reference and preview images together must fit within **64 MiB decoded base64
bytes**, and the whole JSON within 96 MiB. Decoding uses the same bounded image
loader as references, on the project-loading worker. Malformed images abort the
entire open, preserving current work. A capture shared by several layers is
embedded in each preview entry; the same combined size limit applies.

Restoring a project never starts a connection or a capture. Saved stills appear
offline; saved automatic capture choices are **paused** until Capture still,
Resume auto-refresh, changing the input, or explicitly toggling auto-refresh.
Pending jobs, throttles and queued selection captures are not restored. Device
host matching still applies to automatic requests. These settings/stills are not
part of geometry export formats; fixed references retain their existing optional
background export behavior.

For the complete persistence audit and the reasons for keeping other controls
out of project files, see [Project persistence audit](canvas-pilot-persistence-audit.md).

Device links distinguish family, screen/auxiliary and layer. A link from another family cannot be used on the current device without explicit remapping. Multiple editor layers may share one destination; a send scope containing duplicates is rejected with the conflicting layer names. Device state and physical bank roles are never saved in the project. Geometry-only exports omit these links; embedded editor snapshots retain version 17 and use the same migration on import.

### Explicit layer fit (v14)

After input correction and layer crop, retained size is `(rw, rh)` and container
size is `(w, h)`. The centered image uses scale 1 for `one_to_one`,
`min(w/rw,h/rh)` for `centered`, `max(w/rw,h/rh)` for `cropped`, and independent
axis scales for `fullscreen`. Clip to the container; uncovered regions are
transparent. Resizing preserves crop edges in explicit modes. Disabling advanced
fit resets to normal centered cover framing. Input/global inheritance is omitted.

Geometry-only imports/exports describe container bounds for explicit modes and
omit fit/crop semantics, just as they omit normal source framing. Embedded full
projects preserve them; optional exported artwork renders crop/fit with transparent
margins. Midra can transfer explicit modes; Aquilon sends reject them pending a
separate adapter implementation. See [device conversion](midra-device-send.md).

### Layer and source masks (v17)

The reference is the layer rectangle for `layer`, or the full usable source mapped
into output coordinates for `source` (local input crop happens first). Source
reference bounds can extend outside the layer. The reveal rectangle is `(x + left*w, y + top*h, (1-left-right)*w,
(1-top-bottom)*h)` in output pixels. Intersect this rectangle with the rendered
image after crop and fit. Outside pixels are transparent; no image translation,
resampling or container resizing occurs. Blanking already absent pixels has no
additional effect. Empty reveal rectangles are valid. Project, recovery and
embedded-project exchanges preserve the reference mode and all four fractions; geometry-only SVG and
rectangle exchanges intentionally omit masking. Exported placeholder artwork
includes transparent blanking without changing its retained image samples.
Numeric UI fields show output-pixel distances for layer mode and usable-source
pixel distances for source mode. Switching modes keeps the fractions, so the
visible result can change. Reset clears edges while keeping the mode. Mask helper movement changes opposite edges by
equal amounts, clamped to the selected reference, preserving reveal size.
