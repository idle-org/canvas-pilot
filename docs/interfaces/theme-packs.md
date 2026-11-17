# Canvas Pilot theme packs, version 1

Implementation: `crates/canvas-pilot/src/theme_packs.rs`. User instructions and
editable examples: [Add a theme pack](../canvas-pilot/guide/theme-packs.md).
Supported from Canvas Pilot v0.9.26.

## Archive and palette representation

A ZIP contains exactly one regular root file named `manifest.json`. Directories,
symlinks, other paths, extra entries and unsupported ZIP compression/encryption
are rejected. Nothing is extracted. Maximum archive size is 2 MiB; the manifest
has a 4 MiB expanded limit enforced while reading. CRC/decompression failures
reject the pack. JSON is UTF-8; unknown fields and unsupported versions fail.
Repeated JSON object keys have serde's last-value behavior; authors must use unique
keys. Array order determines theme order within the pack.

| Field | Representation and validation |
| --- | --- |
| `version` | Required integer `1` |
| `id` | Required pack ID: 1–63 lowercase ASCII letters, digits or hyphens, no leading/trailing hyphen |
| `revision` | Required positive `u64`, 1–18446744073709551615; revision comparison is numeric |
| `themes` | Required array of 1–16 themes |
| `themes[].id` | Same identifier syntax, unique within this pack |
| `themes[].name` | Nonblank display name, 1–160 UTF-8 bytes, no control characters |
| `themes[].light` | Required boolean selecting egui light/dark base visuals |
| `themes[].colors` | Required complete palette object, fields below |
| `notices` | Optional attribution/license text, default empty; at most 65,536 UTF-8 bytes; controls forbidden except newline, tab and carriage return |

Every color is an opaque sRGB string `#RRGGBB`, exactly six hexadecimal digits
(case-insensitive). Channels have 8-bit precision; no alpha, gradients, numeric
units or geometry are represented. The original ZIP bytes are preserved.

| Palette field | Use |
| --- | --- |
| `bg`, `panel`, `field` | App background, panels/windows, input fields |
| `line`, `text`, `muted` | Borders, primary text, secondary text |
| `cyan`, `amber`, `purple` | Accent/handles, selected-layer/warning accent, selection/button fill |
| `canvas`, `banner`, `grid`, `layer` | Viewport background, example banner, grid, layer fill |
| `pattern` | Exactly two color strings for alternating source-helper tiles |

The theme system derives hover colors from these roles and uses its existing
light/dark error colors. Font families, text sizes, interface scale, geometry,
layout metrics, source images and device behavior are not supplied by packs.
There is no executable content or network loading. Authors are responsible for
contrast; the examples test at least 4.5:1 for primary text on panel, field,
selection and both source-helper tiles. Theme packs cannot override built-ins.

## Installation, updates and removal

Packs live in `<settings-parent>/theme-packs/<id>.zip`. The logical choice is
`<pack-id>/<theme-id>`, so separate packs can reuse a local theme ID. Installation
validates the entire archive before writing; an update replaces the previously
loaded path for that ID, even if its ZIP was manually renamed. A valid destination
owned by another pack ID cannot be overwritten.

A new pack or higher revision installs immediately. Equal/lower revisions and an
unreadable existing destination require overwrite confirmation. The reviewed
source bytes are retained in memory; cancellation changes neither disk nor the
loaded palettes. Before commit, destination bytes must still match those reviewed.
Writes use a flushed temporary file in the destination directory, atomically
persisted with no-clobber behavior for a new pack. Runtime registration changes
only after persistence succeeds. Existing destinations over 2 MiB are not replaced.

Updates replace the entire theme list. The active palette refreshes immediately;
if an update removes the active theme, Midra Dark is selected and saved. Removing
a pack confirms its ID and included names, requires exact equality with its loaded
ZIP bytes, then deletes only the installed copy. A changed/unreadable file leaves
runtime state intact. Removing an active theme also selects and saves Midra Dark;
other themes keep their selection. Unavailable recent entries are pruned during
management, retaining a currently selected missing preference if unrelated to the
operation. Source downloads are untouched.

Startup is offline, scans at most 33 ZIP paths to detect the 32-pack limit, sorts
them and considers at most 32. Too many packs, invalid files and duplicate IDs
produce errors without stopping startup; the first valid sorted file for an ID
wins. Installing a 33rd ZIP is refused; updates at the limit are allowed. Manual
changes to installed files take effect after restart.

## Settings, missing packs and round trips

[Settings version 15](canvas-pilot-settings.md) stores a packed theme as
`{"pack":"tokyo-night/night"}` in `theme` and `recent_themes`; built-in string IDs
are unchanged. Versions 1–14 migrate preserving preferences and history, but pack
choices under a legacy version are rejected. Older applications reject version 15.
The [default fixture](canvas-pilot-settings-v15.example.json) is the migration target.

Missing/invalid packs preserve the saved theme choice and show an unavailable
label, rendering with Midra Dark until the pack is installed again or the user
selects another theme. Installation alone does not select an unrelated theme.
Loaded palettes and selection are scoped to an egui context. High Contrast can
return to a packed previous theme. Project, recovery and exchange files contain
no pack palettes or theme preferences; their representations remain unchanged.

The editable manifests and ZIPs contain identical bytes, checked by Rust tests
and `python3 scripts/build-theme-packs.py --check`. Lifecycle tests cover startup,
selection persistence, live updates, overwrite review/cancellation, changed-file
protection, removal, limits, malformed colors/archives and context isolation.

Distributed example ZIPs use stored (uncompressed) entries with a fixed timestamp
and Unix file metadata so exact rebuilds do not depend on the host or zlib version.
Their manifest bytes, pack revisions and version-1 representation are unchanged;
the importer continues to accept both stored and DEFLATE-compressed packs.

## Bundled collection

`docs/theme-packs/extra-themes.zip` is a normal version-1 pack (`extra-themes`,
revision 3), containing all 16 distributed themes. One import validates and installs
all entries atomically; it does not unpack nested ZIPs. The build script combines
individual manifests in sorted path order, prefixes theme IDs with their source
pack ID and preserves all notices. Individual packs and the collection have distinct
saved choices and can coexist. Updates and removal operate on the whole collection.
