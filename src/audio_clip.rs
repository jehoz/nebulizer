use std::{fs::File, io::BufReader};

use rodio::{source::Buffered, Decoder, Source};

pub type AudioClip = Buffered<Decoder<BufReader<File>>>;

pub fn load_audio_clip(path: String) -> Option<AudioClip> {
    if let Some(file) = File::open(path).ok() {
        if let Some(decoder) = Decoder::new(BufReader::new(file)).ok() {
            Some(decoder.buffered())
        } else {
            None
        }
    } else {
        None
    }
}
