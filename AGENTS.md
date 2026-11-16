# Canvas Pilot

Independent Rust/egui layout editor for presentation switchers; not affiliated with
Analog Way. This repository owns the application, device probe, exchange templates,
manual and build tooling. It must build without a sibling beffroi-tools checkout.
Keep the single-member workspace and adapter design simple; avoid speculative layers
of abstraction. User instructions take precedence over this guidance.

When resuming the migrated conversation, read `docs/development/handoff.md` for
pending decisions, video plans, prior test authorization and publication state.
Treat dated observations as a snapshot and verify current state before acting.

## Project map

- `crates/canvas-pilot/src/`: editor and application. `lib.rs`, `project.rs`,
  `formats.rs` and `snapping.rs` own geometry, persistence and editing rules;
  `gui.rs` and the `*_ui.rs` modules own interaction and presentation.
- `device.rs`, `device/`, `device_send.rs` and `device_capture.rs`: shared device
  representation and Midra/Aquilon implementations. Keep protocol paths, conversion
  and apply behavior inside the appropriate adapter; do not leak them into generic
  geometry or editor controls. Preserve bounded reads, partial writes, preflight,
  cancellation, explicit unsupported-mode errors and verified readback.
- `exchange.rs`, `interchange.rs`, `geometry_formats.rs`, `vendor_formats.rs` and
  `exchange_metadata.rs`: exchange formats and embedded metadata. Preserve the shared
  geometry model, precision, import review and optional vendor-format build.
- `crates/canvas-pilot/tests/` and `tests/fixtures/` within that crate: Rust regression
  tests and sanitized device/clipboard fixtures. `examples/` contains explicitly
  invoked device diagnostics; these are not automated hardware tests.
- `tools/midra_probe.py`: standalone, read-only Python diagnostic tool.
  `tests/` at the repository root covers the probe, templates and build/publication
  helpers. `docs/templates/canvas-pilot/` owns all eleven-format starter files and ZIP.
- `scripts/`, root Cargo/Cross/release configuration and `Jenkinsfile`: independent
  build, checks, packaging, artwork, launchers and documentation publication.

## Authoritative documentation

Use `AGENT_DOCUMENTATION.md` as the index for contracts, research, handoffs,
acceptance evidence and technical notes. Use `docs/development/README.md` for the
project plan, tasks, stretch goals and contribution workflow. Read the linked
records before changing behavior; do not duplicate their status in new checklists.

The user manual is built from the Markdown files listed in
`docs/manuals/canvas-pilot/SUMMARY.md`. Edit those chapters as the single source of
user-facing instructions. Keep READMEs short navigation pages. Do not put task lists,
handoffs or agent contracts on the main README or in the user book's chapter list.
Keep completed work, remaining acceptance and future ideas distinct. Preserve the
documented support boundaries; emulator evidence is not physical-output acceptance.

## Compatibility and device behavior

Document every persistent save/export representation under `docs/interfaces/`,
including fields, units, precision, versions, validation, omissions and round trips.
The project, settings, recovery and exchange contracts and versioned examples are
there, alongside device adapters and reviewed Midra transfers. Update documentation,
fixtures and meaningful regression tests in the same change as a schema modification.
Provide tested migrations or clear unsupported-version errors for breaking changes.
Preserve settings locations and legacy Midra identifiers; renaming the repository
must not reset user preferences or change serialized formats.

Preserve source documents. Imports and failed operations must not silently replace
working projects or existing exports. Keep device writes explicit and within the
user's authorized scope; never infer authorization from a saved address. Geometry
operations must not implicitly route sources, TAKE, or alter shared input settings.
Advanced input writes remain opt-in. Device writes and physical-output acceptance
are never CI tests. Do not commit credentials or unsanitized captures.

## Validation and documentation

For Rust changes, follow `docs/development/ci.md`: formatting, Clippy and rustdoc
with warnings denied, workspace tests, default vendor-format tests and the
`--no-default-features --features no_proprietary` tests. `--all-features` activates
`no_proprietary`, so it does not cover the normal vendor codecs by itself.
Run `python3 scripts/ci.py coverage` for whole-application coverage (75% floor);
never exclude production GUI or format code to inflate it.

Run `python3 -m unittest discover -s tests -v` for Python/build-helper changes.
For documentation changes, run `python3 scripts/manuals.py build` with the mdBook
version pinned in `docs/manuals/mdbook-version`; it validates links, downloads and
print-view anchors. Use `docs/manuals/canvas-pilot/SUMMARY.md` to maintain chapters.
Keep screenshots and template bytes intact unless intentionally updating them;
the template ZIP and individual files must agree. Test actual rendering with the
relevant external tools when available; report checks that could not be completed.

Keep root README concise and put details in `docs/`. Add a succinct Unreleased
entry to `docs/canvas-pilot/changelog.md` for each change, including useful validation
or limitations; it is linked from `docs/changelogs.md`.

## Version bookkeeping

Bump the workspace patch version once per completed subject (feature, fix or
substantive documentation task), not once per commit. Follow-up corrections and
validation for that same subject share its version. Read-only work needs no bump.
Use a requested minor/major bump instead of an additional patch bump. Keep
`Cargo.toml`, the Canvas Pilot entry in `Cargo.lock` and the changelog in sync;
group entries by version under Unreleased until published. Keep this lightweight:
version bookkeeping alone does not call for a tag, binary build, push or release.

## Publication and working conventions

Manual publication owns only `/mnt/books/manuals/canvas-pilot/`; never replace the
shared manuals index or another book. Badge publication owns only
`/mnt/assets/coverage/idle-org/canvas-pilot/main/`. Main/non-PR restrictions apply.
Jenkins archives exactly the version, test and coverage SVGs; it does not create
releases or publish binaries. See `docs/development/ci.md` for job setup and
`docs/development/releases.md` for manually invoked packaging.

Inspect the working tree first. Preserve unrelated changes, use existing dependencies
and Python's standard library where practical, and keep temporary files temporary.
Never commit credentials, caches, generated manuals, release archives or executables.
Every completed task changing repository files must end with a descriptive Git commit.
Stage only task-owned changes, report the commit and validation, and do not push unless
requested. Read-only audits need no empty commit.
