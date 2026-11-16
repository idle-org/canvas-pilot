# Canvas Pilot

Keep the single-member Cargo workspace simple. Preserve adapter boundaries and the
shared geometry representation. Device writes require explicit user authorization;
never run real hardware tests in CI. The Python probe remains read-only.

Document persistent formats, units, precision, validation and migrations under
`docs/interfaces/`. Update fixtures and regression tests with schema changes.
Keep versioned compatibility, settings storage paths and legacy identifiers stable.
Use `docs/canvas-pilot/changelog.md` for succinct Unreleased entries for every change,
and keep the book chapter list and user guide current. See `docs/development/ci.md`
for required Rust/Python checks, both vendor and no_proprietary configurations, and
whole-application coverage. Never exclude production code to inflate coverage.

Manual publication owns only `/mnt/books/manuals/canvas-pilot/`; never replace the
shared manuals index or another book. Badge publication owns only
`/mnt/assets/coverage/idle-org/canvas-pilot/main/`. CI must not create releases.

Inspect the working tree first. Preserve unrelated changes and commit every completed
task with a descriptive message. Report the commit and validation; do not push unless
requested. Do not commit credentials, temporary files, generated manuals or binaries.
