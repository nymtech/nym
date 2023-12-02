use std::{collections::HashMap, net::IpAddr};

use nym_ip_packet_requests::IpPacketResponse;
use nym_sdk::mixnet::{InputMessage, MixnetMessageSender};
use nym_task::{connections::TransmissionLane, TaskClient};
#[cfg(target_os = "linux")]
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{error::IpPacketRouterError, ip_packet_router, util::parse_ip::parse_dst_addr};

// Reads packet from TUN and writes to mixnet client
#[cfg(target_os = "linux")]
pub(crate) struct TunListener {
    pub(crate) tun_reader: tokio::io::ReadHalf<tokio_tun::Tun>,
    pub(crate) mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    pub(crate) task_client: TaskClient,

    // A mirror of the one in IpPacketRouter
    pub(crate) connected_clients: HashMap<IpAddr, ip_packet_router::ConnectedClient>,
    pub(crate) connected_client_rx: UnboundedReceiver<ip_packet_router::ConnectedClientEvent>,
}

#[cfg(target_os = "linux")]
impl TunListener {
    async fn run(mut self) -> Result<(), IpPacketRouterError> {
        let mut buf = [0u8; 65535];
        while !self.task_client.is_shutdown() {
            tokio::select! {
                _ = self.task_client.recv() => {
                    log::trace!("TunListener: received shutdown");
                },
                event = self.connected_client_rx.recv() => match event {
                    Some(ip_packet_router::ConnectedClientEvent::Connect(ip, nym_addr)) => {
                        log::trace!("Connect client: {ip}");
                        self.connected_clients.insert(ip, ip_packet_router::ConnectedClient {
                            nym_address: *nym_addr,
                            last_activity: std::time::Instant::now(),
                        });
                    },
                    Some(ip_packet_router::ConnectedClientEvent::Disconnect(ip)) => {
                        log::trace!("Disconnect client: {ip}");
                        self.connected_clients.remove(&ip);
                    },
                    None => {},
                },
                len = self.tun_reader.read(&mut buf) => match len {
                    Ok(len) => {
                        let Some(dst_addr) = parse_dst_addr(&buf[..len]) else {
                            log::warn!("Failed to parse packet");
                            continue;
                        };

                        let recipient = self.connected_clients.get(&dst_addr).map(|c| c.nym_address);

                        if let Some(recipient) = recipient {
                            let lane = TransmissionLane::General;
                            let packet_type = None;
                            let packet = buf[..len].to_vec();
                            let response_packet = IpPacketResponse::new_ip_packet(packet.into()).to_bytes();
                            let Ok(response_packet) = response_packet else {
                                log::error!("Failed to serialize response packet");
                                continue;
                            };

                            let lane = TransmissionLane::General;
                            let packet_type = None;
                            let mix_hops = Some(0);
                            let input_message = InputMessage::new_regular_custom_hop(*recipient, response_packet, lane, packet_type, mix_hops);

                            // let input_message = InputMessage::new_regular(recipient, response_packet, lane, packet_type);

                            if let Err(err) = self.mixnet_client_sender.send(input_message).await {
                                log::error!("TunListener: failed to send packet to mixnet: {err}");
                            };
                        } else {
                            log::info!("No registered nym-address for packet - dropping");
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
