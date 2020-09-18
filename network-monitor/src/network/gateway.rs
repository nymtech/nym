use futures::channel::mpsc;
use gateway_client::GatewayClient;
use topology::gateway;

pub fn build_gateway_client(gateway: gateway::Node) -> GatewayClient {
    let (mixnet_messages_sender, mixnet_messages_receiver) = mpsc::unbounded();
    let (ack_sender, ack_receiver) = mpsc::unbounded();

    let timeout = time::Duration::from_millis(500);

    gateway_client::GatewayClient::new(
        gateway.client_listener,
        local_identity,
        gateway.identity_key,
        shared_key,
        mixnet_messages_sender,
        ack_sender,
        timeout,
    )
}
