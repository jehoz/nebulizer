use std::{
    f32::consts::{PI, TAU},
    ops::RangeInclusive,
};

use eframe::{
    egui::{
        lerp, remap_clamp, Color32, DragValue, Pos2, Sense, Shape, Stroke, Ui, Vec2, Widget,
        WidgetText,
    },
    emath,
};
use midly::stream::Buffer;

const ARC_RESOLUTION: usize = 32;

/// Copied from egui slider source
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(get_set_value: &mut GetSetValue<'_>) -> f64 {
    (get_set_value)(None)
}

fn set(get_set_value: &mut GetSetValue<'_>, value: f64) {
    (get_set_value)(Some(value));
}

pub struct ParameterKnob<'a> {
    get_set_value: GetSetValue<'a>,
    range: RangeInclusive<f64>,
    diameter: f32,
    drag_speed: f32,
    label: Option<WidgetText>,
    suffix: Option<String>,
}

impl<'a> ParameterKnob<'a> {
    pub fn new<Num: emath::Numeric>(value: &'a mut Num, range: RangeInclusive<Num>) -> Self {
        let range_f64 = range.start().to_f64()..=range.end().to_f64();
        let s = Self::from_get_set(range_f64, move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });

        s
    }

    pub fn from_get_set(
        range: RangeInclusive<f64>,
        get_set_value: impl 'a + FnMut(Option<f64>) -> f64,
    ) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            range,
            diameter: 24.0,
            drag_speed: 0.01,
            label: None,
            suffix: None,
        }
    }

    #[inline]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[inline]
    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = Some(suffix.to_string());
        self
    }
}

impl<'a> Widget for ParameterKnob<'a> {
    fn ui(mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let desired_size = Vec2::splat(self.diameter);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if response.dragged() {
            let drag_delta = response.drag_delta();
            let delta = self.drag_speed
                * (drag_delta.x + drag_delta.y * -1.0)
                * (self.range.end() - self.range.start()) as f32;

            let new_value = (get(&mut self.get_set_value) + delta as f64)
                .clamp(*self.range.start(), *self.range.end());

            set(&mut self.get_set_value, new_value);
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let bg_color = Color32::from_rgb(96, 96, 96);
            let fg_color = Color32::from_rgb(80, 157, 239);

            let buffer_amt = 0.2;

            let min_angle = buffer_amt * PI;
            let max_angle = TAU - (buffer_amt * PI);
            draw_arc(
                ui,
                rect.center(),
                self.diameter / 2.0,
                Stroke::new(3.0, bg_color),
                min_angle,
                max_angle,
            );

            let value = get(&mut self.get_set_value);
            let value_range = (*self.range.start() as f32)..=(*self.range.end() as f32);
            let angle_range = min_angle..=max_angle;
            let value_angle = remap_clamp(value as f32, value_range.clone(), angle_range.clone());
            draw_arc(
                ui,
                rect.center(),
                self.diameter / 2.0,
                Stroke::new(2.0, fg_color),
                remap_clamp(0.0, value_range, angle_range),
                value_angle,
            );

            let tick = Shape::line_segment(
                [
                    rect.center(),
                    rect.center() + Vec2::angled(PI / 2.0 + value_angle) * (self.diameter / 2.0),
                ],
                Stroke::new(2.0, Color32::WHITE),
            );
            ui.painter().add(tick);
        }

        response
    }
}

fn draw_arc(
    ui: &mut Ui,
    center: Pos2,
    radius: f32,
    stroke: Stroke,
    start_angle: f32,
    end_angle: f32,
) {
    let mut points = vec![];
    for i in 0..ARC_RESOLUTION {
        let angle = lerp(start_angle..=end_angle, i as f32 / ARC_RESOLUTION as f32);
        let point = center + Vec2::angled(PI / 2.0 + angle) * radius;
        points.push(point);
    }
    ui.painter().add(Shape::line(points, stroke));
}
