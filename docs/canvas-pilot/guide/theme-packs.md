# Add a theme pack

Theme packs add palettes without rebuilding Canvas Pilot. They change the app,
viewport and source-coordinate helper colors; they do not change projects,
device transfers, imported pictures, fonts or interface size.

## Install and try the collections

Download a ZIP, leave it zipped, then open **Settings → Theme packs → Install theme
pack…**. Select a pack, then click one of its themes to try it immediately. It also
appears in **More themes…** and recent themes. The picker stays open while you
try alternatives. Selecting **High Contrast** twice returns to your last theme,
including an installed one.

Choose either collection, or install both:

- [Video-game themes](../../theme-packs/video-game-themes.zip): **6 themes** —
  Deep Rock Galactic, Detroit: Become Human, Dispatch, Helldivers, Ravenswatch
  and Vivid World.
- [Classic themes](../../theme-packs/classic-themes.zip): **11 themes** —
  Catppuccin Mocha, Dracula, GitHub Dark/Light, Gruvbox Dark, Monokai, Nord,
  One Dark, Solarized Dark/Light and Tokyo Night.

Each ZIP is one normal theme pack. Select **video-game-themes** or **classic-themes**
in the manager to try its palettes. Removal or replacement applies to that whole
collection. Individual downloads below remain available; installing both an
individual pack and its collection keeps separate copies.

### Video-game themes

| Pack | Palette | Editable manifest |
| --- | --- | --- |
| [Detroit: Become Human](../../theme-packs/detroit-become-human.zip) | Deep blue, pale text and cyan android-inspired accents | [manifest.json](../../theme-packs/detroit-become-human/manifest.json) |
| [Ravenswatch](../../theme-packs/ravenswatch.zip) | Midnight blue, ivory and crimson | [manifest.json](../../theme-packs/ravenswatch/manifest.json) |
| [Vivid World](../../theme-packs/vivid-world.zip) | Midnight navy, violet, pink and cyan crystal accents | [manifest.json](../../theme-packs/vivid-world/manifest.json) |
| [Deep Rock Galactic](../../theme-packs/deep-rock-galactic.zip) | Dark, gold/molten-orange accents | [manifest.json](../../theme-packs/deep-rock-galactic/manifest.json) |
| [Dispatch](../../theme-packs/dispatch.zip) | Dark, yellow/cyan accents | [manifest.json](../../theme-packs/dispatch/manifest.json) |
| [Helldivers](../../theme-packs/helldivers.zip) | Dark | [manifest.json](../../theme-packs/helldivers/manifest.json) |

### Classic themes

| Pack | Palette | Editable manifest |
| --- | --- | --- |
| [Tokyo Night](../../theme-packs/tokyo-night.zip) | Dark blue with cool cyan accents | [manifest.json](../../theme-packs/tokyo-night/manifest.json) |
| [Catppuccin Mocha](../../theme-packs/catppuccin.zip) | Soft charcoal and pastel accents | [manifest.json](../../theme-packs/catppuccin/manifest.json) |
| [Dracula](../../theme-packs/dracula.zip) | Dark | [manifest.json](../../theme-packs/dracula/manifest.json) |
| [GitHub Dark](../../theme-packs/github-dark.zip) | Dark | [manifest.json](../../theme-packs/github-dark/manifest.json) |
| [GitHub Light](../../theme-packs/github-light.zip) | Light | [manifest.json](../../theme-packs/github-light/manifest.json) |
| [Gruvbox Dark](../../theme-packs/gruvbox-dark.zip) | Dark | [manifest.json](../../theme-packs/gruvbox-dark/manifest.json) |
| [Monokai](../../theme-packs/monokai.zip) | Dark | [manifest.json](../../theme-packs/monokai/manifest.json) |
| [Nord](../../theme-packs/nord.zip) | Dark | [manifest.json](../../theme-packs/nord/manifest.json) |
| [One Dark](../../theme-packs/one-dark.zip) | Dark | [manifest.json](../../theme-packs/one-dark/manifest.json) |
| [Solarized Dark](../../theme-packs/solarized-dark.zip) | Dark | [manifest.json](../../theme-packs/solarized-dark/manifest.json) |
| [Solarized Light](../../theme-packs/solarized-light.zip) | Light | [manifest.json](../../theme-packs/solarized-light/manifest.json) |

Tokyo Night adapts the [upstream palette](https://github.com/folke/tokyonight.nvim),
and Catppuccin uses [Mocha colors](https://catppuccin.com/palette/). Their license
notices travel inside the ZIP and can be viewed under **Attribution and licenses**.
The Detroit, Vivid World, Ravenswatch, Dispatch, Helldivers and Deep Rock Galactic themes are
original, unofficial color interpretations;
they include no game artwork, fonts or code and are not endorsed by the games' owners.

### Moving from the old combined pack

The two collections replace the former `extra-themes.zip` download and have new
pack IDs. Install the desired collections, select a theme from one of them, then
remove **extra-themes** in Settings if you no longer need its duplicate entries.
Existing installations and saved choices are not renamed or deleted automatically.
Removing a pack while its theme is active falls back to Midra Dark. Standalone
packs, including older Minecraft installations, remain user-managed.

## Manage packs

The manager uses a compact, searchable pack list and shows the contents of only
one selected pack. Search by pack ID or theme name; **Clear** restores the full
list. Rows show the theme count and revision, with a dot marking packs currently
in use. Large collections scroll within the list instead of stretching Settings.

Select a pack to see its theme names, swatches, attribution and
**Remove pack…** action. The contents and license text scroll independently;
removal stays above them. On narrower windows, details appear below the list.
Newly installed or updated packs are selected automatically.
Search and selection are remembered during the session, separately from language
packs. Theme changes save immediately and survive restart.

Install a newer revision of the same pack to update it immediately. Equal or older
revisions ask before replacing the installed copy. Updating an active theme refreshes
its colors. Removing a pack asks for confirmation; if it contains the active theme,
Canvas Pilot returns to Midra Dark. The downloaded ZIP is kept for reinstalling.

If a pack disappears outside the app, Canvas Pilot keeps your saved choice but
uses Midra Dark and marks the theme unavailable. Reinstalling that pack restores
its colors. Invalid packs are reported and skipped; they do not prevent startup.

## Make your own

Start with one of the editable manifests above. Change the pack `id`, each theme's
`id` and `name`, and its `colors`; keep the required fields. Choose `light: true`
for a light palette or `false` for a dark one. Every color is opaque `#RRGGBB`.
Put the file at the ZIP root as `manifest.json`, with no folder or other files.
A pack may contain up to 16 themes. Increase its positive `revision` for updates.

Use your own pack ID when distributing a variant. Retain relevant upstream notices
when adapting licensed palettes. Check primary and secondary text on every surface,
selection fills and both checkerboard tile colors. Light/dark mode does not fix
poorly contrasted colors automatically.

The [theme-pack contract](../../interfaces/theme-packs.md) lists all palette roles,
validation limits, settings compatibility and round-trip behavior. Repository
contributors regenerate the individual ZIPs and both collections with `python3 scripts/build-theme-packs.py`.
