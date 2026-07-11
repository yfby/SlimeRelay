use std::io;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::{build_input_stream, f32_to_bytes, stream_config};
use crate::net;
use crate::{new_sample_buffer, CHUNK_SAMPLES, SERVER_ADDR};

pub fn server() -> io::Result<()> {
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

    let client_addr = net::wait_for_hello(&socket)?;
    println!("Client connected: {}", client_addr);
    net::send_ready(&socket, client_addr)?;

    let buffer = new_sample_buffer();
    let on_error = |err: cpal::Error| eprintln!("Input stream error: {}", err);

    let input_buffer = buffer.clone();
    let stream = build_input_stream(
        &device,
        &config,
        sample_format,
        move |data: &[f32]| {
            input_buffer.lock().unwrap().extend(data.iter());
        },
        on_error,
    );

    stream.play().expect("Failed to start input stream");
    println!("Streaming audio to {}...", client_addr);

    loop {
        thread::sleep(Duration::from_millis(10));

        let chunk: Vec<f32> = {
            let mut buf = buffer.lock().unwrap();
            let n = buf.len().min(CHUNK_SAMPLES);
            buf.drain(..n).collect()
        };

        if chunk.is_empty() {
            continue;
        }

        let bytes = f32_to_bytes(&chunk);
        socket.send_to(&bytes, client_addr)?;
    }
}
