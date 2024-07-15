use std::{ops::RangeInclusive, time::Duration};

use eframe::egui::{lerp, remap_clamp};
use midly::num::u7;
use strum_macros::{Display, VariantArray};

use crate::{
    envelope::{AdsrEnvelope, GrainEnvelope},
    numeric::Numeric,
};

#[derive(Clone)]
pub struct Parameter<I> {
    value: I,
    range: RangeInclusive<I>,
    logarithmic: bool,
    smallest_positive: f64,
}

impl<I> Parameter<I>
where
    I: Numeric,
{
    pub fn new(default: I, range: RangeInclusive<I>) -> Self {
        Self {
            value: default,
            range,
            logarithmic: false,
            smallest_positive: 1e-6,
        }
    }

    pub fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.logarithmic = logarithmic;
        self
    }

    pub fn smallest_positive(mut self, smallest_positive: f64) -> Self {
        self.smallest_positive = smallest_positive;
        self
    }

    pub fn get(&self) -> I {
        self.value
    }

    pub fn set(&mut self, val: I) {
        self.value = val;
    }

    pub fn range(&self) -> RangeInclusive<I> {
        self.range.clone()
    }

    /// Get current value as a normalized position [0,1] in range
    pub fn get_normalized(&self) -> f64 {
        self.value_to_normalized(self.value.to_f64(), self.range_f64())
    }

    /// Set value using a normalized position [0,1] within range
    pub fn set_normalized(&mut self, norm_val: f64) {
        let val = self.normalized_to_value(norm_val, self.range_f64());
        self.value = I::from_f64(val);
    }

    fn range_f64(&self) -> RangeInclusive<f64> {
        self.range.start().to_f64()..=self.range.end().to_f64()
    }

    fn value_to_normalized(&self, value: f64, range: RangeInclusive<f64>) -> f64 {
        let (min, max) = (*range.start(), *range.end());

        if min == max {
            0.5
        } else if min > max {
            1.0 - self.value_to_normalized(value, max..=min)
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

    fn normalized_to_value(&self, normalized: f64, range: RangeInclusive<f64>) -> f64 {
        let (min, max) = (*range.start(), *range.end());

        if min == max {
            min
        } else if min > max {
            self.normalized_to_value(1.0 - normalized, max..=min)
        } else if normalized <= 0.0 {
            min
        } else if normalized >= 1.0 {
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

            let log = lerp(min_log..=max_log, normalized);
            10.0_f64.powf(log)
        } else {
            lerp(range, normalized.clamp(0.0, 1.0))
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum KeyMode {
    Pitch,
    Slice,
}

#[derive(Clone)]
pub struct EmitterParams {
    pub midi_cc_map: MidiControlMap,

    /// Whether MIDI keys control the pitch or the start position
    pub key_mode: KeyMode,

    /// Number of equal-length slices of the clip are mapped to different keys in the Slice key mode
    pub num_slices: Parameter<u8>,

    /// The relative position in the source file where a grain starts (in pitch mode)
    pub position: Parameter<f32>,

    /// Amount of random deviation from position parameter
    pub spray: Parameter<Duration>,

    /// The length of a grain window
    pub length: Parameter<Duration>,

    /// The number of grains played per second (in hz)
    pub density: Parameter<f32>,

    /// Envelope applied to each grain
    pub grain_envelope: GrainEnvelope,

    /// ADSR envelope applied to each note
    pub note_envelope: AdsrEnvelope,

    // Number of notes that can be played simultaneously
    pub polyphony: u32,

    /// Pitch transposition of input sample in semitones
    pub transpose: Parameter<i32>,

    /// The volume level of sound coming out of the emitter, relative to the original audio sample
    pub amplitude: Parameter<f32>,
}

impl Default for EmitterParams {
    fn default() -> Self {
        EmitterParams {
            midi_cc_map: Vec::new(),
            key_mode: KeyMode::Pitch,
            num_slices: Parameter::new(12, 1..=127),
            position: Parameter::new(0.0, 0.0..=1.0),
            spray: Parameter::new(Duration::ZERO, Duration::ZERO..=Duration::from_secs(1))
                .logarithmic(true),
            length: Parameter::new(
                Duration::from_millis(100),
                Duration::ZERO..=Duration::from_secs(1),
            )
            .logarithmic(true),
            density: Parameter::new(10.0, 1.0..=100.0).logarithmic(true),
            grain_envelope: GrainEnvelope {
                amount: 0.5,
                skew: 0.0,
            },
            note_envelope: AdsrEnvelope::default(),
            polyphony: 8,
            transpose: Parameter::new(0, -12..=12),
            amplitude: Parameter::new(1.0, 0.0..=1.0),
        }
    }
}

/// All emitter parameters that can be controlled with MIDI CC messages
#[derive(Clone, Display, VariantArray, PartialEq)]
pub enum ControlParam {
    Position,
    NumSlices,
    Spray,
    Length,
    Density,
    GrainEnvelopeAmount,
    GrainEnvelopeSkew,
    NoteEnvelopeAttack,
    NoteEnvelopeDecay,
    NoteEnvelopeSustain,
    NoteEnvelopeRelease,
    Transpose,
    Amplitude,
}

type MidiControlMap = Vec<(u7, ControlParam)>;
