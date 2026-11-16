# Install and update Canvas Pilot

[Introduction](../introduction.md) · [Your first layout](quickstart.md) · [Advanced guide](overview.md)

## Download the application — recommended

**[Open the Canvas Pilot releases page →](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/releases)**

Download the ready-to-run application for your computer. You do not need to
install Rust, Cargo or Git.

1. Open the **newest stable release** and find its **Assets** list.
2. Download the file starting with **`canvas-pilot-portable-`** that matches your
   computer in the table below. Choose the portable archive, not **Source code**.
3. Extract the entire archive into a folder you want to keep.
4. Open the application inside that folder. On Windows, double-click
   **`canvas-pilot.exe`**. Then follow [your first layout](quickstart.md).

> **Before the first release:** no releases were published when checked on
> 2026-11-16. If the release page is empty, there is nothing to download yet.
> The [development build](#run-the-development-version) is available for people
> comfortable compiling software; otherwise, return when a release is published.

### Which file should I download?

`vX.Y.Z` below stands for the version shown on the release page.

| Your computer | Download this portable archive | Open after extraction |
| --- | --- | --- |
| **Windows**, Intel or AMD 64-bit | `canvas-pilot-portable-x86_64-pc-windows-gnu-vX.Y.Z.zip` | `canvas-pilot.exe` |
| **Linux**, Intel or AMD 64-bit | `canvas-pilot-portable-x86_64-unknown-linux-gnu-vX.Y.Z.tgz` | `canvas-pilot` from your desktop file manager |
| **Mac with Apple Silicon** (M-series), macOS 13 or later | `canvas-pilot-portable-aarch64-apple-darwin-vX.Y.Z.zip` | `canvas-pilot`; optional app launcher below |
| **Mac with an Intel processor**, macOS 13 or later | `canvas-pilot-portable-x86_64-apple-darwin-vX.Y.Z.zip` | `canvas-pilot`; optional app launcher below |

On a Mac, **Apple menu → About This Mac** shows whether it has an Apple chip or
an Intel processor. Linux downloads target glibc systems; Windows ARM, Linux ARM
and Alpine/musl are not currently packaged.

Keep the extracted folder intact: portable archives include icons, launcher
helpers, license information and a quick-start file. The executable is named
`canvas-pilot.exe` on Windows, not `release.exe`. Separate archives without
`portable` in their name are intended for the Cargo installation method below.

If your desktop asks for permission to run the downloaded application, follow
its normal approval process. Some Linux file managers require enabling execution
in the file's properties or launching it from a terminal. Linux also needs the
application's desktop/OpenGL runtime libraries. Check the release notes for any
platform-specific requirements.

### Desktop shortcut or macOS launcher

On Windows, right-click `canvas-pilot.exe` to create a shortcut using your
desktop’s shortcut options.

<details>
<summary>Optional Linux/macOS launcher setup (uses the command line)</summary>

Linux/macOS portable
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

</details>

## Command-line installation — optional

The methods below are for people who already use Cargo or want to compile the
application. Most readers can use the portable download above.

Choose the newest stable release on the
[release page](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/releases).
Replace `vX.Y.Z` in every command with its actual tag, including the `v` prefix.
The `main` branch may contain changes that are not in that release.

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
- **Do I need extra converters?** FFprobe is not needed. Illustrator exchange
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
