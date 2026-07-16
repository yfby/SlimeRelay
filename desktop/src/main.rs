use std::env;

use desktop::client::client;
use desktop::server::server;

fn print_usage() {
    eprintln!("Usage: desktop <command> [options]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  server    Start the audio relay server");
    eprintln!("  client    Connect to an audio relay server");
    eprintln!();
    eprintln!("Server options:");
    eprintln!("  --name <name>      Server name for discovery (default: hostname)");
    eprintln!();
    eprintln!("Client options:");
    eprintln!("  --discover         Auto-discover server via broadcast");
    eprintln!("  <address>          Server address (ip:port) if not using --discover");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  desktop server --name MyPC");
    eprintln!("  desktop client --discover");
    eprintln!("  desktop client 192.168.1.100:34254");
}

fn parse_arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn get_hostname() -> String {
    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "SlimeRelay".to_string());
    println!("Using hostname as server name: {}", host);
    host
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "server" => {
            let server_name = parse_arg(&args, "--name").unwrap_or_else(get_hostname);
            println!("Starting server '{}'...", server_name);
            if let Err(e) = server(&server_name) {
                eprintln!("Server error: {}", e);
            }
        }
        "client" => {
            let discover = has_flag(&args, "--discover");

            if discover {
                if let Err(e) = client("", true) {
                    eprintln!("Client error: {}", e);
                }
            } else {
                let remaining: Vec<&String> =
                    args[2..].iter().filter(|a| !a.starts_with("--")).collect();
                if remaining.is_empty() {
                    eprintln!("Error: Provide a server address or use --discover");
                    eprintln!("Usage: desktop client <address>");
                    eprintln!("       desktop client --discover");
                    std::process::exit(1);
                }
                let addr = remaining[0];
                if let Err(e) = client(addr, false) {
                    eprintln!("Client error: {}", e);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: '{}'", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }
}
