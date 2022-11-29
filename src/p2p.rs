use std::{fmt, time::SystemTime};

use clap::{Parser, Subcommand};
use tokio::{
    sync::{broadcast::error::RecvError, mpsc::error::SendError},
    task::JoinError,
};

mod btc;

pub async fn handshake(config: HandshakeConfig) -> Result<EventChain, P2PError> {
    match &config.commands {
        Commands::Btc { node_addr } => {
            let config = btc::Config {
                node_addr: node_addr.to_owned(),
            };
            btc::handshake(config).await
        }
    }
}

#[derive(Parser, Debug)]
#[command(version)]
#[command(propagate_version = true)]
pub struct HandshakeConfig {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Btc { node_addr: String },
}

#[derive(Debug)]
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

    pub fn is_empty(&self) -> bool {
        self.events.len() == 0
    }

    pub fn get(&self, n: usize) -> Option<&Event> {
        self.events.get(n)
    }
}

impl Default for EventChain {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Event {
    name: String,
    time: SystemTime,
    direction: EventDirection,
}

impl Event {
    fn new(name: String, direction: EventDirection) -> Event {
        Event {
            name,
            direction,
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

#[derive(Debug)]
pub enum EventDirection {
    IN,
    OUT,
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
