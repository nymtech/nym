// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::FreeTierEnforcementConfig;
use crate::ip_pool::IpPair;
use nym_free_tier_enforcement::{
    EnforcementError, FreeTierEnforcement, PeerAddrs, SystemCommandRunner,
};
use std::sync::Arc;
use tracing::info;

struct FreeTierControllerInner {
    /// Datapath enforcement facade, present when the free tier is enabled. Used to admit
    /// new free peers into the rate-limit pool and handed to each `PeerHandle` so it can
    /// confine the peer to the walled garden on exhaustion.
    enforcement: FreeTierEnforcement,
}

impl FreeTierControllerInner {
    fn new(ifname: String, config: FreeTierEnforcementConfig) -> Self {
        FreeTierControllerInner {
            enforcement: FreeTierEnforcement::new(
                ifname.clone(),
                config.pool_bytes_per_second,
                config.walled_garden_whitelist,
                Arc::new(SystemCommandRunner),
            ),
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct FreeTierController {
    inner: Option<Arc<FreeTierControllerInner>>,
}

impl FreeTierController {
    pub(crate) fn new_enabled(ifname: String, config: FreeTierEnforcementConfig) -> Self {
        FreeTierController {
            inner: Some(Arc::new(FreeTierControllerInner::new(ifname, config))),
        }
    }

    pub(crate) fn new_disabled() -> Self {
        FreeTierController { inner: None }
    }

    pub(crate) fn free_tier_enabled(&self) -> bool {
        self.inner.is_some()
    }

    pub(crate) fn confine_to_garden(&self, peer_ips: IpPair) -> Result<bool, EnforcementError> {
        let Some(inner) = &self.inner else {
            return Ok(false);
        };
        inner
            .enforcement
            .send_to_garden(&peer_ips.as_free_tier_peers())?;
        Ok(true)
    }

    pub(crate) fn admit_peer(&self, peer_ips: IpPair) -> Result<bool, EnforcementError> {
        let Some(inner) = &self.inner else {
            return Ok(false);
        };
        inner.enforcement.admit(&peer_ips.as_free_tier_peers())?;
        Ok(true)
    }

    /// Release a peer from ALL free-tier enforcement (pool + walled garden), restoring full
    /// unrestricted access. Idempotent. Used when a formerly-free peer upgrades to paid.
    pub(crate) fn release_peer(&self, peer_ips: IpPair) -> Result<bool, EnforcementError> {
        let Some(inner) = &self.inner else {
            return Ok(false);
        };
        inner.enforcement.release(&peer_ips.as_free_tier_peers())?;
        Ok(true)
    }

    pub(crate) fn reconcile(
        &self,
        pooled: &[PeerAddrs],
        gardened: &[PeerAddrs],
    ) -> Result<(), EnforcementError> {
        let Some(inner) = &self.inner else {
            return Ok(());
        };
        info!(
            "reconciling free-tier enforcement on '{}': {} pooled, {} gardened peer(s)",
            inner.enforcement.interface(),
            pooled.len(),
            gardened.len()
        );
        inner.enforcement.reconcile(pooled, gardened)
    }
}
