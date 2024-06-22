use std::{
    fs::File,
    io::BufReader,
    sync::mpsc::{self, Sender},
};

use eframe::egui::{self, Color32, Ui};
use midir::{MidiIO, MidiInput};
use midly::num::u4;
use rodio::{source::Buffered, Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::{
    emitter::{Emitter, EmitterMessage, EmitterSettings},
    midi::MidiConfig,
};

pub struct EmitterHandle {
    settings: EmitterSettings,
    channel: Sender<EmitterMessage>,
    midi_channel: u4,
}

pub struct NebulizerApp {
    stream: (OutputStream, OutputStreamHandle),
    sink: Sink,

    midi_config: MidiConfig,

    active_panel: GuiPanel,

    emitters: Vec<EmitterHandle>,
}

impl NebulizerApp {
    pub fn new() -> NebulizerApp {
        // setup audio stream
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        NebulizerApp {
            stream: (stream, stream_handle),
            sink,
            midi_config: MidiConfig::new(),
            active_panel: GuiPanel::Emitters,
            emitters: Vec::new(),
        }
    }
}

enum GuiPanel {
    Emitters,
    MidiSetup,
}

impl eframe::App for NebulizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Emitters").clicked() {
                    self.active_panel = GuiPanel::Emitters;
                }

                if ui.button("Midi Setup").clicked() {
                    self.active_panel = GuiPanel::MidiSetup;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.active_panel {
            GuiPanel::Emitters => emitters_panel(self, ui),
            GuiPanel::MidiSetup => midi_setup_panel(self, ui),
        });
    }
}

fn emitters_panel(app: &mut NebulizerApp, ui: &mut Ui) {
    if ui.button("New emitter").clicked() {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            // attempt to load and decode audio file
            if let Some(source) = load_audio_file(path.display().to_string()) {
                let (tx, rx) = mpsc::channel();
                let emitter = Emitter::new(source, rx);
                let handle = EmitterHandle {
                    settings: EmitterSettings::default(),
                    channel: tx,
                    midi_channel: u4::from(0),
                };
                app.emitters.push(handle);
                app.sink.stop();
                app.sink.append(emitter);
                app.sink.play();
                println!("Loaded emitter sound: {}", path.display().to_string());
            } else {
                ui.colored_label(Color32::RED, "Failed to read/decode audio file!");
            }
        }
    }

    for handle in app.emitters.iter_mut() {
        ui.separator();

        ui.monospace("Emitter settings");
        ui.horizontal(|ui| {
            ui.label("Position");
            ui.add(egui::Slider::new(&mut handle.settings.position, 0.0..=1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Grain size");
            ui.add(
                egui::Slider::new(&mut handle.settings.grain_size_ms, 1.0..=1000.0)
                    .suffix("ms")
                    .logarithmic(true),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Envelope");
            ui.add(egui::Slider::new(&mut handle.settings.envelope, 0.0..=1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Overlap");
            ui.add(egui::Slider::new(&mut handle.settings.overlap, 0.0..=0.99));
        });

        // send message to update emitter's settings
        let _ = handle
            .channel
            .send(EmitterMessage::Settings(handle.settings.clone()));
    }
}

fn midi_setup_panel(app: &mut NebulizerApp, ui: &mut Ui) {
    match &app.midi_config.connection {
        Some((name, _conn)) => {
            ui.label(format!("Connected to MIDI port: {}", name));
            if ui.button("Disconnect").clicked() {
                app.midi_config.connection = None;
            }
        }
        None => {
            ui.label("Click one to connect:");
            for port in app.midi_config.ports.clone().iter() {
                if ui
                    .button(app.midi_config.midi_in.port_name(&port).unwrap())
                    .clicked()
                {
                    app.midi_config.connect(port);
                }
            }
        }
    }
}

type AudioClip = Buffered<Decoder<BufReader<File>>>;

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
