use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    time::Duration,
};

use eframe::egui::{self, vec2, Color32, DragValue, Frame, Stroke, Ui};
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
    pub settings: EmitterSettings,
    pub waveform: Option<WaveformData>,
    pub msg_sender: Option<Sender<EmitterMessage>>,
}

impl Default for EmitterHandle {
    fn default() -> Self {
        Self {
            track_name: "".to_string(),
            settings: EmitterSettings::default(),
            waveform: None,
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
                let emitter: Emitter<f32> = Emitter::new(&clip, rx);
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

    let playheads = match handle.settings.key_mode {
        KeyMode::Pitch => {
            vec![handle.settings.position]
        }
        KeyMode::Slice => (0..handle.settings.num_slices)
            .map(|i| i as f32 / handle.settings.num_slices as f32)
            .collect(),
    };

    let waveform_size = ui.available_width() * vec2(1.0, 0.25);
    if let Some(waveform) = &handle.waveform {
        ui.add(
            Waveform::new(waveform.clone())
                .playheads(playheads)
                .grain_length(handle.settings.length)
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
        ui.add(DragValue::new(&mut handle.settings.polyphony).clamp_range(1..=64));

        ui.separator();

        ui.label("Transpose");
        ui.add(
            DragValue::new(&mut handle.settings.transpose)
                .clamp_range(-12..=12)
                .suffix(" st"),
        );
    });

    ui.separator();

    ui.columns(6, |cols| {
        cols[0].vertical_centered_justified(|ui| {
            ui.selectable_value(&mut handle.settings.key_mode, KeyMode::Pitch, "Pitch");
            ui.selectable_value(&mut handle.settings.key_mode, KeyMode::Slice, "Slice");
        });

        match handle.settings.key_mode {
            KeyMode::Pitch => {
                cols[1].add(
                    ParameterKnob::new(&mut handle.settings.position, 0.0..=1.0)
                        .max_decimals(2)
                        .label("Position"),
                );
            }
            KeyMode::Slice => {
                cols[1].add(
                    ParameterKnob::new(&mut handle.settings.num_slices, 1..=127).label("Slices"),
                );
            }
        }
        cols[2].add(
            ParameterKnob::new(
                &mut handle.settings.spray,
                Duration::ZERO..=Duration::from_secs(1),
            )
            .logarithmic(true)
            .label("Spray"),
        );
        cols[3].add(
            ParameterKnob::new(
                &mut handle.settings.length,
                Duration::ZERO..=Duration::from_secs(1),
            )
            .logarithmic(true)
            .label("Length"),
        );
        cols[4].add(
            ParameterKnob::new(&mut handle.settings.density, 1.0..=100.0)
                .logarithmic(true)
                .max_decimals(2)
                .label("Density")
                .suffix(" Hz"),
        );

        cols[5].add(
            ParameterKnob::new(&mut handle.settings.amplitude, 0.0..=1.0)
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
                EnvelopePlot::from_adsr_envelope(&handle.settings.note_envelope)
                    .set_height(plot_height),
            );
            ui.columns(4, |cols| {
                cols[0].add(
                    ParameterKnob::new(
                        &mut handle.settings.note_envelope.attack,
                        Duration::ZERO..=Duration::from_secs(10),
                    )
                    .logarithmic(true)
                    .label("Attack"),
                );
                cols[1].add(
                    ParameterKnob::new(
                        &mut handle.settings.note_envelope.decay,
                        Duration::ZERO..=Duration::from_secs(10),
                    )
                    .logarithmic(true)
                    .label("Decay"),
                );
                cols[2].add(
                    ParameterKnob::new(&mut handle.settings.note_envelope.sustain_level, 0.0..=1.0)
                        .max_decimals(2)
                        .label("Sustain"),
                );
                cols[3].add(
                    ParameterKnob::new(
                        &mut handle.settings.note_envelope.release,
                        Duration::ZERO..=Duration::from_secs(10),
                    )
                    .logarithmic(true)
                    .label("Release"),
                );
            });
        });

        ui.separator();

        ui.vertical(|ui| {
            ui.set_width(right_width);
            ui.add(
                EnvelopePlot::from_grain_envelope(&handle.settings.grain_envelope)
                    .set_height(plot_height),
            );
            ui.columns(2, |cols| {
                cols[0].add(
                    ParameterKnob::new(&mut handle.settings.grain_envelope.amount, 0.0..=1.0)
                        .max_decimals(2)
                        .label("Amount"),
                );

                cols[1].add(
                    ParameterKnob::new(&mut handle.settings.grain_envelope.skew, -1.0..=1.0)
                        .max_decimals(2)
                        .label("Skew"),
                );
            });
        });
    });

    if let Some(sender) = &handle.msg_sender {
        let _ = sender.send(EmitterMessage::Settings(handle.settings.clone()));
    }
}

fn settings_panel(app: &mut NebulizerApp, ui: &mut Ui) {
    ui.label("MIDI Connection");
    match &app.midi_config.connection {
        Some((name, _conn)) => {
            ui.label(format!("Connected to MIDI port: {}", name));
            if ui.button("Disconnect").clicked() {
                app.midi_config.connection = None;
            }
        }
        None => {
            if ui.button("Refresh").clicked() {
                app.midi_config.refresh_ports();
            }

            ui.label("Click one to connect:");
            for port in app.midi_config.ports.clone().iter() {
                if ui
                    .button(app.midi_config.midi_in.port_name(port).unwrap())
                    .clicked()
                {
                    let handle = app.emitter.clone();
                    let app_channel = app.midi_channel.clone();
                    app.midi_config.connect(port, move |channel, message| {
                        if channel == *app_channel.lock().unwrap() {
                            if let Some(sender) = &handle.lock().unwrap().msg_sender {
                                let _ = sender.send(EmitterMessage::Midi(message)).unwrap();
                            }
                        }
                    });
                }
            }
        }
    }
    ui.separator();

    ui.label("MIDI Channel");
    let mut channel = app.midi_channel.lock().unwrap();
    let mut ui_channel: u4 = channel.clone();
    egui::ComboBox::from_label("MIDI Channel")
        .selected_text(channel.to_string())
        .show_ui(ui, |ui| {
            for i in 0..=15 {
                let chan = u4::from(i);
                ui.selectable_value(&mut ui_channel, chan, chan.to_string());
            }
        });
    *channel = ui_channel;
}
