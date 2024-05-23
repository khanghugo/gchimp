use super::*;

#[derive(Default)]
pub struct S2G {}

impl TabProgram for S2G {
    fn tab_title(&self) -> eframe::egui::WidgetText {
        "S2G".into()
    }

    fn tab_ui(&self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.label("this is S2G panel");

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}
