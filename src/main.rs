use eframe::egui::{self, Color32};
use rodio::source::Buffered;
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type AudioSource = Buffered<Decoder<BufReader<File>>>;

struct AudioFile {
    name: String,
    size: usize,
    source: AudioSource,
}

struct AppState {
    stream_handle: OutputStreamHandle,
    sink: Sink,

    audio_file: Option<AudioFile>,
    playing: bool,

    grain_start_position: f32,
    grain_size: u32,
}

impl AppState {
    fn new(stream_handle: OutputStreamHandle, sink: Sink) -> AppState {
        AppState {
            stream_handle,
            sink,

            audio_file: None,
            playing: false,

            grain_start_position: 0.0,
            grain_size: 20,
        }
    }
}

struct NebulizerApp {
    state: Arc<Mutex<AppState>>,
}

fn play_audio(state: Arc<Mutex<AppState>>) {
    loop {
        let state = state.lock().unwrap();

        if let Some(file) = &state.audio_file {
            if state.playing && state.sink.empty() {
                let start_time: Duration = Duration::from_millis(
                    ((state.grain_start_position * file.size as f32)
                        / file.source.sample_rate() as f32) as u64,
                );

                let grain = file
                    .source
                    .clone()
                    .skip_duration(start_time)
                    .take_duration(Duration::from_millis(state.grain_size as u64));
                state.sink.append(grain);
            }
        }
    }
}

fn main() {
    // initialize audio output stream
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let state = Arc::new(Mutex::new(AppState::new(stream_handle, sink)));
    let app = NebulizerApp {
        state: state.clone(),
    };

    thread::spawn(move || play_audio(state));

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("nebulizer", native_options, Box::new(|_cc| Box::new(app)));
}

impl eframe::App for NebulizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut state = self.state.lock().unwrap();
            ui.heading("[UNDER CONSTRUCTION]");

            if ui.button("Open file").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    // attempt to load and decode audio file
                    if let Some(source) = load_audio_file(path.display().to_string()) {
                        state.audio_file = Some(AudioFile {
                            name: path.display().to_string(),
                            size: source.clone().count(),
                            source,
                        })
                    } else {
                        ui.colored_label(Color32::RED, "Failed to read/decode audio file!");
                    }
                }
            }

            ui.label("File:");
            match &state.audio_file {
                None => {
                    ui.monospace("[none]");
                }
                Some(file) => {
                    ui.monospace(&file.name);
                    ui.horizontal(|ui| {
                        if ui.button("Play").clicked() {
                            state.playing = true;
                        }

                        if ui.button("Stop").clicked() {
                            state.sink.stop();
                            state.playing = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Start position");
                        ui.add(
                            egui::Slider::new(&mut state.grain_start_position, 0.0..=100.0)
                                .suffix("%"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Grain size");
                        ui.add(egui::Slider::new(&mut state.grain_size, 10..=100).suffix("ms"));
                    });
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
