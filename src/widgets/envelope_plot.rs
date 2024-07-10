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
        Frame::none()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(Stroke::new(1.0, ui.visuals().faint_bg_color))
            .show(ui, |ui| {
                let height = self.height.unwrap_or(1.0 * ui.available_width());
                let desired_size = vec2(ui.available_width(), height);
                let (_id, rect) = ui.allocate_space(desired_size);

                let inner_rect = rect.shrink2(vec2(2.0, 8.0));

                match self.envelope {
                    Envelope::Grain(env) => draw_grain_envelope(ui, env, inner_rect),
                    Envelope::Adsr(env) => draw_asdr_envelope(ui, env, inner_rect),
                }
            })
            .response
    }
}

fn draw_grain_envelope(ui: &mut Ui, envelope: &GrainEnvelope, rect: Rect) {
    let color = ui.visuals().text_color();
    let to_screen =
        emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 1.0..=0.0), rect);

    let n = 120;
    let points = (0..=n)
        .map(|i| {
            let x = i as f32 / (n as f32);
            let y = envelope.amplitude_at(x);

            to_screen * pos2(x, y)
        })
        .collect();

    let line = epaint::Shape::line(points, Stroke::new(2.0, color));
    ui.painter().add(line);
}

fn draw_asdr_envelope(ui: &mut Ui, envelope: &AdsrEnvelope, rect: Rect) {
    let color = ui.visuals().text_color();
    let to_screen =
        emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 1.0..=0.0), rect);

    let AdsrEnvelope {
        attack,
        decay,
        sustain_level,
        release,
    } = envelope;
    let total_sec = (*attack + *decay + *release).as_secs_f32();

    let points = vec![
        to_screen * pos2(0.0, 0.0),
        to_screen * pos2(attack.as_secs_f32() / total_sec, 1.0),
        to_screen * pos2((*attack + *decay).as_secs_f32() / total_sec, *sustain_level),
        to_screen * pos2(1.0, 0.0),
    ];

    for point in points.iter() {
        ui.painter().circle_filled(*point, 2.0, color);
    }

    let line = epaint::Shape::line(points, Stroke::new(2.0, color));
    ui.painter().add(line);
}
