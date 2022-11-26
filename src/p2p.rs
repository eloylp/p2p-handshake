use std::{error::Error, time::SystemTime};

use clap::Parser;

mod btc;

pub async fn handshake(config: HandshakeConfig) -> Result<EventChain, Box<dyn Error>> {
    btc::handshake(config).await
}

#[derive(Parser, Debug)]
#[command(version)]
pub struct HandshakeConfig {
    #[arg(short, long)]
    pub node_socket: String,
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
