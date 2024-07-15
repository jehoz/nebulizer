use std::{ops::RangeInclusive, time::Duration};

use midly::num::u7;
use strum_macros::{Display, VariantArray};

use crate::{
    envelope::{AdsrEnvelope, GrainEnvelope},
    numeric::Numeric,
};

#[derive(Clone)]
pub struct Parameter<I> {
    pub value: I,
    pub range: RangeInclusive<I>,
}

impl<I> Parameter<I>
where
    I: Numeric,
{
    fn new(default: I, range: RangeInclusive<I>) -> Self {
        Self {
            value: default,
            range,
        }
    }

    pub fn set_from_midi_cc(&mut self, cc_value: u7) {
        let normalized_value = cc_value.as_int() as f64 / 127.0;
        let (min, max) = (self.range.start().to_f64(), self.range.end().to_f64());
        let value_in_range = normalized_value * (max - min) + min;
        self.value = I::from_f64(value_in_range);
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
            spray: Parameter::new(Duration::ZERO, Duration::ZERO..=Duration::from_secs(1)),
            length: Parameter::new(
                Duration::from_millis(100),
                Duration::ZERO..=Duration::from_secs(1),
            ),
            density: Parameter::new(10.0, 1.0..=100.0),
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
