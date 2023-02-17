// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::params::DEFAULT_NUM_MIX_HOPS;
use nym_topology::{NymTopology, NymTopologyError};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

// I'm extremely curious why compiler NEVER complained about lack of Debug here before
#[derive(Debug)]
pub struct TopologyAccessorInner(Option<NymTopology>);

impl AsRef<Option<NymTopology>> for TopologyAccessorInner {
    fn as_ref(&self) -> &Option<NymTopology> {
        &self.0
    }
}

impl TopologyAccessorInner {
    fn new() -> Self {
        TopologyAccessorInner(None)
    }

    fn update(&mut self, new: Option<NymTopology>) {
        self.0 = new;
    }
}

pub struct TopologyReadPermit<'a> {
    permit: RwLockReadGuard<'a, TopologyAccessorInner>,
}

impl<'a> Deref for TopologyReadPermit<'a> {
    type Target = TopologyAccessorInner;

    fn deref(&self) -> &Self::Target {
        &self.permit
    }
}

impl<'a> TopologyReadPermit<'a> {
    /// Using provided topology read permit, tries to get an immutable reference to the underlying
    /// topology. For obvious reasons the lifetime of the topology reference is bound to the permit.
    pub(crate) fn try_get_valid_topology_ref(
        &'a self,
        ack_recipient: &Recipient,
        packet_recipient: Option<&Recipient>,
    ) -> Result<&'a NymTopology, NymTopologyError> {
        // 1. Have we managed to get anything from the refresher, i.e. have the nym-api queries gone through?
        let topology = self
            .permit
            .as_ref()
            .as_ref()
            .ok_or(NymTopologyError::EmptyNetworkTopology)?;

        // 2. does it have any mixnode at all?
        // 3. does it have any gateways at all?
        // 4. does it have a mixnode on each layer?
        topology.ensure_can_construct_path_through(DEFAULT_NUM_MIX_HOPS)?;

        // 5. does it contain OUR gateway (so that we could create an ack packet)?
        if !topology.gateway_exists(ack_recipient.gateway()) {
            return Err(NymTopologyError::NonExistentGatewayError {
                identity_key: ack_recipient.gateway().to_base58_string(),
            });
        }

        // 6. for our target recipient, does it contain THEIR gateway (so that we could create
        if let Some(recipient) = packet_recipient {
            if !topology.gateway_exists(recipient.gateway()) {
                return Err(NymTopologyError::NonExistentGatewayError {
                    identity_key: recipient.gateway().to_base58_string(),
                });
            }
        }

        Ok(topology)
    }
}

impl<'a> From<RwLockReadGuard<'a, TopologyAccessorInner>> for TopologyReadPermit<'a> {
    fn from(read_permit: RwLockReadGuard<'a, TopologyAccessorInner>) -> Self {
        TopologyReadPermit {
            permit: read_permit,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TopologyAccessor {
    // `RwLock` *seems to* be the better approach for this as write access is only requested every
    // few seconds, while reads are needed every single packet generated.
    // However, proper benchmarks will be needed to determine if `RwLock` is indeed a better
    // approach than a `Mutex`
    inner: Arc<RwLock<TopologyAccessorInner>>,
}

impl TopologyAccessor {
    pub fn new() -> Self {
        TopologyAccessor {
            inner: Arc::new(RwLock::new(TopologyAccessorInner::new())),
        }
    }

    pub async fn get_read_permit(&self) -> TopologyReadPermit<'_> {
        self.inner.read().await.into()
    }

    pub(crate) async fn update_global_topology(&self, new_topology: Option<NymTopology>) {
        self.inner.write().await.update(new_topology);
    }

    // only used by the client at startup to get a slightly more reasonable error message
    // (currently displays as unused because health checker is disabled due to required changes)
    pub async fn ensure_is_routable(&self) -> Result<(), NymTopologyError> {
        match &self.inner.read().await.0 {
            None => Err(NymTopologyError::EmptyNetworkTopology),
            Some(ref topology) => topology.ensure_can_construct_path_through(DEFAULT_NUM_MIX_HOPS),
        }
    }
}

impl Default for TopologyAccessor {
    fn default() -> Self {
        TopologyAccessor::new()
    }
}
