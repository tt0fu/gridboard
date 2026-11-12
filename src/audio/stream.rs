use anyhow::{Result, anyhow};
use audioadapter::{Adapter, AdapterIterators};
use rubato::{Fft, FixedSync, Indexing, Resampler, audioadapter_buffers::direct::InterleavedSlice};
use std::{fs::File, path::Path};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{Decoder, DecoderOptions},
    errors::Error,
    formats::{FormatOptions, FormatReader},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

const RESAMPLER_CHUNK_SIZE: usize = 1024;

pub struct StreamingAudio {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    source_rate: u32,
    source_channels: usize,
    target_channels: usize,
    resampler: Option<Fft<f32>>,
    decoded: Vec<f32>,
    total_decoded_frames: usize,
    frames_to_trim: usize,
    produced_frames: usize,
    expected_output_frames: Option<usize>,
    eof: bool,
    finished: bool,
}

impl StreamingAudio {
    pub fn open(path: &Path, target_sample_rate: u32, target_channels: u16) -> Result<Self> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let probed = symphonia::default::get_probe().format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let format = probed.format;
        let (track_id, source_rate, source_channels, decoder) = {
            let track = format
                .default_track()
                .ok_or(anyhow!("Failed to get the default track of the file"))?;
            let source_rate = track
                .codec_params
                .sample_rate
                .ok_or(anyhow!("Failed to get the sample rate of the file"))?;
            let source_channels = track
                .codec_params
                .channels
                .map(|channels| channels.count())
                .ok_or(anyhow!("Failed to get the channel count of the file"))?;
            let decoder = symphonia::default::get_codecs()
                .make(&track.codec_params, &DecoderOptions::default())?;
            (track.id, source_rate, source_channels, decoder)
        };

        if source_rate == 0 || source_channels == 0 {
            return Err(anyhow!("Invalid audio format in {}", path.display()));
        }
        if target_sample_rate == 0 || target_channels == 0 {
            return Err(anyhow!("Invalid target audio format"));
        }

        let resampler = if source_rate != target_sample_rate {
            Some(Fft::<f32>::new(
                source_rate as usize,
                target_sample_rate as usize,
                RESAMPLER_CHUNK_SIZE,
                source_channels,
                FixedSync::Input,
            )?)
        } else {
            None
        };
        let frames_to_trim = resampler
            .as_ref()
            .map(Resampler::output_delay)
            .unwrap_or_default();

        Ok(Self {
            format,
            decoder,
            track_id,
            source_rate,
            source_channels,
            target_channels: target_channels as usize,
            resampler,
            decoded: Vec::new(),
            total_decoded_frames: 0,
            frames_to_trim,
            produced_frames: 0,
            expected_output_frames: None,
            eof: false,
            finished: false,
        })
    }

    pub fn next_chunk(&mut self) -> Option<Vec<f32>> {
        if self.finished {
            return None;
        }

        let chunk = if self.resampler.is_some() {
            self.next_resampled_chunk()
        } else {
            self.next_direct_chunk()
        };

        if chunk.is_none() {
            self.finished = true;
        }
        chunk
    }

    fn next_direct_chunk(&mut self) -> Option<Vec<f32>> {
        self.decode_until(1);
        if self.decoded.is_empty() {
            return None;
        }

        let frames = self.decoded.len() / self.source_channels;
        let input = InterleavedSlice::new(&self.decoded, self.source_channels, frames)
            .expect("Invalid slice length for InterleavedSlice");
        let mut chunk = Vec::with_capacity(frames * self.target_channels);

        for frame in input.iter_frames() {
            push_mixed_frame(
                &mut chunk,
                frame,
                self.source_channels,
                self.target_channels,
            );
        }
        self.decoded.clear();
        Some(chunk)
    }

    fn next_resampled_chunk(&mut self) -> Option<Vec<f32>> {
        loop {
            let input_frames = self
                .resampler
                .as_ref()
                .expect("Resampler is not present")
                .input_frames_next();
            self.decode_until(input_frames);

            let available_frames = self.decoded.len() / self.source_channels;
            if available_frames >= input_frames {
                if let Some(chunk) = self.process_chunk(input_frames, false) {
                    return Some(chunk);
                }
                if self.finished {
                    return None;
                }
                continue;
            }

            if !self.eof {
                continue;
            }

            let expected_output_frames = *self.expected_output_frames.get_or_insert_with(|| {
                let ratio = self
                    .resampler
                    .as_ref()
                    .expect("Resampler is not present")
                    .resample_ratio();
                (ratio * self.total_decoded_frames as f64).ceil() as usize
            });

            if available_frames > 0 {
                if let Some(chunk) = self.process_chunk(available_frames, true) {
                    return Some(chunk);
                }
                if self.finished {
                    return None;
                }
                continue;
            }

            if self.produced_frames < expected_output_frames {
                if let Some(chunk) = self.process_chunk(0, true) {
                    return Some(chunk);
                }
                if self.finished {
                    return None;
                }
                continue;
            }
            return None;
        }
    }

    fn process_chunk(&mut self, input_frames: usize, partial: bool) -> Option<Vec<f32>> {
        let input_samples = input_frames * self.source_channels;
        let indexing = if partial {
            Indexing::new().partial_len(input_frames)
        } else {
            Indexing::new()
        };

        let output = {
            let input = InterleavedSlice::new(
                &self.decoded[..input_samples],
                self.source_channels,
                input_frames,
            )
            .expect("Invalid slice length for InterleavedSlice");
            let resampler = self.resampler.as_mut().expect("Resampler is not present");

            match resampler.process(&input, Some(&indexing)) {
                Ok(output) => output,
                Err(err) => {
                    eprintln!("Resampling error: {err}");
                    self.finished = true;
                    return None;
                }
            }
        };

        self.decoded.drain(..input_samples);

        let output_frames = output.frames();
        let trim = self.frames_to_trim.min(output_frames);
        self.frames_to_trim -= trim;

        let mut frames_to_emit = output_frames - trim;
        if let Some(expected) = self.expected_output_frames {
            frames_to_emit = frames_to_emit.min(expected.saturating_sub(self.produced_frames));
        }
        if frames_to_emit == 0 {
            return None;
        }

        let mut chunk = Vec::with_capacity(frames_to_emit * self.target_channels);
        for frame in output.iter_frames().skip(trim).take(frames_to_emit) {
            push_mixed_frame(
                &mut chunk,
                frame,
                self.source_channels,
                self.target_channels,
            );
        }
        self.produced_frames += frames_to_emit;
        Some(chunk)
    }

    fn decode_until(&mut self, frame_count: usize) {
        while self.decoded.len() / self.source_channels < frame_count && !self.eof {
            self.decode_packet();
        }
    }

    fn decode_packet(&mut self) {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(_) => {
                    self.eof = true;
                    return;
                }
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(buffer) => {
                    let spec = buffer.spec();
                    if spec.rate != self.source_rate
                        || spec.channels.count() != self.source_channels
                    {
                        eprintln!("Audio format changed mid-stream");
                        self.eof = true;
                        return;
                    }

                    let frames = buffer.frames();
                    let mut sample_buf = SampleBuffer::<f32>::new(frames as u64, *spec);
                    sample_buf.copy_interleaved_ref(buffer);
                    self.decoded.extend_from_slice(sample_buf.samples());
                    self.total_decoded_frames += frames;
                    return;
                }
                Err(Error::DecodeError(_)) | Err(Error::IoError(_)) => continue,
                Err(_) => {
                    self.eof = true;
                    return;
                }
            }
        }
    }
}

fn push_mixed_frame(
    output: &mut Vec<f32>,
    mut frame: impl Iterator<Item = f32>,
    source_channels: usize,
    target_channels: usize,
) {
    match (source_channels, target_channels) {
        (1, target) => {
            let sample = frame.next().unwrap_or(0.0);
            output.extend(std::iter::repeat_n(sample, target));
        }
        (_, 1) => {
            output.push(frame.sum::<f32>() / source_channels as f32);
        }
        (_, target) => {
            for _ in 0..target {
                output.push(frame.next().unwrap_or(0.0));
            }
        }
    }
}
