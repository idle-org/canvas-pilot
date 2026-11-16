use canvas_pilot::{
    Layout,
    exchange::Options,
    interchange::{self, Codec},
    project::Project,
};

#[test]
fn craft_dxf_has_owned_entities_and_roundtrips_binary_tags() {
    let project = Project::from_layout(&Layout::default());
    let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
    let bytes = interchange::export(&project, &ids, Codec::Dxf, Options::default()).unwrap();
    let tags = craft_dxf::parse(&bytes).unwrap();
    let handles: Vec<_> = tags
        .iter()
        .filter(|t| t.code == 5 || t.code == 105)
        .map(|t| t.str())
        .collect();
    assert!(
        tags.iter()
            .any(|t| t.code == 100 && t.str() == "AcDbBlockTableRecord")
    );
    let mut entities = 0;
    for (index, tag) in tags.iter().enumerate() {
        if tag.code != 0 || tag.str() != "LWPOLYLINE" {
            continue;
        }
        entities += 1;
        let entity: Vec<_> = tags[index + 1..]
            .iter()
            .take_while(|t| t.code != 0)
            .collect();
        assert!(entity.iter().any(|t| t.code == 5));
        assert!(
            entity
                .iter()
                .any(|t| t.code == 330 && handles.contains(&t.str()))
        );
        for subclass in ["AcDbEntity", "AcDbPolyline"] {
            assert!(entity.iter().any(|t| t.code == 100 && t.str() == subclass));
        }
    }
    assert_eq!(entities, 2);
    let binary = craft_dxf::write_binary(&tags, true);
    let pages = interchange::import(&binary, "layout.DXF").unwrap();
    assert_eq!(
        pages[0]
            .document
            .shapes
            .iter()
            .map(|s| s.geometry)
            .collect::<Vec<_>>(),
        project.layers.iter().map(|l| l.bounds).collect::<Vec<_>>()
    );
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
fn craft_dwg_preserves_canvas_background_and_layer_boxes() {
    for resolution in [[1920, 1080], [3840, 2160]] {
        let mut project = Project::from_layout(&Layout::default());
        project.output_resolution = resolution.into();
        let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
        for background in [false, true] {
            let bytes = interchange::export(
                &project,
                &ids,
                Codec::Dwg,
                Options {
                    extra_parameters: false,
                    background,
                    placeholders: false,
                },
            )
            .unwrap();
            assert!(craft_dwg::is_dwg(&bytes));
            let pages = interchange::import(&bytes, "layout.DWG").unwrap();
            let shapes = &pages[0].document.shapes;
            assert_eq!(shapes.len(), 2 + usize::from(background));
            let boxes: Vec<_> = shapes
                .iter()
                .filter(|s| !s.background_only)
                .map(|s| s.geometry)
                .collect();
            assert_eq!(
                boxes,
                project.layers.iter().map(|l| l.bounds).collect::<Vec<_>>()
            );
        }
    }
}

#[cfg(any(not(feature = "vendor_formats"), feature = "no_proprietary"))]
#[test]
fn licensing_build_rejects_dwg_explicitly() {
    assert!(!Codec::available().contains(&Codec::Dwg));
    assert!(
        interchange::import(b"AC1021invalid", "layout.dwg")
            .unwrap_err()
            .contains("support")
    );
}
