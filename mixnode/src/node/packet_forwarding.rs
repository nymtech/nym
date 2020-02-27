use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;

pub(crate) struct PacketForwarder<'a> {
    tcp_client: multi_tcp_client::Client<'a>,
    conn_tx: mpsc::UnboundedSender<(SocketAddr, Vec<u8>)>,
    conn_rx: mpsc::UnboundedReceiver<(SocketAddr, Vec<u8>)>,
}

impl<'a: 'static> PacketForwarder<'a> {
    pub(crate) async fn new(
        initial_endpoints: Vec<SocketAddr>,
        reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
    ) -> PacketForwarder<'a> {
        let tcp_client_config = multi_tcp_client::Config::new(
            initial_endpoints,
            reconnection_backoff,
            maximum_reconnection_backoff,
        );

        let (conn_tx, conn_rx) = mpsc::unbounded();

        PacketForwarder {
            tcp_client: multi_tcp_client::Client::new(tcp_client_config).await,
            conn_tx,
            conn_rx,
        }
    }

    pub(crate) fn start(mut self, handle: &Handle) -> mpsc::UnboundedSender<(SocketAddr, Vec<u8>)> {
        // TODO: what to do with the lost JoinHandle?
        let sender_channel = self.conn_tx.clone();
        handle.spawn(async move {
            while let Some((address, packet)) = self.conn_rx.next().await {
                match self.tcp_client.send(address, &packet).await {
                    Err(e) => warn!("Failed to forward packet to {:?} - {:?}", address, e),
                    Ok(_) => trace!("Forwarded packet to {:?}", address),
                }
            }
        });
        sender_channel
    }
}
