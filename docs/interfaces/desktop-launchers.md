# Desktop launcher output

`scripts/create-desktop-launcher.py` emits standard operating-system launcher
files for Canvas Pilot. It does not save application data or
import user projects. There is no custom schema version or round-trip importer;
compatibility follows Desktop Entry and macOS bundle conventions.

Linux/BSD output under `--output`:

- `applications/<tool>.desktop`: UTF-8 `[Desktop Entry]`, `Type=Application`,
  human-readable `Name`, quoted absolute `Exec` with desktop field-code escaping,
  absolute `Icon`, `Terminal=false`, `Categories=AudioVideo;Utility;`, and
  `StartupWMClass=<tool>`. The filename matches the egui Wayland app ID.
- `icons/hicolor/256x256/apps/<tool>.png`: 256 × 256 RGBA application artwork.

macOS output is `<display name>.app/Contents/`:

- `Info.plist`: XML plist, `CFBundleName` and `CFBundleDisplayName` contain the
  display name; `CFBundleIdentifier=com.beffroi.<tool>`;
  `CFBundlePackageType=APPL`; `CFBundleExecutable=<tool>`;
  `CFBundleIconFile=app-icon.icns`; `CFBundleVersion=1` identifies the launcher;
  `NSHighResolutionCapable=true`.
- `MacOS/<tool>`: copied executable with mode 0755.
- `Resources/LICENSES`: copied adjacent notices, when present. Symlinks in the
  notices tree are refused.
- `Resources/app-icon.icns`: PNG-backed `ic07`, `ic08`, `ic09`, `ic10` entries,
  at 128, 256, 512 and 1024 pixels respectively.

The binary must resolve to an existing executable regular file. Existing
launcher/icon destinations are refused, including symlinks. Linux/BSD paths are absolute and require the original installation to stay in
place. macOS copies are independent of their source location; recreate the bundle
to update its executables. Fonts stay embedded in the Rust binaries. The helper does not bypass macOS
Gatekeeper, sign bundles, register file associations, or make a binary for
another CPU/OS runnable. No timing values, numeric precision or user-data
migration applies to this format.
