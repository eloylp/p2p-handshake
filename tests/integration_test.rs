use std::env;

use p2p_handshake::p2p::{handshake, Commands, EventChain, EventDirection, HandshakeConfig};

#[tokio::test]
async fn it_makes_btc_handshake() {
    let nodes_addrs = env::var("TEST_NODES")
        .unwrap()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let config = HandshakeConfig {
        timeout: 500,
        commands: Commands::Btc {
            nodes_addrs,
            user_agent: "/Satoshi:23.0.0/".to_string(),
        },
    };
    let ev_chains = handshake(config).await.unwrap();
    ev_chains.iter().for_each(assert_handshake);
}

fn assert_handshake(ev_chain: &EventChain) {
    assert!(ev_chain.is_complete() == true);
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
