# Repository migration

Canvas Pilot was extracted from [beffroi-tools](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools) at
[commit ed653c569896fecf3f9f2db2fe265335d305bb4f](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools/commit/ed653c569896fecf3f9f2db2fe265335d305bb4f). This repository retains its initial commit,
then imports a snapshot of the application, tests, fixtures, templates, documentation,
manual and device probe. Earlier file history remains available in the original
repository at that commit; it has not been rewritten or copied as unrelated history.

The single-member workspace keeps `crates/canvas-pilot/` so fixture and documentation
paths stay stable. Version 0.7.0 and pinned dependency revisions are retained.
Project schemas, settings locations, legacy Midra identifiers and device behavior
are unchanged. Existing projects and preferences require no migration.

Shared CI, coverage, artwork, launcher and manual helpers were copied under the MIT
license and now belong to this repository. Build and test commands work without a
sibling beffroi-tools checkout. The manual keeps its public URL; publication owns
only its book directory and its separate badge directory. Configure the new Jenkins
Multibranch Pipeline as described in [CI](ci.md) before expecting automatic updates.

## Completeness audit — 2026-11-16

Compared all project-owned files at the source commit above with the standalone
repository after migration (`4eef1bc`). All **165 files** were present:

| Transferred material | Files |
| --- | ---: |
| Application source, assets, examples and Rust tests | 68 |
| Guides, tasks, plans, research, screenshots and visual proposals | 27 |
| Canvas Pilot interface contracts and versioned fixtures | 31 |
| Midra interface contracts and fixtures | 5 |
| Adapter contract, Python probe and its/template tests | 4 |
| Book manifest and chapter list | 2 |
| Eleven formats × two template sizes, catalog and ZIP | 24 |
| Probe artwork | 4 |

At that migration commit, 163 files matched the source byte for byte. The only
intentional differences were the changelog (migration entry and historical release
links) and book configuration (new repository URL). In particular, every task and
stretch-goal checkbox, the advanced guide, all three screenshots, three SVG design
proposals, template downloads and regression fixtures were unchanged.

Shared Cargo/CI, coverage, icons, launcher and manual helpers were copied and adapted
separately. Developer documentation, root README, license and `AGENTS.md` were
adapted for the independent project. Earlier shared release notes remain linked
at the fixed source commit. The old repository's general future-program list is
about unrelated DMX/Dante projects and is intentionally not a Canvas Pilot backlog.

The follow-up audit expanded `AGENTS.md` and the documentation index, and added the
transferred demo, aspect, precision, adapter and completed-Aquilon checklists to the
book navigation. No outstanding acceptance item was marked completed by this audit.
Generated HTML is rebuilt from these sources rather than tracked. The public URL
is unchanged; automatic publication still depends on the new Jenkins job setup.
