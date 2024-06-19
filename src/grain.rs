use std::f32::consts::PI;
use std::time::Duration;

use rodio::{
    source::{SkipDuration, TakeDuration},
    Sample, Source,
};

pub struct Grain<I> {
    input: TakeDuration<SkipDuration<I>>,

    size_ns: f32,
    elapsed_ns: f32,
    envelope: f32,
}

impl<I> Grain<I>
where
    I: Clone + Source,
    I::Item: Sample,
{
    fn new(input: I, start: Duration, size: Duration, envelope: f32) -> Grain<I> {
        let grain = input.clone().skip_duration(start).take_duration(size);

        Grain {
            input: grain,
            size_ns: size.as_nanos() as f32,
            elapsed_ns: 0.0,
            envelope: envelope.clamp(0.0, 1.0),
        }
    }
}

impl<I> Iterator for Grain<I>
where
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let factor = tukey_window(self.elapsed_ns, self.size_ns, self.envelope);
        self.elapsed_ns +=
            1_000_000_000.0 / (self.input.sample_rate() as f32 * self.channels() as f32);

        self.input.next().map(|sample| sample.amplify(factor))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for Grain<I>
where
    I: Source,
    I::Item: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.input.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}

fn tukey_window(x: f32, length: f32, radius: f32) -> f32 {
    if x < 0.0 || x > length {
        0.0
    } else if x < 0.5 * length * radius {
        0.5 * (1.0 - f32::cos((2.0 * PI * x) / (length * radius)))
    } else if x < length - 0.5 * length * radius {
        1.0
    } else {
        0.5 * (1.0
            - f32::cos((2.0 * PI * (x - length + (0.5 * length * radius))) / (length * radius)))
    }
}
