# Aquilon / LivePremier support — completed

Status: **first-class supported adapter for standalone AQL Cmax firmware 6.2.x**.
Implementation and emulator acceptance completed on 2026-11-15, using firmware
**6.2.79**. This record replaces the original implementation handoff and its
stale unchecked checklist. It does not extend compatibility to the entire
Aquilon range.

“First-class” means this profile participates in the normal connection, mapping,
project, load/send and capture workflows alongside Midra, with regression tests
and documented limitations. It does not mean every firmware patch or physical
output has been tested.

## Completed implementation and acceptance

- [x] Identify the supported model/firmware and reject unsupported or linked profiles.
- [x] Read bounded, targeted AWJ state with cancellation, timeouts, consistency
  checks and explicit errors; no full-store fallback.
- [x] Discover enabled screens, auxiliaries, allocated layers and source IDs;
  resolve Preview/Program roles per transition group.
- [x] Keep Aquilon paths, geometry conversion and apply behavior in a separate
  adapter around the shared screen/layer representation.
- [x] Import container geometry and supported centered crop/zoom framing;
  report unsupported modes rather than silently approximate them.
- [x] Persist family-aware screen/auxiliary/layer destinations in project v13;
  migrate older projects and recovery data with regression coverage.
- [x] Integrate mapping review, linked load/send, scopes and destination-conflict
  checks with the existing editor workflow.
- [x] Implement both sizing strategies for Preview/Program, fresh preflight,
  selective writes, the apply pulse and verified readback.
- [x] Quantize transmitted sizes/centers for WebRCS compatibility while retaining
  editor precision and checking the resulting visible geometry.
- [x] Pass twelve emulator geometry/crop/zoom transfer cases and restore the
  baseline; separately inspect rounded geometry rendered in WebRCS.
- [x] Discover input snapshots beyond input 16 and reuse pooled, throttled,
  opt-in capture behavior.
- [x] Cover parsing, conversion, persistence, malformed responses, stale state
  and transport behavior with sanitized fixtures and regression tests.
- [x] Complete the recorded workspace validation and update the guide,
  interface contracts, acceptance evidence and changelog.

## Explicit support boundaries

- Supported profile: standalone **AQL Cmax 6.2.x**; emulator-tested patch **6.2.79**.
- Physical-output acceptance remains outstanding. Emulator rendering/readback
  is the current evidence, not a claim of physical-hardware certification.
- Auxiliaries and independent screen groups have synthetic regression coverage;
  the recorded emulator baseline allocates S1 only.
- Linked units, other models/firmware, masks, non-centered fit/anchor modes and
  shared input aspect/crop corrections are not additional completed features.
  They require separately scoped work and validation before support is expanded.
- HTTP authentication was not validated on the authentication-disabled emulator;
  authentication failures remain explicit. AWJ TCP 10606 must be reachable.

See [acceptance evidence and performance](aquilon-compatibility.md),
[adapter contract](../interfaces/device-adapters.md),
[user workflow](guide/device.md#aquilon--livepremier) and
[remaining project acceptance checks](plan.md#remaining-tests).
