# Source dimensions, resolutions and cropping

The local layer model uses **processed source dimensions**, layer crop edges,
fit, masks and final visible bounds. Input preprocessing belongs to the shared
[Input settings window](guide/formats-and-crop.md#shared-device-input-settings).
There is no additional local input crop. After changing input preprocessing on
the switcher, reload affected layers to refresh their source dimensions.

## Names and coordinate spaces

| Name | Meaning |
| --- | --- |
| Source/input resolution | Dimensions of the processed picture consumed by layers |
| Output resolution | Screen dimensions; changing these does not resize layers |
| Layer resolution | Final visible box, or container for an explicit fit mode |
| Layer crop | Edges removed from the processed source before layer fitting |
| Content zoom / offsets | Derived controls for layer crop; not another saved transform |
| View zoom | Camera magnification only; never changes layout pixels |

For example, if the device reports processed content as 1920 × 1000, use that
as the layer input. Do not also subtract a local 40-pixel top and bottom crop.
Physical source cropping and content ratio remain separate device settings.

Implementation limits are 1–16384 pixels per source/output side, positive finite
layer sizes up to 1,000,000 pixels and normal content zoom of 100–10000%.
These are editor limits, not a promise of hardware capability.

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

These four advanced crop edges are in **processed-source pixels**, not
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

Store processed source metadata, layer advanced crops, final visible
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

Retained bounds in processed-source coordinates are `(L, T, Iw - R, Ih - B)`.
Input preprocessing has already occurred; never add its crop to the layer crop.

## Device mapping and explicit fit

The adapter converts layer crop to the device's units using processed dimensions.
Input edits have their own reviewed send and readback; layer sends do not contain
input-setting writes. Firmware rasterization and quantization can still differ
from the mathematical editor preview.

Explicit 1:1, Fullscreen, Centered and Cropped fit modes operate after layer crop.
Their container and image bounds differ from the normal cover model above; see
[fit modes and masking](guide/formats-and-crop.md). Source-relative masks follow
the processed picture; layer-relative masks follow the visible container.

## Persistence and recovery

Projects and supported export snapshots retain processed dimensions, source
relationships, layer crops, fit and masks. Schema 20 removes old local pre-crops
and inactive input proposals. [Best-effort Open project](guide/files.md#recovering-a-partly-readable-project)
reports discarded legacy settings and opens repaired files as unsaved copies.
Shared device input drafts remain session-only.
