use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};

use eframe::{
    egui::{self, vec2, Color32, ComboBox, DragValue, Frame, Stroke, Ui},
    emath::Numeric,
};
use midly::{
    num::{u4, u7},
    MidiMessage,
};
use rodio::{OutputStream, OutputStreamHandle, Source};
use strum::VariantArray;

use crate::{
    audio_clip::AudioClip,
    emitter::{Emitter, EmitterMessage},
    midi::MidiConfig,
    params::{ControlParam, EmitterParams, KeyMode},
    widgets::{
        envelope_plot::EnvelopePlot,
        parameter_knob::ParameterKnob,
        waveform::{GrainDrawData, Waveform, WaveformData},
    },
};

pub struct EmitterHandle {
    pub track_name: String,
    pub params: EmitterParams,
    pub waveform: Option<WaveformData>,
    pub grain_draw_data: Arc<Mutex<Vec<GrainDrawData>>>,
    pub msg_sender: Option<Sender<EmitterMessage>>,
}

impl Default for EmitterHandle {
    fn default() -> Self {
        Self {
            track_name: "".to_string(),
            params: EmitterParams::default(),
            waveform: None,
            grain_draw_data: Arc::new(Mutex::new(Vec::new())),
            msg_sender: None,
        }
    }
}

pub struct NebulizerApp {
    stream: (OutputStream, OutputStreamHandle),

    midi_config: MidiConfig,

    midi_channel: Arc<Mutex<u4>>,

    active_panel: GuiPanel,

    emitter: Arc<Mutex<EmitterHandle>>,

    theme: catppuccin_egui::Theme,
}

impl NebulizerApp {
    pub fn new() -> NebulizerApp {
        // setup audio stream
        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        NebulizerApp {
            stream: (stream, stream_handle),
            midi_config: MidiConfig::new(),
            midi_channel: Arc::new(Mutex::new(u4::from(0))),
            active_panel: GuiPanel::Emitters,
            emitter: Arc::new(Mutex::new(EmitterHandle::default())),
            theme: catppuccin_egui::LATTE,
        }
    }
}

fn handle_midi_msg(emitter: Arc<Mutex<EmitterHandle>>, message: MidiMessage) {
    let handle = &mut emitter.lock().unwrap();
    if let Some(msg_sender) = &handle.msg_sender.clone() {
        match message {
            MidiMessage::NoteOn { key, vel } => {
                let _ = msg_sender.send(EmitterMessage::NoteOn { key, vel });
            }
            MidiMessage::NoteOff { key, vel } => {
                let _ = msg_sender.send(EmitterMessage::NoteOff { key, vel });
            }
            MidiMessage::Controller { controller, value } => {
                let cc_map = handle.params.midi_cc_map.clone();
                let norm_value = value.as_int() as f64 / 127.0;
                for (cc, param) in cc_map.iter() {
                    if *cc == controller {
                        match param {
                            ControlParam::Position => {
                                handle.params.position.set_normalized(norm_value)
                            }
                            ControlParam::NumSlices => {
                                handle.params.num_slices.set_normalized(norm_value)
                            }
                            ControlParam::Spray => handle.params.spray.set_normalized(norm_value),
                            ControlParam::Length => handle.params.length.set_normalized(norm_value),
                            ControlParam::Density => {
                                handle.params.density.set_normalized(norm_value)
                            }
                            ControlParam::GrainEnvelopeAmount => handle
                                .params
                                .grain_envelope
                                .amount
                                .set_normalized(norm_value),
                            ControlParam::GrainEnvelopeSkew => {
                                handle.params.grain_envelope.skew.set_normalized(norm_value)
                            }
                            ControlParam::NoteEnvelopeAttack => handle
                                .params
                                .note_envelope
                                .attack
                                .set_normalized(norm_value),
                            ControlParam::NoteEnvelopeDecay => {
                                handle.params.note_envelope.decay.set_normalized(norm_value)
                            }
                            ControlParam::NoteEnvelopeSustain => handle
                                .params
                                .note_envelope
                                .sustain_level
                                .set_normalized(norm_value),
                            ControlParam::NoteEnvelopeRelease => handle
                                .params
                                .note_envelope
                                .release
                                .set_normalized(norm_value),
                            ControlParam::Transpose => {
                                handle.params.transpose.set_normalized(norm_value)
                            }
                            ControlParam::Amplitude => {
                                handle.params.amplitude.set_normalized(norm_value)
                            }
                        }
                    }
                }
                let _ = msg_sender.send(EmitterMessage::Params(handle.params.clone()));
            }
            _ => {}
        }
    }
}

enum GuiPanel {
    Emitters,
    Settings,
}

impl eframe::App for NebulizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        catppuccin_egui::set_theme(ctx, self.theme);

        egui::TopBottomPanel::top("menu bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Emitters").clicked() {
                    self.active_panel = GuiPanel::Emitters;
                }

                if ui.button("Settings").clicked() {
                    self.active_panel = GuiPanel::Settings;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.active_panel {
            GuiPanel::Emitters => emitters_panel(self, ui),
            GuiPanel::Settings => settings_panel(self, ui),
        });

        ctx.request_repaint();
    }
}

fn emitters_panel(app: &mut NebulizerApp, ui: &mut Ui) {
    let mut handle = app.emitter.lock().unwrap();

    if ui.button("Load new sample").clicked() {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            // attempt to load and decode audio file
            if let Some(clip) = AudioClip::<f32>::load_from_file(path.display().to_string()) {
                // if overwriting existing emitter, terminate it first
                if let Some(sender) = &handle.msg_sender {
                    let _ = sender.send(EmitterMessage::Terminate).unwrap();
                }

                let (tx, rx) = mpsc::channel();
                let emitter: Emitter<f32> = Emitter::new(&clip, rx, handle.grain_draw_data.clone());
                handle.track_name = path.file_name().unwrap().to_str().unwrap().to_string();
                handle.waveform = Some(WaveformData::new(clip));
                handle.msg_sender = Some(tx);
                let _ = app.stream.1.play_raw(emitter.convert_samples());
            } else {
                // TODO make some error popup window since this is only visible for one frame
                ui.colored_label(Color32::RED, "Failed to read/decode audio file!");
            }
        }
    }

    ui.separator();

    ui.horizontal(|ui| {
        ui.monospace(&handle.track_name);
    });

    let playheads = match handle.params.key_mode {
        KeyMode::Pitch => {
            vec![handle.params.position.get()]
        }
        KeyMode::Slice => {
            let slices = handle.params.num_slices.get();
            (0..slices).map(|i| i as f32 / slices as f32).collect()
        }
    };

    let waveform_size = ui.available_width() * vec2(1.0, 0.25);
    if let Some(waveform) = &handle.waveform {
        let draw_grains = handle.grain_draw_data.lock().unwrap().drain(..).collect();
        ui.add(
            Waveform::new(waveform.clone(), draw_grains)
                .playheads(playheads)
                .grain_length(handle.params.length.get())
                .desired_size(waveform_size),
        );
    } else {
        Frame::none()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(Stroke::new(1.0, ui.visuals().faint_bg_color))
            .show(ui, |ui| {
                let _ = ui.allocate_space(waveform_size);
                ui.label("Load a sample :)")
            });
    }

    ui.horizontal(|ui| {
        ui.label("Polyphony");
        ui.add(DragValue::new(&mut handle.params.polyphony).clamp_range(1..=64));

        ui.separator();

        ui.label("Transpose");
        let transpose_param = &mut handle.params.transpose;
        let transpose_range = transpose_param.range();
        ui.add(
            DragValue::from_get_set(|new_val| {
                if let Some(v) = new_val {
                    transpose_param.set(i32::from_f64(v));
                }
                transpose_param.get().to_f64()
            })
            .clamp_range(transpose_range)
            .suffix(" st"),
        );
    });

    ui.separator();

    ui.columns(6, |cols| {
        cols[0].vertical_centered_justified(|ui| {
            ui.selectable_value(&mut handle.params.key_mode, KeyMode::Pitch, "Pitch");
            ui.selectable_value(&mut handle.params.key_mode, KeyMode::Slice, "Slice");
        });

        match handle.params.key_mode {
            KeyMode::Pitch => {
                cols[1].add(
                    ParameterKnob::from_param(&mut handle.params.position)
                        .max_decimals(2)
                        .label("Position"),
                );
            }
            KeyMode::Slice => {
                cols[1]
                    .add(ParameterKnob::from_param(&mut handle.params.num_slices).label("Slices"));
            }
        }
        cols[2].add(ParameterKnob::from_param(&mut handle.params.spray).label("Spray"));
        cols[3].add(ParameterKnob::from_param(&mut handle.params.length).label("Length"));
        cols[4].add(
            ParameterKnob::from_param(&mut handle.params.density)
                .max_decimals(2)
                .label("Density")
                .suffix(" Hz"),
        );

        cols[5].add(
            ParameterKnob::from_param(&mut handle.params.amplitude)
                .max_decimals(2)
                .label("Level"),
        );
    });

    ui.separator();

    let plot_height = ui.available_width() / 6.0;
    let (left_width, right_width) = {
        let spacing = ui.spacing();
        let item_space = spacing.item_spacing.x;
        let margin = spacing.window_margin.left + spacing.window_margin.right;
        let width = ui.available_width() - (margin + item_space);
        (width * 0.67, width * 0.33)
    };

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(left_width);
            ui.add(
                EnvelopePlot::from_adsr_envelope(&handle.params.note_envelope)
                    .set_height(plot_height),
            );
            ui.columns(4, |cols| {
                cols[0].add(
                    ParameterKnob::from_param(&mut handle.params.note_envelope.attack)
                        .label("Attack"),
                );
                cols[1].add(
                    ParameterKnob::from_param(&mut handle.params.note_envelope.decay)
                        .label("Decay"),
                );
                cols[2].add(
                    ParameterKnob::from_param(&mut handle.params.note_envelope.sustain_level)
                        .max_decimals(2)
                        .label("Sustain"),
                );
                cols[3].add(
                    ParameterKnob::from_param(&mut handle.params.note_envelope.release)
                        .label("Release"),
                );
            });
        });

        ui.separator();

        ui.vertical(|ui| {
            ui.set_width(right_width);
            ui.add(
                EnvelopePlot::from_grain_envelope(&handle.params.grain_envelope)
                    .set_height(plot_height),
            );
            ui.columns(2, |cols| {
                cols[0].add(
                    ParameterKnob::from_param(&mut handle.params.grain_envelope.amount)
                        .max_decimals(2)
                        .label("Amount"),
                );

                cols[1].add(
                    ParameterKnob::from_param(&mut handle.params.grain_envelope.skew)
                        .max_decimals(2)
                        .label("Skew"),
                );
            });
        });
    });

    if let Some(sender) = &handle.msg_sender {
        let _ = sender.send(EmitterMessage::Params(handle.params.clone()));
    }
}

fn settings_panel(app: &mut NebulizerApp, ui: &mut Ui) {
    ui.label("MIDI Connection");
    match &app.midi_config.connection {
        Some((name, _conn)) => {
            let mut disconnect_clicked = false;
            ui.horizontal(|ui| {
                ui.label(name);
                disconnect_clicked = ui.button("Disconnect").clicked();
            });
            if disconnect_clicked {
                app.midi_config.connection = None;
            }
        }
        None => {
            ui.horizontal(|ui| {
                if ui.button("Refresh").clicked() {
                    app.midi_config.refresh_ports();
                }
            });

            for port in app.midi_config.ports.clone().iter() {
                ui.horizontal(|ui| {
                    ui.label(app.midi_config.midi_in.port_name(port).unwrap());

                    if ui.button("Connect").clicked() {
                        let handle = app.emitter.clone();
                        let midi_channel = app.midi_channel.clone();
                        app.midi_config.connect(port, move |channel, message| {
                            if channel == *midi_channel.lock().unwrap() {
                                handle_midi_msg(handle.clone(), message);
                            }
                        });
                    }
                });
            }
        }
    }

    ui.separator();

    ui.label("MIDI Channel");
    let mut channel = app.midi_channel.lock().unwrap();
    let mut selected_channel: u4 = channel.clone();
    ComboBox::from_label("")
        .selected_text(channel.to_string())
        .show_ui(ui, |ui| {
            for i in 0..=15 {
                let chan = u4::from(i);
                ui.selectable_value(&mut selected_channel, chan, chan.to_string());
            }
        });
    *channel = selected_channel;

    ui.separator();
    ui.label("MIDI CC");
    let mut handle = app.emitter.lock().unwrap();
    let mut to_delete = None;
    for (e, (cc, param)) in handle.params.midi_cc_map.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ComboBox::from_id_source(format!("cc-{e}"))
                .selected_text(format!("CC {}", cc))
                .show_ui(ui, |ui| {
                    for i in 0u8..=127 {
                        ui.selectable_value(cc, u7::from(i), format!("CC {}", i));
                    }
                });

            ComboBox::from_id_source(format!("param-{e}"))
                .selected_text(param.to_string())
                .show_ui(ui, |ui| {
                    for p in ControlParam::VARIANTS {
                        ui.selectable_value(param, p.clone(), p.to_string());
                    }
                });

            if ui.button("X").clicked() {
                to_delete = Some(e);
            }
        });
    }

    if let Some(idx) = to_delete {
        let _ = handle.params.midi_cc_map.remove(idx);
    }

    if ui.button("+").clicked() {
        handle
            .params
            .midi_cc_map
            .push((0.into(), ControlParam::Position));
    }
}
