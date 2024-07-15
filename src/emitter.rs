use midly::{num::u7, MidiMessage};
use rand::{thread_rng, Rng};
use rodio::cpal::{FromSample, Sample as CpalSample};
use rodio::source::Amplify;
use rodio::{
    source::{Speed, UniformSourceIterator},
    Sample, Source,
};
use std::collections::VecDeque;
use std::{mem, sync::mpsc::Receiver, time::Duration};

use crate::params::{EmitterParams, KeyMode};
use crate::{audio_clip::AudioClip, envelope::AdsrEnvelope, grain::Grain};

#[derive(PartialEq)]
enum NoteState {
    Held(Duration),
    Released(Duration),
    Finished,
}

struct Note {
    key: u7,
    envelope: AdsrEnvelope,

    state: NoteState,

    since_last_grain: Duration,
}

impl Note {
    fn new(key: u7, envelope: AdsrEnvelope) -> Self {
        Self {
            key,
            envelope,
            state: NoteState::Held(Duration::ZERO),
            since_last_grain: Duration::from_secs(100),
        }
    }

    fn update(&mut self, delta_time: Duration) {
        self.since_last_grain += delta_time;
        match self.state {
            NoteState::Held(time) => self.state = NoteState::Held(time + delta_time),
            NoteState::Released(time) => {
                let new_time = time + delta_time;
                if new_time >= self.envelope.release {
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
    NoteOn { key: u7, vel: u7 },
    NoteOff { key: u7, vel: u7 },
    Params(EmitterParams),
    Terminate,
}

type PitchedGrain<I> = UniformSourceIterator<Speed<Amplify<Grain<I>>>, I>;

pub struct Emitter<I>
where
    I: Sample,
{
    audio_clip: AudioClip<I>,
    current_audio_channel: u16,

    pub params: EmitterParams,

    msg_receiver: Receiver<EmitterMessage>,

    notes: VecDeque<Note>,
    grains: Vec<PitchedGrain<I>>,

    terminated: bool,
}

impl<I> Emitter<I>
where
    I: Sample,
{
    pub fn new(audio_clip: &AudioClip<I>, msg_receiver: Receiver<EmitterMessage>) -> Emitter<I>
    where
        I: Sample,
    {
        Emitter {
            audio_clip: audio_clip.clone(),
            current_audio_channel: 0,
            params: EmitterParams::default(),
            msg_receiver,

            notes: VecDeque::new(),
            grains: Vec::new(),

            terminated: false,
        }
    }

    fn make_grain(&self, audio_clip: &AudioClip<I>, note: &Note) -> PitchedGrain<I> {
        let mut rng = thread_rng();

        let start = {
            let pos = match self.params.key_mode {
                KeyMode::Pitch => self.params.position.value,

                KeyMode::Slice => {
                    let slice = note.key.as_int() % self.params.num_slices.value;
                    slice as f32 / self.params.num_slices.value as f32
                }
            };

            if self.params.spray.value > Duration::ZERO {
                let spray_relative = {
                    let spray = self.params.spray.value.as_secs_f32();
                    let clip = audio_clip.total_duration().as_secs_f32();
                    spray / clip
                };
                let min = (pos - spray_relative / 2.0).max(0.0);
                let max = (pos + spray_relative / 2.0).min(1.0);
                rng.gen_range(min..max)
            } else {
                pos
            }
        };

        let speed = match self.params.key_mode {
            KeyMode::Pitch => {
                interval_to_ratio((note.key.as_int() as i32 + self.params.transpose.value) - 60)
            }
            KeyMode::Slice => interval_to_ratio(self.params.transpose.value),
        };

        UniformSourceIterator::new(
            Grain::new(
                audio_clip.clone(),
                start,
                self.params.length.value,
                self.params.grain_envelope.clone(),
            )
            .amplify(note.amplitude())
            .speed(speed),
            2,
            audio_clip.sample_rate,
        )
    }

    fn grain_interval(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.params.density.value)
    }

    fn handle_message(&mut self, msg: EmitterMessage) {
        match msg {
            EmitterMessage::NoteOn { key, .. } => {
                while self.params.polyphony < self.notes.len() as u32 + 1 {
                    self.notes.pop_front();
                }
                self.notes
                    .push_back(Note::new(key, self.params.note_envelope.clone()));
            }
            EmitterMessage::NoteOff { key, .. } => {
                for note in self.notes.iter_mut() {
                    if note.key == key {
                        note.state = NoteState::Released(Duration::ZERO);
                    }
                }
            }
            EmitterMessage::Params(settings) => self.params = settings,
            EmitterMessage::Terminate => {
                self.terminated = true;
            }
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
        while let Ok(msg) = self.msg_receiver.try_recv() {
            self.handle_message(msg);
        }

        if self.terminated {
            return None;
        }

        // only update notes (and potentially create new grains) at the beginning of an interleaved
        // sequence.  this prevents grains from being created with their channels out of sync
        if self.current_audio_channel == 0 {
            let notes = mem::take(&mut self.notes);
            let mut live_notes = vec![];
            for mut note in notes.into_iter() {
                note.update(
                    self.audio_clip
                        .duration_per_sample()
                        .mul_f32(self.audio_clip.channels as f32),
                );

                if note.state == NoteState::Finished {
                    continue;
                }

                if note.since_last_grain >= self.grain_interval() {
                    let g = self.make_grain(&self.audio_clip, &note);
                    self.grains.push(g);
                    note.since_last_grain = Duration::ZERO;
                }

                live_notes.push(note);
            }
            self.notes.extend(live_notes);
        }

        let mut samples = vec![];
        let mut live_grains = vec![];
        for mut grain in self.grains.drain(..) {
            if let Some(sample) = grain.next() {
                live_grains.push(grain);
                samples.push(sample);
            }
        }
        self.grains.extend(live_grains);

        self.current_audio_channel = (self.current_audio_channel + 1) % self.channels();

        if let Some(sample) = samples.into_iter().reduce(|a, b| a.saturating_add(b)) {
            // use tanh as a primitive limiter
            let sample = f32::from_sample(sample.amplify(self.params.amplitude.value)).tanh();
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
        // hard code this for now, but it should probably be configurable
        2
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
