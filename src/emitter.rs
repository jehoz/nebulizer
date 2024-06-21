use midly::{num::u7, MidiMessage};
use rodio::{Sample, Source};
use std::{sync::mpsc::Receiver, time::Duration};

use crate::grain::Grain;

#[derive(Clone)]
pub struct EmitterSettings {
    pub position: f32,
    pub grain_size_ms: f32,
    pub envelope: f32,
    pub overlap: f32,
    pub transpose: i32,
}

impl Default for EmitterSettings {
    fn default() -> Self {
        EmitterSettings {
            position: 0.0,
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
}

pub struct Emitter<I> {
    input: I,

    pub settings: EmitterSettings,

    channel: Receiver<EmitterMessage>,

    ms_since_last_grain: f32,
    notes: Vec<Note>,
    grains: Vec<Grain<I>>,
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

            ms_since_last_grain: 0.0,
            notes: Vec::new(),
            grains: Vec::new(),
        }
    }

    pub fn make_grain(&self) -> Grain<I> {
        let start = self
            .input
            .total_duration()
            .unwrap()
            .mul_f32(self.settings.position);
        let size = Duration::from_nanos((1000000.0 * self.settings.grain_size_ms) as u64);
        Grain::new(&self.input, start, size, self.settings.envelope)
    }

    fn grain_interval_ms(&self) -> f32 {
        self.settings.grain_size_ms * (1.0 - self.settings.overlap)
    }

    fn handle_message(&mut self, msg: EmitterMessage) {
        match msg {
            EmitterMessage::Settings(settings) => self.settings = settings,
            EmitterMessage::Midi(_) => {}
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

        // filter out grains that are done playing
        self.grains.retain(|g| !g.done_playing());

        // make new grain if needed
        self.ms_since_last_grain += 1000.0 / (self.sample_rate() as f32 * self.channels() as f32);
        if self.ms_since_last_grain >= self.grain_interval_ms() {
            let g = self.make_grain();
            self.grains.push(g);
            self.ms_since_last_grain = 0.0;
        }

        // mix all grain samples into one
        let mut samples: Vec<I::Item> = Vec::new();
        for grain in self.grains.iter_mut() {
            if let Some(sample) = grain.next() {
                samples.push(sample);
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
