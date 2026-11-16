# Localization roadmap

[Development](README.md) · [Project plan](../canvas-pilot/plan.md)

## Current pilot: French quick start only

The [French quick start](../canvas-pilot/guide/fr/quickstart.md) is based on machine translation,
requested as a small first step. French interface terminology was accepted on
2026-11-16; the chapter now follows those French control labels and uses three
fresh French screenshots. English remains authoritative. This records approval
of UI wording, not an independent editorial review of the adapted chapter.
The application also has six translated UI previews (see below). The rest of the manual remains English, with explicit fallback links.

For this one-chapter pilot, use translated Markdown in `guide/fr/` and the existing
mdBook build. The summary and reciprocal chapter links expose the language choice;
a `lang="fr"` wrapper identifies French content while the book navigation stays
English. The warning appears in the chapter and print view. Existing assets and
English URLs remain unchanged. Do not duplicate the whole book for this pilot.

The translation comment records the English source commit. When updating the English
quick start, compare it with that revision, update the French text (or visibly mark
it outdated), and advance the recorded revision only after translating the changes.
For the latest aligned source revision:

```sh
git diff daa8975 -- docs/canvas-pilot/guide/quickstart.md
```

The French chapter additionally explains language selection and uses French
controls and images. Keep its machine-translation origin and English-authority
notice visible. Build/link checks are not linguistic verification. Run the pinned manual build after
updates and check French accents, links, screenshots, search and print output.

For a larger translation effort, evaluate
[mdbook-i18n-helpers / gettext](https://github.com/google/mdbook-i18n-helpers/blob/main/i18n-helpers/USAGE.md)
for extracting messages and tracking changed translations. mdBook's
[language setting](https://rust-lang.github.io/mdBook/format/configuration/general.html)
sets book language metadata; it does not manage translation freshness. Separate
language builds and a full language selector remain future work. Do not introduce
both Markdown and gettext as competing maintained translation sources.

The checklist below describes that broader future work. The later reviewed-book
publication criteria do not describe the current status of the whole manual.
The accepted French interface vocabulary does not imply review of other languages.

## Application implementation

The desktop UI uses [rust-i18n](https://docs.rs/rust-i18n/) with embedded English, French, Japanese, Simplified Chinese, Spanish, Ancient Greek
and Esperanto JSON catalogs in `crates/canvas-pilot/locales/`. `sys-locale` obtains the desktop
language. There is no custom translation parser, language-file loader, lookup cache
or network translation service. The lockfile pins the dependency versions.

`src/localization.rs` only resolves the supported language preference and passes an
explicit locale to rust-i18n. Each egui context holds its own resolved language;
there is no process-global `set_locale`, avoiding cross-window/test interference.
Catalogs compile into the binary, so portable releases need no additional files.
The build script watches `locales/`, including catalog-only edits and newly added
files; translators do not need to touch Rust code to trigger a rebuild.

Settings v10 adds `ja`, `zh-CN`, `es`, `grc` and `eo` to v9’s `system`, `en`, `fr`.
Versions 1–8 migrate with System default; v9 preserves its language and all other
preferences. Unknown old language values are rejected rather than reinterpreted. The always
visible **Language / Langue** selector at the top of Settings applies immediately.
Project, export, clipboard and device contracts are unchanged. Every non-English
language has the same preview status, not a human-reviewed complete translation: technical diagnostics, some advanced
controls, device-supplied labels and native file dialogs can still be English.
Native dialogs follow the OS, not this application's preference.

### Fonts and script support

The application embeds full Noto Sans 2.015 (Latin, polytonic Greek and Cyrillic)
and Noto Sans CJK 2.004 (including Japanese and Chinese). The collection is embedded
once, using face 0 for Japanese and face 2 for Simplified Chinese. This preserves
regional Han letterforms and supports arbitrary names beyond catalog characters.
Noto Sans leads proportional text; CJK and existing symbol/emoji fonts provide
fallbacks. Monospace keeps its existing first choice with Noto fallbacks. Language
selection rebuilds the font definitions once per change, never per frame.

See the [font provenance and checksums](../../crates/canvas-pilot/assets/fonts/README.md).
The SIL Open Font License is included with the fonts and embedded in the executable,
available in Settings → Files & settings → Font licenses. The full CJK collection
adds about 19 MiB to the embedded assets; it is deliberately not reduced to only
current translations, which would break future text and imported names.

Tests check every catalog character, every language endonym, mixed Latin/CJK/Greek
strings, both font families and the selected CJK face. Greek catalog text uses NFC
precomposed accents for this renderer. These checks establish glyph coverage, not
human translation review or general support for all complex scripts.

Nepali and Thai were considered but are not selectable: egui 0.31's per-character
layout does not perform the shaping/mark positioning they require. Revisit them
with a maintained shaping-capable renderer, including conjuncts, reordered vowels,
combining marks, cursor movement and selection tests. Do not fake shaping by
rewriting Unicode text, mapping words into private-use glyphs, or romanizing labels.
All available languages are normal choices, without joke labels or special modes.

### Operator terminology and content boundaries

Localization is cosmetic: translate interface instructions, never project content.
Keep all titles and default names/values in their original language, including newly
created layers, folders and example content. Exports, clipboard payloads, generated
artwork, device values and serialization remain independent of the selected language.

For French, use **ratio**, not “rapport”, for aspect-ratio controls. Generic editor
terms may use **écran** and **calque**; do not force these translations onto names
shown by the switcher. Preserve the English MIDRA/WebRCS control and option names
operators need to match on their device, including **Content ratio**, **Transform to**,
**Layer fill option**, **Native**, **Signal**, **Custom**, **Layer setting**,
**Centered**, **Cropped**, **Full screen / Fullscreen**, **1:1**, **Input settings**,
**Global settings**, **Preview**, **Program** and **Blanking**. Retain the spelling
used by the corresponding control. Translate surrounding explanations; an optional
French explanation should accompany, rather than replace, the English device name.
Apply the same principle to other adapters' native terminology.

### Maintaining UI messages

- Use stable descriptive keys and complete messages in `locales/en.json` and
  `locales/fr.json`. Keep existing keys when polishing wording; do not derive keys
  from runtime text or translate arbitrary document/device names.
- Call `tr(ui.ctx(), "ui.message-key")` for simple text. It delegates to rust-i18n.
  For values, call `rust_i18n::t!("key", locale = localization::locale(ctx), name = value)`
  and convert to an owned string for egui. Use named `%{name}` placeholders, never
  concatenate grammatical sentence fragments. Numeric summaries should avoid
  English-only singular/plural assumptions; evaluate plural-aware messages before
  adding inflected count-dependent wording.
- Update all seven catalogs together. English is the fallback; catalog tests check key and
  placeholder parity and direct UI references. Do not translate serialization keys,
  protocol values, keyboard shortcut identities, project names, user input or IDs.
- Give windows and controls stable IDs separate from translated display text.
  Preserve semantic action dispatch; never match translated button text to decide
  what operation to execute.
- Existing diagnostic strings remain English until they can be modeled as structured
  errors with localization at display time. Do not perform string replacement on
  errors from devices, converters or external libraries.
- Follow the Rust gates in [CI](ci.md). Localization tests cover regional fallback,
  independent contexts, catalog completeness, settings migrations and interactive
  switching/restart without changing project bytes, drafts or device-send state.
  A layout regression test checks that language selection and Close remain visible
  in every section at 1000 × 720, in every language. French Settings and menus were
  also visually checked in the Linux desktop app at normal and minimum sizes.
  Native Windows/macOS visual acceptance and human language review remain pending.
  The expanded catalogs also passed native Linux visual checks for Japanese,
  Chinese, Spanish, Esperanto and polytonic Greek at 1000 × 720. Automated switching
  preserves serialized project and clipboard data, settings persist all language
  choices, and whole-application coverage passed at 91.01%.
- Technical validation does not constitute human linguistic review. Keep preview
  status until terminology and actual workflows have been reviewed.

The broader checklist below remains the roadmap for full localization, review and
additional languages; these foundation tasks do not mark the whole goal complete.

## 1. Localize the book

### Prepare the translation workflow

- [ ] Confirm the first language, its reviewer and the scope of its initial release.
  Include installation, quick start, format matrix, template catalog and advanced
  chapters. Developer contracts and historical changelogs may stay in English,
  clearly identified when linked from a translated page.
- [ ] Review source chapters for stale labels, ambiguous instructions and duplicated
  prose before translating them. Translate the canonical chapter sources, not
  READMEs, generated HTML or a second copy of the manual.
- [ ] Establish a shared glossary: layer, source/input, crop, fit, mask/blanking,
  visible box, screen, Preview/Program, load, send and snapshot. Keep product names,
  extensions and actual device identifiers unchanged. Record terms best left in
  English so the book and application use the same vocabulary later.
- [ ] Choose one translation representation: translated Markdown chapters with
  matching paths, or extracted message catalogs. Pilot a short chapter and the
  feature matrix before deciding; document the workflow and avoid maintaining both.
- [ ] Record the source revision for translations and flag changed or missing
  passages for review. Define who reviews updates and what counts as ready to publish.

### Build and publish translated books

- [ ] Extend `scripts/manuals.py` to stage each language with its own title,
  language metadata, navigation and output directory. Retain the existing English
  URLs; use language subpaths for new translations under `/manuals/canvas-pilot/`.
- [ ] Add a visible language selector that preserves the current chapter when
  possible. Provide an explicit fallback to English for untranslated chapters;
  do not silently label English content as translated.
- [ ] Resolve chapter links and section anchors within the chosen language, with
  explicit English links where no translation exists. Check contents, search,
  print view, page titles and image alt text in each build.
- [ ] Reuse the existing template files, ZIP, icons and applicable screenshots.
  Translate the catalog labels and instructions, without creating language-specific
  copies of identical downloads. Preserve existing client-facing download URLs.
- [ ] Translate captions and alt text. While screenshots and the walkthrough use English, keep their
  exact English control labels alongside translated instructions. Identify the
  language shown in screenshots; replace or supplement them after app localization.
- [ ] Extend publication and cleanup tests to cover all language directories while
  protecting other books and the shared manuals index. Publish only reviewed
  languages and make translation availability visible from the English book.

### Book acceptance checklist

- [ ] Review the first translation for natural language and consistent terminology,
  especially crop/mask differences and device-write instructions.
- [ ] Follow installation, first layout, template selection and import/export from
  the translated instructions, using the application version described by the book.
- [ ] Build every published language with the pinned mdBook version. Check all
  local links, anchors, pictures, downloads, print pages and language-switch paths.
- [ ] Check narrow-screen tables, long headings, accented text, search results and
  the format matrix's emoji/numbered notes in the rendered book.
- [ ] Document how to propose a translation, update it after an English edit and
  report a translation error. Mark the first language ready only after review.

## 2. Localize the application

Begin after the book workflow and glossary have been validated.

### Separate display language from application data

- [ ] Inventory visible text: menus, dialogs, toolbar tooltips, keyboard help,
  properties, presets, empty states, status messages, progress, errors, file
  filters, recovery/overwrite prompts, device review and connection controls.
  Include launcher text where relevant. Generated SVG labels and export artwork
  remain in their original language as document content.
- [ ] Choose a small Rust-compatible localization mechanism with stable message
  keys, named placeholders, plural support and English fallback. Trial it in one
  settings panel and an error dialog before migrating the rest of the UI.
- [ ] Keep complete sentences in catalogs; avoid concatenating translated fragments.
  Separate raw device/converter diagnostics from translated explanations, retaining
  technical detail useful for troubleshooting.
- [ ] Keep project field names, schema markers, adapter identifiers, protocol paths,
  enum values, file extensions and numeric serialization language-independent.
  Language changes must not alter geometry, project compatibility or device writes.
- [ ] Protect copy/paste: `gui.rs` currently emits labels such as `Horizontal:` and
  `Crop zoom:` that `import.rs` parses. Keep this exchange representation stable,
  or design and test a versioned language-independent payload before translating
  its human-readable presentation. Preserve existing clipboard samples.
- [ ] Preserve existing layer/folder names, imported names, custom presets and
  device-provided labels. New-project titles, example names and default values also
  remain in their original language; never rename content when switching languages.

### Add language preferences and readable UI

- [ ] Add an application language preference with **System default**, explicit
  supported languages and English fallback. Define region matching and the
  behavior when the saved language is unavailable.
- [ ] Store the preference in application settings, not in projects. Update the
  settings contract, migration examples and regression tests if its schema changes.
- [ ] Define whether language changes apply immediately or need restart. Preserve
  unsaved edits and active operations; switching language must not start a device
  action or discard a draft.
- [ ] Define decimal entry and formatting for comma-decimal locales. Keep file
  exports, clipboard data and device numbers in their documented invariant form;
  reject ambiguous input rather than silently changing a measurement.
- [ ] Verify font coverage for each shipped language and include required font
  licensing. Check emoji, accents, units, shortcuts and fallback glyphs.
- [ ] Make controls tolerate longer text: buttons, narrow sidebars, combo boxes,
  tabs, table headers, tooltips and status bars. Test a deliberately expanded
  pseudo-language before relying on short English strings.
- [ ] Point Help links to the selected language's book with English fallback.
  Refresh localized screenshots only after terminology and layout are stable.

### Translate and validate

- [ ] Translate the agreed UI scope using the book glossary; review destructive
  actions, overwrite/recovery prompts and Preview/Program warnings carefully.
- [ ] Add catalog checks for missing/unused keys, placeholder mismatches, malformed
  messages, plural cases and fallback behavior. Define a completeness threshold
  before a language appears in the language selector.
- [ ] Test settings round trips, missing-language fallback and language switching
  with pending edits, recovery state and disconnected devices.
- [ ] Test copy/paste between languages and confirm saved projects and exchanges
  remain interoperable. Verify locale-independent numeric output and decimal entry.
- [ ] Check supported desktop platforms, display scaling, small windows and long
  translations. Keep a short manual walkthrough for each shipped language.
- [ ] Run the existing default/restricted Rust gates and whole-application coverage;
  add meaningful localization regression tests without duplicating every label.
- [ ] Review the matching book and application together, record translation status
  in release notes, and assign ongoing review of newly added messages.

## Later, when a language needs it

- [ ] Evaluate right-to-left layout, bidirectional text and text shaping before
  advertising a language that requires them; font coverage alone is insufficient.
- [ ] Add translated video captions and transcripts when the planned videos exist.
- [ ] Consider a hosted translation platform only when contributor volume makes
  the repository-based workflow difficult to maintain.
