use eframe::egui::{self, Sense};

use super::{
    programs::{map2prop::Map2Prop, s2g::S2G},
    TabProgram,
};

pub enum Pane {
    Map2Prop(Map2Prop),
    S2G(S2G),
}

impl Pane {
    fn title(&self) -> egui::WidgetText {
        match self {
            Pane::Map2Prop(m2p) => m2p.tab_title(),
            Pane::S2G(s2g) => s2g.tab_title(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        match self {
            Pane::Map2Prop(m2p) => m2p.tab_ui(ui),
            Pane::S2G(s2g) => s2g.tab_ui(ui),
        }
    }
}

pub fn create_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];

    tabs.push(tiles.insert_pane(Pane::Map2Prop(Map2Prop::default())));
    tabs.push(tiles.insert_pane(Pane::S2G(S2G::default())));

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("my_tree", root, tiles)
}

pub struct TreeBehavior {}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.title()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        pane.ui(ui)
    }

    // The entire code again but just a small change so the users cannot drag the tabs.
    fn tab_ui(
        &mut self,
        tiles: &egui_tiles::Tiles<Pane>,
        ui: &mut egui::Ui,
        id: egui::Id,
        tile_id: egui_tiles::TileId,
        active: bool,
        is_being_dragged: bool,
    ) -> egui::Response {
        let text = self.tab_title_for_tile(tiles, tile_id);
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        let galley = text.into_galley(ui, Some(false), f32::INFINITY, font_id);

        let x_margin = self.tab_title_spacing(ui.visuals());
        let (_, rect) = ui.allocate_space(egui::vec2(
            galley.size().x + 2.0 * x_margin,
            ui.available_height(),
        ));

        // Sense::click() instead so the tab cannot be dragged anymore.
        let response = ui.interact(rect, id, Sense::click());

        // Show a gap when dragged
        if ui.is_rect_visible(rect) && !is_being_dragged {
            let bg_color = self.tab_bg_color(ui.visuals(), tiles, tile_id, active);
            let stroke = self.tab_outline_stroke(ui.visuals(), tiles, tile_id, active);
            ui.painter().rect(rect.shrink(0.5), 0.0, bg_color, stroke);

            if active {
                // Make the tab name area connect with the tab ui area:
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    egui::Stroke::new(stroke.width + 1.0, bg_color),
                );
            }

            let text_color = self.tab_text_color(ui.visuals(), tiles, tile_id, active);
            ui.painter().galley(
                egui::Align2::CENTER_CENTER
                    .align_size_within_rect(galley.size(), rect)
                    .min,
                galley,
                text_color,
            );
        }

        self.on_tab_button(tiles, tile_id, response)
    }
}
