# Midra layer fit precision: engineering follow-up

Status: **done for now**, closed for current scope on 2026-11-18. The questions
below are retained for a possible future investigation, not an active stretch
goal. Closure does not establish a verified firmware raster model. The operator
cannot measure subpixel output with their available equipment. API readback alone
cannot establish how the scaler renders pixels. Ask an Analog Way firmware or
video-processing engineer for the following rules, ideally with reference results
or an instrumented simulator. Record model and firmware with every answer.

## Confirmed behavior and current implementation

Operator-confirmed: crop precedes fit; 1:1 is centered pixel-for-pixel; Fullscreen
stretches to the container; Centered fits with transparent margins; Cropped fills
and clips. Explicit modes override input fill. Input/global inheritance is outside
this task. Canvas Pilot uses continuous double-precision geometry, centered
placement, and clipping to the container. For retained `(rw,rh)` in `(w,h)`, scales
are 1, min(w/rw,h/rh), independent axis ratios, and max(w/rw,h/rh), respectively.

Only device writes quantize: dimensions/centers to whole pixels and combined crop
edges to `round(edge/native_axis*65535)`, nearest with halves away from zero.
These are sensible working assumptions, not a verified firmware raster model.

## Questions to send an engineer

1. Are normalized crop edges exactly fractions of 65535, and of which dimensions:
   incoming signal, reported content, or the image after input correction? At what
   stage are fractional retained dimensions quantized, if at all?
2. Is fitting based on fractional retained dimensions or rounded ones? Does each
   explicit mode use the formulas above, after crop, with no further aspect step?
3. For odd container/image sizes, which pixel-center convention and tie-breaking
   rules place the image? Does a half-pixel offset resample 1:1, snap left/right,
   or clip an extra pixel on one edge? How are negative centers handled?
4. Are clipping edges inclusive/exclusive and shared consistently across modes?
   Are uncovered Centered/1:1 pixels fully transparent before compositing/effects?
5. Do Preview and Program or different firmware/scalers apply different rules?
   Can you provide rendered reference images or expected edge coordinates without
   requiring operator measurement tools?

## Concrete reference cases to request

Use an uncorrected 1920×1080 input with no masks/effects. Run all four modes:

- Container 800×600 centered at (960,540), zero crop. Continuous Centered image
  is 800×450, top-left (560,315); Cropped is 1066.666667×600 before clipping.
- Container 801×601 at the same center. Centered is 801×450.5625, top-left
  (559.5,314.71875). Request exact pixel coverage and interpolation behavior.
- Container 800×600, layer crop left/right/top/bottom = 123/45/67/21 source
  pixels. Send normalized fields using the formula above, then request the
  retained dimensions and rendered bounds from those exact transmitted values.
- A 640×360 source in an 801×601 container in 1:1; then a larger-than-container
  source. Request placement, transparent margins and symmetric clipping rules.
- Repeat an odd-size case near a negative output coordinate and in both banks.

## Completion criteria

Record the engineer's authoritative rules, firmware scope and reference outputs;
add regression fixtures covering odd sizes, asymmetric crops and both banks;
adjust adapter quantization if needed. Validate rendered output against those
references when a suitable tool exists. Keep software math tests and API readback
separate from physical output verification.

### Mask rasterization question for the device engineer

WebRCS exposes mask edges as 0–65535 fractions of the live layer size. Please
confirm the precise coordinate stage (container versus fitted image), composition
with crop and all four explicit aspects, rounding of fractional mask boundaries,
and whether edges are inclusive/exclusive when opposite values sum to 65535.
Supply expected pixel bounds or a reference render for asymmetric masks on an
offset, odd-sized container with crop and 1:1/Centered/Fullscreen/Cropped fit.
Our current implementation clips the rendered image by the container-relative
reveal rectangle, allows fully blank axes, and never rescales the image.
This needs engineering/reference-render validation, not manual operator measurements.
