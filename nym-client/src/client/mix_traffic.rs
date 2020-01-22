use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, error, info, trace};
use sphinx::SphinxPacket;
use std::net::SocketAddr;

pub(crate) struct MixMessage(SocketAddr, SphinxPacket);

impl MixMessage {
    pub(crate) fn new(address: SocketAddr, packet: SphinxPacket) -> Self {
        MixMessage(address, packet)
    }
}

pub(crate) struct MixTrafficController;

impl MixTrafficController {
    pub(crate) async fn run(mut rx: mpsc::UnboundedReceiver<MixMessage>) {
        info!("Mix Traffic Controller started!");
        let mix_client = mix_client::MixClient::new();
        while let Some(mix_message) = rx.next().await {
            debug!("Got a mix_message for {:?}", mix_message.0);
            let send_res = mix_client.send(mix_message.1, mix_message.0).await;
            match send_res {
                Ok(_) => {
                    trace!("sent a mix message");
                }
                // TODO: should there be some kind of threshold of failed messages
                // that if reached, the application blows?
                Err(e) => error!(
                    "We failed to send the message to {} :( - {:?}",
                    mix_message.0, e
                ),
            };
        }
    }
}
