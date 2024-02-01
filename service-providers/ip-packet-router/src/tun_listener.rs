use bytes::{Bytes, BytesMut, Buf};
use nym_ip_packet_requests::IpPacketResponse;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_task::TaskClient;
#[cfg(target_os = "linux")]
use tokio::io::AsyncReadExt;
use tokio_util::codec::{Encoder, Decoder};

use crate::{
    error::{IpPacketRouterError, Result},
    mixnet_listener::{self},
    util::{create_message::create_input_message, parse_ip::parse_dst_addr},
};

// Reads packet from TUN and writes to mixnet client
#[cfg(target_os = "linux")]
pub(crate) struct TunListener {
    pub(crate) tun_reader: tokio::io::ReadHalf<tokio_tun::Tun>,
    pub(crate) mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    pub(crate) task_client: TaskClient,
    pub(crate) connected_clients: mixnet_listener::ConnectedClientsListener,
}

#[cfg(target_os = "linux")]
impl TunListener {
    async fn handle_packet(&mut self, buf: &[u8], len: usize, bundled_packet_codec: &mut mixnet_listener::BundledIpPacketCodec) -> Result<()> {
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

            // Bunch together
            let packet_bytes = Bytes::from(packet);
            let mut bundled_packets = BytesMut::new();
            bundled_packet_codec.encode(packet_bytes, &mut bundled_packets).unwrap();
            if bundled_packets.is_empty() {
                return Ok(());
            }
            let bundled_packets = bundled_packets.freeze();

            // let response_packet = IpPacketResponse::new_ip_packet(packet.into())
            let response_packet = IpPacketResponse::new_ip_packet(bundled_packets)
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

    async fn run(mut self) -> Result<()> {
        let mut buf = [0u8; 65535];

        let mut bundled_packet_codec = mixnet_listener::BundledIpPacketCodec::new();

        while !self.task_client.is_shutdown() {
            tokio::select! {
                _ = self.task_client.recv() => {
                    log::trace!("TunListener: received shutdown");
                },
                // TODO: ConnectedClientsListener::update should poll the channel instead
                event = self.connected_clients.connected_client_rx.recv() => match event {
                    Some(event) => self.connected_clients.update(event),
                    None => {
                        log::error!("TunListener: connected client channel closed");
                        break;
                    },
                },
                len = self.tun_reader.read(&mut buf) => match len {
                    Ok(len) => {
                        if let Err(err) = self.handle_packet(&buf, len, &mut bundled_packet_codec).await {
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
