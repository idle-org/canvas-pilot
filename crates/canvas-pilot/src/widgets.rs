//! Shared egui compatibility widgets.
use eframe::egui;

pub trait StableSelectableUi {
    fn stable_selectable_label<'a>(
        &mut self,
        selected: bool,
        text: impl egui::IntoAtoms<'a>,
    ) -> egui::Response;

    fn stable_selectable_value<'a, Value: PartialEq>(
        &mut self,
        current: &mut Value,
        value: Value,
        text: impl egui::IntoAtoms<'a>,
    ) -> egui::Response;
}

impl StableSelectableUi for egui::Ui {
    fn stable_selectable_label<'a>(
        &mut self,
        selected: bool,
        text: impl egui::IntoAtoms<'a>,
    ) -> egui::Response {
        // egui 0.35 drops the idle selectable frame but retains border-adjusted
        // padding, shrinking its layout by twice the stroke width. Keep the frame
        // in layout and make it transparent instead. Match Button's prior-frame
        // state lookup so hover/focus styling and hit testing stay in sync.
        let idle = self
            .ctx()
            .read_response(self.next_auto_id())
            .is_none_or(|response| {
                response.widget_state() == egui::widget_style::WidgetState::Inactive
            });
        let mut button = egui::Button::selectable(selected, text).frame_when_inactive(true);
        if !selected && idle {
            button = button
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(
                    self.visuals().widgets.inactive.bg_stroke.width,
                    egui::Color32::TRANSPARENT,
                ));
        }
        self.add(button)
    }

    fn stable_selectable_value<'a, Value: PartialEq>(
        &mut self,
        current: &mut Value,
        value: Value,
        text: impl egui::IntoAtoms<'a>,
    ) -> egui::Response {
        let mut response = self.stable_selectable_label(*current == value, text);
        if response.clicked() && *current != value {
            *current = value;
            response.mark_changed();
        }
        response
    }
}
