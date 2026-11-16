# Development

[CI and checks](ci.md) · [Manual generation](../manuals/README.md) · [Release preparation](releases.md) · [Repository migration](migration.md)

The root Cargo workspace contains `crates/canvas-pilot/`. The application, adapters,
geometry representation and tests live there. `tools/midra_probe.py` is a standalone,
read-only Python diagnostic tool. Interface contracts and historical fixtures live
in `docs/interfaces/`; maintain compatibility when changing persistence or exports.

See the [plan](../canvas-pilot/plan.md) and [device checks](../canvas-pilot/device-check.md)
for remaining work and hardware validation. Device writes are never CI tests.

See [the documentation index](../README.md) for every planning and acceptance record,
and [AGENTS.md](../../AGENTS.md) for repository working conventions.
