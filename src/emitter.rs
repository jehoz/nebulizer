use midly::{num::u7, MidiMessage};
use rand::{thread_rng, Rng};
use rodio::cpal::{FromSample, Sample as CpalSample};
use rodio::{
    source::{Speed, UniformSourceIterator},
    Sample, Source,
};
use std::{mem, sync::mpsc::Receiver, time::Duration};

use crate::{
    audio_clip::AudioClip,
    envelope::{AdsrEnvelope, GrainEnvelope},
    grain::Grain,
};

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
    pub spray_ms: f32,

    /// The length of a grain window in ms
    pub length_ms: f32,

    /// The number of grains played per second (in hz)
    pub density: f32,

    /// Envelope applied to each grain
    pub grain_envelope: GrainEnvelope,

    /// ADSR envelope applied to each note
    pub note_envelope: AdsrEnvelope,

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
            spray_ms: 0.0,
            length_ms: 100.0,
            density: 10.0,
            grain_envelope: GrainEnvelope {
                amount: 0.5,
                skew: 0.0,
            },
            note_envelope: AdsrEnvelope::default(),
            transpose: 0,
            amplitude: 1.0,
        }
    }
}

#[derive(PartialEq)]
enum NoteState {
    Held(Duration),
    Released(Duration),
    Finished,
}

struct Note<I>
where
    I: Sample,
{
    key: u7,
    envelope: AdsrEnvelope,

    state: NoteState,
    grains: Vec<PitchedGrain<I>>,

    since_last_grain: Duration,
}

impl<I> Note<I>
where
    I: Sample,
{
    fn new(key: u7, envelope: AdsrEnvelope) -> Self {
        Self {
            key,
            envelope,
            state: NoteState::Held(Duration::ZERO),
            grains: Vec::new(),
            since_last_grain: Duration::from_secs(100),
        }
    }

    fn update(&mut self, delta_time: Duration) {
        self.since_last_grain += delta_time;
        match self.state {
            NoteState::Held(time) => self.state = NoteState::Held(time + delta_time),
            NoteState::Released(time) => {
                let new_time = time + delta_time;
                if new_time.as_secs_f32() * 1000.0 >= self.envelope.release_ms {
                    self.state = NoteState::Finished;
                } else {
                    self.state = NoteState::Released(new_time);
                }
            }
            _ => {}
        }
    }

    fn amplitude(&self) -> f32 {
        match self.state {
            NoteState::Held(t) => self.envelope.held_amplitude(t),
            NoteState::Released(t) => self.envelope.released_amplitude(t),
            NoteState::Finished => 0.0,
        }
    }
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

    notes: Vec<Note<I>>,

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
            terminated: false,
        }
    }

    fn make_grain(&self, note: &Note<I>) -> PitchedGrain<I> {
        let mut rng = thread_rng();

        let start = {
            let pos = match self.settings.key_mode {
                KeyMode::Pitch => self.settings.position,

                KeyMode::Slice(num_slices) => {
                    let slice = note.key.as_int() % num_slices;
                    slice as f32 / num_slices as f32
                }
            };

            if self.settings.spray_ms > 0.0 {
                let spray_relative = {
                    let clip_ms = self.audio_clip.total_duration().as_secs_f32() * 1000.0;
                    self.settings.spray_ms / clip_ms
                };
                let min = (pos - spray_relative / 2.0).max(0.0);
                let max = (pos + spray_relative / 2.0).min(1.0);
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

        let duration = Duration::from_secs_f32(self.settings.length_ms * 0.001);

        UniformSourceIterator::new(
            Grain::new(
                self.audio_clip.clone(),
                start,
                duration,
                self.settings.grain_envelope.clone(),
            )
            .speed(speed),
            self.audio_clip.channels,
            self.audio_clip.sample_rate,
        )
    }

    fn grain_interval(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.settings.density)
    }

    fn handle_message(&mut self, msg: EmitterMessage) {
        match msg {
            EmitterMessage::Settings(settings) => self.settings = settings,
            EmitterMessage::Midi(midi_msg) => match midi_msg {
                MidiMessage::NoteOn { key, .. } => {
                    self.notes
                        .push(Note::new(key, self.settings.note_envelope.clone()));
                }
                MidiMessage::NoteOff { key, .. } => {
                    for note in self.notes.iter_mut() {
                        if note.key == key {
                            note.state = NoteState::Released(Duration::ZERO);
                        }
                    }
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
    f32: FromSample<I>,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Ok(msg) = self.channel.try_recv() {
            self.handle_message(msg);
        }

        if self.terminated {
            return None;
        }

        let notes = mem::take(&mut self.notes);
        let mut live_notes = vec![];
        let mut samples: Vec<I> = vec![];

        for mut note in notes.into_iter() {
            note.update(self.audio_clip.duration_per_sample());

            if note.state == NoteState::Finished {
                continue;
            }

            if note.since_last_grain >= self.grain_interval() {
                let g = self.make_grain(&note);
                note.grains.push(g);
                note.since_last_grain = Duration::ZERO;
            }

            let mut note_samples = vec![];
            let mut live_grains = vec![];
            for mut grain in note.grains.drain(..) {
                if let Some(sample) = grain.next() {
                    live_grains.push(grain);
                    note_samples.push(sample);
                }
            }
            note.grains.extend(live_grains);

            if let Some(s) = note_samples.into_iter().reduce(|a, b| a.saturating_add(b)) {
                samples.push(s.amplify(note.amplitude()));
            }

            live_notes.push(note);
        }
        self.notes.extend(live_notes);

        if let Some(sample) = samples.into_iter().reduce(|a, b| a.saturating_add(b)) {
            // use tanh as a very primite limiter
            let sample = f32::from_sample(sample.amplify(self.settings.amplitude)).tanh();
            Some(sample.to_sample())
        } else {
            Some(0.0)
        }
    }
}

impl<I> Source for Emitter<I>
where
    I: Default + Sample,
    f32: FromSample<I>,
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
