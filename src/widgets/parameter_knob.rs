use std::f32::consts::{PI, TAU};

use eframe::egui::{
    lerp, remap_clamp, Color32, DragValue, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2, Widget,
    WidgetText,
};

use crate::{numeric::Numeric, params::Parameter};

const ARC_RESOLUTION: usize = 32;

pub struct ParameterKnob<'a, I> {
    param: &'a mut Parameter<I>,

    diameter: f32,
    drag_speed: f64,
    min_decimals: usize,
    max_decimals: Option<usize>,
    label: Option<WidgetText>,
    suffix: Option<String>,
    fill: Option<Color32>,

    is_duration: bool,
}

impl<'a, I> ParameterKnob<'a, I>
where
    I: Numeric,
{
    pub fn from_param(param: &'a mut Parameter<I>) -> Self {
        let mut knob = Self {
            param,
            diameter: 24.0,
            drag_speed: 0.002,
            min_decimals: 0,
            max_decimals: None,
            label: None,
            suffix: None,
            fill: None,
            is_duration: false,
        };

        if I::INTEGRAL {
            knob.max_decimals = Some(0);
        }

        if I::DURATION {
            knob.is_duration = true;
        }

        knob
    }

    #[inline]
    pub fn min_decimals(mut self, decimals: usize) -> Self {
        self.min_decimals = decimals;
        self
    }

    #[inline]
    pub fn max_decimals(mut self, decimals: usize) -> Self {
        self.max_decimals = Some(decimals);
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

    #[inline]
    pub fn fill(mut self, color: Color32) -> Self {
        self.fill = Some(color);
        self
    }
}

impl<'a, I> ParameterKnob<'a, I>
where
    I: Numeric,
{
    fn allocate_knob_space(&self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::splat(self.diameter);
        ui.allocate_response(desired_size, Sense::click_and_drag())
    }

    /// Just draw the knob widget itself
    fn knob_ui(&mut self, ui: &Ui, response: &mut Response) {
        if response.dragged() {
            let drag_delta = response.drag_delta();
            let delta = (self.drag_speed) * (drag_delta.x + drag_delta.y * -1.0) as f64;

            let norm_val = self.param.get_normalized();
            self.param
                .set_normalized((norm_val + delta).clamp(0.0, 1.0));
            response.mark_changed();
        }

        let rect = response.rect;
        if ui.is_rect_visible(rect) {
            let bg_color = ui.visuals().weak_text_color();
            let tick_color = ui.visuals().text_color();
            let fill_color = self.fill.unwrap_or(ui.visuals().selection.bg_fill);

            let rot_padding = 0.2;
            let radius = self.diameter / 2.0;

            let min_angle = rot_padding * PI;
            let max_angle = TAU - (rot_padding * PI);
            draw_arc(
                ui,
                rect.center(),
                radius,
                Stroke::new(2.0, bg_color),
                min_angle,
                max_angle,
            );

            let normalized = self.param.get_normalized();
            let angle_range = min_angle..=max_angle;
            let value_angle = remap_clamp(normalized as f32, 0.0..=1.0, angle_range.clone());
            draw_arc(
                ui,
                rect.center(),
                radius,
                Stroke::new(2.0, fill_color),
                angle_range.start().clone(),
                value_angle,
            );

            let tick = Shape::line_segment(
                [
                    rect.center(),
                    rect.center() + Vec2::angled(PI / 2.0 + value_angle) * radius,
                ],
                Stroke::new(2.0, tick_color),
            );
            ui.painter().add(tick);
        }
    }
}

impl<'a, I> Widget for ParameterKnob<'a, I>
where
    I: Numeric,
{
    fn ui(mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.vertical_centered(move |ui| {
            if let Some(label) = &self.label {
                ui.label(label.clone());
            }

            let mut response = self.allocate_knob_space(ui);
            self.knob_ui(ui, &mut response);

            let range = self.param.range();
            let range_f64 = range.start().to_f64()..=range.end().to_f64();
            // there's probably a less hacky way to make this work but this is fine for now
            let mut value = self.param.get();
            let mut drag_val = DragValue::from_get_set(|new_val: Option<f64>| {
                let val_ref = &mut value;
                if let Some(v) = new_val {
                    *val_ref = I::from_f64(v);
                }
                val_ref.to_f64()
            })
            .clamp_range(range_f64.clone())
            .min_decimals(self.min_decimals)
            .max_decimals_opt(self.max_decimals)
            .speed(self.drag_speed * (range_f64.end() - range_f64.start()));

            if self.is_duration {
                drag_val = drag_val
                    .custom_formatter(|n, _| {
                        let ms = n * 1000.0;
                        if ms < 100.0 {
                            format!("{ms:.1} ms")
                        } else if ms < 1000.0 {
                            format!("{ms:.0} ms")
                        } else {
                            format!("{n:.2} s")
                        }
                    })
                    .custom_parser(|text| {
                        text.chars()
                            .filter(|c| !c.is_whitespace())
                            .collect::<String>()
                            .parse::<f64>()
                            .map(|ms| ms / 1000.0)
                            .ok()
                    });
            }
            if let Some(suffix) = &self.suffix {
                drag_val = drag_val.suffix(suffix);
            }

            let drag_r = ui.add(drag_val);
            if drag_r.changed() {
                self.param.set(value);
            }
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
