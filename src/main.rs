mod app;
mod audio_clip;
mod emitter;
mod envelope;
mod grain;
mod midi;
mod numeric;
mod widgets;

use nebulizer::Nebulizer;
use nih_plug::prelude::*;

fn main() {
    // let app = NebulizerApp::new();

    // let mut native_options = eframe::NativeOptions::default();
    // native_options.viewport.inner_size = Some(Vec2 { x: 450.0, y: 600.0 });

    // let _ = eframe::run_native("nebulizer", native_options, Box::new(|_cc| Box::new(app)));
    nih_export_standalone::<Nebulizer>();
}
