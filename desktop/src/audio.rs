use cpal::traits::DeviceTrait;
use cpal::{SampleFormat, StreamConfig};

use crate::{CHANNELS, SAMPLE_RATE};

pub fn stream_config() -> StreamConfig {
    StreamConfig {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        buffer_size: cpal::BufferSize::Default,
    }
}

pub fn f32_to_bytes(samples: &[f32]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_ne_bytes()).collect()
}

pub fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / i16::MAX as f32)
        .collect()
}

fn u16_to_f32(samples: &[u16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
        .collect()
}

fn i32_to_f32(samples: &[i32]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / i32::MAX as f32)
        .collect()
}

pub fn build_input_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    on_samples: impl FnMut(&[f32]) + Send + 'static,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> cpal::Stream {
    let mut on_samples = on_samples;
    match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                *config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| on_samples(data),
                on_error,
                None,
            )
            .expect("Failed to build input stream"),
        SampleFormat::I16 => device
            .build_input_stream(
                *config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let samples = i16_to_f32(data);
                    on_samples(&samples);
                },
                on_error,
                None,
            )
            .expect("Failed to build input stream"),
        SampleFormat::U16 => device
            .build_input_stream(
                *config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let samples = u16_to_f32(data);
                    on_samples(&samples);
                },
                on_error,
                None,
            )
            .expect("Failed to build input stream"),
        SampleFormat::I32 => device
            .build_input_stream(
                *config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let samples = i32_to_f32(data);
                    on_samples(&samples);
                },
                on_error,
                None,
            )
            .expect("Failed to build input stream"),
        _ => panic!("Unsupported sample format: {:?}", sample_format),
    }
}

pub fn build_output_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    get_samples: impl FnMut(usize) -> Vec<f32> + Send + 'static,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> cpal::Stream {
    let mut get_samples = get_samples;
    match sample_format {
        SampleFormat::F32 => device
            .build_output_stream(
                *config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let len = data.len();
                    for (out, sample) in data.iter_mut().zip(get_samples(len)) {
                        *out = sample;
                    }
                },
                on_error,
                None,
            )
            .expect("Failed to build output stream"),
        SampleFormat::I16 => device
            .build_output_stream(
                *config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let len = data.len();
                    for (out, sample) in data.iter_mut().zip(get_samples(len)) {
                        *out = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    }
                },
                on_error,
                None,
            )
            .expect("Failed to build output stream"),
        SampleFormat::U16 => device
            .build_output_stream(
                *config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let len = data.len();
                    for (out, sample) in data.iter_mut().zip(get_samples(len)) {
                        let normalized = (sample + 1.0) / 2.0;
                        *out = (normalized.clamp(0.0, 1.0) * u16::MAX as f32) as u16;
                    }
                },
                on_error,
                None,
            )
            .expect("Failed to build output stream"),
        SampleFormat::I32 => device
            .build_output_stream(
                *config,
                move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                    let len = data.len();
                    for (out, sample) in data.iter_mut().zip(get_samples(len)) {
                        *out = (sample.clamp(-1.0, 1.0) * i32::MAX as f32) as i32;
                    }
                },
                on_error,
                None,
            )
            .expect("Failed to build output stream"),
        _ => panic!("Unsupported sample format: {:?}", sample_format),
    }
}
