# Agent documentation

Start with [AGENTS.md](AGENTS.md) for working rules and the
[conversation handoff](docs/development/handoff.md) for dated context and pending
decisions. Verify current code, Git and external state before acting on old notes.
This index links authoritative records; it does not duplicate their content.

## Planning and project history

- [Development entry point](docs/development/README.md).
- [Implementation, to-do, acceptance tests and stretch goals](docs/canvas-pilot/plan.md).
- [Book and application localization roadmap](docs/development/localization.md).
- [Future adapter research and documentation ranking](docs/canvas-pilot/presentation-switcher-adapters.md).
- [Completed Aquilon support task](docs/canvas-pilot/aquilon_support.task.md).
- [Feature demonstration/recording checklist](docs/canvas-pilot/feature-demo-checklist.md).
- [Migration provenance and completeness audit](docs/development/migration.md).
- [Changelog](docs/canvas-pilot/changelog.md).

## Contracts and compatibility

Versioned examples and sanitized fixtures remain beside their contracts and in
`crates/canvas-pilot/tests/fixtures/`; regression tests refer to these paths.

| Contract | Scope |
| --- | --- |
| [Project](docs/interfaces/canvas-pilot.md) | Project schema, versions and migrations |
| [Settings](docs/interfaces/canvas-pilot-settings.md) | User preferences and settings compatibility |
| [Language packs](docs/interfaces/language-packs.md) | Runtime ZIP catalogs, installation and fallback |
| [Recovery](docs/interfaces/canvas-pilot-recovery.md) | Recovery storage and restoration |
| [Layer clipboard](docs/interfaces/canvas-pilot-clipboard.md) | Versioned geometry, crop, fit and blanking payload |
| [Geometry exchange](docs/interfaces/canvas-pilot-exchange.md) | Precision, omissions and round trips |
| [Extra parameters](docs/interfaces/midra-extra-parameters.md) | Embedded project snapshots |
| [Device adapters](docs/interfaces/device-adapters.md) | Shared representation and family-specific behavior |
| [Device import](docs/interfaces/midra-device-import.md) | Read-only import and conversion |
| [Device send](docs/interfaces/midra-device-send.md) | Reviewed writes, preflight, quantization and readback |
| [Input previews](docs/interfaces/midra-input-preview.md) | Capture and cache behavior |
| [Desktop launchers](docs/interfaces/desktop-launchers.md) | Generated shortcut and bundle formats |
| [Persistence audit](docs/interfaces/canvas-pilot-persistence-audit.md) | Coverage of persistent state |

## Research, evidence and validation notes

- [Hardware verification record](docs/canvas-pilot/device-check.md).
- [Aquilon compatibility investigation and emulator evidence](docs/canvas-pilot/aquilon-compatibility.md).
- [Practical Midra aspect acceptance checklist](docs/canvas-pilot/midra-aspect-test-checklist.md).
- [Fit precision and mask-rasterization questions for engineers](docs/canvas-pilot/midra-fit-precision.md).
- [Resolution and crop model](docs/canvas-pilot/resolutions.md), also a user-book reference.
- [Viewport performance reproduction](docs/canvas-pilot/guide/performance.md).
- [Screenshot provenance and capture recipes](docs/canvas-pilot/images/capture.md).
- [Historical visual proposals](docs/canvas-pilot/plan.md#visual-proposals):
  [A](docs/canvas-pilot/design/a-concept.svg), [B](docs/canvas-pilot/design/b-concept.svg),
  [C](docs/canvas-pilot/design/c-concept.svg).
- [Icon-generation provenance](docs/development/icon-prompts.json).

## Build and documentation maintenance

- [CI, validation commands and publication boundaries](docs/development/ci.md).
- [Release preparation and no-build readiness findings](docs/development/releases.md).
- [Book generation and publication](docs/manuals/building.md),
  [chapter list](docs/manuals/canvas-pilot/SUMMARY.md) and
  [book configuration](docs/manuals/canvas-pilot/book.toml).
- [User-book introduction source](docs/canvas-pilot/introduction.md),
  [chapter overview](docs/canvas-pilot/guide/overview.md) and
  [template catalog](docs/templates/canvas-pilot/catalog.md).

The book owns user-facing explanations. Development owns the task-list entry point.
Agent notes, handoffs and contracts are reached through this index and are not book
chapters. Preserve useful cross-references and source documents; generated HTML is
not a second editable source. Historical migration inventories describe their audit
date, not a promise that documentation filenames can never change.
