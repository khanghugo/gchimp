use super::*;

#[derive(Default)]
pub struct Map2Prop {}

impl TabProgram for Map2Prop {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "Map2Prop".into()
    }

    fn tab_ui(&self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.label("this is Map2Prop panel");

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
