use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedReadHalf, TcpStream},
    select, signal,
    sync::{
        broadcast,
        mpsc::{self, error::SendError, UnboundedSender},
    },
    try_join,
};

use crate::p2p::{
    view::{Event, EventChain, EventDirection, EMOJI_WARNING},
    P2PError,
};

pub struct Config {
    pub node_addr: String,
    pub timeout: u64,
    pub user_agent: String,
}

const EXPECTED_HANDSHAKE_MESSAGES: usize = 4;

pub async fn handshake(config: Config) -> Result<EventChain, P2PError> {
    // Setup shutdown broadcast channels
    let (shutdown_tx, _) = broadcast::channel::<usize>(1);

    // Spawn the event chain task.
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel();
    let mut ev_shutdown_rx = shutdown_tx.subscribe();
    let ev_shutdown_tx = shutdown_tx.clone();
    let event_chain_id = config.node_addr.clone();
    let event_chain_handle = tokio::spawn(async move {
        let mut event_chain = EventChain::new(event_chain_id);
        loop {
            select! {
                Some(ev) = ev_rx.recv() => {
                    event_chain.add(ev);
                }
                recv_res = ev_shutdown_rx.recv() => {
                    return match recv_res {
                        Ok(_) => Ok(event_chain),
                        Err(err) => Err(P2PError::from(err)),
                    }
                }
            }
            if event_chain.len() == EXPECTED_HANDSHAKE_MESSAGES {
                event_chain.mark_as_complete();
                ev_shutdown_tx.send(1)?;
            }
        }
    });

    // Stablish TCP connection with timeout.
    let stream = tokio::time::timeout(
        Duration::from_millis(config.timeout),
        TcpStream::connect(&config.node_addr),
    )
    .await??;

    let (recv_stream, mut write_stream) = stream.into_split();

    // Spawn the message writer task. This will take care of serialize all messages write to the socket.
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<RawNetworkMessage>();
    let write_msg_ev_tx = ev_tx.clone();
    let mut write_msg_shutdown_rx = shutdown_tx.subscribe();
    let write_message_handle = tokio::spawn(async move {
        loop {
            select! {
                Some(msg) = msg_rx.recv() => {
                    let msg_type = msg.cmd().to_string();
                    let data = serialize(&msg);
                    write_stream.write_all(data.as_slice()).await?;
                    write_msg_ev_tx.send(Event::new(msg_type, EventDirection::OUT))?;
                }
                result = write_msg_shutdown_rx.recv() => {
                    write_stream.shutdown().await?;
                    return match result {
                        Ok(_) => Ok(()),
                        Err(err) => Err(P2PError::from(err)),
                    }
                }
            }
        }
    });

    // Spawn the frame reader task
    let mut frame_reader_shutdown_rx = shutdown_tx.subscribe();
    let frame_reader_msg_tx = msg_tx.clone();
    let frame_reader_handle = tokio::spawn(async move {
        let mut frame_reader = FrameReader::new(recv_stream, 1024);
        let mut handles = Vec::new();
        loop {
            select! {
                message_res = frame_reader.read_message() => {
                    match message_res {
                        Ok(opt_res) => {
                            if let Some(msg) = opt_res {
                                let handle = tokio::spawn(handle_message(msg, frame_reader_msg_tx.clone(), ev_tx.clone()));
                                handles.push(handle);
                            }
                         },
                        Err(err) => return Err(err),
                    }
                },
                result = frame_reader_shutdown_rx.recv() => {
                   return match result {
                     Ok(_) => {
                       // Ensure all message handles succeeded before ending.
                       futures::future::try_join_all(handles).await?;
                       Ok(())
                     },
                     Err(err) => Err(P2PError::from(err)),
                    }
                }
            }
        }
    });

    // Start the handshake by sending the first VERSION message
    let version_message = version_message(config.node_addr, config.user_agent);
    msg_tx.send(version_message)?;

    // Wait for external shutdown signals ctr+c ...
    let mut ext_shutdown_shutdown_rx = shutdown_tx.subscribe();
    select! {
        _ = tokio::time::sleep(Duration::from_millis(config.timeout)) => {
            shutdown_tx.send(1)?;
        }
        val = signal::ctrl_c() => {
            if val.is_ok(){
                shutdown_tx.send(1)?;
            }
        }
        // Break this select! once an internal shutdown is invoked from any of the subs systems.
        _val = ext_shutdown_shutdown_rx.recv()=>{}
    }

    let (event_chain_res, write_message_res, frame_reader_res) = try_join!(
        event_chain_handle,
        write_message_handle,
        frame_reader_handle
    )?;
    // Check no errors happened in write and frame reader.
    write_message_res?;
    frame_reader_res?;
    // Finally, check the event chain was successful and return it.
    event_chain_res
}

async fn handle_message(
    message: RawNetworkMessage,
    msg_writer: UnboundedSender<RawNetworkMessage>,
    event_publisher: UnboundedSender<Event>,
) -> Result<(), P2PError> {
    let msg_type = message.cmd().to_string();
    match message.payload {
        message::NetworkMessage::Verack => {
            let event = Event::new(msg_type, EventDirection::IN);
            event_publisher.send(event)?;
            Ok(())
        }
        message::NetworkMessage::Version(v) => {
            let mut event = Event::new(msg_type, EventDirection::IN);
            event.set_pair("vers".to_string(), v.version.to_string());
            event.set_pair("user-agent".to_string(), v.user_agent);
            event_publisher.send(event)?;
            msg_writer.send(verack_message())?;
            Ok(())
        }
        _ => {
            println!(
                "{}  received message type not part of handshake: {}",
                EMOJI_WARNING, msg_type
            );
            Ok(())
        }
    }
}

struct FrameReader {
    stream: OwnedReadHalf,
    buffer: BytesMut,
}

impl FrameReader {
    pub fn new(stream: OwnedReadHalf, buff_size: usize) -> FrameReader {
        FrameReader {
            stream,
            buffer: BytesMut::with_capacity(buff_size),
        }
    }
    pub async fn read_message(&mut self) -> Result<Option<RawNetworkMessage>, P2PError> {
        loop {
            if let Ok((message, count)) = deserialize_partial::<RawNetworkMessage>(&self.buffer) {
                self.buffer.advance(count);
                return Ok(Some(message));
            }

            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(P2PError {
                        message: "connection reset by peer".into(),
                    });
                }
            }
        }
    }
}

pub fn verack_message() -> RawNetworkMessage {
    RawNetworkMessage {
        magic: constants::Network::Bitcoin.magic(),
        payload: NetworkMessage::Verack,
    }
}

pub fn version_message(dest_socket: String, user_agent: String) -> RawNetworkMessage {
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
        user_agent,
        0,
    );

    RawNetworkMessage {
        magic: constants::Network::Bitcoin.magic(),
        payload: NetworkMessage::Version(btc_version),
    }
}

impl From<SendError<RawNetworkMessage>> for P2PError {
    fn from(send_err: SendError<RawNetworkMessage>) -> Self {
        P2PError {
            message: send_err.to_string(),
        }
    }
}
