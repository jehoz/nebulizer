mod app;
mod audio_clip;
mod emitter;
mod envelope;
mod grain;
mod midi;
mod numeric;
mod widgets;

use app::NebulizerApp;
use eframe::egui::{Style, Vec2, Visuals};

fn main() {
    let app = NebulizerApp::new();

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.inner_size = Some(Vec2 { x: 450.0, y: 600.0 });

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
