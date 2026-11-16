# Aquilon support and acceptance

[Canvas Pilot plan](plan.md) · [Device workflows](guide/device.md) · [Completed implementation](aquilon_support.task.md)

Implemented and tested 2026-11-15. **Aquilon is a first-class supported adapter**
for standalone AQL Cmax firmware 6.2.x; the tested emulator is **6.2.79** at the
configured custom port. See the [current adapter contract](../interfaces/device-adapters.md).

The supported profile is complete for its documented scope. Physical-output
acceptance remains outstanding; other models, linked units and unsupported
framing modes require separate validation or implementation.

## Current acceptance

- Identity, bounded targeted AWJ reads, screen/layer allocations and
  Preview/Program transition-group resolution work on the emulator.
- Twelve apply/readback/import cases pass: Preview and Program × Output-sized and
  Source aspect × three fractional/portrait/asymmetric crop cases (zoom 1, 2 and 4).
  Original routing, geometry and crop are restored and read back afterward.
- A source-aspect diagnostic exposed IEEE-754 single-precision geometry storage:
  447.1111111111111 reads back as 447.1111145019531. A separate visual check then found WebRCS rejects fractional geometry with
  `oldAnchorRect are not integers`. The final adapter rounds only transmitted
  sizes/centers to whole pixels, preserves editor precision and checks the
  resulting visible box within one pixel. Readback remains strict. A subsequent
  browser check rendered the rounded Preview layer correctly without WebRCS errors.
- Inputs 1 and 17 return 256 × 144 PNG snapshots. Discovery, pooled refresh and
  preselection use Aquilon native IDs rather than Midra input limits.
- Project v13 saves explicit device families and native screen/auxiliary IDs;
  versions 1–12 and legacy recovery migrate without losing geometry or links.
- Regression coverage includes independent screen/auxiliary banks, disabled slots,
  unmapped inputs, malformed state, family mismatches, stale plans and exact
  WebSocket/apply behavior. Auxiliaries are tested with sanitized synthetic state;
  this emulator's baseline only allocates S1.
- Workspace tests, Clippy, formatting and warning-free Rust documentation pass;
  all 62 Python checks pass. Full application line coverage is 90.69%.
- Linked units, older firmware, other models, physical output and advanced shared
  input correction are not claimed as verified. Masks and non-centered fit modes
  remain explicit import limitations, not silently approximated geometry.

The explicit diagnostic is `cargo run --locked -p canvas-pilot --example
check_aquilon_send -- --apply-and-restore IP:PORT`. It temporarily routes S1 layer 1
to LIVE_1 in both banks; production sends never route. A separate
`--visual-check-and-restore` mode holds a known Preview layout for 60 seconds for
WebRCS inspection, then restores it. These diagnostics are never automated CI tests.

## Targeted read performance follow-up

The initial full-store reader has been replaced by batched property reads on
AWJ TCP 10606. The configured HTTP port still handles identity, captures and
WebSocket writes. The [adapter contract](../interfaces/device-adapters.md) records
framing, bounds, endpoint requirements and the consistency check.

Measured on the same emulator: 197180 AWJ response bytes + 124 HTTP identity bytes,
184978 request bytes, and direct reads of 0.314 / 0.302 / 0.297 seconds. The earlier
store was 124077157 bytes, so downloaded payload fell by 99.84%. A counting proxy
confirmed no `/api/stores/device` request. All twelve Preview/Program send cases
passed again with targeted preflight/readback, and the current baseline was restored.

The current manufacturer AWJ V6.2 and REST v6.0 PDFs were accessible using literal
`+` URL path segments; the percent-encoded links returned 403. AWJ property and
collection reads were verified directly. The historical full-store investigation
below describes the earlier implementation, not the current connection path.

## Historical protocol investigation

The following records the investigation before implementation; its future-tense
steps describe the original plan, not the current feature status.
This review compared the current Rust adapter with upstream protocol references;
The initial review did not connect to an Aquilon. A subsequent read-only emulator
probe on 2026-11-15 confirmed the transport details below; no settings were changed.

## Emulator response-size diagnosis

Read-only GETs at the test emulator confirmed `/api/tpp/v1/system`
returns `type: "AQL Cmax"`, firmware **6.2.79**, and `/api/stores/device` returns
**124,076,981 bytes** (about 118 MiB). This exceeds the current Midra 32 MiB state
limit. Raising that limit alone would still fail at the unsupported-family check.
The connection now validates identity before requesting state and explains that
Aquilon needs its separate adapter. The large private response is not committed.
Future Aquilon transport work must account for this measured state size, ideally
with targeted reads rather than repeated full-store downloads. These reads do not
validate geometry, crop, bank resolution, capture or writes.

## Finding

Aquilon (LivePremier) can reuse our HTTP state-read / WebSocket write approach,
but needs a separate, firmware-aware adapter. Adding its name to the Midra model
list would leave bank selection, geometry paths and the final apply command wrong.
Start with firmware 4.x and later as a candidate profile; validate the actual
firmware/state before extending that claim to later releases. Older firmware
needs a separate profile, not guessed aliases.

The upstream [Companion integration](https://github.com/bitfocus/companion-module-analogway-awj)
explicitly separates Midra, LivePremier and LivePremier 4. Its name mentions AWJ,
but its implementation uses the web control transport. Analog Way's documented
AWJ TCP service is another transport, not the same socket or message framing as
our WebSocket connection. We do not need to introduce AWJ TCP for the first adapter.

## Evidence and differences

The source observations below are pinned to Companion revision
`4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2`. They are implementation evidence,
not a substitute for Aquilon emulator/hardware acceptance.

| Area | Current Midra adapter | Aquilon evidence / required work |
| --- | --- | --- |
| Connection | IP plus optional port; HTTP state read and root WebSocket | Upstream uses `/auth/status`, `/api/stores/device` and root WebSocket, with authentication handling. Probe these separately from our `/api/tpp/v1/system` identity read. |
| Identity | Four Midra type strings | Upstream detects `NLC` device identifiers and firmware from system state, including linked-device roots. |
| Preset banks | `UP` / `DOWN` | Aquilon uses `A` / `B`; resolve Preview/Program from transition state, independently for each screen/group. |
| Screens | Numeric screen IDs | Screen and auxiliary IDs include `S1` and `A1`. Firmware 4 separates `screenList` and `auxiliaryList`. |
| Allocation | CURRENT preconfiguration layer count | Upstream reads each screen's `status/pp/layerCount`. Discover actual allocations and editable layers. |
| Size | Layer `size/pp/sizeH` and `sizeV` | Shared Aquilon constants put size under `position/pp`, beside position. |
| Coordinates | Center-based `posH` / `posV` | Upstream subtracts half-width/height to obtain top-left coordinates, matching our editor convention. |
| Crop | Layer `crop/pp` and `mask/pp` | Upstream constants use `cropping/classic/pp` and `cropping/mask/pp`. Units, source correction and fit semantics still need verification. |
| Apply pulse | `device/preset/control/pp/xUpdate` | Older Aquilon uses `device/screenGroupList/control/pp/xUpdate`; firmware 4 profile uses `device/screenAuxGroupList/control/pp/xUpdate`. Upstream pulses false then true. |

Pinned implementation references:

- [Connection, identity and authentication](https://github.com/bitfocus/companion-module-analogway-awj/blob/4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2/src/connection.ts).
- [Common geometry/crop paths](https://github.com/bitfocus/companion-module-analogway-awj/blob/4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2/src/awjdevice/constants.ts)
  and [coordinate conversion](https://github.com/bitfocus/companion-module-analogway-awj/blob/4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2/src/awjdevice/actions.ts).
- [Firmware 4 constants](https://github.com/bitfocus/companion-module-analogway-awj/blob/4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2/src/livepremier4/constants.ts),
  [allocation](https://github.com/bitfocus/companion-module-analogway-awj/blob/4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2/src/livepremier4/choices.ts),
  [bank resolution](https://github.com/bitfocus/companion-module-analogway-awj/blob/4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2/src/livepremier4/subscriptions.ts)
  and [apply pulse](https://github.com/bitfocus/companion-module-analogway-awj/blob/4df8677f912ce95b6e7e5c93f2fd51aa0ae33ff2/src/index.ts).

Analog Way's [Aquilon downloads](https://www.analogway.com/products/aquilon-rs1)
list separate AWJ guides for firmware 2.2–3.x and 4.x onwards, plus a REST guide.
The current links resolve to AWJ guide V6.2 and REST guide v6.0, but their download
server returned HTTP 403 during this investigation; their full contents were not
verified. Use those guides alongside emulator state when implementing crop behavior.
No upstream code or new dependency was copied into the application.

## Implementation sequence

1. **Capture an Aquilon baseline.** Obtain emulator IP:port and firmware, read
   identity/state, record screen and auxiliary allocation, and inspect authentication.
   Keep raw configuration private; derive minimal sanitized regression fixtures.
2. **Separate adapters from transport.** Share address/history, bounded reads,
   timeouts, worker scheduling and WebSocket framing. Give each profile its own
   identity detection, screen enumeration, bank resolution, geometry paths, crop
   conversion and apply pulse. Make Program bank explicit rather than assuming
   the opposite of UP is DOWN.
3. **Import geometry first.** Test differently sized canvases, multiple screens,
   inactive slots, unassigned inputs, A/B swaps and transitions. Preserve native
   device IDs. Do not substitute zeros or default dimensions for missing fields.
4. **Generalize saved links.** `MidraLink::validate` currently accepts only positive
   decimal strings. Introduce family-aware destination identity with an explicit
   project migration and interface examples/tests; preserve existing Midra links.
   Never collapse `S1` and `A1` into the same destination. Keep shared-destination
   conflict checks and folder scopes working across the new identities.
5. **Send position/size with readback.** Resolve the chosen bank, use Aquilon's
   position/size fields and its apply pulse. Verify both application state and the
   emulator's rendered Preview before declaring the send path complete.
6. **Verify crop independently.** Compare source correction, classic crop, mask,
   fitting, zoom/offset and both export sizing strategies against known layouts.
   Reuse editor geometry math where appropriate; keep device conversion separate.
7. **Add capture and scale coverage.** Our snapshot input limit is currently 16,
   and parsing caps allocated layers at 8. Replace family-specific limits with
   discovered capabilities plus explicit resource bounds. Reuse pooled low-rate
   capture scheduling; verify Aquilon snapshot IDs, endpoints and image dimensions.
8. **Acceptance matrix.** Cover single and linked units, each tested firmware
   profile, multiple screens, mapping save/load, history/autoconnect, explicit
   Program opt-in and folder sends. Record tested models/firmwares rather than
   claiming the entire range from one identity string.

No persistent schema or device-send contract changes are made by this research.
The emulator address and firmware are now known; next inspect the captured state
and derive sanitized fixtures before implementing the family-specific adapter.
