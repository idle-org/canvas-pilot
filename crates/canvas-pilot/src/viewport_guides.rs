use super::*;

fn ticks(min: f32, max: f32, origin: f32, scale: f32, extent: u32, step: u32) -> Vec<(f32, u32)> {
    if !scale.is_finite() || scale <= 0.0 || step == 0 {
        return Vec::new();
    }
    let first = (((min - origin) / scale).max(0.0) / step as f32).ceil() as u32;
    let last = (((max - origin) / scale).min(extent as f32) / step as f32)
        .floor()
        .max(0.0) as u32;
    if first > last || last - first > 4096 {
        return Vec::new();
    }
    (first..=last)
        .filter_map(|n| {
            let value = n * step;
            let p = origin + value as f32 * scale;
            (value <= extent && p >= min && p <= max).then_some((p, value))
        })
        .collect()
}
/// High-zoom measurement guides. Drawing cost depends on visible pixels, not canvas size.
pub(super) fn paint(
    painter: &egui::Painter,
    area: Rect,
    origin: Pos2,
    scale: f32,
    output: [u32; 2],
) {
    if scale < 2.0 {
        return;
    }
    let palette = theme::palette(painter.ctx());
    let canvas = Rect::from_min_size(
        origin,
        Vec2::new(output[0] as f32, output[1] as f32) * scale,
    );
    let clipped = painter.with_clip_rect(area.intersect(canvas));
    if scale >= 8.0 {
        for (x, _) in ticks(area.left(), area.right(), origin.x, scale, output[0], 1) {
            clipped.line_segment(
                [Pos2::new(x, area.top()), Pos2::new(x, area.bottom())],
                Stroke::new(0.5_f32, palette.grid.gamma_multiply(0.5)),
            );
        }
        for (y, _) in ticks(area.top(), area.bottom(), origin.y, scale, output[1], 1) {
            clipped.line_segment(
                [Pos2::new(area.left(), y), Pos2::new(area.right(), y)],
                Stroke::new(0.5_f32, palette.grid.gamma_multiply(0.5)),
            );
        }
    }
    let step = [1, 2, 5, 10, 20, 50, 100]
        .into_iter()
        .find(|step| *step as f32 * scale >= 48.0)
        .unwrap_or(100);
    let ruler = painter.with_clip_rect(area);
    for (x, n) in ticks(
        area.left() + 28.0,
        area.right() - 10.0,
        origin.x,
        scale,
        output[0],
        step,
    ) {
        ruler.line_segment(
            [Pos2::new(x, area.top()), Pos2::new(x, area.top() + 4.0)],
            Stroke::new(1.0_f32, palette.muted),
        );
        ruler.text(
            Pos2::new(x, area.top() + 5.0),
            Align2::CENTER_TOP,
            n.to_string(),
            FontId::proportional(10.0),
            palette.muted,
        );
    }
    for (y, n) in ticks(
        area.top() + 24.0,
        area.bottom() - 10.0,
        origin.y,
        scale,
        output[1],
        step,
    ) {
        ruler.line_segment(
            [
                Pos2::new(area.left() + 35.0, y),
                Pos2::new(area.left() + 39.0, y),
            ],
            Stroke::new(1.0_f32, palette.muted),
        );
        ruler.text(
            Pos2::new(area.left() + 3.0, y),
            Align2::LEFT_CENTER,
            n.to_string(),
            FontId::proportional(9.0),
            palette.muted,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guides_follow_pixel_coordinates_and_stay_bounded_when_panned() {
        assert_eq!(
            ticks(0.0, 100.0, -20.0, 10.0, 1920, 1),
            (2..=12)
                .map(|n| (-20.0 + n as f32 * 10.0, n))
                .collect::<Vec<_>>()
        );
        assert!(ticks(0.0, 100.0, 1000.0, 10.0, 1920, 1).is_empty());
        assert!(ticks(0.0, 100.0, -200000.0, 10.0, 1920, 1).is_empty());
        assert!(ticks(0.0, 1000.0, -10000.0, 256.0, 16384, 1).len() < 6);
        assert!(ticks(0.0, 100.0, 0.0, 0.0, 1920, 1).is_empty());
    }
}
