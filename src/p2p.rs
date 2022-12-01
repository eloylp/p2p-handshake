use std::fmt;

use tokio::{
    sync::{broadcast::error::RecvError, mpsc::error::SendError},
    task::{JoinError, JoinHandle},
};

use self::{
    config::{Commands, HandshakeConfig},
    view::{Event, EventChain, HandshakeResult},
};

mod btc;
pub mod config;
pub mod view;

pub async fn handshake(config: HandshakeConfig) -> Result<Vec<HandshakeResult>, P2PError> {
    let join_handles: Vec<(String, JoinHandle<Result<EventChain, P2PError>>)> =
        match &config.commands {
            Commands::Btc {
                nodes_addrs,
                user_agent,
            } => nodes_addrs
                .iter()
                .map(|node_addr| {
                    let config = btc::Config {
                        node_addr: node_addr.to_owned(),
                        timeout: config.timeout.to_owned(),
                        user_agent: user_agent.to_owned(),
                    };
                    let join = tokio::spawn(btc::handshake(config));
                    (node_addr.to_owned(), join)
                })
                .collect(),
        };

    let mut results = Vec::new();
    for (addr, jh) in join_handles {
        let res = jh.await?;
        results.push(HandshakeResult::new(addr, res))
    }
    Ok(results)
}

#[derive(Debug)]
pub struct P2PError {
    message: String,
}

impl fmt::Display for P2PError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P2P error: {}", self.message)
    }
}

impl From<SendError<Event>> for P2PError {
    fn from(err: SendError<Event>) -> Self {
        P2PError {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for P2PError {
    fn from(err: std::io::Error) -> Self {
        P2PError {
            message: err.to_string(),
        }
    }
}

impl From<SendError<usize>> for P2PError {
    fn from(err: SendError<usize>) -> Self {
        P2PError {
            message: err.to_string(),
        }
    }
}

impl From<RecvError> for P2PError {
    fn from(err: RecvError) -> Self {
        P2PError {
            message: err.to_string(),
        }
    }
}

impl From<tokio::sync::broadcast::error::SendError<usize>> for P2PError {
    fn from(err: tokio::sync::broadcast::error::SendError<usize>) -> Self {
        P2PError {
            message: err.to_string(),
        }
    }
}

impl From<JoinError> for P2PError {
    fn from(err: JoinError) -> Self {
        P2PError {
            message: err.to_string(),
        }
    }
}
