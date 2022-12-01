use std::{
    fmt::{self, Display},
    ops::Add,
    time::{Duration, Instant},
};

use super::P2PError;

pub const EMOJI_SUCCESS: &str = "\u{2705}";
pub const EMOJI_WARNING: &str = "\u{26A0}\u{FE0F}";
pub const EMOJI_FAILURE: &str = "\u{274C}";
pub const EMOJI_TIMEOUT: &str = "\u{274C} \u{1F550}";
pub const EMOJI_DIRECTION_OUT: &str = "\u{1F6EB}";
pub const EMOJI_DIRECTION_IN: &str = "\u{1F6EC}";

pub struct HandshakeResult {
    id: String,
    result: Result<EventChain, P2PError>,
}

impl HandshakeResult {
    pub fn new(id: String, result: Result<EventChain, P2PError>) -> HandshakeResult {
        HandshakeResult { id, result }
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    pub fn result(&self) -> Result<&EventChain, &P2PError> {
        self.result.as_ref()
    }
}

impl Display for HandshakeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.result.is_ok() {
            true => {
                write!(f, "{}", self.result().unwrap())
            }
            false => {
                write!(
                    f,
                    "{} {}: {}",
                    EMOJI_FAILURE,
                    self.id,
                    self.result().err().unwrap()
                )
            }
        }
    }
}

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

impl Display for EventChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.is_complete() {
            EMOJI_SUCCESS
        } else {
            EMOJI_TIMEOUT
        };
        write!(f, "{} - {}", status, self.id())?;
        write!(f, " || ")?;

        let mut last_ev: Option<&Event> = None;
        let mut total_time_millis = Duration::from_millis(0);
        for ev in self.events.iter() {
            let elapsed_time = match last_ev {
                Some(l_ev) => ev.time().duration_since(l_ev.time()),
                None => Duration::from_millis(0),
            };
            total_time_millis = total_time_millis.add(elapsed_time);
            if last_ev.is_some() {
                write!(f, " -- {:#?} --> ", elapsed_time)?;
            }
            write!(f, "{}", ev)?;
            last_ev = Some(ev);
        }
        write!(f, " || total time {:#?}.", total_time_millis)
    }
}

pub struct Event {
    name: String,
    time: Instant,
    direction: EventDirection,
    data_pairs: Vec<(String, String)>,
}

impl Event {
    pub fn new(name: String, direction: EventDirection) -> Event {
        Event {
            name,
            direction,
            time: Instant::now(),
            data_pairs: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn time(&self) -> Instant {
        self.time
    }

    pub fn direction(&self) -> &EventDirection {
        &self.direction
    }

    pub fn data_pairs(&self) -> &[(String, String)] {
        self.data_pairs.as_ref()
    }

    pub fn set_pair(&mut self, key: String, val: String) {
        self.data_pairs.push((key, val));
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name(), self.direction())?;
        if !self.data_pairs.is_empty() {
            let mut pairs = String::new();
            pairs.push_str(" (");
            self.data_pairs
                .iter()
                .for_each(|(k, v)| pairs.push_str(format!("{}:{} ", k, v).as_str()));
            pairs = pairs.trim_end().to_string();
            pairs.push(')');
            write!(f, "{}", pairs)?;
        }
        Ok(())
    }
}

pub enum EventDirection {
    IN,
    OUT,
}

impl Display for EventDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let direction = match self {
            EventDirection::IN => EMOJI_DIRECTION_IN,
            EventDirection::OUT => EMOJI_DIRECTION_OUT,
        };
        write!(f, "{}", direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_displays_correctly() {
        let mut event = Event::new("ev_1".to_string(), EventDirection::IN);
        event.set_pair("k1".to_string(), "v1".to_string());
        event.set_pair("k2".to_string(), "v2".to_string());

        assert_eq!(
            format!("ev_1 {} (k1:v1 k2:v2)", EMOJI_DIRECTION_IN),
            event.to_string()
        )
    }

    #[test]
    fn event_chain_shows_nice_user_output_on_success() {
        let mut chain = EventChain::new("192.168.1.1:8333".to_string());

        let fixed_time = Instant::now();

        let mut event = Event {
            name: "version".to_string(),
            direction: EventDirection::OUT,
            time: fixed_time,
            data_pairs: Vec::new(),
        };

        event.set_pair("k1".to_string(), "v1".to_string());
        event.set_pair("k2".to_string(), "v2".to_string());

        chain.add(event);

        chain.add(Event {
            name: "version".to_string(),
            direction: EventDirection::IN,
            time: fixed_time.add(Duration::from_millis(100)),
            data_pairs: Vec::new(),
        });

        chain.add(Event {
            name: "verack".to_string(),
            direction: EventDirection::IN,
            time: fixed_time.add(Duration::from_millis(120)),
            data_pairs: Vec::new(),
        });

        chain.add(Event {
            name: "verack".to_string(),
            direction: EventDirection::OUT,
            time: fixed_time.add(Duration::from_millis(140)),
            data_pairs: Vec::new(),
        });

        chain.mark_as_complete();

        let output = chain.to_string();

        assert_eq!(
            format!("{} - 192.168.1.1:8333 || version {} (k1:v1 k2:v2) -- 100ms --> version {} -- 20ms --> verack {} -- 20ms --> verack {} || total time 140ms.", 
            EMOJI_SUCCESS, EMOJI_DIRECTION_OUT, EMOJI_DIRECTION_IN, EMOJI_DIRECTION_IN, EMOJI_DIRECTION_OUT),
            output
        )
    }

    #[test]
    fn incomplete_event_chain_shows_nice_user_output() {
        let mut chain = EventChain::new("192.168.1.1:8333".to_string());

        let fixed_time = Instant::now();

        chain.add(Event {
            name: "version".to_string(),
            direction: EventDirection::OUT,
            time: fixed_time,
            data_pairs: Vec::new(),
        });

        chain.add(Event {
            name: "version".to_string(),
            direction: EventDirection::IN,
            time: fixed_time.add(Duration::from_millis(100)),
            data_pairs: Vec::new(),
        });

        let output = chain.to_string();

        assert_eq!(
            format!(
                "{} - 192.168.1.1:8333 || version {} -- 100ms --> version {} || total time 100ms.",
                EMOJI_TIMEOUT, EMOJI_DIRECTION_OUT, EMOJI_DIRECTION_IN
            ),
            output
        )
    }

    #[test]
    fn handshake_result_displays_event_chain_on_success() {
        let id = "192.168.1.1:8333".to_string();

        let mut event_chain = EventChain::new(id.clone());
        event_chain.add(Event::new("version".to_string(), EventDirection::IN));
        event_chain.mark_as_complete();

        let result: Result<EventChain, P2PError> = Result::Ok(event_chain);

        let hr = HandshakeResult {
            id: id.clone(),
            result,
        };

        assert_eq!(
            format!(
                "{} - 192.168.1.1:8333 || version {} || total time 0ns.",
                EMOJI_SUCCESS, EMOJI_DIRECTION_IN
            ),
            hr.to_string()
        )
    }

    #[test]
    fn handshake_result_displays_error_on_failure() {
        let id = "192.168.1.1:8333".to_string();

        let error = P2PError {
            message: "connection refused !".to_string(),
        };

        let result: Result<EventChain, P2PError> = Result::Err(error);

        let hr = HandshakeResult {
            id: id.clone(),
            result,
        };

        assert_eq!(
            format!(
                "{} 192.168.1.1:8333: P2P error: connection refused !",
                EMOJI_FAILURE
            ),
            hr.to_string()
        )
    }
}
