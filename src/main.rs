use eframe::egui::{self, Color32};
use rodio::source::Buffered;
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::ops::Deref;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

mod emitter;
mod grain;
mod midi;

use emitter::{Emitter, EmitterMessage, EmitterSettings};

type AudioClip = Buffered<Decoder<BufReader<File>>>;

struct NebulizerApp {
    stream_handle: OutputStreamHandle,
    sink: Sink,

    emitter_channel: Option<Sender<EmitterMessage>>,
    emitter_settings: EmitterSettings,
}

fn main() {
    // match midi::run() {
    //     Ok(_) => (),
    //     Err(err) => println!("FUCK {}", err),
    // }

    // initialize audio output stream
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let app = NebulizerApp {
        stream_handle,
        sink,
        emitter_channel: None,
        emitter_settings: EmitterSettings::default(),
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
                        let (tx, rx) = mpsc::channel();
                        let emitter = Emitter::new(source, rx);
                        self.emitter_channel = Some(tx);
                        self.sink.stop();
                        self.sink.append(emitter);
                    } else {
                        ui.colored_label(Color32::RED, "Failed to read/decode audio file!");
                    }
                }
            }

            ui.monospace("Emitter settings");
            ui.horizontal(|ui| {
                ui.label("Position");
                ui.add(egui::Slider::new(
                    &mut self.emitter_settings.position,
                    0.0..=1.0,
                ));
            });

            ui.horizontal(|ui| {
                ui.label("Grain size");
                ui.add(
                    egui::Slider::new(&mut self.emitter_settings.grain_size_ms, 1.0..=1000.0)
                        .suffix("ms"),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Envelope");
                ui.add(egui::Slider::new(
                    &mut self.emitter_settings.envelope,
                    0.0..=1.0,
                ));
            });

            ui.horizontal(|ui| {
                ui.label("Overlap");
                ui.add(egui::Slider::new(
                    &mut self.emitter_settings.overlap,
                    0.0..=0.99,
                ));
            });
        });

        // send message to update emitter's settings
        if let Some(channel) = &self.emitter_channel {
            let _ = channel.send(EmitterMessage::Settings(self.emitter_settings.clone()));
        }
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
