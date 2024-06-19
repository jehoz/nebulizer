use rodio::{Sample, Source};
use std::time::Duration;

use crate::grain::Grain;

pub struct Emitter<I> {
    input: I,

    position: f32,
    grain_size: Duration,
    envelope: f32,
    overlap: f32,

    grains: Vec<Grain<I>>,
}

impl<I> Emitter<I>
where
    I: Clone + Source,
    I::Item: Sample,
{
    fn new(input: I) -> Emitter<I>
    where
        I: Source,
        I::Item: Sample,
    {
        Emitter {
            input,

            position: 0.0,
            grain_size: Duration::from_millis(25),
            envelope: 0.5,
            overlap: 0.0,

            grains: Vec::new(),
        }
    }
}

impl<I> Iterator for Emitter<I>
where
    I: Clone + Source,
    I::Item: Sample,
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // filter out grains that are done playing
        self.grains = self
            .grains
            .into_iter()
            .filter(|g| !g.done_playing())
            .collect();

        // mix all grain samples into one
        self.grains
            .into_iter()
            .filter_map(|mut g| g.next())
            .reduce(|a, b| a.saturating_add(b))
    }
}

impl<I> Source for Emitter<I>
where
    I: Clone + Iterator + Source,
    I::Item: Sample,
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
