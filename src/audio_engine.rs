use crate::audio::load_audio;
use audioadapter::Adapter;
use cpal::{
    SupportedStreamConfig,
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
    _stream: cpal::Stream,
    config: SupportedStreamConfig,
    cache: Mutex<HashMap<PathBuf, Arc<InterleavedOwned<f32>>>>,
    plays: Arc<Mutex<Vec<Play>>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let device = cpal::default_host()
            .default_output_device()
            .expect("Failed to find output device");
        let config = device
            .default_output_config()
            .expect("Failed to find default config of output device");

        let plays = Arc::new(Mutex::new(Vec::new()));
        let sources_clone = Arc::clone(&plays);

        let stream = Self::build_stream(&device, &config, sources_clone);
        stream.play().expect("Failed to play stream");

        Self {
            _stream: stream,
            config,
            cache: Mutex::new(HashMap::new()),
            plays,
        }
    }

    pub fn play(&mut self, path: &Path) {
        let mut cache = self.cache.lock().expect("Failed to aquire cache mutex");
        let path_buf: PathBuf = path.into();
        let audio = match cache.get(&path_buf) {
            Some(cached) => cached,
            None => {
                let generated = Arc::new(load_audio(
                    path,
                    self.config.sample_rate(),
                    self.config.channels(),
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

    fn build_stream(
        device: &cpal::Device,
        config: &cpal::SupportedStreamConfig,
        plays: Arc<Mutex<Vec<Play>>>,
    ) -> cpal::Stream {
        match config.sample_format() {
            cpal::SampleFormat::I8 => Self::build_stream_typed::<i8>(device, config, plays),
            cpal::SampleFormat::I16 => Self::build_stream_typed::<i16>(device, config, plays),
            cpal::SampleFormat::I24 => Self::build_stream_typed::<cpal::I24>(device, config, plays),
            cpal::SampleFormat::I32 => Self::build_stream_typed::<i32>(device, config, plays),
            cpal::SampleFormat::I64 => Self::build_stream_typed::<i64>(device, config, plays),
            cpal::SampleFormat::U8 => Self::build_stream_typed::<u8>(device, config, plays),
            cpal::SampleFormat::U16 => Self::build_stream_typed::<u16>(device, config, plays),
            cpal::SampleFormat::U24 => Self::build_stream_typed::<cpal::U24>(device, config, plays),
            cpal::SampleFormat::U32 => Self::build_stream_typed::<u32>(device, config, plays),
            cpal::SampleFormat::U64 => Self::build_stream_typed::<u64>(device, config, plays),
            cpal::SampleFormat::F32 => Self::build_stream_typed::<f32>(device, config, plays),
            cpal::SampleFormat::F64 => Self::build_stream_typed::<f64>(device, config, plays),
            sample_format => panic!("Unsupported sample format '{sample_format}'"),
        }
    }

    fn build_stream_typed<T>(
        device: &cpal::Device,
        config: &cpal::SupportedStreamConfig,
        plays: Arc<Mutex<Vec<Play>>>,
    ) -> cpal::Stream
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels() as usize;

        device
            .build_output_stream(
                &config.config(),
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    let mut plays_guard = plays.lock().expect("Failed to aquire plays mutex");
                    data.fill(T::from_sample(0.0f32));
                    for frame in data.chunks_mut(channels) {
                        plays_guard.retain(|a| a.is_playing());

                        let mut sum = vec![0.0f32; channels];
                        let mut cur = vec![0.0f32; channels];
                        for play in plays_guard.iter_mut() {
                            play.advance(cur.as_mut_slice());
                            for i in 0..channels {
                                sum[i] += cur[i];
                            }
                        }

                        for i in 0..channels {
                            frame[i] = T::from_sample(sum[i]);
                        }
                    }
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .expect("Failed to build audio stream")
    }
}
