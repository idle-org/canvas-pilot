//! Selection guides stay above layer artwork; covered edge segments are dashed.
use super::*;

/// Visible picture bounds of higher layers, excluding hidden and blanked regions.
pub(super) fn covers(layout: &Layout, index: usize, origin: Pos2, scale: f32) -> Vec<Rect> {
    let to_rect = |g: Geometry| {
        Rect::from_min_size(
            origin + Vec2::new(g.x as f32, g.y as f32) * scale,
            Vec2::new(g.width as f32, g.height as f32) * scale,
        )
    };
    layout
        .viewport_order()
        .into_iter()
        .skip_while(|&i| i != index)
        .skip(1)
        .filter(|&i| layout.visible(i))
        .filter_map(|i| {
            let g = layout.layers()[i];
            let formats = layout.formats();
            let mut rect = to_rect(g).intersect(to_rect(formats.mask_bounds(i, g).ok()?));
            if let Some(fit) = formats.layer_fit[i] {
                let retained = formats.advanced[i].remaining(formats.input(i).ok()?).ok()?;
                rect = rect.intersect(to_rect(fit.image(retained, g).ok()?));
            }
            rect.is_positive().then_some(rect)
        })
        .collect()
}

// Merge covered intervals before painting: overlaps must not fill dash gaps twice.
fn intervals(start: f32, end: f32, fixed: f32, horizontal: bool, covers: &[Rect]) -> Vec<[f32; 2]> {
    let mut spans: Vec<_> = covers
        .iter()
        .filter_map(|rect| {
            let (min, max, cross_min, cross_max) = if horizontal {
                (rect.left(), rect.right(), rect.top(), rect.bottom())
            } else {
                (rect.top(), rect.bottom(), rect.left(), rect.right())
            };
            let span = [start.max(min), end.min(max)];
            (fixed >= cross_min && fixed <= cross_max && span[0] < span[1]).then_some(span)
        })
        .collect();
    spans.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut merged: Vec<[f32; 2]> = Vec::new();
    for span in spans {
        if let Some(last) = merged.last_mut()
            && span[0] <= last[1]
        {
            last[1] = last[1].max(span[1]);
        } else {
            merged.push(span);
        }
    }
    merged
}

pub(super) fn paint(painter: &egui::Painter, rect: Rect, covers: &[Rect], stroke: Stroke) {
    let clip = painter.clip_rect();
    for (horizontal, fixed, start, end) in [
        (true, rect.top(), rect.left(), rect.right()),
        (true, rect.bottom(), rect.left(), rect.right()),
        (false, rect.left(), rect.top(), rect.bottom()),
        (false, rect.right(), rect.top(), rect.bottom()),
    ] {
        let (min, max, cross_min, cross_max) = if horizontal {
            (clip.left(), clip.right(), clip.top(), clip.bottom())
        } else {
            (clip.top(), clip.bottom(), clip.left(), clip.right())
        };
        if fixed < cross_min || fixed > cross_max {
            continue;
        }
        let (start, end) = (start.max(min), end.min(max));
        if start >= end {
            continue;
        }
        let point = |v| {
            if horizontal {
                Pos2::new(v, fixed)
            } else {
                Pos2::new(fixed, v)
            }
        };
        let mut cursor = start;
        for [from, to] in intervals(start, end, fixed, horizontal, covers) {
            if cursor < from {
                painter.line_segment([point(cursor), point(from)], stroke);
            }
            // Clip first so zooming into a huge layer never generates off-screen dashes.
            painter.extend(egui::Shape::dashed_line(
                &[point(from), point(to)],
                stroke,
                6.0,
                4.0,
            ));
            cursor = to;
        }
        if cursor < end {
            painter.line_segment([point(cursor), point(end)], stroke);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coverage_merges_overlaps_clips_and_keeps_uncovered_edges_solid() {
        let rects = [
            Rect::from_min_max(Pos2::new(10.0, 0.0), Pos2::new(40.0, 50.0)),
            Rect::from_min_max(Pos2::new(30.0, 0.0), Pos2::new(60.0, 50.0)),
            Rect::from_min_max(Pos2::new(80.0, 0.0), Pos2::new(100.0, 50.0)),
        ];
        assert_eq!(
            intervals(20.0, 90.0, 25.0, true, &rects),
            [[20.0, 60.0], [80.0, 90.0]]
        );
        assert!(intervals(0.0, 100.0, 51.0, true, &rects).is_empty());
        assert_eq!(intervals(10.0, 70.0, 35.0, false, &rects), [[10.0, 50.0]]);
    }
    #[test]
    fn only_visible_higher_picture_regions_cover_guides() {
        let mut layout = Layout::default();
        layout.select(Some(1));
        layout
            .apply(
                1,
                Geometry {
                    x: 0.0,
                    y: 0.0,
                    width: 600.0,
                    height: 600.0,
                },
            )
            .unwrap();
        assert!(covers(&layout, 1, Pos2::ZERO, 1.0).is_empty());
        layout
            .set_mask(
                1,
                canvas_pilot::formats::Mask {
                    left: 0.25,
                    ..Default::default()
                },
            )
            .unwrap();
        let covered = covers(&layout, 0, Pos2::new(10.0, 20.0), 2.0);
        assert_eq!(
            covered,
            [Rect::from_min_max(
                Pos2::new(310.0, 20.0),
                Pos2::new(1210.0, 1220.0)
            )]
        );
        layout.set_visible(1, false);
        assert!(covers(&layout, 0, Pos2::ZERO, 1.0).is_empty());
        layout.set_visible(1, true);
        layout.set_mask(1, Default::default()).unwrap();
        layout
            .set_layer_fit(
                1,
                Some(canvas_pilot::formats::LayerFit::Centered),
                Default::default(),
            )
            .unwrap();
        let covered = covers(&layout, 0, Pos2::ZERO, 1.0);
        assert!(covered[0].height() < 600.0);
        assert_eq!(covered[0].center(), Pos2::new(300.0, 300.0));
    }
    #[test]
    fn resize_crop_and_mask_handles_stay_above_covering_artwork() {
        use super::super::tests::frame;
        for mode in 0..3 {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            let g = Geometry {
                x: 200.0,
                y: 200.0,
                width: 640.0,
                height: 360.0,
            };
            for i in 0..2 {
                app.layout.select(Some(i));
                app.layout.apply(i, g).unwrap();
            }
            app.layout.select(Some(0));
            app.edit_content = mode != 0;
            app.edit_mask = mode == 2;
            app.sync();
            let before = app.project();
            frame(&mut app, &ctx, vec![]);
            let output = frame(&mut app, &ctx, vec![]);
            let palette = theme::palette(&ctx);
            let color = match mode {
                0 => palette.amber,
                1 => palette.cyan,
                _ => Color32::from_rgb(222, 108, 220),
            };
            let size = if mode == 0 { 7.0 } else { 9.0 };
            let handles: Vec<_> = output
                .shapes
                .iter()
                .enumerate()
                .filter_map(|(index, shape)| {
                    if let egui::Shape::Rect(rect) = &shape.shape
                        && (rect.rect.size() - Vec2::splat(size)).length() < 0.001
                        && rect.stroke.color == color
                    {
                        Some((index, rect.rect.center()))
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(handles.len(), 8, "mode {mode}");
            let top_artwork = output
                .shapes
                .iter()
                .rposition(|shape| {
                    matches!(&shape.shape,
                egui::Shape::Rect(rect) if rect.fill == palette.layer && rect.rect.width() > 100.0)
                })
                .unwrap();
            assert!(handles.iter().all(|(index, _)| *index > top_artwork));
            let dashes = output.shapes.iter().filter(|shape| matches!(&shape.shape,
                egui::Shape::LineSegment { points, stroke } if stroke.color == color && points[0].distance(points[1]) <= 6.01)).count();
            assert!(dashes > 20, "covered guide should be dashed in mode {mode}");
            assert_eq!(app.project(), before);
        }
    }

    #[test]
    fn dashed_paint_is_bounded_by_viewport_at_extreme_zoom() {
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui
                .painter()
                .with_clip_rect(Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0)));
            let rect = Rect::from_min_max(Pos2::new(-1e9, 20.0), Pos2::new(1e9, 80.0));
            paint(&painter, rect, &[rect], Stroke::new(1.0, Color32::GOLD));
        });
        let lines = output
            .shapes
            .iter()
            .filter(|shape| matches!(shape.shape, egui::Shape::LineSegment { .. }))
            .count();
        assert_eq!(lines, 20);
    }
}
