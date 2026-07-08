use std::collections::VecDeque;
use std::env;
use std::io;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

const SAMPLE_RATE: u32 = 16000;
const CHANNELS: u16 = 1;
const CHUNK_SAMPLES: usize = 512;
const SERVER_ADDR: &str = "127.0.0.1:34254";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: desktop <server|client>");
        return;
    }
    match args[1].as_str() {
        "server" => {
            if let Err(e) = server() {
                eprintln!("Server error: {}", e);
            }
        }
        "client" => {
            if let Err(e) = client() {
                eprintln!("Client error: {}", e);
            }
        }
        other => eprintln!("Unknown argument: {}. Use 'server' or 'client'.", other),
    }
}

type SampleBuffer = Arc<Mutex<VecDeque<f32>>>;

fn stream_config() -> StreamConfig {
    StreamConfig {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        buffer_size: cpal::BufferSize::Default,
    }
}

fn f32_to_bytes(samples: &[f32]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_ne_bytes()).collect()
}

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn server() -> io::Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    println!("Input device: {}", device);

    let supported_config = device
        .default_input_config()
        .expect("Failed to get default input config");
    let sample_format = supported_config.sample_format();
    println!("Device format: {:?}", sample_format);

    let config = stream_config();

    let socket = UdpSocket::bind(SERVER_ADDR)?;
    println!("Server listening on {}", SERVER_ADDR);
    println!("Waiting for client...");

    let mut buf = [0u8; 64];
    let (amt, client_addr) = socket.recv_from(&mut buf)?;
    if &buf[..amt] != b"HELLO" {
        eprintln!("Unexpected handshake from {}", client_addr);
        return Ok(());
    }
    println!("Client connected: {}", client_addr);
    socket.send_to(b"READY", client_addr)?;

    let audio_buf: SampleBuffer = Arc::new(Mutex::new(VecDeque::new()));
    let err_cb = |err: cpal::Error| eprintln!("Input stream error: {}", err);

    let stream = match sample_format {
        SampleFormat::F32 => {
            let ab = audio_buf.clone();
            device.build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    ab.lock().unwrap().extend(data.iter());
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I16 => {
            let ab = audio_buf.clone();
            device.build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<f32> = data
                        .iter()
                        .map(|&s| s as f32 / i16::MAX as f32)
                        .collect();
                    ab.lock().unwrap().extend(samples.iter());
                },
                err_cb,
                None,
            )
        }
        SampleFormat::U16 => {
            let ab = audio_buf.clone();
            device.build_input_stream(
                config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<f32> = data
                        .iter()
                        .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                        .collect();
                    ab.lock().unwrap().extend(samples.iter());
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I32 => {
            let ab = audio_buf.clone();
            device.build_input_stream(
                config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<f32> = data
                        .iter()
                        .map(|&s| s as f32 / i32::MAX as f32)
                        .collect();
                    ab.lock().unwrap().extend(samples.iter());
                },
                err_cb,
                None,
            )
        }
        _ => panic!("Unsupported sample format: {:?}", sample_format),
    }
    .expect("Failed to build input stream");

    stream.play().expect("Failed to start input stream");
    println!("Streaming audio to {}...", client_addr);

    loop {
        thread::sleep(Duration::from_millis(10));
        let samples: Vec<f32> = {
            let mut buf = audio_buf.lock().unwrap();
            let n = buf.len().min(CHUNK_SAMPLES);
            buf.drain(..n).collect()
        };
        if !samples.is_empty() {
            let bytes = f32_to_bytes(&samples);
            socket.send_to(&bytes, client_addr)?;
        }
    }
}

fn client() -> io::Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");
    println!("Output device: {}", device);

    let supported_config = device
        .default_output_config()
        .expect("Failed to get default output config");
    let sample_format = supported_config.sample_format();
    println!("Device format: {:?}", sample_format);

    let config = stream_config();

    let socket = UdpSocket::bind("127.0.0.1:0")?;
    socket.send_to(b"HELLO", SERVER_ADDR)?;
    println!("Sent handshake to {}", SERVER_ADDR);

    let mut buf = [0u8; 64];
    let (amt, _) = socket.recv_from(&mut buf)?;
    if &buf[..amt] != b"READY" {
        eprintln!("Server not ready");
        return Ok(());
    }
    println!("Connected! Receiving audio...");

    let audio_buf: SampleBuffer = Arc::new(Mutex::new(VecDeque::new()));
    let err_cb = |err: cpal::Error| eprintln!("Output stream error: {}", err);

    let stream = match sample_format {
        SampleFormat::F32 => {
            let ab = audio_buf.clone();
            device.build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut buf = ab.lock().unwrap();
                    for sample in data.iter_mut() {
                        *sample = buf.pop_front().unwrap_or(0.0);
                    }
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I16 => {
            let ab = audio_buf.clone();
            device.build_output_stream(
                config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let mut buf = ab.lock().unwrap();
                    for sample in data.iter_mut() {
                        let f = buf.pop_front().unwrap_or(0.0);
                        *sample = (f.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    }
                },
                err_cb,
                None,
            )
        }
        SampleFormat::U16 => {
            let ab = audio_buf.clone();
            device.build_output_stream(
                config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let mut buf = ab.lock().unwrap();
                    for sample in data.iter_mut() {
                        let f = buf.pop_front().unwrap_or(0.0);
                        let norm = (f + 1.0) / 2.0;
                        *sample = (norm.clamp(0.0, 1.0) * u16::MAX as f32) as u16;
                    }
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I32 => {
            let ab = audio_buf.clone();
            device.build_output_stream(
                config,
                move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                    let mut buf = ab.lock().unwrap();
                    for sample in data.iter_mut() {
                        let f = buf.pop_front().unwrap_or(0.0);
                        *sample = (f.clamp(-1.0, 1.0) * i32::MAX as f32) as i32;
                    }
                },
                err_cb,
                None,
            )
        }
        _ => panic!("Unsupported sample format: {:?}", sample_format),
    }
    .expect("Failed to build output stream");

    stream.play().expect("Failed to start output stream");

    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    let mut recv_buf = vec![0u8; CHUNK_SAMPLES * 4];

    loop {
        match socket.recv_from(&mut recv_buf) {
            Ok((amt, _)) => {
                let samples = bytes_to_f32(&recv_buf[..amt]);
                audio_buf.lock().unwrap().extend(samples.iter());
            }
            Err(ref e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) => {
                eprintln!("Receive error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
