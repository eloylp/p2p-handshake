use std::env;

use p2p_handshake::p2p::{handshake, Commands, EventDirection, HandshakeConfig};

#[tokio::test]
async fn it_makes_btc_handshake() {
    let nodes_addrs = vec![env::var("TEST_NODE").unwrap()];
    let args = HandshakeConfig {
        timeout: 200,
        commands: Commands::Btc { nodes_addrs },
    };
    let ev_chains = handshake(args).await.unwrap();
    let ev_chain = ev_chains.first().unwrap();

    assert!(ev_chain.len() == 4);

    assert!(ev_chain.get(0).unwrap().name() == "version");
    assert!(matches!(
        ev_chain.get(0).unwrap().direction(),
        EventDirection::OUT
    ));

    assert!(ev_chain.get(1).unwrap().name() == "version");
    assert!(matches!(
        ev_chain.get(1).unwrap().direction(),
        EventDirection::IN
    ));

    assert!(ev_chain.get(2).unwrap().name() == "verack");
    assert!(matches!(
        ev_chain.get(2).unwrap().direction(),
        EventDirection::IN
    ));

    assert!(ev_chain.get(3).unwrap().name() == "verack");
    assert!(matches!(
        ev_chain.get(3).unwrap().direction(),
        EventDirection::OUT
    ));
}
