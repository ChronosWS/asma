// Copyright (c) 2015 [rust-rcon developers]
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.
// NOTE: Modified to use on Ark: Ascended(tm) for the Ark Server Manager: Ascended

use err_derive::Error;
use tokio::net::TcpStream;
use tracing::trace;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "authentication failed")]
    Auth,
    #[error(display = "command exceeds the maximum length")]
    CommandTooLong,
    #[error(display = "{}", _0)]
    Io(#[error(source)] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

const INITIAL_PACKET_ID: i32 = 1;

pub struct Connection {
    io: TcpStream,
    next_packet_id: i32,
}

impl Connection {
    pub async fn connect(address: impl AsRef<str>, password: impl AsRef<str>) -> Result<Self> {
        let io = TcpStream::connect(address.as_ref()).await?;
        let mut conn = Self {
            io,
            next_packet_id: INITIAL_PACKET_ID,
        };

        conn.auth(password.as_ref()).await?;

        Ok(conn)
    }

    pub async fn cmd(&mut self, cmd: &str) -> Result<(i32, String)> {
        let packet_id = self.send(PacketType::ExecCommand, cmd).await?;
        let received_packet = self.receive_packet().await?;
        trace!("Sent {}, received {}", packet_id, received_packet.id);
        Ok((packet_id, received_packet.get_body().into()))
    }

    pub async fn cmd2(&mut self, cmd: &str) -> Result<(i32, String)> {
        let packet_id = self.send(PacketType::ExecCommand, cmd).await?;
        trace!("Sent message {}", packet_id);
        let received_packet = self.receive_packet().await?;
        trace!("Received {}", received_packet.id);

        let end_id = self.send(PacketType::ExecCommand, "").await?;
        trace!("Sent multi-packet end {}", end_id);
        let end_packet = self.receive_packet().await?;
        trace!("Received {}", end_packet.id);
        Ok((packet_id, received_packet.get_body().into()))
    }

    // async fn receive_response(&mut self) -> Result<String> {
    //     self.receive_single_packet_response().await
    // }

    // async fn receive_single_packet_response(&mut self) -> Result<String> {
    //     let received_packet = self.receive_packet().await?;
    //     Ok(received_packet.get_body().into())
    // }

    // async fn receive_multi_packet_response(&mut self) -> Result<String> {
    //     // TODO: Currently there is an issue where sends and receives must be matched, otherwise 
    //     // the process wedges on sending.

    //     // the server processes packets in order, so send an empty packet and
    //     // remember its id to detect the end of a multi-packet response
    //     let end_id = self.send(PacketType::ExecCommand, "").await?;

    //     let mut result = String::new();

    //     loop {
    //         let received_packet = self.receive_packet().await?;

    //         if received_packet.get_id() == end_id {
    //             // This is the response to the end-marker packet
    //             return Ok(result);
    //         }

    //         result += received_packet.get_body();
    //     }
    // }

    async fn auth(&mut self, password: &str) -> Result<()> {
        self.send(PacketType::Auth, password).await?;
        let received_packet = loop {
            let received_packet = self.receive_packet().await?;
            if received_packet.get_type() == PacketType::AuthResponse {
                break received_packet;
            }
        };

        if received_packet.is_error() {
            Err(Error::Auth)
        } else {
            Ok(())
        }
    }

    async fn send(&mut self, ptype: PacketType, body: &str) -> io::Result<i32> {
        let id = self.generate_packet_id();

        let packet = Packet::new(id, ptype, body.into());

        packet.serialize(&mut self.io).await?;

        Ok(id)
    }

    async fn receive_packet(&mut self) -> io::Result<Packet> {
        Packet::deserialize(&mut self.io).await
    }

    fn generate_packet_id(&mut self) -> i32 {
        let id = self.next_packet_id;

        // only use positive ids as the server uses negative ids to signal
        // a failed authentication request
        self.next_packet_id = self
            .next_packet_id
            .checked_add(1)
            .unwrap_or(INITIAL_PACKET_ID);

        id
    }
}

// Copyright (c) 2015 [rust-rcon developers]
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketType {
    Auth,
    AuthResponse,
    ExecCommand,
    ResponseValue,
    Unknown(i32),
}

impl PacketType {
    fn to_i32(self) -> i32 {
        match self {
            PacketType::Auth => 3,
            PacketType::AuthResponse => 2,
            PacketType::ExecCommand => 2,
            PacketType::ResponseValue => 0,
            PacketType::Unknown(n) => n,
        }
    }

    pub fn from_i32(n: i32, is_response: bool) -> PacketType {
        match n {
            3 => PacketType::Auth,
            2 if is_response => PacketType::AuthResponse,
            2 => PacketType::ExecCommand,
            0 => PacketType::ResponseValue,
            n => PacketType::Unknown(n),
        }
    }
}

#[derive(Debug)]
pub struct Packet {
    length: i32,
    id: i32,
    ptype: PacketType,
    body: String,
}

impl Packet {
    pub fn new(id: i32, ptype: PacketType, body: String) -> Packet {
        Packet {
            length: 10 + body.len() as i32,
            id,
            ptype,
            body,
        }
    }

    pub fn is_error(&self) -> bool {
        self.id < 0
    }

    pub async fn serialize<T: Unpin + AsyncWrite>(&self, w: &mut T) -> io::Result<()> {
        // Write bytes to a buffer first so only one tcp packet is sent
        // This is done in order to not overwhelm a Minecraft server
        let mut buf = Vec::with_capacity(self.length as usize);

        buf.extend_from_slice(&self.length.to_le_bytes());
        buf.extend_from_slice(&self.id.to_le_bytes());
        buf.extend_from_slice(&self.ptype.to_i32().to_le_bytes());
        buf.extend_from_slice(self.body.as_bytes());
        buf.extend_from_slice(&[0x00, 0x00]);

        w.write_all(&buf).await?;

        Ok(())
    }

    pub async fn deserialize<T: Unpin + AsyncRead>(r: &mut T) -> io::Result<Packet> {
        let mut buf = [0u8; 4];

        r.read_exact(&mut buf).await?;
        let length = i32::from_le_bytes(buf);
        r.read_exact(&mut buf).await?;
        let id = i32::from_le_bytes(buf);
        r.read_exact(&mut buf).await?;
        let ptype = i32::from_le_bytes(buf);
        let body_length = length - 10;
        let mut body_buffer = Vec::with_capacity(body_length as usize);

        r.take(body_length as u64)
            .read_to_end(&mut body_buffer)
            .await?;

        let body = String::from_utf8(body_buffer)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        // terminating nulls
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf).await?;

        let packet = Packet {
            length,
            id,
            ptype: PacketType::from_i32(ptype, true),
            body,
        };

        Ok(packet)
    }

    pub fn get_body(&self) -> &str {
        &self.body
    }

    pub fn get_type(&self) -> PacketType {
        self.ptype
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }
}
