use eframe::egui::{self, Align2, Color32, Id, LayerId, Order, TextStyle};

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

        crate::gui::egui::ImageSource::Bytes {
            uri: ::std::borrow::Cow::Borrowed(cow.leak()),
            bytes: crate::gui::egui::load::Bytes::Static(buf.leak()),
        }
    }};
}
