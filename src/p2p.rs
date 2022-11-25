use bytes::{Buf, BytesMut};
use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use bitcoin::{
    consensus::{deserialize_partial, serialize},
    network::{
        address, constants,
        constants::ServiceFlags,
        message,
        message::{NetworkMessage, RawNetworkMessage},
        message_network::VersionMessage,
    },
};
use clap::Parser;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Parser, Debug)]
#[command(version)]
pub struct HandshakeConfig {
    #[arg(short, long)]
    pub node_socket: String,
}

pub async fn btc_handshake(config: HandshakeConfig) -> Result<EventChain, Box<dyn Error>> {
    let node_socket = SocketAddr::from_str(&config.node_socket).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let no_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);

    let btc_version = VersionMessage::new(
        ServiceFlags::NONE,
        now,
        address::Address::new(&node_socket, constants::ServiceFlags::NONE),
        address::Address::new(&no_address, constants::ServiceFlags::NONE),
        now as u64,
        String::from("/Satoshi:23.0.0/"),
        0,
    );

    let btc_envelope = RawNetworkMessage {
        magic: constants::Network::Bitcoin.magic(),
        payload: NetworkMessage::Version(btc_version),
    };

    let stream = TcpStream::connect(config.node_socket).await?;

    let mut conn = Connection::new(stream, 1024);

    let mut event_chain = EventChain::new();
    conn.write_message(&btc_envelope).await?;

    event_chain.add(Event::new("VERSION".to_string(), EventDirection::OUT));
    loop {
        if let Some(message) = conn.read_message().await? {
            match message.payload {
                message::NetworkMessage::Verack => {
                    let event = Event::new("ACK".to_string(), EventDirection::IN);
                    event_chain.add(event);
                    println!("{}", "received ACK!");
                }
                message::NetworkMessage::Version(v) => {
                    let event = Event::new("VERSION".to_string(), EventDirection::IN);
                    event_chain.add(event);
                    println!("{} {:?}", "received version!", v);

                    conn.write_message(&RawNetworkMessage {
                        magic: constants::Network::Bitcoin.magic(),
                        payload: NetworkMessage::Verack,
                    })
                    .await?;

                    let event = Event::new("ACK".to_string(), EventDirection::OUT);
                    event_chain.add(event);
                }
                _ => {
                    let event = Event::new("UNKNOWN".to_string(), EventDirection::IN);
                    event_chain.add(event);
                    println!("{}, {}", "unknown message", message.cmd());
                }
            }
        }

        if event_chain.len() == 4 {
            break;
        }
    }
    return Ok(event_chain);
}

pub struct EventChain {
    events: Vec<Event>,
}

impl EventChain {
    pub fn new() -> Self {
        EventChain { events: Vec::new() }
    }
    pub fn add(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn get(&self, n: usize) -> Option<&Event> {
        self.events.get(n)
    }
}

pub struct Event {
    name: String,
    time: SystemTime,
    direction: EventDirection,
}

impl Event {
    fn new(name: String, direction: EventDirection) -> Event {
        Event {
            name: name,
            direction: direction,
            time: SystemTime::now(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn time(&self) -> SystemTime {
        self.time
    }

    pub fn direction(&self) -> &EventDirection {
        &self.direction
    }
}
pub enum EventDirection {
    IN,
    OUT,
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
