use std::io;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rand::Rng;

use crate::audio::{build_input_stream, f32_to_bytes, stream_config};
use crate::net;
use crate::{CHUNK_SAMPLES, DISCOVERY_PORT, KEEPALIVE_INTERVAL_MS, SERVER_PORT, new_sample_buffer};

pub fn server(server_name: &str) -> io::Result<()> {
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

    let discovery_socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT))?;
    discovery_socket.set_broadcast(true)?;
    println!("Discovery broadcast listening on port {}", DISCOVERY_PORT);

    let server_addr = format!("0.0.0.0:{}", SERVER_PORT);
    let socket = UdpSocket::bind(&server_addr)?;
    println!("Server listening on {}", server_addr);

    let discovery_name = server_name.to_string();
    thread::spawn(move || {
        loop {
            if let Err(e) =
                net::send_discovery_broadcast(&discovery_socket, &discovery_name, SERVER_PORT)
            {
                eprintln!("Discovery broadcast error: {}", e);
            }
            thread::sleep(Duration::from_millis(KEEPALIVE_INTERVAL_MS));
        }
    });

    println!("Waiting for client...");

    let client_addr = net::wait_for_hello(&socket)?;
    println!("Client connected: {}", client_addr);

    let mut session_id = [0u8; 16];
    rand::thread_rng().fill(&mut session_id);
    net::send_ready(&socket, client_addr, &session_id)?;

    let ssrc: u32 = rand::thread_rng().r#gen();
    println!("SSRC: 0x{:08X}", ssrc);

    // audio buffer
    let buffer = new_sample_buffer();
    let input_buffer = buffer.clone();

    let on_error = |err: cpal::Error| eprintln!("Input stream error: {}", err);

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

    let mut sequence: u16 = 0;
    let mut timestamp: u32 = 0;

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
        let rtp_packet = net::build_rtp_packet(sequence, timestamp, ssrc, &bytes);
        socket.send_to(&rtp_packet, client_addr)?;

        sequence = sequence.wrapping_add(1);
        timestamp = timestamp.wrapping_add(CHUNK_SAMPLES as u32);
    }
}
