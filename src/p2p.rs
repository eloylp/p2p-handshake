use std::{fmt, time::SystemTime};

use clap::{Parser, Subcommand};
use futures::future::try_join_all;
use tokio::{
    sync::{broadcast::error::RecvError, mpsc::error::SendError},
    task::{JoinError, JoinHandle},
};

mod btc;

pub async fn handshake(config: HandshakeConfig) -> Result<Vec<EventChain>, P2PError> {
    let join_handles: Vec<JoinHandle<Result<EventChain, P2PError>>> = match &config.commands {
        Commands::Btc { nodes_addrs, user_agent } => nodes_addrs
            .iter()
            .map(|node_addr| {
                let config = btc::Config {
                    node_addr: node_addr.to_owned(),
                    timeout: config.timeout.to_owned(),
                    user_agent: user_agent.to_owned()
                };
                tokio::spawn(btc::handshake(config))
            })
            .collect(),
    };
    let results = try_join_all(join_handles).await?;
    let event_chains = results.into_iter().collect::<Result<Vec<_>, _>>()?;
    Ok(event_chains)
}

#[derive(Parser, Debug)]
#[command(version)]
#[command(propagate_version = true)]
pub struct HandshakeConfig {
    #[arg(
        long,
        short,
        default_value_t = 500,
        help = "maximum per handshake operation time in ms"
    )]
    pub timeout: u64,
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Btc {
        nodes_addrs: Vec<String>,
        #[arg(
            long,
            short,
            help = "the user agent to be used during handshake operation",
            default_value = "/Satoshi:23.0.0/"
        )]
        user_agent: String,
    },
}

#[derive(Debug)]
pub struct EventChain {
    id: String,
    complete: bool,
    events: Vec<Event>,
}

impl EventChain {
    pub fn new(id: String) -> Self {
        EventChain {
            id,
            events: Vec::new(),
            complete: false,
        }
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

    pub fn mark_as_complete(&mut self) {
        self.complete = true;
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
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
