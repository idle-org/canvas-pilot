# Midra input preview stills

The Rust adapter uses the configured IP and optional port for this endpoint
(default 80), including emulator addresses such as `192.168.10.146:3000`.

This is a device-read interface, not a new project or export format. Verified on
Eikos 4K firmware 3.3.10, WebRCS `app.e42fce4879f318b28cde.js`.

## Discovery and request

The connected snapshot's `/device/inputList/items/INPUT_N` supplies:

- `status.pp.isAvailable`: only existing hardware inputs appear.
- `status.pp.plug`: selects `plugList.items[plug].status.signal.pp.isValid`
  for the last-read signal indicator.
- `snapshot.pp.enable`: capture is disabled in the editor when WebRCS snapshots
  are disabled. The application does not write this field or snapshot size.

Each input-selection capture, explicit **Capture still**, or automatic timer request issues
`GET http://<configured-IP>/api/device/snapshots/inputs/N`, with N an integer
1–16 and `Cache-Control: no-cache`. IPv4 and IPv6 literal addresses are accepted;
no proxy or redirect is followed. No immediate retries, WebSocket subscription,
still-library capture or source-routing/preset writes are involved.

Only one request may be in flight, with at least five seconds between starts.
Closing the separate import dialog discards its pending dialog result; cancelling
a request also discards its result. Neither permits another request until the
first completes or times out. Closing layer properties does not cancel a direct
layer capture. The request has a 5-second connection timeout
and 20-second overall timeout. Device-provided resolution is retained; no request
to increase resolution or refresh frequency is made.

## Response and validation

A successful response must be a PNG, at most 16 MiB, and pass the common image
decoder's dimensions/allocation limits. HTTP errors, redirects, invalid PNGs,
oversize responses and decode errors leave references unchanged.
The active-input test returned 512 × 288 pixels; an inactive input returned a
blank 128 × 128 PNG. The device controls image freshness (Last-Modified) and
resolution; GET fetches the latest available snapshot, not a guaranteed
new full-resolution frame. Signal indicators may be stale until reconnect/refresh.

## Input picker filtering

Both the per-layer Preview picker and Import input snapshot picker hide inputs
whose last-read active plug reports no valid signal. **Show inputs without
signal**, inside each dropdown, reveals those available hardware inputs with a
**No signal** label. The toggle is shared for the session and defaults off; it is
not persisted. Unavailable hardware inputs remain excluded. An explicitly chosen
input is retained and labelled if it loses signal or is hidden by this filter;
filtering does not change assignments, capture availability or refresh eligibility.
With no signal-bearing inputs, initial selection remains empty until the user
reveals and chooses one. Signal status comes from the connection snapshot and
updates when the connection is refreshed, not from PNG polling.

## Editor and persistence

**Import input snapshot…** displays the image and its actual pixel dimensions.
Its destination selector lists every editor layer by stable ID and a Reference
image option, independently of current selection. Layer destinations offer
**Keep refreshing this layer**; this is staged until Import into layer and sets
that layer's automatic-refresh flag. Reference destinations never expose or apply
that flag and remain fixed images. Import does not alter selection or geometry. **Add as reference**
is explicit and obeys the 32-image/64 MiB combined-reference limits. Existing
upscale/aspect warnings apply. Capture in this separate import dialog only stages
an image until Import into layer or Add as reference is chosen.
Changing device address/disconnecting invalidates the pending preview; input changes
clear the old preview. A cancelled/late result cannot become a new reference.

The initial image name contains the input number and configured device IP.
Added stills use the existing [project reference object](canvas-pilot.md):
name, embedded PNG image_base64, visible, opacity, preserve_ratio,
native_size, offset. No new fields, version or migration are needed.
**Import into layer** assigns a session-only still texture to the destination
editor layer's stable ID and current local input. It uses the editor's native
source rectangle, input crop and dynamic crop/zoom mapping, including faded
overflow while editing. It does not infer or change device routing. Changing the
local input hides that assignment; opening/New/Recovery clears assignments.
Reordering layers does not move an assignment to another layer.

In the layer Preview tab, the workflow is **Enable input preview → Input →
automatic first still → optional Capture still / Auto-refresh this preview**. Enable and auto-refresh are separate
per-layer booleans saved in project version 11. An enabled preview may have an input selected but no
texture yet. Capture still directly replaces only that layer's texture; repeating
it recaptures. Auto-refresh can fetch the first image without a manual capture.
The physical input picker uses available device inputs and is unrelated to the
editor input identity or Midra layer mapping. Input changes discard the old image.
Manual jobs bind to the layer's stable ID, source identity, physical input and
host at request time; selection changes do not redirect results. Changed/removed
assignments and disabled previews reject late results. Disabling a preview hides
its image and pauses requests without losing its still or chosen opacity. **Crop guide
opacity** is a per-layer saved value from 0–100%, initially 35% when assigned.
The source test-pattern guides paint above the image; selection borders/handles
remain opaque. When the preview is hidden or absent, the slider is disabled at
100% and the original opaque pattern is used. Showing the preview again restores
the chosen opacity. Outside-box content retains the existing 50% ghost treatment.
The same mapping/compositing is used in the viewport and the dedicated Preview
tab in Layer properties. That tab contains capture, auto-refresh, visibility and
guide-opacity controls; Crop contains geometry controls. Opening Preview does not
activate visual crop editing. The layer context menu also exposes preview controls.

Layer-preview assignments, original image bytes, visibility toggles and guide opacity
are saved in project version 11, but omitted from geometry exports. Add as reference remains the fixed-reference path;
generic artwork exports include those references only through the existing
background option. Reloading a project makes no capture requests.
**Auto-refresh this preview** is opt-in per layer and restored paused, with a shared request interval
of 5–300 seconds (default 10). Assignments also retain the device address and
physical input number in the project. Visible layers with enabled automatic previews, unchanged
local inputs and the current device address are eligible; unavailable or
snapshot-disabled inputs are skipped. Unique inputs refresh round-robin, one
request per interval. Every completed capture (manual, selection or timer) shares
one decoded texture across matching automatic previews and layers queued for a
first still on the same device/input, including targets queued while the request
was running. Matching one-shot requests are consumed together on success or failure,
so a failed fetch does not retry once per layer. A queued snapshot-import preview
can share the result too, but still requires explicit import. Changed, removed or
disabled targets reject the result; other inputs keep their own requests.
Existing manual stills without a queued capture are never replaced by another
layer's capture.
Selecting an input queues a one-shot capture in either picker, including the first
signal-bearing input selected when opening snapshot import. Selection captures
use the shared five-second minimum rather than waiting the configured refresh
interval, and take priority over periodic refreshes. While cooling down or fetching,
only the latest choice per target is retained. Closing snapshot import, disabling a
preview, changing its source/device or removing its layer cancels its queued request.
Snapshot-disabled/unavailable inputs are not requested. A failed selection capture
is not retried automatically unless auto-refresh is enabled. Capture still remains
available for explicit recapture; importing the dialog image remains explicit.
Manual, selection and automatic capture share one worker slot and the five-second minimum.
Automatic capture pauses offline, during project loading and while the capture
dialog is open. Errors retain the last good image and wait for the next interval;
there are no immediate retries. Turning off refresh prevents applying an in-flight
automatic result. Project replacement restores saved assignments and discards stale results.
Automatic refresh never changes fixed references or device settings. Timer settings
are persisted in project version 11. Saved automatic previews stay paused until an explicit capture or resume; opening a project makes no device requests.
Live feeds and separate image-file saving are not implemented.

After successful connection/linked load or device-layer import, untouched layer
preview controls preselect the linked device layer’s `INPUT_n` source when that
physical input is available. Signal presence is not required. Existing selections
are preserved; no texture, enabled flag, auto-refresh flag or capture-on-selection
request is created. Opening/enabling these controls does not fetch the preselected
input; Capture still or explicit auto-refresh starts capture. The preselected input is saved with the preview assignment in project version 11.

Aquilon uses the same capture scheduling and image limits through its own discovery profile (`IN_n`, layer source `LIVE_n`, up to 256 discovered inputs). See [Aquilon discovery and endpoints](device-adapters.md). Midra retains its 16-input limit; family-specific IDs are not guessed from a saved layer number.
