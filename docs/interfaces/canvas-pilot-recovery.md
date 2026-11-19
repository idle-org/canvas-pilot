# Canvas Pilot recovery, version 1

`last_project.json` lives beside the per-user `settings.json` (see
[settings paths](canvas-pilot-settings.md)). It is an internal UTF-8 JSON envelope,
not a named project export. Headless/default editors never access this path.

| Field | Meaning |
| --- | --- |
| `version` | Integer `1`; other versions rejected |
| `project` | Complete [project v21](canvas-pilot.md), including embedded reference bytes; its units, precision and validation apply without display rounding |
| `project_path` | Last opened/successfully saved named JSON path, or null; geometry exports do not change it |
| `manual_saved_at` | Unix UTC seconds of the last successful explicit project save, or null when unknown; recovery and measurement-point autosaves do not update it |

Unknown envelope fields and files over 100 MiB are rejected. Project validation
and reference decoding must succeed before replacing the editor. No file at
`project_path` is opened during recovery; it identifies the target for subsequent
measurement-point autosaves. The startup device connection remains governed by
application settings, not recovery.

Startup/Retry reads, validation and embedded-image decoding run on a background
worker. No partial project is installed. Checkpoints (including normal-exit
checkpoints) are suspended until loading completes, so an unfinished or failed
restore cannot overwrite the previous recovery file. Failures retain the file
and pause recovery as before. The envelope remains version 1.

Checkpoints run at most every two seconds, outside active pointer/resize gestures,
and on normal exit. Identical snapshots are skipped. Writes use the existing
same-directory temporary-file, file-sync and atomic replacement helper. A failed
write leaves the previous file intact and pauses retries until requested. Invalid
startup recovery is preserved; Retry reloads it, while explicitly starting New
project permits replacement. Multiple app instances share this single slot;
last successful writer wins.

Recovery retains applied values only: no undo history, pending text/format/import
drafts, transient tools, or credentials. A normal exit cancels unfinished canvas
gestures before the final checkpoint. The schema does not replace named project
JSON: Save continues to emit a bare project v21 document. Opening/New replaces the
single recovery slot; New warns with the manual-save age and Save/Discard/Cancel.
A failed or cancelled manual save never completes Save-and-create-new.

[Legacy recovery example (embedded project v9, migrated to v16)](canvas-pilot-recovery-v1.example.json) is exercised by the startup regression test.

Future breaking envelope changes require a version bump and explicit migration
or rejection. Recovery is not an interchange format; use Save for portable files.

Recovery envelope version 1 is unchanged. Nested project versions 1–11 migrate
through the normal project loader before typed decoding, including the former
single `reference` field. All reference images and visibility settings are saved.
