use midly::{num::u7, MidiMessage};
use rand::{thread_rng, Rng};
use rodio::{
    source::{Speed, UniformSourceIterator},
    Sample, Source,
};
use std::{mem, sync::mpsc::Receiver, time::Duration};

use crate::{audio_clip::AudioClip, grain::Grain};

#[derive(Clone, PartialEq, Eq)]
pub enum KeyMode {
    Pitch,
    Slice(u8),
}

#[derive(Clone)]
pub struct EmitterSettings {
    /// Whether MIDI keys control the pitch or the start position
    pub key_mode: KeyMode,

    /// The relative position in the source file where a grain starts
    pub position: f32,
    /// Amount of random deviation from position parameter
    pub position_rand: f32,

    /// The length of a grain window in ms
    pub grain_size: f32,
    /// Amount of random deviation from of grain_size parameter
    pub grain_size_rand: f32,

    /// The number of grains played per second (in hz)
    pub density: f32,

    /// Proportion of the grain window that is eased in/out
    pub envelope_amount: f32,

    /// Skews the tukey window left or right:
    /// -1 = instant attack, long decay
    ///  0 = symmetrical attack and decay
    ///  1 = long attack instant decay
    pub envelope_skew: f32,

    /// Pitch transposition of input sample in semitones
    pub transpose: i32,

    /// The volume level of sound coming out of the emitter, relative to the original audio sample
    pub amplitude: f32,
}

impl Default for EmitterSettings {
    fn default() -> Self {
        EmitterSettings {
            key_mode: KeyMode::Pitch,
            position: 0.0,
            position_rand: 0.0,
            grain_size: 100.0,
            grain_size_rand: 0.0,
            density: 10.0,
            envelope_amount: 0.5,
            envelope_skew: 0.0,
            transpose: 0,
            amplitude: 1.0,
        }
    }
}

struct Note {
    velocity: u7,
    key: u7,
    ms_since_last_grain: f32,
}

pub enum EmitterMessage {
    Settings(EmitterSettings),
    Midi(MidiMessage),
    Terminate,
}

type PitchedGrain<I> = UniformSourceIterator<Speed<Grain<I>>, I>;

pub struct Emitter<I>
where
    I: Sample,
{
    audio_clip: AudioClip<I>,

    pub settings: EmitterSettings,

    channel: Receiver<EmitterMessage>,

    notes: Vec<Note>,
    // grains: Vec<Grain<I>>,
    grains: Vec<PitchedGrain<I>>,

    terminated: bool,
}

impl<I> Emitter<I>
where
    I: Sample,
{
    pub fn new(audio_clip: AudioClip<I>, channel: Receiver<EmitterMessage>) -> Emitter<I>
    where
        I: Sample,
    {
        Emitter {
            audio_clip,
            settings: EmitterSettings::default(),
            channel,

            notes: Vec::new(),
            grains: Vec::new(),

            terminated: false,
        }
    }

    fn make_grain(&self, note: &Note) -> PitchedGrain<I> {
        let mut rng = thread_rng();

        let start = {
            let pos = match self.settings.key_mode {
                KeyMode::Pitch => self.settings.position,

                KeyMode::Slice(num_slices) => {
                    let slice = note.key.as_int() % num_slices;
                    slice as f32 / num_slices as f32
                }
            };

            if self.settings.position_rand > 0.0 {
                let min = (pos - self.settings.position_rand / 2.0).max(0.0);
                let max = (pos + self.settings.position_rand / 2.0).min(1.0);
                rng.gen_range(min..max)
            } else {
                pos
            }
        };

        let speed = match self.settings.key_mode {
            KeyMode::Pitch => {
                interval_to_ratio((note.key.as_int() as i32 + self.settings.transpose) - 60)
            }
            KeyMode::Slice(_) => interval_to_ratio(self.settings.transpose),
        };

        let length_sec = {
            (if self.settings.grain_size_rand == 0.0 {
                self.settings.grain_size
            } else {
                let scaled_rand = self.settings.grain_size_rand * (999.0);
                let min = (self.settings.grain_size - scaled_rand / 2.0).max(1.0);
                let max = (self.settings.grain_size + scaled_rand / 2.0).min(1000.0);
                rng.gen_range(min..max)
            }) * 0.001
        };

        UniformSourceIterator::new(
            Grain::new(
                self.audio_clip.clone(),
                start,
                Duration::from_secs_f32(length_sec),
                self.settings.envelope_amount,
                self.settings.envelope_skew,
            )
            .speed(speed),
            self.audio_clip.channels,
            self.audio_clip.sample_rate,
        )
    }

    fn grain_interval_ms(&self) -> f32 {
        1000.0 / self.settings.density
    }

    fn handle_message(&mut self, msg: EmitterMessage) {
        match msg {
            EmitterMessage::Settings(settings) => self.settings = settings,
            EmitterMessage::Midi(midi_msg) => match midi_msg {
                MidiMessage::NoteOn { key, vel } => {
                    self.notes.push(Note {
                        velocity: vel,
                        key,
                        ms_since_last_grain: 10000.0,
                    });
                }
                MidiMessage::NoteOff { key, .. } => {
                    self.notes.retain(|n| n.key != key);
                }
                _ => {}
            },
            EmitterMessage::Terminate => self.terminated = true,
        }
    }
}

impl<I> Iterator for Emitter<I>
where
    I: Default + Sample,
{
    type Item = I;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Ok(msg) = self.channel.try_recv() {
            self.handle_message(msg);
        }

        if self.terminated {
            return None;
        }

        // check each note playing and generate grains
        let ms_per_sample = 1000.0 / (self.sample_rate() as f32 * self.channels() as f32);
        let mut notes: Vec<Note> = mem::take(&mut self.notes);
        for note in notes.iter_mut() {
            note.ms_since_last_grain += ms_per_sample;

            if note.ms_since_last_grain >= self.grain_interval_ms() {
                let g = self.make_grain(note);
                self.grains.push(g);
                note.ms_since_last_grain = 0.0;
            }
        }
        self.notes = notes;

        // mix all grain samples into one
        let mut samples: Vec<I> = vec![];
        let mut live_grains = vec![];
        for mut grain in self.grains.drain(..) {
            if let Some(sample) = grain.next() {
                samples.push(sample);
                live_grains.push(grain);
            }
        }
        self.grains.extend(live_grains);

        // attenuate individual grain volume when many are playing simultaneously
        let fac = 1.0 / ((samples.len() as f32).ln() + 1.0);
        for s in samples.iter_mut() {
            *s = s.amplify(fac);
        }

        if let Some(sample) = samples.into_iter().reduce(|a, b| a.saturating_add(b)) {
            Some(sample.amplify(self.settings.amplitude))
        } else {
            Some(I::default())
        }
    }
}

impl<I> Source for Emitter<I>
where
    I: Default + Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.audio_clip.channels
    }

    fn sample_rate(&self) -> u32 {
        self.audio_clip.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// compute pitch ratio from number of semitones between notes
fn interval_to_ratio(semitones: i32) -> f32 {
    2.0_f32.powf(semitones as f32 / 12.0)
}
