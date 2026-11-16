# Midra formats, resolutions and advanced cropping

Status: offline geometry and UI implemented. Source/native formats, shared input
cropping, virtual layer resolution, content zoom/offset, content dragging, dynamic
output resolution, reference remapping and atomic undo are available. Crops are
authoritative; zoom/offset are derived controls. Hardware container behavior,
firmware crop limits and quantization remain unverified and are never presented
as a validated device recipe. No REST calls are made.

Implementation limits: integer native/output dimensions 1–16384 per side;
positive finite layer/input dimensions up to 1,000,000 px; content zoom 100–10000%.
These are planning limits, not claimed hardware capabilities. Invalid edits are
rejected atomically. If framing must shift after a resize/source change, the crop offsets
adjust automatically to fit the new dimensions. Undo restores size and crop
together. Layer properties exposes crop adjustment and reset controls.

## Names and coordinate spaces

| User-facing name | Meaning | Internal representation |
| --- | --- | --- |
| Source format | Original aspect, e.g. 4:3, 16:9, 16:10, with explicit native pixel dimensions | Source identity, native width/height, selected aspect |
| Output format | Aspect of the screen represented by the viewport | Derived from output width/height; optional preset/ratio lock |
| Viewport resolution | Output screen pixels, selected from a preset or entered manually | Positive integer screen width/height |
| Input resolution | Usable source area after input-level cropping; Custom retains the original Source format selection | Four crop edges in native source pixels; remaining dimensions derived |
| Layer resolution | Final visible box on the screen, e.g. 300 × 250 | Final width/height plus **advanced cropping**, stored as four crop edges relative to the usable input |
| Content zoom | How tightly that usable input is framed inside the final box | Derived from crop rectangle and final size; 100% means minimum crop needed to fill the box |
| Offset X / Y | Movement of content inside the final box | Derived from asymmetric crop; positive values move the picture right/down |
| Viewport zoom | Magnification of the editor view only | Camera state; never changes output pixels or cropping |

Assume square pixels for the initial implementation. A source format such as
16:9 does not establish whether its native size is 1280 × 720 or 1920 × 1080.
Require native dimensions before converting pixel crops. Non-square-pixel or
aspect-corrected inputs need an explicit later mapping; never infer one.

Cropping a 1920 × 1080 source to 1920 × 1000 changes the usable aspect to 1.92:1.
The Source format still reads 16:9 because it describes the original source.
It must not force the cropped area back to 16:9 or distort its pixels. This is
the implemented interpretation of Custom input resolution.

## Output controls in the first draft

Provide HD 1280 × 720, Full HD 1920 × 1080 and UHD 4K 3840 × 2160 as explicit
resolution labels. Do not use “4K” alone where it could mean 4096 × 2160.
Offer format choices such as 4:3, 16:9 and 16:10, and always display the actual
width and height. A locked ratio links dimension edits; an unlocked custom pair
sets its own ratio. Changing a format with a ratio lock preserves width and
proposes the corresponding height, showing any integer rounding before commit.
Choosing a resolution preset sets both dimensions and its ratio in one action.

Custom width/height are first-class, not hidden behind a device-preset list.
Reject zero, negative, noninteger and nonfinite viewport dimensions. Enforce
documented implementation limits, but distinguish these from supported device
output modes. An arbitrary planning resolution is not a promise that the device
can be configured to produce it.

Changing viewport resolution preserves layer positions and sizes in screen
pixels. Recompute displayed right/bottom offsets; do not silently reposition or
scale layers to preserve margins. Revalidate image mapping and off-screen status.
Changing the application window size only changes the camera/layout.

## Input resolution: source-level crop

For native dimensions `Nw × Nh` and input crops `Il, Ir, It, Ib`:

```text
Iw = Nw - Il - Ir
Ih = Nh - It - Ib
```

All crop amounts are nonnegative and the remaining area must be positive.
Custom dimensions may reduce either axis but cannot manufacture additional source
pixels. Initially center the retained area, dividing removed pixels equally;
provide explicit crop placement so black bars or unwanted edges need not be
symmetric. Preserve fractional results internally until device quantization.

Example: native **1920 × 1080**, input **1920 × 1000** gives input crops
**left 0, right 0, top 40, bottom 40**. Source format remains **16:9**.

Input cropping belongs to the source identity. If both layers use the same
physical input, they share its input-resolution correction, while each has its
own advanced crop. Show which layers an input edit affects and apply it as one
undoable transaction. Different proposed input crops cannot be exported as if
they were independent settings for the same physical input.

## Layer resolution: advanced cropping

The visible layer box `Bw × Bh` is fixed while its content is reframed, just like
an avatar crop. Cropping followed by proportional fitting produces the apparent
zoom. There is no independent image-zoom transform to export to WebRCS.

The baseline uses **cover** geometry: scale proportionally until both box
dimensions are covered, then remove overflow. It is not sufficient to choose
the numerically largest side; compare both scale ratios. This avoids blank bars
for landscape, portrait and unusually wide boxes.

Let the usable input be `Iw × Ih`, content zoom be `z ≥ 1`, and offsets `ox, oy`
be in final output pixels. At 100%, `z = 1`; viewport camera zoom is unrelated.

```text
k0 = max(Bw / Iw, Bh / Ih)       # minimum proportional scale that covers box
k  = k0 * z                    # output pixels per usable-input pixel
Rw = Bw / k                    # retained source region width
Rh = Bh / k                    # retained source region height

L = (Iw - Rw) / 2 - ox / k
R = (Iw - Rw) / 2 + ox / k
T = (Ih - Rh) / 2 - oy / k
B = (Ih - Rh) / 2 + oy / k
```

These four advanced crop edges are in **post-input-crop source pixels**, not
screen pixels or percentages. The retained region starts at `(L, T)`. It has
aspect `Bw / Bh`, so proportional fitting reproduces the final box exactly.

The source-to-box mapping makes the offset sign explicit:

```text
box_x = k * (input_x - L)
box_y = k * (input_y - T)
screen_x = visible_box_left + box_x
screen_y = visible_box_top  + box_y
```

Valid content offsets keep every crop nonnegative:

```text
abs(ox) <= (k * Iw - Bw) / 2
abs(oy) <= (k * Ih - Bh) / 2
```

At baseline, one axis usually has no panning range. Increased zoom creates more
room to pan. Dragging stops at these explicit limits; invalid typed values show
an error rather than silently changing. Never reveal pixels removed by the input
crop. Actual firmware crop limits may impose stricter bounds or maximum zoom.

### Authoritative state and inverse controls

Store native source metadata, input crops, layer advanced crops, final visible
size and visible position. The crops are authoritative; do not persist a second,
independent zoom/offset transform that can disagree with them. The UI derives:

```text
Rw = Iw - L - R
Rh = Ih - T - B
k  = Bw / Rw                    # must also equal Bh / Rh
z  = k / max(Bw / Iw, Bh / Ih)
ox = k * (R - L) / 2
oy = k * (B - T) / 2
```

Reject imported crop tuples that do not satisfy positive remaining area, matching
aspect and supported precision. Do not silently stretch mismatched crops.
Preserve exact/rational values where practical; never round the intermediate
444.444… width to 445 before computing a crop.

Composing the two crop stages gives native-source retained bounds:

```text
native_left   = Il + L
native_top    = It + T
native_right  = Nw - Ir - R
native_bottom = Nh - Ib - B
```

This is a geometric identity, not an instruction to send the same crop twice.
Keep input and layer settings separate. The device adapter must establish whether
layer pixel crops use native dimensions, corrected input dimensions or another
reference and convert once. Percent crop denominators also need verification.

### Worked examples and expected values

All examples center the crop unless an offset is specified. Native dimensions
are 1920 × 1080 and the final layer box is 300 × 250.

| Case | Input area | Content zoom | Projected full usable input | Advanced crops L / R / T / B, in usable-input pixels |
| --- | --- | --- | --- | --- |
| Uncorrected 16:9 | 1920 × 1080 | 100% | 444.444… × 250 | 312 / 312 / 0 / 0 |
| Custom input | 1920 × 1000 | 100% | 480 × 250 | 360 / 360 / 0 / 0 |
| Custom input, tighter framing | 1920 × 1000 | 200% | 960 × 500 | 660 / 660 / 250 / 250 |
| Same, content moved right 30 px | 1920 × 1000 | 200% | 960 × 500 | 600 / 720 / 250 / 250 |

The uncorrected example removes **144.444… output pixels in total horizontally**,
or 72.222… on either side. The custom-input example removes **180 output pixels**,
or 90 per side. These are equivalent projected crop amounts, not the stored
source-pixel values in the table. Every resulting visible box is 300 × 250.

At 200% with Custom input, the native combined crops are left/right 660 and
top/bottom 290: the original 40-pixel input correction plus the 250-pixel layer
crop on each vertical side. The retained native region is 600 × 500, scaled by
0.5 to fill the 300 × 250 box. This checks that input correction is applied once.

## Device mapping: keep the container separate

Implemented device adapter: supports both an **output-sized Midra layer with internal
cropping** and a **source-aspect layer container with derived sizing
and cropping**. These use distinct container sizing algorithms, described in
[the device exchange contract](../interfaces/midra-device-send.md), with hardware
write/readback checks on Eikos 3.3.10. Physical-screen comparison remains outstanding. See
[remaining acceptance checks](plan.md#remaining-tests).


The intended hardware aspect choice is **Centered**, as requested. The
[Analog Way Midra 4K V3.2 manual, pp. 66 and 87](https://www.bhphotovideo.com/lit_files/1168243.pdf#page=87)
describes input crops in pixels and layer cropping with resizing, with a separate
layer aspect selection. It does not establish every processing-order or rounding
detail needed here for the installed firmware.

The adapter must return input crop settings, layer crop settings and units,
aspect mode, device layer width/height and device center position. Keep these
derived values distinct from the visible layer box. Verify whether a crop with
the exact target aspect allows the device container to equal that box in Centered
mode. If the device uses a different container, derive its size and position
from the measured mapping. An asymmetric crop may require a position correction.

The projected full-input sizes in the example table are explanatory geometry,
**not automatically the layer sizes to enter into WebRCS**. Do not confuse the
helper's cover calculation with the device's named Centered/Cropped settings.
Until verified, show calculations as provisional and do not label them a validated
device recipe. Unsupported results must not be approximated with masking.

Quantize only at the device boundary. Check the reconstructed visible rectangle
and sampled source region after quantization. Show the requested and achievable
values and residual error; block a supposedly exact recipe when its error exceeds
the verified device precision. Never silently alter the user's final box.

## Interaction and changes to the inspector

Retain the A/B/C positioning layouts. Add sections in this order after the draft:

1. **Source**: source identity, Source format and native dimensions; Input
   resolution preset/custom and its crop placement. Shared-input effects visible.
2. **Layer resolution**: final width/height and optional ratio lock, followed by
   the existing corner positioning controls.
3. **Framing**: Content zoom, Offset X/Y, Recenter and Reset framing. An explicit
   “Adjust layer crop” mode makes dragging reframe content inside the fixed box;
   ordinary dragging still moves the complete box. Escape leaves framing mode.
4. **WebRCS values**: derived size/position, crop settings and aspect mode, with
   an expandable explanation of units and any representability error.

Recenter preserves zoom and clears offsets. Reset framing sets baseline zoom
and centered crops; neither changes input correction or final visible bounds.
Pointer-wheel zoom remains camera zoom; content zoom uses its own control to
avoid accidental geometry edits. Each drag/edit is one undoable operation.

Cropping requires a content illustration: a source-coordinate test pattern can
show framing without live video. A user-provided source still may optionally be
added later and must remain distinct from the screen-wide reference image.
Do not suggest the screen reference picture is the layer's source content.

Moving or aligning a layer leaves crop settings untouched. Resizing the final
box preserves its selected positioning anchor; preserve content zoom and the
normalized center of the retained source region, then recompute the required
crop aspect. If that center becomes infeasible, show the necessary adjustment
before applying. Source/input-resolution changes use the same explicit adjustment
policy and affect all layers sharing the source. Do not mutate the crop merely
because the inspector's units or selected screen corner change.

## Acceptance matrix for implementation

- Round-trip crop ↔ zoom/offset for all worked examples without accumulating drift.
- Equal source/box aspect at baseline gives zero advanced crop; landscape-to-
  portrait and portrait-to-landscape use the correct constrained axis.
- Increasing zoom preserves final size and crop aspect; content pan changes
  opposite crop edges equally and oppositely while preserving retained dimensions.
- Moving the box changes only its screen position; changing camera zoom changes
  no source, input, layer or device geometry.
- Test zero/negative dimensions, oversized input custom dimensions, zoom below
  baseline, nonfinite values, exhausted crop area and out-of-range offsets.
- Test odd dimensions, fractional edges, extreme aspect ratios, device rounding
  and repeated zoom/reset cycles without drift or premature integer rounding.
- Input-crop changes affect shared-source layers consistently; independent layer
  crops remain independent. Undo restores all affected geometry atomically.
- Test resizing, source-format changes and reference-image mapping changes with
  off-screen layers and mixed selections; no implicit proportional repositioning.
- Compare the final visible bounds and sampled source corners with manual WebRCS
  results for centered/asymmetric crops and nonzero input correction. Verify both
  crop stages, aspect mode and device container rather than checking numbers alone.
- Keep these checks independent of UI rendering; add interaction verification for
  mode switching, focus, framing limits, preview and undo when the UI is built.
