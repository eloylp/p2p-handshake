use std::process::exit;

use clap::Parser;
use p2p_handshake::p2p;

#[tokio::main]
async fn main() {
    let config = p2p::HandshakeConfig::parse();
    if let Err(err) = p2p::handshake(config).await {
        println!("error executing p2p handshake: {}", err);
        exit(1)
    }
}
