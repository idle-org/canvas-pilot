# Multiple project screens

Status: design recorded; implementation paused pending confidence in existing features.
Tracked in [Gitea #5](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/issues/5).
[Development](README.md) · [Project plan and existing acceptance checks](../canvas-pilot/plan.md)

## Purpose and agreed direction

Keep several screen layouts in one project, displaying and editing one screen at
a time. Sharing layers and folders between screens is the central feature: separate
projects with copy/paste are not sufficient. Keep one `.pilot.json` document rather
than a project folder with independent screen saves.

A screen dropdown selects the active screen. Explore a Layers / Screens sidebar:
Layers shows the active screen's layers and folders; Screens shows each screen with
its resolution and an expandable list of its layers. Selecting an occurrence selects
its screen and layer. Shared occurrences need a clear shared indicator.

Different screen aspect ratios are allowed. Scaling must remain proportional,
without independent horizontal/vertical deformation.

### Placement and scaling refinement

Provide two separate controls, so alignment and scale can be combined:

- **Alignment:** Center (default), Top left, Top, Top right, Left, Right,
  Bottom left, Bottom, Bottom right.
- **Scale:** Automatic fit, or an explicit percentage. Automatic fit calculates
  and displays the percentage (for example 200% or 50%); a custom value can be
  entered. Automatic fitting uses the largest uniform scale that fits the layout
  inside the destination screen.

Proposed math, to confirm during the visual phase: for a reference layout W × H
and destination w × h, automatic scale is min(w/W, h/H). For scale s and anchor
fractions ax, ay chosen from 0, 0.5, 1, placement offsets are
ax × (w − sW), ay × (h − sH). Transform positions and sizes together using the same
scale. A 1920 × 1080 layout becomes 200% on 3840 × 2160. On 1920 × 1200 it stays
100%, with 60 pixels above and below when centered, or 120 below when top-aligned.

A manual scale can exceed the screen. Show overflow in the preview; do not silently
change the requested scale or introduce source cropping. Define clipping explicitly
before implementation. Alignment applies to the reference layout rectangle, not a
bounding box that changes whenever a layer moves.

The precise owner of alignment/scale (screen, folder occurrence or layer occurrence)
is still a design decision. Start the prototype with a screen-level default and
assess whether per-folder/per-layer overrides are actually needed. Screen membership
is selectable for each layer and folder regardless of that decision.

## Proposed behavior to review in the prototype

These are recommendations, not implemented behavior or fully settled requirements.

- One shared layer definition holds its geometry and appearance. Editing it updates
  its occurrences on all assigned screens. Show the affected screens near the controls.
- A **Screens…** checklist assigns layers and folders. Children use folder assignments
  by default; **Choose screens** enables an explicit override. Keep membership separate
  from temporary visibility.
- **Make independent on this screen** creates a separate layer when a divergent design
  is needed. Avoid implicit, partially linked copies.
- **Remove from this screen** and **Delete from all screens** are distinct actions.
  Keep layers without assignments in an Unassigned view.
- Start with shared stacking order filtered by membership. Evaluate the need for
  per-screen ordering before adding another override system.
- Keep source relationships shared; place device links on screen occurrences because
  one shared layer may target different switcher layers on different screens.
- Keep reference placement, captures and workspace views screen-specific. Reuse image
  bytes where useful, without coupling their placement.
- Use project-wide undo for shared edits, with clear affected-screen context.
- Screen selection, membership changes and project opening never send device commands.
  Device transfers explicitly identify the screen and destination.
- Native-pixel **1:1** fit remains 1:1 even when its container scales; its visible result
  may therefore differ across resolutions. Verify crop, fit and blanking interactions.

## Persistence and exchange plan

- [ ] Specify stable screen and folder identities, shared layer definitions,
  memberships, reference coordinates, placement rules and per-screen device mappings.
- [ ] Decide how editing in a scaled occurrence maps back into shared coordinates,
  preserving precision without cumulative round-trip scaling.
- [ ] Version the project schema and migrate current single-screen projects into one
  screen without changing geometry, settings or device links.
- [ ] Preserve recovery and shared references; define behavior when removing a screen,
  changing its resolution or deleting the reference screen.
- [ ] Keep **Active screen** as the default export scope; offer **Selected screens…**.
- [ ] Audit each existing codec for multiple pages, slides, frames or artboards before
  promising a single-file multi-screen export. Where supported, use one named screen
  per page/slide/frame/artboard.
- [ ] For single-canvas codecs, export the active screen or a ZIP of named screen files.
  This is an exchange option, not the project's storage model.
- [ ] Version JSON/CSV exchange representations where screen identifiers are added.
- [ ] Define snapshot scope explicitly: retain screen/sharing relationships for exported
  content, with clear omissions. Preserve existing inactive device-proposal semantics.
- [ ] Explain that external editors generally receive independent geometry and cannot
  be assumed to retain live sharing relationships or embedded metadata.
- [ ] Review screen selection and layer matching on reimport. Offer updating the shared
  definition or making an independent occurrence; surface conflicting external edits.
  Never silently apply the last encountered edit to all screens.
- [ ] Preserve the distinction between geometry reimport and restoring the original
  snapshot: restoration does not incorporate later external artwork changes.
- [ ] Update interface contracts, fixtures, format matrices and manual with the code.

## Work phases and checkpoints

### 0. Establish confidence in current behavior

No multi-screen application changes are requested yet. Existing work needs testing
before expanding the model; passing automated tests does not replace manual acceptance.

- [ ] Review the existing project test list and identify unverified recent behavior.
  Keep results there rather than creating a second competing acceptance record here.
- [ ] Exercise current project save/open/recovery, crop and fit modes, 1:1 movement,
  blanking, clipboard appearance/names, geometry exchange and snapshot restoration.
- [ ] Review input mappings and inactive input proposals; distinguish emulator readback
  from physical-output evidence. Keep device writes explicitly authorized.
- [ ] Record failures, limitations and fixes before deciding to resume this feature.

### 1. Visual-only prototype, after resuming

- [ ] Use sample 1080p, 4K and different-ratio screens with shared folders and layers.
- [ ] Mock the dropdown, sidebar tabs, screen membership, inherited selections,
  shared indicators and independent-layer action.
- [ ] Mock alignment and automatic/custom scale controls, including displayed
  percentages, unused space and overflow previews.
- [ ] Show proposed export scope, shared-edit conflicts and removal wording.
- [ ] Keep prototype state disposable: no production persistence/schema changes,
  geometry engine changes or device writes. Do not imply functional support.
- [ ] Review usability and settle open decisions before implementing the model.

### 2. Implement the agreed behavior

- [ ] Implement persistence/migrations and geometry rules with meaningful regression tests.
- [ ] Connect the reviewed UI, undo/recovery, exchange and device mappings in stages.
- [ ] Run the repository Rust gates, vendor-format and restricted-format tests,
  coverage, Python checks and manual validation as applicable.

### 3. Acceptance

- [ ] Share a layer and folder across screens; edit once and verify all occurrences.
- [ ] Verify 1080p↔4K scaling, different ratios, all nine anchors, custom percentages,
  overflow, fractional coordinates and repeated screen switching without drift.
- [ ] Verify folder inheritance/overrides, visibility, unassigned layers, independent
  copies, ordering and the two deletion scopes.
- [ ] Verify every fit mode, crop offsets and blanking, especially native-pixel 1:1.
- [ ] Save/open/recover and undo across screens; migrate representative old projects.
- [ ] Round-trip each supported exchange path, including selection scope, sharing
  metadata, external conflicts and explicit unsupported/omitted features.
- [ ] Verify independent device mappings and explicitly scoped transfers, with no
  implicit sends or input routing changes when switching screens.

## Decisions still open

Reference-coordinate ownership; per-screen versus per-occurrence scale overrides;
what folder overrides mean for newly added children; clipping at custom scales;
resolution-change policy; screen deletion and undo navigation; export/import scope
and stable identity through external editors. Resolve these in the prototype and
contracts instead of making silent implementation assumptions.
