# Advanced exchange feature matrix

[Format tiers and basic matrix](format-matrix.md) · [Starter downloads](../../templates/canvas-pilot/catalog.md)

Gold saves the full project. Silver restores an optional embedded snapshot. Bronze
imports geometry only. These tiers describe Canvas Pilot state preservation, not
application quality or a guarantee that third-party editors retain metadata.
Gold uses Canvas Pilot’s application-specific `.pilot.json` format, intended only
for Canvas Pilot. It is not an industry standard or a measure of compatibility
with other editors; use an exchange format for those workflows.

## Which editing features survive?

This table separates editable state from a picture of its appearance. “Snapshot”
means a supported exchange format exported with extra parameters and reopened through **Restore parameters from export**.

| Feature | Project Open/Save | Plain geometry file import | Snapshot restore |
| --- | :---: | :---: | :---: |
| Layer position and size | ✅ | ✅ | ✅ |
| Full-precision saved coordinates | ✅ | ❌ [1](#editing-notes) | ✅ |
| Output screen resolution | ✅ | ❌ | ✅ |
| Layer names | ✅ | ✅ · opt-in, when supplied | ✅ |
| Stacking order | ✅ | ❌ | ✅ |
| Folders, collapsed folders, visibility and locks | ✅ | ❌ | ✅ [2](#editing-notes) |
| Source formats, assignments and shared input relationships | ✅ | ❌ | ✅ [2](#editing-notes) |
| Input crop edges | ✅ | ❌ | ✅ |
| Normal layer crop zoom and offsets | ✅ | ❌ | ✅ |
| Advanced fit modes and their crop edges | ✅ | ❌ | ✅ |
| Editable blanking masks (Layer / Source) | ✅ | ❌ | ✅ |
| Resize pivots and group resize settings | ✅ | ❌ | ✅ |
| Saved device screen/layer links (not physical source routing) | ✅ | ❌ | ✅ |
| Previously saved input correction proposals, preserved inactive | ✅ | ❌ | ✅ |
| Reference images and their individual settings | ✅ | ❌ | ❌ |
| Capture assignments and saved preview stills | ✅ | ❌ | ❌ |
| Selection, camera, guides, snapping and workspace settings | ✅ | ❌ | ✅ [2](#editing-notes) |
| Undo history, pending edits and active device operations | ❌ | ❌ | ❌ |
| Application theme, connection history and Program permission | ❌ [3](#editing-notes) | ❌ | ❌ |

### Editing notes

1. Geometry **exports round boxes to whole pixels**; import can retain fractional
   coordinates where the reader supports them. Use a project or snapshot for
   full-precision recovery. Geometry import never reorders destination layers.
2. Snapshot restoration includes only exported layers and their referenced inputs;
   selection and folders are filtered accordingly. It replaces the document with
   the saved snapshot, not a merge of the external artwork's latest edits.
3. Application preferences are saved separately from projects. Opening a project
   or restoring a snapshot does not connect or send to a switcher.

For a complete editable backup, **Save project** remains the recommended choice.

## Copying layer appearance

Canvas Pilot’s **Copy layer values** also preserves exact geometry, crop edges,
offsets, fit and blanking through the clipboard. Its mapping review offers an
optional **Appearance** group; source assignments and device links remain unchanged.
The separate **Rename destination layers** option applies supplied source names.
See [clipboard preservation and limits](files.md#canvas-pilot-layer-clipboard).
This is separate from plain geometry import and export snapshot restoration.

## Saved input corrections

Previously saved input correction proposals remain preserved in projects and in
Silver exports with **Include extra Canvas Pilot parameters (experimental)**.
They remain inactive: the legacy controls for creating, staging and sending them
have been removed. No saved proposal automatically changes an input.

For new device edits, use [Input settings…](formats-and-crop.md#shared-device-input-settings).
Its drafts are session-only and require their own explicit review and send. Local
source dimensions/crops and shared source assignments still belong to the project.

For practical examples across editors, see [what gets lost between formats](conversion-losses.md).

## Limits and round trips

Silver formats are SVG, draw.io, geometry JSON, HTML, PPTX, FigCraft, CSV and DXF.
Their snapshots preserve the original values, independently of rounded artwork.
They do not turn crop, mask or source settings into native editing controls in the
other application. Background/placeholder pictures can show the result, but cannot
restore editable state by themselves.

FIG, DWG and Illustrator have no verified metadata carrier through the current codecs.
Keep a `.pilot.json` companion when using these Bronze formats. Clipboard geometry
also does not restore embedded snapshots. Use **Restore parameters from export**
for Silver; use ordinary geometry import for the artwork's latest positions.

Automated round trips cover all fit modes, fractional crops, both mask references, shared
inputs, mappings, saved proposals and workspace state through every Silver codec.
FigCraft metadata is also tested through its native load/save cycle. External
application resaves can strip metadata; this is not a promise of lossless round
trips through every editor. See the [carrier contract](../../interfaces/midra-extra-parameters.md)
for format-specific limits and validation evidence.
