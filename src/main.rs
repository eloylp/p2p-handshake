use std::process::exit;

use clap::Parser;
use p2p_handshake::p2p::{config::HandshakeConfig, handshake};

#[tokio::main]
async fn main() {
    let config = HandshakeConfig::parse();
    match handshake(config).await {
        Ok(handshake_result) => handshake_result.iter().for_each(|hr| println!("{}", hr)),
        Err(err) => {
            println!("{}", err);
            exit(1)
        }
    }
}
