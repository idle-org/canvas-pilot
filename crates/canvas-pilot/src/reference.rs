use super::*;
use std::io::{Cursor, Read};
// Keep large concurrent imports from multiplying decoder peak memory.
static DECODE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone)]
pub(super) struct Reference {
    pub texture: egui::TextureHandle,
    pub name: String,
    pub bytes: Vec<u8>,
    pub size: [u32; 2],
    pub visible: bool,
    pub opacity: f32,
    pub preserve_ratio: bool,
    pub native_size: bool,
    pub offset: [f64; 2],
}

impl Reference {
    pub fn load(ctx: &egui::Context, file: &egui::DroppedFile) -> Result<Self, String> {
        let _decode_guard = DECODE_LOCK
            .lock()
            .map_err(|_| "Image decoder lock failed")?;
        let bytes = if let Some(bytes) = &file.bytes {
            bytes.to_vec()
        } else if let Some(path) = &file.path {
            let mut bytes = Vec::new();
            std::fs::File::open(path)
                .map_err(|e| format!("Cannot open reference image: {e}"))?
                .take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| format!("Cannot read reference image: {e}"))?;
            bytes
        } else {
            return Err("Drop a PNG, JPEG or BMP file.".into());
        };
        let rgba = decode(&bytes, ctx.input(|i| i.max_texture_side) as u32)?;
        let size = [rgba.width(), rgba.height()];
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [size[0] as usize, size[1] as usize],
            rgba.as_raw(),
        );
        let name = file
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.name.clone());
        Ok(Self {
            texture: ctx.load_texture(
                "reference",
                color,
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Linear,
                    ..Default::default()
                },
            ),
            name,
            bytes,
            size,
            visible: true,
            opacity: 1.0,
            preserve_ratio: false,
            native_size: false,
            offset: [0.0; 2],
        })
    }
    pub fn warning(&self, output: [u32; 2]) -> Option<(bool, String)> {
        if self.native_size {
            None
        } else {
            warning(self.size, self.preserve_ratio, output)
        }
    }
    pub fn paint(&self, painter: &egui::Painter, screen: Rect, output: [u32; 2]) {
        if !self.visible {
            return;
        }
        let rect = self.placed_rect(screen, output);
        painter
            .with_clip_rect(painter.clip_rect().intersect(screen))
            .image(
                self.texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE.gamma_multiply(self.opacity),
            );
    }
    fn placed_rect(&self, screen: Rect, output: [u32; 2]) -> Rect {
        let scale = screen.width() / output[0] as f32;
        let rect = if self.native_size {
            Rect::from_min_size(
                screen.min,
                Vec2::new(self.size[0] as f32, self.size[1] as f32) * scale,
            )
        } else if self.preserve_ratio {
            let size = Vec2::new(self.size[0] as f32, self.size[1] as f32);
            let scale = (screen.width() / size.x).min(screen.height() / size.y);
            Rect::from_center_size(screen.center(), size * scale)
        } else {
            screen
        };
        rect.translate(Vec2::new(self.offset[0] as f32, self.offset[1] as f32) * scale)
    }
}

fn decode(bytes: &[u8], max_side: u32) -> Result<image::RgbaImage, String> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Reference image exceeds 64 MiB.".into());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::Bmp)
    ) {
        return Err("Unsupported reference image. Use PNG, JPEG or BMP.".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(max_side.min(8192));
    limits.max_image_height = Some(max_side.min(8192));
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(|i| i.to_rgba8())
        .map_err(|e| format!("Cannot decode reference image: {e}"))
}

fn warning([w, h]: [u32; 2], preserve_ratio: bool, [ow, oh]: [u32; 2]) -> Option<(bool, String)> {
    let different_ratio = u64::from(w) * u64::from(oh) != u64::from(h) * u64::from(ow);
    if different_ratio && !preserve_ratio {
        Some((
            true,
            format!(
                "SEVERE WARNING · {w} × {h} to {ow} × {oh}: aspect ratio changed; picture distorted. Import remains usable."
            ),
        ))
    } else if [w, h] != [ow, oh] {
        Some((
            false,
            format!(
                "WARNING · {w} × {h}: scaled to fit, proportions preserved{}.",
                if different_ratio {
                    "; empty margins"
                } else {
                    ""
                }
            ),
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_mapping_and_offsets_are_in_screen_pixels() {
        let ctx = egui::Context::default();
        let mut bytes = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(4, 2, image::Rgba([40, 90, 150, 255]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let file = egui::DroppedFile {
            name: "guide.png".into(),
            bytes: Some(bytes.into_inner().into()),
            ..Default::default()
        };
        let mut reference = Reference::load(&ctx, &file).unwrap();
        reference.native_size = true;
        reference.offset = [20.0, 30.0];
        let screen = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(960.0, 540.0));
        assert_eq!(
            reference.placed_rect(screen, [1920, 1080]),
            Rect::from_min_size(Pos2::new(20.0, 35.0), Vec2::new(2.0, 1.0))
        );
        assert!(reference.warning([1920, 1080]).is_none());
        reference.native_size = false;
        reference.preserve_ratio = false;
        assert_eq!(
            reference.placed_rect(screen, [1920, 1080]).size(),
            screen.size()
        );
        assert!(reference.warning([1920, 1080]).unwrap().0);
    }
    #[test]
    fn scaling_warnings_distinguish_resolution_and_ratio() {
        assert!(warning([1920, 1080], false, [1920, 1080]).is_none());
        assert!(warning([1920, 1080], false, [1920, 1200]).unwrap().0);
        assert!(!warning([1920, 1080], false, [3840, 2160]).unwrap().0);
        assert!(warning([1920, 1200], false, [1920, 1200]).is_none());
        for size in [[3840, 2160], [1280, 720]] {
            assert!(!warning(size, false, [1920, 1080]).unwrap().0);
        }
        assert!(warning([1920, 1200], false, [1920, 1080]).unwrap().0);
        assert!(!warning([1920, 1200], true, [1920, 1080]).unwrap().0);
    }
    #[test]
    fn decodes_pixels_and_rejects_corrupt_or_oversized_images() {
        let image = image::RgbaImage::from_pixel(4, 2, image::Rgba([30, 60, 90, 128]));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        assert_eq!(decode(bytes.get_ref(), 4096).unwrap(), image);
        assert!(decode(bytes.get_ref(), 2).is_err());
        assert!(decode(b"bad image", 4096).is_err());
    }
}
