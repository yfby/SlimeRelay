use std::env;

use desktop::client::client;
use desktop::server::server;

// TODO: virtual microphone setup for Windows and Linux, and virtual audio device for macOS.
#[cfg(target_os = "linux")]
fn check_os() {
    println!("Running on Linux!");
}

#[cfg(target_os = "windows")]
fn check_os() {
    println!("Running on Windows!");
}

#[cfg(target_os = "macos")]
fn check_os() {
    println!("Running on macOS!");
}

fn main() {
    check_os();
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
