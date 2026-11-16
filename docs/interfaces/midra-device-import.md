# Midra device snapshot and container SVG bridge

Implementation: [`tools/midra_probe.py`](../../tools/midra_probe.py).
Tests: [`tests/test_midra_probe.py`](../../tests/test_midra_probe.py).
Hardware evidence and usage: [Eikos check](../canvas-pilot/device-check.md).

## Snapshot

`--save-snapshot` writes UTF-8 JSON containing the raw object from
`GET /api/stores/device`; the root key is `device`. No editor project version
or custom wrapper is added. This is **vendor state**, not a Canvas Pilot project.
The firmware owns its schema. The Rust adapter accepts Eikos 4K, Pulse 4K,
QuickVu 4K and QuickMatrix 4K identities and validates the shared state fields.
The Rust connection checks identity before downloading the device store: unsupported
families, including Aquilon/LivePremier, receive an explicit compatibility error.
Its supported Midra state reads remain capped at 32 MiB.
Reads were checked on Eikos 4K 3.3.10 hardware and the Pulse 4K 3.3.7 emulator
(one 1920 × 1080 screen with four allocated layers). QuickVu and QuickMatrix
have synthetic parser coverage; physical acceptance remains outstanding.
The Rust connection accepts an IP with optional port, shared across HTTP and
WebSocket; see [connection syntax](../canvas-pilot/guide/device.md#connect-a-device-or-emulator).
`--snapshot` reads it offline, at most 32 MiB, and rejects missing required fields
when extracting geometry. Unknown vendor fields are preserved on capture.
Snapshots can contain private configuration. They are never sent back to a unit.

Geometry extraction uses these paths below `device`:

- `transition/screenList/items/{screen}/status/pp/transition`: only `AT_DOWN`
  (Preview `UP`) or `AT_UP` (Preview `DOWN`) is accepted. Transitional states fail.
- `screenList/items/{screen}/canvas/status/size/pp/{sizeH,sizeV}`: canvas pixels.
- `screenList/items/{screen}/presetList/items/{bank}/liveLayerList/items/{layer}`:
  `position/pp/{posH,posV}` is the center in canvas pixels;
  `size/pp/{sizeH,sizeV}` is the device container size in pixels.
- `crop/pp` and `mask/pp`: all four edges must be zero for SVG extraction.
  Their nonzero units/semantics are intentionally not converted yet.

## Container SVG

This uses the existing [geometry SVG marker](canvas-pilot.md#legacy-svg-geometry-exchange-version-1),
`data-midra-layout-geometry="1"`, so a full-screen container is not mistaken for
an ignorable background. Root `width`, `height`, and `viewBox` come from the device
canvas. One or two rectangles are emitted in explicit `--layers` order, with IDs
`screen-{screen}-layer-{layer}`, opaque fill and no stroke. Duplicate selections
are refused. Missing slots fail rather than being skipped.

Rectangle x/y = center minus half-size. Values are not rounded: this is an import
bridge preserving device measurements, not the editor's rounded export. SVG
imports accept fractional pixels. Numeric values must be finite, non-boolean,
within ±1,000,000; widths/heights must be positive. The editor performs its own
validation again before applying. Existing destinations are refused.

Only container position/size round-trips. No source association, aspect fitting,
input crop, zoom, visibility, borders, effects or device connection is encoded.
The desktop SVG mapping dialog must be reviewed before applying; the image's
visible bounds are not guaranteed to equal the container. This format must not
be presented as a full device/project backup or as validated crop conversion.

## Fractional content resolution in the Rust adapter

Device-reported `status/pp/contentWidth` and `contentHeight` can be fractional.
For centered crop reconstruction, the Rust adapter rounds them to the nearest
whole pixel (positive half-pixels round upward), while keeping raw vendor state
unchanged. Each original value must still be finite and within 1–16384; rounding
does not rescue out-of-range values. The rounded dimensions become the editor's
native source resolution, and normalized crop edges use those same dimensions.
Container positions/sizes and the existing SVG bridge retain fractional precision.

A nonblocking layer warning gives both original and rounded dimensions. It is
shown in connection/import/mapping feedback and retained after linked loading.
Whole-pixel reports produce no rounding warning. The existing no-content fallback
for unrouted, uncropped layers is unchanged. Invalid crops and real resolution
mismatches remain errors.
