use eframe::egui;

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
}
