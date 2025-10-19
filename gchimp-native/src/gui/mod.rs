use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use egui_wgpu::{wgpu, WgpuSetupCreateNew};

use eframe::egui::{self, ThemePreference};
use egui_tiles::Tree;
use utils::preview_file_being_dropped;

use gchimp::err;

use crate::{
    config::{parse_config, parse_config_from_file, Config},
    gui::programs::mdlscrub::render::pipeline::MdlScrubRenderer,
    persistent_storage::PersistentStorage,
};

use self::{
    constants::{PROGRAM_HEIGHT, PROGRAM_WIDTH},
    pane::{create_tree, Pane, TreeBehavior},
};

mod constants;
mod pane;
mod programs;
mod utils;

trait TabProgram {
    fn tab_title(&self) -> egui::WidgetText {
        "MyProgram".into()
    }

    fn tab_ui(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.separator();

        // Make it non drag-able
        egui_tiles::UiResponse::None
    }
}

pub fn gui() -> eyre::Result<()> {
    use crate::persistent_storage::PersistentStorage;

    let config_res = parse_config();

    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../.././media/logo.png")).unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // This is OKAY for now.
            .with_inner_size([PROGRAM_WIDTH, PROGRAM_HEIGHT])
            .with_drag_and_drop(true)
            .with_icon(icon)
            .with_maximize_button(true)
            .with_minimize_button(true),
        // from default egui_wgpu but with push constant enabled
        wgpu_options: egui_wgpu::WgpuConfiguration {
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(WgpuSetupCreateNew {
                device_descriptor: Arc::new(|adapter| {
                    let base_limits = if adapter.get_info().backend == wgpu::Backend::Gl {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    };

                    wgpu::DeviceDescriptor {
                        label: Some("egui wgpu device"),
                        required_limits: wgpu::Limits {
                            // When using a depth buffer, we have to be able to create a texture
                            // large enough for the entire surface, and we want to support 4k+ displays.
                            max_texture_dimension_2d: 8192,
                            max_push_constant_size: 128,
                            ..base_limits
                        },
                        required_features: wgpu::Features::PUSH_CONSTANTS,
                        ..Default::default()
                    }
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    let theme_preference = if let Ok(config) = &config_res {
        if config.theme.contains("light") {
            ThemePreference::Light
        } else if config.theme.contains("dark") {
            ThemePreference::Dark
        } else {
            ThemePreference::System
        }
    } else {
        ThemePreference::System
    };

    let persistent_storage = PersistentStorage::start()?;
    let persistent_storage = Arc::new(Mutex::new(persistent_storage));

    let gui_res = eframe::run_native(
        "gchimp",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let wgpu_render_state = cc.wgpu_render_state.as_ref().unwrap();
            let wgpu_context = WgpuContext {
                device: Arc::new(wgpu_render_state.device.clone()),
                queue: Arc::new(wgpu_render_state.queue.clone()),
                target_format: wgpu_render_state.target_format,
            };

            // wgpu_render_state.renderer.read().callback_resources

            Ok(Box::new(MyApp::new(
                config_res,
                persistent_storage,
                theme_preference,
                wgpu_context,
            )))
        }),
    );

    match gui_res {
        Ok(_) => Ok(()),
        Err(err) => err!("Error with running gchimp GUI: {}", err),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn gui() -> eyre::Result<()> {
    todo!("gui wasm32")
}

pub struct CustomRenderer {
    pub mdlscrub_renderer: MdlScrubRenderer,
}

#[derive(Debug, Clone)]
pub struct WgpuContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    target_format: wgpu::TextureFormat,
}

pub struct MyApp {
    tree: Option<Tree<Pane>>,
    _no_config_status: String,
    // duplicated because create_tree should have been a struct method
    persistent_storage: Arc<Mutex<PersistentStorage>>,
    theme: ThemePreference,
    wgpu_context: WgpuContext,
    // custom_renderer: CustomRenderer,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.options_mut(|option| option.theme_preference = self.theme);

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tree) = &mut self.tree {
                let mut behavior = TreeBehavior {};
                tree.ui(&mut behavior, ui);
            } else {
                if ui.button("Add config.toml").highlight().clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .pick_file()
                    {
                        self.update_from_config(path.as_path());
                    }
                }

                let mut readonly_buffer = self._no_config_status.as_str();
                ui.add(egui::TextEdit::multiline(&mut readonly_buffer));

                let ctx = ui.ctx();

                preview_file_being_dropped(ctx);

                ctx.input(|i| {
                    for dropped_file in i.raw.dropped_files.iter() {
                        if let Some(path) = &dropped_file.path {
                            if path.extension().unwrap() == "toml" {
                                self.update_from_config(path);
                            }
                        }
                    }
                });
            }
        });
    }
}

impl MyApp {
    pub fn new(
        config_res: eyre::Result<Config>,
        persistent_storage: Arc<Mutex<PersistentStorage>>,
        theme: ThemePreference,
        wgpu_context: WgpuContext,
    ) -> Self {
        let custom_renderer = CustomRenderer {
            mdlscrub_renderer: MdlScrubRenderer::new(wgpu_context.clone()),
        };

        if let Err(err) = config_res {
            return Self {
                tree: None,
                _no_config_status: format!("Error with parsing config.toml: {}", err),
                persistent_storage,
                theme,
                wgpu_context,
                // custom_renderer,
            };
        }

        Self {
            tree: Some(create_tree(
                config_res.unwrap(),
                persistent_storage.clone(),
                custom_renderer,
            )),
            _no_config_status: "".to_string(),
            persistent_storage,
            theme,
            wgpu_context,
            // custom_renderer,
        }
    }

    fn update_from_config(&mut self, path: &Path) {
        let config_res = parse_config_from_file(path);

        let new_app = Self::new(
            config_res,
            self.persistent_storage.clone(),
            self.theme,
            self.wgpu_context.clone(),
        );

        *self = new_app;
    }
}
