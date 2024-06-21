mod app;
mod emitter;
mod grain;
mod midi;

use app::NebulizerApp;

fn main() {
    // match midi::run() {
    //     Ok(_) => (),
    //     Err(err) => println!("FUCK {}", err),
    // }

    let app = NebulizerApp::new();

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("nebulizer", native_options, Box::new(|_cc| Box::new(app)));
}
