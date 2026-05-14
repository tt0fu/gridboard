pub mod audio;
pub mod audio_engine;
use std::{thread::sleep, time::Duration};

use crate::audio_engine::AudioEngine;

fn main() {
    let mut engine = AudioEngine::new();
    for _ in 0..100 {
        engine.play(&std::path::PathBuf::from("test.mp3"));
        sleep(Duration::from_millis(50));
    }
    sleep(Duration::from_millis(10000));
}
