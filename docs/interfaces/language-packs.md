# Canvas Pilot language packs, versions 1 and 2

Implementation: `crates/canvas-pilot/src/language_packs.rs`. User instructions and
editable downloads: [Add a language pack](../canvas-pilot/guide/language-packs.md).
[Example archive](../language-packs/language-pack.zip).

## Archive representation

ZIP, maximum 2 MiB archive bytes and 4 MiB cumulative uncompressed bytes, checked
while reading. At most 17 entries: `manifest.json` plus 1–16 catalog files. Files
must be at the archive root. Directories, symlinks, duplicate ZIP names, unexpected
files and path separators are rejected. Nothing is extracted. Entry CRC/decompression
errors reject the pack. Unsupported ZIP encryption/compression is rejected by `zip`.

All JSON uses UTF-8. Manifest version is integer `1` or `2`; unknown manifest/description
fields and other versions are rejected. Field order has no meaning.

| Field | Contract |
| --- | --- |
| `version` | Required integer `1` (legacy) or `2` |
| `revision` | Format 2: required positive `u64` integer (1–18446744073709551615). Format 1: absent, interpreted as revision 0. Higher numbers mean newer content; no clock/timezone or app-version comparison. |
| `id` | Pack identifier; 1–63 lowercase ASCII letters, digits or hyphens; no leading/trailing hyphen |
| `languages` | Required array of 1–16 language descriptions |
| `languages[].locale` | Unique identifier with the same syntax as `id`; reserved built-in identifiers and `system` rejected |
| `languages[].name` | English display name, 1–160 UTF-8 bytes, nonblank, no control characters |
| `languages[].native_name` | Native display name, same constraints |

Each description requires `<locale>.json`, a flat object mapping existing English
catalog keys to translated strings. At least one message is required. Missing keys
fall back to embedded English through rust-i18n; unknown keys reject the whole pack.
Values must be nonblank, at most 16,384 UTF-8 bytes, with no control characters other
than newline/tab. `%{placeholder}` names must match the English message's set;
unterminated/empty placeholders are rejected. Message order is irrelevant. JSON's
ordinary serde map behavior applies: repeated object keys resolve to the last value;
authors must use unique keys. No geometry, numbers, project data or precision fields
are part of this format.

Neither version supplies fonts, language inheritance, executable plugins, network
URLs or automatic downloads. The built-in font repertoire determines rendering.
No override of built-in catalogs is allowed. Catalogs use an internal `pack:`
namespace in the runtime rust-i18n backend; the manifest contains only `uk`, etc.
Catalogs are loaded into memory, not read on every label render. Selection is
explicit per egui context, never a process-global current language.

## Installation and persistence

Installation validates every entry and locale collision before writing. Packs are
identified by `id`; locales owned by other installed IDs always reject the operation.
A new pack is persisted without clobbering to `<settings-parent>/language-packs/<id>.zip`.
An update replaces that ID's actual loaded path (including a manually named ZIP).
The source is read once, bounded and retained in memory during review. The original
ZIP bytes are preserved exactly, without extraction or re-export.

A same-ID higher revision updates immediately. Equal/lower revisions, legacy-over-
legacy and a pre-existing unreadable-as-pack destination require explicit overwrite
confirmation. Cancel changes neither disk nor runtime catalogs. A destination over
2 MiB cannot be replaced through this flow. Before committing, destination bytes are
compared against those reviewed; a change requires a fresh selection. A temporary
file in the destination directory is flushed, then atomically persisted/replaced;
runtime catalogs and available-language metadata update only after persistence succeeds.
Updates may add/remove locales; old descriptions are replaced, never appended.
The revision comparison uses the installed file, not only cached metadata.

Removal confirms the pack ID and all included languages. The installed file is
validated against its recorded ID/revision before deletion; a failure retains the
runtime registration. Successful deletion removes the pack's selector entries
immediately. Source downloads are untouched. Removing/updating away the active
language selects and saves English; other choices are retained. No restart is
required. Built-in catalogs cannot be removed or overridden.

Startup reads at most 32 `.zip` files from that directory, sorted by path when
within the limit. If there are more, the first 32 discovered entries are considered,
then sorted, and an error is reported; users must reduce the count. Invalid packs
are skipped with errors. The first valid pack supplying a locale wins; a later
colliding pack or duplicate ID is rejected in full. Installation also refuses a
33rd ZIP, but replacing an existing ZIP is allowed at the limit. Startup is offline.
Manual removal/replacement takes effect after restart.

The [settings contract](canvas-pilot-settings.md) stores an installed choice as
`"language": {"pack": "uk"}`. Missing packs retain this preference but render in
English, with an unavailable indicator in the selector. Installing the missing
pack re-applies that preference. Existing language confirmation, cancellation and
20-second rollback apply; installation alone does not select another language.
System default resolves only built-in locales. Project/recovery documents contain
no language-pack settings; device/export behavior is unchanged.

## Compatibility and evidence

Pack v1 is supported from application v0.9.7. Pack v2 and live management are
supported from v0.9.16; older applications reject format 2 rather than ignoring
its revision. The example archive is format 2, revision 1. Built-in English keys define the
accepted catalog vocabulary for the running version; removed/renamed keys require
pack updates. Adding an English key does not break partial packs. Applications before
v0.9.7 lack pack loading and reject settings v13. Tests cover ZIP/source equality,
install/restart/removal, fallback, placeholder substitution, rejected archives,
revision ordering, confirmed overwrite, canceled/stale review, live removal, settings migration, confirmation rollback and unchanged project data.
Example catalog characters are checked against the bundled fonts. Example wording
is unreviewed machine translation, not linguistic acceptance.
