# Midra layer aspect: practical acceptance checklist

Exact pixel/subpixel measurements are **not required**. Precision work is deferred
to the [engineering stretch goal](midra-fit-precision.md).

## Confirmed by the operator

- [x] Crop is applied before every fit mode.
- [x] 1:1 is centered and keeps pixel-for-pixel scale regardless of layer resizing.
- [x] Centered preserves proportions and leaves transparent margins where needed.
- [x] Cropped preserves proportions, fills the container and clips overflow.
- [x] Fullscreen stretches independently to fill the container.
- [x] Explicit layer modes override input fill; Input setting delegates to it.

## Optional visual smoke test with ordinary equipment

- [ ] Save the device configuration and record model/firmware. Use Preview.
- [ ] Use a familiar image with a circle or face and obvious edges; set a layer
  rectangle with a different aspect ratio. Disable masks/effects for this check.
- [ ] In Canvas Pilot, open Layer properties → Crop → Advanced layer fit.
- [ ] Reset layer crop, choose Centered, send: proportions remain natural and
  unused sides show the background.
- [ ] Choose Fullscreen, send: image fills the rectangle and visibly stretches.
- [ ] Choose Cropped, send: image fills the rectangle without stretching and
  some content is cut off.
- [ ] Choose 1:1, send; resize the layer and send again: content scale stays the
  same, with more/less clipping or background visible.
- [ ] In 1:1, double-click the layer; drag a helper side/corner. The picture
  should crop without changing pixel scale; the retained region remains centered.
  Drag inside the layer (including an empty margin) to reposition the whole crop;
  its size and pixel scale must stay fixed. Check source-edge stops and arrow keys.
  Verify the crop fields, Undo and Escape. Zoom out the viewport if handles lie
  outside the screen.
- [ ] Apply a noticeable asymmetric crop; repeat modes and check that the crop
  stays applied. No ruler or pixel measurements are needed.
- [ ] Load the layer: mode, container and crop should return to the editor;
  save/reopen the project and check the same appearance.
- [ ] Uncheck Advanced layer fit: normal centered cover/zoom controls return.
- [ ] Repeat on Program only when it is safe and explicitly wanted.

Report the mode, what looked wrong, and device model/firmware if anything differs.
Automated tests verify math, persistence and mock protocol round trips. They do
not claim to verify the physical scaler or firmware-specific pixel placement.
