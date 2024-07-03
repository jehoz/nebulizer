use rodio::{Sample, Source};
use std::time::Duration;

use crate::{audio_clip::AudioClip, envelope::GrainEnvelope};

pub struct Grain<I>
where
    I: Sample,
{
    audio_clip: AudioClip<I>,

    index: usize,
    total_duration: Duration,
    elapsed_duration: Duration,

    envelope: GrainEnvelope,
}

impl<I> Grain<I>
where
    I: Sample,
{
    pub fn new(
        audio_clip: AudioClip<I>,
        start_position: f32,
        length: Duration,
        envelope: GrainEnvelope,
    ) -> Grain<I> {
        let index = {
            let samples_per_channel = audio_clip.data.len() / audio_clip.channels as usize;
            (samples_per_channel as f32 * start_position) as usize * audio_clip.channels as usize
        };

        Grain {
            audio_clip,
            index,
            total_duration: length,
            elapsed_duration: Duration::ZERO,
            envelope,
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

            let sample = self
                .audio_clip
                .data
                .get(self.index)
                .map(|s| s.amplify(factor));
            self.index += 1;
            self.elapsed_duration += self.audio_clip.duration_per_sample();
            sample
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.audio_clip.data.len(), Some(self.audio_clip.data.len()))
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
        self.audio_clip.channels
    }

    fn sample_rate(&self) -> u32 {
        self.audio_clip.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }
}
