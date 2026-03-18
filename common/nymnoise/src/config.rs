// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use arc_swap::ArcSwap;
use nym_crypto::asymmetric::x25519;
use nym_noise_keys::VersionedNoiseKeyV1;
use snow::params::NoiseParams;
use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Duration};

use strum_macros::{EnumIter, FromRepr};
use tokio::sync::{Mutex, MutexGuard};

#[derive(Default, Debug, Clone, Copy, EnumIter, FromRepr, Eq, PartialEq)]
#[repr(u8)]
#[non_exhaustive]
pub enum NoisePattern {
    #[default]
    XKpsk3 = 1,
    IKpsk2 = 2,
}

impl NoisePattern {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::XKpsk3 => "Noise_XKpsk3_25519_AESGCM_SHA256",
            Self::IKpsk2 => "Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s", //Wireguard handshake (not exactly though)
        }
    }

    // SAFETY: we have tests to ensure that hardcoded pattern are correct
    #[allow(clippy::unwrap_used)]
    pub(crate) fn psk_position(&self) -> u8 {
        //automatic parsing, works for correct pattern, more convenient
        match self.as_str().find("psk") {
            Some(n) => {
                let psk_index = n + 3;
                let psk_char = self.as_str().chars().nth(psk_index).unwrap();
                psk_char.to_string().parse().unwrap()
            }
            None => 0,
        }
    }

    // SAFETY : we have tests to ensure that hardcoded pattern are correct
    #[allow(clippy::unwrap_used)]
    pub(crate) fn as_noise_params(&self) -> NoiseParams {
        self.as_str().parse().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct NoiseNetworkView {
    inner: Arc<NoiseNetworkViewInner>,
}

#[derive(Debug)]
struct NoiseNetworkViewInner {
    update_lock: Mutex<()>,
    nodes: ArcSwap<HashMap<IpAddr, NoiseNode>>,
}

#[derive(Debug, Clone)]
pub struct NoiseNode {
    key: VersionedNoiseKeyV1,

    // flag indicating whether this node is a nym node or a network monitor agent
    is_nym_node: bool,
}

impl NoiseNode {
    pub fn new_nym_node(key: VersionedNoiseKeyV1) -> Self {
        NoiseNode {
            key,
            is_nym_node: true,
        }
    }

    pub fn new_network_monitor_agent(key: VersionedNoiseKeyV1) -> Self {
        NoiseNode {
            key,
            is_nym_node: false,
        }
    }

    pub fn is_nym_node(&self) -> bool {
        self.is_nym_node
    }
}

impl NoiseNetworkView {
    pub fn new_empty() -> Self {
        NoiseNetworkView {
            inner: Arc::new(NoiseNetworkViewInner {
                update_lock: Mutex::new(()),
                nodes: Default::default(),
            }),
        }
    }

    pub async fn get_update_permit(&self) -> MutexGuard<'_, ()> {
        self.inner.update_lock.lock().await
    }

    // this MUST not be called without obtaining the permit first.
    // the reason the data is not wrapped in mutex itself is to reduce the overhead
    // for the reading tasks performing packet routing
    pub fn swap_view(&self, _permit: MutexGuard<'_, ()>, new: HashMap<IpAddr, NoiseNode>) {
        self.inner.nodes.store(Arc::new(new));
    }

    pub fn all_nodes(&self) -> HashMap<IpAddr, NoiseNode> {
        self.inner.nodes.load().as_ref().clone()
    }
}

#[derive(Clone)]
pub struct NoiseConfig {
    network: NoiseNetworkView,

    pub(crate) local_key: Arc<x25519::KeyPair>,
    pub(crate) pattern: NoisePattern,
    pub(crate) timeout: Duration,

    pub(crate) unsafe_disabled: bool, // allows for nodes to not attempt to do a noise handshake, VERY UNSAFE, FOR DEBUG PURPOSE ONLY
}

impl NoiseConfig {
    pub fn new(
        noise_key: Arc<x25519::KeyPair>,
        network: NoiseNetworkView,
        timeout: Duration,
    ) -> Self {
        NoiseConfig {
            network,
            local_key: noise_key,
            pattern: Default::default(),
            timeout,
            unsafe_disabled: false,
        }
    }

    #[must_use]
    pub fn with_noise_pattern(mut self, pattern: NoisePattern) -> Self {
        self.pattern = pattern;
        self
    }

    #[must_use]
    pub fn with_unsafe_disabled(mut self, disabled: bool) -> Self {
        self.unsafe_disabled = disabled;
        self
    }

    pub(crate) fn get_noise_key(&self, address: IpAddr) -> Option<VersionedNoiseKeyV1> {
        // with the default bind address being `[::]:1789`,
        // it can happen that a responder sees the ipv6-mapped address of the initiator,
        // this checks for that
        let base_ip = address;
        let canonical_ip = base_ip.to_canonical();

        if let Some(node) = self.network.inner.nodes.load().get(&base_ip) {
            return Some(node.key);
        }

        Some(self.network.inner.nodes.load().get(&canonical_ip)?.key)
    }

    pub(crate) fn supports_noise(&self, ip_addr: IpAddr) -> bool {
        self.get_noise_key(ip_addr).is_some()
    }
}

#[cfg(test)]
mod tests {
    use snow::params::NoiseParams;

    use super::NoisePattern;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    // The goal of these is to make sure every NoisePatterns are correct and unwrap can be used on them

    #[test]
    fn noise_patterns_are_valid() {
        for pattern in NoisePattern::iter() {
            assert!(NoiseParams::from_str(pattern.as_str()).is_ok())
        }
    }

    #[test]
    fn noise_patterns_psk_position_is_valid() {
        for pattern in NoisePattern::iter() {
            match pattern {
                NoisePattern::XKpsk3 => assert_eq!(pattern.psk_position(), 3),
                NoisePattern::IKpsk2 => assert_eq!(pattern.psk_position(), 2),
            }
        }
    }
}
