use std::{f32::consts::PI, time::Duration};

use eframe::egui::lerp;

#[derive(Clone)]
pub struct AdsrEnvelope {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain_level: f32,
    pub release_ms: f32,
}

impl AdsrEnvelope {
    pub fn held_amplitude(&self, held_for: Duration) -> f32 {
        let held_ms = held_for.as_secs_f32() * 1000.0;
        if held_ms <= self.attack_ms {
            lerp(0.0..=1.0, held_ms / self.attack_ms)
        } else if held_ms <= self.attack_ms + self.decay_ms {
            lerp(
                1.0..=self.sustain_level,
                (held_ms - self.attack_ms) / self.decay_ms,
            )
        } else {
            self.sustain_level
        }
    }

    pub fn released_amplitude(&self, since_released: Duration) -> f32 {
        let released_ms = since_released.as_secs_f32() * 1000.0;
        if released_ms > self.release_ms {
            0.0
        } else {
            lerp(self.sustain_level..=0.0, released_ms / self.release_ms)
        }
    }

    pub fn oneshot_amplitude(&self, since_triggered: Duration) -> f32 {
        let attack_decay = Duration::from_secs_f32(self.attack_ms + self.decay_ms);

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
            attack_ms: 1.0,
            decay_ms: 1000.0,
            sustain_level: 1.0,
            release_ms: 15.0,
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
