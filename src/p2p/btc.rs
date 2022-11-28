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
    join,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    select,
    sync::{
        broadcast,
        mpsc::{self, UnboundedSender},
    },
};

use super::{Event, EventChain, EventDirection, HandshakeConfig};

pub async fn handshake(config: HandshakeConfig) -> Result<EventChain, Box<dyn Error>> {
    // Setup shutdown broadcast channels
    let (shutdown_tx, _) = broadcast::channel(10);

    // Spawn the event chain task and configure its channels
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel();
    let mut ev_shutdown_rx = shutdown_tx.subscribe();
    let ev_shutdown_tx = shutdown_tx.clone();
    let event_chain_handle = tokio::spawn(async move {
        let mut event_chain = EventChain::new();
        loop {
            select! {
                Some(ev) = ev_rx.recv() => {
                    event_chain.add(ev);
                }
                Ok(_) = ev_shutdown_rx.recv() => {
                    break;
                }
            }
            if event_chain.len() == 4 {
                ev_shutdown_tx.send(1).unwrap();
            }
        }
        event_chain
    });

    // Stablish TCP connection
    let stream = TcpStream::connect(&config.node_socket).await?;
    let (mut recv_stream, mut write_stream) = stream.into_split();

    // Configure the message writer. This will take care of all messages
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<RawNetworkMessage>();
    let write_msg_ev_tx = ev_tx.clone();
    let mut write_msg_shutdown_rx = shutdown_tx.subscribe();
    let write_message_handle = tokio::spawn(async move {
        loop {
            select! {
                Some(msg) = msg_rx.recv() => {
                    let msg_type = msg.cmd().to_string();
                    write_message(&mut write_stream, msg).await.unwrap();
                    write_msg_ev_tx.send(Event::new(msg_type, EventDirection::OUT)).unwrap();
                }
                Ok(_) = write_msg_shutdown_rx.recv() => {
                    break;
                }
            }
        }
    });

    // Start the handshake by sending the first ACK message
    let version_message = version_message(config.node_socket);
    msg_tx.send(version_message)?;

    let mut frame_reader = FrameReader::new(&mut recv_stream, 1024);

    loop {
        if let Some(message) = frame_reader.read_message().await? {
            handle_message(message, msg_tx.clone(), ev_tx.clone()).await?;
        }

        if event_chain_handle.is_finished() {
            break;
        }
    }

    let (event_chain, _) = join!(event_chain_handle, write_message_handle);
    return Ok(event_chain.unwrap());
}

async fn handle_message<'a>(
    message: RawNetworkMessage,
    msg_writer: UnboundedSender<RawNetworkMessage>,
    event_publisher: UnboundedSender<Event>,
) -> Result<(), Box<dyn Error>> {
    let msg_type = message.cmd().to_string();
    match message.payload {
        message::NetworkMessage::Verack => {
            let event = Event::new(msg_type, EventDirection::IN);
            event_publisher.send(event)?;
            Ok(())
        }
        message::NetworkMessage::Version(v) => {
            let event = Event::new(msg_type, EventDirection::IN);
            event_publisher.send(event)?;
            msg_writer.send(verack_message())?;
            Ok(())
        }
        _ => Ok(()),
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
    stream: &'a mut OwnedReadHalf,
    buffer: BytesMut,
}

impl FrameReader<'_> {
    pub fn new<'a>(stream: &'a mut OwnedReadHalf, buff_size: usize) -> FrameReader {
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

async fn write_message(stream: &mut OwnedWriteHalf, message: RawNetworkMessage) -> io::Result<()> {
    let data = serialize(&message);
    stream.write_all(data.as_slice()).await?;
    Ok(())
}
