use std::io;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::{build_output_stream, bytes_to_f32, stream_config};
use crate::microphone::setup_virtual_microphone;
use crate::net;
use crate::{CHUNK_SAMPLES, DISCOVERY_PORT, KEEPALIVE_TIMEOUT_MS, new_sample_buffer};

pub fn client(server_ip: &str, discover: bool) -> io::Result<()> {
    let server_addr = if discover {
        println!("Searching for server...");
        let discovery_socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT))?;
        discovery_socket.set_read_timeout(Some(Duration::from_secs(30)))?;
        let (name, addr) = net::wait_for_discovery(&discovery_socket)?;
        println!("Discovered server '{}' at {}", name, addr);
        addr.to_string()
    } else {
        server_ip.to_string()
    };

    setup_virtual_microphone();
    let host = cpal::default_host();
    let device = host
        .output_devices()
        .expect("Failed to enumerate output devices")
        .find(|d| d.description().unwrap().name() == "slime_sink")
        .expect("slime_sink output device not found");
    println!("Output device: {}", device);

    let supported_config = device
        .default_output_config()
        .expect("Failed to get default output config");
    let sample_format = supported_config.sample_format();
    println!("Device format: {:?}", sample_format);

    let config = stream_config();

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    net::send_hello(&socket, &server_addr)?;
    println!("Sent handshake to {}", server_addr);

    let _session_id = net::wait_for_ready(&socket)?;
    println!("Connected! Receiving audio...");

    let buffer = new_sample_buffer();
    let on_error = |err: cpal::Error| eprintln!("Output stream error: {}", err);

    let output_buffer = buffer.clone();
    let stream = build_output_stream(
        &device,
        &config,
        sample_format,
        move |len: usize| {
            let mut buf = output_buffer.lock().unwrap();
            (0..len).map(|_| buf.pop_front().unwrap_or(0.0)).collect()
        },
        on_error,
    );

    stream.play().expect("Failed to start output stream");

    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut recv_buf = vec![0u8; CHUNK_SAMPLES * 4 + 12];
    let mut last_rtp_received = Instant::now();

    loop {
        if last_rtp_received.elapsed() > Duration::from_millis(KEEPALIVE_TIMEOUT_MS) {
            eprintln!("Connection lost: no packets received for {}s", KEEPALIVE_TIMEOUT_MS / 1000);
            break;
        }

        match socket.recv_from(&mut recv_buf) {
            Ok((amt, _)) => {
                match net::parse_message(&recv_buf[..amt]) {
                    Ok(net::Message::Rtp { payload, .. }) => {
                        last_rtp_received = Instant::now();
                        let samples = bytes_to_f32(&payload);
                        buffer.lock().unwrap().extend(samples.iter());
                    }
                    Ok(net::Message::Bye { reason }) => {
                        println!("Server sent BYE: {}", reason);
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Invalid packet: {}", e);
                    }
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => continue,
            Err(e) => {
                eprintln!("Receive error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
