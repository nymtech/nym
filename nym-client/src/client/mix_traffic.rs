use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use sphinx::SphinxPacket;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub(crate) struct MixMessage(SocketAddr, SphinxPacket);
pub(crate) type MixMessageSender = mpsc::UnboundedSender<MixMessage>;
pub(crate) type MixMessageReceiver = mpsc::UnboundedReceiver<MixMessage>;

impl MixMessage {
    pub(crate) fn new(address: SocketAddr, packet: SphinxPacket) -> Self {
        MixMessage(address, packet)
    }
}

pub(crate) struct MixTrafficController {
    tcp_client: multi_tcp_client::Client,
    mix_rx: MixMessageReceiver,
}

impl MixTrafficController {
    pub(crate) fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        mix_rx: MixMessageReceiver,
    ) -> Self {
        let tcp_client_config = multi_tcp_client::Config::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
        );

        MixTrafficController {
            tcp_client: multi_tcp_client::Client::new(tcp_client_config),
            mix_rx,
        }
    }

    async fn on_message(&mut self, mix_message: MixMessage) {
        debug!("Got a mix_message for {:?}", mix_message.0);
        self.tcp_client
            // TODO: possibly we might want to get an actual result here at some point
            .send(mix_message.0, mix_message.1.to_bytes(), false)
            .await
            .unwrap(); // if we're not waiting for response, we MUST get an Ok
    }

    pub(crate) async fn run(&mut self) {
        while let Some(mix_message) = self.mix_rx.next().await {
            self.on_message(mix_message).await;
        }
    }

    pub(crate) fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            self.run().await;
        })
    }
}
