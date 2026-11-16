# Extra Canvas Pilot parameters in geometry exports

This optional extension is selected by **Export layers → Include extra Canvas Pilot
parameters (experimental)**. It defaults off. Plain exports retain their existing
representation. Native **Save project** already saves the full project and needs
no additional switch.

Use **Project → Restore parameters from export…** to recover the original snapshot
into a new, unsaved project. The existing replacement review applies. The source
artwork never becomes the project-save/autosave path. Geometry import and clipboard
paste continue to import the artwork's current geometry; they do not silently
restore old positions, links or crop settings from metadata. This matters when an
external editor moves/resizes a shape without updating the embedded snapshot.

## Format audit and implemented carriers

| Format | Carrier / result | Compatibility limits |
| --- | --- | --- |
| SVG | `<metadata><parameters xmlns="urn:beffroi:midra:parameters:1">BASE64</parameters></metadata>` inside the SVG root | Non-rendered foreign-namespace metadata. SVG geometry exchange remains v2. Tested with librsvg rendering. Optimizers may remove metadata. |
| draw.io | Global root-cell custom property: `<object id="0" beffroi_midra_parameters="BASE64"><mxCell/></object>` | Graph model/geometry version stays 1. Installed draw.io Desktop opened and exported this to SVG; parameters also recovered from the SVG's embedded diagram. Compressed single-page draw.io models are supported through the existing bounded decoder. |
| Geometry JSON | Version **2**, required root string `beffroi_midra_parameters` | All v1 geometry fields unchanged. Versions 1 and 2 import; v1 exporters remain byte-compatible when the option is off. Older strict readers reject v2 rather than misread it. |
| Geometry CSV | Version **2** in every row; added `beffroi_midra_parameters` column | The first data row holds BASE64; remaining rows contain empty cells. Header/row widths remain equal and ordinary CSV quoting is used. No nonstandard comment lines. Versions 1 and 2 import; older schema-aware readers reject v2. |
| ASCII DXF | Group **999** comments after the existing geometry marker, each `beffroi-midra-parameters:CHUNK` | Ordered base64 chunks of at most 200 characters (comment lines under 255). Craft still reads the same rectangles. Comments may be stripped on CAD resave; binary DXF does not use group 999. |
| HTML | `<meta name="beffroi_midra_parameters" content="BASE64"/>` in `<head>` | Inert document metadata, no scripts or hidden visible objects. HTML geometry version stays 1. |
| PPTX | `docProps/custom.xml`, string property `beffroi_midra_parameters` | Standard custom-properties content type and package-root relationship; `vt:lpwstr`, pid 2, fmtid `{D5CDD505-2E9C-101B-9397-08002B2CF9AE}`. Existing ZIP parts are preserved; native DeckCraft import checks unchanged geometry. Microsoft PowerPoint resave not tested here. |
| DWG | Not enabled | DXF comments do not provide a reliable binary-DWG carrier. XRECORD/XData are potential future paths, but the current Craft conversion bridge has no verified application-payload round trip. No bytes are appended to the binary file. |
| FigCraft FIGCRAFT | First page node `plugin_data.beffroi_midra_parameters` in native v1 `document.json` | ZIP and bare JSON restore supported. Native FigCraft 0.5.1 load/save preserves the field. Duplicate payloads are rejected; document JSON reads are bounded to 16 MiB. Other ZIP members are preserved. |
| Figma FIG | Not enabled | Figma has plugin data, but the installed FigCraft 0.5.1 native exporter explicitly warns `plugin data was left out`. Its model field is not evidence of native preservation. |
| Illustrator | Not enabled | PDF-compatible Illustrator can have document metadata, but the current VectorCraft SVG↔Illustrator conversion path has no verified application-payload preservation. No unverified PDF/Illustrator trailer changes are made. |
| Midra project JSON | Already complete | Project v17 remains the authoritative full-fidelity save. |

Unsupported codecs disable the option with an explanation. Programmatic attempts
to request the extension on those codecs fail explicitly, never silently discard
it. Vendor availability and the `no_proprietary` build flag still apply.

## Payload contract (extension version 1)

`BASE64` is standard padded RFC 4648 base64 of UTF-8, pretty-printed
[project JSON](canvas-pilot.md), currently version 17. It is inert data, not an
executable instruction, URL or file path. Project format/version and validation
remain authoritative. Future incompatible carrier/payload meanings require a new
extension version and explicit migration or rejection.

The snapshot includes only exported layer IDs, in document stack order, with
full-precision bounds, names, folders, visibility, locks, pivots, family-aware device links,
source/input crop, normal/advanced layer crop, all fit modes and blanking masks. Explicitly saved inactive input proposals are filtered to included layers; they never grant permission to stage or send. Only referenced input slots are included;
source IDs are remapped consecutively, preserving shared source relationships.
Selection, explicit folders and collapsed folders are filtered to retained layers; unrelated empty folders are omitted. Output resolution,
positioning, camera, guide and workspace settings remain available. Pixel-rounded
artwork and full-precision stored coordinates are intentionally distinct.

All reference-image objects and preview assignments/images/device-address bindings
are omitted. Artwork background/placeholder options operate independently; they do
not embed capture settings. Application preferences, connection history, active
connections, pending device edits/jobs and undo history are never included.

The decoded metadata JSON is limited to **1 MiB**, and the complete exchange file
still must fit within **16 MiB** after embedding. PPTX custom-properties XML reads
are bounded to 2 MiB. No archive members are extracted onto the filesystem. Invalid
base64 or project data rejects explicit snapshot restoration. For geometry JSON
and CSV, the known v2 extension is removed before strict v1 geometry validation;
invalid optional payload contents do not prevent importing otherwise valid artwork.
Unknown versions and unknown geometry fields remain errors. More than one nonempty
CSV snapshot is rejected on restoration.

The [JSON v2 example](midra-geometry-v2-metadata.example.json) is a tested example;
its decoded snapshot is also validated as a project. Plain v1 fixtures remain
unchanged. Other carriers use the same payload bytes and are round-trip tested
with background on/off and optional placeholders. The regression suite also checks
that edited artwork is imported as edited, unsupported codecs refuse the option,
and recovered exports are never reused as project-save paths.

## Standards and codec evidence

- [W3C SVG extensibility](https://www.w3.org/TR/SVG11/extend.html) explicitly allows
  application metadata for round trips; [SVG 2 structure](https://www.w3.org/TR/SVG2/struct.html)
  describes metadata containers.
- [draw.io global custom properties](https://www.drawio.com/docs/manual/advanced/global-custom-properties/)
  documents root-cell custom data.
- [Autodesk DXF group codes](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-3F0380A5-1C15-464D-BC66-2C5F094BCFB9.htm)
  documents group 999 comments. [Binary DXF](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-DXF/files/GUID-FC1C3C69-DBC2-49E4-893A-000D6538C0FE.htm)
  does not use that comment group.
- [WHATWG document metadata](https://html.spec.whatwg.org/dev/semantics.html)
  describes name/content metadata. [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  describes CSV fields and quoting; this extension adds a regular column.
- [Microsoft custom properties](https://learn.microsoft.com/en-us/office/open-xml/word/how-to-set-a-custom-property-in-a-word-processing-document)
  describes the shared Open XML custom-property representation; its package
  relationship applies to presentations too.
- [Figma plugin data](https://developers.figma.com/docs/rest-api/file-endpoints/)
  exists in the platform. The local `figcraft-format 0.5.1/src/fig/export.rs`
  exporter nevertheless rejects preservation in `plugin_data`; support stays off.
- Craft DWG source inspected at revision
  `79e97b6febcb62de406e5bcf5a724bd3f7333065`; current native adapter conversion
  boundaries are in `crates/canvas-pilot/src/vendor_formats.rs`.
