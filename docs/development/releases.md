# Release preparation

`rust-release.toml` configures portable Windows and Linux builds through Cross and
Apple Silicon/Intel macOS 13+ builds through the pinned cargo-zigbuild container.
Run release preparation from the repository root with the separately installed
`rust_realease` tool; the configuration packages the application, icons,
launcher helper and MIT notices. No FFprobe is needed. This is a manually invoked
workflow, separate from Jenkins; keep generated archives and binaries untracked.
Review bundles on their target platforms before publishing a release.

```sh
# Inspect the release plan without publishing.
rust_realease . --package canvas-pilot --dry-run --stop-after inspect
```

## Lightweight readiness check

```sh
rust_realease . --package canvas-pilot --dry-run
```

Unlike `--stop-after inspect`, the full dry run checks required metadata, build
prerequisites and the release tag. It does not compile, edit manifests, create tags
or publish. An exit failure for a missing tag is expected before release preparation.
`--compile-only` is **not** a lightweight check: it runs the actual builds and packages
artifacts. Run it later on a capable build host:

```sh
rust_realease . --package canvas-pilot --compile-only
```

### Readiness audit — 2026-11-16

Checked with `rust_realease 0.1.6-rc.3`, without compiling or publishing:

| Check | Result |
| --- | --- |
| Package/version and target plan | `canvas-pilot` 0.7.0; Linux, Windows, Apple Silicon and Intel macOS |
| Required cargo-binstall metadata | Added; matches the canonical `.tgz` archive layout |
| Portable bundle inputs | Icons, launcher scripts, MIT license and standalone quick-start file present |
| Cargo manifest/lock | Locked offline metadata inspection succeeds |
| Host prerequisites | Cross, cargo-binstall and a working Podman backend available |
| Apple image | Pinned registry manifest exists and its SHA-256 matches |
| Full dry run after metadata repair | Only reported prerequisite failure: missing `v0.7.0` tag |

No release tag was created. Choose the intended release version, commit the final
sources, then explicitly prepare its matching tag on that commit when authorizing
a release. A publishing run requires a clean checkout. Use the configured Gitea
token environment variable or the tool's `--token-file` option; keep credentials
outside Git. No authenticated release creation/upload was exercised by this audit.

This establishes configuration readiness, not successful cross-platform binaries.
Cross compilation, linker/SDK compatibility, packaged application startup and
post-upload cargo-binstall verification still require the actual build/release
workflow. Prior native tests are recorded in the handoff. The pinned Apple image
was not cached locally at this audit; its compressed layers total about 1,440 MiB
before any layer reuse. Image pulls and four target builds should run on a capable
host rather than as a quick check on an underpowered laptop.

The portable archive uses `docs/canvas-pilot/portable-quickstart.txt` instead of the
repository README, so it does not depend on unbundled relative documentation links.
The canonical cargo-binstall archives remain separate from the named portable bundles.

For a local launcher after `cargo build --release --locked`:

```sh
python3 scripts/create-desktop-launcher.py canvas-pilot --binary "$PWD/target/release/canvas-pilot" --output "$HOME/.local/share"
```

On macOS add `--platform macos --output "$HOME/Applications"`. Existing launchers
are never overwritten. Regenerate icons with `scripts/build-icons.py` (ImageMagick).
Artwork provenance is in `icon-prompts.json`.
