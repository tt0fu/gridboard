use crate::load_audio::load_audio;
use audioadapter::Adapter;
use cpal::{
    BufferSize::Fixed,
    Device, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rubato::audioadapter_buffers::owned::InterleavedOwned;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

struct Play {
    audio: Arc<InterleavedOwned<f32>>,
    frame: usize,
}

impl Play {
    fn new(audio: Arc<InterleavedOwned<f32>>) -> Self {
        Self { audio, frame: 0 }
    }
    fn is_playing(&self) -> bool {
        self.frame < self.audio.frames()
    }
    fn advance(&mut self, slice: &mut [f32]) {
        self.audio.copy_from_frame_to_slice(self.frame, 0, slice);
        self.frame += 1;
    }
}

pub struct AudioEngine {
    _stream: Stream,
    config: StreamConfig,
    cache: Mutex<HashMap<PathBuf, Arc<InterleavedOwned<f32>>>>,
    plays: Arc<Mutex<Vec<Play>>>,
    volume: Arc<Mutex<f32>>,
}

impl AudioEngine {
    pub fn from_config(config: &StreamConfig) -> Self {
        let device = cpal::default_host()
            .default_output_device()
            .expect("Failed to find output device");

        let plays = Arc::new(Mutex::new(Vec::new()));
        let volume = Arc::new(Mutex::new(1.0));

        let stream = Self::build_stream(&device, &config, plays.clone(), volume.clone());
        stream.play().expect("Failed to play stream");

        Self {
            _stream: stream,
            config: config.clone(),
            cache: Mutex::new(HashMap::new()),
            plays,
            volume,
        }
    }

    pub fn from_parameters(channels: u16, sample_rate: u32, buffer_size: u32) -> Self {
        Self::from_config(&StreamConfig {
            channels,
            sample_rate,
            buffer_size: Fixed(buffer_size),
        })
    }

    pub fn play(&mut self, path: &Path) {
        let mut cache = self.cache.lock().expect("Failed to aquire cache mutex");
        let path_buf: PathBuf = path.into();
        let audio = match cache.get(&path_buf) {
            Some(cached) => cached,
            None => {
                let generated = Arc::new(load_audio(
                    path,
                    self.config.sample_rate,
                    self.config.channels,
                ));
                cache.insert(path_buf, generated.clone());
                &generated.clone()
            }
        };
        self.plays
            .lock()
            .expect("Failed to aquire plays mutex")
            .push(Play::new(audio.clone()));
    }

    pub fn stop_all(&mut self) {
        self.plays
            .lock()
            .expect("Failed to aquire plays mutex")
            .clear();
    }

    pub fn get_volume(&mut self) -> f32 {
        *self.volume.lock().expect("Failed to aquire volume mutex")
    }

    pub fn set_volume(&mut self, volume: f32) {
        *self.volume.lock().expect("Failed to aquire volume mutex") = volume;
    }

    fn build_stream(
        device: &Device,
        config: &StreamConfig,
        plays: Arc<Mutex<Vec<Play>>>,
        volume: Arc<Mutex<f32>>,
    ) -> cpal::Stream {
        let channels = config.channels as usize;

        device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut plays_guard = plays.lock().expect("Failed to aquire plays mutex");
                    let volume = *volume.lock().expect("Failed to aquire volume mutex");
                    data.fill(0.0f32);
                    for frame in data.chunks_mut(channels) {
                        plays_guard.retain(|a| a.is_playing());

                        let mut cur = vec![0.0f32; channels];
                        for play in plays_guard.iter_mut() {
                            play.advance(cur.as_mut_slice());
                            for i in 0..channels {
                                frame[i] += cur[i] * volume;
                            }
                        }
                    }
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .expect("Failed to build audio stream")
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::from_parameters(2, 48000, 4096)
    }
}
