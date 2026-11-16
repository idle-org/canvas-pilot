# Conversation handoff — 2026-11-16

Read this when resuming work in the standalone Canvas Pilot repository. This is a
snapshot of conversation context, not a replacement for the current code, Git status,
[plan](../canvas-pilot/plan.md), [contracts](../interfaces/README.md) or
[AGENTS.md](../../AGENTS.md). Recheck status before acting on dated information.

## Where work stands

The migration and README redesign are complete. The user is reopening the
conversation in this project; no additional feature implementation is currently
underway. Application behavior was preserved during extraction.

- Local project: `/home/beffroi/Documents/Git/canvas-pilot`.
- Original project: sibling `../beffroi-tools`.
- Gitea: https://proxy.dandaedre.com/gitea/idle-org/canvas-pilot
- Public book: https://books.dandaedre.com/manuals/canvas-pilot/
- All 165 project-owned files were audited against the pre-migration source commit.
  See [migration provenance and inventory](migration.md).
- `AGENTS.md` contains repository conventions and a project/documentation map.
  [The documentation index](../README.md) links the advanced guide, task/test lists,
  stretch goals, engineer questions, adapter research and demonstration script.

## Git and publication state at handoff

The migration commit `4eef1bc` was pushed and verified on Gitea. Follow-up commits
`a595838` (audit/agent guidance) and `b2fcab4` (README redesign) were committed locally
but not pushed. This handoff is another local commit. Check
`git log origin/main..HEAD` and fetch when appropriate rather than assuming this
snapshot is still current. Do not push without a user request.

The original repository's extraction/link commit is `7b91ede`, committed locally
and not pushed. Its working tree was clean at the last check.

The Gitea repository description and avatar were updated through the API and
verified. The avatar is `crates/canvas-pilot/assets/app-icon.png`. The description
explains desktop layout editing, offline work and Midra/Aquilon geometry transfers.
These settings are server-side; they do not require a Git commit.

The new Jenkins job hookup has not been performed in this conversation. Setup is
in [CI](ci.md). The three new badge URLs returned HTTP 404 at the last check;
the README therefore keeps them in a collapsed Build status section pending the
first publication. No new book deployment was performed during migration.
Canvas Pilot publishes only its own book and badges; beffroi-tools retains ownership
of the shared manuals landing page. Never synchronize/delete that shared root.

A user-provided Gitea token remains at `../gitea-token.txt` on this workstation,
outside both repositories. It was successfully used for repository settings and
avatar upload. Do not copy or display it. Repository API operations worked; querying
`/api/v1/user` returned 403, so account-profile access must not be assumed. Gitea's
own `/swagger.v1.json` documents the API, including base64 repository-avatar upload.

## Decisions and preferences to retain

- The name is **Canvas Pilot**. It is independent software, not affiliated with
  Analog Way. Keep existing settings paths and serialized Midra identifiers compatible.
- Adapters should be first-class and practical, with a shared intermediate geometry
  representation and isolated family-specific behavior. Avoid abstraction for its
  own sake. Preserve targeted reads and partial writes for performance.
- Aquilon support is completed for the documented standalone AQL Cmax 6.2.x profile;
  do not reopen the completed task or extend its claim to untested configurations.
- Multiple editor screens are a distant stretch goal: a dropdown selects one screen
  at a time. Other adapters need protocol documentation and a usable emulator before
  development. Their ranking is research, not a promised release schedule.
- Export cleanup was explicitly marked solved. Saving a captured input as a separate
  image was removed from the backlog at the user's request; do not restore that task.
- The operator cannot perform exact pixel/subpixel measurements. Keep precision and
  mask-rasterization questions in the engineer-facing stretch goal; do not make those
  measurements a prerequisite for ordinary operator acceptance.
- Explicit layer aspect modes take precedence over shared input fill. Crop is applied
  first; 1:1 uses centered native pixels, Fullscreen stretches, Centered fits inside,
  and Cropped fills/clips. Advanced fit stays behind an opt-in control. Existing
  helpers support the intended crop/move interactions, and masking uses a separate
  magenta box. Consult the code and guide before altering their behavior.

## Pending input-ratio scope

The user described correcting a stretched/deformed input, usually left on Native,
with preset or custom ratios. Manual research established the distinction between
**Content ratio** and **Transform to**; the result is already recorded in
[Formats and crop](../canvas-pilot/guide/formats-and-crop.md#content-ratio-versus-transform-to).
Advanced device controls already exist. A follow-up question about adding a **local
editor correction/preview model** was not answered before the migration.
Do not treat this as an implemented feature or a finalized new schema. Resume from
the documented distinction and clarify the desired local behavior when that work
is taken up; preserve the input-agnostic editing workflow and opt-in shared writes.

## Video plans

The user plans a **showcase video** and a **video manual**, and asked whether videos
also belong in the book. No videos, hosting choice or embed implementation exist yet.
The [feature demonstration checklist](../canvas-pilot/feature-demo-checklist.md)
already supplies an ordered recording script.

The user approved the following approach and asked to put it in the stretch goals:

- Showcase near the top of the README and book introduction.
- A walkthrough linked from quick start, with chapter timestamps.
- Optional short demonstrations beside difficult interactions: crop, 1:1 movement,
  masks and device mapping/transfers. Start with the two main videos first.
- Written instructions remain the searchable reference; captions, no autoplay, and
  the application version help viewers. Host large video files outside Git and link
  or embed them once a host is chosen.

The existing screenshots are real captures from before the Canvas Pilot rename;
the redesigned README explicitly labels them. Their recipes and provenance are in
[the screenshot guide](../canvas-pilot/images/README.md). Do not edit old screenshots
to imply a new capture; recapture the application if updating them.

## Device testing context

The user explicitly authorized testing and writes on the emulator at
`192.168.10.146:3000` and later said it was online again. At the last recorded work
it was an Aquilon emulator; AWJ TCP 10606 was also needed. This is historical scope,
not evidence that the endpoint is online or unchanged today. Discover identity
before writing, preserve/restore the baseline, and keep emulator results distinct
from physical-output acceptance.

The physical Midra at `192.168.10.143:80` was used read-only in the recent work;
the emulator authorization does not extend to that device. No device connection
is needed for repository, documentation or routine automated tests.

## Last validation evidence

The extraction passed formatting, both Clippy feature configurations, rustdoc,
Python tests and Rust tests. Canvas Pilot had 395 passing default-feature tests and
390 in each restricted configuration; the remaining workspace had 149 Rust tests.
Measured line coverage was 91.03% for Canvas Pilot and 89.83% for beffroi-tools
against a 75% floor. These are historical measurements, not live CI badges.

The expanded book built with pinned mdBook 0.5.4 and validated 41 pages and 1,826
local links/assets. The README was rendered locally and visually inspected;
local links and badge tests passed. Subsequent documentation edits should rebuild
the book; code changes need the relevant checks in [CI](ci.md).
