use std::env;

use p2p_handshake::p2p::{btc_handshake, EventDirection, HandshakeConfig};

#[tokio::test]
async fn it_makes_btc_handshake() {
    let args = HandshakeConfig {
        node_socket: env::var("TEST_NODE").unwrap(),
    };
    let ev_chain = btc_handshake(args).await.unwrap();

    assert!(ev_chain.len() == 4);

    assert!(ev_chain.get(0).unwrap().name() == "VERSION");
    assert!(matches!(
        ev_chain.get(0).unwrap().direction(),
        EventDirection::OUT
    ));

    assert!(ev_chain.get(1).unwrap().name() == "VERSION");
    assert!(matches!(
        ev_chain.get(1).unwrap().direction(),
        EventDirection::IN
    ));

    assert!(ev_chain.get(2).unwrap().name() == "ACK");
    assert!(matches!(
        ev_chain.get(2).unwrap().direction(),
        EventDirection::OUT
    ));

    assert!(ev_chain.get(3).unwrap().name() == "ACK");
    assert!(matches!(
        ev_chain.get(3).unwrap().direction(),
        EventDirection::IN
    ));
}
