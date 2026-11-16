# File and device interfaces

[Documentation](../README.md) · [Canvas Pilot guide](../canvas-pilot/README.md)

These contracts describe persistent formats, compatibility, validation and device
transfers. Versioned examples live beside the relevant contract and are exercised
by regression tests.

| Contract | Contents |
| --- | --- |
| [Canvas Pilot project](canvas-pilot.md) | JSON project versions, SVG geometry, validation and migration |
| [Application settings](canvas-pilot-settings.md) | Per-user preferences and settings versions |
| [Geometry exchange](canvas-pilot-exchange.md) | Import/export formats, precision, omissions and round trips |
| [Extra export parameters](midra-extra-parameters.md) | Optional embedded project snapshots and per-format compatibility |
| [Recovery files](canvas-pilot-recovery.md) | Recovery storage and restore behavior |
| [Device import](midra-device-import.md) | Read-only device geometry import |
| [Device send](midra-device-send.md) | Reviewed writes, crop conversion and restore diagnostics |
| [Input previews](midra-input-preview.md) | Device snapshot retrieval and caching |
| [Desktop launchers](desktop-launchers.md) | Generated shortcuts and launcher metadata |


- [Device adapter model and Aquilon exchange](device-adapters.md): family-aware destinations, bounded reads, bank roles, geometry/crop precision and capture support.
