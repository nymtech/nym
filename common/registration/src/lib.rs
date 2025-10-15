// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use nym_authenticator_requests::AuthenticatorVersion;
use nym_crypto::asymmetric::x25519::PublicKey;
use nym_ip_packet_requests::IpPair;
use nym_sphinx::addressing::{NodeIdentity, Recipient};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NymNode {
    pub identity: NodeIdentity,
    pub ip_address: IpAddr,
    pub ipr_address: Option<Recipient>,
    pub authenticator_address: Option<Recipient>,
    pub version: AuthenticatorVersion,
}

#[derive(Clone, Debug)]
pub struct GatewayData {
    pub public_key: PublicKey,
    pub endpoint: SocketAddr,
    pub private_ipv4: Ipv4Addr,
    pub private_ipv6: Ipv6Addr,
}

#[derive(Clone, Copy, Debug)]
pub struct AssignedAddresses {
    pub entry_mixnet_gateway_ip: IpAddr,
    pub exit_mixnet_gateway_ip: IpAddr,
    pub mixnet_client_address: Recipient,
    pub exit_mix_address: Recipient,
    pub interface_addresses: IpPair,
}
