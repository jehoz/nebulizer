use std::{
    f32::consts::{PI, TAU},
    ops::RangeInclusive,
};

use eframe::egui::{
    lerp, remap_clamp, Color32, DragValue, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2, Widget,
    WidgetText,
};

use crate::numeric::Numeric;

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
    drag_speed: f64,
    logarithmic: bool,
    smallest_positive: f64,
    label: Option<WidgetText>,
    suffix: Option<String>,

    is_duration: bool,
}

impl<'a> ParameterKnob<'a> {
    pub fn new<Num: Numeric>(value: &'a mut Num, range: RangeInclusive<Num>) -> Self {
        let range_f64 = range.start().to_f64()..=range.end().to_f64();
        let mut knob = Self::from_get_set(range_f64, move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });

        if Num::DURATION {
            knob.is_duration = true;
            knob.smallest_positive = 0.001;
        }

        knob
    }

    pub fn from_get_set(
        range: RangeInclusive<f64>,
        get_set_value: impl 'a + FnMut(Option<f64>) -> f64,
    ) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            range,
            diameter: 24.0,
            drag_speed: 0.002,
            logarithmic: false,
            smallest_positive: 1e-6,
            label: None,
            suffix: None,
            is_duration: false,
        }
    }

    #[inline]
    pub fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.logarithmic = logarithmic;
        self
    }

    #[inline]
    pub fn smallest_positive(mut self, smallest_positive: f64) -> Self {
        self.smallest_positive = smallest_positive;
        self
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

impl<'a> ParameterKnob<'a> {
    fn allocate_knob_space(&self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::splat(self.diameter);
        ui.allocate_response(desired_size, Sense::click_and_drag())
    }

    /// Just draw the knob widget itself
    fn knob_ui(&mut self, ui: &Ui, response: &mut Response) {
        if response.dragged() {
            let drag_delta = response.drag_delta();
            let delta = (self.drag_speed) * (drag_delta.x + drag_delta.y * -1.0) as f64;

            let val = get(&mut self.get_set_value);
            let new_pos = self.value_to_position(val, self.range.clone());
            let new_val = self.position_to_value(new_pos + delta, self.range.clone());
            set(&mut self.get_set_value, new_val);
            response.mark_changed();
        }

        let rect = response.rect;
        if ui.is_rect_visible(rect) {
            let bg_color = Color32::from_rgb(96, 96, 96);
            let fg_color = Color32::from_rgb(80, 157, 239);

            let rot_padding = 0.2;
            let radius = self.diameter / 2.0;

            let min_angle = rot_padding * PI;
            let max_angle = TAU - (rot_padding * PI);
            draw_arc(
                ui,
                rect.center(),
                radius,
                Stroke::new(3.0, bg_color),
                min_angle,
                max_angle,
            );

            let value = get(&mut self.get_set_value);
            let normalized = self.value_to_position(value, self.range.clone());
            let angle_range = min_angle..=max_angle;
            let value_angle = remap_clamp(normalized as f32, 0.0..=1.0, angle_range.clone());
            draw_arc(
                ui,
                rect.center(),
                radius,
                Stroke::new(2.0, fg_color),
                angle_range.start().clone(),
                value_angle,
            );

            let tick = Shape::line_segment(
                [
                    rect.center(),
                    rect.center() + Vec2::angled(PI / 2.0 + value_angle) * radius,
                ],
                Stroke::new(2.0, Color32::WHITE),
            );
            ui.painter().add(tick);
        }
    }

    /// computes the normalized knob position [0.0, 1.0] from value in range
    fn value_to_position(&self, value: f64, range: RangeInclusive<f64>) -> f64 {
        let (min, max) = (*range.start(), *range.end());

        if min == max {
            0.5
        } else if min > max {
            1.0 - self.value_to_position(value, max..=min)
        } else if value <= min {
            0.0
        } else if value >= max {
            1.0
        } else if self.logarithmic {
            assert!(
                min >= 0.0 && max > min,
                "Logarithmic scale only implemented for positive ranges right now"
            );

            let min_log = if min <= self.smallest_positive {
                self.smallest_positive.log10()
            } else {
                min.log10()
            };
            let max_log = max.log10();

            remap_clamp(value.log10(), min_log..=max_log, 0.0..=1.0)
        } else {
            remap_clamp(value, range, 0.0..=1.0)
        }
    }

    /// computes the appropriate value in range from the normalized knob position
    fn position_to_value(&self, position: f64, range: RangeInclusive<f64>) -> f64 {
        let (min, max) = (*range.start(), *range.end());

        if min == max {
            min
        } else if min > max {
            self.position_to_value(1.0 - position, max..=min)
        } else if position <= 0.0 {
            min
        } else if position >= 1.0 {
            max
        } else if self.logarithmic {
            assert!(
                min >= 0.0 && max > min,
                "Logarithmic scale only implemented for positive ranges right now"
            );
            let min_log = if min <= self.smallest_positive {
                self.smallest_positive.log10()
            } else {
                min.log10()
            };
            let max_log = max.log10();

            let log = lerp(min_log..=max_log, position);
            10.0_f64.powf(log)
        } else {
            lerp(range, position.clamp(0.0, 1.0))
        }
    }
}

impl<'a> Widget for ParameterKnob<'a> {
    fn ui(mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.vertical_centered(move |ui| {
            if let Some(label) = &self.label {
                ui.label(label.clone());
            }

            let mut response = self.allocate_knob_space(ui);
            self.knob_ui(ui, &mut response);

            let mut drag_val = DragValue::from_get_set(self.get_set_value)
                .clamp_range(self.range.clone())
                .speed(self.drag_speed * (self.range.end() - self.range.start()));
            if self.is_duration {
                drag_val = drag_val.custom_formatter(|n, _| {
                    let ms = n * 1000.0;
                    if ms < 100.0 {
                        format!("{ms:.1} ms")
                    } else if ms < 1000.0 {
                        format!("{ms:.0} ms")
                    } else {
                        format!("{n:.2} s")
                    }
                });
            }
            if let Some(suffix) = &self.suffix {
                drag_val = drag_val.suffix(suffix);
            }
            ui.add(drag_val);
        })
        .response
    }
}

fn draw_arc(ui: &Ui, center: Pos2, radius: f32, stroke: Stroke, start_angle: f32, end_angle: f32) {
    let mut points = vec![];
    for i in 0..ARC_RESOLUTION {
        let angle = lerp(start_angle..=end_angle, i as f32 / ARC_RESOLUTION as f32);
        let point = center + Vec2::angled(PI / 2.0 + angle) * radius;
        points.push(point);
    }
    ui.painter().add(Shape::line(points, stroke));
}
