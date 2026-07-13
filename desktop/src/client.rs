use std::io;
use std::net::UdpSocket;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::{build_output_stream, bytes_to_f32, stream_config};
use crate::net;
use crate::virtual_microphone;
use crate::{CHUNK_SAMPLES, SERVER_ADDR, new_sample_buffer};

// slime_sink

pub fn client() -> io::Result<()> {
    virtual_microphone::setup_virtual_microphone();
    let host = cpal::default_host();

    let devices = host.output_devices().expect("Failed to get output devices");

    for device in devices {
        println!("Device Name: {}", device.description().unwrap().name());
    }

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

    let socket = UdpSocket::bind("127.0.0.1:0")?;
    net::send_hello(&socket, SERVER_ADDR)?;
    println!("Sent handshake to {}", SERVER_ADDR);

    net::wait_for_ready(&socket)?;
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

    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    let mut recv_buf = vec![0u8; CHUNK_SAMPLES * 4];

    loop {
        match socket.recv_from(&mut recv_buf) {
            Ok((amt, _)) => {
                let samples = bytes_to_f32(&recv_buf[..amt]);
                buffer.lock().unwrap().extend(samples.iter());
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
