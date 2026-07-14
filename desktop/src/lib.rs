use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;
pub const CHUNK_SAMPLES: usize = 512;
pub const SERVER_ADDR: &str = "127.0.0.1:34254";

pub type SampleBuffer = Arc<Mutex<VecDeque<f32>>>;

pub fn new_sample_buffer() -> SampleBuffer {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub mod audio;
pub mod client;
pub mod microphone;
pub mod net;
pub mod server;
pub mod ui;
