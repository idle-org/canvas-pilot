//! Explicit size presets in output-screen pixels, committed as one geometry edit.
use crate::{Geometry, Layout};

/// Position and size selected layers without routing sources or changing stacking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizePreset {
    /// Every selected layer covers the output screen.
    Fullscreen,
    /// One half-width, half-height layer at a position in a 3×3 alignment grid.
    Quarter { column: u8, row: u8 },
    /// Fill cells left-to-right, top-to-bottom; excess layers repeat cells.
    Tiles { columns: u16, rows: u16 },
    /// Put the shorter dimension on X, preserving top-left position.
    Portrait,
    /// Put the longer dimension on X, preserving top-left position.
    Landscape,
}

impl Layout {
    /// Apply a preset in layer-list order, including members of collapsed folders.
    /// All affected geometry/crop changes succeed together and share one Undo step.
    pub fn apply_size_preset(&mut self, preset: SizePreset) -> Result<(), String> {
        self.apply_size_preset_with_proportions(preset, false)
    }

    /// Fit each layer inside its destination without changing its current aspect
    /// ratio when requested. Orientation presets remain explicit dimension swaps.
    pub fn apply_size_preset_with_proportions(
        &mut self,
        preset: SizePreset,
        keep_proportions: bool,
    ) -> Result<(), String> {
        let selected = self.selected_indices();
        if selected.is_empty() {
            return Err("Select at least one layer for a size preset.".into());
        }
        if selected.iter().any(|&i| !self.selectable(i)) {
            return Err("Unlock and show selected layers before resizing them.".into());
        }
        match preset {
            SizePreset::Quarter { column, row } if column > 2 || row > 2 || selected.len() != 1 => {
                return Err("Choose one layer and a position in the 3×3 quarter grid.".into());
            }
            SizePreset::Tiles { columns, rows }
                if !(1..=256).contains(&columns) || !(1..=256).contains(&rows) =>
            {
                return Err("Use 1–256 columns and rows.".into());
            }
            _ => {}
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut order = Vec::new();
        for i in (0..self.layers.len()).rev() {
            let folder = &self.info[i].folder;
            if folder.is_empty() {
                if self.selected[i] {
                    order.push(i);
                }
            } else if seen.insert(folder.clone()) {
                order.extend(
                    self.folder_members(folder)
                        .into_iter()
                        .rev()
                        .filter(|&j| self.selected[j]),
                );
            }
        }
        let [width, height] = self.formats.output.map(f64::from);
        let mut next = self.layers.clone();
        for (position, i) in order.into_iter().enumerate() {
            let original = next[i];
            next[i] = match preset {
                SizePreset::Fullscreen => Geometry {
                    x: 0.0,
                    y: 0.0,
                    width,
                    height,
                },
                SizePreset::Quarter { column, row } => Geometry {
                    x: f64::from(column) * width / 4.0,
                    y: f64::from(row) * height / 4.0,
                    width: width / 2.0,
                    height: height / 2.0,
                },
                SizePreset::Tiles { columns, rows } => {
                    let column = position
                        % (usize::from(columns) * usize::from(rows))
                        % usize::from(columns);
                    let row = position % (usize::from(columns) * usize::from(rows))
                        / usize::from(columns);
                    let cell_width = width / f64::from(columns);
                    let cell_height = height / f64::from(rows);
                    Geometry {
                        x: column as f64 * cell_width,
                        y: row as f64 * cell_height,
                        width: cell_width,
                        height: cell_height,
                    }
                }
                SizePreset::Portrait => Geometry {
                    width: original.width.min(original.height),
                    height: original.width.max(original.height),
                    ..original
                },
                SizePreset::Landscape => Geometry {
                    width: original.width.max(original.height),
                    height: original.width.min(original.height),
                    ..original
                },
            };
            if keep_proportions && !matches!(preset, SizePreset::Portrait | SizePreset::Landscape) {
                let target = next[i];
                let scale = (target.width / original.width).min(target.height / original.height);
                let width = original.width * scale;
                let height = original.height * scale;
                next[i] = Geometry {
                    x: target.x + (target.width - width) / 2.0,
                    y: target.y + (target.height - height) / 2.0,
                    width,
                    height,
                };
            }
        }
        self.finish_drag();
        self.commit(next)
    }
}

impl Layout {
    /// Replace layer cropping with a centered ratio crop from uncropped bounds.
    pub fn apply_crop_preset(&mut self, aspect: crate::formats::Aspect) -> Result<(), String> {
        self.apply_crop_preset_with_existing(aspect, false)
    }

    /// Optionally trim inside the existing crop instead of restoring its bounds.
    pub fn apply_crop_preset_with_existing(
        &mut self,
        aspect: crate::formats::Aspect,
        keep_existing: bool,
    ) -> Result<(), String> {
        use crate::formats::Aspect;
        let ratio = match aspect {
            Aspect::FourThree | Aspect::SixteenNine | Aspect::SixteenTen => aspect.ratio().unwrap(),
            _ => return Err("Choose a 4:3, 16:9 or 16:10 crop preset.".into()),
        };
        let selected = self.selected_indices();
        if selected.is_empty() {
            return Err("Select at least one layer for a crop preset.".into());
        }
        let mut layers = self.layers.clone();
        let mut formats = self.formats.clone();
        for i in selected {
            if !self.selectable(i) {
                return Err("Unlock and show selected layers before cropping them.".into());
            }
            let mut old = layers[i];
            if !keep_existing && formats.advanced[i] != Default::default() {
                let input = formats.input(i)?;
                let retained = formats.advanced[i].remaining(input)?;
                let width = old.width * input[0] / retained[0];
                let height = old.height * input[1] / retained[1];
                old = Geometry {
                    x: old.x + (old.width - width) / 2.0,
                    y: old.y + (old.height - height) / 2.0,
                    width,
                    height,
                }
                .validate()?;
                formats.advanced[i] = Default::default();
                layers[i] = old;
            }
            if (old.width / old.height - ratio).abs() <= ratio * 1e-10 {
                continue;
            }
            let width = old.width.min(old.height * ratio);
            let height = old.height.min(old.width / ratio);
            let next = Geometry {
                x: old.x + (old.width - width) / 2.0,
                y: old.y + (old.height - height) / 2.0,
                width,
                height,
            }
            .validate()?;
            let input = formats.input(i)?;
            let retained = formats.advanced[i].remaining(input)?;
            let image = match formats.layer_fit[i] {
                Some(mode) => mode.image(retained, old)?,
                None => old,
            };
            let crop = &mut formats.advanced[i];
            crop.left += ((next.x - image.x) / image.width * retained[0]).max(0.0);
            crop.right +=
                ((image.x + image.width - next.x - width) / image.width * retained[0]).max(0.0);
            crop.top += ((next.y - image.y) / image.height * retained[1]).max(0.0);
            crop.bottom +=
                ((image.y + image.height - next.y - height) / image.height * retained[1]).max(0.0);
            crop.remaining(input)?;
            formats.mapping(i, next)?;
            layers[i] = next;
        }
        self.finish_drag();
        let before = self.snapshot();
        self.layers = layers;
        self.formats = formats;
        self.remember(before);
        Ok(())
    }
}
