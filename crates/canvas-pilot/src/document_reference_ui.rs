use super::*;
use canvas_pilot::reference_document::Document;
use std::{
    collections::VecDeque,
    sync::mpsc::{self, Receiver},
};

type Job<T> = Option<Receiver<Result<T, String>>>;

#[derive(Default)]
pub(super) struct DocumentReferenceUi {
    queue: VecDeque<egui::DroppedFile>,
    file: Option<egui::DroppedFile>,
    loading: Job<Document>,
    rendering: Job<super::super::reference::Reference>,
    document: Option<Document>,
    preview: Option<super::super::reference::Reference>,
    page: usize,
    resolution: u32,
    layout_only: bool,
    error: String,
}

fn poll<T>(job: &mut Job<T>) -> Option<Result<T, String>> {
    match job.as_ref()?.try_recv() {
        Ok(result) => {
            *job = None;
            Some(result)
        }
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => {
            *job = None;
            Some(Err("Document reader stopped. Please try again.".into()))
        }
    }
}

impl DocumentReferenceUi {
    pub(super) fn enqueue(&mut self, file: egui::DroppedFile) -> Result<(), String> {
        if self.queue.len() >= 32 {
            return Err("Up to 32 reference documents can be queued.".into());
        }
        self.queue.push_back(file);
        Ok(())
    }

    fn load(&mut self, ctx: &egui::Context) {
        let Some(file) = self.file.clone() else {
            return;
        };
        let layout_only = self.layout_only;
        self.preview = None;
        self.document = None;
        self.rendering = None;
        self.error.clear();
        let (tx, rx) = mpsc::channel();
        self.loading = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let name = file
                    .path
                    .as_deref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(file.name);
                let bytes = if let Some(bytes) = file.bytes {
                    bytes.to_vec()
                } else {
                    canvas_pilot::reference_document::read(
                        file.path.as_deref().ok_or("Missing document path")?,
                    )?
                };
                Document::open(bytes, &name, layout_only)
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    fn render(&mut self, ctx: &egui::Context) {
        let Some(document) = self.document.clone() else {
            return;
        };
        self.preview = None;
        self.error.clear();
        let page = self.page;
        let resolution = self.resolution;
        let (tx, rx) = mpsc::channel();
        self.rendering = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = document.render(page, resolution).and_then(|bytes| {
                let mut reference = super::super::reference::Reference::load(
                    &ctx,
                    &egui::DroppedFile {
                        name: format!("{} · {}", document.name, document.pages[page]),
                        bytes: Some(bytes.into()),
                        ..Default::default()
                    },
                )?;
                reference.preserve_ratio = true;
                Ok(reference)
            });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    fn close(&mut self) {
        self.file = None;
        self.document = None;
        self.preview = None;
        self.loading = None;
        self.rendering = None;
        self.error.clear();
    }

    pub(super) fn show(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<super::super::reference::Reference> {
        if self.file.is_none()
            && let Some(file) = self.queue.pop_front()
        {
            self.file = Some(file);
            self.layout_only = false;
            self.page = 0;
            self.resolution = 1920;
            self.load(ctx);
        }
        if let Some(result) = poll(&mut self.loading) {
            match result {
                Ok(document) => {
                    self.document = Some(document);
                    self.page = 0;
                    self.render(ctx);
                }
                Err(error) => self.error = error,
            }
        }
        if let Some(result) = poll(&mut self.rendering) {
            match result {
                Ok(reference) => self.preview = Some(reference),
                Err(error) => self.error = error,
            }
        }
        self.file.as_ref()?;
        let mut open = true;
        let mut add = false;
        let mut cancel = false;
        let mut reload = false;
        let mut render = false;
        egui::Window::new("Import document as reference")
            .id(egui::Id::new("document-reference-import"))
            .open(&mut open)
            .default_width(600.0)
            .show(ctx, |ui| {
                ui.label("Choose a page and check its appearance before adding it.");
                ui.add_enabled_ui(self.loading.is_none() && self.rendering.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Render:");
                        reload |= ui
                            .selectable_value(&mut self.layout_only, false, "Artwork")
                            .changed();
                        reload |= ui
                            .selectable_value(&mut self.layout_only, true, "Layout boxes")
                            .changed();
                    });
                });
                if let Some(document) = &self.document {
                    ui.label(&document.name);
                    ui.label(&document.note);
                    ui.add_enabled_ui(self.rendering.is_none(), |ui| {
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("reference-document-page")
                                .selected_text(&document.pages[self.page])
                                .show_ui(ui, |ui| {
                                    for (i, name) in document.pages.iter().enumerate() {
                                        render |=
                                            ui.selectable_value(&mut self.page, i, name).changed();
                                    }
                                });
                            egui::ComboBox::from_id_salt("reference-document-resolution")
                                .selected_text(format!("{} px longest edge", self.resolution))
                                .show_ui(ui, |ui| {
                                    for size in [1280, 1920, 2560, 3840, 4096] {
                                        render |= ui
                                            .selectable_value(
                                                &mut self.resolution,
                                                size,
                                                format!("{size} px"),
                                            )
                                            .changed();
                                    }
                                });
                        });
                    });
                }
                if let Some(preview) = &self.preview {
                    ui.add(
                        egui::Image::new((preview.texture.id(), preview.texture.size_vec2()))
                            .max_size(egui::vec2(560.0, 330.0)),
                    );
                    ui.small(format!(
                        "{} × {} px · proportions preserved",
                        preview.size[0], preview.size[1]
                    ));
                }
                if self.loading.is_some() || self.rendering.is_some() {
                    ui.spinner();
                    ui.label("Preparing reference…");
                }
                if !self.error.is_empty() {
                    ui.colored_label(theme::palette(ctx).amber, &self.error);
                }
                add = ui
                    .add_enabled(
                        self.preview.is_some() && !reload && !render,
                        egui::Button::new("Add reference"),
                    )
                    .clicked();
                cancel = ui.button("Cancel").clicked();
            });
        if !open || cancel {
            self.close();
            return None;
        }
        if reload {
            self.load(ctx);
        } else if render {
            self.render(ctx);
        }
        if add {
            let reference = self.preview.take();
            self.close();
            return reference;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{frame, mouse};
    use super::*;

    fn wait_preview(app: &mut Editor, ctx: &egui::Context) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while app.document_reference.preview.is_none()
            && app.document_reference.error.is_empty()
            && std::time::Instant::now() < until
        {
            frame(app, ctx, vec![]);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            app.document_reference.error.is_empty(),
            "{}",
            app.document_reference.error
        );
        assert!(app.document_reference.preview.is_some());
    }

    #[test]
    fn pdf_review_adds_selected_page_as_portable_reference_without_changing_layers() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.project();
        app.load_reference(
            &ctx,
            &egui::DroppedFile {
                name: "client.pdf".into(),
                bytes: Some(
                    include_bytes!("../tests/fixtures/reference-pages.pdf")
                        .to_vec()
                        .into(),
                ),
                ..Default::default()
            },
        );
        wait_preview(&mut app, &ctx);
        assert_eq!(
            app.project(),
            before,
            "preview must not change the document"
        );
        app.document_reference.page = 1;
        app.document_reference.resolution = 200;
        app.document_reference.render(&ctx);
        wait_preview(&mut app, &ctx);
        let output = frame(&mut app, &ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "Add reference" => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        assert_eq!(app.references.len(), 1);
        let reference = &app.references[0];
        assert_eq!(reference.name, "client.pdf · Page 2");
        assert!(reference.preserve_ratio);
        assert_eq!(reference.size, [200, 200]);
        assert_eq!(app.project().layers, before.layers);
        let saved =
            canvas_pilot::project::Project::from_json(&app.project().to_json().unwrap()).unwrap();
        let mut restored = Editor::default();
        restored.install_project(&ctx, saved).unwrap();
        assert_eq!(restored.references[0].bytes, reference.bytes);
        assert!(restored.references[0].preserve_ratio);
        assert!(app.document_reference.file.is_none());
    }

    #[test]
    fn cancellation_and_failed_documents_keep_existing_project_and_ignore_late_results() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.project();
        app.document_reference.file = Some(egui::DroppedFile {
            name: "bad.pdf".into(),
            bytes: Some(b"broken".to_vec().into()),
            ..Default::default()
        });
        app.document_reference.load(&ctx);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.document_reference.error.is_empty() && std::time::Instant::now() < until {
            frame(&mut app, &ctx, vec![]);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(app.document_reference.error.contains("Cannot open PDF"));
        assert_eq!(app.project(), before);
        let (tx, rx) = mpsc::channel();
        app.document_reference.loading = Some(rx);
        app.document_reference.close();
        assert!(tx.send(Err("late result".into())).is_err());
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.project(), before);
        assert!(app.document_reference.error.is_empty());
        let (tx, rx) = mpsc::channel::<Result<Document, String>>();
        drop(tx);
        assert!(poll(&mut Some(rx)).unwrap().is_err());
    }
}
