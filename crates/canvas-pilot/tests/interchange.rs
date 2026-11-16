use canvas_pilot::{
    Layout,
    exchange::Options,
    interchange::{self, Codec},
    project::Project,
};

#[test]
fn unified_text_codecs_preserve_geometry() {
    let project = Project::from_layout(&Layout::default());
    let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
    for codec in [
        Codec::Svg,
        Codec::Drawio,
        Codec::Json,
        Codec::Csv,
        Codec::Dxf,
        Codec::Html,
    ] {
        let bytes = interchange::export(&project, &ids, codec, Options::default()).unwrap();
        let pages = interchange::import(&bytes, &format!("layout.{}", codec.extension())).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].document.shapes.len(), ids.len());
    }
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
fn native_binary_codecs_roundtrip_boxes_background_and_artwork() {
    let mut layout = Layout::default();
    layout
        .apply(
            0,
            canvas_pilot::Geometry {
                x: -12.5,
                y: 271.4,
                width: 945.2,
                height: 531.58,
            },
        )
        .unwrap();
    let mut project = Project::from_layout(&layout);
    project.layers[0].name = "Camera & écran".into();
    let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
    for codec in [Codec::Pptx, Codec::Figma, Codec::Figcraft] {
        for artwork in [false, true] {
            let bytes = interchange::export(
                &project,
                &ids,
                codec,
                Options {
                    extra_parameters: false,
                    background: true,
                    placeholders: artwork,
                },
            )
            .unwrap();
            assert!(bytes.starts_with(b"PK"));
            if codec == Codec::Figcraft {
                let (doc, _) = figcraft_format::load_bytes(&bytes).unwrap();
                let resaved = figcraft_format::save_bytes(&doc, None).unwrap();
                let json = figcraft_format::to_json(&doc).unwrap().to_string();
                for data in [&resaved[..], json.as_bytes()] {
                    let pages = interchange::import(data, "resaved.FIGCRAFT").unwrap();
                    assert_eq!(pages[0].document.shapes.len(), 3);
                    assert_eq!(pages[0].document.shapes[1].geometry.x, -13.0);
                }
            }
            let pages =
                interchange::import(&bytes, &format!("layout.{}", codec.extension())).unwrap();
            assert_eq!(pages.len(), 1, "{codec:?}");
            let shapes = &pages[0].document.shapes;
            assert_eq!(shapes.len(), 3, "{codec:?}");
            assert!(shapes[0].background_only, "{codec:?}");
            assert_eq!(shapes[1].id, "Camera & écran");
            let g = shapes[1].geometry;
            for (got, expected) in [
                (g.x, -13.0),
                (g.y, 271.0),
                (g.width, 945.0),
                (g.height, 532.0),
            ] {
                assert!(
                    (got - expected).abs() < 0.001,
                    "{codec:?}: {got} != {expected}"
                );
            }
            if artwork {
                if codec == Codec::Pptx {
                    let native = deckcraft_pptx::import(&bytes).unwrap();
                    assert_eq!(native.media.len(), 3);
                } else {
                    let (native, _) = if codec == Codec::Figcraft {
                        figcraft_format::load_bytes(&bytes).unwrap()
                    } else {
                        figcraft_format::fig::import(&bytes, "layout.fig").unwrap()
                    };
                    assert!(!native.images.is_empty());
                }
            } else if codec == Codec::Pptx {
                let native = deckcraft_pptx::import(&bytes).unwrap();
                assert!(matches!(
                    native.slides[0].shapes[1].fill,
                    Some(deckcraft_model::Fill::Solid { .. })
                ));
            }
        }
    }
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
fn pptx_slides_are_separate_and_group_coordinates_are_resolved() {
    use deckcraft_geom::Xfrm;
    use deckcraft_model::{Presentation, Shape, ShapeKind, Slide};
    let mut doc = Presentation::default();
    let child = Shape {
        name: "Nested".into(),
        xfrm: Some(Xfrm::new(10.0, 20.0, 30.0, 40.0)),
        ..Default::default()
    };
    let group = Shape {
        xfrm: Some(Xfrm::new(75.0, 150.0, 200.0, 200.0)),
        kind: ShapeKind::Group {
            children: vec![child],
            child: Xfrm::new(0.0, 0.0, 100.0, 100.0),
        },
        ..Default::default()
    };
    doc.slides = vec![
        std::sync::Arc::new(Slide {
            shapes: vec![group],
            ..Default::default()
        }),
        std::sync::Arc::new(Slide {
            id: deckcraft_model::SlideId(2),
            ..Default::default()
        }),
    ];
    let pages = interchange::import(&deckcraft_pptx::export(&doc).unwrap(), "slides.pptx").unwrap();
    assert_eq!(pages.len(), 2);
    let g = pages[0].document.shapes[0].geometry;
    assert!((g.x - 95.0 / 0.75).abs() < 0.001);
    assert!((g.y - 190.0 / 0.75).abs() < 0.001);
    assert!((g.width - 80.0).abs() < 0.001);
    assert!(pages[1].document.shapes.is_empty());
}

#[test]
fn disabled_vendor_codecs_are_not_advertised_or_accepted() {
    if cfg!(all(
        feature = "vendor_formats",
        not(feature = "no_proprietary")
    )) {
        return;
    }
    assert!(!Codec::available().contains(&Codec::Figma));
    assert!(!Codec::available().contains(&Codec::Figcraft));
    assert!(interchange::import(b"{}", "test.figcraft").is_err());
    assert!(interchange::import(b"fig-test", "test.fig").is_err());
    let project = Project::from_layout(&Layout::default());
    assert!(
        interchange::export(
            &project,
            &[project.layers[0].id],
            Codec::Pptx,
            Options::default()
        )
        .is_err()
    );
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
fn figma_frames_get_independent_origins_and_keep_hidden_children_out() {
    use figcraft_doc::{Document, Node, NodeKind};
    let mut doc = Document::new("External frames");
    let page = doc
        .pages
        .first()
        .copied()
        .unwrap_or_else(|| doc.add_page("Page"));
    for (name, x) in [("Left", 1000.0), ("Right", 4000.0)] {
        let mut frame = Node::new(doc.alloc_id(), NodeKind::Frame, name, 1920.0, 1080.0);
        frame.transform = figcraft_geom::Affine::translate((x, 900.0));
        let frame = doc.insert(frame, page, None).unwrap();
        for visible in [true, false] {
            let mut node = Node::new(doc.alloc_id(), NodeKind::Rectangle, "Content", 300.0, 250.0);
            node.transform = figcraft_geom::Affine::translate((20.0, 30.0));
            node.visible = visible;
            doc.insert(node, frame, None).unwrap();
        }
    }
    let (bytes, _) = figcraft_format::fig::export(&doc, None).unwrap();
    let pages = interchange::import(&bytes, "external.fig").unwrap();
    assert_eq!(pages.len(), 2);
    for page in pages {
        assert_eq!(page.document.shapes.len(), 1);
        assert_eq!(page.document.shapes[0].geometry.x, 20.0);
        assert_eq!(page.document.shapes[0].geometry.y, 30.0);
    }
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
fn native_export_refuses_names_that_would_turn_content_into_a_background() {
    let mut project = Project::from_layout(&Layout::default());
    project.layers[0].name = "midra-background".into();
    for codec in [Codec::Pptx, Codec::Figma, Codec::Figcraft] {
        let error =
            interchange::export(&project, &[project.layers[0].id], codec, Options::default())
                .unwrap_err();
        assert!(error.contains("Rename the layer"));
    }
}

#[test]
fn corrupt_native_and_oversized_files_fail_without_changing_any_project() {
    assert!(interchange::import(b"PKgarbage", "broken.pptx").is_err());
    assert!(
        interchange::import(
            &vec![0; canvas_pilot::exchange::MAX_EXCHANGE_BYTES + 1],
            "large.svg"
        )
        .is_err()
    );
}

#[test]
fn illustrator_svg_wrappers_preserve_layer_and_background_roles() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080">
      <g id="midra-background" data-midra-role="background"><image width="1920" height="1080"/></g>
      <g id="layer-1" data-midra-role="layer"><image width="300" height="250" transform="matrix(1 0 0 1 100 200)"/></g>
    </svg>"#;
    let pages = interchange::import(svg, "vectorcraft.svg").unwrap();
    let shapes = &pages[0].document.shapes;
    assert!(shapes[0].background_only);
    assert_eq!(shapes[1].id, "layer-1");
    assert_eq!(shapes[1].geometry.x, 100.0);
    assert!(!shapes[1].background);
}
