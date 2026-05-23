use crate::{config::Config, load_audio::load_audio};
use anyhow::{Result, anyhow};
use audioadapter::Adapter;
use cpal::{
    BufferSize::Fixed,
    Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rubato::audioadapter_buffers::owned::InterleavedOwned;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
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
    cache: HashMap<PathBuf, Arc<InterleavedOwned<f32>>>,
    plays: Arc<Mutex<Vec<Play>>>,
    volume: Arc<RwLock<f32>>,
}

impl AudioEngine {
    pub fn from_stream_config(config: &StreamConfig) -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(anyhow!("Failed to get the default output device"))?;

        let plays: Arc<Mutex<Vec<Play>>> = Arc::new(Mutex::new(Vec::new()));
        let volume = Arc::new(RwLock::new(1.0));

        let channels = config.channels as usize;

        let stream = device.build_output_stream(
            &config,
            {
                let plays_clone = plays.clone();
                let volume_clone = volume.clone();
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut plays_guard = plays_clone.lock().expect("Failed to aquire plays mutex");
                    let volume = *volume_clone.read().expect("Failed to aquire volume lock");
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
                }
            },
            |err| eprintln!("Audio stream error: {err}"),
            None,
        )?;
        stream.play()?;

        Ok(Self {
            _stream: stream,
            config: config.clone(),
            cache: HashMap::new(),
            plays,
            volume,
        })
    }

    pub fn from_parameters(channels: u16, sample_rate: u32, buffer_size: u32) -> Result<Self> {
        Self::from_stream_config(&StreamConfig {
            channels,
            sample_rate,
            buffer_size: Fixed(buffer_size),
        })
    }

    pub fn from_config(config: &Config) -> Result<Self> {
        Self::from_parameters(config.channels, config.sample_rate, config.buffer_size)
    }

    pub fn play(&mut self, path: &Path) -> Result<()> {
        let path_buf: PathBuf = path.into();
        let audio = match self.cache.get(&path_buf) {
            Some(cached) => cached,
            None => {
                let generated = Arc::new(load_audio(
                    path,
                    self.config.sample_rate,
                    self.config.channels,
                )?);
                self.cache.insert(path_buf, generated.clone());
                &generated.clone()
            }
        };
        self.plays
            .lock()
            .expect("Failed to aquire plays mutex")
            .push(Play::new(audio.clone()));
        Ok(())
    }

    pub fn stop_all(&mut self) {
        self.plays
            .lock()
            .expect("Failed to aquire plays mutex")
            .clear();
    }

    pub fn get_volume(&mut self) -> f32 {
        *self.volume.read().expect("Failed to aquire volume lock")
    }

    pub fn set_volume(&mut self, volume: f32) {
        *self.volume.write().expect("Failed to aquire volume lock") = volume;
    }
}
