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
