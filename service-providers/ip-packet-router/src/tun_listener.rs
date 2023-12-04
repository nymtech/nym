use std::{collections::HashMap, net::IpAddr};

use nym_ip_packet_requests::IpPacketResponse;
use nym_sdk::mixnet::{InputMessage, MixnetMessageSender, Recipient};
use nym_task::{connections::TransmissionLane, TaskClient};
#[cfg(target_os = "linux")]
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    error::{IpPacketRouterError, Result},
    mixnet_listener,
    util::parse_ip::parse_dst_addr,
};

// Reads packet from TUN and writes to mixnet client
#[cfg(target_os = "linux")]
pub(crate) struct TunListener {
    pub(crate) tun_reader: tokio::io::ReadHalf<tokio_tun::Tun>,
    pub(crate) mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    pub(crate) task_client: TaskClient,

    // A mirror of the one in IpPacketRouter
    pub(crate) connected_clients: HashMap<IpAddr, mixnet_listener::ConnectedClient>,
    pub(crate) connected_client_rx: UnboundedReceiver<mixnet_listener::ConnectedClientEvent>,
}

fn create_input_message(
    nym_address: Recipient,
    response_packet: Vec<u8>,
    mix_hops: Option<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    if let Some(mix_hops) = mix_hops {
        InputMessage::new_regular_with_custom_hops(
            nym_address,
            response_packet,
            lane,
            packet_type,
            mix_hops,
        )
    } else {
        InputMessage::new_regular(nym_address, response_packet, lane, packet_type)
    }
}

#[cfg(target_os = "linux")]
impl TunListener {
    async fn handle_packet(&mut self, buf: &[u8], len: usize) -> Result<()> {
        let Some(dst_addr) = parse_dst_addr(&buf[..len]) else {
            log::warn!("Failed to parse packet");
            return Ok(());
        };

        if let Some(mixnet_listener::ConnectedClient {
            nym_address,
            mix_hops,
            ..
        }) = self.connected_clients.get(&dst_addr)
        {
            let packet = buf[..len].to_vec();
            let response_packet = IpPacketResponse::new_ip_packet(packet.into())
                .to_bytes()
                .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket {
                    source: err,
                })?;
            let input_message = create_input_message(*nym_address, response_packet, *mix_hops);

            self.mixnet_client_sender
                .send(input_message)
                .await
                .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })?;
        } else {
            log::info!("No registered nym-address for packet - dropping");
        }

        Ok(())
    }

    async fn handle_connected_client_event(
        &mut self,
        event: mixnet_listener::ConnectedClientEvent,
    ) {
        match event {
            mixnet_listener::ConnectedClientEvent::Connect(mixnet_listener::ConnectEvent {
                ip,
                nym_address,
                mix_hops,
            }) => {
                log::trace!("Connect client: {ip}");
                self.connected_clients.insert(
                    ip,
                    mixnet_listener::ConnectedClient {
                        nym_address,
                        mix_hops,
                        last_activity: std::time::Instant::now(),
                    },
                );
            }
            mixnet_listener::ConnectedClientEvent::Disconnect(
                mixnet_listener::DisconnectEvent(ip),
            ) => {
                log::trace!("Disconnect client: {ip}");
                self.connected_clients.remove(&ip);
            }
        }
    }

    async fn run(mut self) -> Result<()> {
        let mut buf = [0u8; 65535];
        while !self.task_client.is_shutdown() {
            tokio::select! {
                _ = self.task_client.recv() => {
                    log::trace!("TunListener: received shutdown");
                },
                event = self.connected_client_rx.recv() => match event {
                    Some(event) => self.handle_connected_client_event(event).await,
                    None => {
                        log::error!("TunListener: connected client channel closed");
                        break;
                    },
                },
                len = self.tun_reader.read(&mut buf) => match len {
                    Ok(len) => {
                        if let Err(err) = self.handle_packet(&buf, len).await {
                            log::error!("tun: failed to handle packet: {err}");
                        }
                    },
                    Err(err) => {
                        log::warn!("iface: read error: {err}");
                        // break;
                    }
                }
            }
        }
        log::debug!("TunListener: stopping");
        Ok(())
    }

    pub(crate) fn start(self) {
        tokio::spawn(async move {
            if let Err(err) = self.run().await {
                log::error!("tun listener router has failed: {err}")
            }
        });
    }
}
