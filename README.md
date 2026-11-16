# Canvas Pilot

![Version](https://assets.dandaedre.com/coverage/idle-org/canvas-pilot/main/version.svg)
![Tests](https://assets.dandaedre.com/coverage/idle-org/canvas-pilot/main/test.svg)
![Coverage](https://assets.dandaedre.com/coverage/idle-org/canvas-pilot/main/coverage.svg)

Arrange screen layers, exchange geometry, and review transfers to Analog Way Midra
and Aquilon presentation switchers. An independent project, not affiliated with Analog Way.

```sh
git clone https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot.git
cd canvas-pilot
cargo run --locked
```

Use a current stable Rust toolchain. Linux desktop builds need the usual X11/Wayland
and GTK development libraries. Python tooling requires Python 3.11+.

[Full manual](https://books.dandaedre.com/manuals/canvas-pilot/) ·
[User guide](docs/canvas-pilot/guide/README.md) ·
[Templates](docs/templates/canvas-pilot/README.md) ·
[Changelog](docs/canvas-pilot/changelog.md) ·
[Development](docs/development/README.md) ·
[Tasks and stretch goals](docs/canvas-pilot/plan.md) ·
[Documentation index](docs/README.md)

Originally part of [beffroi-tools](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools); see [migration notes](docs/development/migration.md).
