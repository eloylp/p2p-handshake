use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{SystemTime, UNIX_EPOCH},
};

use bitcoin::network::{
    address,
    constants::{self, ServiceFlags},
    message::{NetworkMessage, RawNetworkMessage},
    message_network::VersionMessage,
};

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
