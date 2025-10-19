use std::sync::{Arc, Mutex};

use eframe::egui::{self, vec2, Id, Response, Sense, Stroke, TextStyle, Ui, Vec2, WidgetText};
use egui_tiles::{TabState, TileId, Tiles, UiResponse};

use crate::{config::Config, persistent_storage::PersistentStorage};

use super::{
    programs::{
        blbh::BLBHGui, map2mdl::Map2MdlGui, misc::Misc, s2g::S2GGui, skymod::SkyModGui,
        textile::TexTileGui, waddy::WaddyGui,
    },
    TabProgram,
};

pub enum Pane {
    Map2Prop(Map2MdlGui),
    S2G(S2GGui),
    SkyMod(SkyModGui),
    TexTile(TexTileGui),
    Waddy(WaddyGui),
    Blbh(BLBHGui),
    // DemDoc(DemDoc),
    Misc(Misc),
}

impl Pane {
    fn title(&self) -> egui::WidgetText {
        match self {
            Pane::Map2Prop(m2p) => m2p.tab_title(),
            Pane::S2G(s2g) => s2g.tab_title(),
            Pane::SkyMod(skymod) => skymod.tab_title(),
            Pane::TexTile(textile) => textile.tab_title(),
            Pane::Waddy(a) => a.tab_title(),
            // Pane::DemDoc(a) => a.tab_title(),
            Pane::Blbh(a) => a.tab_title(),
            Pane::Misc(misc) => misc.tab_title(),
        }
    }

    fn ui(&mut self, ui: &mut Ui) -> UiResponse {
        match self {
            Pane::Map2Prop(m2p) => m2p.tab_ui(ui),
            Pane::S2G(s2g) => s2g.tab_ui(ui),
            Pane::SkyMod(skymod) => skymod.tab_ui(ui),
            Pane::TexTile(textile) => textile.tab_ui(ui),
            Pane::Waddy(a) => a.tab_ui(ui),
            // Pane::DemDoc(a) => a.tab_ui(ui),
            Pane::Blbh(a) => a.tab_ui(ui),
            Pane::Misc(misc) => misc.tab_ui(ui),
        }
    }
}

pub fn create_tree(
    app_config: Config,
    persistent_storage: Arc<Mutex<PersistentStorage>>,
) -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let tabs = vec![
        // TODO maybe no need to clone config?
        // config is readonly so there should be a way with it
        tiles.insert_pane(Pane::S2G(S2GGui::new(app_config.clone()))),
        tiles.insert_pane(Pane::SkyMod(SkyModGui::new(app_config.clone()))),
        tiles.insert_pane(Pane::TexTile(TexTileGui::default())),
        tiles.insert_pane(Pane::Waddy(WaddyGui::new(persistent_storage))),
        tiles.insert_pane(Pane::Map2Prop(Map2MdlGui::new(app_config.clone()))),
        tiles.insert_pane(Pane::Blbh(BLBHGui::new(app_config.clone()))),
        // tiles.insert_pane(Pane::DemDoc(DemDoc::default())),
        tiles.insert_pane(Pane::Misc(Misc::default())),
    ];

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("my_tree", root, tiles)
}

pub struct TreeBehavior {}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> WidgetText {
        pane.title()
    }

    fn pane_ui(
        &mut self,
        ui: &mut Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        pane.ui(ui)
    }

    // The entire code again but just a small change so the users cannot drag the tabs.
    fn tab_ui(
        &mut self,
        tiles: &mut Tiles<Pane>,
        ui: &mut Ui,
        id: Id,
        tile_id: TileId,
        state: &TabState,
    ) -> Response {
        let text = self.tab_title_for_tile(tiles, tile_id);
        let close_btn_size = Vec2::splat(self.close_button_outer_size());
        let close_btn_left_padding = 4.0;
        let font_id = TextStyle::Button.resolve(ui.style());
        let galley = text.into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, font_id);

        let x_margin = self.tab_title_spacing(ui.visuals());

        let button_width = galley.size().x
            + 2.0 * x_margin
            + f32::from(state.closable) * (close_btn_left_padding + close_btn_size.x);
        let (_, tab_rect) = ui.allocate_space(vec2(button_width, ui.available_height()));

        // --- DISABLE DRAG ---
        let tab_response = ui
            .interact(tab_rect, id, Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        // Show a gap when dragged
        if ui.is_rect_visible(tab_rect) && !state.is_being_dragged {
            let bg_color = self.tab_bg_color(ui.visuals(), tiles, tile_id, state);
            let stroke = self.tab_outline_stroke(ui.visuals(), tiles, tile_id, state);
            ui.painter().rect(
                tab_rect.shrink(0.5),
                0.0,
                bg_color,
                stroke,
                egui::StrokeKind::Inside,
            );

            if state.active {
                // Make the tab name area connect with the tab ui area:
                ui.painter().hline(
                    tab_rect.x_range(),
                    tab_rect.bottom(),
                    Stroke::new(stroke.width + 1.0, bg_color),
                );
            }

            // Prepare title's text for rendering
            let text_color = self.tab_text_color(ui.visuals(), tiles, tile_id, state);
            let text_position = egui::Align2::LEFT_CENTER
                .align_size_within_rect(galley.size(), tab_rect.shrink(x_margin))
                .min;

            // Render the title
            ui.painter().galley(text_position, galley, text_color);

            // Conditionally render the close button
            if state.closable {
                let close_btn_rect = egui::Align2::RIGHT_CENTER
                    .align_size_within_rect(close_btn_size, tab_rect.shrink(x_margin));

                // Allocate
                let close_btn_id = ui.auto_id_with("tab_close_btn");
                let close_btn_response = ui
                    .interact(close_btn_rect, close_btn_id, Sense::click_and_drag())
                    .on_hover_cursor(egui::CursorIcon::Default);

                let visuals = ui.style().interact(&close_btn_response);

                // Scale based on the interaction visuals
                let rect = close_btn_rect
                    .shrink(self.close_button_inner_margin())
                    .expand(visuals.expansion);
                let stroke = visuals.fg_stroke;

                // paint the crossed lines
                ui.painter() // paints \
                    .line_segment([rect.left_top(), rect.right_bottom()], stroke);
                ui.painter() // paints /
                    .line_segment([rect.right_top(), rect.left_bottom()], stroke);

                // // Give the user a chance to react to the close button being clicked
                // // Only close if the user returns true (handled)
                // if close_btn_response.clicked() {
                //     log::debug!("Tab close requested for tile: {tile_id:?}");

                //     // Close the tab if the implementation wants to
                //     if self.on_tab_close(tiles, tile_id) {
                //         log::debug!("Implementation confirmed close request for tile: {tile_id:?}");

                //         tiles.remove(tile_id);
                //     } else {
                //         log::debug!("Implementation denied close request for tile: {tile_id:?}");
                //     }
                // }
            }
        }

        self.on_tab_button(tiles, tile_id, tab_response)
    }
}
