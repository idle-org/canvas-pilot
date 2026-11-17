# Aquilon 6.2 fixture

Sanitized from the user-authorized AQL Cmax 6.2.79 emulator, read 2026-11-15.
Only native screen status, selected preset-layer geometry/crop/source fields,
transition groups and four input status/settings records are retained. Labels,
network configuration, credentials and unrelated system/hardware state are omitted.
S1 has seven allocated numbered layers; its stored eighth slot tests allocation
filtering. S2 and A1 are disabled. Test code creates explicit synthetic variations
for active auxiliaries, canvas changes, bank swaps and malformed fields.

The full 118 MiB response remains a private temporary artifact. The targeted-read collector and parser exercise this fixture; test helpers
add collection keys and identity fields for the mock AWJ endpoint; live write/restore diagnostics
are separate examples, never CI tests.

`aquilon-models.json` pairs HTTP names with the eleven model codes documented in
Analog Way's AWJ V6.2 guide, section 2.1 (including the RS Alpha spelling alias).
Tests substitute these identities and firmware versions into the Cmax fixture.
These synthetic cases exercise identity matching and the common adapter; they
are not captures or acceptance evidence for those models/firmware releases.

Mask regressions vary the sanitized layer's four `cropping.mask.pp` edges and
container geometry. They cover both editor mask origins, centered margins in both
sizing strategies, Preview/Program, full blanking, reset-to-zero, malformed units
and quantization limits. The separate RS1 6.2.79 live diagnostic and WebRCS evidence
are recorded in `docs/canvas-pilot/aquilon-compatibility.md`; the fixture remains
the original Cmax capture, not a relabelled RS1 capture.

`aquilon-rs1-mask-6.2.json` is a separate sanitized RS1 6.2.79 layer readback from
2026-11-17. The explicitly invoked diagnostic temporarily sent a fractional
640.5 × 360.25 Preview layer with asymmetric mask, then restored its baseline.
The fixture retains identity, one routed layer's geometry/crop/mask, and the
requested editor geometry/mask for the one-pixel round-trip assertion. WebRCS's
rendered clip percentages matched the native mask units. No labels, addresses,
credentials or unrelated configuration are included.
