use std::{f32::consts::PI, time::Duration};

use eframe::egui::lerp;

#[derive(Clone)]
pub struct AdsrEnvelope {
    pub attack: Duration,
    pub decay: Duration,
    pub sustain_level: f32,
    pub release: Duration,
}

impl AdsrEnvelope {
    pub fn held_amplitude(&self, held_for: Duration) -> f32 {
        if held_for <= self.attack {
            lerp(
                0.0..=1.0,
                held_for.as_secs_f32() / self.attack.as_secs_f32(),
            )
        } else if held_for <= self.attack + self.decay {
            lerp(
                1.0..=self.sustain_level,
                (held_for - self.attack).as_secs_f32() / self.decay.as_secs_f32(),
            )
        } else {
            self.sustain_level
        }
    }

    pub fn released_amplitude(&self, since_released: Duration) -> f32 {
        if since_released > self.release {
            0.0
        } else {
            lerp(
                self.sustain_level..=0.0,
                since_released.as_secs_f32() / self.release.as_secs_f32(),
            )
        }
    }

    pub fn oneshot_amplitude(&self, since_triggered: Duration) -> f32 {
        let attack_decay = self.attack + self.decay;

        if since_triggered <= attack_decay {
            self.held_amplitude(since_triggered)
        } else {
            self.released_amplitude(since_triggered - attack_decay)
        }
    }
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self {
            attack: Duration::from_millis(1),
            decay: Duration::from_millis(1000),
            sustain_level: 1.0,
            release: Duration::from_millis(15),
        }
    }
}

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
