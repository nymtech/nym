// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use arc_swap::ArcSwap;
use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy)]
pub enum NoisePattern {
    #[default]
    XKpsk3,
    IKpsk2,
}

impl NoisePattern {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::XKpsk3 => "Noise_XKpsk3_25519_AESGCM_SHA256",
            Self::IKpsk2 => "Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s", //Wireguard handshake (not exactly though)
        }
    }

    pub(crate) fn psk_position(&self) -> u8 {
        //automatic parsing, works for correct pattern, more convenient
        match self.as_str().find("psk") {
            Some(n) => {
                let psk_index = n + 3;
                let psk_char = self.as_str().chars().nth(psk_index).unwrap();
                psk_char.to_string().parse().unwrap()
                //if this fails, it means hardcoded pattern are wrong
            }
            None => 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum NoiseVersion {
    V1 = 1isize,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct VersionedNoiseKey {
    pub version: NoiseVersion,
    #[serde(with = "bs58_x25519_pubkey")]
    pub pub_key: x25519::PublicKey,
}

#[derive(Debug, Default)]
struct NoiseKeys {
    inner: ArcSwap<HashMap<SocketAddr, VersionedNoiseKey>>,
}

// SW NOTE : Only for phased upgrade. To remove once we decide all nodes have to support Noise
#[derive(Debug, Default)]
struct NoiseSupport {
    inner: ArcSwap<HashMap<IpAddr, NoiseVersion>>,
}

#[derive(Debug, Clone, Default)]
pub struct NoiseNodes {
    keys: Arc<NoiseKeys>,
    support: Arc<NoiseSupport>,
}

impl NoiseNodes {
    pub fn new_empty() -> Self {
        NoiseNodes {
            keys: Default::default(),
            support: Default::default(),
        }
    }

    pub fn swap_nodes(&self, new: HashMap<SocketAddr, VersionedNoiseKey>) {
        let noise_support = new
            .iter()
            .map(|(s_addr, key)| (s_addr.ip(), key.version))
            .collect::<HashMap<_, _>>();
        self.keys.inner.store(Arc::new(new));
        self.support.inner.store(Arc::new(noise_support));
    }
}

#[derive(Clone)]
pub struct NoiseConfig {
    network: NoiseNodes,

    pub(crate) local_key: Arc<x25519::KeyPair>,
    pub(crate) pattern: NoisePattern,
    pub(crate) time_based_component: u32, //TODO : Figureout what to put there

    pub(crate) unsafe_disabled: bool, // allows for nodes to not attempt to do a noise handshake, VERY UNSAFE, FOR DEBUG PURPOSE ONLY
}

impl NoiseConfig {
    pub fn new(noise_key: Arc<x25519::KeyPair>, network: NoiseNodes) -> Self {
        NoiseConfig {
            network,
            local_key: noise_key,
            pattern: Default::default(),
            unsafe_disabled: false,
            time_based_component: 0, //SW How can we sync this?
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
    //SW This can lead to some troubles if two nodes shares the same IP and one support Noise but not the other. This in only for the progressive update though and there is no workaround
    //SW also seen problems due to ipv4-to-6 changing the address seen
    pub(crate) fn get_noise_support(&self, ip_addr: IpAddr) -> Option<NoiseVersion> {
        self.network.support.inner.load().get(&ip_addr).copied()
    }
}
