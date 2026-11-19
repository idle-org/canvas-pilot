# Canvas Pilot application settings, version 16

Canvas Pilot was formerly named Midra Layout. The rename preserves existing wire-format identifiers, `.midra.json` projects, settings/recovery locations and `MIDRA_VECTORCRAFT_CLI` configuration. No schema version changes are needed. New exports use Canvas Pilot in human-readable titles; legacy files remain readable.

Implementation: `crates/canvas-pilot/src/settings.rs`. This per-user JSON file is
independent of the [project JSON and SVG contracts](canvas-pilot.md).
[Complete default example](canvas-pilot-settings-v16.example.json).
Version 1 files migrate automatically: the saved theme becomes the only entry in
`recent_themes`, with all other preferences preserved. The [version 1 example](canvas-pilot-settings-v1.example.json)
remains a migration fixture. Version 2 files then migrate by adding disabled
snapping with all target types enabled. The [version 2 example](canvas-pilot-settings-v2.example.json)
remains a migration fixture too. Version 3 files migrate by adding an empty `midra_host`; the [version 3 example](canvas-pilot-settings-v3.example.json) remains a migration fixture. Version 4 files migrate by adding `allow_sending_to_program: false`; the [version 4 example](canvas-pilot-settings-v4.example.json) remains a migration fixture. Version 5 files migrate with `autoconnect_at_startup: false`; the [version 5 example](canvas-pilot-settings-v5.example.json) remains a migration fixture. Version 6 files migrate with the overlay grid visible, snapping to it disabled, and `grid_divisions: [8, 6]`; the [version 6 example](canvas-pilot-settings-v6.example.json) remains a migration fixture. Version 7 migrates with an empty `recent_midra_hosts` list; the [version 7 example](canvas-pilot-settings-v7.example.json) remains a migration fixture. Version 8 migrates with `language: "system"`; the [version 8 example](canvas-pilot-settings-v8.example.json) remains a migration fixture. Version 9 migrates without changing any preferences; its `system`, `en` and `fr` values remain valid. The [version 9 example](canvas-pilot-settings-v9.example.json) remains a migration fixture. Version 10 adds language choices. Version 11 adds `ui_scale` and `text_scale`; v1–v10 settings migrate to 100% interface scale with separate text scaling disabled, preserving all existing preferences. The [version 10 example](canvas-pilot-settings-v10.example.json) remains a migration fixture. Version 12 adds `pt-PT`, `ne`, `th`, `hi` and `tok`; v1–v11 settings migrate without changing existing preferences. Values outside each old version’s language set are rejected before migration. The [version 11 example](canvas-pilot-settings-v11.example.json) remains a migration fixture. Version 13 adds installed language choices; v12 preferences migrate unchanged. The [version 12 example](canvas-pilot-settings-v12.example.json) remains a fixture. Version 14 adds required `snapping.mask_edges` (false). Versions 1–13 migrate with mask snapping disabled; supplying the field under an older version is rejected. The [version 13 example](canvas-pilot-settings-v13.example.json) remains a migration fixture. Version 15 adds installed theme choices and migrates v1–v14 without changing other preferences. The [version 14 example](canvas-pilot-settings-v14.example.json) remains a migration fixture. Version 16 adds required `snapping.force_pixels` (false). Versions 1–15 migrate with forced whole-pixel snapping disabled; supplying the field under an older version is rejected. The [version 15 example](canvas-pilot-settings-v15.example.json) remains a migration fixture. Saving writes version 16, which older applications reject.

The `directories` 6 crate resolves `ProjectDirs::from("org", "beffroi", "midra-layout")`.
The filename is `settings.json` beneath `config_dir()`:

| Platform | Location |
| --- | --- |
| Linux | `$XDG_CONFIG_HOME/midra-layout/settings.json`, otherwise `~/.config/midra-layout/settings.json` |
| Windows | `%APPDATA%\beffroi\midra-layout\config\settings.json` (Roaming AppData) |
| macOS | `~/Library/Application Support/org.beffroi.midra-layout/settings.json` |

UTF-8, pretty-printed JSON with a trailing newline; maximum input size 64 KiB.
Every field is required, unknown fields are rejected, and unknown versions or
unknown theme names are rejected. There are no numeric coordinates or units.

| Field | Values / default |
| --- | --- |
| `version` | Integer `16` |
| `ui_scale` | Finite number, 0.5–4.0 inclusive; default 1.0. egui interface zoom relative to the OS display scale. |
| `text_scale` | `null` (default, disabled) or finite number 0.5–2.0 inclusive. Additional standard-widget text multiplier. |
| `language` | `"system"` (default), `"en"`, `"fr"`, `"ja"`, `"zh-CN"`, `"es"`, `"grc"`, `"eo"`, `"pt-PT"`, `"ne"`, `"th"`, `"hi"`, `"tok"`, or `{"pack":"uk"}` for an installed language. Pack identifiers use 1–63 lowercase ASCII letters, digits and hyphens, without a leading/trailing hyphen. Unknown built-in string values are rejected. |
| `autoconnect_at_startup` | Boolean; default false. Try one background connection at application startup if the saved address is nonblank. |
| `allow_sending_to_program` | Boolean; default false. Enables the Program destination in the exporter. Turning it off restores Preview; applies to quick Apply too. |
| `midra_host` | Device IP address text with optional TCP port (e.g. `192.168.10.146:3000`, `[::1]:3000`; default port 80; root HTTP URLs also accepted); default `""`. Saved as typed, including incomplete drafts; validated when connecting. |
| `theme` | Built-in theme ID below or `{"pack":"pack-id/theme-id"}`; default `"midra"` |
| `recent_themes` | 1–3 unique built-in IDs or pack references, newest first; first must equal `theme`; default `["midra"]` |
| `show_toolbar_icon_names` | Boolean; default false |
| `keep_aspect_ratio` | Legacy compatibility value; current projects store the required resize lock on each layer and new layers default to true |
| `show_source_pattern` | Boolean; default false. Shows source patterns on other layers only during crop editing; normal editing always hides patterns. |
| `snapping` | Object with required boolean fields `enabled` (false), `pixels` (true), `force_pixels` (false), `screen_edges` (true; screen edges plus 25%, 50% and 75% guides on both axes), `layer_edges` (true), `mask_edges` (false), `grid` (false), `show_grid` (true), and `grid_divisions` ([8, 6]); unknown fields rejected |

Scale preferences apply on startup. Slider drafts apply and save on pointer
release; typed values wait until text entry ends. UI controls
use 5-percentage-point increments; the file accepts any finite in-range multiplier.
Separate text scaling changes standard widget text styles without compounding on
repeated application. Explicit-size artwork and labels follow interface zoom only.
Neither scale changes project dimensions, export text, clipboard bytes or device
coordinates. Restore defaults and Reset interface size restore 1.0 / null.
`high_contrast` is the new theme identifier, also valid in `recent_themes`.

Language is a workstation preference, never a project field or exchange value.
Changing **Settings → Language / Langue** previews immediately and saves after
confirmation, without restarting or reinstalling the project. It preserves selections, undo, numeric
entry drafts, device links and staged proposals, and never connects or sends.
Resetting defaults restores System default. System language is resolved at startup
and when explicitly selected; external OS language changes are not watched live.
System matching uses the language subtag for English, French, Japanese, Spanish,
Ancient Greek (`grc`, not modern Greek `el`), Esperanto, Nepali, Thai, Hindi and Toki Pona. Portuguese uses `pt-PT`; `pt` and non-Brazilian regional Portuguese locales resolve to it, while `pt-BR` falls back to English rather than silently substituting a regional translation. Chinese without a
Traditional marker uses Simplified Chinese; `Hant`, `TW`, `HK`, and `MO` fall back
to English. Unsupported or unavailable locales also use English. Regional and
encoding suffixes such as `ja_JP.UTF-8` are accepted. Explicit `zh-CN` always selects
Simplified Chinese. Catalog fallback is English. Decimal input and serialized numeric values retain
their existing dot-decimal behavior in every language. Names supplied by documents,
users or devices, protocol identifiers, fit wire values and theme names are unchanged.

Loaded once at startup before the editor appears. Explicit Options changes save
the current preferences and theme history immediately. The directory is created lazily;
missing files do not trigger a write until the user changes an option. Writes
use a temporary file in the same directory, file sync and atomic replacement.
Multiple application instances use last-successful-write wins; no merging or
live reloading is performed. Selecting a theme moves it to the front, removes any
previous occurrence, and trims the list to three. Starting the app does not reorder
the restored history. Opening or dismissing the full picker changes nothing.

Project JSON v1/v2 remains unchanged. Opening a project can restore its embedded
caption, aspect-ratio and source-preview preferences for that document; it never
writes application settings. Theme remains global. Editing an option afterward
saves all currently active preferences as the next startup defaults. Snapping remains
an application preference and is not read from or written into project JSON.

Invalid/unreadable settings fall back to defaults, report the failure in Options,
and disable writes for that run to preserve the original file. Repair/move the
file and restart to recover. A write failure retains in-memory preferences and
the previous destination where atomic publication failed; Options offers a retry.
Failure to locate a configuration directory also disables persistence with an
Options error. Tests use temporary directories and never touch user settings.

The device address is saved immediately when edited in **Midra connection…**. Startup attempts a read-only connection only when `autoconnect_at_startup` is enabled and the saved address is nonblank. It never sends layer settings. Opening a project preserves the current connection and address, clears import choices, and loads the project’s saved layer destination links. Connection status and snapshots are not persisted. Layer destination links are saved in project JSON, not application settings. Restore default settings disables autoconnect and leaves the connection address unchanged; edit it in the connection window.

No layer geometry, source crops, camera state, project paths, image bytes or clipboard contents are stored. No settings are exported to SVG.
Breaking or additive schema changes under this strict contract require a version
bump, updated documentation/examples/tests, and either an explicit migration or
an actionable unsupported-version error.

## Bundled theme catalog

All 18 CSS themes in `SOTV_Websites/sotv/song_database/styles/themes/` are
represented, alongside Midra Dark. Palettes are bundled in `src/theme.rs`:
no sibling checkout or CSS interpreter is required at build or runtime. Corporate
uses the website's base defaults. The existing three palettes retain their
hand-tuned viewport colors; other palettes map body/panel/sidebar/tab surfaces,
main/subtle text, button accent and border tokens into the editor. Action fills
blend accent and panel; light-theme text/accent colors are darkened for contrast.
The picker includes both light and dark themes with color swatches.

| Theme | Settings ID | Source CSS |
| --- | --- | --- |
| Midra Dark | `midra` | Original editor palette |
| High Contrast | `high_contrast` | Original accessibility palette; black surfaces, bright text, stronger borders |
| Corporate Light | `corporate_light` | `theme-corporate.css` and `../base/variables.css` |
| Foxified Dark | `foxified_dark` | `theme-foxified-dark.css` |
| Bluecore | `bluecore` | `theme-bluecore.css` |
| Dupont Monsanto | `dupont_monsanto` | `theme-dupont-monsanto.css` |
| Eden Green | `eden_green` | `theme-eden-green.css` |
| Foxified Light | `foxified_light` | `theme-foxified-light.css` |
| HCI | `hci` | `theme-hci.css` |
| Iron Docks | `iron_docks` | `theme-iron-docks.css` |
| Peaceframe | `peaceframe` | `theme-peaceframe.css` |
| Rebel | `rebel` | `theme-rebel.css` |
| Sirius Mind | `sirius_mind` | `theme-sirius-mind.css` |
| Solarized | `solarized` | `theme-solarized.css` |
| Sons Of Banard | `sons_of_banard` | `theme-sons-of-banard.css` |
| Storm Pilot | `storm_pilot` | `theme-storm-pilot.css` |
| Tessier Ashpol | `tessier_ashpol` | `theme-tessier-ashpol.css` |
| USTF | `ustf` | `theme-ustf.css` |
| Verdant Rebel | `verdant_rebel` | `theme-verdant-rebel.css` |
| Weicat Industrial | `weicat_industrial` | `theme-weicat-industrial.css` |

Language selection previews the new UI for 20 seconds. Only confirmation persists
the new `language` value. Cancellation, Escape or timeout restores the previous
choice. While a preview is pending, all settings writes retain the previous
confirmed language, so quitting or crashing cannot persist an accidental selection.
The cancel button uses the previously displayed locale, including resolved System
default. Preview state and its deadline are not serialized; preview state adds no serialized fields.

The Settings window and quick menus share the same settings. Changes apply
and save immediately, except for scaling interaction and language confirmation described above; closing
the window requires no general Apply step. Restore default
preferences resets appearance and editing values, including theme history and snapping, without
changing layer geometry or pending position/size text fields. Snapping uses a
fixed six-logical-pixel proximity radius converted by viewport scale. It affects
layer movement, resize handles and pivot placement; numeric entry, nudges and crop-content gestures
bypass it. Resize snapping preserves anchors and proportions and shows guides only
when the resulting edge actually reaches its target.
`layer_edges` independently controls outer borders as movement anchors and
stationary targets, including mirrored/opposite resize contact. Disabling it does
not change `mask_edges`. Centers, pixel rounding and directly dragged resize handles
retain their existing behavior. The serialized field and default remain unchanged.
`mask_edges` adds the nonempty intersection of mask reveal, layer aperture and
fitted picture as edge and center targets. Hidden layers are skipped, locked layers included.
Movement also adds selected mask edges and centers as anchors; resize handles remain on the
outer rectangle, with masked edges and centers available on stationary targets.
Moving centers align to screen interior guides, overlay grid lines and stationary
mask centers. This extends the existing `mask_edges` flag without changing its
serialized name, default or schema version. Mirrored
contact uses outer edges. The option defaults to false.

`force_pixels` makes integer pixel coordinates take priority over other snap targets for moves, resize handles and pivots. It defaults to false because rounding every resized edge can slightly approximate a locked aspect ratio. Shift+Space temporarily enables this mode during a gesture, even when regular snapping is disabled; it takes precedence over Shift’s 10 px grid. Space without Shift retains temporary canvas panning.

The session-only toolbar grid tools select 10 or 100 screen pixels (mutually exclusive). Shift temporarily selects the 10 px grid; Ctrl+Shift selects 100 px. If that same grid tool is selected, the matching shortcut temporarily disables it instead. Alt overrides both grids and temporarily inverts the saved snapping-enabled value (off becomes on; on becomes off), including on release. The chosen pixel/screen/layer-edge targets still apply. These modifiers do not change the saved options or settings version.

**Settings → Midra integration → Allow sending to Program** saves immediately. Restore default settings disables it. Projects do not override this application preference; the Preview/Program destination itself remains session-only.

**Try autoconnecting at startup** is in Settings → Midra integration. It saves immediately but applies on the next launch; toggling it does not initiate a connection. One background attempt uses the regular connection timeout and error handling, with no automatic retry loop. Failures leave the app offline and are reported in Midra connection. Project loading does not trigger an attempt.

The session-only Pivot tool can combine with either grid. Ctrl without Shift temporarily inverts Pivot; Ctrl+Shift does not affect it. Tool selections are not persisted in application settings or projects; they survive project loads and selection changes within a session and start off on launch. Pivot positions belong to each layer and are saved in project v9, not application settings.

Select/Move/Pan/Zoom navigation tool choice is also session-only and survives project loads.
It starts at Select on launch; V/M/H/Z switch tools only outside text entry. Camera
zoom/pan remain part of the project, not application settings.

Pivot placement also has nine built-in local anchors (corners, side midpoints,
center), active independently of `snapping.enabled` within the same six-logical-pixel
radius. Anchors take priority over active grids and configured edge/pixel snapping.
Alt bypasses these anchors and grids while inverting configured snapping as usual.
This adds no settings fields; pivot positions are saved per layer in projects.

Recovery autosave is a separate `last_project.json` beside settings, not a settings field. See [recovery format](canvas-pilot-recovery.md).

## Overlay grid

`snapping.grid_divisions` contains integer column and row counts, each 1–64.
Grid lines are at `output_width * i / columns` and `output_height * j / rows`,
including output boundaries. Calculations retain fractional screen pixels (no
rounding); the same coordinates drive rendering, movement, resize and pivot
snapping. `show_grid` controls visibility independently of `grid`, which adds these
lines to the magnet targets when `snapping.enabled` is active. Movement snaps
layer edges and centers; resizing uses the existing anchor/ratio rules. Existing
six-logical-pixel proximity tolerance, release behavior and modifier priorities
remain unchanged; explicit 10/100 px grids take precedence. Crop-content resizing
is not snapped. Invalid subdivision counts are rejected on settings load.

The Grid toolbar icon opens visibility, snapping and subdivision controls; they
also appear in Settings → Editing as a separate Overlay grid section. The overlay is drawn above example
artwork and reference pictures, below editable layers, clipped to the output.
It is an editor guide, omitted from all project/export artwork. Preferences save
immediately and survive project changes; v1–v6 migration does not alter geometry.

Custom ports use the existing `midra_host` string without a schema change. The
port is shared by HTTP reads/captures and WebSocket writes, including startup
autoconnection. Hostnames, URL schemes, paths and port 0 are rejected on use.

## Recent connections

`recent_midra_hosts` is an array of zero to four unique canonical IP:port strings,
most recently connected first (for example `["192.168.10.146:3000", "192.168.10.143:80"]`).
IPv6 entries use brackets. Successful reads or verified sends promote an endpoint;
failed or cancelled attempts and typed drafts do not enter the list. Default port
80 is explicit so equivalent addresses deduplicate. Invalid addresses, noncanonical
strings, duplicates and more than four entries are rejected.

The connection window lists these under **Recent machines…**. Choosing one sets
the saved address, clears old device state/staged input edits, and starts a read-only
connection. The picker is disabled during a pending operation. History belongs
to application settings, not project files; migration starts empty because older
settings do not record whether their last address connected successfully.

Installed language choices are described by the [language-pack contract](language-packs.md).
They are allowed only in v13; older versions reject object-valued language fields.
Unavailable packs preserve the identifier and render English rather than discarding
preferences. Installation re-applies a saved missing-pack choice.

## Installed themes

Theme packs use `{"pack":"pack-id/theme-id"}` in `theme` and `recent_themes`.
Identifiers follow the [theme-pack contract](theme-packs.md). Built-in string IDs
remain unchanged; packed choices in settings versions 1–14 are rejected. Missing
packs retain the preference with Midra Dark rendering and an unavailable label.
Explicit removal or updates that remove the active theme save Midra Dark instead.
Re-clicking active High Contrast restores the preceding recent theme, including a
packed theme; applying startup settings never toggles High Contrast off.
