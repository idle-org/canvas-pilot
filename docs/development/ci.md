# CI and local checks

Create a Jenkins Multibranch Pipeline for the Gitea repository `idle-org/canvas-pilot`,
using the root `Jenkinsfile` and the existing `rust_no_docker` worker. Configure SCM
credentials and branch discovery in Jenkins; Gitea repository access does not configure
Jenkins jobs. Main publishes only after successful validation; pull requests never publish.

```sh
python3 -m unittest discover -s tests -v
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo clippy --locked -p canvas-pilot --all-targets -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked -p canvas-pilot
cargo test --locked -p canvas-pilot --no-default-features --features no_proprietary
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
python3 scripts/ci.py coverage
```

Coverage uses default features and includes the GUI and vendor format codecs.
`--all-features` activates `no_proprietary`, so the default and restricted builds
are checked separately. The floor in `coverage.toml` applies to the whole application.
CI pins cargo-llvm-cov 0.9.1 and mdBook via `docs/manuals/mdbook-version`.

The worker requires writable mounts `/mnt/assets/coverage` and `/mnt/books` plus
`rsync`. Cleanup archives exactly three SVG badges and publishes them only to
`/mnt/assets/coverage/idle-org/canvas-pilot/main/`. The manual publisher updates
only `/mnt/books/manuals/canvas-pilot/`; beffroi-tools owns the shared index.
CI does not create releases or publish binaries.
