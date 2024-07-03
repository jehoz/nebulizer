use std::f32::consts::PI;

#[derive(Clone)]
pub struct GrainEnvelope {
    pub amount: f32,
    pub skew: f32,
}

impl GrainEnvelope {
    pub fn amplitude_at(&self, x: f32) -> f32 {
        tukey_window(x, 1.0, self.amount, self.skew)
    }
}

fn tukey_window(x: f32, length: f32, radius: f32, skew: f32) -> f32 {
    let b = skew.clamp(-1.0, 1.0) * length * radius;
    if x < 0.0 || x > length {
        0.0
    } else if x < 0.5 * (length * radius + b) {
        0.5 * (1.0 - f32::cos((2.0 * PI * x) / (length * radius + b)))
    } else if x < length - 0.5 * (length * radius - b) {
        1.0
    } else {
        0.5 * (1.0
            + f32::cos(
                (2.0 * PI * (x - length + (0.5 * (length * radius - b)))) / (length * radius - b),
            ))
    }
}
