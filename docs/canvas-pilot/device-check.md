# Eikos connectivity and Preview check

## Verified on 2026-11-13

User-authorized hardware check on an **Eikos 4K, firmware 3.3.10**. The unit
reported this identity through `GET /api/tpp/v1/system`. Its WebRCS bundle was
`app.e42fce4879f318b28cde.js`. The device address was supplied for this session;
it is not a compiled-in application default.

- HTTP read access and WebRCS WebSocket feedback worked without authentication
  on this configured device. Do not assume other installations permit this.
- `GET /api/stores/device` returned the state tree used by WebRCS, including
  screen canvas dimensions, layer center positions/container sizes, source
  assignments, crop/mask fields and transition state.
- The explicit REST read
  `/api/tpp/v1/screens/2/live-layers/1/presets/preview` reported input 4,
  matching the internal Preview bank. That REST response contained source/status,
  not the full geometry. Geometry was obtained from the WebRCS state endpoint.
- Screen 2's transition state was `AT_DOWN`: Program was `DOWN`, Preview `UP`.
  WebRCS's own bank-selection code maps `AT_UP` the other way around. These
  are physical banks, **not permanent Program/Preview identifiers**.
- A single Preview layer property was changed over the WebRCS WebSocket:
  screen 2, bank `UP`, layer 1, `position.pp.posH`, **1436 → 1437 → 1436**.
  Readback confirmed the intermediate change and restoration. The complete
  tested layer matched its original state after restoration. All four screen
  Program preset trees matched before, during and after this test.
- No TAKE, CUT, transition, Program, input/global crop, output configuration,
  source-routing or preset-memory command was sent. Raw snapshots remain
  temporary local artifacts and are not committed.

The message envelope observed and tested was `channel: "DEVICE"`, with
`data.path` an array of state-tree keys and `data.value` the new scalar value.
The read path for the tested property was:

```
device/screenList/items/2/presetList/items/UP/liveLayerList/items/1/position/pp/posH
```

This is evidence of a working bidirectional link, **not validation of either
advanced-crop export strategy**. Only horizontal center movement was exercised.
The WebRCS internal endpoint/protocol is firmware-specific, rather than a promise
of stable public REST geometry support. Manufacturer references are linked from
[the official Eikos downloads section](https://www.analogway.com/products/eikos-4k).
The linked API PDFs were not accessible during this check; protocol findings
above come from the installed device and its own WebRCS bundle.

## Read-only import bridge

Open **Midra connection…** in the main menu or **Settings → Midra integration**. Enter the device IP and click **Connect**. Then open the separate **Import → Midra layers…** window.
Reads run in a worker thread; canceling discards late results. Choose a screen and
choose a destination beside each device layer: **Do not import**, **New layer**,
or an existing visible, unlocked editor layer. Click **Import layers** directly
in this window; no prior selection or matching layer count is required. Duplicate
or removed destinations are rejected without partial changes. The entire import,
including new layers, is one undo step. Output resolutions must match. Reading never
changes the document; only applying the reviewed import does. An optional **Try autoconnecting at startup** preference makes one background read attempt using the remembered address. No polling or automatic retries occur. The address is remembered in application settings; no credentials are stored. The
footer shows **ONLINE** after a successful read (last-read status, not continuous
monitoring), and **OFFLINE** when refreshing, after a failed read, or after editing
the address.

The Rust adapter was tested against the same Eikos 3.3.10: two 1920×1080 screens;
disabled zero-canvas screen slots are filtered. A CLI smoke check is available:

```sh
cargo run -p canvas-pilot --example read_midra -- DEVICE_IP
```

A standard-library Python helper remains available for snapshot capture/offline replay:

```sh
python3 tools/midra_probe.py --host DEVICE_IP --save-snapshot /tmp/eikos-state.json
python3 tools/midra_probe.py --host DEVICE_IP --screen 2 --layers 1 2 --svg /tmp/preview.svg
# Replay a saved snapshot without connecting:
python3 tools/midra_probe.py --snapshot /tmp/eikos-state.json --screen 2 --layers 1 2 --svg /tmp/replay.svg
```

Select the matching number of editor layers, choose **Import → Geometry file…**, and review
mapping before applying. Match the editor output resolution to the device canvas;
SVG geometry import does not replace the editor's screen settings.
The helper exports **container geometry only**: center minus half-size becomes
top-left x/y, preserving half pixels. Source aspect fitting can make the visible
picture differ from this container. Routing, source resolution, source cropping,
effects, borders, opacity and zoom are not imported. Nonzero layer crop or mask
is refused rather than silently imported as final visible bounds. Device slots
are explicitly selected; unused slots are not assumed to be available layers.

The helper has no device write operation, bypasses system proxies, refuses
redirects, bounds reads to 32 MiB and never overwrites an existing output file.
Snapshots may contain network/private configuration; keep them out of Git and
sanitize before sharing. Its exchange contract is documented in
[interfaces](../interfaces/midra-device-import.md).

## Reviewed send and cropped-layer round trip

The later test authorization permits Program changes and accepts the physical-bank
TAKE race. This supersedes the earlier Preview-only restriction above.

**Export → Configure Midra export…** now supports
explicit Preview or Program targeting, per-selected-layer destination mapping,
output-sized or source-aspect containers, and exact before/after field review.
Each send writes directly, then reads the device back; it does not reject changes made since the snapshot. A partial
or uncertain result is reported without automatically repeating or rolling back writes.
Local editing, importing, undo/redo, connection and project loading never send.

On Eikos 4K firmware **3.3.10**, both sizing modes passed actual write/readback/restore
checks on screen 2 Preview layer 1 (UP) and Program layer 2 (DOWN). Cases used a
301 × 251 visible box at 12.5, 34.5, 1920 × 1080 native input corrected to 1920 × 1000,
200% content zoom and offsets +30, −10. All nine fields (size, center, four crop
edges, Centered aspect) matched readback; original values were restored after each
case. A final snapshot confirmed all screen preset trees matched their pre-test
state. The imported visible geometry matched within one pixel. Device-source routing,
TAKE, input configuration, output configuration, masks and effects were not changed.

The installed WebRCS bundle maps crop edges linearly from 0–65535 to 0–100% or
0–contentWidth/contentHeight pixels. Its layer size limits are 0–65535 and center
limits −67268–67268; this adapter requires positive dimensions. These findings
support the [device exchange contract](../interfaces/midra-device-send.md).

These tests verify firmware acceptance, readback and mathematical import round trip.
They do not replace visual comparison on a physical screen with a known test image;
the tested device reported layer state OFF. Platform/display acceptance remains open.

The explicit hardware diagnostic (never part of automated CI) is:

```sh
cargo run -p canvas-pilot --example check_midra_send -- --apply-and-restore DEVICE_IP 2 1 preview
# Program writes are deliberate here:
cargo run -p canvas-pilot --example check_midra_send -- --apply-and-restore DEVICE_IP 2 2 program
```

Centered layers with valid content dimensions and no mask now import visible
bounds and source-pixel crop framing. Their combined crop is stored in the local
layer crop; the device does not reveal which part originated from this editor's
input correction. Other aspect modes retain container-only import and refuse
nonzero cropping. Shared input settings and source routing are not synchronized.

The manufacturer's [REST API guide v3.0.13](https://s3.eu-west-3.amazonaws.com/aw.store01/Site+Internet/Series/Midra+4k/Common/Rest+API+Programmer%27s+Guide/midra4k_rest_api_programmers_guide_v3_0_13.pdf)
was inspected. It documents logical Preview source assignment, but not geometry
writes; this implementation therefore uses the installed WebRCS WebSocket protocol.
The physical bank is captured in the review and remains fixed across the send.

## Missing preset apply diagnosed

The operator reported that WebRCS geometry changed while the physical multiviewer
Preview did not. The bundled WebRCS code calls `z1()` after geometry edits, which
dispatches `DEVICE_PRESET_CONTROL_SET_ONE` for `xUpdate` false, then true.
The device snapshot had `device/preset/control/pp/enableAutoUpdate: false`.
Earlier tests checked only staging-store readback and therefore did not establish
that geometry had reached video processing. The exporter now sends this apply
pulse after geometry writes, even when no geometry fields differ. A direct pulse
was sent to the connected Eikos to apply its already-staged layout. The operator confirmed that the physical multiviewer Preview then matched WebRCS, establishing the missing apply pulse as the cause. Layer status `OFF` is not
sufficient evidence that the operator's layers are disabled, and must not be used
as that diagnosis.

## Input preview capture checked on 2026-11-14

The same WebRCS client exposes GET /api/device/snapshots/inputs/:id.
A read-only request to active input 3 returned HTTP 200, PNG, 512 × 288
(591155 bytes). Input 1, with no signal, returned a blank 128 × 128 PNG.
The input state reported snapshots enabled with size 512_512; the active
image retained its 16:9 aspect. The response included Last-Modified and
Cache-Control max-age=0. No continuous stream or write was needed. Raw input
images/state stay temporary and are not committed.

The implementation fetches a still only on demand, limits concurrent requests
to one and enforces a five-second minimum interval. It does not change snapshot
enable/size, input routing, capture-library commands or any preset.
See [input preview interface](../interfaces/midra-input-preview.md).


### Allocated layers versus firmware slots

The snapshot contains eight live-layer slots per screen, even when the active
configuration allocates only one or two. The Rust adapter reads
`device/preconfig/status/stateList/items/CURRENT/screenList/items/{screen}/pp/layerCount`
and exposes only IDs 1 through that count for import, mapping and sending.
The cached Eikos state reported one allocated layer for screen 1 and two for
screen 2; other screen allocations were zero. This is independent of input routing,
signal presence, OFF state and the number of physical inputs. Background and
foreground are not live-layer destinations. Staged NEW configuration is ignored.
