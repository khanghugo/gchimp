use eframe::egui;

use self::{map2prop::Map2Prop, s2g::S2G};

mod map2prop;
mod s2g;
mod utils;

trait TabProgram {
    fn tab_title(&self) -> egui::WidgetText;
    fn tab_ui(&self, ui: &mut egui::Ui) -> egui_tiles::UiResponse;
}

enum Pane {
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

    fn ui(&self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        match self {
            Pane::Map2Prop(m2p) => m2p.tab_ui(ui),
            Pane::S2G(s2g) => s2g.tab_ui(ui),
        }
    }
}

struct TreeBehavior {}

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

pub fn gui() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let mut tree = create_tree();

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut behavior = TreeBehavior {};
            tree.ui(&mut behavior, ui);
        });
    })
}

fn create_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];

    tabs.push(tiles.insert_pane(Pane::Map2Prop(Map2Prop::default())));
    tabs.push(tiles.insert_pane(Pane::S2G(S2G::default())));

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("my_tree", root, tiles)
}
