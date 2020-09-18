use std::{sync::Arc, time};

use crypto::asymmetric::identity;
use futures::channel::mpsc;
use gateway_client::GatewayClient;
use rand::rngs::OsRng;
use topology::gateway;

const DEFAULT_RNG: OsRng = OsRng;

pub fn new_gateway_client(gateway: gateway::Node) -> GatewayClient {
    let (mixnet_messages_sender, mixnet_messages_receiver) = mpsc::unbounded();
    let (ack_sender, ack_receiver) = mpsc::unbounded();

    let local_identity = Arc::new(identity::KeyPair::new_with_rng(&mut DEFAULT_RNG));

    let timeout = time::Duration::from_millis(500);

    gateway_client::GatewayClient::new(
        gateway.client_listener,
        local_identity,
        gateway.identity_key,
        None,
        mixnet_messages_sender,
        ack_sender,
        timeout,
    )
}
