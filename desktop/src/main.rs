use std::env;

use desktop::client::client;
use desktop::server::server;

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
