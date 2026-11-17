//! Compact, session-only browsing shared by installed language and theme packs.
use super::*;

#[derive(Clone, Default)]
struct Browser {
    query: String,
    selected: Option<String>,
    reveal: bool,
}

pub(super) struct Row {
    pub id: String,
    pub summary: String,
    pub search: String,
    pub active: bool,
}

pub(super) fn focus(ctx: &egui::Context, kind: &str, pack: &str) {
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("pack-browser", kind)),
            Browser {
                query: String::new(),
                selected: Some(pack.into()),
                reveal: true,
            },
        )
    });
}

pub(super) fn show(
    ui: &mut egui::Ui,
    kind: &str,
    mut rows: Vec<Row>,
    details: impl FnOnce(&mut egui::Ui, &str, f32),
) {
    let id = egui::Id::new(("pack-browser", kind));
    let mut state = ui
        .data(|data| data.get_temp::<Browser>(id))
        .unwrap_or_default();
    let previous_query = state.query.clone();
    let previous_selection = state.selected.clone();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .id(id.with("search"))
                .hint_text("Search packs or contents…")
                .desired_width((ui.available_width() - 60.0).max(80.0)),
        );
        if ui
            .add_enabled(!state.query.is_empty(), egui::Button::new("Clear"))
            .clicked()
        {
            state.query.clear();
        }
    });
    let query = state.query.trim().to_lowercase();
    let filtered: Vec<_> = rows
        .iter()
        .filter(|row| {
            query.is_empty()
                || row.search.to_lowercase().contains(&query)
                || row.id.contains(&query)
        })
        .collect();
    ui.weak(format!(
        "{} of {} packs · select a pack to inspect its contents",
        filtered.len(),
        rows.len()
    ));
    if !filtered
        .iter()
        .any(|row| state.selected.as_ref() == Some(&row.id))
    {
        state.selected = filtered
            .iter()
            .find(|row| row.active)
            .or_else(|| filtered.first())
            .map(|row| row.id.clone());
    }
    ui.spacing_mut().scroll = egui::style::ScrollStyle {
        bar_width: 10.0,
        foreground_color: true,
        ..egui::style::ScrollStyle::solid()
    };
    let wide = ui.available_width() >= 460.0;
    let height = (ui.ctx().content_rect().height() - 330.0).clamp(180.0, 360.0);
    let reveal =
        state.reveal || previous_query != state.query || previous_selection != state.selected;
    let list = |ui: &mut egui::Ui, height: f32, selected: &mut Option<String>| {
        egui::ScrollArea::vertical()
            .id_salt(id.with("list"))
            .max_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.weak(if rows.is_empty() {
                        "No packs installed. Install a ZIP to get started."
                    } else {
                        "No matches. Try a name, pack ID or language code."
                    });
                }
                ui.spacing_mut().item_spacing.y = 4.0;
                for row in &filtered {
                    let title_size = egui::TextStyle::Body.resolve(ui.style()).size;
                    let summary_size = egui::TextStyle::Small.resolve(ui.style()).size;
                    let (_, rect) = ui.allocate_space(Vec2::new(
                        ui.available_width(),
                        (title_size + summary_size + 14.0).max(40.0),
                    ));
                    let response = ui.interact(rect, id.with(&row.id), egui::Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("{} · {}", row.id, row.summary),
                        )
                    });
                    let chosen = selected.as_ref() == Some(&row.id);
                    let visuals = ui.style().interact_selectable(&response, chosen);
                    ui.painter().rect_filled(rect, 3.0, visuals.bg_fill);
                    ui.painter()
                        .rect_stroke(rect, 3.0, visuals.bg_stroke, StrokeKind::Inside);
                    let title = format!("{}{}", if row.active { "● " } else { "" }, row.id);
                    for (text, y, size, color) in [
                        (title, 4.0, title_size, visuals.text_color()),
                        (
                            row.summary.clone(),
                            title_size + 9.0,
                            summary_size,
                            visuals.text_color(),
                        ),
                    ] {
                        let galley = egui::WidgetText::from(egui::RichText::new(text).size(size))
                            .into_galley(
                                ui,
                                Some(egui::TextWrapMode::Truncate),
                                rect.width() - 12.0,
                                egui::TextStyle::Body,
                            );
                        ui.painter()
                            .galley(rect.min + Vec2::new(6.0, y), galley, color);
                    }
                    if chosen && reveal {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response
                        .on_hover_text(format!(
                            "{}\n{}{}",
                            row.id,
                            row.summary,
                            if row.active { "\nIn use" } else { "" }
                        ))
                        .clicked()
                    {
                        *selected = Some(row.id.clone());
                    }
                }
            });
    };
    if wide {
        let width = (ui.available_width() * 0.38).clamp(160.0, 240.0);
        ui.horizontal_top(|ui| {
            ui.set_height(height);
            ui.allocate_ui_with_layout(
                Vec2::new(width, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| list(ui, height, &mut state.selected),
            );
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                if let Some(selected) = &state.selected {
                    details(ui, selected, height);
                }
            });
        });
    } else {
        list(ui, 110.0, &mut state.selected);
        ui.separator();
        if let Some(selected) = &state.selected {
            details(ui, selected, height.min(220.0));
        }
    }
    state.reveal = false;
    ui.data_mut(|data| data.insert_temp(id, state));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows(count: usize) -> Vec<Row> {
        (0..count)
            .map(|i| Row {
                id: format!("pack-{i:03}"),
                summary: "16 themes · r2".into(),
                search: format!("Palette {i:03} 日本語"),
                active: i == 1,
            })
            .collect()
    }
    #[test]
    fn row_selection_search_and_clear_work_with_pointer_and_keyboard() {
        let ctx = egui::Context::default();
        ctx.global_style_mut(|s| s.animation_time = 0.0);
        let id = egui::Id::new(("pack-browser", "themes"));
        let run = |events| {
            let mut selected = None;
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(700.0, 720.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    show(ui, "themes", rows(20), |_, pack, _| {
                        selected = Some(pack.to_owned())
                    })
                },
            );
            (selected, output)
        };
        run(vec![]);
        run(vec![]);
        let pos = ctx
            .read_response(id.with("pack-005"))
            .unwrap()
            .rect
            .center();
        run(vec![
            egui::Event::PointerMoved(pos),
            super::super::tests::mouse(pos, true),
        ]);
        assert_eq!(
            run(vec![super::super::tests::mouse(pos, false)])
                .0
                .as_deref(),
            Some("pack-005")
        );
        ctx.memory_mut(|m| m.request_focus(id.with("search")));
        assert_eq!(
            run(vec![egui::Event::Text("Palette 019".into())])
                .0
                .as_deref(),
            Some("pack-019")
        );
        let output = run(vec![]).1;
        let clear = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "Clear" => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap();
        run(vec![
            egui::Event::PointerMoved(clear),
            super::super::tests::mouse(clear, true),
        ]);
        run(vec![super::super::tests::mouse(clear, false)]);
        assert!(
            ctx.data(|d| d.get_temp::<Browser>(id))
                .unwrap()
                .query
                .is_empty()
        );
        focus(&ctx, "themes", "pack-007");
        assert_eq!(run(vec![]).0.as_deref(), Some("pack-007"));
    }

    #[test]
    fn collections_stay_bounded_and_search_selects_contents_without_crossing_sections() {
        for width in [340.0, 700.0] {
            let ctx = egui::Context::default();
            ctx.global_style_mut(|s| s.animation_time = 0.0);
            let run = |kind, count| {
                let mut bounds = Rect::NOTHING;
                let mut selected = None;
                let _ = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 720.0))),
                        ..Default::default()
                    },
                    |ui| {
                        ui.set_width(width - 20.0);
                        show(ui, kind, rows(count), |ui, id, _| {
                            selected = Some(id.to_owned());
                            ui.label("Contents");
                        });
                        bounds = ui.min_rect();
                    },
                );
                (bounds, selected)
            };
            let (small, selected) = run("themes", 2);
            assert_eq!(selected.as_deref(), Some("pack-001"));
            let (large, _) = run("themes", 200);
            assert!((small.height() - large.height()).abs() < 2.0);
            assert!(large.width() <= width);
            assert!(
                large.height() < 500.0,
                "width={width} small={small:?} large={large:?}"
            );
            let id = egui::Id::new(("pack-browser", "themes"));
            ctx.data_mut(|d| {
                d.insert_temp(
                    id,
                    Browser {
                        query: "Palette 199".into(),
                        selected: Some("pack-001".into()),
                        ..Default::default()
                    },
                )
            });
            assert_eq!(run("themes", 200).1.as_deref(), Some("pack-199"));
            assert_eq!(run("languages", 2).1.as_deref(), Some("pack-001"));
            ctx.data_mut(|d| {
                d.insert_temp(
                    id,
                    Browser {
                        query: "missing".into(),
                        selected: None,
                        ..Default::default()
                    },
                )
            });
            assert!(run("themes", 200).1.is_none());
            ctx.data_mut(|d| {
                d.insert_temp(
                    id,
                    Browser {
                        query: "日本語".into(),
                        selected: Some("pack-199".into()),
                        ..Default::default()
                    },
                )
            });
            assert_eq!(run("themes", 200).1.as_deref(), Some("pack-199"));
            // Removal or update cannot leave details attached to a stale pack.
            assert_eq!(run("themes", 2).1.as_deref(), Some("pack-001"));
            assert!(run("themes", 0).1.is_none());
        }
    }
}
