use futures::channel::mpsc;
use futures::StreamExt;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;

pub(crate) struct PacketForwarder {
    tcp_client: multi_tcp_client::Client,
    conn_tx: mpsc::UnboundedSender<(SocketAddr, Vec<u8>)>,
    conn_rx: mpsc::UnboundedReceiver<(SocketAddr, Vec<u8>)>,
}

impl PacketForwarder {
    pub(crate) fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
    ) -> PacketForwarder {
        let tcp_client_config = multi_tcp_client::Config::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
        );

        let (conn_tx, conn_rx) = mpsc::unbounded();

        PacketForwarder {
            tcp_client: multi_tcp_client::Client::new(tcp_client_config),
            conn_tx,
            conn_rx,
        }
    }

    pub(crate) fn start(mut self, handle: &Handle) -> mpsc::UnboundedSender<(SocketAddr, Vec<u8>)> {
        // TODO: what to do with the lost JoinHandle?
        let sender_channel = self.conn_tx.clone();
        handle.spawn(async move {
            while let Some((address, packet)) = self.conn_rx.next().await {
                // as a mix node we don't care about responses, we just want to fire packets
                // as quickly as possible
                self.tcp_client.send(address, packet, false).await.unwrap(); // if we're not waiting for response, we MUST get an Ok
            }
        });
        sender_channel
    }
}
