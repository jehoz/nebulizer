use std::{
    fs::File,
    io::BufReader,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
};

use eframe::egui::{self, Color32, DragValue, Ui};
use egui_extras_xt::{common::WidgetShape, knobs::AudioKnob};
use midly::num::u4;
use rodio::{source::Buffered, Decoder, OutputStream, OutputStreamHandle, Source};

use crate::{
    emitter::{Emitter, EmitterMessage, EmitterSettings},
    midi::MidiConfig,
    widgets::envelope_plot::envelope_plot,
};

pub struct EmitterHandle {
    pub track_name: String,
    pub settings: EmitterSettings,
    pub channel: Sender<EmitterMessage>,
    pub midi_channel: u4,
}

pub struct NebulizerApp {
    stream: (OutputStream, OutputStreamHandle),

    midi_config: MidiConfig,

    active_panel: GuiPanel,

    emitters: Arc<Mutex<Vec<EmitterHandle>>>,
}

impl NebulizerApp {
    pub fn new() -> NebulizerApp {
        // setup audio stream
        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        NebulizerApp {
            stream: (stream, stream_handle),
            midi_config: MidiConfig::new(),
            active_panel: GuiPanel::Emitters,
            emitters: Arc::new(Mutex::new(Vec::new())),
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
    if ui.button("Load new sample").clicked() {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            // attempt to load and decode audio file
            if let Some(source) = load_audio_file(path.display().to_string()) {
                let (tx, rx) = mpsc::channel();
                let emitter = Emitter::new(source, rx);
                let handle = EmitterHandle {
                    track_name: path.file_name().unwrap().to_str().unwrap().to_string(),
                    settings: EmitterSettings::default(),
                    channel: tx,
                    midi_channel: u4::from(0),
                };
                let mut emitters = app.emitters.lock().unwrap();
                emitters.push(handle);
                let _ = app.stream.1.play_raw(emitter.convert_samples());
                println!("Loaded emitter sound: {}", path.display().to_string());
            } else {
                ui.colored_label(Color32::RED, "Failed to read/decode audio file!");
            }
        }
    }

    let mut emitters = app.emitters.lock().unwrap();
    *emitters = emitters
        .drain(0..)
        .enumerate()
        .filter_map(|(e, mut handle)| {
            let mut delete_clicked = false;
            ui.separator();

            ui.horizontal(|ui| {
                ui.monospace(&handle.track_name);

                if ui.button("X").clicked() {
                    delete_clicked = true;
                }
            });

            ui.push_id(e, |ui| {
                egui::ComboBox::from_label("MIDI Channel")
                    .selected_text(handle.midi_channel.to_string())
                    .show_ui(ui, |ui| {
                        for i in 0..=15 {
                            let chan = u4::from(i);
                            ui.selectable_value(&mut handle.midi_channel, chan, chan.to_string());
                        }
                    })
            });

            ui.columns(5, |columns| {
                columns[0].vertical_centered(|ui| {
                    ui.add(DragValue::new(&mut handle.settings.amplitude).clamp_range(0.0..=1.0));
                    ui.add(
                        AudioKnob::new(&mut handle.settings.amplitude)
                            .diameter(32.0)
                            .shape(WidgetShape::Circle)
                            .drag_length(4.0)
                            .spread(0.8),
                    );
                    ui.label("Level");
                });

                columns[1].vertical_centered(|ui| {
                    ui.add(DragValue::new(&mut handle.settings.position).clamp_range(0.0..=1.0));
                    ui.add(
                        AudioKnob::new(&mut handle.settings.position)
                            .diameter(32.0)
                            .shape(WidgetShape::Circle)
                            .drag_length(4.0)
                            .spread(0.8),
                    );
                    ui.label("Position");

                    ui.add(
                        AudioKnob::new(&mut handle.settings.position_rand)
                            .diameter(24.0)
                            .shape(WidgetShape::Circle)
                            .drag_length(4.0)
                            .spread(0.8),
                    );
                    ui.label("Rand");
                });

                columns[2].vertical_centered(|ui| {
                    ui.add(
                        DragValue::new(&mut handle.settings.grain_size)
                            .suffix(" ms")
                            .clamp_range(1.0..=1000.0),
                    );
                    ui.add(
                        AudioKnob::new(&mut handle.settings.grain_size)
                            .diameter(32.0)
                            .shape(WidgetShape::Circle)
                            .range(1.0..=1000.0)
                            .drag_length(4.0)
                            .spread(0.8),
                    );
                    ui.label("Grain length");

                    ui.add(
                        AudioKnob::new(&mut handle.settings.grain_size_rand)
                            .diameter(24.0)
                            .shape(WidgetShape::Circle)
                            .drag_length(4.0)
                            .spread(0.8),
                    );
                    ui.label("Rand");
                });

                columns[3].vertical_centered(|ui| {
                    ui.add(
                        DragValue::new(&mut handle.settings.density)
                            .suffix(" Hz")
                            .clamp_range(1.0..=100.0),
                    );
                    ui.add(
                        AudioKnob::new(&mut handle.settings.density)
                            .diameter(32.0)
                            .shape(WidgetShape::Circle)
                            .range(1.0..=100.0)
                            .drag_length(4.0)
                            .spread(0.8),
                    );
                    ui.label("Density");
                });

                columns[4].vertical_centered(|ui| {
                    ui.add(DragValue::new(&mut handle.settings.envelope).clamp_range(0.0..=1.0));
                    ui.add(
                        AudioKnob::new(&mut handle.settings.envelope)
                            .diameter(32.0)
                            .shape(WidgetShape::Circle)
                            .range(0.0..=1.0)
                            .drag_length(4.0)
                            .spread(0.8),
                    );
                    ui.label("Envelope");
                });
            });

            envelope_plot(ui, handle.settings.envelope, 0.0);

            ui.horizontal(|ui| {
                ui.label("Transpose");
                ui.add(egui::Slider::new(&mut handle.settings.transpose, -36..=36));
            });

            ui.collapsing("MIDI CC", |ui| {
                ui.columns(2, |columns| {
                    columns[0].label("Pitchbend");
                    columns[1].label("not ready yet :(")
                });
            });

            if delete_clicked {
                let _ = handle.channel.send(EmitterMessage::Terminate);

                // return none to remove emitter handle from list
                None
            } else {
                // send message to update emitter's settings
                let _ = handle
                    .channel
                    .send(EmitterMessage::Settings(handle.settings.clone()));

                Some(handle)
            }
        })
        .collect();
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
                let emitters = app.emitters.clone();
                if ui
                    .button(app.midi_config.midi_in.port_name(&port).unwrap())
                    .clicked()
                {
                    app.midi_config.connect(port, move |channel, message| {
                        let handles = emitters.lock().unwrap();
                        for handle in handles.iter() {
                            if handle.midi_channel == channel {
                                let _ = handle.channel.send(EmitterMessage::Midi(message));
                            }
                        }
                    });
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
