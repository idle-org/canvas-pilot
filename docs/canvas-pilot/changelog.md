# Canvas Pilot changelog

[All changelogs](../changelogs.md)

## Unreleased

- Put portable release downloads first in installation help, with exact archive
  patterns by platform and optional command-line methods below. Added seven real
  offline screenshots to advanced guide chapters, with captions and capture recipes.

- Reworded conversational test attributions as manual reviews across the catalog,
  guides and acceptance records, preserving dates and validation boundaries.

- Made the template catalog’s format-choice link more visible with a compact
  callout and bold introduction, shared by the Markdown page and book.

- Ordered the file feature matrix by preservation support, starting with Canvas
  Pilot projects, and explained capability ties and the AI converter requirement.

- Placed native-codec note 3 beside format names and extra-settings note 4 in
  each supporting row’s Extra settings cell in the feature matrix.

- Moved the detailed editing-state preservation comparison from the format matrix
  to the import/export guide beside snapshot restoration, retaining a link.

- Moved the extra-settings note reference beside each supporting format name
  in the feature matrix.

- Shortened the feature matrix’s Pictures note to the export option names.

- Removed the project-format note from the feature matrix and renumbered the
  remaining note references.

- Switched the format matrix to ✅ / ❌ emoji, replaced ambiguous asterisks with
  linked numbered notes, and linked the comparison from the introduction, first
  layout, clipboard import and snapshot-restoration guidance.

- Added prominent README format recommendations and paired Markdown/book links
  for the format matrix and template catalog. Added a short client handoff workflow
  to the shared catalog; downloads and detailed capabilities keep one source.

- Restored the illustrated main README with a workspace preview, feature overview
  and layout workflow. Kept status badges at the top, detailed instructions in the
  book and task lists behind the Development link.

- New project saves suggest `.pilot.json` (Canvas Pilot). Open project accepts
  any `.json` filename and validates its contents, including legacy `.midra.json`
  projects. Files are not renamed automatically. Updated the manual and checked
  read/write and overwrite protection for preferred, generic and legacy filenames;
  JSON contents and schema version are unchanged.

- Added user-guide feature matrices for all eleven exchange formats, project saves
  and separate clipboard workflows. Distinguished geometry, names, visual crop/mask
  pictures and editable snapshot restoration, checked against the codec behavior.

- Simplified READMEs to navigation and kept user instructions in the book's source
  chapters. Moved task-list discovery behind Development and indexed contracts,
  research and handoffs in `AGENT_DOCUMENTATION.md`; removed internal records from
  the user book's navigation while preserving their files and evidence.


- Added an installation/help chapter covering release downloads, tagged-manifest
  cargo-binstall without compile fallback, cargo install from a release tag,
  platform launchers, updates and troubleshooting. Linked it from the README,
  book, guides and portable quick start; noted the pending first release.


- Restored build-status badges to the top of the README. Checked release readiness
  without compilation or publication, added required cargo-binstall metadata and
  replaced the portable archive's repository README with standalone instructions.


- Saved a conversation handoff for reopening the standalone project, including
  pending input-ratio scope, planned videos, device-test context and publication
  state. Linked it from agent/developer guidance and retained pending plans in
  the task list without changing completed acceptance status. Added the approved
  showcase, video walkthrough and optional chapter clips as a stretch goal.


- Redesigned the repository README with an illustrated introduction, layout workflow,
  feature overview, format/device guidance and direct setup, template and help links.
  Existing screenshots are identified as captures from before the rename.


- Audited all 165 migrated project files; task lists, stretch goals, advanced guide,
  screenshots, templates and fixtures were retained. Expanded agent guidance and
  documentation navigation, and included the transferred checklists/research in
  the book. Recorded the comparison against the exact pre-migration commit.


- Moved Canvas Pilot, its device probe, templates, tests and manual into the standalone
  `idle-org/canvas-pilot` repository. Builds and documentation no longer require
  beffroi-tools; project/settings compatibility and the public manual URL are unchanged.
  CI publishes only this book and its own badges. The coverage command also
  initializes its output directory when run directly from a fresh checkout.


- Fix stale format-acceptance links after the backlog reorganization, restoring
  generated manual and print-page anchor validation.

- Document the manual-verified distinction between input Content ratio and
  Transform to, their existing advanced device controls, and the limits of local
  preview modeling. No geometry, device writes or save format changes.

- Add independent layer masking: magenta handles, whole-window drag, arrows,
  numeric pixel edges, alignment, center/reset and Undo/Escape, without resizing content.
  Midra imports and reviewed sends preserve masks; Aquilon rejects unsupported
  masking. Project v15 migrates older files with zero masks; recovery, embedded
  snapshots and export artwork preserve blanking. Tests cover pixel preservation,
  gestures, migration and device round trips. Physical mask-edge rasterization
  remains an engineering follow-up. Workspace tests (539 passed, one ignored),
  lint/docs and coverage (90.63%) passed; visual mask drag/resize checked. The
  Aquilon emulator at 192.168.10.146:3000 passed Preview/Program regression
  with both sizing strategies; original state restored and read back.

- Allow moving the whole 1:1 crop by dragging anywhere inside the layer,
  including empty margins, or using arrow keys (Shift for ten pixels). Opposite
  crop edges move together, preserving crop size, pixel scale and the container;
  stop at source bounds, with one-step Undo and Escape cancellation. No schema
  or device-protocol changes.

- Disable the crop helper button, context-menu action and double-click shortcut
  for Centered/Cropped: those modes fit automatically. Keep numeric crop fields
  available in Crop properties; normal crop and the 1:1 helper are unchanged.

- Add the double-click crop helper for advanced 1:1 fit: independent side/corner
  handles trim or restore source pixels while preserving pixel scale and the
  layer container. Show a faded source guide, synchronize crop fields, and keep
  one-step Undo/Escape cancellation. Centering follows Midra's 1:1 semantics;
  no project schema or device protocol changes. Regression tests cover edge
  limits, every handle, double-click interaction and undo/cancel.

- Add opt-in advanced Midra layer fit: centered 1:1, Centered, Fullscreen and
  Cropped, with crop-first previews/export artwork, container resizing, device
  load/send and undo. Normal zoom/crop remains the default; input inheritance
  is excluded and Aquilon rejects unsupported explicit modes. Project v14
  persists fit with migrations from v1–13. Automated math, UI and mock-device
  round trips cover the modes; physical raster precision remains unverified.
  Replace impractical operator measurements with an engineer-ready stretch goal.

- Narrow Midra aspect work to explicit layer Crop-menu modes following operator
  findings: 1:1 keeps pixel scale during resizing, and explicit layer modes
  override input fill. Retire input-inheritance/correction tests from this task;
  keep advanced input controls separate. Runtime behavior is unchanged.

- Add an ordered feature demonstration checklist covering project creation,
  editing, geometry exchange, device transfers and input previews.

- Add an operator checklist and results template for Midra fill/aspect tests,
  covering scaling, inheritance, crop order, input correction and rounding.

- Prioritize Midra input layer-fill and layer aspect support. Review the official
  V3.2 user manual, AWJ v3.2 and REST v3.0.13 guides; record the remaining
  inheritance, scaling, cropping and physical-output checks in the test list.
  This is research/planning only; mode conversion behavior is unchanged.

- Split the backlog into a to-do list (including stretch goals) and a test list.
  Move Inkscape clipboard acceptance to testing and remove standalone captured
  input image saving from planned work following scope review.

- Register standalone Aquilon AQL Cmax 6.2.x as first-class supported, based on
  the recorded 6.2.79 emulator acceptance. Replace the stale implementation
  handoff with completed milestones and explicit scope boundaries; physical
  output, other models/firmware and unsupported modes remain unverified.

- Record manually reviewed clipboard workflows for Illustrator, draw.io and
  Figma. Narrow remaining clipboard acceptance to Inkscape; QCAD and PowerPoint
  remain file-exchange workflows for this scope.

- Validate the supplied Illustrator SVG clipboard sample with regression coverage
  for fractional coordinates and optional centered-stroke bounds. Record QCAD as
  a file-exchange workflow following workflow review; no clipboard adapter is planned.
  Existing SVG parsing handles the sample; live clipboard delivery remains unverified.

- Mark export/workflow cleanup solved following user confirmation; move it from
  the remaining backlog to completed implementation.

- Preserve the supplied Figma SVG clipboard sample as a regression fixture for
  ordered rectangular paths and matrix translations through the paste parser.
  Existing SVG parsing handles it; native Figma clipboard formats and live
  clipboard delivery remain unverified.

- Add a sourced presentation-switcher adapter survey, ranked by public control
  documentation with separate geometry feasibility assessments, and a stretch
  goal to consider selected families. Research only; no new support is enabled.

- Compact Measure position from into a dropdown with side-by-side From screen
  and To layer pickers and a summary of both points. Preserve linked/independent
  point selection and project autosave; regression checks exercise the popup.

- Record a distant-future stretch goal for multiple editor screens in one project,
  selected through a dropdown with one screen displayed at a time.

- Add vector send/load arrows to the bottom transfer buttons, matching the mapping controls without relying on symbol-font glyphs. Visually checked at 1440 and 1000 pixels wide.

- Replace Aquilon full-store downloads with batched AWJ reads on TCP 10606, retaining fresh preflight and verified sends. Emulator reads fell from 124 MB to about 197 KB and took about 0.30 seconds directly. Twelve transfer cases passed with restoration; malformed/incomplete responses and changing snapshots fail explicitly. HTTP/WebSocket custom ports remain supported; AWJ must be reachable on the same IP.

- Replace unsupported arrow glyphs in transfer labels and help text with plain text so they render with the bundled UI font.

- Add a separate Aquilon / LivePremier 6.2 adapter around a shared screen/layer model, with bounded cancellable state reads, allocated-slot discovery, explicit bank roles, fractional geometry and centered crop/zoom transfer, and input previews above 16. Sends preflight conversion inputs and bank roles, apply the correct family pulse and verify readback. Twelve emulator Preview/Program geometry/crop cases passed with baseline restoration; device writes round sizes/centers to whole pixels because WebRCS rejects fractional fields despite firmware accepting them. The initial profile targets standalone AQL Cmax; linked systems, other firmware/models and unsupported fit/input-correction modes remain explicit limitations.
- Save project version 13 with family-aware `device_link` destinations; migrate versions 1–12 and retain old recovery/embedded-project compatibility. Use device-neutral connection and mapping controls while preserving Midra behavior.

- Rename Midra Layout to Canvas Pilot across the application, Rust package, manuals and starter-file catalog. Existing project formats, settings and recovery locations remain compatible; Midra device names remain unchanged. Workspace Rust gates and 62 Python tests pass; full coverage is 90.55%. Illustrator starter contents retain legacy metadata because VectorCraft CLI was unavailable.

Changes since [v0.7.0](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools/src/commit/ed653c5/docs/releases/v0.7.0.md); these are not yet part of a new
release. Current project schema: **v13**; application settings: **v8**.

- Audit against all 21 commits from v0.7.0 through `a7b51cb`: no missing change
  entries. Reconcile superseded capture-menu wording, project-version progression
  and template counts; preserve the released history below.

- Add an Aquilon implementation handoff covering the observed emulator, bounded
  state loading, family-specific geometry/transfers, saved links, crop, capture
  and staged acceptance checks. No device support is enabled by this task.

- Recommend SVG for Illustrator exchange in the README, manual and template
  catalog. Keep AI as an optional compatibility target requiring external
  VectorCraft CLI, with executable discovery and setup documented.

- Group viewport context-menu property windows under Properties and capture under
  Input preview & crop guides, with quick preview/opacity controls. Keep common
  editing actions direct, move Delete after Lock/Hide, and group canvas options
  at the bottom. UI tests exercise property submenus and multi-selection actions.

- Allow Midra mapping & transfer to open while offline, with Connection settings
  available and scope/mapping controls hidden until connected. UI regression
  coverage verifies offline opening does not start a read or send.

- Validate device identity before downloading state. Aquilon now reports its
  unimplemented adapter instead of a misleading 32 MiB limit error. A read-only
  AQL Cmax 6.2.79 emulator probe measured a 118 MiB store; Aquilon connectivity
  remains pending. Mock HTTP coverage checks that no state request is made.

- Record user acceptance of Figma FIG, FigCraft FIGCRAFT, DXF and DWG
  import/export; mark these format issues solved in the plan and compatibility notes.

- Add native FigCraft (.figcraft) geometry import/export, file drops and page/frame
  selection using the existing crate. Include background/artwork options and both
  starter sizes in the combined ZIP. Test native resaving and geometry/artwork
  round trips; no_proprietary builds keep this codec disabled.

- Offer Download all templates as ZIP first in the README and starter catalog,
  replacing the Illustrator-only archive. With the FigCraft addition above, it
  contains **22 Full HD/UHD starter files across 11 formats**.
  Tests check every archived filename and byte against the individual templates.

- Clear layer selection when clicking empty space in the Layers list, including
  gaps between rows and the gutter beside folder contents. Regression tests cover
  blank clicks and normal folder/layer selection.

- Make the folder icon create an empty folder without moving selected layers; use
  one tooltip. Both layer-list and viewport Move to folder menus move all selected
  layers when invoked on a selected layer. Empty folders support rename/delete,
  undo and save/load in project v12, with v1–v11 migration. Added folder lifecycle
  and context-menu selection regression tests.

- Add opt-in extra Midra parameters to SVG, draw.io, geometry JSON/CSV, ASCII DXF,
  HTML and PPTX, using metadata/custom fields or comments. Geometry remains
  editable; explicit Restore parameters from export recovers the original snapshot
  into an unsaved project. JSON/CSV v2 carries metadata; plain v1 exports are
  unchanged. Document all formats, including unsupported DWG/FIG/FIGCRAFT/AI metadata embedding.
  Round-trip tests cover settings and geometry; installed draw.io exported the
  metadata-bearing diagram to SVG, and librsvg rendered both SVG variants.

- Audit project persistence: version 11 introduced saved snapping/grid setup, group resize
  mode, collapsed folders, capture assignments/stills, guide opacity and refresh
  settings. Saved auto-refresh reopens paused. Versions 1–10 migrate explicitly;
  invalid previews leave current work intact. Added round-trip/migration and
  preview restoration regression tests; documented project/application/session
  ownership in the persistence audit. Project v12 subsequently adds persistent
  empty folders, as noted above.

- Preselect linked Midra input sources in untouched capture controls after connection or layer import/load, without enabling previews or requesting images. Preserve operator selections and leave unassigned/non-input sources unselected; regression coverage checks the scheduler stays idle.

- Move input selection into the layer properties Preview window to avoid nested
  dropdowns closing when long input lists extend outside the menu. The current
  entry point is **Input preview & crop guides → Configure capture…**; quick
  preview/opacity controls remain in that submenu.

- Group Load and Send mapped layers on the final row of Midra mapping & transfer, with vector down/up arrows and direction tooltips. Keep send preparation separate from the explicit Send click and retain scope-aware loading.

- Round fractional device-reported content dimensions to whole pixels for centered crop import/readback and send compatibility, with nonblocking before/after warnings. Preserve raw state and fractional box geometry; invalid ranges and genuine source mismatches still fail. Regression tests cover cropped import, warning behavior, both send sizing strategies and invalid ranges.

- Restore recovery sessions silently instead of opening a “Recovery restored” message. Keep recovery failures and retry information visible; regression coverage checks restored edits and absence of the success popup.

- Investigate Aquilon/LivePremier compatibility against the upstream web-control implementation; document firmware-specific geometry, bank and apply paths, saved-link constraints, and the staged adapter/acceptance plan. Research only: no Aquilon support or hardware verification claimed.

- Add a Recent machines picker for the last four successful connections, saved across restarts with distinct ports and most-recent-first ordering. Selecting an entry reads the device immediately; failed attempts do not displace history. Settings v8 migrates v1–v7 with empty history; tests cover ordering, deduplication, failures, persistence and validation.

- Support optional device ports for HTTP state reads, input captures and WebSocket sends, preserving the address in preferences/autoconnect. Accept all four Midra 4K identities (Eikos, Pulse, QuickVu, QuickMatrix) with device-reported allocation. Read-checked Pulse 4K 3.3.7 at port 3000 (four layers); add address, model, capture and WebSocket port regressions. Other model hardware acceptance remains outstanding.

- Replace the layer-header Folder, Add image and Manage text buttons with compact vector icons, grouped on the right, with tooltips and accessible action names.

## v0.7.0 — 2026-11-14

- Split the Midra mdBook into Quick start and Advanced sections, with a first-layout walkthrough and nested workflow/contract chapters. Keep the existing detailed chapters and screenshot capture notes linked.

- Update the shared Rust application version to 0.7.0; see the [release notes](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools/src/commit/ed653c5/docs/releases/v0.7.0.md).

- Add direct links to the published mdBook manual from the project README, tool guide and documentation indexes.

- Mark PPTX, draw.io and SVG format acceptance as solved following manual review confirming all three work (2026-11-14).

- Replace the incomplete DXF skeleton with the official shared Craft DXF document writer/reader; add optional Craft DWG exchange with the same pixel geometry and canvas limits. Refresh CAD starter files and cover document structure, binary DXF and DWG round trips. Add a byte-preserving ZIP of the AI templates to bypass Gitea/browser PDF preview downloads.

- Place insufficient import-destination warnings directly below the destination scope selector instead of repeating them near the Import button.

- Simplify geometry/clipboard import into a shared review without repeated acquisition controls. Collapse clipboard and before/after details, emphasize destination mapping, and add independent All / Selected / Folder scopes plus per-item exclusion. Preserve compatible mappings and canvas selection; regression tests cover scope filtering, atomic import and review visibility.

- Route supported geometry file drops through the regular asynchronous import/mapping review, including SVG/draw.io and native formats when available. Keep raster drops as references; support path/data drops and multiple documents through the file/page chooser, with a combined 16 MiB limit and atomic failure. Regression tests cover codecs, case-insensitive names, batches and invalid/oversized inputs.

- Add an mdBook manual built from the existing guides, with local assets and downloads. Jenkins publishes successful main builds under `/mnt/books/manuals/`; staging, link validation and scoped publication have regression coverage.

- Replace the long overview with a visual quick start, move detailed workflows into topic guides, and centralize all nine starter-file formats in one catalog. Add real application screenshots with reproducible capture notes; retain the existing template files and validation contracts.

- Clarify shared Midra links: warn with other linked layer/folder names while allowing link edits; disable conflicting scoped sends and list all overlapping destinations. Preserve conflict-free folder/selection sends and remapping. Remove nested-folder/additional-codec backlog items and replace the extra-links proposal with this shared-destination workflow. Regression tests cover warning text, send blocking and scope isolation.

- Remove duplicate overlay-grid controls from snapping menus; keep them in the adjacent Grid toolbar menu and a separate section in Editing preferences.

- Move the background grid out of the sample artwork into a full-output overlay above reference images. Add a Grid toolbar menu, independent visibility/snapping, and 1–64 columns/rows (default 8 × 6). Persist these preferences in settings v7 with migration from v1–v6; share fractional grid positions across movement, resizing and pivot snapping. Regression tests cover rendering order, toggles, snapping and settings compatibility.

- Keep the screen name and output resolution visible with reference images, reserving their fit-screen margin. Viewport tests check label visibility and clipping with and without imported images.

- Double-click again in visual crop mode to finish editing; preserve selection and framing. Viewport regression coverage checks entering, exiting and reopening without geometry changes.

- Pool input captures across matching queued layers and automatic previews, including requests queued during a fetch. Share one decoded texture and consume matching requests on failure to avoid per-layer retries; preserve existing manual stills. Regression tests cover direct, automatic and import captures, failure handling and different-input isolation.

- Capture a first still automatically when selecting a preview input, in both pickers. Queue the latest choice behind the shared five-second throttle and active request, without waiting the auto-refresh cycle; retain explicit recapture/import. Regression tests cover throttling, coalescing and cancelling stale targets.

- Hide no-signal inputs by default in both preview pickers; add a shared session-only Show inputs without signal toggle and explicit No signal labels. Preserve existing choices when filtering or signal loss occurs, and leave initial selection empty when no input has signal. UI tests cover both pickers and the empty state.

- Add Import input snapshot… with an explicit destination picker for any editor layer or a fixed reference image. Layer imports optionally keep refreshing; references never auto-refresh. Preserve selection/geometry and existing snapshot limits; regression tests cover both destinations without a selection.

- Simplify layer previews to Enable → Input → Capture still → Auto-refresh. Capture directly replaces that layer's image; auto-refresh is per layer and can fetch the first image. Preserve serialized low-rate requests, bind manual results to their original target, and keep manual stills unchanged by other layers' refreshes. Regression tests cover control order, recapture, stale results and automatic first capture.

- Move captured-input controls and the image preview into a dedicated Preview properties tab, with a matching right-click entry. Crop keeps its geometry controls; the existing preview context submenu remains available. Tab-switch regression checks include Preview.

- Keep Mapping limited to Midra screen/layer links; move editor input identity into the opt-in Advanced Source section. Offer editor Inputs 1–8 independently of layer count, with New editor input for additional local identities. Apply creates missing slots and assigns the source atomically with Undo; project round-trip tests cover shared Input 8, expansion and limits. Midra layer links and physical routing remain separate.

- Filter Midra destinations by each screen's CURRENT allocated layer count, not the eight firmware storage slots or physical inputs. Keep allocated layers available without assigned sources; exclude unallocated slots from import, mapping and sends, including saved links. Regression checks cover active versus staged allocation and Preview/Program sends.

- Keep existing Midra layers available for mapping, import and send regardless of input assignment or OFF state. Uncropped layers without content dimensions import their container; sends use editor dimensions when hardware content dimensions are unavailable, preserving routing. Regression tests cover unassigned layers and both send strategies/targets.

- Use absolute Gitea main-branch URLs for repository file links in the root README; verify linked files exist.

## v0.6.0 — 2026-11-14

- Synchronize the shared Cargo package version with the v0.6.0 milestone tag; see the [release notes](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools/src/commit/ed653c5/docs/releases/v0.6.0.md).

- Add opt-in input-preview auto-refresh with a 5–300 second request interval (default 10). Visible previews refresh round-robin with shared input textures, one background request at a time; pause offline/manual capture, retain images on failures and keep fixed references unchanged. Timer controls and assignments remain session-only. Regression checks cover scheduling, throttling and response handling.

- Use captured input stills as toggleable layer previews, with crop guides composited above them and a 0–100% opacity slider in Crop properties and the layer context menu. Guides return to full opacity without an enabled preview; source/input/dynamic crop and ghost overflow share the same mapping. Layer-preview assignments/settings are session-only; capture is throttled, with optional auto-refresh added above.

- Group the guide and supporting documentation in this tool’s folder; update portal, cross-document and packaging links for the new paths.

- Add manual Midra input-still capture through the verified WebRCS PNG endpoint, with preview/Add as reference, one request at a time and a five-second minimum interval. Device-setting writes are not involved; active-input 512 × 288 and inactive blank-image responses checked on the Eikos.
- Load project/recovery files and decode embedded references in a background worker, install atomically, preserve work on errors/cancel and pause recovery autosave while loading. Reconcile the historical plan into completed implementation, remaining features and separate hardware/platform acceptance checks.

- Add README-linked two-layer PPTX, Figma FIG and Illustrator AI templates in Full HD and UHD 4K. Extend the template generator with format selection and import verification; test native fixture geometry, with AI checked through VectorCraft 0.24.1.

- Remove the redundant Copy coordinates button from the title header and right-align the device label. Existing copy commands remain available.

- Move Menu from the title header to the toolbar immediately before Project, within the same wrapping group.

- Place Load before Send in the status bar.

- Align status-bar connection, mapping, send/load and scope controls in one vertically centered row with consistent spacing and button heights; let labels determine positions instead of fixed offsets.

- Rename Apply changes and Configure Midra export to “Midra mapping & transfer…” so the action clearly opens layer mapping and send/load settings; actual sending remains a separate button.

- Keep multiple reference images in a purple section of the layer list, with add, eye, remove, settings and Show only actions; references remain fixed and excluded from layer/device operations. Add removal to Import and viewport/reference context menus. Queue multi-file imports in order, composite visible guides in viewport/artwork exports, and save all images in project v10 with tested v1–v9/recovery migration. Corrupt additions or project images preserve existing work.

- Add Paste layer values beside Copy layer values in the layer context menu, using the existing clipboard import and mapping review before applying changes.

- Make Source an advanced, opt-in section with explanations of input aspect correction and pre-layer input crop. Show the linked device input's actual WebRCS aspect/crop controls separately from the local geometry model. Stage only edited fields, include them in scoped sends only while enabled, and pulse input crop updates. The opt-in and staged device edits are session-only and clear on disconnect, address/project change or uncheck. Verified field names/ranges against the connected Eikos WebRCS; regression tests use mock devices, without hardware writes.

- Add a Mapping properties tab and right-click entry with selectable Midra screen/layer links and Unmap. Unmapped layers are excluded from linked sends/loads; link edits are local, saved with projects and undoable. Move the separate editor input identity selector into Mapping.

- Draw the snapping dropdown chevron and layer order arrows as vector icons, avoiding missing font glyphs while preserving tooltips, disabled states and button accessibility.

- Default Midra transfers to All layers with a shared All/Selected/Folder selector. Folder mode follows clicked folders for quick alternate-layout sends/loads; links remain editable and out-of-scope links are preserved. Import suggests saved links and saves remapped/new-layer links and folder membership atomically with geometry. Tests cover scope isolation, reused destinations, in-flight reads and Undo; no hardware writes were needed.

- Organize the flat layer sidebar into collapsible folder groups: select contents, double-click rename, group eye/lock controls, drag layers between folders/root, and reorder folders as blocks. Add new-folder and right-click actions, with atomic Undo and existing project compatibility; empty groups disappear and nesting is not introduced.

- Reorder the viewport toolbar into shared bordered groups: Project/Import, Undo/Redo, Select/Move, view controls, Align/Snap/Pivot, and Output/Options. Groups wrap together on narrow windows and preserve optional icon captions.

- Snap moving layers, resize handles and pivots to screen center and quarter guides on both axes. Moving layer centers also snap to these interior guides; the existing screen snapping option controls all screen guides.

- Use a crisp, theme-aware magnet icon for the snapping dropdown; preserve snapping controls and active grid indicators.

- Unify file exchange around shared geometry preparation, byte-oriented codecs and one Export layers window. Add optional native PPTX/FIG import/export with slide/frame selection and embedded artwork, plus local VectorCraft AI conversion. File conversions run off the UI thread. Document and test `--no-default-features --features no_proprietary`; CI covers both configurations and production coverage includes the codecs.

- Add versioned geometry JSON, RFC-style CSV, DXF and HTML/CSS import/export alongside SVG and draw.io. All six formats have two-layer Full HD/UHD templates and round-trip tests; artwork support is explicit per format.

- Add README-linked two-layer SVG and draw.io templates for Full HD and UHD 4K, generated by the application exporter and checked byte-for-byte in regression tests.

- Import SVG rendered by draw.io Desktop, including its SVG 1.1 DOCTYPE, nested cell wrappers, symbol/image layers and generated label fallbacks. A background-preserved native 31.7.0 round trip was checked; draw.io's cropped SVG exports necessarily retain their rebased coordinates.

- Add current-layer SVG and draw.io exports with optional embedded backgrounds and source-pattern placeholders; import native/compressed draw.io files. Round-trip tests cover scopes/artwork and whole-pixel geometry; draw.io Desktop 31.7.0 rendering and XML re-save were checked. Use JSON for full source/crop fidelity.

- Hide tiled source patterns outside crop editing, even with saved pattern display enabled. Double-click/Crop properties reveals the selected layer’s pattern; the existing preference now shows other layers’ patterns only while cropping.

- Constrain Crop-tab X/Y offsets to the valid content bounds, including after zoom decreases; typed and dragged edits clamp to the edge instead of producing offset-range errors. Undo remains available.

- Remove redundant position/size and output Apply buttons and their close prompts; names/folders commit on Enter or focus loss with Undo. Rename staged source and clipboard/SVG actions to “Apply source changes” and “Import mapped values”; Midra sends remain explicit.

- Add ImageGen application artwork in PNG, Windows ICO and macOS ICNS formats. Embed native icons and support Linux/BSD desktop entries and macOS app launchers; native Windows/macOS visual checks remain required. Include artwork for the Python device probe.

- Fix the crop-alignment regression test’s Clippy range-loop warning so CI passes with warnings denied; retain both offset assertions.

- Rename the positioning toggle to “Use a different measure point”.

- Keep the example presentation visible for every output format, adapting its title, grid, tagline and screen labels to the new dimensions.

- Right-click a layer in the viewport or layer list to rename/delete it; list menus stay compact. Delete supports Undo and respects locked/hidden/last-layer protection.

- Swap new-project default labels: top layer is 01 / Presenter, lower layer is 02 / Slides. Geometry, draw order and existing projects are unchanged.

- Recover the last session automatically from a separate two-second autosave; named saves stay explicit. New project offers Save/Discard/Cancel and shows the last manual-save age. Failed/corrupt recovery is reported and preserved.

- Measurement-point choices and the independent-point toggle autosave to the current project file; other unsaved edits remain untouched. New projects need a first Save.

- Separate Select (V, previous behavior) from Move (M), which drags the existing selection without selecting, deselecting or resizing.

- Layer properties’ Layer tab applies size/locked-ratio changes immediately; remove its Apply button. Drag edits undo together; Source edits remain explicit.

- Layer properties’ Crop tab automatically enables visual crop editing and disables it on leaving/closing; remove its redundant toggle button.

- Ctrl/Pivot now works on crop/zoom handles, keeping the source pixel under the saved gravity point fixed; crop limits prevent empty borders.

- Group grid snapping tools and snapping options in a toolbar dropdown; the label shows the active grid. Pivot remains independently accessible.

- Save gravity points per layer across selection changes and reopening; placement/reset supports Undo. Project v9 migrates older files to centered pivots.

- Pivot dragging snaps to corners, side midpoints and center even with snapping off; also honors grid/pixel/screen/layer options. Alt bypasses built-in anchors; guides and linked pivots follow the result.

- Optional “Use a different layer point” exposes independent screen/layer measurement points (e.g. screen top-left → layer center). Numeric sizing preserves the endpoint. Project v8 saves the choice; v1–7 retain matched points.

- Hide bottom interaction hints when viewport space is tight, reclaiming their 40 px footer. Off-view layer warnings remain visible.

- Fit screen now includes the example backdrop’s Screen 1/resolution labels, preventing clipping in shorter/wider viewports.

- Move the reference drag/drop/import hint into the example artwork footer and reclaim its separate viewport status row. Image details and warnings remain above the view.

- Default backdrop title is “Example Conference Name”; edit/reset it in Options → Example backdrop or Settings → Reference images. Project v7 saves the title and migrates v1–6.

- Add persistent Move/Pan/Zoom icons (V/H/Z). Zoom supports click, Alt-click out and horizontal drag; Space/middle-drag pans from any tool. Escape cancels camera drags; grid/pivot choices stay independent.

- Layer lists now show the topmost layer first; drag/drop and arrows match. Selection no longer raises layers temporarily. Saved project/SVG stacking stays compatible.

- Show current aspect ratios beside dimensions and the ratio lock. Accept manual W:H, W/H or decimal ratios; locked edits keep width/reference and adjust height, unlocked edits wait for the lock. Layer properties initially remained Apply-based; subsequent automatic application is recorded above.

- Add persistent 10 px/100 px grid and Pivot toolbar icons with shortcut tooltips. Grids exclude each other; Pivot combines with either. Matching modifiers temporarily invert toggles; choices last for the session.

- Ctrl shows a movable resize pivot; Ctrl-resize keeps it fixed. Same/Mirror/Opposite modes share or reflect relative pivots. Double-click resets; Ctrl+Shift keeps the 100 px grid. Session-only, with snapping and Undo.

- Hold Alt to temporarily invert movement/resize snapping (on ↔ off), overriding Shift grids without changing saved settings.

- Add Snapping controls to viewport right-click and quick Options menus; share and automatically save the same editing settings.

- In Adjust layer crop mode, alignment buttons move zoomed content within the fixed layer box; preserve zoom and support Undo.

- Snap mirrored/opposite resizing to exact edge contact between both moving layers, including unequal sizes; preserve anchors/ratios, Alt bypass and Undo.

- Distinguish completely out-of-view layers from partial/complete output-screen overflow; give navigation versus positioning advice. Rename Fit selected to Zoom to selection.

- Give the canvas more space: compact reference hint/status, tighter viewport margins and footer. Warnings still wrap. Fix resize edge snapping at fractional viewport scales.

- Add center to “Measure position from”; numeric resizing preserves center offsets. Project v6 saves the reference and migrates v1–5 corners.

- Rename Preferences to Settings; menu actions now read “Open all settings…”.

- Shift-drag moves/resizes on a 10 px grid; Ctrl+Shift uses 100 px, Alt bypasses. Replaces resize snap inversion; Shift-click and empty-canvas additive selection remain.

- Add a subtle “Double-click to edit” reminder in the layer-list header.

- Completed the input-preview backlog in this release: verified snapshot access, manual capture and optional timed refresh (see above).

- Reject negative/non-finite crop entries; crop-edge dragging stops at zero and zoom stays within 100–10000%. Signed content offsets remain supported.

- Keep the chosen resize mode through selection changes, Escape, Undo/Redo and project loads in the current session; single-layer resizing still works normally.

- “Options save automatically” is non-selectable informational text.

- Reorder layers with manager ↑/↓ buttons or drag-and-drop in the inspector. One-step Undo; IDs, links, crops and pending names follow the layer. Test save/reload and SVG order.

- `b251798`: Crop zoom/offsets and output resolution apply on edit; position/size applies when leaving a field. Pending drafts prompt Apply/Discard on close; Apply buttons highlight changes. Device sends remain explicit.
- Early apply-on-edit validation: workspace tests, fmt, Clippy and rustdoc passed; LLVM tools were unavailable then. Subsequent milestone checks include passing coverage (89.99%); see release notes for hardware/platform limitations.

## v0.5.0 — 2026-11-14

- `8322eab`: Tagged baseline before the apply-on-edit changes. Editor, saved screen/layer links, Preview loading and explicit device sends.
