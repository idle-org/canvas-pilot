# Development

Canvas Pilot is a single-member Rust workspace in `crates/canvas-pilot/`.
The Python device probe lives in `tools/`; Python tests and build helpers live in
`tests/` and `scripts/`. For application usage, read the [book](https://books.dandaedre.com/manuals/canvas-pilot/).

| Work | Reference |
| --- | --- |
| Build, tests and coverage | [CI and local checks](ci.md) |
| Current milestone | [v0.8.0 — standalone and advanced exchange](release-notes/v0.8.0.md) |
| Release preparation | [Packaging and readiness](releases.md) |
| Edit or publish the book | [Manual generation](../manuals/building.md) |
| Tasks, acceptance tests and stretch goals | [Project plan](../canvas-pilot/plan.md) |
| Stretch-goal Gitea issues | [Existing issue and publication-ready drafts](stretch-goal-issues.md) |
| Localization: book, then application | [Task list and acceptance criteria](localization.md) |
| Future adapter candidates | [Research and requirements](../canvas-pilot/presentation-switcher-adapters.md) |
| Showcase and video manual preparation | [Recording checklist](../canvas-pilot/feature-demo-checklist.md) |
| Agent context, contracts and technical evidence | [Agent documentation](../../AGENT_DOCUMENTATION.md) |

Keep overall task status in the plan and localization subtasks in its linked roadmap. Keep user instructions in the book's source chapters;
READMEs provide navigation, not parallel copies. Follow [AGENTS.md](../../AGENTS.md)
when changing code or documentation.
