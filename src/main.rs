use eframe::egui::{self, Color32};
use rodio::source::Buffered;
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod emitter;
mod grain;

use emitter::{Emitter, EmitterSettings};

type AudioClip = Buffered<Decoder<BufReader<File>>>;

struct NebulizerApp {
    stream_handle: OutputStreamHandle,
    sink: Sink,

    emitter_settings: Arc<Mutex<EmitterSettings>>,
}

fn main() {
    // initialize audio output stream
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let settings = Arc::new(Mutex::new(EmitterSettings::default()));
    let app = NebulizerApp {
        stream_handle,
        sink,
        emitter_settings: settings,
    };

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("nebulizer", native_options, Box::new(|_cc| Box::new(app)));
}

impl eframe::App for NebulizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("[UNDER CONSTRUCTION]");

            if ui.button("Open file").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    // attempt to load and decode audio file
                    if let Some(source) = load_audio_file(path.display().to_string()) {
                        let settings_copy = self.emitter_settings.clone();
                        let emitter = Emitter::new(source).periodic_access(
                            Duration::from_millis(5),
                            move |e| {
                                let settings = settings_copy.lock().unwrap();
                                e.settings = settings.deref().clone();
                            },
                        );
                        self.sink.clear();
                        self.sink.append(emitter);
                        self.sink.play();
                    } else {
                        ui.colored_label(Color32::RED, "Failed to read/decode audio file!");
                    }
                }
            }

            let mut settings = self.emitter_settings.lock().unwrap();
            ui.monospace("Emitter settings");
            ui.horizontal(|ui| {
                ui.label("Position");
                ui.add(egui::Slider::new(&mut settings.position, 0.0..=1.0));
            });

            ui.horizontal(|ui| {
                ui.label("Grain size");
                ui.add(egui::Slider::new(&mut settings.grain_size_ms, 1.0..=1000.0).suffix("ms"));
            });
        });
    }
}

fn load_audio_file(path: String) -> Option<AudioClip> {
    if let Some(file) = File::open(path).ok() {
        if let Some(decoder) = Decoder::new(BufReader::new(file)).ok() {
            Some(decoder.buffered())
        } else {
            None
        }
    } else {
        None
    }
}
