use crate::{config::Config, stream_audio::StreamingAudio};
use anyhow::{Result, anyhow};
use cpal::{
    BufferSize::Fixed,
    Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{Receiver, TryRecvError, sync_channel},
    },
};

const PLAY_QUEUE_CHUNKS: usize = 8;

struct Play {
    chunks: Receiver<Vec<f32>>,
    current: Vec<f32>,
    position: usize,
    channels: usize,
    finished: bool,
}

impl Play {
    fn new(chunks: Receiver<Vec<f32>>, channels: usize) -> Self {
        Self {
            chunks,
            current: Vec::new(),
            position: 0,
            channels,
            finished: false,
        }
    }

    fn is_playing(&self) -> bool {
        !self.finished
    }

    fn advance(&mut self, output: &mut [f32], volume: f32) {
        while self.position >= self.current.len() {
            match self.chunks.try_recv() {
                Ok(chunk) => {
                    self.current = chunk;
                    self.position = 0;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    return;
                }
            }
        }

        let frame_end = (self.position + self.channels).min(self.current.len());
        for (output, sample) in output
            .iter_mut()
            .zip(&self.current[self.position..frame_end])
        {
            *output += sample * volume;
        }
        self.position = frame_end;
    }
}

pub struct AudioEngine {
    _stream: Stream,
    config: StreamConfig,
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
            config,
            {
                let plays = plays.clone();
                let volume = volume.clone();
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut plays = plays.lock().expect("Failed to aquire plays mutex");
                    let volume = *volume.read().expect("Failed to aquire volume lock");
                    data.fill(0.0);

                    for frame in data.chunks_mut(channels) {
                        plays.retain(Play::is_playing);
                        for play in plays.iter_mut() {
                            play.advance(frame, volume);
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
        let path = PathBuf::from(path);
        let sample_rate = self.config.sample_rate;
        let channels = self.config.channels;
        let (sender, receiver) = sync_channel(PLAY_QUEUE_CHUNKS);

        std::thread::Builder::new()
            .name("gridboard-audio".to_string())
            .spawn(move || {
                let mut audio = match StreamingAudio::open(&path, sample_rate, channels) {
                    Ok(audio) => audio,
                    Err(err) => {
                        eprintln!("Failed to prepare audio {}: {err}", path.display());
                        return;
                    }
                };
                while let Some(chunk) = audio.next_chunk() {
                    if sender.send(chunk).is_err() {
                        break;
                    }
                }
            })?;

        self.plays
            .lock()
            .expect("Failed to aquire plays mutex")
            .push(Play::new(receiver, channels as usize));
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
