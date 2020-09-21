use std::{sync::Arc, time};

use crypto::asymmetric::identity;
use gateway_client::GatewayClient;
use topology::gateway;

use super::{AckSender, MixnetSender};

pub fn new_gateway_client(
    gateway: gateway::Node,
    identity: Arc<identity::KeyPair>,
    ack_sender: AckSender,
    mixnet_messages_sender: MixnetSender,
) -> GatewayClient {
    let timeout = time::Duration::from_millis(500);

    gateway_client::GatewayClient::new(
        gateway.client_listener,
        identity,
        gateway.identity_key,
        None,
        mixnet_messages_sender,
        ack_sender,
        timeout,
    )
}
