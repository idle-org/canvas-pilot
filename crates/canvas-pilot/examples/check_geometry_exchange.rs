//! Manual native-editor round trip; all paths are explicit, no device access.
use canvas_pilot::{
    Geometry, Layout,
    exchange::{self, Format, Options},
    project::{Project, Resolution, write_file},
};

fn template_project(resolution: [u32; 2]) -> Project {
    let mut project = Project::from_layout(&Layout::default());
    let scale = resolution[0] as f64 / 1920.0;
    project.output_resolution = Resolution::from(resolution);
    for layer in &mut project.layers {
        layer.bounds = Geometry {
            x: layer.bounds.x * scale,
            y: layer.bounds.y * scale,
            width: layer.bounds.width * scale,
            height: layer.bounds.height * scale,
        };
    }
    project
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, directory, formats @ ..] if command == "native" && !formats.is_empty() => {
            use canvas_pilot::interchange::{self, Codec};
            let directory = std::path::Path::new(directory);
            std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
            let project = Project::from_layout(&Layout::default());
            let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
            for extension in formats {
                let codec = Codec::available()
                    .into_iter()
                    .find(|c| c.extension() == extension)
                    .ok_or("Unsupported format in this build")?;
                for pictures in [false, true] {
                    let bytes = interchange::export(
                        &project,
                        &ids,
                        codec,
                        Options {
                            extra_parameters: false,
                            background: true,
                            placeholders: pictures,
                        },
                    )?;
                    let filename = format!(
                        "{}-{}.{}",
                        if pictures { "pictures" } else { "plain" },
                        "native",
                        extension
                    );
                    write_file(&directory.join(&filename), &bytes, false)?;
                    let pages = interchange::import(&bytes, &filename)?;
                    let shapes: Vec<_> = pages[0]
                        .document
                        .shapes
                        .iter()
                        .filter(|s| !s.background_only)
                        .collect();
                    if shapes.len() != project.layers.len()
                        || shapes
                            .iter()
                            .zip(&project.layers)
                            .any(|(s, l)| s.geometry != l.bounds)
                    {
                        return Err(format!(
                            "Native round-trip mismatch for {filename}: {pages:?}"
                        ));
                    }
                    println!("Verified native geometry and background: {filename}");
                }
            }
        }
        [command, directory] if command == "generate" => {
            let directory = std::path::Path::new(directory);
            std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
            let project = Project::from_layout(&Layout::default());
            let ids = project.layers.iter().map(|l| l.id).collect::<Vec<_>>();
            for format in Format::ALL {
                for (name, options) in [
                    ("plain", Options::default()),
                    (
                        "illustrated",
                        Options {
                            extra_parameters: false,
                            background: true,
                            placeholders: true,
                        },
                    ),
                ] {
                    let options = Options {
                        extra_parameters: false,
                        placeholders: options.placeholders && format.supports_placeholders(),
                        ..options
                    };
                    let contents = exchange::export(&project, &ids, format, options)?;
                    write_file(
                        &directory.join(format!("{name}.{}", format.extension())),
                        contents.as_bytes(),
                        false,
                    )?;
                }
            }
        }
        [command, paths @ ..] if command == "verify" && !paths.is_empty() => {
            let project = Project::from_layout(&Layout::default());
            for path in paths {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                let pages = canvas_pilot::interchange::import(&bytes, path)?;
                if pages.len() != 1 {
                    return Err(format!("Expected one page: {path}"));
                }
                let imported = &pages[0].document;
                let shapes: Vec<_> = imported
                    .shapes
                    .iter()
                    .filter(|s| !s.background_only)
                    .collect();
                if shapes.len() != project.layers.len()
                    || shapes
                        .iter()
                        .zip(&project.layers)
                        .any(|(s, l)| s.geometry != l.bounds)
                {
                    return Err(format!("Geometry mismatch: {path}"));
                }
                println!("Verified {} layer boxes: {path}", shapes.len());
            }
        }
        [command, directory, extensions @ ..] if command == "templates" => {
            use canvas_pilot::interchange::{self, Codec};
            let codecs = if extensions.is_empty() {
                Codec::available()
                    .into_iter()
                    .filter(|c| {
                        !matches!(
                            c,
                            Codec::Pptx | Codec::Figma | Codec::Figcraft | Codec::Illustrator
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                extensions
                    .iter()
                    .map(|extension| {
                        Codec::available()
                            .into_iter()
                            .find(|c| c.extension() == extension)
                            .ok_or_else(|| format!("Unsupported format in this build: {extension}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            let directory = std::path::Path::new(directory);
            std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
            for (label, resolution) in [("1080p", [1920, 1080]), ("4k", [3840, 2160])] {
                let project = template_project(resolution);
                let ids = project
                    .layers
                    .iter()
                    .map(|layer| layer.id)
                    .collect::<Vec<_>>();
                for &format in &codecs {
                    let contents = interchange::export(&project, &ids, format, Options::default())?;
                    let filename = format!("canvas-pilot-{label}-2-layers.{}", format.extension());
                    let pages = interchange::import(&contents, &filename)?;
                    if pages.len() != 1
                        || pages[0]
                            .document
                            .shapes
                            .iter()
                            .map(|s| s.geometry)
                            .collect::<Vec<_>>()
                            != project.layers.iter().map(|l| l.bounds).collect::<Vec<_>>()
                    {
                        return Err(format!("Template round-trip mismatch: {filename}"));
                    }
                    write_file(
                        &directory.join(format!(
                            "canvas-pilot-{label}-2-layers.{}",
                            format.extension()
                        )),
                        &contents,
                        true,
                    )?;
                }
            }
        }
        _ => {
            return Err(
                "Usage: check_geometry_exchange generate DIRECTORY | templates DIRECTORY [EXTENSION...] | native DIRECTORY EXTENSION... | verify FILE...".into(),
            );
        }
    }
    Ok(())
}
