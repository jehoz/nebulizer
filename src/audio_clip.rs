use std::{fs::File, io::BufReader, sync::Arc};

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
