# Canvas Pilot starter files

[Canvas Pilot](../../canvas-pilot/introduction.md) · [User guide](../../canvas-pilot/guide/overview.md) · [Geometry format contract](../../interfaces/canvas-pilot-exchange.md)

**[Download all templates as ZIP](canvas-pilot-all-templates.zip)** — all eleven formats
in both Full HD and UHD 4K (22 files). Extract the archive and use whichever
format suits your editor. This also preserves file names when browsers would
otherwise preview a template, including PDF-compatible Illustrator files.

Individual downloads are listed below.

This folder is the single source for the base files in every supported geometry
exchange format. Each file contains the same two named 16:9 layer boxes on an
exact pixel canvas, without backgrounds or placeholder pictures. The UHD layout
is an exact 2× scale of the Full HD layout.

Choose your canvas and format, download the file, then edit it in the matching
application or import it through **Import → Geometry file…** in Canvas Pilot.
Review the detected rectangles and their destination layers before applying.

| Format | Full HD · 1920 × 1080 | UHD 4K · 3840 × 2160 | Use with |
| --- | --- | --- | --- |
| SVG | [Download .svg](canvas-pilot-1080p-2-layers.svg) | [Download .svg](canvas-pilot-4k-2-layers.svg) | Vector editors and browsers |
| draw.io | [Download .drawio](canvas-pilot-1080p-2-layers.drawio) | [Download .drawio](canvas-pilot-4k-2-layers.drawio) | diagrams.net / draw.io |
| Geometry JSON | [Download .json](canvas-pilot-1080p-2-layers.json) | [Download .json](canvas-pilot-4k-2-layers.json) | Scripts and geometry exchange |
| Geometry CSV | [Download .csv](canvas-pilot-1080p-2-layers.csv) | [Download .csv](canvas-pilot-4k-2-layers.csv) | Spreadsheets and data tools |
| DXF | [Download .dxf](canvas-pilot-1080p-2-layers.dxf) | [Download .dxf](canvas-pilot-4k-2-layers.dxf) | CAD tools |
| DWG | [Download .dwg](canvas-pilot-1080p-2-layers.dwg) | [Download .dwg](canvas-pilot-4k-2-layers.dwg) | CAD tools; Craft bridge |
| HTML | [Download .html](canvas-pilot-1080p-2-layers.html) | [Download .html](canvas-pilot-4k-2-layers.html) | Browsers and web layouts |
| PowerPoint | [Download .pptx](canvas-pilot-1080p-2-layers.pptx) | [Download .pptx](canvas-pilot-4k-2-layers.pptx) | PowerPoint, LibreOffice Impress or DeckCraft |
| Figma FIG | [Download .fig](canvas-pilot-1080p-2-layers.fig) | [Download .fig](canvas-pilot-4k-2-layers.fig) | Figma or FigCraft |
| FigCraft | [Download .figcraft](canvas-pilot-1080p-2-layers.figcraft) | [Download .figcraft](canvas-pilot-4k-2-layers.figcraft) | FigCraft |
| Illustrator AI (optional) | [Download .ai](canvas-pilot-1080p-2-layers.ai) | [Download .ai](canvas-pilot-4k-2-layers.ai) | Requires VectorCraft CLI for Midra import/export; prefer SVG |

FIG, FIGCRAFT, DXF and DWG import/export were confirmed working by the user on
2026-11-15. PPTX, draw.io and SVG were previously confirmed; see the
[format acceptance list](../../canvas-pilot/plan.md#completed-acceptance).

**Prefer SVG when exchanging layouts with Illustrator.** The AI templates remain
available for compatibility, but Midra needs the separately installed VectorCraft
CLI to import or export them. See [AI setup](../../canvas-pilot/guide/files.md#illustrator-ai-optional-converter-required).

## Geometry files and saved projects

These files exchange visible layer boxes. They do not store the complete editor
state, source assignments, crop settings or device links. In particular, the
`.json` starter is **geometry JSON**, not a saved Midra project. Use **Project →
Save project…** for a full `.midra.json` project; see the
[project format](../../interfaces/canvas-pilot.md). Versioned schema and migration
examples stay beside their [interface contracts](../../interfaces/README.md).

PPTX and FIG support uses the optional native codecs included in the default
build. Illustrator AI import/export requires a separately installed VectorCraft
CLI. See [dependencies and licensing builds](../../interfaces/canvas-pilot-exchange.md#optional-native-document-codecs-and-licensing-builds).

## Maintaining the base files

After updating any starter file, rebuild the combined download from this folder:

```sh
python3 -m zipfile -c canvas-pilot-all-templates.zip canvas-pilot-*-2-layers.*
```

The archive contains exact copies of all 22 templates; regression tests verify
the file names and bytes and the manual includes the same archive.

Keep all formats here and link to this catalog instead of copying files or
maintaining separate download lists. The [generation and verification commands](../../interfaces/canvas-pilot-exchange.md)
rebuild both canvas sizes with the application exporter and verify their geometry.
Text and native PPTX/FIG/DWG fixtures are covered by regression tests; the AI check
requires VectorCraft and is opt-in. The AI samples were checked with VectorCraft
0.24.1, not Adobe Illustrator.

The DXF and DWG samples were opened and rendered with **QCAD Professional 3.28.2**.
Both formats were also saved as R21 (2007) in QCAD and reimported with unchanged
layer coordinates. DXF and DWG carry unitless pixel geometry and no pictures.
