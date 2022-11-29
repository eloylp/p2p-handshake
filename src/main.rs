use std::process::exit;

use clap::Parser;
use p2p_handshake::p2p;

#[tokio::main]
async fn main() {
    let config = p2p::HandshakeConfig::parse();
    match p2p::handshake(config).await {
        Ok(ev_chains) => ev_chains.iter().for_each(|ev| println!("{:?}", ev)),
        Err(err) => {
            println!("error executing p2p handshake: {}", err);
            exit(1)
        }
    }
}
