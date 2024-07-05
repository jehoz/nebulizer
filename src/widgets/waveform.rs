use std::time::Duration;

use eframe::{
    egui::{pos2, vec2, Color32, Frame, Rect, Rounding, Stroke, Ui, Widget},
    emath, epaint,
};
use rodio::cpal::FromSample;
use rodio::{cpal::Sample as CpalSample, Sample};

use crate::audio_clip::AudioClip;

const WAVEFORM_RESOLUTION: usize = 256;

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
    playhead: Option<(f32, f32)>,
}

impl Waveform {
    pub fn new(data: WaveformData) -> Self {
        Self {
            data,
            playhead: None,
        }
    }

    pub fn playhead(self, position: f32, length_ms: f32) -> Self {
        Self {
            playhead: Some((position, length_ms)),
            ..self
        }
    }
}

impl Widget for Waveform {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        Frame::canvas(ui.style())
            .show(ui, |ui| {
                let color = Color32::from_additive_luminance(196);
                let playhead_color = Color32::from_rgba_unmultiplied(255, 183, 0, 64);

                let desired_size = ui.available_width() * vec2(1.0, 0.35);
                let (_id, rect) = ui.allocate_space(desired_size);

                let bar_width = rect.width() / WAVEFORM_RESOLUTION as f32;

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 1.0..=-1.0),
                    rect,
                );

                let mut shapes = vec![];

                // waveform
                let n = self.data.points.len();
                for i in 0..n {
                    let x = (i as f32) / (n as f32);
                    let y = self.data.points[i];
                    let p1 = to_screen * pos2(x, y);
                    let p2 = to_screen * pos2(x, -y);
                    shapes.push(epaint::Shape::line_segment(
                        [p1, p2],
                        Stroke::new(bar_width, color),
                    ));
                }

                // playhead
                if let Some((start, length_ms)) = self.playhead {
                    if length_ms > 0.0 {
                        let length_relative =
                            length_ms / (self.data.clip_duration.as_secs_f32() * 1000.0);
                        let end = (start + length_relative).min(1.0);
                        shapes.push(epaint::Shape::rect_filled(
                            Rect::from_min_max(
                                to_screen * pos2(start, 1.0),
                                to_screen * pos2(end, -1.0),
                            ),
                            Rounding::ZERO,
                            playhead_color,
                        ));
                    }

                    shapes.push(epaint::Shape::line_segment(
                        [to_screen * pos2(start, 1.0), to_screen * pos2(start, -1.0)],
                        Stroke::new(2.0, playhead_color.to_opaque()),
                    ));
                }
                ui.painter().extend(shapes)
            })
            .response
    }
}
