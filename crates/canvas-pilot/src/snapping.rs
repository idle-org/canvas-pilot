//! Proximity snapping for layer movement and anchored single/group resizing.
use crate::{Geometry, Layout, ResizeHandle, ResizeMode};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapping {
    pub enabled: bool,
    pub pixels: bool,
    pub screen_edges: bool,
    pub layer_edges: bool,
    pub mask_edges: bool,
    pub grid: bool,
    pub show_grid: bool,
    pub grid_divisions: [u32; 2],
}
impl Default for Snapping {
    fn default() -> Self {
        Self {
            enabled: false,
            pixels: true,
            screen_edges: true,
            layer_edges: true,
            mask_edges: false,
            grid: false,
            show_grid: true,
            grid_divisions: [8, 6],
        }
    }
}
impl Snapping {
    /// Output-relative overlay lines, bounded for rendering and snapping alike.
    pub fn grid_guides(self, size: u32, axis: usize) -> Vec<f64> {
        let count = self.grid_divisions[axis].clamp(1, 64);
        (0..=count)
            .map(|i| size as f64 * i as f64 / count as f64)
            .collect()
    }
}
// Screen coordinates remain fractional for custom resolutions not divisible by four.
fn screen_guides(size: u32) -> [f64; 5] {
    [0.0, 0.25, 0.5, 0.75, 1.0].map(|fraction| size as f64 * fraction)
}

// Mask fractions are resolved through the same source/layer reference as rendering.
// Clip to the layer aperture; a fully blanked layer has no visible snap edges.
fn mask_geometry(formats: &crate::formats::Formats, index: usize, g: Geometry) -> Option<Geometry> {
    let mask = formats.mask_bounds(index, g).ok()?;
    let picture = if let Some(fit) = formats.layer_fit[index] {
        fit.image(
            formats.advanced[index]
                .remaining(formats.input(index).ok()?)
                .ok()?,
            g,
        )
        .ok()?
    } else {
        g
    };
    let x = g.x.max(mask.x).max(picture.x);
    let y = g.y.max(mask.y).max(picture.y);
    let width = (g.x + g.width)
        .min(mask.x + mask.width)
        .min(picture.x + picture.width)
        - x;
    let height = (g.y + g.height)
        .min(mask.y + mask.height)
        .min(picture.y + picture.height)
        - y;
    (width > 0.0 && height > 0.0).then_some(Geometry {
        x,
        y,
        width,
        height,
    })
}

fn edges(g: Geometry, axis: usize) -> [f64; 2] {
    if axis == 0 {
        [g.x, g.x + g.width]
    } else {
        [g.y, g.y + g.height]
    }
}

impl Snapping {
    fn targets(
        self,
        formats: &crate::formats::Formats,
        index: usize,
        g: Geometry,
        axis: usize,
    ) -> impl Iterator<Item = f64> {
        let mask = self
            .mask_edges
            .then(|| mask_geometry(formats, index, g))
            .flatten();
        self.layer_edges
            .then_some(g)
            .into_iter()
            .flat_map(move |g| edges(g, axis))
            .chain(mask.into_iter().flat_map(move |g| {
                let [low, high] = edges(g, axis);
                [low, high, g.center()[axis]]
            }))
    }
}

impl Layout {
    /// Snap a pivot without editing geometry. Local anchors take precedence over
    /// grid/edge options; the caller may bypass them for precise free placement.
    pub fn snap_pivot(
        &self,
        index: usize,
        point: [f64; 2],
        options: Snapping,
        tolerance: f64,
        grid: Option<f64>,
        anchors: bool,
    ) -> ([f64; 2], [Option<f64>; 2]) {
        let Some(g) = self.layers.get(index).copied() else {
            return (point, [None; 2]);
        };
        if !tolerance.is_finite() || tolerance < 0.0 || point.iter().any(|v| !v.is_finite()) {
            return (point, [None; 2]);
        }
        if anchors {
            let distance = |p: [f64; 2]| (p[0] - point[0]).hypot(p[1] - point[1]);
            if let Some(target) = ResizeHandle::positions(g)
                .into_iter()
                .map(|(_, p)| p)
                .chain(std::iter::once(g.center()))
                .filter(|p| distance(*p) <= tolerance)
                .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
            {
                return (target, target.map(Some));
            }
        }
        if let Some(spacing) = grid.filter(|s| s.is_finite() && *s > 0.0) {
            let target = point.map(|v| (v / spacing).round() * spacing);
            return (target, target.map(Some));
        }
        if !options.enabled {
            return (point, [None; 2]);
        }
        let mut target = point;
        let mut guides = [None; 2];
        for axis in 0..2 {
            let mut edges = Vec::new();
            if options.screen_edges {
                edges.extend(screen_guides(self.formats.output[axis]));
            }
            if options.grid {
                edges.extend(options.grid_guides(self.formats.output[axis], axis));
            }
            if options.layer_edges || options.mask_edges {
                for (i, other) in self.layers.iter().enumerate() {
                    if i != index && self.visible(i) {
                        edges.extend(options.targets(&self.formats, i, *other, axis));
                    }
                }
            }
            if let Some(edge) = edges
                .into_iter()
                .filter(|e| (*e - point[axis]).abs() <= tolerance)
                .min_by(|a, b| (a - point[axis]).abs().total_cmp(&(b - point[axis]).abs()))
            {
                target[axis] = edge;
                guides[axis] = Some(edge);
            } else if options.pixels {
                target[axis] = point[axis].round();
            }
        }
        (target, guides)
    }
    /// Recompute from the gesture's original snapshot, never from a previous snapped frame.
    /// `tolerance` is in screen pixels; the UI converts its fixed logical hit distance.
    /// Layer resize handles preserve anchors/ratios; content crop gestures do not snap.
    pub fn drag_to_snapped(
        &mut self,
        point: [f64; 2],
        options: Snapping,
        tolerance: f64,
    ) -> [Option<f64>; 2] {
        if self.resize.is_some() && options.enabled && tolerance.is_finite() && tolerance >= 0.0 {
            return self.resize_to_snapped(point, options, tolerance, None);
        }
        let mut guides = [None; 2];
        let Some((selected, before, start)) = self.drag.clone() else {
            self.drag_to(point);
            return guides;
        };
        if !options.enabled || !tolerance.is_finite() || tolerance < 0.0 {
            self.drag_to(point);
            return guides;
        }
        if point.iter().any(|v| !v.is_finite()) {
            return guides;
        }
        let mut delta = [point[0] - start[0], point[1] - start[1]];
        let edge = |g: Geometry, axis: usize| {
            if axis == 0 {
                [g.x, g.x + g.width]
            } else {
                [g.y, g.y + g.height]
            }
        };
        for axis in 0..2 {
            let mut targets = Vec::new();
            if options.screen_edges {
                targets.extend(screen_guides(self.formats.output[axis]));
            }
            let grid_targets = if options.grid {
                options.grid_guides(self.formats.output[axis], axis)
            } else {
                Vec::new()
            };
            targets.extend(grid_targets.iter().copied());
            let mut mask_centers = Vec::new();
            if options.layer_edges || options.mask_edges {
                for (i, g) in before.layers.iter().enumerate() {
                    if !selected[i] && self.visible[i] {
                        targets.extend(options.targets(&before.formats, i, *g, axis));
                        if options.mask_edges
                            && let Some(mask) = mask_geometry(&before.formats, i, *g)
                        {
                            mask_centers.push(mask.center()[axis]);
                        }
                    }
                }
            }
            let mut best: Option<(f64, f64)> = None;
            for (i, g) in before.layers.iter().enumerate() {
                if !selected[i] {
                    continue;
                }
                let mask = options
                    .mask_edges
                    .then(|| mask_geometry(&before.formats, i, *g))
                    .flatten();
                for (is_center, moving) in options
                    .layer_edges
                    .then_some(*g)
                    .into_iter()
                    .flat_map(|g| edge(g, axis))
                    .chain(mask.into_iter().flat_map(|g| edge(g, axis)))
                    .map(|v| (false, v))
                    .chain(std::iter::once((true, g.center()[axis])))
                    .chain(mask.map(|g| (true, g.center()[axis])))
                {
                    for target in &targets {
                        // Centers align to screen interior/grid lines and visible mask
                        // centers; existing layer-edge behavior stays unchanged.
                        if is_center
                            && (!options.screen_edges
                                || !screen_guides(self.formats.output[axis])[1..4].contains(target))
                            && !grid_targets.contains(target)
                            && !mask_centers.contains(target)
                        {
                            continue;
                        }
                        let correction = *target - (moving + delta[axis]);
                        if correction.abs() <= tolerance
                            && best.is_none_or(|(d, _)| correction.abs() < d.abs())
                        {
                            best = Some((correction, *target));
                        }
                    }
                }
            }
            if let Some((correction, target)) = best {
                delta[axis] += correction;
                guides[axis] = Some(target);
            } else if options.pixels
                && let Some(i) = selected.iter().position(|v| *v)
            {
                let coordinate = edge(before.layers[i], axis)[0] + delta[axis];
                delta[axis] += coordinate.round() - coordinate;
            }
        }
        let mut next = before.layers;
        for (i, g) in next.iter_mut().enumerate() {
            if selected[i] {
                g.x += delta[0];
                g.y += delta[1];
            }
        }
        if next.iter().all(|g| g.validate().is_ok()) {
            self.layers = next;
        } else {
            guides = [None; 2];
        }
        guides
    }
    /// Snap a layer move or resize to an absolute screen-pixel grid.
    /// Groups move rigidly using their bounding top-left; crop gestures bypass this grid.
    pub fn drag_to_grid(&mut self, point: [f64; 2], spacing: f64) -> [Option<f64>; 2] {
        if !spacing.is_finite() || spacing <= 0.0 || point.iter().any(|v| !v.is_finite()) {
            return [None; 2];
        }
        if self.resize.is_some() {
            return self.resize_to_snapped(
                point,
                Snapping {
                    enabled: true,
                    pixels: true,
                    screen_edges: false,
                    layer_edges: false,
                    ..Default::default()
                },
                0.0,
                Some(spacing),
            );
        }
        let Some((selected, before, start)) = self.drag.clone() else {
            self.drag_to(point);
            return [None; 2];
        };
        let mut origin = [f64::INFINITY; 2];
        for (i, g) in before.layers.iter().enumerate() {
            if selected[i] {
                origin[0] = origin[0].min(g.x);
                origin[1] = origin[1].min(g.y);
            }
        }
        let target: [f64; 2] = std::array::from_fn(|axis| {
            ((origin[axis] + point[axis] - start[axis]) / spacing).round() * spacing
        });
        let delta = [target[0] - origin[0], target[1] - origin[1]];
        let mut next = before.layers;
        for (i, g) in next.iter_mut().enumerate() {
            if selected[i] {
                g.x += delta[0];
                g.y += delta[1];
            }
        }
        if next.iter().all(|g| g.validate().is_ok()) {
            self.layers = next;
            target.map(Some)
        } else {
            [None; 2]
        }
    }
    fn resize_to_snapped(
        &mut self,
        point: [f64; 2],
        options: Snapping,
        tolerance: f64,
        grid: Option<f64>,
    ) -> [Option<f64>; 2] {
        let r = self.resize.clone().expect("resize gesture");
        self.drag_to_with_precision(point, grid.is_some());
        if self.gesture_error.is_some() || point.iter().any(|v| !v.is_finite()) {
            return [None; 2];
        }
        let base = self.snapshot();
        let handle_point = |geometry| {
            ResizeHandle::positions(geometry)
                .into_iter()
                .find(|(handle, _)| *handle == r.handle)
                .unwrap()
                .1
        };
        let current = handle_point(self.layers[r.index]);
        // Free corners can reach both grid axes; proportional corners choose one
        // driving axis below so the fixed anchor and aspect ratio remain exact.
        if let Some(spacing) = grid
            && !r.keep_ratio
            && matches!(r.handle, ResizeHandle::Corner(_))
        {
            let targets = current.map(|v| (v / spacing).round() * spacing);
            self.drag_to_with_precision(
                std::array::from_fn(|axis| point[axis] + targets[axis] - current[axis]),
                true,
            );
            if self.gesture_error.is_some() {
                self.restore(base);
                return [None; 2];
            }
            let reached = handle_point(self.layers[r.index]);
            return std::array::from_fn(|axis| {
                ((reached[axis] - targets[axis]).abs() < 1e-6).then_some(targets[axis])
            });
        }
        let mut candidates = Vec::new();
        for (axis, _) in current.iter().enumerate() {
            if (axis == 0 && matches!(r.handle, ResizeHandle::Top | ResizeHandle::Bottom))
                || (axis == 1 && matches!(r.handle, ResizeHandle::Left | ResizeHandle::Right))
            {
                continue;
            }
            let mut targets = Vec::new();
            if options.screen_edges {
                targets.extend(screen_guides(self.formats.output[axis]));
            }
            if options.grid {
                targets.extend(options.grid_guides(self.formats.output[axis], axis));
            }
            if options.layer_edges || options.mask_edges {
                for (i, g) in r.before.layers.iter().enumerate() {
                    if !r.targets[i] && self.visible(i) {
                        targets.extend(options.targets(&r.before.formats, i, *g, axis));
                    }
                }
            }
            if let Some(target) = targets
                .into_iter()
                .filter(|t| (*t - current[axis]).abs() <= tolerance)
                .min_by(|a, b| {
                    (a - current[axis])
                        .abs()
                        .total_cmp(&(b - current[axis]).abs())
                })
            {
                candidates.push((1, (target - current[axis]).abs(), axis, target, None));
            }
            if options.layer_edges && matches!(r.mode, ResizeMode::Mirror | ResizeMode::Opposite) {
                let original = r.before.layers[r.index];
                let moving = handle_point(original)[axis];
                let anchor = r.anchor(r.index)[axis];
                for (i, other) in r.before.layers.iter().enumerate() {
                    if i == r.index || !r.targets[i] || !self.visible(i) {
                        continue;
                    }
                    let other_handle = if r.mode == ResizeMode::Mirror {
                        r.handle.mirrored_horizontally()
                    } else {
                        r.handle.opposite()
                    };
                    let other_moving = ResizeHandle::positions(*other)
                        .into_iter()
                        .find(|(h, _)| *h == other_handle)
                        .unwrap()
                        .1[axis];
                    let other_anchor = r.anchor(i)[axis];
                    let span = moving - anchor;
                    let other_span = other_moving - other_anchor;
                    // Both edges move with the same scale factor, in opposite
                    // directions. Solve their meeting point, not a stale edge.
                    if span * other_span >= 0.0 {
                        continue;
                    }
                    let factor = (other_anchor - anchor) / (span - other_span);
                    let target = anchor + span * factor;
                    if factor > 0.0
                        && target.is_finite()
                        && (target - current[axis]).abs() <= tolerance
                    {
                        candidates.push((0, (target - current[axis]).abs(), axis, target, Some(i)));
                    }
                }
            }
            if options.pixels {
                let spacing = grid.unwrap_or(1.0);
                let target = (current[axis] / spacing).round() * spacing;
                candidates.push((2, (target - current[axis]).abs(), axis, target, None));
            }
        }
        candidates.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        for (priority, _, axis, target, meeting) in candidates {
            let mut adjusted = point;
            // The regular preview may be pixel-rounded. Correct from the exact
            // pointer geometry, or fractional viewport scales can miss the snap.
            self.drag_to_with_precision(point, true);
            adjusted[axis] += target - handle_point(self.layers[r.index])[axis];
            if r.keep_ratio && matches!(r.handle, ResizeHandle::Corner(_)) {
                let original = r.before.layers[r.index];
                let original_handle = handle_point(original);
                let anchor = r.anchor(r.index);
                if (original_handle[axis] - anchor[axis]).abs() < 1e-9 {
                    self.restore(base.clone());
                    continue;
                }
                let factor = (target - anchor[axis]) / (original_handle[axis] - anchor[axis]);
                for j in 0..2 {
                    adjusted[j] = r.start[j] + (original_handle[j] - anchor[j]) * (factor - 1.0);
                }
            }
            self.drag_to_with_precision(adjusted, true);
            let touching = meeting.is_none_or(|i| {
                let a = self.layers[r.index];
                let b = self.layers[i];
                let other_axis = 1 - axis;
                let low = |g: Geometry| if other_axis == 0 { g.x } else { g.y };
                let size = |g: Geometry| if other_axis == 0 { g.width } else { g.height };
                let other_handle = if r.mode == ResizeMode::Mirror {
                    r.handle.mirrored_horizontally()
                } else {
                    r.handle.opposite()
                };
                let other_edge = ResizeHandle::positions(b)
                    .into_iter()
                    .find(|(h, _)| *h == other_handle)
                    .unwrap()
                    .1[axis];
                (other_edge - target).abs() < 1e-6
                    && low(a).max(low(b)) < (low(a) + size(a)).min(low(b) + size(b))
            });
            // Ratio/minimum-size constraints can prevent the requested snap. Never show a false guide.
            if self.gesture_error.is_none()
                && touching
                && (handle_point(self.layers[r.index])[axis] - target).abs() < 1e-6
            {
                let mut guides = [None; 2];
                if priority < 2 || grid.is_some() {
                    guides[axis] = Some(target);
                }
                return guides;
            }
            self.restore(base.clone());
        }
        [None; 2]
    }
}
