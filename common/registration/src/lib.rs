// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_authenticator_requests::AuthenticatorVersion;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_ip_packet_requests::IpPair;
use nym_kkt_ciphersuite::{Ciphersuite, KEM, KEMKeyDigests};
use nym_sphinx::addressing::Recipient;
use nym_wireguard_types::PresharedKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub use lp_messages::*;
use nym_crypto::asymmetric::x25519::DHPublicKey;
pub use serialisation::BincodeError;

mod lp_messages;
mod serialisation;

#[derive(Debug, Clone)]
pub struct NymNodeInformation {
    pub identity: ed25519::PublicKey,
    pub ip_address: IpAddr,
    pub ipr_address: Option<Recipient>,
    pub authenticator_address: Option<Recipient>,
    pub lp_data: Option<NymNodeLPInformation>,
    pub version: AuthenticatorVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireguardRegistrationData {
    /// Public x25519 key of this gateway
    #[serde(with = "bs58_x25519_pubkey")]
    pub public_key: x25519::PublicKey,

    /// Port at which this gateway is accessible for wireguard
    pub port: u16,

    /// Ipv4 address assigned to this peer
    pub private_ipv4: Ipv4Addr,

    /// Ipv6 address assigned to this peer
    pub private_ipv6: Ipv6Addr,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct WireguardConfiguration {
    #[serde(with = "bs58_x25519_pubkey")]
    pub public_key: x25519::PublicKey,
    pub psk: Option<PresharedKey>,
    pub endpoint: SocketAddr,
    pub private_ipv4: Ipv4Addr,
    pub private_ipv6: Ipv6Addr,
}

#[derive(Clone, Debug)]
pub struct NymNodeLPInformation {
    pub address: SocketAddr,
    pub expected_kem_key_hashes: BTreeMap<KEM, KEMKeyDigests>,
    pub x25519: DHPublicKey,

    // to be inferred from node's version
    pub ciphersuite: Ciphersuite,

    /// Supported protocol version of the remote gateway.
    /// Included in case we have to downgrade our version.
    pub lp_protocol_version: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct AssignedAddresses {
    pub entry_mixnet_gateway_ip: IpAddr,
    pub exit_mixnet_gateway_ip: IpAddr,
    pub mixnet_client_address: Recipient,
    pub exit_mix_address: Recipient,
    pub interface_addresses: IpPair,
}
