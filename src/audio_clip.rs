use std::{fs::File, io::BufReader, sync::Arc, time::Duration};

use rodio::{cpal::FromSample, Decoder, Sample, Source};

#[derive(Clone)]
pub struct AudioClip<I>
where
    I: Sample,
{
    pub data: Arc<[I]>,
    pub channels: u16,
    pub sample_rate: u32,
}

impl<I> AudioClip<I>
where
    I: Sample + FromSample<i16>,
{
    pub fn load_from_file(path: String) -> Option<Self> {
        if let Some(file) = File::open(path).ok() {
            if let Some(decoder) = Decoder::new(BufReader::new(file)).ok() {
                let channels = decoder.channels();
                let sample_rate = decoder.sample_rate();
                Some(AudioClip {
                    data: decoder
                        .buffered()
                        .convert_samples()
                        .collect::<Vec<I>>()
                        .into(),
                    channels,
                    sample_rate,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}

const NANOS_PER_SEC: u64 = 1_000_000_000;

impl<I> AudioClip<I>
where
    I: Sample,
{
    pub fn total_duration(&self) -> Duration {
        self.duration_per_sample().mul_f64(self.data.len() as f64)
    }

    pub fn duration_per_sample(&self) -> Duration {
        let ns = NANOS_PER_SEC / (self.sample_rate as u64 * self.channels as u64);
        Duration::new(0, ns as u32)
    }
}
