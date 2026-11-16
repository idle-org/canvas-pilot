//! Bounds-only imports. SVG is parsed locally; external content is never fetched.
use crate::{Geometry, parse_drawio};
use roxmltree::Node;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct DetectedShape {
    pub id: String,
    pub geometry: Geometry,
    pub outline: [f64; 2],
    pub background: bool,
    pub background_only: bool,
    pub zoom: Option<f64>,
    pub appearance: Option<crate::clipboard::Appearance>,
}
impl DetectedShape {
    pub fn paste_geometry(
        &self,
        before: Geometry,
        size: bool,
        outline: bool,
    ) -> Result<Geometry, String> {
        if self.background_only {
            return Err(
                "Ignore the background image; image content cannot be imported as layer values."
                    .into(),
            );
        }
        let mut g = self.geometry;
        if !size {
            g.width = before.width;
            g.height = before.height;
        } else if outline {
            g.x -= self.outline[0] / 2.0;
            g.y -= self.outline[1] / 2.0;
            g.width += self.outline[0];
            g.height += self.outline[1];
        }
        g.validate()
    }
}
#[derive(Clone, Debug)]
pub struct ImportDocument {
    pub shapes: Vec<DetectedShape>,
    pub svg: bool,
    pub note: String,
}
fn xml_options() -> roxmltree::ParsingOptions {
    roxmltree::ParsingOptions {
        // Draw.io Desktop emits the standard SVG 1.1 public DOCTYPE. Roxmltree
        // never fetches its external identifier and limits entity expansion.
        allow_dtd: true,
        nodes_limit: 200_000,
    }
}
pub fn parse_values(text: &str) -> Result<ImportDocument, String> {
    if text.len() > crate::exchange::MAX_EXCHANGE_BYTES {
        return Err("Import text exceeds 16 MiB.".into());
    }
    if let Some((_, json)) = text.rsplit_once(crate::clipboard::MARKER) {
        return crate::clipboard::parse(json);
    }
    if let Some(geometry) = crate::exchange_metadata::geometry_text(text)? {
        return parse_values(&geometry);
    }
    if text.contains(" — visible box (device container unverified)") {
        return parse_midra_values(text);
    }
    if text.trim_start().starts_with('{') {
        if text.contains("\"beffroi.midra-geometry\"") {
            return crate::geometry_formats::parse_json(text);
        }
        return Err(
            "This JSON is not a geometry exchange. Use Open project for .pilot.json files (or legacy .midra.json projects).".into(),
        );
    }
    if crate::geometry_formats::looks_like_csv(text) {
        return crate::geometry_formats::parse_csv(text);
    }
    if crate::geometry_formats::looks_like_dxf(text) {
        return crate::geometry_formats::parse_dxf(text);
    }
    if text.trim_start().starts_with('<')
        && roxmltree::Document::parse_with_options(text, xml_options())
            .is_ok_and(|d| d.root_element().has_tag_name("svg"))
    {
        return parse_svg(text);
    }
    if text.trim_start().starts_with('<')
        && roxmltree::Document::parse_with_options(text, xml_options())
            .is_ok_and(|d| d.root_element().has_tag_name("html"))
    {
        return crate::geometry_formats::parse_html(text);
    }
    Ok(ImportDocument {
        shapes: parse_drawio(text)?
            .into_iter()
            .map(|s| DetectedShape {
                id: s.id,
                geometry: s.geometry,
                outline: [s.stroke_width; 2],
                background: s.background,
                background_only: s.background,
                zoom: None,
                appearance: None,
            })
            .collect(),
        svg: false,
        note: "Draw.io: 1 diagram unit = 1 output pixel; original top-left coordinates retained."
            .into(),
    })
}
/// Read the human-readable position values emitted by the app's copy actions.
fn parse_midra_values(text: &str) -> Result<ImportDocument, String> {
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut shapes = Vec::new();
    let mut remaining = lines.as_slice();
    while !remaining.is_empty() {
        if remaining.len() < 5 || shapes.len() >= 256 {
            return Err("Incomplete Midra values: each layer needs its name, Horizontal, Vertical, Width and Height (up to 256 layers).".into());
        }
        let (block, rest) = remaining.split_at(5);
        remaining = rest;
        let zoom = if remaining
            .first()
            .is_some_and(|line| line.starts_with("Crop zoom:"))
        {
            let value = remaining[0]
                .strip_prefix("Crop zoom:")
                .unwrap()
                .trim()
                .strip_suffix('%')
                .ok_or("Crop zoom must be a percentage.")?
                .trim()
                .parse::<f64>()
                .map_err(|_| "Invalid crop zoom percentage.")?
                / 100.0;
            if !value.is_finite() || !(1.0..=100.0).contains(&value) {
                return Err("Crop zoom must be between 100% and 10000%.".into());
            }
            remaining = &remaining[1..];
            Some(value)
        } else {
            None
        };
        let id = block[0]
            .strip_suffix(" — visible box (device container unverified)")
            .filter(|id| !id.is_empty())
            .ok_or("Invalid Midra layer heading.")?;
        let mut values = [0.0; 4];
        for (j, field) in ["Horizontal:", "Vertical:", "Width:", "Height:"]
            .iter()
            .enumerate()
        {
            values[j] = block[j + 1]
                .strip_prefix(field)
                .and_then(|v| v.trim().strip_suffix("px"))
                .ok_or_else(|| format!("Missing or invalid {field} value in {id}."))?
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("Invalid {field} number in {id}."))?;
            if !values[j].is_finite() {
                return Err(format!("{field} must be finite in {id}."));
            }
        }
        let [horizontal, vertical, width, height] = values;
        let geometry = Geometry {
            x: horizontal - width / 2.0,
            y: vertical - height / 2.0,
            width,
            height,
        }
        .validate()?;
        shapes.push(DetectedShape {
            id: id.into(),
            geometry,
            outline: [0.0; 2],
            background: false,
            background_only: false,
            zoom,
            appearance: None,
        });
    }
    Ok(ImportDocument {
        shapes,
        svg: false,
        note: "Canvas Pilot: center positions converted to top-left bounds. Copied values are rounded to whole pixels; optional crop zoom is available separately. Source formats and crop offsets are not copied.".into(),
    })
}

#[derive(Clone, Copy)]
struct Matrix {
    x: f64,
    y: f64,
    tx: f64,
    ty: f64,
}
impl Matrix {
    fn nested(self, n: Node) -> Result<Self, String> {
        let local = n.attribute("transform").unwrap_or("");
        let t = svgtypes::Transform::from_str(local)
            .map_err(|e| format!("Invalid SVG transform: {e}"))?;
        if [t.a, t.b, t.c, t.d, t.e, t.f]
            .iter()
            .any(|v| !v.is_finite())
            || t.b.abs() > 1e-10
            || t.c.abs() > 1e-10
            || t.a == 0.0
            || t.d == 0.0
        {
            return Err("SVG rotation, skew and collapsed transforms are unsupported. Use axis-aligned rectangles.".into());
        }
        Ok(Self {
            x: self.x * t.a,
            y: self.y * t.d,
            tx: self.tx + self.x * t.e,
            ty: self.ty + self.y * t.f,
        })
    }
    fn bounds(self, g: Geometry) -> Result<Geometry, String> {
        let x = [self.tx + self.x * g.x, self.tx + self.x * (g.x + g.width)];
        let y = [self.ty + self.y * g.y, self.ty + self.y * (g.y + g.height)];
        Geometry {
            x: x[0].min(x[1]),
            y: y[0].min(y[1]),
            width: (x[1] - x[0]).abs(),
            height: (y[1] - y[0]).abs(),
        }
        .validate()
    }
}
fn length(text: &str, percent: Option<f64>) -> Result<f64, String> {
    let value =
        svgtypes::Length::from_str(text).map_err(|e| format!("Invalid SVG length {text}: {e}"))?;
    use svgtypes::LengthUnit::*;
    let factor = match value.unit {
        None | Px => 1.0,
        In => 96.0,
        Cm => 96.0 / 2.54,
        Mm => 96.0 / 25.4,
        Pt => 96.0 / 72.0,
        Pc => 16.0,
        Percent => {
            percent.ok_or("SVG root percentage dimensions need an explicit pixel size.")? / 100.0
        }
        _ => return Err("Font-relative SVG lengths are unsupported.".into()),
    };
    let result = value.number * factor;
    if !result.is_finite() {
        return Err("SVG dimensions must be finite.".into());
    }
    Ok(result)
}
fn property<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attribute("style")
        .and_then(|s| {
            s.split(';')
                .filter_map(|part| part.split_once(':'))
                .filter(|(key, _)| key.trim() == name)
                .map(|(_, v)| v.trim())
                .next_back()
        })
        .or_else(|| node.attribute(name))
}
fn inherited<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.ancestors()
        .filter(|n| n.is_element())
        .find_map(|n| property(n, name).filter(|v| *v != "inherit"))
}
fn hidden(node: Node) -> Result<bool, String> {
    for n in node.ancestors().filter(|n| n.is_element()) {
        if property(n, "display") == Some("none") {
            return Ok(true);
        }
        if let Some(v) = property(n, "opacity") {
            let opacity = v
                .parse::<f64>()
                .map_err(|_| "Unsupported SVG opacity value.")?;
            if !opacity.is_finite() {
                return Err("Invalid SVG opacity.".into());
            }
            if opacity <= 0.0 {
                return Ok(true);
            }
        }
    }
    Ok(matches!(
        inherited(node, "visibility"),
        Some("hidden" | "collapse")
    ))
}
pub(crate) fn rectangle_points(mut points: Vec<[f64; 2]>) -> Result<Geometry, String> {
    if points.len() == 5 && points.first() == points.last() {
        points.pop();
    }
    if points.len() != 4 || points.iter().flatten().any(|v| !v.is_finite()) {
        return Err("Only four-corner rectangular SVG paths/polygons are supported.".into());
    }
    let minx = points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let maxx = points
        .iter()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let miny = points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let maxy = points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    for j in 0..4 {
        let a = points[j];
        let b = points[(j + 1) % 4];
        if !(a[0] == minx || a[0] == maxx)
            || !(a[1] == miny || a[1] == maxy)
            || a == b
            || !((a[0] == b[0]) ^ (a[1] == b[1]))
            || points[..j].contains(&a)
        {
            return Err("SVG path/polygon is not an axis-aligned rectangle.".into());
        }
    }
    Geometry {
        x: minx,
        y: miny,
        width: maxx - minx,
        height: maxy - miny,
    }
    .validate()
}
fn path_rectangle(data: &str) -> Result<Geometry, String> {
    use svgtypes::PathSegment::*;
    let mut points = Vec::new();
    let mut current = [0.0; 2];
    let mut closed = false;
    for segment in svgtypes::PathParser::from(data) {
        if closed {
            return Err("Multiple SVG subpaths are unsupported.".into());
        }
        match segment.map_err(|e|format!("Invalid SVG path: {e}"))? {
            MoveTo{abs,x,y} if points.is_empty()=>{current=if abs {[x,y]} else {[current[0]+x,current[1]+y]};points.push(current);}
            LineTo{abs,x,y} if !points.is_empty()=>{current=if abs {[x,y]} else {[current[0]+x,current[1]+y]};points.push(current);}
            HorizontalLineTo{abs,x} if !points.is_empty()=>{current[0]=if abs{x}else{current[0]+x};points.push(current);}
            VerticalLineTo{abs,y} if !points.is_empty()=>{current[1]=if abs{y}else{current[1]+y};points.push(current);}
            ClosePath{..}=>closed=true,
            _=>return Err("Only straight, closed rectangular SVG paths are supported; curves and multiple subpaths are refused.".into()),
        }
        if points.len() > 5 {
            return Err("SVG path contains more than four rectangle corners.".into());
        }
    }
    if !closed {
        return Err("SVG rectangle paths must be closed.".into());
    }
    rectangle_points(points)
}
fn walk(
    node: Node,
    matrix: Matrix,
    viewport: [f64; 2],
    page: [f64; 2],
    out: &mut Vec<DetectedShape>,
    depth: usize,
) -> Result<(), String> {
    if depth > 64 {
        return Err("SVG nesting exceeds 64 levels.".into());
    }
    if !node.is_element() {
        return Ok(());
    }
    let tag = node.tag_name().name();
    if matches!(tag, "defs" | "metadata" | "title" | "desc" | "namedview") {
        return Ok(());
    }
    if tag == "style" {
        return Err(
            "SVG stylesheets are unsupported. Use inline styles or presentation attributes.".into(),
        );
    }
    if hidden(node)? && !matches!(tag, "svg" | "g" | "a") {
        return Ok(());
    }
    for name in ["clip-path", "mask", "filter", "transform-origin"] {
        if property(node, name).is_some_and(|v| v != "none") {
            return Err(format!(
                "SVG {name} is unsupported; import plain rectangles without clipping or effects."
            ));
        }
    }
    if node.attribute("style").is_some_and(|s| {
        s.split(';').any(|part| {
            part.split_once(':').is_some_and(|(k, _)| {
                matches!(
                    k.trim(),
                    "transform" | "x" | "y" | "width" | "height" | "rx" | "ry"
                )
            })
        })
    }) {
        return Err("SVG CSS geometry/transform is unsupported; use attributes.".into());
    }
    let matrix = matrix.nested(node)?;
    if matches!(tag, "svg" | "g" | "a") {
        if tag == "svg" && depth != 0 {
            return Err("Nested SVG viewports are unsupported.".into());
        }
        for child in node.children() {
            walk(child, matrix, viewport, page, out, depth + 1)?;
        }
        return Ok(());
    }
    let dim = |name: &str, default: Option<&str>, axis: usize| {
        length(
            node.attribute(name)
                .or(default)
                .ok_or_else(|| format!("Missing SVG {name}."))?,
            Some(viewport[axis]),
        )
    };
    let raw = match tag {
        "rect" | "image" => {
            if tag == "rect" && (dim("rx", Some("0"), 0)? != 0.0 || dim("ry", Some("0"), 1)? != 0.0)
            {
                return Err("Rounded SVG rectangles are unsupported. Use square corners.".into());
            }
            Geometry {
                x: dim("x", Some("0"), 0)?,
                y: dim("y", Some("0"), 1)?,
                width: dim("width", None, 0)?,
                height: dim("height", None, 1)?,
            }
            .validate()?
        }
        "path" => path_rectangle(node.attribute("d").ok_or("Missing SVG path data.")?)?,
        "polygon" => {
            let numbers = svgtypes::NumberListParser::from(
                node.attribute("points")
                    .ok_or("Missing SVG polygon points.")?,
            )
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Invalid SVG polygon points: {e}"))?;
            if numbers.len() % 2 != 0 {
                return Err("SVG polygon points need X/Y pairs.".into());
            }
            rectangle_points(numbers.as_chunks::<2>().0.to_vec())?
        }
        _ => {
            return Err(format!(
                "Unsupported visible SVG element <{tag}>. Export rectangles only (plus an optional full-page background)."
            ));
        }
    };
    let geometry = matrix.bounds(raw)?;
    let full_page = geometry.x.abs() < 1e-6
        && geometry.y.abs() < 1e-6
        && (geometry.width - page[0]).abs() < 1e-6
        && (geometry.height - page[1]).abs() < 1e-6;
    let version = node
        .document()
        .root_element()
        .attribute("data-midra-layout-geometry");
    let exported_layer = match version {
        Some("1") => tag == "rect",
        // VectorCraft retains per-object metadata on a wrapping group but
        // drops the root marker when it writes an Illustrator-derived SVG.
        Some("2") | None => inherited(node, "data-midra-role") == Some("layer"),
        _ => false,
    };
    let marked_background = matches!(version, Some("2") | None)
        && inherited(node, "data-midra-role") == Some("background");
    if marked_background && !full_page {
        return Err("Exported background must cover the SVG page.".into());
    }
    let background = marked_background || (out.is_empty() && full_page && !exported_layer);
    if tag == "image" && !background && !exported_layer {
        return Err(
            "Only a full-page background or an exported layer placeholder image is supported."
                .into(),
        );
    }
    let stroke = if inherited(node, "stroke").is_none_or(|v| v == "none")
        || inherited(node, "stroke-opacity") == Some("0")
    {
        0.0
    } else {
        length(inherited(node, "stroke-width").unwrap_or("1"), None)?
    };
    if stroke < 0.0 {
        return Err("SVG stroke width cannot be negative.".into());
    }
    if stroke > 0.0 && inherited(node, "vector-effect").is_some_and(|v| v != "none") {
        return Err("SVG non-scaling strokes are unsupported.".into());
    }
    if stroke > 0.0
        && matches!(tag, "path" | "polygon")
        && inherited(node, "stroke-linejoin").is_some_and(|v| v != "miter")
    {
        return Err("SVG rectangular paths require miter stroke joins for outline import.".into());
    }
    let id = node
        .attribute(("http://www.inkscape.org/namespaces/inkscape", "label"))
        .or_else(|| node.attribute("id"))
        .or_else(|| {
            node.ancestors().skip(1).find_map(|parent| {
                parent
                    .attribute("data-midra-role")
                    .and_then(|_| parent.attribute("id"))
            })
        })
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{tag} {}", out.len() + 1));
    out.push(DetectedShape {
        id,
        geometry,
        outline: [stroke * matrix.x.abs(), stroke * matrix.y.abs()],
        background,
        background_only: background && (tag == "image" || marked_background),
        zoom: None,
        appearance: None,
    });
    if out.iter().filter(|s| !s.background_only).count() > 256 || out.len() > 257 {
        return Err("Import is limited to 256 rectangles.".into());
    }
    Ok(())
}

/// Draw.io Desktop's SVG exporter keeps the source cell ID on a wrapper group,
/// but turns image cells into `<symbol>/<use>` and adds HTML/raster label
/// fallbacks. Read one geometry primitive from each cell instead of treating
/// those rendering details as independent layer rectangles.
fn drawio_svg_shapes(
    root: Node,
    matrix: Matrix,
    viewport: [f64; 2],
) -> Result<Option<Vec<DetectedShape>>, String> {
    let has_root_cells = root
        .descendants()
        .any(|n| n.attribute("data-cell-id") == Some("0"))
        && root
            .descendants()
            .any(|n| n.attribute("data-cell-id") == Some("1"));
    if !has_root_cells {
        return Ok(None);
    }
    let mut out = Vec::new();
    for cell in root.descendants().filter(|n| {
        n.is_element()
            && n.attribute("data-cell-id")
                .is_some_and(|id| !matches!(id, "0" | "1"))
    }) {
        let id = cell.attribute("data-cell-id").unwrap();
        if out.iter().any(|shape: &DetectedShape| shape.id == id) {
            return Err(format!("Duplicate draw.io SVG cell ID: {id}."));
        }
        let primitive = cell
            .descendants()
            .find(|n| n.is_element() && matches!(n.tag_name().name(), "rect" | "image" | "use"))
            .ok_or_else(|| {
                format!(
                    "Draw.io SVG cell {id} is not a rectangle or image. Export axis-aligned layer boxes."
                )
            })?;
        let dim = |name: &str, default: Option<&str>, axis: usize| {
            length(
                primitive
                    .attribute(name)
                    .or(default)
                    .ok_or_else(|| format!("Missing SVG {name} in draw.io cell {id}."))?,
                Some(viewport[axis]),
            )
        };
        if primitive.has_tag_name("rect")
            && (dim("rx", Some("0"), 0)? != 0.0 || dim("ry", Some("0"), 1)? != 0.0)
        {
            return Err(format!(
                "Draw.io SVG cell {id} is rounded; export square layer boxes."
            ));
        }
        let raw = Geometry {
            x: dim("x", Some("0"), 0)?,
            y: dim("y", Some("0"), 1)?,
            width: dim("width", None, 0)?,
            height: dim("height", None, 1)?,
        }
        .validate()?;
        let mut cell_matrix = matrix;
        let mut ancestors: Vec<_> = primitive
            .ancestors()
            .take_while(|node| *node != root)
            .filter(|node| node.is_element())
            .collect();
        ancestors.reverse();
        for ancestor in ancestors {
            cell_matrix = cell_matrix.nested(ancestor)?;
        }
        cell_matrix = cell_matrix.nested(primitive)?;
        let geometry = cell_matrix.bounds(raw)?;
        let stroke = if inherited(primitive, "stroke").is_none_or(|v| v == "none")
            || inherited(primitive, "stroke-opacity") == Some("0")
        {
            0.0
        } else {
            length(inherited(primitive, "stroke-width").unwrap_or("1"), None)?
        };
        if stroke < 0.0 {
            return Err("SVG stroke width cannot be negative.".into());
        }
        let background = id == "midra-background";
        out.push(DetectedShape {
            id: id.to_owned(),
            geometry,
            outline: [stroke * cell_matrix.x.abs(), stroke * cell_matrix.y.abs()],
            background,
            background_only: background,
            zoom: None,
            appearance: None,
        });
        if out.iter().filter(|shape| !shape.background_only).count() > 256 || out.len() > 257 {
            return Err("Import is limited to 256 rectangles.".into());
        }
    }
    if out.is_empty() {
        return Err("No layer cells detected in draw.io SVG.".into());
    }
    Ok(Some(out))
}

pub fn parse_svg(text: &str) -> Result<ImportDocument, String> {
    if text.len() > crate::exchange::MAX_EXCHANGE_BYTES {
        return Err("SVG exceeds 16 MiB.".into());
    }
    let document = roxmltree::Document::parse_with_options(text, xml_options())
        .map_err(|e| format!("Invalid SVG XML: {e}"))?;
    let root = document.root_element();
    if let Some(version) = root.attribute("data-midra-layout-geometry")
        && !matches!(version, "1" | "2")
    {
        return Err("Unsupported Canvas Pilot SVG exchange version.".into());
    }
    if !root.has_tag_name("svg") {
        return Err("Choose an SVG file with an <svg> root.".into());
    }
    if document.descendants().any(|n| {
        (n.is_element() && n.has_tag_name("style"))
            || n.pi().is_some_and(|pi| pi.target == "xml-stylesheet")
    }) {
        return Err("SVG stylesheets are unsupported; use inline styles.".into());
    }
    let vb = root
        .attribute("viewBox")
        .map(svgtypes::ViewBox::from_str)
        .transpose()
        .map_err(|e| format!("Invalid SVG viewBox: {e}"))?;
    let viewport = if let Some(v) = vb {
        [v.w, v.h]
    } else {
        [
            length(
                root.attribute("width")
                    .ok_or("SVG needs width/height or a viewBox.")?,
                None,
            )?,
            length(
                root.attribute("height")
                    .ok_or("SVG needs width/height or a viewBox.")?,
                None,
            )?,
        ]
    };
    crate::formats::validate_size(viewport)?;
    let page = [
        root.attribute("width")
            .map(|s| length(s, None))
            .transpose()?
            .unwrap_or(viewport[0]),
        root.attribute("height")
            .map(|s| length(s, None))
            .transpose()?
            .unwrap_or(viewport[1]),
    ];
    crate::formats::validate_size(page)?;
    let mut matrix = Matrix {
        x: 1.0,
        y: 1.0,
        tx: 0.0,
        ty: 0.0,
    };
    if let Some(v) = vb {
        let aspect = root
            .attribute("preserveAspectRatio")
            .unwrap_or("xMidYMid meet");
        if aspect == "none" {
            matrix = Matrix {
                x: page[0] / v.w,
                y: page[1] / v.h,
                tx: -v.x * page[0] / v.w,
                ty: -v.y * page[1] / v.h,
            };
        } else if matches!(aspect, "xMidYMid" | "xMidYMid meet") {
            let k = (page[0] / v.w).min(page[1] / v.h);
            matrix = Matrix {
                x: k,
                y: k,
                tx: (page[0] - k * v.w) / 2.0 - v.x * k,
                ty: (page[1] - k * v.h) / 2.0 - v.y * k,
            };
        } else {
            return Err("SVG preserveAspectRatio currently supports xMidYMid meet or none. Other alignments/cropping must be flattened first.".into());
        }
    }
    let drawio_svg = drawio_svg_shapes(root, matrix, viewport)?;
    let rendered_by_drawio = drawio_svg.is_some();
    let mut shapes = Vec::new();
    if let Some(imported) = drawio_svg {
        shapes = imported;
    } else {
        walk(root, matrix, viewport, page, &mut shapes, 0)?;
    }
    if shapes.is_empty() {
        return Err("No visible rectangles detected in SVG.".into());
    }
    Ok(ImportDocument {
        shapes,
        svg: true,
        note: format!(
            "{}SVG page: {} × {} px. CSS units use 96 px/in; viewBox and axis-aligned transforms are applied. Coordinates are relative to this SVG page; draw.io may crop and rebase exports without a background.",
            if rendered_by_drawio {
                "Draw.io-rendered "
            } else {
                ""
            },
            crate::view_pixels(page[0]),
            crate::view_pixels(page[1])
        ),
    })
}
