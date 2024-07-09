use crate::envelope::{AdsrEnvelope, GrainEnvelope};

use eframe::{
    egui::{pos2, vec2, Frame, Rect, Ui, Widget},
    emath,
    epaint::{self, Stroke},
};

enum Envelope<'a> {
    Grain(&'a GrainEnvelope),
    Adsr(&'a AdsrEnvelope),
}

pub struct EnvelopePlot<'a> {
    envelope: Envelope<'a>,
    height: Option<f32>,
}

impl<'a> EnvelopePlot<'a> {
    pub fn from_grain_envelope(envelope: &'a GrainEnvelope) -> Self {
        Self {
            envelope: Envelope::Grain(envelope),
            height: None,
        }
    }

    pub fn from_adsr_envelope(envelope: &'a AdsrEnvelope) -> Self {
        Self {
            envelope: Envelope::Adsr(envelope),
            height: None,
        }
    }

    pub fn set_height(mut self, height: f32) -> EnvelopePlot<'a> {
        self.height = Some(height);
        self
    }
}

impl<'a> Widget for EnvelopePlot<'a> {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        // Frame::canvas(ui.style())
        Frame::none()
            .show(ui, |ui| {
                let height = self.height.unwrap_or(1.0 * ui.available_width());
                let desired_size = vec2(ui.available_width(), height);
                let (_id, rect) = ui.allocate_space(desired_size);

                match self.envelope {
                    Envelope::Grain(env) => draw_grain_envelope(ui, env, rect),
                    Envelope::Adsr(env) => draw_asdr_envelope(ui, env, rect),
                }
            })
            .response
    }
}

fn draw_grain_envelope(ui: &mut Ui, envelope: &GrainEnvelope, rect: Rect) {
    let color = ui.visuals().text_color();
    let to_screen =
        emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 1.25..=-0.25), rect);

    let n = 120;
    let points = (0..=n)
        .map(|i| {
            let x = i as f32 / (n as f32);
            let y = envelope.amplitude_at(x);

            to_screen * pos2(x, y)
        })
        .collect();

    let line = epaint::Shape::line(points, Stroke::new(1.0, color));
    ui.painter().add(line);
}

fn draw_asdr_envelope(ui: &mut Ui, envelope: &AdsrEnvelope, rect: Rect) {
    let color = ui.visuals().text_color();
    let to_screen =
        emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 1.25..=-0.25), rect);

    let AdsrEnvelope {
        attack_ms,
        decay_ms,
        sustain_level,
        release_ms,
    } = envelope;
    let total_ms = attack_ms + decay_ms + release_ms;

    let points = vec![
        to_screen * pos2(0.0, 0.0),
        to_screen * pos2(attack_ms / total_ms, 1.0),
        to_screen * pos2((attack_ms + decay_ms) / total_ms, *sustain_level),
        to_screen * pos2(1.0, 0.0),
    ];

    let line = epaint::Shape::line(points, Stroke::new(1.0, color));
    ui.painter().add(line);
}
