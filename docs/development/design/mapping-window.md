# Mapping window — visual study 01

[Open the standalone mockup](mapping-window.html) · [Development](../README.md)

Status: proposal for visual review, not an implemented application window.
Open the HTML file locally in a browser. It contains sample data, inline CSS and
native controls only: no JavaScript, external assets, storage, network requests
or device commands. Dropdown choices do not update other fields or counts.

## The idea

Choose an **editor layer** on the left. Read its routes from top to bottom:
**one device source → this editor layer → several device destinations**.
The source means the device layer from which geometry/appearance is loaded, not
its video input or signal routing. The actual Load and Send controls stay outside
this mapping editor; Apply mapping would only save routing choices.

The first connection is the proposed initial connection for a new load mapping;
that does not imply automatically choosing a device layer or performing a load.
A load source has three explicit parts: connection, device screen and device layer.
It can be absent independently of the send targets.

Send targets are grouped by connection, with screen/layer rows underneath. Repeat
an editor layer across different layers on one screen or across screens without
repeating the editor controls. Multiple connections are shown only as a collapsed
future example; their implementation is not part of this study.

The useful shortcut is **Follow load source** on the first send target. This is a
proposal, not a new saved setting: it would preserve today's simple synchronized
workflow while allowing extra independent destinations. Unlinking it would keep
its last destination, then allow editing it separately. Whether this should be a
persistent relationship or an explicit “Copy load source” action needs agreement
before changing the schema. Removing a load source must never silently delete or
retarget send destinations.

The list shows small source/target counts so a layer with no routes or several
destinations is easy to spot. It deliberately avoids a large checkbox matrix:
connection × screen × layer combinations would quickly overwhelm that layout.

## Questions for visual review

- Is the selected-layer approach comfortable, or is a project-wide route overview
  also needed? Start with this focused editor; an overview can remain a separate view.
- Does “Follow load source” communicate the link clearly? A linked row should be
  read-only in the real UI until unlinked; native controls here are illustrative.
- Should adding a destination open a searchable connection/screen/layer picker,
  then place the result in its connection group? This avoids three permanent
  selectors for connections that are already grouped.
- Should mapping edits use Apply/Cancel as shown, or immediate undoable changes?
  Apply/Cancel is a proposal for review, not a change to current behavior.

## States to design next, before implementation

Unmapped/send-only/load-only layers; unavailable connections; conflicting send
destinations; different screen resolutions; large layer lists; narrow windows;
keyboard navigation and long translations. A future connection selector needs
explicit identities and reconnection behavior, not automatic matching by name.

A resolution mismatch should remain an actionable validation result until an
explicit sizing policy is agreed. Do not invent automatic scaling in this window.
Multi-device execution and partial failures belong to a later transfer review.

The existing [stretch goal](../../canvas-pilot/plan.md#stretch-goals-distant-future)
and [mapping contract](../../interfaces/canvas-pilot.md#directional-device-mapping-version-19)
remain authoritative for implemented scope. This mockup changes neither.
