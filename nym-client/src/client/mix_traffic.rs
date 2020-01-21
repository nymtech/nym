use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use sphinx::SphinxPacket;
use std::io;
use std::io::prelude::*;
use std::net::SocketAddr;

pub(crate) struct MixMessage(SocketAddr, SphinxPacket);

impl MixMessage {
    pub(crate) fn new(address: SocketAddr, packet: SphinxPacket) -> Self {
        MixMessage(address, packet)
    }
}

pub(crate) struct MixTrafficController;

impl MixTrafficController {
    // this was way more difficult to implement than what this code may suggest...
    pub(crate) async fn run(mut rx: mpsc::UnboundedReceiver<MixMessage>) {
        let mix_client = mix_client::MixClient::new();
        while let Some(mix_message) = rx.next().await {
            info!(
                "[MIX TRAFFIC CONTROL] - got a mix_message for {:?}",
                mix_message.0
            );
            let send_res = mix_client.send(mix_message.1, mix_message.0).await;
            match send_res {
                Ok(_) => {
                    print!(".");
                    io::stdout().flush().ok().expect("Could not flush stdout");
                }
                Err(e) => error!("We failed to send the message :( - {:?}", e),
            };
        }
    }
}
