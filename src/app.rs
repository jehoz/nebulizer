use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};

use eframe::egui::{self, Color32, Ui};
use midly::num::u4;
use rodio::{OutputStream, OutputStreamHandle, Source};

use crate::{
    audio_clip::AudioClip,
    emitter::{Emitter, EmitterMessage, EmitterSettings, KeyMode},
    midi::MidiConfig,
    widgets::{
        envelope_plot::EnvelopePlot,
        parameter_knob::ParameterKnob,
        waveform::{Waveform, WaveformData},
    },
};

pub struct EmitterHandle {
    pub track_name: String,
    pub waveform: WaveformData,
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
            if let Some(clip) = AudioClip::<f32>::load_from_file(path.display().to_string()) {
                let (tx, rx) = mpsc::channel();
                let emitter = Emitter::new(clip.clone(), rx);
                let handle = EmitterHandle {
                    track_name: path.file_name().unwrap().to_str().unwrap().to_string(),
                    waveform: WaveformData::new(clip),
                    settings: EmitterSettings::default(),
                    channel: tx,
                    midi_channel: u4::from(0),
                };
                let mut emitters = app.emitters.lock().unwrap();
                emitters.push(handle);
                let _ = app.stream.1.play_raw(emitter.convert_samples());
            } else {
                // TODO make some error popup window since this is only visible for one frame
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

            match handle.settings.key_mode {
                KeyMode::Pitch => {
                    ui.add(
                        Waveform::new(handle.waveform.clone())
                            .playhead(handle.settings.position, handle.settings.length_ms),
                    );
                }
                KeyMode::Slice(_) => {
                    ui.add(Waveform::new(handle.waveform.clone()));
                }
            }

            match handle.settings.key_mode {
                KeyMode::Pitch => {
                    if ui.button("Note => Pitch").clicked() {
                        handle.settings.key_mode = KeyMode::Slice(12);
                    }
                }
                KeyMode::Slice(num_slices) => {
                    if ui.button("Note => Slice").clicked() {
                        handle.settings.key_mode = KeyMode::Pitch;
                    }

                    ui.horizontal(|ui| {
                        ui.label(format!("{} slices", num_slices));
                        if ui.button("-").clicked() {
                            handle.settings.key_mode = KeyMode::Slice(num_slices - 1);
                        }
                        if ui.button("+").clicked() {
                            handle.settings.key_mode = KeyMode::Slice(num_slices + 1);
                        }
                    });
                }
            }

            ui.columns(3, |cols| {
                cols[0].vertical_centered(|ui| {
                    ui.columns(2, |cols| {
                        cols[0].add(
                            ParameterKnob::new(&mut handle.settings.amplitude, 0.0..=1.0)
                                .label("Level"),
                        );

                        cols[1].add(
                            ParameterKnob::new(&mut handle.settings.transpose, -12..=12)
                                .label("Transpose")
                                .suffix(" st"),
                        );
                    });
                });

                cols[1].vertical_centered(|ui| {
                    ui.columns(2, |cols| {
                        cols[0].add(
                            ParameterKnob::new(&mut handle.settings.position, 0.0..=1.0)
                                .label("Position"),
                        );

                        cols[1].add(
                            ParameterKnob::new(&mut handle.settings.spray_ms, 0.0..=1000.0)
                                .logarithmic(true)
                                .smallest_positive(1.0)
                                .label("Spray")
                                .suffix(" ms"),
                        );
                    });

                    ui.columns(2, |cols| {
                        cols[0].add(
                            ParameterKnob::new(&mut handle.settings.length_ms, 1.0..=1000.0)
                                .logarithmic(true)
                                .label("Length")
                                .suffix(" ms"),
                        );

                        cols[1].add(
                            ParameterKnob::new(&mut handle.settings.density, 1.0..=100.0)
                                .logarithmic(true)
                                .label("Density")
                                .suffix(" Hz"),
                        );
                    });
                });

                cols[2].vertical_centered(|ui| {
                    ui.label("Envelope");
                    ui.add(EnvelopePlot::new(&handle.settings.grain_envelope));
                    ui.columns(2, |cols| {
                        cols[0].add(
                            ParameterKnob::new(
                                &mut handle.settings.grain_envelope.amount,
                                0.0..=1.0,
                            )
                            .label("Amount"),
                        );

                        cols[1].add(
                            ParameterKnob::new(
                                &mut handle.settings.grain_envelope.skew,
                                -1.0..=1.0,
                            )
                            .label("Skew"),
                        );
                    });
                });
            });

            ui.columns(4, |cols| {
                cols[0].add(
                    ParameterKnob::new(&mut handle.settings.note_envelope.attack_ms, 0.0..=10000.0)
                        .logarithmic(true)
                        .smallest_positive(1.0)
                        .label("Attack")
                        .suffix(" ms"),
                );
                cols[1].add(
                    ParameterKnob::new(&mut handle.settings.note_envelope.decay_ms, 0.0..=10000.0)
                        .logarithmic(true)
                        .smallest_positive(1.0)
                        .label("Decay")
                        .suffix(" ms"),
                );
                cols[2].add(
                    ParameterKnob::new(&mut handle.settings.note_envelope.sustain_level, 0.0..=1.0)
                        .label("Sustain"),
                );
                cols[3].add(
                    ParameterKnob::new(
                        &mut handle.settings.note_envelope.release_ms,
                        0.0..=10000.0,
                    )
                    .logarithmic(true)
                    .smallest_positive(1.0)
                    .label("Release")
                    .suffix(" ms"),
                );
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
                    .button(app.midi_config.midi_in.port_name(port).unwrap())
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
