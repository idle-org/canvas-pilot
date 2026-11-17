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
        }
        self.finish_drag();
        self.commit(next)
    }
}
