// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::gateway_clients_cache::ActiveGatewayClients;
use crate::network_monitor::monitor::receiver::{GatewayClientUpdate, GatewayClientUpdateSender};
use crypto::asymmetric::identity;
use crypto::asymmetric::identity::PUBLIC_KEY_LENGTH;
use log::{debug, info, trace, warn};
use std::time::Duration;
use tokio::time::{sleep, Instant};

// TODO: should it perhaps be moved to config along other timeout values?
const PING_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) struct GatewayPinger {
    gateway_clients: ActiveGatewayClients,
    gateways_status_updater: GatewayClientUpdateSender,
    pinging_interval: Duration,
}

impl GatewayPinger {
    pub(crate) fn new(
        gateway_clients: ActiveGatewayClients,
        gateways_status_updater: GatewayClientUpdateSender,
        pinging_interval: Duration,
    ) -> Self {
        GatewayPinger {
            gateway_clients,
            gateways_status_updater,
            pinging_interval,
        }
    }

    fn notify_connection_failure(&self, raw_gateway_id: [u8; PUBLIC_KEY_LENGTH]) {
        // if this unwrap failed it means something extremely weird is going on
        // and we got some solar flare bitflip type of corruption
        let gateway_key = identity::PublicKey::from_bytes(&raw_gateway_id)
            .expect("failed to recover gateways public key from valid bytes");

        // remove the gateway listener channels
        self.gateways_status_updater
            .unbounded_send(GatewayClientUpdate::Failure(gateway_key))
            .expect("packet receiver seems to have died!");
    }

    async fn ping_and_cleanup_all_gateways(&self) {
        info!("Pinging all active gateways");

        let lock_acquire_start = Instant::now();
        let active_gateway_clients_guard = self.gateway_clients.lock().await;
        trace!(
            "Acquiring lock took {:?}",
            Instant::now().duration_since(lock_acquire_start)
        );

        if active_gateway_clients_guard.is_empty() {
            debug!("no gateways to ping");
            return;
        }

        // don't keep the guard the entire time - clone all Arcs and drop it
        //
        // this clippy warning is a false positive as we cannot get rid of the collect by moving
        // everything into a single iterator as it would require us to hold the lock the entire time
        // and that is exactly what we want to avoid
        #[allow(clippy::needless_collect)]
        let active_gateway_clients = active_gateway_clients_guard
            .iter()
            .map(|(_, handle)| handle.clone_data_pointer())
            .collect::<Vec<_>>();
        drop(active_gateway_clients_guard);

        let ping_start = Instant::now();

        let mut clients_to_purge = Vec::new();

        // since we don't need to wait for response, we can just ping all gateways sequentially
        // if it becomes problem later on, we can adjust it.
        for client_handle in active_gateway_clients.into_iter() {
            trace!(
                "Pinging: {}",
                identity::PublicKey::from_bytes(&client_handle.raw_identity())
                    .unwrap()
                    .to_base58_string()
            );
            // if we fail to obtain the lock it means the client is being currently used to send messages
            // and hence we don't need to ping it to keep connection alive
            if let Ok(mut unlocked_handle) = client_handle.try_lock_client() {
                if let Some(active_client) = unlocked_handle.inner_mut() {
                    match tokio::time::timeout(PING_TIMEOUT, active_client.send_ping_message())
                        .await
                    {
                        Err(_timeout) => {
                            warn!(
                                "we timed out trying to ping {} - assuming the connection is dead.",
                                active_client.gateway_identity().to_base58_string(),
                            );
                            clients_to_purge.push(client_handle.raw_identity());
                        }
                        Ok(Err(err)) => {
                            warn!(
                                "failed to send ping message to gateway {} - {} - assuming the connection is dead.",
                                active_client.gateway_identity().to_base58_string(),
                                err,
                            );
                            clients_to_purge.push(client_handle.raw_identity());
                        }
                        _ => {}
                    }
                } else {
                    clients_to_purge.push(client_handle.raw_identity());
                }
            }
        }

        info!(
            "Purging {} gateways, acquiring lock",
            clients_to_purge.len()
        );
        // purge all dead connections
        // reacquire the guard
        let lock_acquire_start = Instant::now();
        let mut active_gateway_clients_guard = self.gateway_clients.lock().await;
        info!(
            "Acquiring lock took {:?}",
            Instant::now().duration_since(lock_acquire_start)
        );

        for gateway_id in clients_to_purge.into_iter() {
            if let Some(removed_handle) = active_gateway_clients_guard.remove(&gateway_id) {
                if !removed_handle.is_invalid().await {
                    info!("Handle is invalid, purging");
                    // it was not invalidated by the packet sender meaning it probably was some unbonded node
                    // that was never cleared
                    self.notify_connection_failure(gateway_id);
                }
                info!("Handle is not invalid, not purged")
            }
        }

        let ping_end = Instant::now();
        let time_taken = ping_end.duration_since(ping_start);
        info!("Pinging all active gateways took {:?}", time_taken);
    }

    pub(crate) async fn run(&self) {
        loop {
            sleep(self.pinging_interval).await;
            self.ping_and_cleanup_all_gateways().await
        }
    }
}
