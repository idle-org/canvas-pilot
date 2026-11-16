# What gets lost between formats?

[Format recommendations](format-matrix.md) · [Advanced feature matrix](advanced-exchange.md)

A conversion can preserve the picture while losing the information needed to edit
it in the same way. Here are a few common paths, rather than a complete compatibility
survey. External-editor behavior below is based on linked vendor documentation;
possible losses during later re-saving are cautions, not claims that every editor
was tested. Canvas Pilot behavior follows its current import/export implementation.

## Three different things to preserve

| What survives? | What that actually means |
| --- | --- |
| Appearance | It still looks right. Text might now be outlines, or several objects might be one image. |
| Editable geometry | Individual shapes can still move or resize. They need not retain their original names, grouping or behavior. |
| Application structure | Components, diagram connections or Canvas Pilot source mappings still have their original meaning. This normally needs native data understood by the receiving application. |

SVG can contain vectors, text, images and extra metadata. The `.svg` extension alone
does not tell you which of those a particular exporter wrote or an importer reads.

## 1. Figma → SVG → another vector editor

**Typical goal:** send a layout or drawing for further visual editing.

Shapes can remain vector artwork. Figma offers text outlining and layer-name IDs:
outlined lettering keeps its shape but ceases to be editable text; keeping text
requires suitable fonts in the receiving environment. Including IDs gives names a
chance to travel, but does not guarantee the destination uses them as layer names.
See [Figma’s SVG export settings](https://developers.figma.com/docs/plugins/api/ExportSettings/).

Treat this as artwork exchange: do not expect the receiver to reconstruct Figma
components, variants, Auto Layout rules or prototype interactions from the SVG.
That is a workflow inference, not a promise about every importer. Returning the
SVG to Figma does not make it a replacement for the original design file.

**Keep:** the original Figma document if those editing relationships matter.

## 2. draw.io → SVG → draw.io

**Typical goal:** share a picture that can also be reopened as an editable diagram.

With **Include a copy of my diagram**, the SVG carries diagram data alongside the
rendering. draw.io can use that data to reopen the diagram. Without it, the SVG
picture alone is not the original diagram model. This is documented by
[draw.io’s SVG export guide](https://www.drawio.com/docs/manual/export/export-to-svg/).

This embedded diagram is different from a Canvas Pilot extra-parameters snapshot;
the two are not interchangeable. Prefer `.drawio` when the recipient primarily
needs to edit diagram shapes and connections.

## 3. Figma → SVG → draw.io → SVG

**Typical goal:** insert a Figma drawing into a diagram, then share the result.

An ordinary SVG inserted into draw.io is an image shape: you can position and resize
it, but its internal rectangles do not become independent diagram nodes with
editable connector relationships. It can still be vector artwork; “one image” does
not necessarily mean rasterized pixels.
[draw.io documents this import limitation](https://www.drawio.com/docs/manual/insert/svg-insert/).

The final SVG therefore should not be assumed to contain separately editable
versions of the original Figma objects. If you need two independently movable
screen boxes, create two diagram shapes rather than inserting both as one SVG image.

## 4. Figma or draw.io → SVG → Canvas Pilot

**Typical goal:** transfer the positions and sizes of screen/layer boxes.

Canvas Pilot reads supported rectangle geometry, including supported rectangular
paths, and offers supplied names through **Rename destination layers**. It does
not recreate the source application's components, connectors or arbitrary artwork.
SVG IDs may be technical identifiers rather than useful names: check the review.

Masks, clipping, effects, rounded corners and rotated geometry are not equivalent
to plain layout rectangles; unsupported constructions can be rejected instead of
being silently approximated. A picture that looks like two boxes is not necessarily
two importable boxes. Import into existing layers also leaves their source settings
and device links in place.

**Example:** two plain 640 × 360 rectangles exported from Figma can provide two
layer boxes. The same drawing inserted into draw.io as one SVG image cannot be
assumed to yield those same two boxes when exported again. Inspect the import review.
See the [file and clipboard matrix](format-matrix.md) for supported paths.

## 5. Canvas Pilot → SVG → another editor → Canvas Pilot

**Typical goal:** adjust placement elsewhere without losing crop and mask work.

A plain export carries geometry, names and optional preview pictures. Its box
coordinates are rounded to whole pixels. Pictures can show a crop or mask, but do
not encode its editable controls. Source assignments, device links and the Layer/
Source mask reference are not recovered by ordinary geometry import.

With **extra Canvas Pilot parameters**, the SVG also contains an original project
snapshot. If that metadata survives the external editor, **Restore parameters from
export** restores that snapshot—not the external editor’s subsequent changes.

**Example:** export a box at x=100 with a source mask, then move the SVG box to x=200.
Normal import reads the new geometry. Snapshot restoration restores the original
x=100 and mask. It does not automatically merge the new position with the old state.
You can instead reopen the saved project and import the edited geometry into its
layers, reviewing the result because resizing can reframe a crop.

An editor or SVG optimizer may remove metadata. Saving as PNG cannot preserve the
editable vector boxes merely because the picture still looks correct. Keep the
`.pilot.json` companion; see [snapshot limits](advanced-exchange.md#limits-and-round-trips).

## A small check before committing to a conversion path

Try two differently named boxes, some editable text and one crop/mask. After each
hop, check the box count, names, coordinates, text editability and whether the crop
is still a control or only visible artwork. If native metadata is involved, change
one box before reopening to check whether you are seeing the latest artwork or an
older embedded snapshot. Keep the original file until that path meets your needs.
