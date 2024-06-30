use crate::window::tukey_window;

use eframe::{
    egui::{pos2, vec2, Color32, Frame, Pos2, Rect, Ui, Widget},
    emath,
    epaint::{self, Stroke},
};

pub struct EnvelopePlot {
    amount: f32,
    skew: f32,
}

impl EnvelopePlot {
    pub fn new(amount: f32, skew: f32) -> Self {
        Self { amount, skew }
    }
}

impl Widget for EnvelopePlot {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        Frame::canvas(ui.style())
            .show(ui, |ui| {
                let color = Color32::from_white_alpha(240);

                let desired_size = ui.available_width() * vec2(1.0, 0.5);
                let (_id, rect) = ui.allocate_space(desired_size);

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, 1.0..=-1.0),
                    rect,
                );

                let mut shapes = vec![];
                let n = 120;

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let x = i as f32 / (n as f32);
                        let y = tukey_window(x, 1.0, self.amount, self.skew);

                        to_screen * pos2(x, y - 0.5)
                    })
                    .collect();

                shapes.push(epaint::Shape::line(points, Stroke::new(1.0, color)));

                ui.painter().extend(shapes)
            })
            .response
    }
}
