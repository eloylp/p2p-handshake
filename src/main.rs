use clap::Parser;
use p2p_handshake::p2p;

#[tokio::main]
async fn main() {
    let config = p2p::HandshakeConfig::parse();
    p2p::handshake(config).await.unwrap();
}
