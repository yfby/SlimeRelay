use std::io;
use std::net::{SocketAddr, UdpSocket};

const HANDSHAKE_BUF_SIZE: usize = 64;

pub fn wait_for_hello(socket: &UdpSocket) -> io::Result<SocketAddr> {
    let mut buf = [0u8; HANDSHAKE_BUF_SIZE];
    let (amt, client_addr) = socket.recv_from(&mut buf)?;
    if &buf[..amt] != b"HELLO" {
        eprintln!("Unexpected handshake from {}", client_addr);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected HELLO handshake",
        ));
    }
    Ok(client_addr)
}

pub fn send_ready(socket: &UdpSocket, addr: SocketAddr) -> io::Result<()> {
    socket.send_to(b"READY", addr)?;
    Ok(())
}

pub fn send_hello(socket: &UdpSocket, server_addr: &str) -> io::Result<()> {
    socket.send_to(b"HELLO", server_addr)?;
    Ok(())
}

pub fn wait_for_ready(socket: &UdpSocket) -> io::Result<()> {
    let mut buf = [0u8; HANDSHAKE_BUF_SIZE];
    let (amt, _) = socket.recv_from(&mut buf)?;
    if &buf[..amt] != b"READY" {
        eprintln!("Server not ready");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Server did not respond with READY",
        ));
    }
    Ok(())
}
