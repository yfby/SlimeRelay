use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;
pub const CHUNK_SAMPLES: usize = 512;
pub const SERVER_PORT: u16 = 34254;
pub const DISCOVERY_PORT: u16 = 34255;
pub const SERVER_ADDR: &str = "127.0.0.1:34254";

pub const PROTOCOL_VERSION: u8 = 0x01;
pub const MSG_DISCOVERY: u8 = 0x01;
pub const MSG_HELLO: u8 = 0x02;
pub const MSG_READY: u8 = 0x03;
pub const MSG_RTP: u8 = 0x80;
pub const MSG_BYE: u8 = 0xC0;

pub const RTP_PT_PCM: u8 = 96;
pub const RTP_HEADER_SIZE: usize = 12;
pub const SESSION_ID_SIZE: usize = 16;
pub const SERVER_NAME_SIZE: usize = 32;

pub const KEEPALIVE_INTERVAL_MS: u64 = 2000;
pub const KEEPALIVE_TIMEOUT_MS: u64 = 6000;

pub type SampleBuffer = Arc<Mutex<VecDeque<f32>>>;

// audio buffer
pub fn new_sample_buffer() -> SampleBuffer {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub mod audio;
pub mod client;
pub mod eframe_gui;
pub mod microphone;
pub mod net;
pub mod server;
