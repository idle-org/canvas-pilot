# Device adapters and Aquilon exchange

Canvas Pilot's device-neutral snapshot contains a profile, screens with native
IDs, output pixel resolutions, explicit Preview/Program banks, and allocated
layers with source identity, visible/container geometry, optional source/crop
framing, optional explicit crop-first fit, and unsupported-mode explanations. It is transient, never a save format.
The small profile registry selects family implementations; UI, scopes, history,
transport and editor geometry do not contain native path substitutions.

Persistent destinations use [project version 15](canvas-pilot.md): family +
screen + layer. Midra links migrate explicitly from versions 1–12. Neither bank
roles nor credentials nor raw device state are persisted. Current standalone
profiles do not support linked-unit identity; linked Aquilon devices are refused.

Midra represents its four explicit modes using the shared `LayerFit` enum and
container geometry; fit math stays in the geometry model and wire conversion in
the adapter. A null fit retains the normal visible-box zoom/crop workflow. Aquilon
currently imports that normal workflow and rejects explicit-fit sends; future
adapters must explicitly implement or reject each mode.

## Aquilon profile and reads

Recognized standalone models: **RS alpha, RS1–RS6, Cmini, C, C+ and Cmax**.
The adapter accepts every numeric firmware version and attempts the bounded
LivePremier 6.2 protocol reads. Outside **6.2.x**, connection and transfer review
show: **backward compatibility is not guaranteed**. This is best-effort compatibility,
not an implementation of every historical protocol: missing or incompatible fields
fail explicitly before a send can be planned. In particular, firmware before 4.x
uses a different screen/group schema and may not connect. No legacy path guessing
or full-store fallback is performed. Linked systems and unknown models remain rejected.

Model codes follow the manufacturer's AWJ V6.2 guide, section 2.1:
`NLC_RSALPHA`, `NLC_RS1`–`NLC_RS6`, `NLC_CMINI`, `NLC_C`, `NLC_CPLUS`,
`NLC_CMAX`. HTTP names use the `AQL ` prefix and model spelling above; RS alpha
also accepts `RS Alpha`. The HTTP model must match the exact AWJ model, not just
another member of the Aquilon family. Capacities still come from discovered state.
**Cmax 6.2.79** has recorded emulator acceptance; see the later **RS1 6.2.79**
mask round trips in the [acceptance record](../canvas-pilot/aquilon-compatibility.md). Other models and firmware
have synthetic regression coverage; physical-output acceptance remains outstanding.

`/api/tpp/v1/system`
on the configured HTTP port supplies identity. Aquilon geometry uses **AWJ TCP
10606 on the same IP**, independently of the HTTP/WebSocket port. AWJ model and
firmware are checked against the HTTP identity. A blocked/disabled AWJ port is an
explicit connection error; there is no full-store HTTP fallback. Midra retains
its existing HTTP reader. The emulator has authentication disabled; HTTP auth
failures remain explicit and credentials are not saved.

The [manufacturer's AWJ guide V6.2](https://dwn01.analogway.com/Site+Internet/Series/Aquilon/Common/AWJ+Protocol/AW_Aquilon_AWJ_ProgrammersGuide_V6.2.pdf)
documents property reads over TCP. Requests are `{"op":"get","path":"..."}`,
terminated by byte `0x04`. Canonical paths start with `DeviceObject`; list,
item and property segments use `$screen`, `@items` and `@props`. Responses are
matched by exact canonical path, including out-of-order replies. Error, missing,
unexpected, duplicate, truncated and oversized responses fail the read.

Reads discover `itemKeys`, then screen modes and available inputs. Only enabled
screens, their allocated numbered layers in the two current role banks, active
input plugs, framing settings and snapshot availability are requested. No memory
banks, inactive layer slots, hardware inventories or other large subtrees are read.
These targeted values populate the same normalized adapter state as before.
Requests are pipelined in batches of **64**; parent-object reads are avoided because
the emulator returns empty objects for them. A second pass compares all selected
values, including topology and bank roles; a changing snapshot is rejected instead
of being used for a send. No cache, persistent subscription or AWJ writes are used.

Bounds: **256** entries per discovered collection, **65536** selected fields,
**16 KiB** per response, and **16 MiB** aggregate response bytes across both passes.
TCP connect/write timeout is **3 seconds**, collection deadline **30 seconds**,
and read waits check cancellation at **250 ms** intervals. HTTP identity retains
its 5-second connect / 45-second request limits. All work stays on the worker.

On the 6.2.79 emulator, a complete two-pass read received **197180 bytes** over
AWJ plus **124 bytes** of HTTP identity, versus the previous **124077157-byte**
store download: **99.84% less downloaded payload**. AWJ requests totalled 184978
bytes. Three direct debug-build reads took **0.314, 0.302 and 0.297 seconds**;
a counting proxy took 1.601 seconds. These are local emulator measurements,
excluding TCP/HTTP header overhead, not guarantees for every device configuration.

Enabled `screenList` and `auxiliaryList` entries supply `status.pp.layerCount`
(0–128) and `status.size.pp.sizeH/sizeV`. Canvas dimensions also obey editor
limits (1–16384 integer pixels per side). `status.pp.mode: DISABLED` excludes an
entry even if it retains nonzero stored dimensions. Layer IDs are numbered slots,
not inputs; native/background layers are omitted.

Each screen's `status.pp.transitionGroup` identifies a `screenAuxGroupList` item.
At `AT_UP`, its `control.pp.presetUp` is Program and `presetDown` Preview;
`AT_DOWN` reverses roles. Distinct A/B/C bank identities are validated. Other
transition states block import/send planning. Roles are not assumed from bank names.

## Geometry, crop and writes

Layer path: `device/{screenList|auxiliaryList}/items/{S1|A1}/presetList/items/{bank}/layerList/items/{layer}`.
Position and size share `position.pp`: `posH`, `posV` are centers; `sizeH`, `sizeV`
are pixel sizes. Editor geometry is top-left based. Firmware accepts/stores f32
values, but WebRCS 6.2.79 throws `oldAnchorRect are not integers` when rendering
fractional position/size fields. Therefore sends round sizes and centers to whole
pixels (sizes 1–1000000). Saved/editor geometry retains double precision; imported
half-pixel/top-left geometry is not rounded. Conversion rejects a rounded visible
size error above one pixel. Center rounding contributes at most half a pixel.
Strict readback checks the quantized values rather than silently accepting clamps.
Only `MIDDLE_CENTER` anchors are currently accepted. Source routing is
`source.pp.inputNum` (`LIVE_n`, `NONE`, etc.); production geometry sends omit it.

`cropping.classic.pp.{left,right,top,bottom}` are integer fractions 0–65535.
Layer crop combines the editor input crop and dynamic crop in native source pixels,
then rounds `edge/native_axis*65535`. Exhausted axes are rejected. Sends set
`cropping.classic.pp.aspectOverride` to `CENTERED`. Rectangular blanking uses the
independent `cropping.mask.pp.{left,right,top,bottom}` fields described below.
Crop and mask decreases precede increases, releasing opposite edges first.

Live input `LIVE_n` resolves `inputList.items.IN_n` and its active plug. The
plug's `status.signal.pp.imageWidth/imageHeight` supply native dimensions.
Shared input cropping and aspect transforms are explicitly unsupported for
visible-bound conversion in this initial profile; their controls remain in
WebRCS and are never sent by the adapter. Unrouted/color layers with zero crop
can still import container geometry. Unknown source dimensions with nonzero
crop do not silently produce a visible-box estimate.

Both Output-sized and Source aspect use the existing editor sizing contract.
The retained cropped source fits inside the chosen container with a uniform
minimum-axis scale. Quantized crop must reproduce the requested visible size
within one output pixel. Import reverses this conversion for centered framing;
unsupported fit modes and malformed masks are reported on each layer and block its import.

Explicit sends refresh state first and reject stale banks, source/framing values
or changed destinations before writing. They use the shared root WebSocket
`DEVICE` framing documented in [Midra exchange](midra-device-send.md), then pulse
`device/screenAuxGroupList/control/pp/xUpdate` false → true. No TAKE, routing,
shared input or effect writes occur. Reviewed mask fields are part of the geometry transfer. Changed fields are read back; partial
writes, failed applies and mismatches report uncertain outcomes without automatic
retry writes or rollback. The emulator diagnostic explicitly restores its baseline.

## Input previews and acceptance limits

Discovery uses available `IN_n` entries, current-plug signal validity and
`snapshot.pp.enable`. The adapter resource limit is 256 inputs; unavailable inputs
remain excluded. `GET /api/device/snapshots/inputs/n` returns PNG data. Inputs 1
and 17 on the emulator returned **256 × 144** PNGs. The existing 16 MiB image
limit, decoder bounds, serialized five-second minimum and pooling apply unchanged.
No polling setting or snapshot size is written to the device.

Sanitized native fields are recorded in `tests/fixtures/aquilon-6.2.json`;
no raw emulator dump, labels, network settings or credentials are committed.
Synthetic variations cover auxiliaries, multiple banks, malformed states and
unsupported modes. Emulator readback and WebRCS rendering are separate acceptance
checks from physical output. Older firmware, other models, linked units and
physical-output behavior remain unverified.

## Mask appearance

Device-neutral layer appearance pairs optional fit with independent blanking
(`LayerAppearance`). Imports apply geometry, source/crop, fit and mask atomically.
Aquilon now imports and sends rectangular masks, including fully blanked axes.
This does not implement Cut & Fill, keying, effects or explicit layer-fit sends.

WebRCS 6.2.79 (`app.1a22b5b509cf7c14eb86.js`) defines all four mask fields as
integers **0–65535**. Its Mask panel converts those values to percentages or
**layer width/height** pixels and limits an edge to 65535 minus its opposite.
Its rendering clips the layer with the same percentage rectangle. These fields
are independent of classic source crop. The `canUseMask` resource flag exposed by
the AWJ guide concerns Cut & Fill; it is not used to enable rectangular blanking.

Sends convert either editor mask origin (Layer or Source) to an output-space
reveal rectangle, then express it relative to the rounded device container.
For Source aspect sizing this includes the centered margins around the visible
image. Clip edges to the container, round fractions ×65535 and cap opposite sums
at 65535. Reject a quantized reveal position/size error above one output pixel,
measured on the centered visible image. A completely zero editor mask explicitly
sends four zeros, clearing any destination mask in the reviewed send. Geometry,
classic crop and source routing are not changed by mask conversion itself.

Import validates integer ranges and opposite-edge sums, builds the device-container
reveal rectangle, then intersects it with the centered visible image. The imported
mask is relative to that image (Layer origin), preserving its geometry and crop
rather than shrinking the layer to the mask. An unrouted layer with zero classic
crop can retain container geometry and its mask. Fully blanked axes are legal;
malformed masks block that layer's import and send planning.
The device cannot preserve the editor's Source-origin editing intent; retain the
project for that. Blanking in already empty container margins has no visible effect.

All mask values participate in the exact before/after review, stale-destination
checks, apply pulse and readback. Failure does not trigger an automatic retry or
rollback. The explicitly invoked diagnostic backs up its baseline to a temporary
JSON file before writes, exercises both banks, origins, sizing strategies and
mask/reset cases, then restores routing, geometry, crop and masks and verifies
all targeted screen/auxiliary state. It is never a CI/hardware test.
