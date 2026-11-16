use canvas_pilot::import::{parse_svg, parse_values};
use canvas_pilot::{Geometry, Layout};
fn svg(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080">{body}</svg>"#
    )
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
#[test]
fn supplied_figma_svg_clipboard_preserves_rectangle_order_and_translation() {
    let doc = parse_values(include_str!("fixtures/figma-rectangles.svg")).unwrap();
    assert!(doc.svg);
    assert_eq!(doc.shapes.len(), 2);
    for (shape, x) in doc.shapes.iter().zip([0.0, 1040.0]) {
        let expected = Geometry {
            x,
            y: 0.0,
            width: 640.0,
            height: 360.0,
        };
        assert_eq!(shape.geometry, expected);
        assert!(!shape.background);
        assert!(!shape.background_only);
        assert_eq!(shape.outline, [0.0, 0.0]);
        assert_eq!(
            shape.paste_geometry(expected, true, true).unwrap(),
            expected
        );
    }
}
#[test]
fn supplied_illustrator_svg_clipboard_preserves_fractional_bounds_and_strokes() {
    let doc = parse_values(include_str!("fixtures/illustrator-rectangles.svg")).unwrap();
    assert!(doc.svg);
    assert_eq!(doc.shapes.len(), 2);
    for (shape, (x, y)) in doc.shapes.iter().zip([(0.354, 0.354), (395.791, 461.357)]) {
        let expected = Geometry {
            x,
            y,
            width: 1814.174,
            height: 1020.473,
        };
        close(shape.geometry.x, expected.x);
        close(shape.geometry.y, expected.y);
        close(shape.geometry.width, expected.width);
        close(shape.geometry.height, expected.height);
        assert!(!shape.background);
        assert!(!shape.background_only);
        assert_eq!(shape.outline, [0.709, 0.709]);
        assert_eq!(
            shape.paste_geometry(expected, true, false).unwrap(),
            shape.geometry
        );
        let outlined = shape.paste_geometry(expected, true, true).unwrap();
        close(outlined.x, x - 0.3545);
        close(outlined.y, y - 0.3545);
        close(outlined.width, 1814.883);
        close(outlined.height, 1021.182);
    }
}
#[test]
fn svg_detects_rectangles_background_names_transforms_and_outlines() {
    let doc=parse_svg(&svg(r#"<rect id="background" width="100%" height="100%" fill="black"/><g transform="translate(12 20) scale(2 3)" stroke="white" stroke-width="4"><rect id="presenter" x="10" y="30" width="100" height="50"/><rect id="slides" x="150" y="30" width="100" height="50"/></g>"#)).unwrap();
    assert_eq!(doc.shapes.len(), 3);
    assert!(doc.shapes[0].background);
    assert!(!doc.shapes[1].background);
    assert_eq!(doc.shapes[1].id, "presenter");
    assert_eq!(
        doc.shapes[1].geometry,
        Geometry {
            x: 32.0,
            y: 110.0,
            width: 200.0,
            height: 150.0
        }
    );
    assert_eq!(doc.shapes[1].outline, [8.0, 12.0]);
    let g = doc.shapes[1]
        .paste_geometry(doc.shapes[1].geometry, true, true)
        .unwrap();
    assert_eq!(
        g,
        Geometry {
            x: 28.0,
            y: 104.0,
            width: 208.0,
            height: 162.0
        }
    );
}
#[test]
fn svg_units_viewbox_letterboxing_and_preserve_aspect_none_are_applied() {
    let d=parse_svg(r#"<svg width="2in" height="25.4mm" viewBox="10 20 200 100"><rect x="10" y="20" width="200" height="100"/></svg>"#).unwrap();
    let g = d.shapes[0].geometry;
    close(g.x, 0.0);
    close(g.y, 0.0);
    close(g.width, 192.0);
    close(g.height, 96.0);
    let d=parse_svg(r#"<svg width="200" height="200" viewBox="10 20 200 100"><rect x="10" y="20" width="200" height="100"/></svg>"#).unwrap();
    assert_eq!(
        d.shapes[0].geometry,
        Geometry {
            x: 0.0,
            y: 50.0,
            width: 200.0,
            height: 100.0
        }
    );
    let d=parse_svg(r#"<svg width="200" height="200" viewBox="10 20 200 100" preserveAspectRatio="none"><rect x="10" y="20" width="200" height="100"/></svg>"#).unwrap();
    assert_eq!(
        d.shapes[0].geometry,
        Geometry {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0
        }
    );
    assert!(d.shapes[0].background);
}
#[test]
fn rectangular_paths_polygons_hidden_nodes_and_image_background_are_handled() {
    let doc=parse_svg(&svg(r#"<image width="1920" height="1080" href="https://example.invalid/no-fetch.png"/><g style="display:none"><circle r="50"/></g><path id="path" d="M12 30 h100 v50 h-100 z"/><polygon points="200,30 300,30 300,80 200,80"/>"#)).unwrap();
    assert_eq!(doc.shapes.len(), 3);
    assert!(doc.shapes[0].background_only);
    assert!(
        doc.shapes[0]
            .paste_geometry(doc.shapes[0].geometry, true, false)
            .is_err()
    );
    assert_eq!(
        doc.shapes[1].geometry,
        Geometry {
            x: 12.0,
            y: 30.0,
            width: 100.0,
            height: 50.0
        }
    );
    assert_eq!(
        doc.shapes[2].geometry,
        Geometry {
            x: 200.0,
            y: 30.0,
            width: 100.0,
            height: 50.0
        }
    );
    let d = parse_svg(&svg(
        r#"<g visibility="hidden"><rect visibility="visible" width="40" height="30"/></g>"#,
    ))
    .unwrap();
    assert_eq!(d.shapes.len(), 1);
}

#[test]
fn drawio_rendered_svg_uses_cell_geometry_and_ignores_label_fallbacks() {
    let rendered = r##"<?xml version="1.0"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
 width="1921px" height="1081px" viewBox="0 0 1921 1081">
 <defs><symbol id="picture" viewBox="0 0 640 360"><image width="640" height="360"/></symbol></defs>
 <g data-cell-id="0"><g data-cell-id="1">
  <g data-cell-id="midra-background"><g><image x="0" y="0" width="1920" height="1080"/></g></g>
  <g data-cell-id="layer-2"><g><use href="#picture" x="120" y="180" width="640" height="360"/></g>
   <g><switch><foreignObject width="100%" height="100%"/><image x="121" y="353.5" width="638" height="17"/></switch></g>
  </g>
  <g data-cell-id="layer-1"><g transform="translate(10 5)"><rect x="1150" y="175" width="640" height="360" fill="#173b50" stroke="none"/></g></g>
 </g></g>
 <switch><g/><a><text>Text is not SVG</text></a></switch>
</svg>"##;
    let doc = parse_svg(rendered).unwrap();
    assert_eq!(parse_values(rendered).unwrap().shapes.len(), 3);
    assert!(doc.note.starts_with("Draw.io-rendered SVG page:"));
    assert_eq!(doc.shapes.len(), 3);
    assert_eq!(doc.shapes[0].id, "midra-background");
    assert!(doc.shapes[0].background_only);
    assert_eq!(doc.shapes[1].id, "layer-2");
    assert_eq!(
        doc.shapes[1].geometry,
        Geometry {
            x: 120.0,
            y: 180.0,
            width: 640.0,
            height: 360.0,
        }
    );
    assert_eq!(doc.shapes[2].id, "layer-1");
    assert_eq!(doc.shapes[2].geometry.x, 1160.0);
    assert_eq!(doc.shapes[2].geometry.y, 180.0);
}

#[test]
fn unsupported_or_ambiguous_svg_is_refused_without_partial_shapes() {
    for body in [
        r#"<rect width="40" height="30"/><circle r="5"/>"#,
        r#"<g transform="rotate(30)"><rect width="40" height="30"/></g>"#,
        r#"<rect width="40" height="30" rx="5"/>"#,
        r#"<rect width="40" height="30" clip-path="url(#c)"/>"#,
        r#"<style>rect {width:400px}</style><rect width="40" height="30"/>"#,
        r#"<rect width="40" height="30" style="transform:scale(2)"/>"#,
        r#"<path d="M0 0 C0 10 10 10 10 0 Z"/>"#,
        r#"<path d="M0 0 H10 V10 H0"/>"#,
        r#"<polygon points="0,0 10,10 20,0 10,-10"/>"#,
        r#"<polygon points="0,0 10,0 10,10 0,10 garbage"/>"#,
        r#"<polygon points="0,0 10,0 10,10 0,10 42"/>"#,
        r##"<use href="#shape"/>"##,
        r#"<image width="30" height="20"/>"#,
        r#"<rect width="0" height="20"/>"#,
    ] {
        assert!(parse_svg(&svg(body)).is_err(), "accepted {body}");
    }
    assert!(
        parse_svg(r#"<svg width="100%" height="100%"><rect width="30" height="20"/></svg>"#)
            .is_err()
    );
    assert!(parse_svg("not xml").is_err());
    assert!(parse_values(&"x".repeat(1_048_577)).is_err());
}
#[test]
fn batch_import_is_one_transaction_and_refuses_count_duplicates_and_invalid_geometry() {
    let mut l = Layout::default();
    l.set_selection([true; 2]);
    let before = l.layers().clone();
    let formats = l.formats().clone();
    let a = Geometry {
        x: 12.0,
        y: 271.0,
        width: 945.0,
        height: 531.58,
    };
    let b = Geometry {
        x: 1000.0,
        y: 60.0,
        width: 300.0,
        height: 250.0,
    };
    assert!(l.apply_import(&[]).unwrap_err().contains("count mismatch"));
    assert!(l.apply_import(&[(0, a), (0, b)]).is_err());
    assert!(
        l.apply_import(&[(0, a), (1, Geometry { width: -1.0, ..b })])
            .is_err()
    );
    assert_eq!(*l.layers(), before);
    assert_eq!(*l.formats(), formats);
    l.apply_import(&[(1, a), (0, b)]).unwrap();
    assert_eq!(*l.layers(), [b, a]);
    l.undo();
    assert_eq!(*l.layers(), before);
    assert_eq!(*l.formats(), formats);
    l.redo();
    assert_eq!(*l.layers(), [b, a]);
    l.set_locked(1, true);
    assert!(l.apply_import(&[(0, a), (1, b)]).is_err());
}
#[test]
fn drawio_auto_detection_and_svg_text_preserve_fractional_bounds() {
    let d = parse_values(include_str!("fixtures/drawio-rectangle.txt")).unwrap();
    assert!(!d.svg);
    assert_eq!(d.shapes[0].geometry.height, 531.58);
    let d = parse_values(&svg(
        r#"<rect x="12.25" y="20.5" width="30.75" height="40.25"/>"#,
    ))
    .unwrap();
    assert!(d.svg);
    assert_eq!(d.shapes[0].geometry.x, 12.25);
}

#[test]
fn midra_values_convert_centers_and_reject_incomplete_or_invalid_batches() {
    let one = "01 / Presenter — visible box (device container unverified)\nHorizontal: -10 px\nVertical: 200 px\nWidth: 301 px\nHeight: 251 px";
    let doc = parse_values(&format!("{one}\r\n\r\n{one}")).unwrap();
    assert_eq!(doc.shapes.len(), 2);
    assert_eq!(
        doc.shapes[0].geometry,
        Geometry {
            x: -160.5,
            y: 74.5,
            width: 301.0,
            height: 251.0
        }
    );
    for bad in [
        one.replace("Width: 301 px", "Width: NaN px"),
        one.replace("Height: 251 px", "Height: -1 px"),
        one.replace("Vertical: 200 px", "Vertical: inf px"),
        one.replace("Width: 301 px", "Width: 301 mm"),
        one.replace("Width: 301 px", "Horizontal: 301 px"),
        one.replace("Height: 251 px", ""),
        format!("{one}\n\nunexpected content"),
    ] {
        assert!(parse_values(&bad).is_err(), "accepted {bad}");
        assert!(parse_values(&format!("{one}\n\n{bad}")).is_err());
    }
}

#[test]
fn native_clipboard_accepts_optional_zoom_and_rejects_invalid_percentages() {
    let old = "Layer — visible box (device container unverified)\nHorizontal: 100 px\nVertical: 200 px\nWidth: 100 px\nHeight: 100 px";
    let text = format!("{old}\nCrop zoom: 250 %\n\n{old}");
    let doc = parse_values(&text).unwrap();
    assert_eq!(doc.shapes[0].zoom, Some(2.5));
    assert_eq!(doc.shapes[1].zoom, None);
    for value in [
        "NaN %",
        "99 %",
        "10001 %",
        "200 px",
        "200 %\nCrop zoom: 300 %",
    ] {
        assert!(parse_values(&format!("{old}\nCrop zoom: {value}")).is_err());
    }
}

#[test]
fn geometry_and_zoom_import_is_atomic_and_clamps_offsets_on_zoom_out() {
    let mut l = Layout::default();
    l.set_selection([true; 2]);
    l.set_framing(
        1,
        canvas_pilot::formats::Framing {
            zoom: 3.0,
            offset: [200.0, 100.0],
        },
    )
    .unwrap();
    let before = l.layers().clone();
    let formats = l.formats().clone();
    let values = [
        (
            0,
            Geometry {
                x: 99.0,
                ..before[0]
            },
        ),
        (1, before[1]),
    ];
    assert!(
        l.apply_import_with_zoom(&values, &[(0, 2.0), (1, f64::NAN)])
            .is_err()
    );
    assert_eq!(*l.layers(), before);
    assert_eq!(*l.formats(), formats);
    l.apply_import_with_zoom(&values, &[(0, 2.0), (1, 1.0)])
        .unwrap();
    let framing = l.formats().mapping(1, l.layers()[1]).unwrap().framing;
    assert_eq!(framing.zoom, 1.0);
    assert_eq!(framing.offset, [0.0; 2]);
    l.undo();
    assert_eq!(*l.layers(), before);
    assert_eq!(*l.formats(), formats);
}

#[test]
fn explicit_mapping_without_selection_is_atomic_and_preserves_selection() {
    let mut layout = Layout::default();
    layout.select(None);
    let before = layout.layers().clone();
    let formats = layout.formats().clone();
    let a = Geometry {
        x: 12.0,
        y: 34.0,
        width: 400.0,
        height: 300.0,
    };
    let b = Geometry { x: 600.0, ..a };
    assert!(
        layout
            .apply_mapped_import_with_zoom(&[(0, a), (0, b)], &[])
            .is_err()
    );
    assert!(
        layout
            .apply_mapped_import_with_zoom(&[(9, a)], &[])
            .is_err()
    );
    assert!(
        layout
            .apply_mapped_import_with_zoom(&[(1, a), (0, b)], &[(0, f64::NAN)])
            .is_err()
    );
    assert_eq!(*layout.layers(), before);
    assert_eq!(*layout.formats(), formats);
    layout
        .apply_mapped_import_with_zoom(&[(1, a), (0, b)], &[(1, 2.0)])
        .unwrap();
    assert_eq!(*layout.layers(), [b, a]);
    assert!(layout.selected_indices().is_empty());
    layout.undo();
    assert_eq!(*layout.layers(), before);
    assert_eq!(*layout.formats(), formats);
    layout.redo();
    assert_eq!(*layout.layers(), [b, a]);
    assert!(layout.selected_indices().is_empty());
    layout.set_locked(1, true);
    assert!(
        layout
            .apply_mapped_import_with_zoom(&[(1, b)], &[])
            .is_err()
    );
}
