# Add a language pack

Language packs add interface translations without rebuilding Canvas Pilot. English
remains the reference language, and missing messages fall back to English. Packs do
not translate project names, device responses or exported geometry.

## Try the example

1. Download [language-pack.zip](../../language-packs/language-pack.zip).
2. In Canvas Pilot, open **Menu → Open all settings…**.
3. Choose **Language packs**, then click **Install language pack…** and select the ZIP. Leave it zipped.
4. Open **Language / Langue**. The list now includes **Українська — Ukrainian**
   and **भोजपुरी — Bhojpuri**, below the built-in languages; scroll if needed.
5. Select one and confirm within 20 seconds. Cancellation and the countdown still
   use your previous language. Confirmed choices survive restart.

The example provides 39 messages per language, including Settings, common controls,
confirmation and a message with a substitution. It is an intentionally partial,
unreviewed machine translation for teaching pack creation. The remaining interface
stays English. Bhojpuri is identified as `bho`, separately from Hindi (`hi`), following
[the ISO language-code list](https://www.loc.gov/standards/iso639-2/php/code_list-utf8.php).
It demonstrates the same Devanagari font support used by Hindi.

These packs reuse the fonts already bundled with Canvas Pilot. Version 1 and 2 packs
contain translations only: they neither install fonts nor reduce the current
application download size. Ukrainian uses the existing Cyrillic coverage.

Installed packs appear under bold, collapsible headings showing the pack ID and
revision. Click a heading or its arrow to hide or show the indented language list
and **Remove pack…** action. Packs start expanded.

## Make your own pack

Download these three editable example files into the same folder:

- [manifest.json](../../language-packs/example/manifest.json)
- [uk.json](../../language-packs/example/uk.json)
- [bho.json](../../language-packs/example/bho.json)

The manifest describes the pack and its languages:

```json
{
  "version": 2,
  "id": "example-uk-bho",
  "revision": 1,
  "languages": [
    {"locale": "uk", "name": "Ukrainian", "native_name": "Українська"},
    {"locale": "bho", "name": "Bhojpuri", "native_name": "भोजपुरी"}
  ]
}
```

Each language has a UTF-8 JSON file named after its `locale`. Use a unique lowercase
language identifier, such as `uk`, and a unique pack `id`. Display names are plain
text; no markup is supported. For another language, change the identifiers, names
and filenames together. New languages do not need a Rust enum entry or a rebuild.

Start with a few messages, then add more from the
[English source catalog](../../../crates/canvas-pilot/locales/en.json). Copy keys
exactly and translate only their values. Omit messages you have not translated.
Do not copy the whole English catalog merely to appear complete.

```json
{
  "ui.settings": "Налаштування",
  "ui.close": "Закрити",
  "theme.current": "Поточна тема: %{name}"
}
```

Keep placeholders such as `%{name}` and `%{seconds}` unchanged; their position in the
sentence may change. JSON requires double quotes and no trailing commas. Save as
UTF-8. The importer rejects unknown keys, empty values and mismatched placeholders.
Built-in languages cannot be replaced by packs.

From the folder containing the three JSON files, create the ZIP with Python:

```sh
python3 -m zipfile -c language-pack.zip manifest.json uk.json bho.json
```

Or use an archive tool: select the files themselves, not their enclosing folder.
The ZIP must contain only `manifest.json` and the declared language JSON files at
its root. Install it through Settings, switch language, and check long labels,
confirmation, fallback messages and text rendering. Ask fluent speakers to review
translations before presenting a pack as ready for everyday use.

## Storage, updates and removal

The installer copies the validated ZIP to `language-packs/<pack-id>.zip` beside
`settings.json`. To find that directory, use **Settings → Files & settings** and the
settings-file location displayed there. Keep installed packs there across app
updates. You can also copy a ZIP into this directory while the app is closed;
it is loaded on the next launch.

Use **Settings → Language packs** to see installed packs, their revisions and included languages.
**Remove pack…** asks for confirmation and removes every language bundled in that
pack immediately. Removing the active language switches to English and saves that
choice. Built-in languages cannot be removed. Keep the source ZIP if you want to
reinstall later; removal does not delete your original download.

To update, choose **Install language pack…** and select the new ZIP:

- Keep the same pack `id` and increase `revision` for a newer release. A higher
  revision replaces the installed pack immediately, including the currently selected
  translations. No restart or manual file removal is needed.
- An equal revision (a sidegrade, including the same ZIP) or lower revision asks
  before overwriting. Cancel leaves the installed pack unchanged.
- Legacy format-1 packs have revision 0. A format-2 revision 1 pack updates them
  directly; installing a legacy pack over another installed pack requires confirmation.
- Languages belonging to a different installed pack cannot be overwritten. Remove
  that pack explicitly first. An update can add/remove languages within its own pack;
  if it removes the active language, Canvas Pilot switches to English.

`version` describes the manifest format and stays **2**; `revision` is a positive
integer that you increase for each release (1, 2, 3, …). It is independent of the app
version and file timestamps, so timezone or clock changes do not affect ordering.

If a pack is removed outside the app, the saved language remains marked unavailable
and renders in English until reinstalled. System default continues to resolve
built-in languages; select installed languages explicitly.

Invalid packs are rejected without installing any of their languages. Startup
skips invalid packs and reports errors. Packs are limited to 2 MiB compressed,
4 MiB expanded and 16 languages each; at most 32 ZIPs are loaded. They do not execute
code, load remote content, or change device settings. The full format is documented
in the [language-pack contract](../../interfaces/language-packs.md).
