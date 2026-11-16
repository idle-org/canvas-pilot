# Release preparation

## Published milestone: v0.8.0

[Download v0.8.0](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/releases/tag/v0.8.0).
The annotated tag identifies source commit `eb82dc7cc91105793d2d31552da71763d2218df5`;
subsequent publication documentation does not move that tag. The release has 21
assets: four portable bundles, four canonical cargo-binstall archives, four raw
binaries, the artifact manifest/checksums, and seven manual/template/build extras.
The [release notes](release-notes/v0.8.0.md) describe compatibility and downloads.

All four builds passed. Linux GUI startup/render passed with isolated settings.
Both macOS binaries passed Mach-O architecture, minimum macOS 13.0, system-library,
archive-layout and checksum checks. Native Windows/macOS runtime testing was not
available. All 21 assets were downloaded and hash-checked before publication and
again anonymously after publication. cargo-binstall retrieved all four targets
from the public release; each installed binary matched the built artifact SHA-256.

### Required macOS compiler override

The original pinned SDK image contains Rust 1.93, but the locked CAD dependencies
require Rust 1.95 or newer. The first real release build exposed this mismatch;
the earlier dry run could not detect it. v0.8.0 keeps its application source and
lockfile unchanged and uses Rust 1.100.0 for all targets.

The [compiler recipe](../../scripts/release/Containerfile.apple) extends the original
immutable image with Rust 1.100.0 and both Apple target libraries. Its built digest,
Cross image digests and validation boundaries are in the release's
`build-provenance.json`. The macOS configuration used is attached as
`rust-release-v0.8.0.toml`; the image is locally built, not a public registry upload.

For a fresh build host, download `Containerfile.apple`, `BUILDING-v0.8.0.md` and
`rust-release-v0.8.0.toml` from the release. Build the recipe, then set the supplied
configuration's `build.apple.image` to your local image's immutable digest, as
explained in the build instructions. A rebuilt image's digest can differ because
of build timestamps. From a clean tagged checkout:

```sh
rust_realease . --package canvas-pilot --tag v0.8.0 --dry-run --config /path/to/rust-release-v0.8.0.toml
rust_realease . --package canvas-pilot --tag v0.8.0 --compile-only --artifact-format both --config /path/to/rust-release-v0.8.0.toml
```

Keep default features enabled for vendor formats. The second command builds and
packages all targets without publishing. For this publication, the successful
Linux/Windows binaries were retained after the first macOS failure; macOS was
rerun with the compatible compiler. Retained binaries were packaged to the same
configured layouts with entry/hash/permission/architecture verification. The
combined artifact manifest records this rather than claiming one uninterrupted run.

Release assets were uploaded as a draft using the authorized token, verified, then
published through Gitea. Credentials and generated archives remain outside Git.
Do not replace existing public assets or move the release tag for future fixes;
prepare a new version instead.

## Packaging configuration

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

### Historical readiness audit — 2026-11-16 (before the milestone)

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
