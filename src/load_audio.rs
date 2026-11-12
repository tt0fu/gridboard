use anyhow::{Result, anyhow};
use audioadapter::{Adapter, AdapterIterators, AdapterMut};
use rubato::{Fft, FixedSync, Resampler, audioadapter_buffers::owned::InterleavedOwned};
use std::path::Path;
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

pub fn load_audio(
    path: &Path,
    target_sample_rate: u32,
    target_channels: u16,
) -> Result<InterleavedOwned<f32>> {
    let (samples, source_rate) = decode_file(path)?;
    let resampled = resample(samples, source_rate, target_sample_rate)?;
    Ok(mix_channels(resampled, target_channels as usize))
}

// (samples, source sample rate)
fn decode_file(path: &Path) -> Result<(InterleavedOwned<f32>, u32)> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let probed = symphonia::default::get_probe().format(
        &Hint::new(),
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or(anyhow!("Failed to get the default track of the file"))?;
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let track_id = track.id;

    let mut samples = Vec::new();
    let mut channels = 0;
    let mut rate = 0;

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buffer) => {
                let spec = buffer.spec();
                channels = spec.channels.count();
                rate = spec.rate;

                let mut sample_buf = SampleBuffer::<f32>::new(buffer.frames() as u64, *spec);
                sample_buf.copy_interleaved_ref(buffer);
                samples.extend_from_slice(sample_buf.samples());
            }
            Err(Error::DecodeError(_)) => (),
            Err(_) => break,
        }
    }

    if channels == 0 || rate == 0 {
        return Err(anyhow!("No audio found in {}", path.display()));
    }

    let frame_count = samples.len() / channels;
    Ok((
        InterleavedOwned::new_from(samples, channels, frame_count)?,
        rate,
    ))
}

fn resample(
    source: InterleavedOwned<f32>,
    source_rate: u32,
    target_rate: u32,
) -> Result<InterleavedOwned<f32>> {
    if source_rate == target_rate {
        return Ok(source);
    }

    let mut resampler = Fft::<f32>::new(
        source_rate as usize,
        target_rate as usize,
        2048,
        source.channels(),
        FixedSync::Both,
    )?;

    Ok(resampler.process_all(&source, source.frames(), None)?)
}

fn mix_channels(input: InterleavedOwned<f32>, target_channels: usize) -> InterleavedOwned<f32> {
    let source_channels = input.channels();
    if source_channels == target_channels {
        return input;
    }

    let frames = input.frames();
    let mut output = InterleavedOwned::new(0.0f32, target_channels, frames);

    for (frame_index, mut frame) in input.iter_frames().enumerate() {
        match (source_channels, target_channels) {
            (1, target) => {
                let sample = frame.next().unwrap_or(0.0);
                for channel in 0..target {
                    output.write_sample(channel, frame_index, &sample);
                }
            }
            (_, 1) => {
                let sample = frame.sum::<f32>() / source_channels as f32;
                output.write_sample(0, frame_index, &sample);
            }
            _ => {
                for (channel, sample) in frame.take(target_channels).enumerate() {
                    output.write_sample(channel, frame_index, &sample);
                }
            }
        }
    }
    output
}
