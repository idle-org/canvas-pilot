# Device connection and transfers

[Canvas Pilot](../introduction.md) · [Guide contents](overview.md) · [Starter files](../../templates/canvas-pilot/catalog.md)

## Connect a device or emulator

Enter an IP address in **Device IP / port**, optionally followed by a TCP port:
`192.168.10.146:3000`. Without a port, the app uses port 80. IPv6 addresses
with a port use brackets, for example `[::1]:3000`. Omit URL schemes and paths.
The address is saved in preferences and used for identity, input snapshots,
automatic reconnect and WebSocket sends. Aquilon also requires AWJ TCP port
10606 on the same IP for its targeted geometry reads; Midra reads use the HTTP port.

Use **Recent machines…** to switch to one of the last four successful connections.
Choosing an entry connects immediately to read its state; it does not send layers.
History survives restarts, retains custom ports and excludes failed attempts.

The adapter accepts Eikos 4K, Pulse 4K, QuickVu 4K and QuickMatrix 4K and reads
screen sizes and allocated layer counts from the device instead of assuming
Eikos's configuration. This covers the Midra **4K** family, not earlier Midra
products or Alta. Aquilon / LivePremier has its own adapter, described below. Required firmware fields
are still validated. Eikos 4K hardware and a Pulse 4K 3.3.7 emulator have been
read successfully; QuickVu/QuickMatrix have parser regression coverage but
still need individual emulator/hardware acceptance.

## Use folders as local device layouts

The **Apply to** selector is shared by the status bar, Export menu, and
send/import windows. It starts at **All layers**, regardless of canvas selection.
Choose **Only selected layers** for an explicit selection filter, or **Folder: …**
for a complete folder. The choice lasts for the session; it is not a project setting.
It affects device transfers, not clipboard/file import or geometry export scopes.

To prepare several layouts that reuse the same device layers:

1. Put each layout in a folder and configure its saved screen/layer links through
   **Midra mapping & transfer…**. Every row can still be remapped or unlinked.
2. Choose **Folder: …** in the status selector, or right-click a folder and choose
   **Use folder for Midra transfers**.
3. Click **Send folder links**. Then click another folder name and send again:
   Folder mode follows the clicked folder. Clicking a folder never sends by itself.
4. **Load folder links** reads fresh Preview geometry/cropping into that folder.
   Loading requires visible, unlocked destinations and supports Undo.

Multiple editor layers may link to the same device destination, within or across
folders. Mapping shows a warning listing the other linked layers and their folders.
Sending is disabled when the chosen scope contains overlapping **screen + layer**
destinations, with a list of the conflicts. Choose a conflict-free folder or selection,
or remap/skip rows; links outside that send scope do not block it.
Linked send/load skip unlinked layers. An empty selection or missing folder never
falls back to All. Transfers already started retain the links captured at click time.
These are local geometry layouts, not device preset recalls or source-routing memories.

**Import layers from Midra** proposes unambiguous saved links within the scope.
Each row offers Skip or a scoped existing layer; All/Folder scopes also allow New layer. **Use saved links**
restores those suggestions; **Import unmapped as new layers** fills unused rows.
New layers join the chosen folder, and importing saves their reviewed screen/layer
links with the geometry as one Undo step. Existing layers keep their folder. Multiple
local copies of the same device layer can be updated together with linked Load;
the manual import dialog requires one unambiguous destination per device row.

## Midra connection and sending

Open **Midra connection…** in the main menu (also in **Settings → Midra integration**), enter the device IP, then **Connect**. The address saves automatically in the application config and survives project loads and restarts; startup connects automatically only if **Try autoconnecting at startup** is enabled in Settings → Midra integration.

Open **Import → Midra layers…** for the separate layer import window. **Refresh layers** reads a fresh snapshot; **Connection settings…** opens the connection window.
Choose a destination beside each device layer: existing layer, new layer, or skip.
Explicit Midra modes import container bounds, crop, fit and mask; unsupported inherited crop cases
are explained in the review. Imports are one undoable edit.

Open **Export → Midra mapping & transfer…** even while offline, then use
**Connection settings…** to connect. Mapping and transfer controls appear once
device data is available. Choose editor layers,
map each to a Midra layer, select **Preview** or **Program**, and select a container
strategy: **Output-sized** or **Source aspect**. Expand **Exact device changes** to
review sizes, centers, crop values and aspect, then click **Send mapped layers**. The final row groups **Load linked layers**
(down arrow, device to editor) and **Send mapped layers** (up arrow, editor to
device); the Load label follows the selected transfer scope.
The button is disabled while a send is in progress. Matching stored values can still be sent to apply staged geometry to hardware.
The source's native dimensions must match the destination's current content.

The target bank is fixed when configured. Sending starts immediately without a stale-state check. Writes are sequential;
partial failures require refreshing, not automatic retries or rollback. Success
means all changed fields matched device readback. There are no automatic sends
from edits, imports or undo. Closing the connection window does not cancel a send
already started; wait for its result before loading another project.

See [hardware checks and limitations](../device-check.md) and the
[device exchange contract](../../interfaces/midra-device-send.md). The footer's ONLINE
status reflects the last successful read, not continuous polling.

## Capture an input still from Midra

For a layer, open **Layer properties → Preview** (or right-click → **Configure capture…**):

When a connection or layer load completes, linked layers preselect their assigned
physical input if available. This does not enable previews or request an image,
and existing capture choices are preserved. Click **Capture still** when ready,
or explicitly enable auto-refresh. Inputs without signal can still be preselected.

1. Check **Enable input preview**.
2. Choose the physical input to preview. Inputs without signal are hidden by
   default; **Show inputs without signal** inside the dropdown reveals them with
   a **No signal** label. The same filter applies to the snapshot-import picker.
   Signal indicators reflect the last connection refresh. This only chooses an image source; it
   does not change Midra input routing or the editor's source-format identity.
3. Selecting an input captures a first still automatically and loads it straight
   into this layer. It waits only for the shared five-second request throttle, not
   the auto-refresh interval. Click **Capture still**, directly below the input
   selector, to replace the still. Selecting a different input queues its first
   capture too; rapid changes keep only the latest choice. The snapshot-import
   picker also captures on selection, but still requires an explicit import.
4. Alternatively, enable **Auto-refresh this preview** below Capture. It fetches
   the first image automatically and then updates it on the timer.

Auto-refresh is per layer. **Request interval** is shared across automatic
previews, defaults to 10 seconds and accepts 5–300 seconds. Visible automatic
previews refresh one input at a time; automatic previews sharing an input share
one fetched image. First captures queued for that same input also share the
result, even when selected while a request is running. One fetch serves all
waiting layers; existing stills with auto-refresh off remain unchanged.
Requests are serialized with a minimum five-second gap, including manual captures.
A failed refresh retains the last image; changing the input clears the previous
input's image. Disabling the preview hides it and pauses its refreshes.

**Crop guide opacity** defaults to 35% with an image; without an enabled image,
guides remain fully opaque and the slider is disabled. Selection handles stay
fully visible. Layer previews, captured stills and timer settings are saved in the project.
Saved auto-refresh reopens paused: use **Resume auto-refresh** or **Capture still**
to resume. Opening a project does not fetch images. The timer
pauses offline and while the separate capture/import dialog is open. Reduced
snapshot resolution and freshness are controlled by the device. If snapshots
are disabled for an input, enable them in WebRCS and refresh the connection.

**Import → Import input snapshot…** opens a capture/import window (also available
from the reference-images context menu). Choose the physical input and a
**Destination**: any editor layer, or **Reference image**. No layer selection is
required. Capture and inspect the still, then click **Import into layer** or
**Add as reference**. Layer destinations offer **Keep refreshing this layer**
and the shared refresh interval. This starts timed updates after import; leaving
it off imports a fixed layer still. Selection and layer geometry are unchanged.

Reference images never offer auto-refresh: they are fixed screen guides embedded
in the project, with the usual size/aspect warnings. Cancel/close discards a pending
dialog capture; another request waits for completion or the 20-second timeout.
No capture action changes device settings or routing. See the
[input preview contract](../../interfaces/midra-input-preview.md).

Opening a project or restoring recovery now reads, validates and decodes embedded
references in a worker. The current editor stays intact until the whole project
is ready. Manual loading has a Cancel button; errors keep the old project.
Recovery autosave pauses while loading, including on exit.


In the opt-in **Layer properties → Source** section, **Editor input** offers Inputs 1–8 separately
from Midra screen/layer destinations. Use **New editor input** for additional
local identities, then **Apply source changes**. Layers sharing an editor input
share its source format and input crop. Assignments are saved in projects and
undoable; they do not change physical input routing.


## Aquilon / LivePremier

Aquilon is a first-class supported adapter for standalone AQL Cmax firmware 6.2.x,
verified on the 6.2.79 emulator; physical-output acceptance remains outstanding.
Connect using the same Device connection window.
The device identifies its family automatically. Screens use `S1`, `S2`, …;
auxiliaries use `A1`, `A2`, … and stay distinct in saved mappings. Only enabled
screens and allocated numbered layers appear; empty/unrouted layers remain available.

Load, selection/folder scopes, mapping review, Preview/Program sends and input
previews use the same controls as Midra. The Program preference still applies.
The adapter resolves each screen's current transition group; an active transition
or a changed bank/source before Send requires a refresh, without writing.

Centered live-input framing supports layer crop and zoom. Fractional geometry is
retained in projects; sends round device sizes/centers to whole pixels for WebRCS
compatibility and check the visible-box error within one pixel.
Masks, non-centered fit modes and shared input aspect/crop corrections are
reported explicitly when their visible bounds cannot be imported. Shared input
correction controls remain managed in WebRCS for this adapter. Ordinary sends
never route inputs, TAKE, or change shared input processing.

Aquilon reads only the required properties through AWJ TCP 10606, in batches,
including a consistency check. The tested emulator now downloads about 197 KB
per refresh instead of 124 MB, with direct reads around 0.30 seconds. Preflight
and verification use the same targeted reader. If the AWJ port is disabled or
blocked, the connection reports an error instead of falling back to the full
store. Connect, load and send remain background operations. Input snapshots retain
pooled, throttled, opt-in behavior, including inputs above 16. Older firmware,
linked units and other Aquilon models remain unverified. See the
[adapter contract](../../interfaces/device-adapters.md) for tested scope and limits.

Saved device input corrections are optional [inactive proposals](advanced-exchange.md#saved-input-corrections); opening a file never enables or sends them.
