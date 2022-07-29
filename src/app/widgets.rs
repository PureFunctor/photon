use eframe::egui;

/// A colored button activated by a keypress or click.
pub struct EffectPad<'a> {
    name: &'a str,
    key: egui::Key,
    color: egui::Color32,
}

/// An event emitted during interaction with the effect pad.
pub enum EffectPadEvent {
    On,
    Off,
}

/// The total area occupied by the effect pad.
pub const PAD_TOTAL_SIZE: egui::Vec2 = egui::vec2(100.0, 60.0);

/// The height allocated to the effect pad's label.
pub const PAD_LABEL_HEIGHT: egui::Vec2 = egui::vec2(0.0, 20.0);

impl<'a> EffectPad<'a> {
    /// Creates a new [`EffectPad`].
    pub fn new(name: &'a str, key: egui::Key, color: impl Into<egui::Color32>) -> Self {
        Self {
            name,
            key,
            color: color.into(),
        }
    }

    /// Renders the effect pad and runs a callback on interaction.
    pub fn show(&self, ui: &mut egui::Ui, mut on_key: impl FnMut(EffectPadEvent)) {
        let (total_rect, _) = ui.allocate_exact_size(PAD_TOTAL_SIZE, egui::Sense::hover());

        let button_rect = egui::Rect::from_points(&[
            total_rect.left_top() + PAD_LABEL_HEIGHT,
            total_rect.right_top() + PAD_LABEL_HEIGHT,
            total_rect.left_bottom(),
            total_rect.right_bottom(),
        ]);

        let label_rect = egui::Rect::from_points(&[
            total_rect.left_top(),
            total_rect.right_top(),
            button_rect.left_top(),
            button_rect.right_top(),
        ]);

        let button_response =
            ui.allocate_rect(button_rect, egui::Sense::hover().union(egui::Sense::drag()));

        if ui.is_rect_visible(total_rect) {
            if button_response.hovered() {
                ui.output().cursor_icon = egui::CursorIcon::PointingHand;
            }

            let is_on = ui.input().key_down(self.key) || button_response.dragged();
            let is_off = ui.input().key_released(self.key) || button_response.drag_released();

            if is_on {
                on_key(EffectPadEvent::On);
            } else if is_off {
                on_key(EffectPadEvent::Off);
            }

            if is_on {
                let mut hsva_color: egui::color::Hsva = self.color.into();
                hsva_color.v -= 0.2;
                ui.painter()
                    .rect(button_rect, 5.0, hsva_color, egui::Stroke::none());
            } else {
                ui.painter()
                    .rect(button_rect, 5.0, self.color, egui::Stroke::none());
            }

            ui.painter().text(
                label_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{} - {:?}", self.name, self.key),
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        }
    }
}
