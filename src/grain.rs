use rodio::{
    source::{Amplify, Speed, UniformSourceIterator},
    Sample, Source,
};
use std::time::Duration;

use crate::{audio_clip::AudioClip, envelope::GrainEnvelope, widgets::waveform::GrainDrawData};

pub struct Grain<I>
where
    I: Sample,
{
    inner: UniformSourceIterator<Speed<Amplify<GrainInner<I>>>, I>,
    envelope: GrainEnvelope,

    total_duration: Duration,
    elapsed_duration: Duration,
    duration_per_sample: Duration,
    sample_rate: u32,

    // just for animating on the GUI
    start_position: f32,
    position_per_second: f32,
}

impl<I> Grain<I>
where
    I: Sample,
{
    pub fn new(
        audio_clip: AudioClip<I>,
        start_position: f32,
        length: Duration,
        speed: f32,
        amplitude: f32,
        envelope: GrainEnvelope,
    ) -> Grain<I> {
        let index = {
            let samples_per_channel = audio_clip.data.len() / audio_clip.channels as usize;
            (samples_per_channel as f32 * start_position) as usize * audio_clip.channels as usize
        };
        let sample_rate = audio_clip.sample_rate;
        let clip_samples = audio_clip.data.len();
        let duration_per_sample = audio_clip.duration_per_sample().mul_f32(1.0 / speed);
        let total_duration = length.mul_f32(1.0 / speed);
        let position_per_second =
            (speed * sample_rate as f32) / (clip_samples as f32 / audio_clip.channels as f32);

        let inner = UniformSourceIterator::new(
            GrainInner::new(audio_clip, index)
                .amplify(amplitude)
                .speed(speed),
            2,
            sample_rate,
        );

        Grain {
            inner,
            envelope,
            total_duration,
            elapsed_duration: Duration::ZERO,
            duration_per_sample,
            sample_rate,
            start_position,
            position_per_second,
        }
    }

    pub fn draw(&self) -> GrainDrawData {
        let elapsed = self.elapsed_duration.as_secs_f32();
        GrainDrawData {
            current_position: self.position_per_second * elapsed + self.start_position,
            current_progress: elapsed / self.total_duration.as_secs_f32(),
        }
    }
}

impl<I> Iterator for Grain<I>
where
    I: Sample,
{
    type Item = I;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.elapsed_duration >= self.total_duration {
            None
        } else {
            let factor = self.envelope.amplitude_at(
                self.elapsed_duration.as_secs_f32() / self.total_duration.as_secs_f32(),
            );

            let sample = self.inner.next().map(|s| s.amplify(factor));

            self.elapsed_duration += self.duration_per_sample;
            sample
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I> Source for Grain<I>
where
    I: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }
}

/// Just plays raw samples from audio clip
struct GrainInner<I>
where
    I: Sample,
{
    audio_clip: AudioClip<I>,
    index: usize,
}

impl<I> GrainInner<I>
where
    I: Sample,
{
    fn new(audio_clip: AudioClip<I>, start_index: usize) -> Self {
        Self {
            audio_clip,
            index: start_index,
        }
    }
}

impl<I> Iterator for GrainInner<I>
where
    I: Sample,
{
    type Item = I;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.audio_clip.data.get(self.index);
        self.index += 1;
        sample.copied()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.audio_clip.data.len(), Some(self.audio_clip.data.len()))
    }
}

impl<I> Source for GrainInner<I>
where
    I: Sample,
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
