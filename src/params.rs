use std::{ops::RangeInclusive, time::Duration};

use midly::num::u7;
use strum_macros::{Display, VariantArray};

use crate::envelope::{AdsrEnvelope, GrainEnvelope};

#[derive(Clone)]
pub struct Parameter<I> {
    pub value: I,
    pub range: RangeInclusive<I>,
}

impl<I> Parameter<I> {
    fn new(default: I, range: RangeInclusive<I>) -> Self {
        Self {
            value: default,
            range,
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
    pub num_slices: u8,

    /// The relative position in the source file where a grain starts (in pitch mode)
    pub position: f32,

    /// Amount of random deviation from position parameter
    pub spray: Duration,

    /// The length of a grain window
    pub length: Duration,

    /// The number of grains played per second (in hz)
    pub density: f32,

    /// Envelope applied to each grain
    pub grain_envelope: GrainEnvelope,

    /// ADSR envelope applied to each note
    pub note_envelope: AdsrEnvelope,

    // Number of notes that can be played simultaneously
    pub polyphony: u32,

    /// Pitch transposition of input sample in semitones
    pub transpose: i32,

    /// The volume level of sound coming out of the emitter, relative to the original audio sample
    pub amplitude: f32,
}

impl Default for EmitterParams {
    fn default() -> Self {
        EmitterParams {
            midi_cc_map: Vec::new(),
            key_mode: KeyMode::Pitch,
            num_slices: 12,
            position: 0.0,
            spray: Duration::ZERO,
            length: Duration::from_millis(100),
            density: 10.0,
            grain_envelope: GrainEnvelope {
                amount: 0.5,
                skew: 0.0,
            },
            note_envelope: AdsrEnvelope::default(),
            polyphony: 8,
            transpose: 0,
            amplitude: 1.0,
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
    NoteEnvelope,
    Transpose,
    Amplitude,
}

type MidiControlMap = Vec<(u7, ControlParam)>;
