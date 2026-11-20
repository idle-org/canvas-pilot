<p align="center">
  <img src="crates/canvas-pilot/assets/app-icon.png" width="96" height="96" alt="Canvas Pilot icon">
</p>
<h1 align="center">Canvas Pilot</h1>
<p align="center"><strong>Compose your screen layout. Frame every layer. Bring it to the switcher.</strong></p>

<p align="center">
  <a href="docs/canvas-pilot/changelog.md"><img src="https://assets.dandaedre.com/coverage/idle-org/canvas-pilot/main/version.svg" alt="Version"></a>
  <a href="docs/development/ci.md"><img src="https://assets.dandaedre.com/coverage/idle-org/canvas-pilot/main/test.svg" alt="Tests"></a>
  <a href="docs/development/ci.md"><img src="https://assets.dandaedre.com/coverage/idle-org/canvas-pilot/main/coverage.svg" alt="Coverage"></a>
</p>

<p align="center">
  <a href="https://books.dandaedre.com/manuals/canvas-pilot/docs/canvas-pilot/guide/install.html"><strong>Install and update</strong></a> ·
  <a href="https://books.dandaedre.com/manuals/canvas-pilot/">Read the manual</a> ·
  <a href="https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/releases">Releases</a> ·
  <a href="https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/issues">Report an issue</a>
</p>

Canvas Pilot is a desktop layout editor for presentations and live events. Arrange
slides, cameras and other content as layers on a screen, give them precise sizes
and positions, and crop each picture into place. Prepare everything offline,
exchange layouts with drawing tools, or connect to a supported **Analog Way Midra
or Aquilon** to load and send layer geometry.

Canvas Pilot prepares the layout; your presentation switcher renders the live output.
You can use the editor and file exchange without any connected equipment.

<!-- Screenshot recipe: docs/canvas-pilot/images/capture.md#01-workspacepng -->
[![Workspace showing a Slides and Presenter layout, pixel measurements and layer controls](docs/canvas-pilot/images/01-workspace.png)](docs/canvas-pilot/guide/quickstart.md)

*Slides + Presenter on a Full HD canvas. Screenshots predate the Canvas Pilot
rename and still show the former “Midra Layout” name.*

## Open source and development

Canvas Pilot is open-source software under the [MIT license](LICENSE). Its
development includes LLM-generated or LLM-assisted work, with human review.
It is used in professional settings; supported workflows and current
limitations are documented in this repository.

## From an idea to a screen layout

1. **Set the stage.** Choose a canvas size or import geometry. Add a reference
   image to position layers against your design.
2. **Arrange and frame.** Drag, resize, snap and align layers. Enter exact pixel
   measurements, organize layers in folders, and adjust the crop visually.
3. **Save or exchange.** Keep an editable project, export a layout for another tool,
   or start from the supplied Full HD and 4K templates.
4. **Transfer when ready.** Connect to a supported device, review mappings and
   explicitly load or send. Ordinary editing stays local.

[**Walk through your first layout →**](docs/canvas-pilot/guide/quickstart.md)

## Tools for getting the picture right

| Arrange the screen | Frame the content | Work with the device |
| --- | --- | --- |
| Exact sizes and positions | Visual crop, zoom and offset | Load existing layer geometry |
| Snapping, guides and alignment | Reference images and input stills | Review layer-to-device mappings |
| Multi-selection and group resizing | Advanced layer fit and Midra masks | Send selected, folder or all layers |
| Folders, visibility and undo/redo | Saved projects and recovery | Update changed parameters efficiently |

<details>
<summary><strong>See the crop workspace</strong></summary>

<!-- Screenshot recipe: docs/canvas-pilot/images/capture.md#03-croppng -->
![Crop controls and on-canvas content framing in the earlier Midra Layout interface](docs/canvas-pilot/images/03-crop.png)

Adjust the picture inside its layer while keeping the layout in view.
[Explore crop controls and fit modes →](docs/canvas-pilot/guide/formats-and-crop.md)

</details>

## Bring your layout with you

Choose the format your client already edits: **SVG for vector editors (including
Illustrator)**, **draw.io for diagrams.net**, **PPTX for PowerPoint**, **FIG for
Figma**, or **DXF / DWG for CAD**. Keep a Canvas Pilot project for continued editing.
The comparison guide explains which features each file and clipboard path preserves.

| Choose or download | Markdown | Book |
| --- | --- | --- |
| **Choose a format** — recommendations and feature matrix | [Format guide](docs/canvas-pilot/guide/format-matrix.md) | [Format guide in the book](https://books.dandaedre.com/manuals/canvas-pilot/docs/canvas-pilot/guide/format-matrix.html) |
| **Download templates** — pick an editor and Full HD or 4K | [Template catalog](docs/templates/canvas-pilot/catalog.md) | [Template catalog in the book](https://books.dandaedre.com/manuals/canvas-pilot/docs/templates/canvas-pilot/catalog.html) |

**Sending a template to a client?** Share the
[template download page](https://books.dandaedre.com/manuals/canvas-pilot/docs/templates/canvas-pilot/catalog.html).
They can choose their application and canvas size, download a starter file, edit
its layer boxes and send it back. The Markdown catalog and book use the same
source and template files.

[Import and export instructions](docs/canvas-pilot/guide/files.md)

For **Midra 4K** and **Aquilon / LivePremier**, see the
[device guide](docs/canvas-pilot/guide/device.md) for supported profiles, setup and
feature limits. Support depends on model and firmware; Aquilon validation uses
the documented standalone emulator profile.

---

[**Full manual**](https://books.dandaedre.com/manuals/canvas-pilot/) ·
[Development](docs/development/README.md) ·
[Changelog](docs/canvas-pilot/changelog.md) · [MIT license](LICENSE)

Independent software, not affiliated with Analog Way.
