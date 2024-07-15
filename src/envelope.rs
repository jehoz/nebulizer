use std::{f32::consts::PI, time::Duration};

use eframe::egui::lerp;

use crate::params::Parameter;

#[derive(Clone)]
pub struct AdsrEnvelope {
    pub attack: Parameter<Duration>,
    pub decay: Parameter<Duration>,
    pub sustain_level: Parameter<f32>,
    pub release: Parameter<Duration>,
}

impl AdsrEnvelope {
    pub fn held_amplitude(&self, held_for: Duration) -> f32 {
        let attack = self.attack.get();
        let decay = self.decay.get();
        let sustain = self.sustain_level.get();

        if held_for <= attack {
            lerp(0.0..=1.0, held_for.as_secs_f32() / attack.as_secs_f32())
        } else if held_for <= attack + decay {
            lerp(
                1.0..=sustain,
                (held_for - attack).as_secs_f32() / decay.as_secs_f32(),
            )
        } else {
            sustain
        }
    }

    pub fn released_amplitude(&self, since_released: Duration) -> f32 {
        let release = self.release.get();
        let sustain = self.sustain_level.get();

        if since_released > release {
            0.0
        } else {
            lerp(
                sustain..=0.0,
                since_released.as_secs_f32() / release.as_secs_f32(),
            )
        }
    }

    pub fn oneshot_amplitude(&self, since_triggered: Duration) -> f32 {
        let attack_decay = self.attack.get() + self.decay.get();

        if since_triggered <= attack_decay {
            self.held_amplitude(since_triggered)
        } else {
            self.released_amplitude(since_triggered - attack_decay)
        }
    }
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        let time_range = Duration::ZERO..=Duration::from_secs(10);
        Self {
            attack: Parameter::new(Duration::ZERO, time_range.clone()).logarithmic(true),
            decay: Parameter::new(Duration::from_secs(1), time_range.clone()).logarithmic(true),
            sustain_level: Parameter::new(1.0, 0.0..=1.0),
            release: Parameter::new(Duration::from_millis(15), time_range).logarithmic(true),
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
