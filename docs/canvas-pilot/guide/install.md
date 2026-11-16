# Install and update Canvas Pilot

[Introduction](../introduction.md) · [Your first layout](quickstart.md) · [Advanced guide](overview.md)

Choose the installation method that suits your computer:

| Method | Compiles on your computer? | Best for |
| --- | --- | --- |
| Download a release archive | No | Getting the desktop application with minimal setup |
| `cargo binstall` | No, with the options below | Installing a released binary into your Cargo tools folder |
| `cargo install` | Yes | Building a selected release from source |

**Release availability:** at the documentation check on 2026-11-16, this repository
had no published releases. The release-based instructions below become usable when
a release and its assets are published. Until then, use the development build at
the end of this page. An empty release page is not an installation failure.

## Choose the latest release

Open the [Canvas Pilot releases page](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/releases)
and choose the newest stable release, unless you intentionally want a prerelease.
Its tag identifies the exact source and binaries. Wherever the commands below say
`vX.Y.Z`, replace it with that release's actual tag, including the `v` prefix.
Do not type `vX.Y.Z` literally. The `main` branch is ongoing development and may
contain changes that are not in the latest release.

## Download a compiled application

No Rust compiler, Cargo or Git is needed for this method.

1. Open the release and expand its **Assets** list.
2. Download the **portable** archive matching your operating system and CPU.
3. Extract the entire archive to a permanent folder before launching the program.

| Computer | Target in the filename | Portable archive | Launch after extraction |
| --- | --- | --- | --- |
| Windows, Intel/AMD 64-bit | `x86_64-pc-windows-gnu` | `.zip` | Double-click `canvas-pilot.exe` |
| Linux, Intel/AMD 64-bit with glibc | `x86_64-unknown-linux-gnu` | `.tgz` | Run `./canvas-pilot` in a graphical desktop session |
| macOS 13+, Apple Silicon | `aarch64-apple-darwin` | `.zip` | Run `./canvas-pilot`, or create the launcher below |
| macOS 13+, Intel | `x86_64-apple-darwin` | `.zip` | Run `./canvas-pilot`, or create the launcher below |

For example, the naming pattern for a Windows portable download is
`canvas-pilot-portable-x86_64-pc-windows-gnu-vX.Y.Z.zip`. The executable inside is
`canvas-pilot.exe`, not `release.exe`. Gitea's automatically generated **Source code**
archives contain source, not a compiled application. The separate canonical `.tgz`
assets are used by cargo-binstall; portable archives also include icons, launcher
helpers, license information and a quick-start file.

If release notes list additional runtime requirements, follow those instructions.
Linux needs the application's desktop/OpenGL runtime libraries. Windows ARM,
Linux ARM and Alpine/musl are not in the current release target list.

### Desktop shortcut or macOS launcher

Windows users can create a shortcut to the extracted `.exe`. Linux/macOS portable
archives include an optional launcher helper requiring Python 3.11+. From the
extracted folder, run the command for your system:

```sh
# Linux
python3 scripts/create-desktop-launcher.py canvas-pilot --binary "$PWD/canvas-pilot" --output "$HOME/.local/share"
```

```sh
# macOS
python3 scripts/create-desktop-launcher.py canvas-pilot --platform macos --binary "$PWD/canvas-pilot" --output "$HOME/Applications"
```

The Linux shortcut points to the extracted folder; keep that folder in place.
The macOS launcher copies the executable and must be recreated after an update.
The helper refuses existing launchers and does not sign or notarize the app.
Follow your operating system's normal approval process for downloaded applications.

## Install a compiled release with cargo binstall

Install Cargo and [cargo-binstall](https://github.com/cargo-bins/cargo-binstall#installation)
first. Its upstream installation page offers prebuilt cargo-binstall downloads.
You also need Git for the tagged checkout below.

Canvas Pilot's release metadata lives in its Gitea repository. Use the manifest
from the selected release instead of relying on crates.io package discovery:

```sh
git clone --depth 1 --branch vX.Y.Z https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot.git canvas-pilot-release
cargo binstall canvas-pilot --manifest-path canvas-pilot-release/crates/canvas-pilot/Cargo.toml --strategies crate-meta-data
```

On Windows, use this installation command after the same clone command to select
the published GNU target explicitly:

```sh
cargo binstall canvas-pilot --manifest-path canvas-pilot-release/crates/canvas-pilot/Cargo.toml --strategies crate-meta-data --targets x86_64-pc-windows-gnu
```

The tagged manifest supplies the version, download URL and binary layout.
`--strategies crate-meta-data` restricts installation to the project's release
assets: if no suitable binary exists, it fails without starting a source build.
Review the proposed download and confirm installation. Then run:

```sh
canvas-pilot
```

This installs the executable, not the portable bundle's icons and launcher helper.
For those extras, use the portable download. See the upstream
[cargo-binstall manifest documentation](https://github.com/cargo-bins/cargo-binstall/blob/main/SUPPORT.md)
for how this installation method resolves release assets.

## Build a release with cargo install

This method compiles locally and can take substantial time and disk space. Use
prebuilt downloads or cargo-binstall on a computer with limited build capacity.
Install a current stable Rust toolchain and your platform's native compiler/linker;
Linux also needs the development libraries used by egui/rfd's desktop backend.
See [development guidance](../../development/README.md).

Select the actual tag of the latest published release, then run:

```sh
cargo install --git https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot.git --tag vX.Y.Z --locked canvas-pilot
canvas-pilot
```

The Git URL selects this project, and `--tag` selects the released source rather
than the development branch. The default build includes the optional vendor format
codecs. [Cargo's installation documentation](https://doc.rust-lang.org/cargo/commands/cargo-install.html)
explains source selection, install locations and replacement behavior.

## Update, uninstall and common questions

- **Update a portable installation:** close the app, extract the new release into
  a separate folder, then update your shortcut or recreate the launcher. Keep your
  saved projects. Do not overwrite a running executable.
- **Update a Cargo installation:** select the new release tag and repeat the chosen
  method. For cargo-binstall, use a fresh tagged checkout directory or deliberately
  update your existing release checkout. If an existing binary causes a conflict,
  review its origin before explicitly choosing `--force` to replace it.
- **Command not found:** ensure Cargo's executable directory is on `PATH`, normally
  `~/.cargo/bin` on Linux/macOS or `%USERPROFILE%\.cargo\bin` on Windows; a custom
  `CARGO_HOME` or installation root changes that location. Reopen the terminal.
- **No matching binary:** check the selected release's assets and target table.
  Cargo-binstall cannot download an asset that has not been published. It will not
  compile as a fallback with the command above.
- **Remove a Cargo installation:** run `cargo uninstall canvas-pilot`. For a portable
  installation, remove its extracted folder and shortcut. Neither instruction asks
  you to delete saved projects or manually remove application preferences.
- **Do I need a switcher?** No. Local editing and file exchange work offline.
- **Do I need extra converters?** FFprobe is not needed. Illustrator AI exchange
  requires VectorCraft; SVG exchange works without that optional converter.

## Run the development version

Before the first release, or when intentionally testing unreleased changes:

```sh
git clone https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot.git
cd canvas-pilot
cargo run --locked
```

This compiles the checkout and runs it locally; it does not install a released
binary. Once the app opens, continue with [your first layout](quickstart.md).
