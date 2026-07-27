use std::io;
use std::net::{SocketAddr, UdpSocket};

use crate::{
    DISCOVERY_PORT, MSG_BYE, MSG_DISCOVERY, MSG_HELLO, MSG_READY, MSG_RTP, PROTOCOL_VERSION,
    RTP_HEADER_SIZE, RTP_PT_PCM, SERVER_NAME_SIZE, SESSION_ID_SIZE,
};

#[derive(Debug, Clone)]
pub enum Message {
    Discovery {
        server_name: String,
        port: u16,
    },
    Hello,
    Ready {
        session_id: [u8; SESSION_ID_SIZE],
    },
    Rtp {
        sequence: u16,
        timestamp: u32,
        ssrc: u32,
        payload: Vec<u8>,
    },
    Bye {
        reason: String,
    },
}

pub fn build_hello() -> Vec<u8> {
    vec![PROTOCOL_VERSION, MSG_HELLO]
}

pub fn build_ready(session_id: &[u8; SESSION_ID_SIZE]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 1 + SESSION_ID_SIZE);
    buf.push(PROTOCOL_VERSION);
    buf.push(MSG_READY);
    buf.extend_from_slice(session_id);
    buf
}

pub fn build_discovery(server_name: &str, port: u16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 1 + SERVER_NAME_SIZE + 2);
    buf.push(PROTOCOL_VERSION);
    buf.push(MSG_DISCOVERY);

    let name_bytes = server_name.as_bytes();
    let mut name_padded = [0u8; SERVER_NAME_SIZE];
    let copy_len = name_bytes.len().min(SERVER_NAME_SIZE);
    name_padded[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
    buf.extend_from_slice(&name_padded);

    buf.extend_from_slice(&port.to_le_bytes());
    buf
}

pub fn build_rtp_packet(sequence: u16, timestamp: u32, ssrc: u32, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(RTP_HEADER_SIZE + payload.len());

    let v_p_x_cc: u8 = 0x80;
    let m_pt: u8 = RTP_PT_PCM;
    buf.push(v_p_x_cc);
    buf.push(m_pt);
    buf.extend_from_slice(&sequence.to_be_bytes());
    buf.extend_from_slice(&timestamp.to_be_bytes());
    buf.extend_from_slice(&ssrc.to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}

pub fn build_bye(reason: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 1 + 32);
    buf.push(PROTOCOL_VERSION);
    buf.push(MSG_BYE);

    let reason_bytes = reason.as_bytes();
    let mut reason_padded = [0u8; 32];
    let copy_len = reason_bytes.len().min(32);
    reason_padded[..copy_len].copy_from_slice(&reason_bytes[..copy_len]);
    buf.extend_from_slice(&reason_padded);
    buf
}

pub fn wait_for_discovery(socket: &UdpSocket) -> io::Result<(String, SocketAddr)> {
    let mut buf = [0u8; 128];
    loop {
        let (_, src_addr) = socket.recv_from(&mut buf)?;
        match parse_message(&buf[..]) {
            Ok(Message::Discovery { server_name, port }) => {
                let server_addr = SocketAddr::new(src_addr.ip(), port);
                return Ok((server_name, server_addr));
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }
}

pub fn send_discovery_broadcast(
    socket: &UdpSocket,
    server_name: &str,
    port: u16,
) -> io::Result<()> {
    let msg = build_discovery(server_name, port);
    let broadcast_addr = format!("255.255.255.255:{}", DISCOVERY_PORT);
    socket.send_to(&msg, broadcast_addr)?;
    Ok(())
}

pub fn wait_for_hello(socket: &UdpSocket) -> io::Result<SocketAddr> {
    let mut buf = [0u8; 128];
    loop {
        let (amt, client_addr) = socket.recv_from(&mut buf)?;
        match parse_message(&buf[..amt]) {
            Ok(Message::Hello) => return Ok(client_addr),
            Ok(_) => continue,
            Err(_) => continue,
        }
    }
}

pub fn send_hello(socket: &UdpSocket, server_addr: &str) -> io::Result<()> {
    let msg = build_hello();
    socket.send_to(&msg, server_addr)?;
    Ok(())
}

pub fn wait_for_ready(socket: &UdpSocket) -> io::Result<[u8; SESSION_ID_SIZE]> {
    let mut buf = [0u8; 128];
    let (amt, _) = socket.recv_from(&mut buf)?;
    match parse_message(&buf[..amt]) {
        Ok(Message::Ready { session_id }) => Ok(session_id),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Server did not respond with READY",
        )),
        Err(e) => Err(e),
    }
}

pub fn send_ready(
    socket: &UdpSocket,
    addr: SocketAddr,
    session_id: &[u8; SESSION_ID_SIZE],
) -> io::Result<()> {
    let msg = build_ready(session_id);
    socket.send_to(&msg, addr)?;
    Ok(())
}

pub fn parse_message(data: &[u8]) -> io::Result<Message> {
    if data.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Message too short",
        ));
    }

    if data[0] == 0x80 && data.len() >= RTP_HEADER_SIZE {
        let sequence = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let payload = data[RTP_HEADER_SIZE..].to_vec();
        return Ok(Message::Rtp {
            sequence,
            timestamp,
            ssrc,
            payload,
        });
    }

    let version = data[0];
    if version != PROTOCOL_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unsupported protocol version: {}", version),
        ));
    }

    let msg_type = data[1];

    match msg_type {
        MSG_DISCOVERY => {
            if data.len() < 1 + 1 + SERVER_NAME_SIZE + 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Discovery message too short",
                ));
            }
            let name_bytes = &data[2..2 + SERVER_NAME_SIZE];
            let name = String::from_utf8_lossy(name_bytes)
                .trim_end_matches('\0')
                .to_string();
            let port = u16::from_le_bytes([data[2 + SERVER_NAME_SIZE], data[3 + SERVER_NAME_SIZE]]);
            Ok(Message::Discovery {
                server_name: name,
                port,
            })
        }
        MSG_HELLO => Ok(Message::Hello),
        MSG_READY => {
            if data.len() < 1 + 1 + SESSION_ID_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Ready message too short",
                ));
            }
            let mut session_id = [0u8; SESSION_ID_SIZE];
            session_id.copy_from_slice(&data[2..2 + SESSION_ID_SIZE]);
            Ok(Message::Ready { session_id })
        }
        MSG_RTP => {
            unreachable!("RTP already handled above")
        }
        MSG_BYE => {
            if data.len() < 1 + 1 + 32 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Bye message too short",
                ));
            }
            let reason_bytes = &data[2..2 + 32];
            let reason = String::from_utf8_lossy(reason_bytes)
                .trim_end_matches('\0')
                .to_string();
            Ok(Message::Bye { reason })
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown message type: 0x{:02X}", msg_type),
        )),
    }
}
