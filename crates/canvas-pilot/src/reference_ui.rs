use super::*;

impl Editor {
    pub(super) fn remove_reference(&mut self, i: usize) {
        if i >= self.references.len() {
            return;
        }
        self.references.remove(i);
        self.reference_selected = match self.reference_selected {
            Some(selected) if selected > i => Some(selected - 1),
            Some(selected) if selected == i => {
                self.references.len().checked_sub(1).map(|last| i.min(last))
            }
            selected => selected,
        };
        self.reference_error.clear();
    }
    pub(super) fn reference_actions_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Add reference images…").clicked() {
            ui.close_menu();
            self.import_reference_file(ui.ctx());
        }
        if ui
            .add_enabled(
                self.device.online(),
                egui::Button::new("Import input snapshot…"),
            )
            .on_disabled_hover_text("Connect to a device first.")
            .clicked()
        {
            self.open_snapshot_import();
            ui.close_menu();
        }
        if self.references.is_empty() {
            return;
        }
        let mut selected = self.reference_selected.unwrap_or(self.references.len() - 1);
        egui::ComboBox::from_id_salt("reference-choice")
            .selected_text(&self.references[selected].name)
            .show_ui(ui, |ui| {
                for (i, reference) in self.references.iter().enumerate() {
                    ui.selectable_value(&mut selected, i, &reference.name);
                }
            });
        self.reference_selected = Some(selected);
        if ui.button("Reference settings…").clicked() {
            self.reference_open = true;
            ui.close_menu();
        }
        if ui.button("Show only this reference").clicked() {
            self.show_only_reference(selected);
            ui.close_menu();
        }
        if ui.button("Remove reference image").clicked() {
            self.remove_reference(selected);
            ui.close_menu();
        }
    }
    fn show_only_reference(&mut self, selected: usize) {
        for (i, reference) in self.references.iter_mut().enumerate() {
            reference.visible = i == selected;
        }
    }
    pub(super) fn reference_rows(&mut self, ui: &mut egui::Ui, ink: &Ink, y: f32, w: f32) {
        if self.references.is_empty() {
            return;
        }
        let palette = theme::palette(ui.ctx());
        ink.text(0., y + 8., "REFERENCE IMAGES", 11., palette.purple);
        if ui
            .put(
                Rect::from_min_size(ink.point(w - 65., y), Vec2::new(65., 28.)),
                egui::Button::new("+ Add…"),
            )
            .clicked()
        {
            self.import_reference_file(ui.ctx());
        }
        let mut remove = None;
        for i in (0..self.references.len()).rev() {
            let row_y = y + 44. + (self.references.len() - 1 - i) as f32 * 44.;
            let selected =
                self.reference_selected == Some(i) && self.layout.selected_indices().is_empty();
            ink.card(
                0.,
                row_y,
                w,
                38.,
                palette.field,
                if selected {
                    palette.purple
                } else {
                    palette.line
                },
            );
            let label_rect =
                Rect::from_min_size(ink.point(10., row_y), Vec2::new((w - 90.).max(0.), 38.));
            ui.painter().with_clip_rect(label_rect).text(
                label_rect.left_center(),
                Align2::LEFT_CENTER,
                &self.references[i].name,
                FontId::proportional(14.),
                if self.references[i].visible {
                    palette.purple
                } else {
                    palette.muted
                },
            );
            let row = ui
                .interact(
                    label_rect,
                    egui::Id::new(("reference-row", i)),
                    egui::Sense::click(),
                )
                .on_hover_text(
                    "Fixed reference image · double-click for settings · right-click for actions",
                );
            if row.clicked() || row.secondary_clicked() {
                self.reference_selected = Some(i);
                self.layout.select(None);
                self.edit_content = false;
                self.sync();
            }
            if row.double_clicked() {
                self.reference_open = true;
            }
            row.context_menu(|ui| {
                if ui.button("Reference settings…").clicked() {
                    self.reference_selected = Some(i);
                    self.reference_open = true;
                    ui.close_menu();
                }
                if ui.button("Show only this reference").clicked() {
                    self.show_only_reference(i);
                    ui.close_menu();
                }
                if ui.button("Remove reference image").clicked() {
                    remove = Some(i);
                    ui.close_menu();
                }
            });
            let eye = Rect::from_min_size(ink.point(w - 72., row_y + 5.), Vec2::splat(28.));
            if state_icon(
                ui,
                eye,
                egui::Id::new(("reference-visible", i)),
                false,
                !self.references[i].visible,
                if self.references[i].visible {
                    "Hide reference image"
                } else {
                    "Show reference image"
                },
            )
            .clicked()
            {
                self.references[i].visible = !self.references[i].visible;
            }
            let close = Rect::from_min_size(ink.point(w - 36., row_y + 5.), Vec2::splat(28.));
            let response = ui
                .interact(
                    close,
                    egui::Id::new(("reference-remove", i)),
                    egui::Sense::click(),
                )
                .on_hover_text("Remove reference image");
            let color = if response.hovered() {
                palette.purple
            } else {
                palette.muted
            };
            for sign in [-1., 1.] {
                ui.painter().line_segment(
                    [
                        close.center() + Vec2::new(-4., sign * -4.),
                        close.center() + Vec2::new(4., sign * 4.),
                    ],
                    Stroke::new(1.5_f32, color),
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Remove reference image")
            });
            if response.clicked() {
                remove = Some(i);
            }
            // References deliberately have no drag payload, resize handles or device mapping.
        }
        if let Some(i) = remove {
            self.remove_reference(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{frame, mouse};
    use super::*;

    fn file(name: &str, color: [u8; 4]) -> egui::DroppedFile {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(4, 4, image::Rgba(color))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        egui::DroppedFile {
            name: name.into(),
            bytes: Some(bytes.into_inner().into()),
            ..Default::default()
        }
    }
    fn click_id(app: &mut Editor, ctx: &egui::Context, id: egui::Id) {
        frame(app, ctx, vec![]);
        frame(app, ctx, vec![]);
        let pos = ctx.read_response(id).unwrap().rect.center();
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }

    #[test]
    fn references_append_paint_and_toggle_independently_without_editable_geometry() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let geometry = app.layout.layers().clone();
        app.load_reference(&ctx, &file("First", [255, 0, 0, 255]));
        app.load_reference(&ctx, &file("Second", [0, 0, 255, 128]));
        app.wait_reference();
        assert_eq!(
            app.references
                .iter()
                .map(|r| r.name.as_str())
                .collect::<Vec<_>>(),
            ["First", "Second"]
        );
        let output = frame(&mut app, &ctx, vec![]);
        for reference in &app.references {
            assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Mesh(mesh) if mesh.texture_id == reference.texture.id())));
        }
        click_id(&mut app, &ctx, egui::Id::new(("reference-row", 1_usize)));
        assert!(app.layout.selected_indices().is_empty());
        let pos = ctx
            .read_response(egui::Id::new(("reference-row", 1_usize)))
            .unwrap()
            .rect
            .center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos + Vec2::new(0., 60.))],
        );
        frame(&mut app, &ctx, vec![mouse(pos + Vec2::new(0., 60.), false)]);
        assert_eq!(app.references[1].offset, [0.; 2]);
        assert_eq!(*app.layout.layers(), geometry);
        click_id(
            &mut app,
            &ctx,
            egui::Id::new(("reference-visible", 1_usize)),
        );
        assert!(!app.references[1].visible);
        assert!(app.references[0].visible);
        let output = frame(&mut app, &ctx, vec![]);
        assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Mesh(mesh) if mesh.texture_id == app.references[1].texture.id())));
        app.show_only_reference(1);
        assert!(!app.references[0].visible);
        assert!(app.references[1].visible);
        click_id(&mut app, &ctx, egui::Id::new(("reference-remove", 1_usize)));
        assert_eq!(app.references.len(), 1);
        assert_eq!(app.references[0].name, "First");
        assert_eq!(app.reference_selected, Some(0));
        app.remove_reference(0);
        assert!(app.references.is_empty());
        assert_eq!(app.reference_selected, None);
        assert_eq!(*app.layout.layers(), geometry);
    }

    #[test]
    fn removal_keeps_current_reference_identity_and_bad_addition_keeps_existing_images() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        for name in ["First", "Second", "Third"] {
            app.load_reference(&ctx, &file(name, [0, 255, 0, 255]));
        }
        app.wait_reference();
        app.remove_reference(0);
        assert_eq!(app.reference_selected, Some(1));
        assert_eq!(app.references[1].name, "Third");
        app.load_reference(
            &ctx,
            &egui::DroppedFile {
                bytes: Some(Vec::from(b"bad image").into()),
                ..Default::default()
            },
        );
        app.wait_reference();
        assert!(!app.reference_error.is_empty());
        assert_eq!(app.references.len(), 2);
        let project = app.project();
        let mut restored = Editor::default();
        restored.install_project(&ctx, project.clone()).unwrap();
        assert_eq!(restored.project(), project);
    }
}
