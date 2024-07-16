mod app;
mod audio_clip;
mod emitter;
mod envelope;
mod grain;
mod midi;
mod numeric;
mod params;
mod widgets;

use app::NebulizerApp;
use eframe::egui::Vec2;

fn main() {
    let app = NebulizerApp::new();

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.inner_size = Some(Vec2 { x: 450.0, y: 440.0 });
    native_options.viewport.resizable = Some(false);

    let _ = eframe::run_native("nebulizer", native_options, Box::new(|_cc| Box::new(app)));
}
