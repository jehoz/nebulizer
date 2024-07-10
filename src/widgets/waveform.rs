use std::time::Duration;

use eframe::{
    egui::{pos2, vec2, Color32, Frame, Rect, Rounding, Stroke, Ui, Widget},
    emath, epaint,
};
use rodio::cpal::FromSample;
use rodio::{cpal::Sample as CpalSample, Sample};

use crate::audio_clip::AudioClip;

const WAVEFORM_RESOLUTION: usize = 216;

#[derive(Clone)]
pub struct WaveformData {
    points: Box<[f32]>,
    clip_duration: Duration,
}

impl WaveformData {
    pub fn new<I>(clip: AudioClip<I>) -> Self
    where
        I: Sample,
        f32: FromSample<I>,
    {
        let bin_size = clip.data.len() / WAVEFORM_RESOLUTION;

        let mut points: [f32; WAVEFORM_RESOLUTION] = [0.0; WAVEFORM_RESOLUTION];
        for i in 0..WAVEFORM_RESOLUTION {
            let mut max = 0.0;
            for j in 0..bin_size {
                let val = f32::from_sample(clip.data[j + i * bin_size]).abs();
                if val > max {
                    max = val;
                }
            }
            points[i] = max;
        }

        Self {
            points: Box::new(points),
            clip_duration: clip.total_duration(),
        }
    }
}

pub struct Waveform {
    data: WaveformData,
    playheads: Vec<f32>,
    grain_length: Duration,
}

impl Waveform {
    pub fn new(data: WaveformData) -> Self {
        Self {
            data,
            playheads: Vec::new(),
            grain_length: Duration::ZERO,
        }
    }

    pub fn playheads(mut self, positions: Vec<f32>) -> Self {
        self.playheads = positions;
        self
    }

    pub fn grain_length(mut self, grain_length: Duration) -> Self {
        self.grain_length = grain_length;
        self
    }
}

impl Widget for Waveform {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        Frame::none()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(Stroke::new(1.0, ui.visuals().faint_bg_color))
            .show(ui, |ui| {
                let waveform_color = ui.visuals().text_color();
                let playhead_color = ui.visuals().selection.bg_fill;

                let desired_size = ui.available_width() * vec2(1.0, 0.35);
                let (_id, rect) = ui.allocate_space(desired_size);

                let bar_width = rect.width() / WAVEFORM_RESOLUTION as f32;

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 1.0..=-1.0),
                    rect.shrink(1.0),
                );

                let mut shapes = vec![];

                // draw playhead beginnings opaque behind waveform
                for position in self.playheads.iter() {
                    shapes.push(epaint::Shape::line_segment(
                        [
                            to_screen * pos2(*position, 1.0),
                            to_screen * pos2(*position, -1.0),
                        ],
                        Stroke::new(1.0, playhead_color.to_opaque()),
                    ));
                }

                // draw waveform
                let n = self.data.points.len();
                for i in 0..n {
                    let x = (i as f32) / (n as f32);
                    let y = self.data.points[i];
                    let p1 = to_screen * pos2(x, y);
                    let p2 = to_screen * pos2(x, -y);
                    shapes.push(epaint::Shape::line_segment(
                        [p1, p2],
                        Stroke::new(bar_width, waveform_color),
                    ));
                }

                // draw boxes extending from playheads on top of waveform
                for position in self.playheads.iter() {
                    if self.grain_length > Duration::ZERO {
                        let length_relative =
                            self.grain_length.as_secs_f32() / self.data.clip_duration.as_secs_f32();
                        let end = (position + length_relative).min(1.0);
                        shapes.push(epaint::Shape::rect_filled(
                            Rect::from_min_max(
                                to_screen * pos2(*position, 1.0),
                                to_screen * pos2(end, -1.0),
                            ),
                            Rounding::ZERO,
                            playhead_color,
                        ));
                    }
                }
                ui.painter().extend(shapes)
            })
            .response
    }
}
