use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use bitcoin::{
    consensus::{deserialize_partial, serialize},
    network::{
        address,
        constants::{self, ServiceFlags},
        message::{self, NetworkMessage, RawNetworkMessage},
        message_network::VersionMessage,
    },
};
use bytes::{Buf, BytesMut};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::{Event, EventChain, EventDirection, HandshakeConfig};

pub async fn handshake(config: HandshakeConfig) -> Result<EventChain, Box<dyn Error>> {
    let stream = TcpStream::connect(&config.node_socket).await?;

    let mut conn = Connection::new(stream, 1024);

    let mut event_chain = EventChain::new();
    let node_socket = SocketAddr::from_str(&config.node_socket).unwrap();
    let version_message = version_message(&node_socket);

    conn.write_message(&version_message).await?;
    event_chain.add(Event::new("VERSION".to_string(), EventDirection::OUT));

    loop {
        if let Some(message) = conn.read_message().await? {
            handle_message(message, &mut conn, &mut event_chain).await?;
        }

        if event_chain.len() == 4 {
            break;
        }
    }
    return Ok(event_chain);
}

async fn handle_message(
    message: RawNetworkMessage,
    conn: &mut Connection,
    event_chain: &mut EventChain,
) -> Result<(), Box<dyn Error>> {
    match message.payload {
        message::NetworkMessage::Verack => {
            let event = Event::new("ACK".to_string(), EventDirection::IN);
            event_chain.add(event);
            println!("{}", "received ACK!");
            Ok(())
        }
        message::NetworkMessage::Version(v) => {
            let event = Event::new("VERSION".to_string(), EventDirection::IN);
            event_chain.add(event);
            println!("{} {:?}", "received version!", v);

            conn.write_message(&verack_message()).await?;
            let event = Event::new("ACK".to_string(), EventDirection::OUT);
            event_chain.add(event);
            Ok(())
        }
        _ => {
            let event = Event::new("UNKNOWN".to_string(), EventDirection::IN);
            event_chain.add(event);
            println!("{}, {}", "unknown message", message.cmd());
            Ok(())
        }
    }
}

pub fn verack_message() -> RawNetworkMessage {
    RawNetworkMessage {
        magic: constants::Network::Bitcoin.magic(),
        payload: NetworkMessage::Verack,
    }
}

pub fn version_message(dest_socket: &SocketAddr) -> RawNetworkMessage {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let no_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);

    let btc_version = VersionMessage::new(
        ServiceFlags::NONE,
        now,
        address::Address::new(dest_socket, constants::ServiceFlags::NONE),
        address::Address::new(&no_address, constants::ServiceFlags::NONE),
        now as u64,
        String::from("/Satoshi:23.0.0/"),
        0,
    );

    RawNetworkMessage {
        magic: constants::Network::Bitcoin.magic(),
        payload: NetworkMessage::Version(btc_version),
    }
}

struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream, buff_size: usize) -> Connection {
        Connection {
            stream,
            buffer: BytesMut::with_capacity(buff_size),
        }
    }

    pub async fn read_message(&mut self) -> Result<Option<RawNetworkMessage>, Box<dyn Error>> {
        loop {
            if let Ok((message, count)) = deserialize_partial::<RawNetworkMessage>(&mut self.buffer)
            {
                self.buffer.advance(count);
                return Ok(Some(message));
            }

            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    pub async fn write_message(&mut self, message: &RawNetworkMessage) -> io::Result<()> {
        let data = serialize(message);
        self.stream.write_all(data.as_slice()).await?;
        Ok(())
    }
}
