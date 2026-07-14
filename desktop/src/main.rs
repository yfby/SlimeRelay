use std::env;

use desktop::client::client;
use desktop::server::server;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: desktop <server|client> <address>");
        return;
    }
    match args[1].as_str() {
        "server" => {
            if let Err(e) = server() {
                eprintln!("Server error: {}", e);
            }
        }
        "client" => {
            if args.len() < 3 {
                eprintln!("Usage: desktop client <address>");
                return;
            }
            if let Err(e) = client(&args[2]) {
                eprintln!("Client error: {}", e);
            }
        }
        other => eprintln!("Unknown argument: {}. Use 'server' or 'client'.", other),
    }
}
