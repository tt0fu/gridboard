use rubato::{
    Fft, FixedSync, Resampler,
    audioadapter::{Adapter, AdapterIterators, AdapterMut},
    audioadapter_buffers::{direct::InterleavedSlice, owned::InterleavedOwned},
};
use std::path::Path;
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

pub fn load_audio(
    path: &Path,
    target_sample_rate: u32,
    target_channels: u16,
) -> InterleavedOwned<f32> {
    let (samples, source_rate) = decode_file(path);

    resample_and_mix(
        samples,
        source_rate,
        target_sample_rate,
        target_channels as usize,
    )
}

// (samples, source sample rate)
fn decode_file(path: &Path) -> (InterleavedOwned<f32>, u32) {
    let file = std::fs::File::open(path).expect("Failed to open file");
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();

    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();
    let decoder_opts: DecoderOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .unwrap();

    let mut format = probed.format;

    let track = format.default_track().unwrap();

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .unwrap();

    let track_id = track.id;

    let mut samples: Vec<f32> = Vec::new();
    let mut channels = 0;
    let mut rate = 0;

    loop {
        let Ok(packet) = format.next_packet() else {
            break;
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(buffer) => {
                let spec = buffer.spec();
                channels = spec.channels.count();
                rate = spec.rate;
                let mut sample_buf = SampleBuffer::<f32>::new(buffer.capacity() as u64, *spec);
                sample_buf.copy_interleaved_ref(buffer);
                samples.extend_from_slice(sample_buf.samples());
            }
            Err(Error::DecodeError(_)) => (),
            Err(_) => break,
        }
    }

    let frame_count = samples.len() / channels;
    (
        InterleavedOwned::new_from(samples, channels, frame_count).unwrap(),
        rate,
    )
}

fn resample_and_mix(
    source: InterleavedOwned<f32>,
    source_rate: u32,
    target_rate: u32,
    target_channels: usize,
) -> InterleavedOwned<f32> {
    let resampled = {
        if source_rate != target_rate {
            let ratio = target_rate as f64 / source_rate as f64;
            let estimate_samples = (source.frames() as f64 * ratio) as usize;
            let mut outdata = vec![0.0; 2 * source.channels() * estimate_samples];
            let outdata_capacity = outdata.len() / source.channels();
            let mut output_adapter =
                InterleavedSlice::new_mut(&mut outdata, source.channels(), outdata_capacity)
                    .unwrap();
            Fft::<f32>::new(
                source_rate as usize,
                target_rate as usize,
                2048,
                1,
                source.channels(),
                FixedSync::Both,
            )
            .expect("Failed to create resampler")
            .process_all_into_buffer(&source, &mut output_adapter, source.frames(), None)
            .expect("Failed to resample track");
            InterleavedOwned::new_from(outdata, source.channels(), estimate_samples + 1000).unwrap()
        } else {
            source
        }
    };

    mix_channels(resampled, target_channels)
}

fn mix_channels(input: InterleavedOwned<f32>, target_count: usize) -> InterleavedOwned<f32> {
    let source_count = input.channels();
    let frames = input.frames();
    if source_count == target_count {
        return input;
    }

    let mut output = InterleavedOwned::new(0.0f32, target_count, frames);

    match (source_count, target_count) {
        (1, 2) => {
            output.copy_from_other_to_channel(&input, 0, 0, 0, 0, frames);
            output.copy_from_other_to_channel(&input, 0, 1, 0, 0, frames);
        }
        (2, 1) => {
            for (i, mut frame) in input.iter_frames().enumerate() {
                let first = frame.next().unwrap();
                let second = frame.next().unwrap();
                output.write_sample(0, i, &((first + second) * 0.5));
            }
        }
        _ => {
            for channel in 0..(source_count.min(target_count)) {
                output.copy_from_other_to_channel(&input, channel, channel, 0, 0, frames);
            }
        }
    }
    output
}
