use std::env;

use p2p_handshake::p2p::{
    config::{Commands, HandshakeConfig},
    handshake,
    view::{EventDirection, HandshakeResult},
};

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
    handshake(config)
        .await
        .unwrap()
        .iter()
        .for_each(assert_handshake);
}

fn assert_handshake(result: &HandshakeResult) {
    let ev_chain = result.result().unwrap();

    assert!(ev_chain.is_complete() == true);
    assert!(ev_chain.len() == 4);

    assert!(ev_chain.get(0).unwrap().name().eq("version"));
    assert!(matches!(
        ev_chain.get(0).unwrap().direction(),
        EventDirection::OUT
    ));

    assert!(ev_chain.get(1).unwrap().name().eq("version"));
    assert!(matches!(
        ev_chain.get(1).unwrap().direction(),
        EventDirection::IN
    ));

    // Last 2 events should be the "verack" (IN and OUT) and they can happen at any time.
    // In order to make this tests more resilient, we just check types and that their
    // directions are different.
    assert!(ev_chain.get(2).unwrap().name().eq("verack"));
    assert!(ev_chain.get(3).unwrap().name().eq("verack"));

    let direction_2 = ev_chain.get(2).unwrap().direction();
    let direction_3 = ev_chain.get(3).unwrap().direction();
    assert!(direction_2.to_string() != direction_3.to_string());
}
