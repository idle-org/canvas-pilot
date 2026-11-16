# Viewport performance

[Canvas Pilot](../introduction.md) · [Guide contents](overview.md) · [Starter files](../../templates/canvas-pilot/catalog.md)

## Viewport capacity check

Run the explicit headless timing check with:

```sh
cargo test --locked --release -p canvas-pilot --bin canvas-pilot viewport_capacity_timing -- --ignored --nocapture
```

On the development Linux host (2026-11-13), 256 visible layers at 1440×940
measured p50 **0.73 ms**, p95 **0.78 ms** across 100 frames after 20 warm-up frames.
This measures egui CPU frame construction, not GPU presentation, image decoding,
OS/DPI behavior or performance on an operator laptop. It is not a release gate.
