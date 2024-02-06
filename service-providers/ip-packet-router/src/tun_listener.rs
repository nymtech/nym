use std::time::Duration;

use bytes::{Buf, Bytes, BytesMut};
use nym_ip_packet_requests::IpPacketResponse;
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};
use nym_task::TaskClient;
#[cfg(target_os = "linux")]
use tokio::io::AsyncReadExt;
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    error::{IpPacketRouterError, Result},
    mixnet_listener::{self},
    util::{create_message::create_input_message, parse_ip::parse_dst_addr},
};

// Data flow is
// mixnet_listener -> decode -> handle_packet -> write_to_tun
// tun_listener -> (task: send to connected client handler for processing -> encode) -> mixnet_sender
// This handler is spawned as a task, and it listens to IP packets passed from the tun_listener,
// encodes it, and then sends to mixnet.
pub(crate) struct ConnectedClientHandler {
    nym_address: Recipient,
    mix_hops: Option<u8>,
    tun_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    codec: mixnet_listener::BundledIpPacketCodec,
    codec_timer: tokio::time::Interval,
    close_rx: tokio::sync::oneshot::Receiver<()>,
}

impl ConnectedClientHandler {
    pub(crate) fn new(
        nym_address: Recipient,
        mix_hops: Option<u8>,
        tun_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
        mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
        close_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Self {
        Self {
            nym_address,
            mix_hops,
            tun_rx,
            mixnet_client_sender,
            codec: mixnet_listener::BundledIpPacketCodec::new(),
            codec_timer: tokio::time::interval(Duration::from_millis(20)),
            close_rx,
        }
    }

    async fn flush_current_bundled_packets_and_send(
        &mut self,
        // bundled_packet_codec: &mut mixnet_listener::BundledIpPacketCodec,
        // bundle_timer: &mut tokio::time::Interval,
    ) -> Result<()> {
        let mut bundled_packets = self.codec.flush_current_buffer();
        if !bundled_packets.is_empty() {
            // // TEMPORARY
            // let Some(connect_client) = self.connected_clients.get_first() else {
            //     return Ok(());
            // };
            // let mixnet_listener::ConnectedClient {
            //     nym_address,
            //     mix_hops,
            //     ..
            // } = connect_client;

            let response_packet = IpPacketResponse::new_ip_packet(bundled_packets)
                .to_bytes()
                .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket {
                    source: err,
                })?;
            let input_message =
                create_input_message(self.nym_address, response_packet, self.mix_hops);

            self.mixnet_client_sender
                .send(input_message)
                .await
                .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })?;
        }
        Ok(())
    }

    async fn handle_packet(&mut self, packet: Vec<u8>) -> Result<()> {
        // If we are the first packet, start the timer
        if self.codec.is_empty() {
            self.codec_timer.reset();
        }

        // Bunch together
        let packet_bytes = Bytes::from(packet);
        let mut bundled_packets = BytesMut::new();
        self.codec
            .encode(packet_bytes, &mut bundled_packets)
            .unwrap();
        if bundled_packets.is_empty() {
            return Ok(());
        }
        let bundled_packets = bundled_packets.freeze();
        self.codec_timer.reset();

        // let response_packet = IpPacketResponse::new_ip_packet(packet.into())
        let response_packet = IpPacketResponse::new_ip_packet(bundled_packets)
            .to_bytes()
            .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })?;
        let input_message = create_input_message(self.nym_address, response_packet, self.mix_hops);

        self.mixnet_client_sender
            .send(input_message)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })?;
        Ok(())
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = &mut self.close_rx => {
                    log::trace!("ConnectedClientHandler: received shutdown");
                    break;
                },
                _ = self.codec_timer.tick() => {
                    if let Err(err) = self.flush_current_bundled_packets_and_send().await {
                        log::error!("connected client handler: failed to flush and send bundled packets: {err}");
                    }
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
    // async fn flush_current_bundled_packets_and_send(
    //     &mut self,
    //     bundled_packet_codec: &mut mixnet_listener::BundledIpPacketCodec,
    //     bundle_timer: &mut tokio::time::Interval,
    // ) -> Result<()> {
    //     let mut bundled_packets = bundled_packet_codec.flush_current_buffer();
    //     if !bundled_packets.is_empty() {
    //         // TEMPORARY
    //         let Some(connect_client) = self.connected_clients.get_first() else {
    //             return Ok(());
    //         };
    //         let mixnet_listener::ConnectedClient {
    //             nym_address,
    //             mix_hops,
    //             ..
    //         } = connect_client;
    //
    //         let response_packet = IpPacketResponse::new_ip_packet(bundled_packets)
    //             .to_bytes()
    //             .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket {
    //                 source: err,
    //             })?;
    //         let input_message = create_input_message(*nym_address, response_packet, *mix_hops);
    //
    //         self.mixnet_client_sender
    //             .send(input_message)
    //             .await
    //             .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })?;
    //     }
    //     Ok(())
    // }

    async fn handle_packet(
        &mut self,
        buf: &[u8],
        len: usize,
        bundled_packet_codec: &mut mixnet_listener::BundledIpPacketCodec,
        bundle_timer: &mut tokio::time::Interval,
    ) -> Result<()> {
        let Some(dst_addr) = parse_dst_addr(&buf[..len]) else {
            log::warn!("Failed to parse packet");
            return Ok(());
        };

        if let Some(mixnet_listener::ConnectedClient {
            nym_address,
            mix_hops,
            tun_tx,
            ..
        }) = self.connected_clients.get(&dst_addr)
        {
            let packet = buf[..len].to_vec();

            tun_tx.send(packet).unwrap();

            // // If we are the first packet, start the timer
            // if bundled_packet_codec.is_empty() {
            //     bundle_timer.reset();
            // }
            //
            // // Bunch together
            // let packet_bytes = Bytes::from(packet);
            // let mut bundled_packets = BytesMut::new();
            // bundled_packet_codec
            //     .encode(packet_bytes, &mut bundled_packets)
            //     .unwrap();
            // if bundled_packets.is_empty() {
            //     return Ok(());
            // }
            // let bundled_packets = bundled_packets.freeze();
            // bundle_timer.reset();
            //
            // // let response_packet = IpPacketResponse::new_ip_packet(packet.into())
            // let response_packet = IpPacketResponse::new_ip_packet(bundled_packets)
            //     .to_bytes()
            //     .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket {
            //         source: err,
            //     })?;
            // let input_message = create_input_message(*nym_address, response_packet, *mix_hops);
            //
            // self.mixnet_client_sender
            //     .send(input_message)
            //     .await
            //     .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })?;
        } else {
            log::info!("No registered nym-address for packet - dropping");
        }

        Ok(())
    }

    async fn run(mut self) -> Result<()> {
        let mut buf = [0u8; 65535];

        let mut bundled_packet_codec = mixnet_listener::BundledIpPacketCodec::new();
        // tokio timer for flushing the buffer
        let mut bundle_timer = tokio::time::interval(Duration::from_millis(20));

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
                // _ = bundle_timer.tick() => {
                //     if let Err(err) = self.flush_current_bundled_packets_and_send(&mut bundled_packet_codec, &mut bundle_timer).await {
                //         log::error!("tun: failed to flush and send bundled packets: {err}");
                //     }
                // },
                len = self.tun_reader.read(&mut buf) => match len {
                    Ok(len) => {
                        if let Err(err) = self.handle_packet(&buf, len, &mut bundled_packet_codec, &mut bundle_timer).await {
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
