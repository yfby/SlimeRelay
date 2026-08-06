use std::io;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::{build_output_stream, bytes_to_f32, stream_config};
use crate::microphone::setup_virtual_microphone;
use crate::net;
use crate::{CHUNK_SAMPLES, DISCOVERY_PORT, KEEPALIVE_TIMEOUT_MS, new_sample_buffer};

struct Client {
    socket: UdpSocket,
    audio_buffer: crate::SampleBuffer,
}

impl Client {
    fn new() -> io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            socket,
            audio_buffer: new_sample_buffer(),
        })
    }

    fn discover(&self) -> io::Result<String> {
        println!("Searching for server...");
        let discovery_socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT))?;
        discovery_socket.set_read_timeout(Some(Duration::from_secs(30)))?;
        let (name, addr) = net::wait_for_discovery(&discovery_socket)?;
        println!("Discovered server '{}' at {}", name, addr);
        Ok(addr.to_string())
    }

    fn connect(&self, server_addr: &str) -> io::Result<[u8; 16]> {
        net::send_hello(&self.socket, server_addr)?;
        println!("Sent handshake to {}", server_addr);
        let session_id = net::wait_for_ready(&self.socket)?;
        println!("Connected! Receiving audio...");
        Ok(session_id)
    }

    fn setup_audio(&self) -> cpal::Stream {
        setup_virtual_microphone();

        let host = cpal::default_host();
        let device = host
            .output_devices()
            .expect("Failed to enumerate output devices")
            .find(|d| d.description().unwrap().name() == "slime_sink")
            .expect("slime_sink outputdevice not found");
        println!("Output device: {}", device);

        let supported_config = device
            .default_output_config()
            .expect("Failed to get default output config");
        let sample_format = supported_config.sample_format();
        println!("Device format: {:?}", sample_format);

        let config = stream_config();
        let output_buffer = self.audio_buffer.clone();

        let on_error = |err: cpal::Error| eprintln!("Output stream error: {}", err);

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
        stream
    }

    fn run(&self) -> io::Result<()> {
        self.socket
            .set_read_timeout(Some(Duration::from_millis(100)))?;
        let mut recv_buf = vec![0u8; CHUNK_SAMPLES * 4 + 12];
        let mut last_rtp_received = Instant::now();

        loop {
            if last_rtp_received.elapsed() > Duration::from_millis(KEEPALIVE_TIMEOUT_MS) {
                eprintln!(
                    "Connection lost: no packets received for {}s",
                    KEEPALIVE_TIMEOUT_MS / 1000
                );
                break;
            }

            match self.socket.recv_from(&mut recv_buf) {
                Ok((amt, _)) => match net::parse_message(&recv_buf[..amt]) {
                    Ok(net::Message::Rtp { payload, .. }) => {
                        last_rtp_received = Instant::now();
                        let samples = bytes_to_f32(&payload);
                        self.audio_buffer.lock().unwrap().extend(samples.iter());
                    }
                    Ok(net::Message::Bye { reason }) => {
                        println!("Server sent BYE: {}", reason);
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Invalid packet: {}", e);
                    }
                },
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
}

pub fn run_client(server_ip: &str, discover: bool) {
    if let Err(e) = client(server_ip, discover) {
        eprintln!("Client error: {}", e);
    }
}

pub fn client(server_ip: &str, discover: bool) -> io::Result<()> {
    let client = Client::new()?;

    let server_addr = if discover {
        client.discover()?
    } else {
        server_ip.to_string()
    };

    client.connect(&server_addr)?;
    let _stream = client.setup_audio();
    client.run()?;

    Ok(())
}
