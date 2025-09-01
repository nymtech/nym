// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::Instant,
};

use nym_ip_packet_requests::IpPair;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::{
    constants::CLIENT_MIXNET_INACTIVITY_TIMEOUT,
    error::{IpPacketRouterError, Result},
    tun_listener,
};

use super::ConnectedClientId;

pub(crate) struct ConnectedClients {
    // The set of connected clients
    clients_ipv4_mapping: HashMap<Ipv4Addr, ConnectedClient>,
    clients_ipv6_mapping: HashMap<Ipv6Addr, ConnectedClient>,

    // Notify the tun listener when a new client connects or disconnects
    tun_listener_connected_client_tx: mpsc::UnboundedSender<ConnectedClientEvent>,
}

impl ConnectedClients {
    pub(crate) fn new() -> (Self, tun_listener::ConnectedClientsListener) {
        let (connected_client_tx, connected_client_rx) = mpsc::unbounded_channel();
        (
            Self {
                clients_ipv4_mapping: Default::default(),
                clients_ipv6_mapping: Default::default(),
                tun_listener_connected_client_tx: connected_client_tx,
            },
            tun_listener::ConnectedClientsListener::new(connected_client_rx),
        )
    }

    pub(crate) fn is_ip_connected(&self, ips: &IpPair) -> bool {
        self.clients_ipv4_mapping.contains_key(&ips.ipv4)
            || self.clients_ipv6_mapping.contains_key(&ips.ipv6)
    }

    pub(crate) fn get_client_from_ip_mut(&mut self, ip: &IpAddr) -> Option<&mut ConnectedClient> {
        match ip {
            IpAddr::V4(ip) => self.clients_ipv4_mapping.get_mut(ip),
            IpAddr::V6(ip) => self.clients_ipv6_mapping.get_mut(ip),
        }
    }

    pub(crate) fn is_client_connected(&self, client_id: &ConnectedClientId) -> bool {
        self.clients_ipv4_mapping
            .values()
            .any(|client| client.client_id == *client_id)
    }

    pub(crate) fn disconnect_client(&mut self, client_id: &ConnectedClientId) {
        if let Some(ips) = self.lookup_ip_from_client_id(client_id) {
            tracing::debug!("Disconnect client that requested to do so: {ips}");
            self.disconnect_client_handle(ips);
        }
    }

    fn disconnect_client_handle(&mut self, ips: IpPair) {
        self.clients_ipv4_mapping.remove(&ips.ipv4);
        self.clients_ipv6_mapping.remove(&ips.ipv6);
        self.tun_listener_connected_client_tx
            .send(ConnectedClientEvent::Disconnect(DisconnectEvent(ips)))
            .inspect_err(|err| {
                tracing::error!("Failed to send disconnect event: {err}");
            })
            .ok();
    }

    pub(crate) fn lookup_ip_from_client_id(&self, client_id: &ConnectedClientId) -> Option<IpPair> {
        self.clients_ipv4_mapping
            .iter()
            .find_map(|(ipv4, connected_client)| {
                if connected_client.client_id == *client_id {
                    Some(IpPair::new(*ipv4, connected_client.ipv6))
                } else {
                    None
                }
            })
    }

    #[allow(unused)]
    fn lookup_client(&self, client_id: &ConnectedClientId) -> Option<&ConnectedClient> {
        self.clients_ipv4_mapping
            .values()
            .find(|connected_client| connected_client.client_id == *client_id)
    }

    pub(crate) fn connect(
        &mut self,
        ips: IpPair,
        client_id: ConnectedClientId,
        forward_from_tun_tx: mpsc::UnboundedSender<Vec<u8>>,
        close_tx: oneshot::Sender<()>,
        handle: tokio::task::JoinHandle<()>,
    ) {
        // The map of connected clients that the mixnet listener keeps track of. It monitors
        // activity and disconnects clients that have been inactive for too long.
        let client = ConnectedClient {
            client_id: client_id.clone(),
            ipv6: ips.ipv6,
            last_activity: Arc::new(RwLock::new(Instant::now())),
            close_tx: Arc::new(CloseTx {
                client_id,
                inner: Some(close_tx),
            }),
            handle: Arc::new(handle),
        };

        tracing::info!("Inserting {} and {}", ips.ipv4, ips.ipv6);
        self.clients_ipv4_mapping.insert(ips.ipv4, client.clone());
        self.clients_ipv6_mapping.insert(ips.ipv6, client);

        // Send the connected client info to the tun listener, which will use it to forward packets
        // to the connected client handler.
        self.tun_listener_connected_client_tx
            .send(ConnectedClientEvent::Connect(Box::new(ConnectEvent {
                ips,
                forward_from_tun_tx,
            })))
            .inspect_err(|err| {
                tracing::error!("Failed to send connected client event: {err}");
            })
            .ok();
    }

    pub(crate) async fn update_activity(&mut self, ips: &IpPair) -> Result<()> {
        if let Some(client) = self.clients_ipv4_mapping.get(&ips.ipv4) {
            *client.last_activity.write().await = Instant::now();
            Ok(())
        } else {
            Err(IpPacketRouterError::FailedToUpdateClientActivity)
        }
    }

    // Identify connected client handlers that have stopped without being told to stop
    pub(crate) fn get_finished_client_handlers(&mut self) -> Vec<(IpPair, ConnectedClientId)> {
        self.clients_ipv4_mapping
            .iter_mut()
            .filter_map(|(ip, connected_client)| {
                if connected_client.handle.is_finished() {
                    Some((
                        IpPair::new(*ip, connected_client.ipv6),
                        connected_client.client_id.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) async fn get_inactive_clients(&mut self) -> Vec<(IpPair, ConnectedClientId)> {
        let now = Instant::now();
        let mut ret = vec![];
        for (ip, connected_client) in self.clients_ipv4_mapping.iter() {
            if now.duration_since(*connected_client.last_activity.read().await)
                > CLIENT_MIXNET_INACTIVITY_TIMEOUT
            {
                ret.push((
                    IpPair::new(*ip, connected_client.ipv6),
                    connected_client.client_id.clone(),
                ))
            }
        }
        ret
    }

    pub(crate) fn disconnect_stopped_client_handlers(
        &mut self,
        stopped_clients: Vec<(IpPair, ConnectedClientId)>,
    ) {
        for (ips, _) in &stopped_clients {
            tracing::info!("Disconnect stopped client: {ips}");
            self.disconnect_client_handle(*ips);
        }
    }

    pub(crate) fn disconnect_inactive_clients(
        &mut self,
        inactive_clients: Vec<(IpPair, ConnectedClientId)>,
    ) {
        for (ips, _) in &inactive_clients {
            tracing::info!("Disconnect inactive client: {ips}");
            self.disconnect_client_handle(*ips);
        }
    }

    pub(crate) fn find_new_ip(&self) -> Option<IpPair> {
        crate::util::generate_new_ip::find_new_ips(
            &self.clients_ipv4_mapping,
            &self.clients_ipv6_mapping,
        )
    }
}

pub(crate) struct CloseTx {
    // pub(crate) nym_address: Recipient,
    pub(crate) client_id: ConnectedClientId,
    // Send to connected clients listener to stop. This is option only because we need to take
    // ownership of it when the client is dropped.
    pub(crate) inner: Option<oneshot::Sender<()>>,
}

#[derive(Clone)]
pub(crate) struct ConnectedClient {
    // The nym address of the connected client that we are communicating with on the other side of
    // the mixnet
    // pub(crate) nym_address: Recipient,
    pub(crate) client_id: ConnectedClientId,

    // The assigned IPv6 address of this client
    pub(crate) ipv6: Ipv6Addr,

    // Keep track of last activity so we can disconnect inactive clients
    pub(crate) last_activity: Arc<RwLock<Instant>>,

    // Channel to send close signal to the connected client handler
    // This is currently unused because the disconnect command it not yet implemented.
    #[allow(unused)]
    pub(crate) close_tx: Arc<CloseTx>,

    // Handle for the connected client handler
    pub(crate) handle: Arc<tokio::task::JoinHandle<()>>,
}

impl ConnectedClient {
    pub(crate) async fn update_activity(&self) {
        *self.last_activity.write().await = Instant::now();
    }
}

impl Drop for CloseTx {
    fn drop(&mut self) {
        tracing::debug!("signal to close client: {}", self.client_id);
        if let Some(close_tx) = self.inner.take() {
            close_tx.send(()).ok();
        }
    }
}

pub(crate) enum ConnectedClientEvent {
    Disconnect(DisconnectEvent),
    Connect(Box<ConnectEvent>),
}

pub(crate) struct DisconnectEvent(pub(crate) IpPair);

pub(crate) struct ConnectEvent {
    pub(crate) ips: IpPair,
    pub(crate) forward_from_tun_tx: mpsc::UnboundedSender<Vec<u8>>,
}
