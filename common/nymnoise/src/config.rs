// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use arc_swap::ArcSwap;
use nym_crypto::asymmetric::x25519;
use nym_noise_keys::{NoiseVersion, VersionedNoiseKey};
use snow::params::NoiseParams;

use strum_macros::{EnumIter, FromRepr};

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

#[derive(Debug, Default)]
struct SocketAddrToKey {
    inner: ArcSwap<HashMap<SocketAddr, VersionedNoiseKey>>,
}

// SW NOTE : Only for phased upgrade. To remove once we decide all nodes have to support Noise
#[derive(Debug, Default)]
struct IpAddrToVersion {
    inner: ArcSwap<HashMap<IpAddr, NoiseVersion>>,
}

#[derive(Debug, Clone, Default)]
pub struct NoiseNetworkView {
    keys: Arc<SocketAddrToKey>,
    support: Arc<IpAddrToVersion>,
}

impl NoiseNetworkView {
    pub fn new_empty() -> Self {
        NoiseNetworkView {
            keys: Default::default(),
            support: Default::default(),
        }
    }

    pub fn swap_view(&self, new: HashMap<SocketAddr, VersionedNoiseKey>) {
        let noise_support = new
            .iter()
            .map(|(s_addr, key)| (s_addr.ip(), key.supported_version))
            .collect::<HashMap<_, _>>();
        self.keys.inner.store(Arc::new(new));
        self.support.inner.store(Arc::new(noise_support));
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

    pub(crate) fn get_noise_key(&self, s_address: &SocketAddr) -> Option<VersionedNoiseKey> {
        self.network.keys.inner.load().get(s_address).copied()
    }

    // Only for phased update
    //SW This can lead to some troubles if two nodes share the same IP and one support Noise but not the other.
    // This in only for the progressive update though and there is no workaround
    pub(crate) fn get_noise_support(&self, ip_addr: IpAddr) -> Option<NoiseVersion> {
        let plain_ip_support = self.network.support.inner.load().get(&ip_addr).copied();

        // SW default bind address being [::]:1789, it can happen that a responder sees the ipv6-mapped address of the initiator, this check for that
        let canonical_ip = &ip_addr.to_canonical();
        let canonical_ip_support = self.network.support.inner.load().get(canonical_ip).copied();
        plain_ip_support.or(canonical_ip_support)
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
