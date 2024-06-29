use eframe::{
    egui::{pos2, vec2, Color32, Frame, Rect, Rounding, Stroke, Ui, Vec2},
    emath, epaint,
};
use rodio::cpal::Sample as CpalSample;
use rodio::Source;

use crate::audio_clip::AudioClip;

const WAVEFORM_RESOLUTION: u32 = 256;

pub struct WaveformData {
    points: Vec<f32>,
}

impl WaveformData {
    pub fn new(clip: AudioClip) -> WaveformData {
        let mut points: Vec<f32> = vec![];
        let bin_size = {
            let seconds = clip.total_duration().unwrap().as_secs_f32();
            let samples = seconds * clip.sample_rate() as f32 * clip.channels() as f32;
            samples as u32 / WAVEFORM_RESOLUTION
        };

        let mut clip = clip.clone().convert_samples::<f32>();
        let mut max = 0.0;
        for _ in 0..WAVEFORM_RESOLUTION {
            let mut acc: f32 = 0.0;
            for _ in 0..bin_size {
                acc += f32::from_sample(clip.next().unwrap()).abs();
            }
            let val = acc / bin_size as f32;
            if val > max {
                max = val;
            }
            points.push(val);
        }
        points = points.iter().map(|p| p / max).collect();

        WaveformData { points }
    }
}

pub fn waveform(ui: &mut Ui, data: &WaveformData, playhead_position: f32, playhead_range: f32) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let color = Color32::from_additive_luminance(196);
        let playhead_color = Color32::from_rgba_unmultiplied(88, 180, 237, 64);

        let desired_size = ui.available_width() * vec2(1.0, 0.35);
        let (_id, rect) = ui.allocate_space(desired_size);

        let to_screen =
            emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 1.0..=-1.0), rect);

        let mut shapes = vec![];

        // waveform
        let n = data.points.len();
        for i in 0..n {
            let x = (i as f32) / (n as f32);
            let y = data.points[i];
            let p1 = to_screen * pos2(x, y);
            let p2 = to_screen * pos2(x, -y);
            shapes.push(epaint::Shape::line_segment(
                [p1, p2],
                Stroke::new(1.0, color),
            ));
        }

        // playhead
        if playhead_range > 0.0 {
            let min = (playhead_position - playhead_range / 2.0).max(0.0);
            let max = (playhead_position + playhead_range / 2.0).min(1.0);
            shapes.push(epaint::Shape::rect_filled(
                Rect::from_min_max(to_screen * pos2(min, 1.0), to_screen * pos2(max, -1.0)),
                Rounding::ZERO,
                playhead_color,
            ));
        }

        shapes.push(epaint::Shape::line_segment(
            [
                to_screen * pos2(playhead_position, 1.0),
                to_screen * pos2(playhead_position, -1.0),
            ],
            Stroke::new(2.0, playhead_color.to_opaque()),
        ));
        ui.painter().extend(shapes)
    });
}
