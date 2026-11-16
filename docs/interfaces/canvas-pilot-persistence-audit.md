# Canvas Pilot persistence audit

Audited against the editor, project loader, application settings, capture controls,
layer tree, format properties, import/export controls and device transfer state.
Current ownership for [project version 17](canvas-pilot.md). This is an ownership audit:
not every control should become document data.

| Parameters | Owner and decision |
| --- | --- |
| Output resolution / aspect | Project: exact width/height. The output aspect preset is derived from them, not stored twice. |
| Layer position, dimensions, stacking order | Project: full-precision bounds and array order; display/export rounding never changes the saved values. |
| Input format, native dimensions, custom input crop, shared input slot | Project: `inputs` and layer `source_id`, already saved. |
| Layer format, dynamic crop, zoom and offset, explicit fit modes and blanking masks | Project: final bounds and advanced crop, already saved. Zoom/offset/aspect are derived from these authoritative values. |
| Names, stable IDs, next ID, folder membership, visibility, lock, pivot | Project, already saved. Folder visibility/locking acts on members, so there is no separate state to save. Explicit folder names are saved separately in project v12 and may have no members. |
| Device screen/layer mapping | Project, already saved per layer. Physical source routing is separate and is not silently changed by a geometry send. |
| Reference images, order, visibility, opacity, fit/native sizing, offset | Project, already embedded. Focused reference row is temporary. |
| Selected layers, viewport zoom/pan, positioning origins | Project, already saved. Selection is validated against visible/unlocked IDs. |
| Backdrop title, source guide pattern, outline mode, aspect-ratio lock, icon captions | Project, already saved. Pattern/ratio lock/captions also have application defaults. |
| Magnet, pixel/edge snapping, overlay visibility/divisions, overlay snapping | **Added to project**. Application settings remain the defaults for a new workspace; opening a project restores its setup. |
| 10/100-pixel tool grid, group resize mode | **Added to project**. They determine editing behavior and now follow the saved workspace rather than the previously opened document. |
| Collapsed folder rows | **Added to project**. Keeps larger memory collections manageable; stale folder names are filtered out. |
| Preview input, device-address binding, editor-source binding, enable flag | **Added to project** per stable layer ID. Preselected-but-not-yet-captured inputs are saved too. |
| Last captured still, guide opacity | **Added to project**. Original image bytes are embedded for offline reuse, subject to the shared 64 MiB image budget. |
| Auto-refresh choice, refresh interval, show no-signal inputs | **Added to project**. Auto-refresh restores paused; no fetch or connection is initiated by loading. Resume/capture is explicit. |
| Theme/recent themes, connection address/history, startup connection, allow Program | Application settings only. These belong to the workstation/operator; opening a document must not switch connections or global permissions. Capture host strings are bindings, not connection instructions. |
| All/selected/folder transfer scope, target bank, export sizing strategy | Session choices. Links and folders are persistent; the current transfer action and destination bank are not document content. Existing behavior is retained. |
| Advanced shared-device input edit checkbox and staged edits | Session-only by design. These are pending changes to external shared input settings; only explicit edits are sent. Local source/crop geometry is saved separately. |
| Import mapping, size/position/zoom toggles, ignored items/pages | Temporary import review. Applied results are saved; unfinished operations and their external payloads are not. |
| Geometry export codec, selection scope, background/placeholders options | Temporary export choices. No destination files are reopened or overwritten by project loading. |
| Alignment reference, prepared numeric ratio, active navigation/pivot/grid gesture, crop-edit mode, open tab/menu/dialog | Temporary editing context. No partial interaction is replayed; the resize mode and grid spacing themselves are saved as above. |
| Undo/redo, text drafts, drag snapshots, selection marquee, errors/progress, cached device capabilities, network jobs/timers | Transient runtime state. Loading starts a fresh document history and discards late capture results. |
| Named project path and last manual-save time | Recovery envelope only, not the portable project. Recovery uses the same project representation and image validation. |

The [application settings contract](canvas-pilot-settings.md) is unchanged (v8).
The [recovery envelope](canvas-pilot-recovery.md) is unchanged (v1); its nested
project migrates to v17. Geometry export representations are unchanged and omit
capture images/settings; optional fixed-reference backgrounds retain their
existing behavior.

Regression coverage includes current-schema round trips, versions 1–15 migration,
invalid assignments/settings and unknown/missing fields, embedded-still decoding,
atomic failure on malformed images, cleared stale previews, restore-without-fetch,
manual resumption eligibility and saved resize setup. The guide and example JSON
are updated with the schema.

## Version 16 addition: saved input proposals

Explicitly saved advanced input corrections now belong to the project as inert
`input_proposals`, maintained by `InputEdits.saved` and project save/restore.
The enable flag and staged device writes remain session-only. Opening/recovery
clears those flags and pending writes before installing saved proposals. Export
snapshots filter proposals to included, correctly mapped layers. The
[project contract](canvas-pilot.md#inactive-input-correction-proposals) defines
validation and the [advanced matrix](../canvas-pilot/guide/advanced-exchange.md)
distinguishes these proposals from source geometry and actual device routing.
