mod app;
mod emitter;
mod grain;
mod midi;

use app::NebulizerApp;
use eframe::egui::{Style, Visuals};

fn main() {
    let app = NebulizerApp::new();

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "nebulizer",
        native_options,
        Box::new(|cc| {
            let style = Style {
                visuals: Visuals::dark(),
                ..Style::default()
            };
            cc.egui_ctx.set_style(style);
            Box::new(app)
        }),
    );
}
