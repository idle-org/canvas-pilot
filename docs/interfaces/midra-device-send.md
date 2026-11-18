# Midra reviewed layer exchange

The Rust adapter uses the configured IP and optional port for this endpoint
(default 80), including emulator addresses such as `192.168.10.146:3000`.

This is a transient device protocol, not a project/save schema. Reference collections in project v10 do not change device layer mapping. Compatibility exercised: Eikos 4K firmware 3.3.10, installed WebRCS
`app.e42fce4879f318b28cde.js`. Other firmware needs verification.

## Addressing and fields

HTTP GET `/api/tpp/v1/system` and `/api/stores/device` supply identity and snapshots.
A WebSocket at `ws://DEVICE_IP/` accepts JSON text:

```json
{"channel":"DEVICE","data":{"path":["device","screenList","items","2","presetList","items","UP","liveLayerList","items","1","position","pp","posH"],"value":162}}
```

Normal geometry fields under the mapped live layer are:

- `size.pp.sizeH`, `size.pp.sizeV`: integer container pixels, 1–65535.
- `position.pp.posH`, `position.pp.posV`: integer center coordinates, −67268–67268.
- `crop.pp.left`, `right`, `top`, `bottom`: normalized integers, 0–65535.
- `mask.pp.left`, `right`, `top`, `bottom`: normalized container-edge blanking, 0–65535.
- `crop.pp.aspectOverride`: `CENTERED` normally; explicit fit also supports `1_1`, `FULLSCREEN`, `CROPPED`.

At `AT_DOWN`, Preview is UP and Program DOWN; `AT_UP` reverses those roles.
The selected configuration captures one physical bank. Send first refreshes the device and rejects changed bank roles, conversion inputs or destination values before writing. Commands do not retarget halfway through. No TAKE or source routing,
effect, opacity or output setting is sent. Shared input aspect/crop fields
are omitted unless explicitly opted into and edited as described below.

## Conversion

When the destination reports positive content dimensions, the editor native
source dimensions must equal `status.pp.contentWidth/contentHeight`. Missing or
zero dimensions (for example, an unassigned input) use the editor's native
dimensions for crop conversion and do not block sending. Reported dimensions
describe the incoming content used for this layer, not an instruction to change
input hardware configuration.

Combine each local input crop edge with its advanced layer crop edge, in native
source pixels. Quantize each combined edge to `round(edge / native_axis * 65535)`.
Reject exhausted retained regions, invalid geometry and
source-resolution mismatch. Input correction is folded into the layer crop,
so it affects only the mapped destination.

For normal crop/zoom (no explicit fit), **Output-sized**: round the requested visible width/height to container pixels.
For **Source aspect**, compute `k = max(visible_width/native_width,
visible_height/native_height)` and round `native * k` as the container size. The
post-crop retained source fits inside this container with Centered aspect. Both
modes center the container at the rounded requested visible center.

Reconstruct the retained size from quantized crop fractions, then use
`min(container_width/retained_width, container_height/retained_height)` to derive
visible bounds. Reject size errors above one output pixel. Center rounding has
at most half a pixel of error. Stored editor coordinates retain full precision;
review values use one decimal and transmitted geometry uses whole pixels.

For advanced explicit fit, the editor rectangle is the device container. Both
sizing strategies use its rounded dimensions and center. Crop still uses the
combined normalized conversion above. Apply crop first; scale retained content by
1 (1:1), the minimum axis ratio (Centered), the maximum ratio (Cropped), or both
independent ratios (Fullscreen); center and clip to the container. Empty margins
are transparent. No visible-size reconstruction check is applied to explicit
modes because image bounds can legitimately differ from container bounds.

All rounding uses nearest integer, halves away from zero. The editor retains
full precision. Firmware crop quantization, half-pixel placement and raster edge
rules remain unverified; see the [engineering follow-up](../canvas-pilot/midra-fit-precision.md).

## Import and round trip

All four explicit modes with valid content dimensions import
container geometry, explicit fit, raw crop and normalized masking. Imported sources are independent
local sources with device content dimensions, zero input correction, and combined
cropping stored as layer crop. Original input/layer crop separation cannot be
recovered. Imports are atomic and undoable, including new layers/sources. This
also means Centered device loads open the advanced-fit checkbox automatically.
Inherited/unknown modes remain container-only when crop is zero, with masking preserved;
nonzero crop is rejected instead of guessing the effective inherited mode.
The Python snapshot-to-SVG bridge remains container-only under its existing
[contract](midra-device-import.md).

## Execution and failure semantics

Only an explicit Send click creates a worker. Read/refresh, editing, project load,
import, undo and redo never write. Requests run off the UI thread with bounded
socket operations. After preflight, changed fields are written sequentially once. Crop and mask decreases precede increases
to avoid temporarily exhausting an axis while moving either window.
Readback checks every changed field. Valid plans remain sendable even when stored geometry matches: sending still applies staged values. Failure reports
partial/uncertain outcomes without blind retries or automatic rollback.

Program selection is explicit. The application defaults to Preview and hides Program unless the persisted `allow_sending_to_program` preference is enabled. Turning it off resets the exporter to Preview, including quick Apply. Layer screen/destination links are saved in project version 5. Plans and raw snapshots are not saved in projects or preferences. The device address is saved in [application settings](canvas-pilot-settings.md), never in projects. Refresh and screen changes preserve saved links. Physical display
validation remains separate from the successful device-field and import round trips.

The status-bar and Export-menu **Midra mapping & transfer…** actions only open export configuration, even when current layers have valid saved links. They never start a write. The user chooses a transfer scope, mappings, target and sizing, then explicitly clicks **Send mapped layers** for the displayed screen. Empty/unlinked rows are skipped. The load button uses all saved links in the scope, including links to other screens. Busy/offline actions are disabled. **Send all linked layers** remains a separate direct-send action using saved links. No persistent schema changes are involved.

Snapshot layer lists include only allocated live-layer entries, independently of
input assignment (`NONE`, empty or absent) and OFF state. The active allocation comes from
`device/preconfig/status/stateList/items/CURRENT/screenList/items/{screen}/pp/layerCount`.
Only numeric live-layer IDs 1 through that count are listed or accepted as send
destinations, even though the firmware stores eight slots. The staged NEW
configuration, physical input count, background and foreground planes do not
determine this list. Missing/invalid allocation metadata produces a connection
error rather than exposing all slots. Missing layer IDs are not synthesized. Input routing is informational and is never changed by geometry
transfers. A Centered layer without usable content dimensions and with zero crop
imports container geometry and mask without inferred source framing; nonzero crops
still require valid dimensions for visible-bound reconstruction.

The direct send button uses the shared transfer scope. Its default **All layers** includes all linked document layers, even unselected, hidden or locked ones. **Only selected layers** uses exactly the current selection (an empty selection does not mean All); **Folder: …** uses all members of the named folder, independently of selection/visibility/locks. Unlinked layers are skipped. Plans are grouped by saved screen ID and all groups are validated before starting. Screen groups send sequentially with readback; on failure later groups are not sent, and the result reports the failed screen and completed-group count. Links never auto-send and are not tied to an IP address.

## Preset apply pulse

After the geometry writes, send two DEVICE WebSocket messages at
`device/preset/control/pp/xUpdate`: `false`, then `true`, matching WebRCS's
`DEVICE_PRESET_CONTROL_SET_ONE` action sequence. This commits staged preset edits
when `enableAutoUpdate` is false. Do not change `enableAutoUpdate` or issue TAKE.
The pulse is global and may apply other pending preset edits; it is not scoped to
an individual screen/layer. Each screen send currently ends with this pulse.
Always pulse on an explicit valid send, even if the staged geometry already
matches and the change list is empty. A pulse-write failure is reported separately
from geometry-write failure. Geometry readback follows the pulse; a readable store
alone does not prove the physical video output updated. The trigger itself is not
included in geometry comparisons, because it is a command rather than geometry.

**Send all linked layers** sends immediately using saved links and the current target and sizing settings, without opening the export window. The button shows progress and its tooltip reports completion. Invalid links or send failures open the export window with the error.

**Load linked layers**, beside the connection status and in Export, reads fresh
Midra Preview geometry and crop framing into linked editor layers in the shared
transfer scope. The default **All layers** ignores canvas selection; **Only selected
layers** and **Folder: …** explicitly restrict the operation. The load button inside
**Midra mapping & transfer…** uses that same scope and the saved links. Both actions are read-only
on the device and always load Preview (independent of the send target). They do
not recall device memories or presets. Loading is one atomic Undo step; links
remain saved. Missing screens/layers, mismatched output resolutions, unsupported
geometry, and hidden or locked editor destinations reject the complete import
without changing the document. Unlock/show destinations before loading. Results
appear in the transfer window or the load button's status tooltip. No project or
settings schema changes are involved.

Closing a device import review with chosen destinations offers Apply/Discard.
Apply imports into the local editor atomically; it never writes to the device.
Closing send configuration does not send. Local property edits that apply
immediately still require an explicit device Send action.

## Shared transfer scopes and folder layouts

The desktop keeps a session-only scope: All (default), Selected, or one folder name.
Scope controls both direct linked send/load and the destination candidates in manual
Midra import/export review. All does not depend on selection; Selected with no selection
and Folder with no members are empty scopes, never implicit All. Scope changes do not
edit viewport selection or saved links. In Folder mode, clicking a folder name changes
the scope. Folder rename follows the renamed scope; a missing folder requires choosing
another scope. The scope is omitted from project/settings/export representations.

Direct send/load captures stable layer IDs and their saved `(screen, layer)` links
when clicked. Switching scope while a read is pending cannot redirect that import.
Direct sends validate destination uniqueness within the chosen scope, across each
screen. Duplicate destinations within or across folders are valid persistent links.
Mapping warnings list other linked editor layers and folders, without preventing
link edits. Conflicting sends are disabled before any writes and list every
conflicting screen/layer and editor layer in the chosen scope. A conflict-free
folder/selection or remapped/skipped review rows can send; out-of-scope links do
not block that send. Loading the same device layer into multiple
local linked copies is allowed; destination editor IDs must still be unique, visible,
and unlocked. Existing geometry validation and atomic import guarantees are unchanged.

The send review lists all scoped editor layers on the currently browsed screen.
Saved links on that screen populate editable destinations; empty rows are skipped.
Browsing another screen does not rewrite existing links. An explicit destination
change saves the new link; a scope change never removes out-of-scope links.
Manual device import suggests unique saved links within scope, while retaining manual
row remapping until scope/screen changes or refresh. Ambiguous/unlinked rows start
skipped. Creating layers is explicit and available only in All/Folder scopes; new layers
inherit the scope's folder (root in All). A successful manual import saves reviewed device links, geometry,
source/crop data and new-layer membership in one undoable transaction. Invalid mapping,
link or geometry rejects the entire transaction. This uses existing layer fields (unchanged in project v10);
there is no schema/version change. Source routing and device memories are not imported.

Layer properties → **Mapping** edits the existing per-layer `midra_link.screen`
and `midra_link.layer` fields locally. **Unmap** saves `midra_link: null`, so
linked sends and loads skip the layer. These edits are undoable and never issue
device commands. Connected selectors use discovered screens/layers; offline
editing accepts positive numeric IDs. Project schema and version are unchanged.

## Historical opt-in advanced input edits (through v0.9.55)

Removed in v0.9.56: these UI/staging controls and their layer-plan extension no
longer exist. Existing saved proposals remain inert and round-trip as before.
The independent shared-input editor below is the only device-input editing path.

The Source tab's device controls map one-for-one to the installed WebRCS
Advanced Input settings. They are separate from the editor's local source model
and its existing combined layer-crop conversion above. Field names, enum values,
ranges and input-crop update pulse were checked against the connected Eikos
snapshot and `app.e42fce4879f318b28cde.js`; this change did not write to hardware.

Read the linked Preview layer's `source.pp.input`, then the input's
`status.pp.plug`. The write root is
`device/inputList/items/INPUT_N/plugList/items/P/settings/`:

| Suffix | Encoding / units |
| --- | --- |
| `aspect/pp/signal` | `NATIVE`, `5_4`, `4_3`, `16_10`, `15_9`, `16_9`, `21_9`, `256_135`, `64_27` |
| `aspect/pp/transformTo` | `SIGNAL`, `5_4`, `4_3`, `16_10`, `15_9`, `16_9`, `21_9`, `CUSTOM` |
| `aspect/pp/customRatio` | Integer ratio × 1000, 250–10000; UI displays 0.250–10.000 |
| `aspect/pp/layerFill` | `LAYER_SETTING`, `1_1`, `CENTERED`, `FULLSCREEN`, `CROPPED` |
| `cropping/control/pp/predefined` | `NONE`, `LETTERBOX_1_78`, `LETTERBOX_1_85`, `LETTERBOX_2_35`, `PILLARBOX_1_33` |
| `cropping/control/pp/left`, `right`, `top`, `bottom` | Whole input pixels, 0–8191 |

Input crop is processed before layer crop/zoom and does not set layer-container
resolution. These settings are shared across consumers of the physical input;
they are not bank-local. A Preview geometry send with opted-in input edits can
therefore affect a Program layer using the same input. Layer exports continue
to use their own explicit aspect override; input layer-fill edits may affect
other layers configured to inherit input settings.

The checkbox defaults off; merely enabling it or opening the tab stages nothing.
Only edited fields are retained; reverting to the captured value removes the
edit. Plans include a staged field only when an explicitly mapped destination
in the chosen physical bank currently uses that input and plug. A Program
layer with different routing does not receive the Preview input's edits.
Duplicate mappings to one input add each field once per screen plan. Raw input
edits are listed in Exact device changes and apply to both reviewed and direct
linked sends. They never route sources or write unrelated input defaults.

Input field writes precede layer writes. After writes, each changed input crop
gets its own `cropping/control/pp/xUpdate` false→true pulse, then the normal
preset update pulse runs. Readback checks edited values, excluding intermediate
pulse values. Existing partial-failure reporting applies; no automatic rollback
or resend. Refresh/load reads resulting device content geometry; the input
controls do not predict device resampling or update the local source model.

This is session-only state, not part of project JSON, SVG, preferences or
recovery. Uncheck, disconnect, address change and project load discard staged
input edits and reset opt-in. Existing files migrate without changes and never
opt into shared input writes. No persistent format/version change is introduced.

### Fractional device content reports

Native-source compatibility compares the editor's integer resolution with the
same rounded content dimensions used during snapshot import (nearest pixel,
positive halves upward, original values bounded to 1–16384). Fractional reports
such as 1919.7 × 1080.2 therefore match 1920 × 1080 instead of blocking a send.
Actual resolution mismatches still fail. Snapshot feedback reports the rounding;
raw device state is not rewritten. Existing whole-pixel size/position export and
crop quantization remain unchanged.

## Layer masking

Midra WebRCS's Mask controls describe an erasing effect without resizing content.
The inspected device WebRCS application (read-only, firmware UI bundle
`app.e42fce4879f318b28cde.js`) exposes integer edge attributes 0–65535 and converts
pixel fields against the live layer's `sizeH`/`sizeV`, with the opposite edge
subtracted from the maximum. The [manufacturer manual, §15.2.6](https://www.novelty.fr/wp-content/uploads/downloaded/downloads/materiel_manuels/5008-4642-user-manual-midra-4k.pdf)
also describes per-layer masking independently of source resizing.

The editor stores fractions of its layer rectangle. For explicit fit and
Output-sized normal crop, convert the reveal rectangle to the rounded device
container, then round each fraction ×65535. Source-aspect normal crop can use a
larger container: include its centered margins when converting the reveal
rectangle. A completely zero editor mask sends four zeros. Opposite quantized
edges are capped at a sum of 65535; fully blanked axes are legal. Import divides
wire values by 65535 and validates the sums, separately from source/crop/fit.
All values participate in preview review, stale-state checks and readback.
Sending an unmasked editor layer explicitly clears an existing destination mask.

The canvas, saved project and export artwork implement the same clipping model.
Automated tests cover gesture cancellation, pixel preservation, migration and
wire round trips for every fit and both sizing strategies. Physical raster-edge
accuracy remains part of the engineering follow-up; no hardware write was used
to establish it. Aquilon rectangular masking is described separately in the
[adapter contract](device-adapters.md#mask-appearance).

### Source-relative editor masks

Both mask references convert to output-space reveal bounds before device-container
quantization. Source bounds follow usable input after local input cropping and before
layer crop/fit; reveal bounds are clipped to the device container. Masking does not
change routing, input settings or crop values. The device stores blanking only, so
a later device load returns a layer-relative mask; retain the project to preserve
source-relative editing intent. Aquilon uses the same editor reveal bounds with
its own container-relative conversion.

## Separate load and send mappings

The underlying layout stores `load_mapping` separately from `send_mapping`.
Linked sends flatten the scoped send destinations, check destination collisions,
and group by screen for adapter preparation. Loading uses only the load source.
One local layer can be planned for several distinct device layers; both Midra and
Aquilon still reject overlapping destination writes. This is an internal capability:
current controls keep both directions synchronized with one destination, and the
current connection is the only accepted connection scope. No UI or transfer defaults
change. Persistence and migrations are specified in the
[project contract](canvas-pilot.md#directional-device-mapping-version-19).

## Independent shared-input editor (v0.9.55)

`device_inputs::InputDraft` is a transient adapter-specific input/active-plug draft,
not a persistent project field. The common window edits native input preprocessing
without changing editor sources, layer geometry, routing or preset selection.
Only edited fields are sent after fresh signal/plug/settings checks; readback is
verified. There is no automatic retry, rollback or preset-apply pulse. Partial
failures are reported and require refresh. This does not migrate existing source
geometry or saved layer-bound input proposals; only their saved data and local geometry compatibility remain.

Aquilon 6.2 uses `IN_N`, `KEEP_INPUT_RATIO` and unsigned 0–255 color controls
(neutral 128), with hue 90–270 (neutral 180). Midra uses `INPUT_N`, `SIGNAL`, signed
−128–127 color values and hue −90–90. Crop bounds use signal `aspectWidth` and
`aspectHeight` when published, rather than potentially cropped image dimensions.
Aquilon horizontal edges allow 0–32767, vertical edges 0–8191; Midra uses 0–8191.
At least one pixel must remain. Crop zoom rounds once to whole native pixels.

For Midra, crop apply pulses `settings/cropping/control/pp/xUpdate`. Aquilon pulses
`settings/pp/xUpdate`; it must not receive the Midra path. Opposing crop edges are
released before larger crops are written. Aquilon selected-input reads request
only an explicit set of fields, bound response/time budgets and double-check their
values and active plug before publishing the draft. Normal layout reads remain
unchanged. Unsupported or absent fields are not invented.

Input-window capture uses the existing bounded snapshot reader and shared throttle.
It never updates layer preview images or changes input crop to obtain a raw image.
The input crop editor uses the existing sizing pattern behind available pixels.
For Aquilon, fresh reads bracket capture; stable crop/aspect/plug state and a matching
thumbnail frame ratio are required for compositing. The image is clipped to the
captured status-crop region using corresponding full-frame UV coordinates; it is
never stretched to a newly edited crop. Unknown pixels retain the pattern. Other
capture geometries remain separate previews. No raw-capture contract is claimed. Aquilon Cmax 6.2.79 emulator testing showed
blanked cropped edges; Pulse 4K 3.3.7 served an unchanged thumbnail. Input drafts
and captures are not serialized, and existing project/exchange versions are unchanged.

Manual diagnostic: `cargo run -p canvas-pilot --example check_device_inputs -- HOST`
changes Input 1 crop and restores the original values. Run only against an expressly
authorized test device. An optional second argument saves before/after PNGs in an
existing temporary directory. It is never part of CI or physical-output acceptance.

### Removal of legacy controls (v0.9.56)

Layer send plans no longer accept staged input-setting extensions. Shared-input
sends remain separate and explicit. The local source-model permission is now an
independent editor-only checkbox. Existing serialized `input_proposals` are retained
and filtered by the same layer/link identity rules, but cannot be activated by the
removed UI. No project, recovery or exchange schema/version change is required.
