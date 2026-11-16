# Canvas Pilot manual

The book uses existing documentation as chapter sources, listed in
[SUMMARY.md](canvas-pilot/SUMMARY.md). Add prose to its owning document instead of
duplicating it here. Asset downloads and chapter links are rebased during staging.

```sh
cargo install mdbook --version "$(cat docs/manuals/mdbook-version)" --locked
python3 scripts/manuals.py build
python3 scripts/manuals.py check
```

The standalone preview is `build/manuals/site/manuals/index.html`. Generated files
are ignored. Build checks every local link and anchor, including the print view.

The public URL stays https://books.dandaedre.com/manuals/canvas-pilot/.
On a successful main build, Jenkins runs `python3 scripts/manuals.py publish`.
Publication requires `BRANCH_NAME=main`, no `CHANGE_ID`, `rsync`, and the existing
writable `/mnt/books` mount. Only `manuals/canvas-pilot/` is synchronized with deletion;
the shared index and other books are preserved. beffroi-tools publishes that index
and links to this independently published book. No automatic deployment occurs
until the new Jenkins job is configured.
