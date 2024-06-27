use crate::window::tukey_window;

use eframe::{
    egui::{pos2, vec2, Color32, Frame, Pos2, Rect, Ui},
    emath,
    epaint::{self, Stroke},
};

pub fn envelope_plot(ui: &mut Ui, amount: f32, skew: f32) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let color = Color32::from_white_alpha(240);

        let desired_size = ui.available_width() * vec2(1.0, 0.35);
        let (_id, rect) = ui.allocate_space(desired_size);

        let to_screen =
            emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

        let mut shapes = vec![];
        {
            let n = 120;

            let points: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let t = i as f32 / (n as f32);
                    let y = tukey_window(t, 1.0, amount, skew);

                    to_screen * pos2(t as f32, -1.0 * y + 0.5)
                })
                .collect();

            shapes.push(epaint::Shape::line(points, Stroke::new(1.0, color)));
        }

        ui.painter().extend(shapes)
    });
}
