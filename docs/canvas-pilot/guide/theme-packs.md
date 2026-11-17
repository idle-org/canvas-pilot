# Add a theme pack

Theme packs add palettes without rebuilding Canvas Pilot. They change the app,
viewport and source-coordinate helper colors; they do not change projects,
device transfers, imported pictures, fonts or interface size.

## Install and try the examples

Download a ZIP, leave it zipped, then open **Settings → Theme packs → Install theme
pack…**. Click a theme beneath its pack heading to try it immediately. It also
appears in **More themes…** and recent themes. The picker stays open while you
try alternatives. Selecting **High Contrast** twice returns to your last theme,
including an installed one.

| Pack | Palette | Editable manifest |
| --- | --- | --- |
| [Tokyo Night](../../theme-packs/tokyo-night.zip) | Dark blue with cool cyan accents | [manifest.json](../../theme-packs/tokyo-night/manifest.json) |
| [Catppuccin Mocha](../../theme-packs/catppuccin.zip) | Soft charcoal and pastel accents | [manifest.json](../../theme-packs/catppuccin/manifest.json) |
| [Detroit: Become Human](../../theme-packs/detroit-become-human.zip) | Deep blue, pale text and cyan android-inspired accents | [manifest.json](../../theme-packs/detroit-become-human/manifest.json) |
| [Minecraft Overworld](../../theme-packs/minecraft.zip) | Earth and stone surfaces with grass-green accents | [manifest.json](../../theme-packs/minecraft/manifest.json) |

Tokyo Night adapts the [upstream palette](https://github.com/folke/tokyonight.nvim),
and Catppuccin uses [Mocha colors](https://catppuccin.com/palette/). Their license
notices travel inside the ZIP and can be viewed under **Attribution and licenses**.
The Detroit and Minecraft themes are original, unofficial color interpretations;
they include no game artwork, fonts or code and are not endorsed by the games' owners.

## Manage packs

Each pack has a bold, collapsible heading with its ID and revision. Expand it to
see theme swatches, select a theme, read its attribution or choose **Remove pack…**.
Packs start expanded. Theme changes save immediately and survive restart.

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
contributors regenerate the four ZIPs with `python3 scripts/build-theme-packs.py`.
