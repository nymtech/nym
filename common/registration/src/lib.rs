// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_authenticator_requests::AuthenticatorVersion;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_ip_packet_requests::IpPair;
use nym_kkt_ciphersuite::{KEM, KEMKeyDigests};
use nym_sphinx::addressing::Recipient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub use lp_messages::{
    LpDvpnRegistrationRequest, LpMixnetGatewayData, LpMixnetRegistrationRequest,
    LpRegistrationData, LpRegistrationRequest, LpRegistrationResponse, RegistrationMode,
};
pub use serialisation::BincodeError;

mod lp_messages;
mod serialisation;

#[derive(Debug, Clone)]
pub struct NymNode {
    pub identity: ed25519::PublicKey,
    pub ip_address: IpAddr,
    pub ipr_address: Option<Recipient>,
    pub authenticator_address: Option<Recipient>,
    pub lp_data: Option<GatewayLpData>,
    pub version: AuthenticatorVersion,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GatewayData {
    #[serde(with = "bs58_x25519_pubkey")]
    pub public_key: x25519::PublicKey,
    pub endpoint: SocketAddr,
    pub private_ipv4: Ipv4Addr,
    pub private_ipv6: Ipv6Addr,
}

#[derive(Clone, Debug)]
pub struct GatewayLpData {
    pub address: SocketAddr,
    pub expected_kem_key_hashes: HashMap<KEM, KEMKeyDigests>,
    pub x25519: x25519::PublicKey,
}

#[derive(Clone, Copy, Debug)]
pub struct AssignedAddresses {
    pub entry_mixnet_gateway_ip: IpAddr,
    pub exit_mixnet_gateway_ip: IpAddr,
    pub mixnet_client_address: Recipient,
    pub exit_mix_address: Recipient,
    pub interface_addresses: IpPair,
}
