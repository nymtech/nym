use std::{collections::HashMap, net::IpAddr};

use nym_sdk::mixnet::Recipient;
use nym_task::TaskClient;
#[cfg(target_os = "linux")]
use tokio::io::AsyncReadExt;

use crate::{
    error::Result,
    mixnet_listener::{self},
    util::parse_ip::parse_dst_addr,
};

pub(crate) struct ConnectedClientMirror {
    pub(crate) nym_address: Recipient,
    pub(crate) mix_hops: Option<u8>,
    pub(crate) last_activity: std::time::Instant,
    // Forward packets we read from the TUN device to the connected clients listener
    pub(crate) forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

pub(crate) struct ConnectedClientsListener {
    clients: HashMap<IpAddr, ConnectedClientMirror>,
    connected_client_rx:
        tokio::sync::mpsc::UnboundedReceiver<mixnet_listener::ConnectedClientEvent>,
}

impl ConnectedClientsListener {
    pub(crate) fn new(
        connected_client_rx: tokio::sync::mpsc::UnboundedReceiver<
            mixnet_listener::ConnectedClientEvent,
        >,
    ) -> Self {
        ConnectedClientsListener {
            clients: HashMap::new(),
            connected_client_rx,
        }
    }

    pub(crate) fn get(&self, ip: &IpAddr) -> Option<&ConnectedClientMirror> {
        self.clients.get(ip)
    }

    pub(crate) fn update(&mut self, event: mixnet_listener::ConnectedClientEvent) {
        match event {
            mixnet_listener::ConnectedClientEvent::Connect(connected_event) => {
                let mixnet_listener::ConnectEvent {
                    ip,
                    nym_address,
                    mix_hops,
                    forward_from_tun_tx,
                } = *connected_event;
                log::trace!("Connect client: {ip}");
                self.clients.insert(
                    ip,
                    ConnectedClientMirror {
                        nym_address,
                        mix_hops,
                        last_activity: std::time::Instant::now(),
                        forward_from_tun_tx,
                    },
                );
            }
            mixnet_listener::ConnectedClientEvent::Disconnect(
                mixnet_listener::DisconnectEvent(ip),
            ) => {
                log::trace!("Disconnect client: {ip}");
                self.clients.remove(&ip);
            }
        }
    }
}

// Reads packet from TUN and writes to mixnet client
#[cfg(target_os = "linux")]
pub(crate) struct TunListener {
    pub(crate) tun_reader: tokio::io::ReadHalf<tokio_tun::Tun>,
    // pub(crate) mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    pub(crate) task_client: TaskClient,
    pub(crate) connected_clients: ConnectedClientsListener,
}

#[cfg(target_os = "linux")]
impl TunListener {
    async fn handle_packet(&mut self, buf: &[u8], len: usize) -> Result<()> {
        let Some(dst_addr) = parse_dst_addr(&buf[..len]) else {
            log::warn!("Failed to parse packet");
            return Ok(());
        };

        if let Some(ConnectedClientMirror {
            nym_address,
            mix_hops,
            last_activity,
            forward_from_tun_tx,
        }) = self.connected_clients.get(&dst_addr)
        {
            let packet = buf[..len].to_vec();
            forward_from_tun_tx.send(packet).unwrap();
        } else {
            log::info!("No registered nym-address for packet - dropping");
        }

        Ok(())
    }

    async fn run(mut self) -> Result<()> {
        let mut buf = [0u8; 65535];
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
