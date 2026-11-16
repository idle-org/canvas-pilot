# Canvas Pilot layer clipboard, version 3

Implementation: `clipboard.rs`, `import.rs` and the shared paste review in `gui.rs`.
This is a Canvas Pilot clipboard representation, not a device protocol or a native
clipboard format for other applications. It changes no project/file schema.

## Text envelope

**Copy layer values** and viewport Ctrl/Cmd-C emit the existing readable summary
(center coordinates and dimensions in rounded screen pixels; normal crop zoom as
percent), followed by this exact delimiter:

```text
\n\nCanvas Pilot layer data:\n
```

The escapes above represent actual newlines. The remainder is UTF-8 JSON. The last
literal delimiter identifies the payload; names inside JSON use standard JSON
escaping. The summary is informational: editing it does not override the JSON.
Readers validate the payload and never fall back to rounded geometry when it is
invalid. Raw legacy summaries without this delimiter still use the existing parser.

See the [current example](canvas-pilot-clipboard-v3.example.txt) and
[legacy v2 example](canvas-pilot-clipboard-v2.example.txt) and
[legacy v1 example](canvas-pilot-clipboard-v1.example.txt).

## JSON schema and validation

Objects reject unknown fields. All listed fields are required, including nullable
`fit`. The maximum complete text size is 16 MiB; layers must contain 1–256 entries.
No URLs, file paths, commands, images or arbitrary application state are evaluated.

| Field | Meaning |
| --- | --- |
| `format` | Exactly `canvas-pilot.layers` |
| `version` | Integer `3`; readers also accept `1` and `2`; other versions reject the whole paste |
| `layers` | Ordered array of copied layers; selection order follows document order |
| `layers[].name` | In v2/v3, the actual layer name without an ID prefix; nonblank, at most 256 characters to read, 1–128 to apply. In v1, a review label only |
| `layers[].bounds` | `x`, `y`, `width`, `height`: full-precision screen pixels; top-left coordinates, finite within ±1,000,000 and positive dimensions up to 1,000,000 |
| `layers[].appearance.input_size` | Two positive finite effective source dimensions in pixels, after local input crop; validated by the existing source-size limits |
| `layers[].appearance.crop` | `left`, `right`, `top`, `bottom`: nonnegative finite effective-source pixel crop edges, leaving positive width/height |
| `layers[].appearance.fit` | `null` for normal crop/zoom, or `one_to_one`, `centered`, `fullscreen`, `cropped` |
| `layers[].appearance.mask` | Required `origin`: `layer` or `source`; `left`, `right`, `top`, `bottom`: finite fractions of that reference, each 0–1; opposite sums must be at most 1, with fully blank axes allowed |

Versions 1/2 migrate masks to `origin: "layer"`; supplying `origin` in those
versions is rejected. Version 3 requires the field; unknown origins are rejected.

Normal crop must match the visible-box aspect and supported 1–100 zoom range.
Offsets and zoom are derived from crop and bounds rather than stored twice.
Advanced modes apply crop before fit. Values are not display-rounded in JSON;
mask proportions do not become source-pixel crops.

## Applying a paste

All data enters the existing destination mapping review. Position and Format are
independent. An **Appearance** group offers **Crop & fit** and **Blanking**, enabled
initially for this payload. Appearance-only and name-only pastes are valid. **Rename destination layers** is
off by default, copies v2/v3 names only, and leaves folders/IDs unchanged. External geometry
formats have no such payload; older normal-crop summaries offer their legacy zoom
control instead. Ignored items do not affect availability or mapped values.

Crop & fit requires the destination's effective source dimensions to match the
recorded dimensions within 1e-8 pixels per axis. It copies exact crop and mode;
it does not adapt different inputs silently. Normal-crop aspect must also match
the resulting box. Mismatches block the batch with guidance to include Format,
match the local source dimensions, or disable Crop & fit. Blanking can be applied
independently across different input sizes as proportions of the selected layer/source reference; source mode follows the
destination’s source and crop/fit, rather than copying source pixel distances.
Unchecked appearance fields remain unchanged, except that a selected Format change
may reframe existing crop under the editor's normal resize rules.

Mappings must be unique, visible, unlocked and still refer to the reviewed layer
identities. Apply is atomic and one undo step: malformed data, duplicate mappings,
incompatible crops or invalid masks cannot partially change the document. Device
writes are never part of paste. Copying and opening the review are read-only.

## Omissions and compatibility

Folders, identity, visibility, locks, pivots, source formats,
input crop, shared source relationships, device links, input proposals, captures,
references and workspace settings are not copied by this representation. Only the
recorded source dimensions accompany crop for compatibility validation. Use a
project or embedded export snapshot for the wider state.

Existing normal-crop summaries remain readable, with optional zoom and their
historical integer geometry. Old advanced-container summaries are not upgraded
implicitly because they lack the crop/source information needed to reconstruct
appearance. New copies of all fit modes contain the v2 payload. V1 stays readable for geometry
and appearance; its names may include display ID prefixes and are not offered for
renaming. V1-only readers reject v2 with an unsupported-version error. Older application
versions do not support the appended payload; remove the payload only when
intentionally exchanging the legacy normal-crop geometry/zoom subset.

Regression tests cover every mode, crop offsets, fractional values, masks,
independent appearance controls, legacy summaries, invalid schemas, source/aspect
mismatches, locked destinations, whole-batch rollback and undo/redo.
