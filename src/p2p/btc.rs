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
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
    select,
    sync::mpsc::{self, UnboundedSender},
};

use super::{Event, EventChain, EventDirection, HandshakeConfig};

pub async fn handshake(config: HandshakeConfig) -> Result<EventChain, Box<dyn Error>> {
    let mut stream = TcpStream::connect(&config.node_socket).await?;
    let (mut recv_stream, mut write_stream) = stream.split();

    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel();
    let event_chain_handle = tokio::spawn(async move {
        let mut event_chain = EventChain::new();
        loop {
            select! {
                Some(ev) = ev_rx.recv() => {
                    println!("{:?}", ev);
                    event_chain.add(ev);
                }
            }
            if event_chain.len() == 4 {
                break;
            }
        }

        event_chain
    });

    let version_message = version_message(config.node_socket);
    write_message(&mut write_stream, &version_message).await?;

    ev_tx
        .send(Event::new("VERSION".to_string(), EventDirection::OUT))
        .unwrap();

    let mut frame_reader = FrameReader::new(&mut recv_stream, 1024);

    let event_publisher = ev_tx.clone();

    loop {
        if let Some(message) = frame_reader.read_message().await? {
            handle_message(message, &mut write_stream, &event_publisher).await?;
        }

        if event_chain_handle.is_finished() {
            break;
        }
    }

    let event_chain = event_chain_handle.await?;
    Ok(event_chain)
}

async fn handle_message<'a>(
    message: RawNetworkMessage,
    write_stream: &mut WriteHalf<'a>,
    event_publisher: &UnboundedSender<Event>,
) -> Result<(), Box<dyn Error>> {
    match message.payload {
        message::NetworkMessage::Verack => {
            let event = Event::new("ACK".to_string(), EventDirection::IN);
            event_publisher.send(event).unwrap();
            println!("{}", "received ACK!");
            Ok(())
        }
        message::NetworkMessage::Version(v) => {
            let event = Event::new("VERSION".to_string(), EventDirection::IN);
            event_publisher.send(event).unwrap();
            println!("{} {:?}", "received version!", v);

            write_message(write_stream, &verack_message()).await?;
            let event = Event::new("ACK".to_string(), EventDirection::OUT);
            event_publisher.send(event).unwrap();
            println!("{} {:?}", "sent ACK!", v);

            Ok(())
        }
        _ => {
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

pub fn version_message(dest_socket: String) -> RawNetworkMessage {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let no_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);
    let node_socket = SocketAddr::from_str(&dest_socket).unwrap();

    let btc_version = VersionMessage::new(
        ServiceFlags::NONE,
        now,
        address::Address::new(&node_socket, constants::ServiceFlags::NONE),
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

struct FrameReader<'a> {
    stream: &'a mut ReadHalf<'a>,
    buffer: BytesMut,
}

impl FrameReader<'_> {
    pub fn new<'a>(stream: &'a mut ReadHalf<'a>, buff_size: usize) -> FrameReader {
        FrameReader {
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
}

async fn write_message<'a>(
    stream: &mut WriteHalf<'a>,
    message: &RawNetworkMessage,
) -> io::Result<()> {
    let data = serialize(message);
    stream.write_all(data.as_slice()).await?;
    Ok(())
}
