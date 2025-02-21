// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use nym_ip_packet_requests::IpPair;
use nym_task::TaskClient;
#[cfg(target_os = "linux")]
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use crate::clients::{ConnectEvent, ConnectedClientEvent, DisconnectEvent};
use crate::{error::Result, util::parse_ip::parse_dst_addr};

// The TUN listener keeps a local map of the connected clients that has its state updated by the
// mixnet listener. Basically it's just so that we don't have to have mutexes around shared state.
// It's even ok if this is slightly out of date
struct ConnectedClientMirror {
    forward_from_tun_tx: mpsc::UnboundedSender<Vec<u8>>,
    ips: IpPair,
}

pub(crate) struct ConnectedClientsListener {
    clients_ipv4: HashMap<Ipv4Addr, ConnectedClientMirror>,
    clients_ipv6: HashMap<Ipv6Addr, ConnectedClientMirror>,
    connected_client_rx: mpsc::UnboundedReceiver<ConnectedClientEvent>,
}

impl ConnectedClientsListener {
    pub(crate) fn new(connected_client_rx: mpsc::UnboundedReceiver<ConnectedClientEvent>) -> Self {
        ConnectedClientsListener {
            clients_ipv4: HashMap::new(),
            clients_ipv6: HashMap::new(),
            connected_client_rx,
        }
    }

    fn get(&self, ip: &IpAddr) -> Option<&ConnectedClientMirror> {
        match ip {
            IpAddr::V4(ip) => self.clients_ipv4.get(ip),
            IpAddr::V6(ip) => self.clients_ipv6.get(ip),
        }
    }

    pub(crate) fn update(&mut self, event: ConnectedClientEvent) {
        match event {
            ConnectedClientEvent::Connect(connected_event) => {
                let ConnectEvent {
                    ips,
                    forward_from_tun_tx,
                } = *connected_event;
                log::trace!("Connect client: {ips}");
                self.clients_ipv4.insert(
                    ips.ipv4,
                    ConnectedClientMirror {
                        forward_from_tun_tx: forward_from_tun_tx.clone(),
                        ips,
                    },
                );
                self.clients_ipv6.insert(
                    ips.ipv6,
                    ConnectedClientMirror {
                        forward_from_tun_tx,
                        ips,
                    },
                );
            }
            ConnectedClientEvent::Disconnect(DisconnectEvent(ips)) => {
                log::trace!("Disconnect client: {ips}");
                self.clients_ipv4.remove(&ips.ipv4);
                self.clients_ipv6.remove(&ips.ipv6);
            }
        }
    }
}

// Reads packet from TUN and writes to mixnet client
#[cfg(target_os = "linux")]
pub(crate) struct TunListener {
    pub(crate) tun_reader: tokio::io::ReadHalf<tokio_tun::Tun>,
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
            forward_from_tun_tx,
            ips,
        }) = self.connected_clients.get(&dst_addr)
        {
            let packet = buf[..len].to_vec();
            if forward_from_tun_tx.send(packet).is_err() {
                log::warn!("Failed to forward packet to connected client {dst_addr}: disconnecting it from tun listener");
                self.connected_clients
                    .update(ConnectedClientEvent::Disconnect(DisconnectEvent(*ips)));
            }
        } else {
            log::info!(
                "dropping packet from network: no registered client for destination: {dst_addr}"
            );
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
