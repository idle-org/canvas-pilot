# Localization roadmap

[Development](README.md) · [Project plan](../canvas-pilot/plan.md)

## Current pilot: French quick start only

The [French quick start](../canvas-pilot/guide/fr/quickstart.md) is an explicitly
unverified machine translation, requested as a small first step. English remains
authoritative. No application translation, screenshot changes or human review is
claimed. The rest of the manual remains English, with explicit fallback links.

For this one-chapter pilot, use translated Markdown in `guide/fr/` and the existing
mdBook build. The summary and reciprocal chapter links expose the language choice;
a `lang="fr"` wrapper identifies French content while the book navigation stays
English. The warning appears in the chapter and print view. Existing assets and
English URLs remain unchanged. Do not duplicate the whole book for this pilot.

The translation comment records the English source commit. When updating the English
quick start, compare it with that revision, update the French text (or visibly mark
it outdated), and advance the recorded revision only after translating the changes.
For the initial source revision:

```sh
git diff 1ac01cbf65dbad206f5b989f24924333e688f646 -- docs/canvas-pilot/guide/quickstart.md
```

The initial difference is only the new reciprocal language link. Human review remains
pending; build/link checks are not linguistic verification. Keep the visible warning
until an actual review establishes a new status. Run the pinned manual build after
updates and check French accents, links, screenshots, search and print output.

For a larger translation effort, evaluate
[mdbook-i18n-helpers / gettext](https://github.com/google/mdbook-i18n-helpers/blob/main/i18n-helpers/USAGE.md)
for extracting messages and tracking changed translations. mdBook's
[language setting](https://rust-lang.github.io/mdBook/format/configuration/general.html)
sets book language metadata; it does not manage translation freshness. Separate
language builds and a full language selector remain future work. Do not introduce
both Markdown and gettext as competing maintained translation sources.

The checklist below describes that broader future work. The pilot intentionally
ships without a reviewer, with the explicit warning above; the later reviewed-book
publication criteria do not describe its current status.

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
- [ ] Translate captions and alt text. While the app is English-only, keep its
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
  Include generated SVG labels, export artwork and launcher text where relevant.
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
  device-provided labels. Decide which new-project example names may be localized;
  never rename existing content when switching languages.

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
