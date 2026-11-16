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
# Build and package locally without creating a release.
rust_realease . --package canvas-pilot --compile-only
```

For a local launcher after `cargo build --release --locked`:

```sh
python3 scripts/create-desktop-launcher.py canvas-pilot --binary "$PWD/target/release/canvas-pilot" --output "$HOME/.local/share"
```

On macOS add `--platform macos --output "$HOME/Applications"`. Existing launchers
are never overwritten. Regenerate icons with `scripts/build-icons.py` (ImageMagick).
Artwork provenance is in `icon-prompts.json`.
