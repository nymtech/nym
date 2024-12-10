// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::monitor::receiver::{GatewayClientUpdate, GatewayClientUpdateSender};
use crate::support::nyxd;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_gateway_client::GatewayClient;
use std::ops::{Deref, DerefMut};
use tracing::warn;

pub(crate) struct GatewayClientHandle {
    client: GatewayClient<nyxd::Client, PersistentStorage>,
    gateways_status_updater: GatewayClientUpdateSender,
}

impl GatewayClientHandle {
    pub(crate) fn new(
        client: GatewayClient<nyxd::Client, PersistentStorage>,
        gateways_status_updater: GatewayClientUpdateSender,
    ) -> Self {
        GatewayClientHandle {
            client,
            gateways_status_updater,
        }
    }
}

impl Drop for GatewayClientHandle {
    fn drop(&mut self) {
        if self
            .gateways_status_updater
            .unbounded_send(GatewayClientUpdate::Disconnect(
                self.client.gateway_identity(),
            ))
            .is_err()
        {
            warn!("fail to cleanly shutdown gateway connection")
        }
    }
}

impl Deref for GatewayClientHandle {
    type Target = GatewayClient<nyxd::Client, PersistentStorage>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for GatewayClientHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}
