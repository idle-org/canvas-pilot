# Menus and preferences

[Canvas Pilot](../introduction.md) · [Guide contents](overview.md) · [Starter files](../../templates/canvas-pilot/catalog.md)

## Application menu

The header’s **Menu** brings the application’s commands together:

- **Project**: open/save JSON and export SVG.
- **Import**: SVG, clipboard layer values, and reference images.
- **Edit**: undo/redo, copy values, select all editable layers, or deselect.
- **View**: fit screen/selection, 1:1 pixels, output format, and reference settings.
- **Layers**: selected-layer properties, crop/reset, alignment, shared resizing,
  lock/hide, and per-layer selection/visibility/lock controls.
- **Options**: resize aspect ratio and the source-coordinate preview.

The toolbar, cog and right-click shortcuts use the same commands and preferences.
Detailed numeric controls remain in their existing properties dialogs.

Enable **Options → Show toolbar icon names** (also available from the toolbar cog)
to display captions beneath the icons. It is off by default and saved with the
project; older projects open with captions off.

### Interface language

Open **Menu → Open all settings…** and use **Language / Langue** at the top.
Choose **System default**, **English** or **Français**. The change takes effect
immediately, saves automatically and survives restarting. In French, the route is
**Menu → Ouvrir tous les réglages…**; language names stay recognizable in either UI.

System default uses French for French regional locales and English otherwise.
French is a translation preview; some advanced controls and technical diagnostics
remain in English. The book's [French quick start](fr/quickstart.md) remains an
unverified machine translation. Screenshots and its quoted commands use English;
select English to follow those labels exactly.

Changing language preserves the open project, pending edits and device connections.
It does not rename layers, change file contents or send device commands. Numeric
fields continue to use a decimal point in both languages. Projects do not carry
language preferences to another workstation.

### Application themes

**Menu → Options → Theme** (also in the toolbar cog) shows up to three recently
used themes, newest first. **More themes…** opens the complete picker with color
swatches: Midra Dark plus all 18 themes from the SOTV song-database theme folder.
A fresh installation lists only Midra Dark until other themes are selected.
Selecting a theme updates the UI and viewport immediately and saves the recent
history across restarts. Reusing a theme moves it to the top without duplicates.
Closing the picker without choosing leaves the theme and history unchanged.

The viewport, grid, layer boxes and source test patterns use the selected palette;
selected layers remain gold (darker gold for contrast in light themes). Imported
reference images retain their colors. Theme and history survive opening projects
and are not included in project JSON or geometry exports.

The SOTV palettes are bundled with the app from
`sotv/song_database/styles/themes/` in the sibling `SOTV_Websites` repository;
Corporate Light uses `styles/base/variables.css`. No sibling repository is needed
to build or run Canvas Pilot. See the [theme catalog and settings contract](../../interfaces/canvas-pilot-settings.md)
for names, source files, stable IDs and palette adaptations.

### Saved application options

Options save automatically when changed: theme, toolbar icon names, aspect-ratio
lock and source-coordinate preview. No separate Apply or Save is needed.
The `directories` crate selects the OS configuration directory. Canvas Pilot retains
the former Midra Layout location so existing preferences and recovery remain available:

- Linux: `$XDG_CONFIG_HOME/midra-layout/settings.json`, or
  `~/.config/midra-layout/settings.json` when XDG_CONFIG_HOME is unset.
- Windows: `%APPDATA%\beffroi\midra-layout\config\settings.json`.
- macOS: `~/Library/Application Support/org.beffroi.midra-layout/settings.json`.

Hover **Options save automatically** in Options to see the actual path. These are
startup defaults. Existing project files still restore their saved aspect-ratio,
caption and source-preview settings for the open document, without rewriting the
application defaults. A subsequent explicit Options change saves the current set
of options as the new defaults. Theme always remains an application preference.

Missing settings use defaults; the file is created on the first option change.
Unreadable, malformed or unsupported settings use defaults and show an Options
error; the original file is preserved and saving is disabled until the file is
repaired or moved and the application restarted. Write failures keep options
active in memory and offer **Retry saving options**. No project data, images or
clipboard contents are stored here. The [settings interface](../../interfaces/canvas-pilot-settings.md)
documents the file and compatibility rules.

**Keep aspect ratio when resizing** is also available below the position/size
fields in the selected layer's right panel and in the layer
right-click menu (including multiple selections). These controls share the
Options setting and save changes automatically. They do not apply or discard
pending size/position edits. Crop zoom remains proportional.

<!-- Screenshot recipe: ../images/capture.md#advanced-guide-captures -->
![Settings dialog with Appearance selected and editing, references, files and device categories](../images/09-guide-settings.png)

*Settings are grouped by purpose; appearance changes apply immediately. The header still uses the former Midra Layout name.*

## Settings window

Open **Menu → Open all settings…**, **Options → Open all settings…**, the toolbar cog's
**Open all settings…** entry, or **Ctrl/Cmd+,**. The window groups the available controls:

- **Appearance:** themes, recent choices, icon captions and source-coordinate preview.
- **Editing:** aspect-ratio lock, movement snapping and a keyboard/mouse reference.
- **Reference images:** import and access to the current project's image settings.
- **Files & settings:** the settings-file path, copy-path action and restore defaults.
- **Midra integration:** current offline status and compatibility limitations.

Changes apply and save immediately, sharing state with quick menus and inspector
shortcuts. Close simply dismisses the window. Restore defaults resets preferences
without altering geometry or pending position/size edits. Device controls are not
presented as implemented until their mapping and transport are available.

### Movement snapping

The viewport right-click menu and quick Options menu both include **Snapping**:
enable movement/resizing snapping and choose pixel, screen or visible-layer edges.
These are the same automatically saved controls as Settings → Editing.

Enable **Settings → Editing → Snap movement & resizing** and choose pixel,
screen-edge and/or other-visible-layer-edge targets. Nearby edges show a guide;
the proximity radius stays six logical pixels at any zoom. A selection moves as
a rigid group. Hidden layers are ignored; visible locked layers can be targets.
Hold **Alt** to temporarily invert snapping: off when enabled, on when disabled. Turning snapping on/off alone never moves layers.
Exact numeric entry and nudges bypass proximity snapping; resize handles snap while preserving their anchors/ratios, and crop-content
handles retain their existing behavior. One drag remains one undo step, with Escape to cancel.
