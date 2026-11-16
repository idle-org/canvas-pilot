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
