use midly::{num::u7, MidiMessage};
use rand::random;
use rodio::cpal::Sample as CpalSample;
use rodio::{Sample, Source};
use std::{sync::mpsc::Receiver, time::Duration};

use crate::grain::Grain;

#[derive(Clone)]
pub struct EmitterSettings {
    pub position: f32,
    pub spray_ms: f32,
    pub grain_size_ms: f32,
    pub envelope: f32,
    pub overlap: f32,
    pub transpose: i32,
}

impl Default for EmitterSettings {
    fn default() -> Self {
        EmitterSettings {
            position: 0.0,
            spray_ms: 0.0,
            grain_size_ms: 75.0,
            envelope: 0.75,
            overlap: 0.5,
            transpose: 0,
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

pub struct Emitter<I>
where
    I: Source,
    I::Item: Sample,
{
    input: I,

    pub settings: EmitterSettings,

    channel: Receiver<EmitterMessage>,

    notes: Vec<Note>,
    grains: Vec<Grain<I>>,

    terminated: bool,
}

impl<I> Emitter<I>
where
    I: Clone + Source,
    I::Item: Sample,
{
    pub fn new(input: I, channel: Receiver<EmitterMessage>) -> Emitter<I>
    where
        I: Clone + Source,
        I::Item: Sample,
    {
        Emitter {
            input,
            settings: EmitterSettings::default(),
            channel,

            notes: Vec::new(),
            grains: Vec::new(),

            terminated: false,
        }
    }

    pub fn make_grain(&self, amplitude: f32, speed: f32) -> Grain<I> {
        let mut start = self
            .input
            .total_duration()
            .unwrap()
            .mul_f32(self.settings.position);

        let offset = random::<f32>() * self.settings.spray_ms - self.settings.spray_ms / 2.0;
        if offset < 0.0 {
            if start.as_secs_f32() <= (-offset * 1000.0) {
                start = Duration::from_millis(0);
            } else {
                start -= Duration::from_millis(1).mul_f32(-offset);
            }
        } else {
            start += Duration::from_millis(1).mul_f32(offset);
        }

        let size = Duration::from_millis(1).mul_f32(self.settings.grain_size_ms);

        Grain::new(
            &self.input,
            start,
            size,
            self.settings.envelope,
            amplitude,
            speed,
        )
    }

    fn grain_interval_ms(&self) -> f32 {
        self.settings.grain_size_ms * (1.0 - self.settings.overlap)
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
    I: Clone + Source,
    I::Item: Default + Sample,
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // handle any messages waiting in the channel
        loop {
            match self.channel.try_recv() {
                Ok(msg) => self.handle_message(msg),
                Err(_) => break,
            }
        }

        if self.terminated {
            return None;
        }

        // filter out grains that are done playing
        self.grains.retain(|g| !g.done_playing());

        // check each note playing and generate grains
        let mut notes: Vec<Note> = self.notes.drain(0..).collect();
        for note in notes.iter_mut() {
            note.ms_since_last_grain +=
                1000.0 / (self.sample_rate() as f32 * self.channels() as f32);

            if note.ms_since_last_grain >= self.grain_interval_ms() {
                let amplitude = (note.velocity.as_int() as f32) / 127.0;
                let speed =
                    interval_to_ratio((note.key.as_int() as i32 + self.settings.transpose) - 60);
                let g = self.make_grain(amplitude, speed);
                self.grains.push(g);
                note.ms_since_last_grain = 0.0;
            }
        }
        self.notes = notes;

        // mix all grain samples into one
        let mut samples: Vec<I::Item> = Vec::new();
        for grain in self.grains.iter_mut() {
            if let Some(sample) = grain.next() {
                samples.push(CpalSample::from_sample(sample));
            }
        }

        if let Some(sample) = samples.into_iter().reduce(|a, b| a.saturating_add(b)) {
            Some(sample)
        } else {
            Some(I::Item::default())
        }
    }
}

impl<I> Source for Emitter<I>
where
    I: Clone + Iterator + Source,
    I::Item: Default + Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.input.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// compute pitch ratio from number of semitones between notes
fn interval_to_ratio(semitones: i32) -> f32 {
    2.0_f32.powf(semitones as f32 / 12.0)
}
