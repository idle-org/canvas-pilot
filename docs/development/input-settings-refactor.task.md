# Shared input settings and Midra preprocessing

Status: input-owned window and isolated sends implemented; automated checks and
emulator crop tests passed. Physical-output and persistence-extension acceptance
remain explicitly listed below.
Aquilon is the primary development target; Midra emulator testing is secondary.
Checkpoint: annotated local tag `pre-input-settings-refactor`, commit `b711239`.
The tag includes the non-cumulative crop-preset change and has not been pushed.

## Intended behavior

A dedicated Inputs window owns shared input preprocessing. It lists every
available device input, independently of whether an editor layer uses it, and
shows the selected input/active plug, signal information and known consumers.
Layer properties link to the relevant input; layer crop, fit, position and masks
remain layer-owned. A local editor source ID is never interpreted as a physical
input number. Inputs on different connections require different identities.

Edits remain local drafts until an explicit reviewed Send input settings action.
Changing a selection, opening a project or capturing an image sends no settings.
The review explains that an input change affects every device consumer, including
ones outside this project and potentially Program. No routing or TAKE is implied.

Reuse the canvas interaction conventions for crop edges, whole-window movement,
pivot choice, crop zoom and view zoom, with numeric edges, reset and Undo/Escape.
Crop zoom changes the retained input region; view zoom changes only the viewport.
Do not invent an additional Midra input zoom stage when native crop/aspect fields
already describe the operation. Keep Content ratio / Transform to terminology.

## Evidence checked on 2026-11-18

Official sources, downloaded from the Pulse 4K product page:

- [Midra 4K user manual, 2026-05-21](https://dwn01.analogway.com/Site+Internet/Series/Midra+4k/Common/User+Manual/AW_Midra4K_UserManual_20260521.pdf), sections 10.2.3–10.2.7.
- [AWJ programmer guide v3.2](https://dwn01.analogway.com/Site+Internet/Series/Midra+4k/Common/AWJ+Protocol/midra_4k_awj_programmers_guide_v3.2.pdf), introduction and section 7.
- [REST API guide v3.0.13](https://dwn01.analogway.com/Site+Internet/Series/Midra+4k/Common/Rest+API+Programmer%27s+Guide/midra4k_rest_api_programmers_guide_v3_0_13.pdf), section 8.

The manual confirms input crop/aspect replaces the input content for all layers
using it. Available image controls include Content ratio, Transform to, Layer fill,
input crop/Finder, global and RGB brightness/contrast, saturation, hue, sharpness,
LUT correction, and chroma/luma/Cut & Fill keying. Signal settings additionally
include color type, dynamic range, luminance and HDCP. Controls must be grouped,
capability-gated and mapped to observed native fields/ranges; this inventory is
not permission to guess commands or to claim every firmware supports them.
LUT allocation and Cut & Fill have additional resource/input dependencies.

The AWJ guide explicitly does not enumerate the entire device object model.
Current code only has established paths/ranges for aspect and crop controls.
Inspect the connected device and WebRCS controls before adding the other writes.

Both API guides document the regular input-thumbnail endpoint, but do not specify
its position before/after crop, aspect correction, color correction or keying.
They describe padded thumbnails; existing measured captures have different sizes
from the guide's example. Do not infer native dimensions or preprocessing stage
from PNG size. No separate raw/native capture endpoint was established.

Aquilon Cmax 6.2.79 now runs at the authorized emulator address on port 3000.
Input 1 asymmetric crop writes were verified and restored, with editable screen
state unchanged. Captures show blackened cropped edges, not recoverable raw pixels.
Pulse 4K 3.3.7 also passed crop write/restoration checks but served unchanged images.
Neither observation establishes physical-device capture pipeline behavior.

The current window uses session-only, per-input drafts. Existing project source
geometry and inert input proposals remain compatible; they have not been migrated
into a new persistent input schema. Legacy layer-send proposal controls and staging were removed in v0.9.56. New input-window edits never add local layer crop/zoom.

Aquilon definitions were checked against its served WebRCS app and native state:
unsigned color values, KEEP_INPUT_RATIO, larger horizontal crop range, aspect
bounds and settings/pp/xUpdate differ from Midra. Selected-input reads stay bounded
and targeted; ordinary layout fetches do not acquire these additional controls.

## Refactor boundaries

- Completed in v0.9.57: removed the legacy local source/pre-crop UI and model,
  the combined input-plus-layer send conversion, and `InputProposal` persistence.
  The approved schema break is version 20. Shared input drafts remain session-only.
- Native Open project uses reported best-effort recovery and opens repaired data
  as an unsaved copy. Strict exchange imports do not silently repair snapshots.
- Processed source dimensions and sharing remain necessary for layer geometry;
  device imports detach sources where necessary. Physical input edits never become
  an additional local crop.
- Device input Send must operate without sending unrelated layer geometry or
  pulsing preset apply. Validate active plug, relevant signal/settings and edited
  values against fresh reads; pulse input crop apply only when needed and verify
  readback. Partial failures must remain visible, without automatic rollback.
- After an input change, refresh affected layer content dimensions before sending
  layer geometry. Do not combine stale conversion with newly applied input settings.
- Distinguish raw signal, preprocessed content and layer coordinates. Legacy local
  crop needs an explicit compatibility path; never silently reinterpret it as a
  physical input write or double-apply real input preprocessing.

Aquilon `source_info` still refuses active shared input crop/aspect correction
for framing transfer. The shared input editor and schema cleanup do not establish
that conversion; keep it pending until processed dimensions/coordinates are verified.

## Capture decision and acceptance

- [x] Inspect implemented input paths, enums, ranges and update actions in native WebRCS; verify Aquilon crop writes on the emulator.
- [ ] Identify whether a native/raw capture mode exists in WebRCS or the API.
- [ ] Establish snapshot stage with a marked edge/grid image, asymmetric crop and
  aspect change; compare before/after images and reported dimensions. A simulator
  serving static thumbnails cannot establish physical processing order.
- [x] Display processed captures, with sizing artwork filling unavailable pixels. Only
  overlay editable crop geometry with a known coordinate transform; never invent
  missing edge pixels or silently reset device crop to obtain an image.
- [x] Build the input-owned window, isolated drafts, explicit send review and
  native crop/view interaction, including capture cancellation/stale-result guards.
- [ ] Prove two layers sharing one device input receive one input change, retain
  independent layer framing and do not double-crop. Include unrepresented device
  consumers, input/plug changes, signal loss and stale-state rejection.
- [ ] Exercise every implemented field, integer quantization, crop release-before-
  increase ordering, update pulses, no-op sends and partial failures with mocks
  and the emulator; preserve existing Midra/Aquilon layer-transfer regression tests.
- [ ] Test old-project migration, Undo/Redo, recovery and every supported snapshot
  format round trip. Saved drafts stay inactive; plain geometry exports state losses.
- [ ] Update contracts, manual, feature matrices, screenshots and changelog; run
  required Rust checks, coverage and book validation before declaring completion.

## Remaining acceptance boundaries

Raw-capture recovery is replaced by the requested sizing-pattern fallback. The
Aquilon composite uses known status-crop bounds and matching source/thumbnail frame
ratios; other cases keep the unmodified capture separate. Capture checks bracket
image acquisition and reject changed geometry. Physical capture-stage/latency tests
remain useful; emulator output is not physical-device acceptance.

The checks above deliberately keep future persistent input-owned proposals and
exhaustive per-field/physical-output acceptance open. This implementation introduces
no persistence migration: existing project, recovery and snapshot formats retain
their tested source geometry and inactive proposal behavior. Current shared-window
drafts are session-only. Color/keying/LUT output still requires visual acceptance
on appropriate allocated hardware; crop write/readback/restoration was exercised
on both emulators, with Aquilon as the continuing target.
