//! Source-space cropping and output-space framing. No device protocol assumptions.
use crate::Geometry;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aspect {
    FourThree,
    SixteenNine,
    SixteenTen,
    Square,
    Custom,
}
impl Aspect {
    pub const ALL: [Self; 5] = [
        Self::FourThree,
        Self::SixteenNine,
        Self::SixteenTen,
        Self::Square,
        Self::Custom,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::FourThree => "4:3",
            Self::SixteenNine => "16:9",
            Self::SixteenTen => "16:10",
            Self::Square => "1:1",
            Self::Custom => "Custom",
        }
    }
    pub fn ratio(self) -> Option<f64> {
        match self {
            Self::FourThree => Some(4.0 / 3.0),
            Self::SixteenNine => Some(16.0 / 9.0),
            Self::SixteenTen => Some(1.6),
            Self::Square => Some(1.0),
            Self::Custom => None,
        }
    }
}
/// Explicit crop-first fitting into a layer container. None in Formats retains
/// the original visible-box zoom/crop workflow.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerFit {
    OneToOne,
    Centered,
    Fullscreen,
    Cropped,
}
impl LayerFit {
    pub const ALL: [Self; 4] = [
        Self::OneToOne,
        Self::Centered,
        Self::Fullscreen,
        Self::Cropped,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::OneToOne => "1:1 pixels",
            Self::Centered => "Centered (fit)",
            Self::Fullscreen => "Fullscreen (stretch)",
            Self::Cropped => "Cropped (fill)",
        }
    }
    pub fn midra(self) -> &'static str {
        match self {
            Self::OneToOne => "1_1",
            Self::Centered => "CENTERED",
            Self::Fullscreen => "FULLSCREEN",
            Self::Cropped => "CROPPED",
        }
    }
    pub fn from_midra(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.midra() == value)
    }
    /// Unclipped retained-image bounds in output pixels. Fractional geometry is
    /// intentional; hardware subpixel rasterization is not assumed.
    pub fn image(self, retained: [f64; 2], container: Geometry) -> Result<Geometry, String> {
        validate_size(retained)?;
        container.validate()?;
        let ratios = [
            container.width / retained[0],
            container.height / retained[1],
        ];
        let scale = match self {
            Self::OneToOne => [1.0; 2],
            Self::Centered => [ratios[0].min(ratios[1]); 2],
            Self::Fullscreen => ratios,
            Self::Cropped => [ratios[0].max(ratios[1]); 2],
        };
        let size = [retained[0] * scale[0], retained[1] * scale[1]];
        Ok(Geometry {
            x: container.x + (container.width - size[0]) / 2.0,
            y: container.y + (container.height - size[1]) / 2.0,
            width: size[0],
            height: size[1],
        })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crop {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}
impl Crop {
    /// Move the image under a fixed crop window, in source pixels. Opposite
    /// edges trade pixels so retained size is unchanged; stop at source bounds.
    pub fn pan_image(self, delta: [f64; 2]) -> Result<Self, String> {
        if delta.iter().any(|v| !v.is_finite())
            || [self.left, self.right, self.top, self.bottom]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(
                "Crop edges and movement must be finite, with nonnegative crop edges.".into(),
            );
        }
        let dx = delta[0].clamp(-self.right, self.left);
        let dy = delta[1].clamp(-self.bottom, self.top);
        Ok(Self {
            left: self.left - dx,
            right: self.right + dx,
            top: self.top - dy,
            bottom: self.bottom + dy,
        })
    }
    pub fn remaining(self, size: [f64; 2]) -> Result<[f64; 2], String> {
        if [self.left, self.right, self.top, self.bottom]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err("Crop edges must be finite and nonnegative.".into());
        }
        let remaining = [
            size[0] - self.left - self.right,
            size[1] - self.top - self.bottom,
        ];
        validate_size(remaining)?;
        Ok(remaining)
    }
    pub fn centered(native: [f64; 2], size: [f64; 2]) -> Result<Self, String> {
        validate_size(native)?;
        validate_size(size)?;
        if size[0] > native[0] || size[1] > native[1] {
            return Err("Input resolution cannot exceed the native source dimensions.".into());
        }
        Ok(Self {
            left: (native[0] - size[0]) / 2.0,
            right: (native[0] - size[0]) / 2.0,
            top: (native[1] - size[1]) / 2.0,
            bottom: (native[1] - size[1]) / 2.0,
        })
    }
}
/// Device-neutral appearance imported atomically with geometry and source crop.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayerAppearance {
    pub fit: Option<LayerFit>,
    pub mask: Mask,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskOrigin {
    #[default]
    Layer,
    Source,
}
/// Independent blanking, measured as fractions of the selected reference box.
/// Opposite edges may meet (fully blank), but may not cross.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mask {
    pub origin: MaskOrigin,
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}
impl Mask {
    /// Reference box in output pixels; the usable source precedes layer cropping.
    pub fn reference(
        self,
        input: [f64; 2],
        crop: Crop,
        fit: Option<LayerFit>,
        layer: Geometry,
    ) -> Result<Geometry, String> {
        if self.origin == MaskOrigin::Layer {
            return Ok(layer);
        }
        let retained = crop.remaining(input)?;
        let image = match fit {
            Some(fit) => fit.image(retained, layer)?,
            None => layer,
        };
        let sx = image.width / retained[0];
        let sy = image.height / retained[1];
        Ok(Geometry {
            x: image.x - crop.left * sx,
            y: image.y - crop.top * sy,
            width: input[0] * sx,
            height: input[1] * sy,
        })
    }
    pub fn is_empty(self) -> bool {
        self.left == 0.0 && self.right == 0.0 && self.top == 0.0 && self.bottom == 0.0
    }

    pub fn validate(self) -> Result<Self, String> {
        if [self.left, self.right, self.top, self.bottom]
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.left + self.right > 1.0
            || self.top + self.bottom > 1.0
        {
            return Err(
                "Mask edges must be fractions between 0 and 1; opposite edges cannot cross.".into(),
            );
        }
        Ok(self)
    }
    pub fn bounds(self, g: Geometry) -> Geometry {
        Geometry {
            x: g.x + self.left * g.width,
            y: g.y + self.top * g.height,
            width: (1.0 - self.left - self.right).max(0.0) * g.width,
            height: (1.0 - self.top - self.bottom).max(0.0) * g.height,
        }
    }
    /// Translate the reveal window in output pixels, stopping at container edges.
    pub fn translated(self, delta: [f64; 2], g: Geometry) -> Result<Self, String> {
        self.validate()?;
        g.validate()?;
        if delta.iter().any(|v| !v.is_finite()) {
            return Err("Invalid mask movement.".into());
        }
        let dx = (delta[0] / g.width).clamp(-self.left, self.right);
        let dy = (delta[1] / g.height).clamp(-self.top, self.bottom);
        Ok(Self {
            left: self.left + dx,
            right: self.right - dx,
            top: self.top + dy,
            bottom: self.bottom - dy,
            ..self
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub aspect: Aspect,
    pub native: [u32; 2],
    pub input_crop: Crop,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            aspect: Aspect::SixteenNine,
            native: [1920, 1080],
            input_crop: Crop::default(),
        }
    }
}
impl Source {
    pub fn input(self) -> Result<[f64; 2], String> {
        validate_resolution(self.native)?;
        if let Some(ratio) = self.aspect.ratio()
            && (self.native[1] as f64 - self.native[0] as f64 / ratio).abs() > 0.500001
        {
            return Err("Native dimensions do not match Source format. Choose Custom or correct the native dimensions.".into());
        }
        self.input_crop.remaining(self.native.map(f64::from))
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub zoom: f64,
    pub offset: [f64; 2],
}
impl Default for Framing {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: [0.0; 2],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mapping {
    pub framing: Framing,
    pub scale: f64,
    pub retained: [f64; 2],
    pub offset_limits: [f64; 2],
}
pub fn validate_size(size: [f64; 2]) -> Result<(), String> {
    if size
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.0 || *v > 1_000_000.0)
    {
        return Err(
            "Dimensions must be positive, finite and no larger than 1,000,000 pixels.".into(),
        );
    }
    Ok(())
}
pub fn resolution_from_values(size: [f64; 2]) -> Result<[u32; 2], String> {
    if size
        .iter()
        .any(|v| !v.is_finite() || v.fract() != 0.0 || *v < 1.0 || *v > 16384.0)
    {
        return Err("Resolution must use whole pixels from 1 to 16384 per side.".into());
    }
    Ok(size.map(|v| v as u32))
}
pub fn validate_resolution(size: [u32; 2]) -> Result<(), String> {
    if size.iter().any(|v| !(1..=16384).contains(v)) {
        return Err("Resolution must use whole pixels from 1 to 16384 per side (planning limit, not a device mode).".into());
    }
    Ok(())
}
pub fn limits(input: [f64; 2], size: [f64; 2], zoom: f64) -> Result<(f64, [f64; 2]), String> {
    validate_size(input)?;
    validate_size(size)?;
    if !zoom.is_finite() || !(1.0..=100.0).contains(&zoom) {
        return Err("Content zoom must be between 100% and 10000% (planning limit).".into());
    }
    let k = (size[0] / input[0]).max(size[1] / input[1]) * zoom;
    Ok((
        k,
        [
            (k * input[0] - size[0]).max(0.0) / 2.0,
            (k * input[1] - size[1]).max(0.0) / 2.0,
        ],
    ))
}
pub fn crop_from_framing(
    input: [f64; 2],
    size: [f64; 2],
    framing: Framing,
) -> Result<Crop, String> {
    let (k, limit) = limits(input, size, framing.zoom)?;
    for (i, bound) in limit.iter().enumerate() {
        if !framing.offset[i].is_finite() || framing.offset[i].abs() > bound + 1e-7 {
            return Err(format!(
                "Content offset {} must be between {:.1} and {:.1} output px.",
                if i == 0 { "X" } else { "Y" },
                -bound,
                bound
            ));
        }
    }
    Ok(Crop {
        left: ((input[0] - size[0] / k) / 2.0 - framing.offset[0] / k).max(0.0),
        right: ((input[0] - size[0] / k) / 2.0 + framing.offset[0] / k).max(0.0),
        top: ((input[1] - size[1] / k) / 2.0 - framing.offset[1] / k).max(0.0),
        bottom: ((input[1] - size[1] / k) / 2.0 + framing.offset[1] / k).max(0.0),
    })
}
pub fn mapping(input: [f64; 2], size: [f64; 2], crop: Crop) -> Result<Mapping, String> {
    validate_size(input)?;
    validate_size(size)?;
    let retained = crop.remaining(input)?;
    let kx = size[0] / retained[0];
    let ky = size[1] / retained[1];
    // Recover scale from the less-cropped axis to avoid cancellation when
    // a tiny retained span is subtracted from a very wide source.
    let k = if retained[0] / input[0] >= retained[1] / input[1] {
        kx
    } else {
        ky
    };
    let tolerance =
        1e-8_f64.max(8.0 * f64::EPSILON * (input[0] / retained[0] + input[1] / retained[1]));
    if (kx - ky).abs() > tolerance * kx.max(ky) {
        return Err("Advanced crop aspect does not match the visible layer box.".into());
    }
    let zoom = k / (size[0] / input[0]).max(size[1] / input[1]);
    // Tolerate only floating-point roundoff at the documented zoom boundaries.
    if !(1.0 - 1e-8..=100.0 + 1e-8).contains(&zoom) {
        return Err("Crop requires content zoom outside 100%–10000%.".into());
    }
    let framing = Framing {
        zoom: zoom.clamp(1.0, 100.0),
        offset: [
            k * (crop.right - crop.left) / 2.0,
            k * (crop.bottom - crop.top) / 2.0,
        ],
    };
    let (_, offset_limits) = limits(input, size, framing.zoom)?;
    Ok(Mapping {
        framing,
        scale: k,
        retained,
        offset_limits,
    })
}
/// Preserve zoom and the normalized retained center after source or box changes.
/// Returns proposed offsets and whether moving the retained center was necessary.
pub fn reframed(
    old_input: [f64; 2],
    old_size: [f64; 2],
    crop: Crop,
    input: [f64; 2],
    size: [f64; 2],
) -> Result<(Crop, bool), String> {
    let old = mapping(old_input, old_size, crop)?;
    let center = [
        (crop.left + old.retained[0] / 2.0) / old_input[0],
        (crop.top + old.retained[1] / 2.0) / old_input[1],
    ];
    let (k, bounds) = limits(input, size, old.framing.zoom)?;
    let desired = [
        (0.5 - center[0]) * input[0] * k,
        (0.5 - center[1]) * input[1] * k,
    ];
    let offset = [
        desired[0].clamp(-bounds[0], bounds[0]),
        desired[1].clamp(-bounds[1], bounds[1]),
    ];
    let adjusted = (desired[0] - offset[0]).abs() > 1e-7 || (desired[1] - offset[1]).abs() > 1e-7;
    Ok((
        crop_from_framing(
            input,
            size,
            Framing {
                zoom: old.framing.zoom,
                offset,
            },
        )?,
        adjusted,
    ))
}
#[derive(Clone, Debug, PartialEq)]
pub struct Formats {
    pub output: [u32; 2],
    pub sources: Vec<Source>,
    pub source_for: Vec<usize>,
    pub advanced: Vec<Crop>,
    pub layer_fit: Vec<Option<LayerFit>>,
    pub masks: Vec<Mask>,
}
impl Default for Formats {
    fn default() -> Self {
        Self {
            output: [1920, 1080],
            sources: vec![Source::default(); 2],
            source_for: vec![0, 1],
            advanced: vec![Crop::default(); 2],
            layer_fit: vec![None; 2],
            masks: vec![Mask::default(); 2],
        }
    }
}
impl Formats {
    pub fn mask_reference(&self, i: usize, g: Geometry) -> Result<Geometry, String> {
        self.masks[i].reference(self.input(i)?, self.advanced[i], self.layer_fit[i], g)
    }
    pub fn mask_bounds(&self, i: usize, g: Geometry) -> Result<Geometry, String> {
        Ok(self.masks[i].bounds(self.mask_reference(i, g)?))
    }

    pub fn input(&self, i: usize) -> Result<[f64; 2], String> {
        self.sources[self.source_for[i]].input()
    }
    pub fn mapping(&self, i: usize, g: Geometry) -> Result<Mapping, String> {
        if let Some(mode) = self.layer_fit[i] {
            let retained = self.advanced[i].remaining(self.input(i)?)?;
            let image = mode.image(retained, g)?;
            return Ok(Mapping {
                framing: Framing::default(),
                scale: image.width / retained[0],
                retained,
                offset_limits: [0.0; 2],
            });
        }
        mapping(self.input(i)?, [g.width, g.height], self.advanced[i])
    }
    pub fn resize(&mut self, i: usize, before: Geometry, after: Geometry) -> Result<(), String> {
        if self.layer_fit[i].is_some()
            || [before.width, before.height] == [after.width, after.height]
        {
            return Ok(());
        }
        let (crop, _) = reframed(
            self.input(i)?,
            [before.width, before.height],
            self.advanced[i],
            self.input(i)?,
            [after.width, after.height],
        )?;
        self.advanced[i] = crop;
        Ok(())
    }
}
