use eframe::egui::{self, Color32};
use rodio::buffer::SamplesBuffer;
use rodio::source::Buffered;
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;

type AudioSource = Buffered<Decoder<BufReader<File>>>;

struct AudioFile {
    name: String,
    source: AudioSource,
}

struct NebulizerApp {
    stream_handle: OutputStreamHandle,
    sink: Sink,

    audio_file: Option<AudioFile>,
}

impl NebulizerApp {
    fn new(stream_handle: OutputStreamHandle, sink: Sink) -> NebulizerApp {
        NebulizerApp {
            stream_handle,
            sink,

            audio_file: None,
        }
    }
}

fn main() {
    // initialize audio output stream
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "nebulizer",
        native_options,
        Box::new(|_cc| Box::new(NebulizerApp::new(stream_handle, sink))),
    );
}

impl eframe::App for NebulizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("[UNDER CONSTRUCTION]");

            if ui.button("Open file").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    // attempt to load and decode audio file
                    if let Some(source) = load_audio_file(path.display().to_string()) {
                        self.audio_file = Some(AudioFile {
                            name: path.display().to_string(),
                            source,
                        })
                    } else {
                        ui.colored_label(Color32::RED, "Failed to read/decode audio file!");
                    }
                }
            }

            ui.label("File:");
            match &self.audio_file {
                None => {
                    ui.monospace("[none]");
                }
                Some(file) => {
                    ui.monospace(&file.name);
                    if ui.button("Play").clicked() {
                        self.sink.stop();

                        let source = file.source.clone();
                        self.sink.append(source);
                    }

                    if ui.button("Stop").clicked() {
                        self.sink.stop();
                    }
                }
            }
        });
    }
}

fn load_audio_file(path: String) -> Option<AudioSource> {
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