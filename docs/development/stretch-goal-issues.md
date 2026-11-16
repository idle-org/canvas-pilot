# Stretch-goal issue drafts

[Development](README.md) · [Project plan](../canvas-pilot/plan.md#stretch-goals-distant-future)

These four issues are ready to publish. The supplied token can access the
repository but Gitea rejected issue reads and creation with HTTP 403: missing
`read:issue` and `write:issue`. No new issues were created. Publication requires
a token with Issues: Read and Write access; publication status is recorded in
the project plan. Copy each title and body below into a new issue, or publish
with an issue-enabled token after checking for duplicates.

The fifth stretch goal, additional presentation-switcher adapters, is already
tracked in [Gitea issue #1](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/issues/1).
Reuse that issue instead of creating another. These are future goals, not release
commitments. The plan and linked roadmaps remain the detailed source documents.

## Localize the manual first, then the application

**Issue title:** Localize the manual first, then the application

### Issue body

Make Canvas Pilot usable in other languages, starting with the book and then the application. This is a distant-future goal with no committed release date. English remains the source language; French is a suggested first translation, subject to confirming a reviewer and the initial language scope.

## Phase 1: book

- [ ] Confirm the first language, reviewer and release scope; establish a glossary shared with the future UI translation.
- [ ] Pilot one chapter and the feature matrix before choosing translated Markdown or message catalogs. Translate canonical sources and track their source revision.
- [ ] Build each language with its own navigation and output path while preserving existing English and template-download URLs.
- [ ] Provide a chapter-aware language selector and clearly identified English fallback.
- [ ] Review translated installation, quick start, format matrices, template catalog and advanced instructions. Keep exact English control labels while the application remains English-only.
- [ ] Validate links, anchors, search, print view, images, tables, downloads and publication boundaries with the pinned mdBook version.
- [ ] Document contribution, review and update procedures before publishing the first language.

## Phase 2: application

Start after the book workflow and glossary have been validated.

- [ ] Inventory visible text and choose a small localization mechanism with stable keys, complete messages, named placeholders, plurals and English fallback.
- [ ] Add a workstation language preference, including System default and defined region/fallback behavior. Preserve unsaved work when switching languages.
- [ ] Keep project schemas, protocol identifiers, file extensions, numeric serialization and clipboard exchange language-independent. Never rename existing layers or folders.
- [ ] Define decimal-entry behavior without ambiguous measurements or changes to device writes.
- [ ] Validate long translations, font coverage, shortcuts, scaling, small windows, missing messages and translated Help links.
- [ ] Review recovery, overwrite and Preview/Program wording; test cross-language clipboard/project interoperability and run the existing Rust/coverage gates.

## Later, when needed

Evaluate right-to-left layout and shaping before advertising languages that require them. Add translated video captions when videos exist; consider a hosted translation service only if contributor volume warrants it.

## Completion criteria and reference

The first agreed language is reviewed and usable in the book and application, with a maintainable update process and unchanged file/device compatibility. Detailed implementation and acceptance tasks remain in the [localization roadmap](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/src/branch/main/docs/development/localization.md).

<!-- canvas-pilot-stretch:localization -->

## Create a showcase video and guided video manual

**Issue title:** Create a showcase video and guided video manual

### Issue body

Provide two complementary videos: a short feature showcase and a practical guided manual. Written instructions remain the searchable reference. This approach is approved as a distant-future goal; recording, hosting and implementation are still pending, with no committed release date.

## Scope

- [ ] Record a short showcase for the README and book introduction.
- [ ] Record a guided video manual linked from quick start, with chapter timestamps.
- [ ] Use the existing [feature demonstration checklist](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/src/branch/main/docs/canvas-pilot/feature-demo-checklist.md) as the recording outline, checking current UI labels before recording.
- [ ] Select a host and keep large video files outside Git.
- [ ] Include captions and the recorded application version; provide descriptive links and no autoplay.
- [ ] Link or embed the videos in the book without replacing the written walkthroughs.
- [ ] After the two main videos, consider short chapter demonstrations where motion helps: normal crop controls, 1:1 crop movement, masks and device mapping/transfers.

## Completion criteria

The showcase and timestamped manual are published and linked from their intended entry points. Captions and version information are available, the book remains useful without playback, and links/embeds work on narrow screens. Optional chapter clips are considered separately after the two main videos.

<!-- canvas-pilot-stretch:videos -->

## Confirm Midra fit and mask rasterization with a firmware engineer

**Issue title:** Confirm Midra fit and mask rasterization with a firmware engineer

### Issue body

Obtain authoritative Midra crop, fit and mask rasterization rules from an Analog Way firmware or video-processing engineer. Current continuous crop-first geometry and nearest-integer device fields are working assumptions, not a verified hardware raster model.

This is a distant-future precision follow-up, not a prerequisite for using the existing fit modes. Operator pixel measurements are not required: the available equipment cannot establish subpixel rendering, and API readback alone cannot prove physical output.

## Questions and evidence

- [ ] Confirm normalized crop units, their coordinate stage and when retained dimensions are quantized.
- [ ] Confirm crop-first fitting for 1:1, Centered, Fullscreen and Cropped, including fractional retained sizes.
- [ ] Establish pixel-center conventions, half-pixel tie-breaking, interpolation, negative centers and clipping/transparent-edge rules.
- [ ] Establish mask coordinates relative to container/fitted image, composition with crop/fit, fractional boundary rounding and inclusive/exclusive edges, including fully blank axes.
- [ ] Identify any Preview/Program, model, firmware or scaler differences.
- [ ] Obtain rendered reference images, expected pixel bounds or an instrumented simulator rather than asking operators for unavailable measurements.

## Acceptance criteria

- [ ] Record the authoritative answers with model/firmware scope and provenance.
- [ ] Cover odd-sized containers, asymmetric crops/masks, smaller and larger 1:1 sources, negative coordinates and both banks using the concrete cases in the [engineering brief](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/src/branch/main/docs/canvas-pilot/midra-fit-precision.md).
- [ ] Add sanitized reference fixtures and adjust adapter quantization/rendering only where the evidence requires it.
- [ ] Keep software math tests, protocol readback and physical-output validation explicitly distinct; document remaining uncertainties.

<!-- canvas-pilot-stretch:midra-precision -->

## Support multiple project screens with an active-screen dropdown

**Issue title:** Support multiple project screens with an active-screen dropdown

### Issue body

Allow a project to contain several screen layouts and select the active screen from a dropdown. Display and edit **one screen at a time**, preserving each layout when switching. This is a distant-future goal with no committed release date; simultaneous multi-screen editing is outside its scope.

## Scope and decisions

- [ ] Define screen identity, naming, creation/removal and active-screen selection with a small, clear UI.
- [ ] Define which state belongs to each screen and which is shared: output dimensions, layers, source relationships, references, workspace state and device mappings.
- [ ] Preserve each screen's layout and settings when switching without implicit device actions.
- [ ] Define how import/export, recovery, undo and device transfer scopes relate to the active screen; make the selected scope visible.
- [ ] Document project persistence and version migration before implementation. Existing single-screen projects must keep their layout and behavior.

## Acceptance criteria

- [ ] Create at least two distinct screen layouts, switch between them and recover their independent geometry/settings unchanged.
- [ ] Save/reopen and recover a multi-screen project without losing screen identity or content.
- [ ] Migrate existing single-screen projects through tested fixtures, with clear errors for unsupported versions.
- [ ] Verify that import/export and explicit device operations affect only the documented scope, with no automatic send or source-routing changes on screen selection.
- [ ] Update the manual, persistence contracts and meaningful regression tests.

Reference: [project stretch goals](https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot/src/branch/main/docs/canvas-pilot/plan.md#stretch-goals-distant-future).

<!-- canvas-pilot-stretch:multiple-screens -->
