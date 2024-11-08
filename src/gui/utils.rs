use eframe::egui::{self, Align2, Color32, Context, Id, LayerId, Order, TextStyle, TextureHandle};

/// Preview hovering files:
pub fn preview_file_being_dropped(ctx: &egui::Context) {
    preview_files_being_dropped_min_max_file(ctx, 1, 1);
}

pub fn preview_files_being_dropped_min_max_file(ctx: &egui::Context, min: usize, max: usize) {
    if ctx.input(|i| min <= i.raw.hovered_files.len() && i.raw.hovered_files.len() <= max) {
        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            "Drag-n-Drop",
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}

// fn is_in_rect(p: Pos2, rect: Rect) -> bool {
//     let is_in = |v, min, max| min <= v && v <= max;

//     is_in(p.x, rect.min.x, rect.max.x) && is_in(p.y, rect.min.y, rect.max.y)
// }

// gamer
#[macro_export]
macro_rules! include_image {
    ($path:expr) => {{
        let mut buf: Vec<u8> = vec![];
        let _ = std::fs::OpenOptions::new()
            .read(true)
            .open($path)
            .unwrap()
            .read_to_end(&mut buf);

        let cow = format!("bytes://{}", $path);

        $crate::gui::egui::ImageSource::Bytes {
            uri: ::std::borrow::Cow::Borrowed(cow.leak()),
            bytes: $crate::gui::egui::load::Bytes::Static(buf.leak()),
        }
    }};
}

#[allow(dead_code)]
pub fn display_image_viewport_from_uri(
    ctx: &Context,
    uri: &str,
    name: impl AsRef<str> + Into<String> + std::hash::Hash,
) -> bool {
    let should_draw = ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of(&name),
        egui::ViewportBuilder::default()
            .with_title(name)
            .with_inner_size([512., 512.]),
        |ctx, _class| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::Image::from_uri(uri));

                if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape))
                {
                    return true;
                };

                false
            })
        },
    );

    should_draw.inner
}

pub fn display_image_viewport_from_texture(ctx: &Context, texture: &TextureHandle) -> bool {
    let should_draw = ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of(texture.name()),
        egui::ViewportBuilder::default()
            .with_title(texture.name())
            .with_inner_size(
                texture.size_vec2() + egui::Vec2 { x: 16., y: 16. }, // border :()
            ),
        |ctx, _class| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::Image::new(texture));

                if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape))
                {
                    return true;
                };

                false
            })
        },
    );

    should_draw.inner
}

#[derive(Clone)]
pub struct WadImage {
    name: String,
    dimensions: (u32, u32),
    texture: egui::TextureHandle,
}

impl WadImage {
    #[allow(dead_code)]
    pub fn new(
        handle: &egui::TextureHandle,
        name: impl AsRef<str> + Into<String>,
        dimensions: (u32, u32),
    ) -> Self {
        Self {
            texture: handle.clone(),
            name: name.into(),
            dimensions,
        }
    }

    pub fn from_wad_image(
        ui: &mut egui::Ui,
        name: impl AsRef<str> + Into<String>,
        image: &[u8],
        palette: &[[u8; 3]],
        dimensions: (u32, u32),
    ) -> Self {
        let image = image
            .iter()
            .flat_map(|color_index| palette[*color_index as usize])
            .collect::<Vec<u8>>();
        // Load the texture only once.
        let handle = ui.ctx().load_texture(
            name.as_ref(),
            egui::ColorImage::from_rgb([dimensions.0 as usize, dimensions.1 as usize], &image),
            Default::default(),
        );

        Self {
            texture: handle,
            name: name.into().to_owned(),
            dimensions,
        }
    }

    pub fn texture(&self) -> &egui::TextureHandle {
        &self.texture
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}
