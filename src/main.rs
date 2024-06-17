use eframe::egui;

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "nebulizer",
        native_options,
        Box::new(|_cc| Box::new(NebulizerApp::default())),
    );
}

#[derive(Default)]
struct NebulizerApp {
    dropped_file: Option<egui::DroppedFile>,
}

impl eframe::App for NebulizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("[UNDER CONSTRUCTION]");

            match &self.dropped_file {
                None => {
                    ui.label("Drag a file onto the window :)");
                }
                Some(file) => {
                    let info = if let Some(path) = &file.path {
                        path.display().to_string()
                    } else {
                        "???".to_owned()
                    };

                    ui.label("File:");
                    ui.monospace(info);
                }
            }
        });

        // collect dropped file
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_file = i.raw.dropped_files.first().cloned();
            }
        });
    }
}
