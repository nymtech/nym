use std::{collections::HashMap, net::IpAddr};

use nym_ip_packet_requests::response::IpPacketResponse;
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};
use nym_task::TaskClient;
#[cfg(target_os = "linux")]
use tokio::io::AsyncReadExt;

use crate::{
    error::{IpPacketRouterError, Result},
    mixnet_listener::{self},
    util::{create_message::create_input_message, parse_ip::parse_dst_addr},
};

// Data flow
// Out: mixnet_listener -> decode -> handle_packet -> write_to_tun
// In: tun_listener -> [connected_client_handler -> encode] -> mixnet_sender

// This handler is spawned as a task, and it listens to IP packets passed from the tun_listener,
// encodes it, and then sends to mixnet.
pub(crate) struct ConnectedClientHandler {
    nym_address: Recipient,
    mix_hops: Option<u8>,
    tun_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    close_rx: tokio::sync::oneshot::Receiver<()>,
}

impl ConnectedClientHandler {
    pub(crate) fn new(
        nym_address: Recipient,
        mix_hops: Option<u8>,
        tun_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
        mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
        close_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Self {
        ConnectedClientHandler {
            nym_address,
            mix_hops,
            tun_rx,
            mixnet_client_sender,
            close_rx,
        }
    }

    async fn handle_packet(&mut self, packet: Vec<u8>) -> Result<()> {
        let response_packet = IpPacketResponse::new_ip_packet(packet.into())
            .to_bytes()
            .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })?;
        let input_message = create_input_message(self.nym_address, response_packet, self.mix_hops);

        self.mixnet_client_sender
            .send(input_message)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })?;

        Ok(())
    }

    async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = &mut self.close_rx => {
                    log::warn!("ConnectedClientHandler: received shutdown");
                    break;
                },
                packet = self.tun_rx.recv() => match packet {
                    Some(packet) => {
                        if let Err(err) = self.handle_packet(packet).await {
                            log::error!("connected client handler: failed to handle packet: {err}");
                        }
                    },
                    None => {
                        log::error!("connected client handler: tun channel closed");
                        break;
                    }
                }
            }
        }

        log::warn!("ConnectedClientHandler: exiting");
        Ok(())
    }

    pub(crate) fn start(self) {
        tokio::spawn(async move {
            if let Err(err) = self.run().await {
                log::error!("connected client handler has failed: {err}")
            }
        });
    }
}
