// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::latest::registration::IpPair;
use crate::{AuthenticatorVersion, Error, v1, v2, v3, v4, v5, v6};
use nym_credentials_interface::{
    BandwidthCredential, CredentialSpendingData, TicketType, UnknownTicketType,
};
use nym_crypto::asymmetric::x25519;
use nym_wireguard_types::PeerPublicKey;
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum CurrentUpgradeModeStatus {
    Enabled,
    Disabled,
    // everything pre-v6
    Unknown,
}

impl CurrentUpgradeModeStatus {
    pub fn is_enabled(&self) -> bool {
        matches!(self, CurrentUpgradeModeStatus::Enabled)
    }
}

impl From<bool> for CurrentUpgradeModeStatus {
    fn from(value: bool) -> Self {
        if value {
            CurrentUpgradeModeStatus::Enabled
        } else {
            CurrentUpgradeModeStatus::Disabled
        }
    }
}

impl From<CurrentUpgradeModeStatus> for Option<bool> {
    fn from(value: CurrentUpgradeModeStatus) -> Self {
        match value {
            CurrentUpgradeModeStatus::Enabled => Some(true),
            CurrentUpgradeModeStatus::Disabled => Some(false),
            CurrentUpgradeModeStatus::Unknown => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BandwidthClaim {
    pub credential: BandwidthCredential,
    pub kind: TicketType,
}

impl TryFrom<CredentialSpendingData> for BandwidthClaim {
    type Error = UnknownTicketType;

    fn try_from(credential: CredentialSpendingData) -> Result<Self, Self::Error> {
        Ok(BandwidthClaim {
            kind: TicketType::try_from_encoded(credential.payment.t_type)?,
            credential: BandwidthCredential::from(credential),
        })
    }
}

pub trait Versionable {
    fn version(&self) -> AuthenticatorVersion;
}

impl Versionable for v1::GatewayClient {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V1
    }
}

impl Versionable for v1::registration::InitMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V1
    }
}

impl Versionable for v2::registration::InitMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V2
    }
}

impl Versionable for v3::registration::InitMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V3
    }
}

impl Versionable for v4::registration::InitMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V4
    }
}

impl Versionable for v5::registration::InitMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V5
    }
}

impl Versionable for v6::registration::InitMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V6
    }
}

impl Versionable for v2::registration::FinalMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V2
    }
}

impl Versionable for v3::registration::FinalMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V3
    }
}

impl Versionable for v4::registration::FinalMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V4
    }
}

impl Versionable for v5::registration::FinalMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V5
    }
}

impl Versionable for v6::registration::FinalMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V6
    }
}

impl Versionable for PeerPublicKey {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V3
    }
}

impl Versionable for v3::topup::TopUpMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V3
    }
}

impl Versionable for v4::topup::TopUpMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V4
    }
}

impl Versionable for v5::topup::TopUpMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V5
    }
}
impl Versionable for v6::topup::TopUpMessage {
    fn version(&self) -> AuthenticatorVersion {
        AuthenticatorVersion::V6
    }
}

pub trait UpgradeModeStatus {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus;
}

impl UpgradeModeStatus for v1::response::PendingRegistrationResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v1::response::RegisteredResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v1::response::RemainingBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v2::response::PendingRegistrationResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v2::response::RegisteredResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v2::response::RemainingBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}
impl UpgradeModeStatus for v3::response::PendingRegistrationResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v3::response::RegisteredResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v3::response::RemainingBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v3::response::TopUpBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v4::response::PendingRegistrationResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v4::response::RegisteredResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v4::response::RemainingBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v4::response::TopUpBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v5::response::PendingRegistrationResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v5::response::RegisteredResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v5::response::RemainingBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v5::response::TopUpBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        CurrentUpgradeModeStatus::Unknown
    }
}

impl UpgradeModeStatus for v6::response::PendingRegistrationResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        self.upgrade_mode_enabled.into()
    }
}

impl UpgradeModeStatus for v6::response::RegisteredResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        self.upgrade_mode_enabled.into()
    }
}

impl UpgradeModeStatus for v6::response::RemainingBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        self.upgrade_mode_enabled.into()
    }
}

impl UpgradeModeStatus for v6::response::TopUpBandwidthResponse {
    fn upgrade_mode_status(&self) -> CurrentUpgradeModeStatus {
        self.upgrade_mode_enabled.into()
    }
}

pub trait InitMessage: Versionable + fmt::Debug {
    fn pub_key(&self) -> PeerPublicKey;
}

impl InitMessage for v1::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

impl InitMessage for v2::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

impl InitMessage for v3::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

impl InitMessage for v4::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

impl InitMessage for v5::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

impl InitMessage for v6::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

pub trait FinalMessage: Versionable + fmt::Debug {
    fn gateway_client_pub_key(&self) -> PeerPublicKey;
    fn verify(&self, private_key: &x25519::PrivateKey, nonce: u64) -> Result<(), Error>;
    fn private_ips(&self) -> IpPair;
    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr>;
    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr>;
    fn gateway_client_mac(&self) -> Vec<u8>;
    fn credential(&self) -> Option<BandwidthClaim>;
}

impl FinalMessage for v1::GatewayClient {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    fn verify(&self, private_key: &x25519::PrivateKey, nonce: u64) -> Result<(), Error> {
        self.verify(private_key, nonce)
    }

    fn private_ips(&self) -> IpPair {
        self.private_ip.into()
    }

    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr> {
        match self.private_ip {
            std::net::IpAddr::V4(ipv4_addr) => Some(ipv4_addr),
            std::net::IpAddr::V6(_) => None,
        }
    }

    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr> {
        None
    }

    fn gateway_client_mac(&self) -> Vec<u8> {
        self.mac.to_vec()
    }

    fn credential(&self) -> Option<BandwidthClaim> {
        None
    }
}

impl FinalMessage for v2::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
    }

    fn verify(&self, private_key: &x25519::PrivateKey, nonce: u64) -> Result<(), Error> {
        self.gateway_client.verify(private_key, nonce)
    }

    fn private_ips(&self) -> IpPair {
        self.gateway_client.private_ip.into()
    }

    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr> {
        match self.gateway_client.private_ip {
            std::net::IpAddr::V4(ipv4_addr) => Some(ipv4_addr),
            std::net::IpAddr::V6(_) => None,
        }
    }

    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr> {
        None
    }

    fn gateway_client_mac(&self) -> Vec<u8> {
        self.gateway_client.mac.to_vec()
    }

    fn credential(&self) -> Option<BandwidthClaim> {
        self.credential.clone().and_then(|c| {
            c.try_into()
                .inspect_err(|err| error!("credential conversion error: {err}"))
                .ok()
        })
    }
}

impl FinalMessage for v3::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
    }

    fn verify(&self, private_key: &x25519::PrivateKey, nonce: u64) -> Result<(), Error> {
        self.gateway_client.verify(private_key, nonce)
    }

    fn private_ips(&self) -> IpPair {
        self.gateway_client.private_ip.into()
    }

    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr> {
        match self.gateway_client.private_ip {
            std::net::IpAddr::V4(ipv4_addr) => Some(ipv4_addr),
            std::net::IpAddr::V6(_) => None,
        }
    }

    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr> {
        None
    }

    fn gateway_client_mac(&self) -> Vec<u8> {
        self.gateway_client.mac.to_vec()
    }

    fn credential(&self) -> Option<BandwidthClaim> {
        self.credential.clone().and_then(|c| {
            c.try_into()
                .inspect_err(|err| error!("credential conversion error: {err}"))
                .ok()
        })
    }
}

impl FinalMessage for v4::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
    }

    fn verify(&self, private_key: &x25519::PrivateKey, nonce: u64) -> Result<(), Error> {
        self.gateway_client.verify(private_key, nonce)
    }

    fn private_ips(&self) -> IpPair {
        // v4 -> v5 -> v6
        v5::registration::IpPair::from(self.gateway_client.private_ips).into()
    }

    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr> {
        Some(self.gateway_client.private_ips.ipv4)
    }

    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr> {
        Some(self.gateway_client.private_ips.ipv6)
    }

    fn gateway_client_mac(&self) -> Vec<u8> {
        self.gateway_client.mac.to_vec()
    }

    fn credential(&self) -> Option<BandwidthClaim> {
        self.credential.clone().and_then(|c| {
            c.try_into()
                .inspect_err(|err| error!("credential conversion error: {err}"))
                .ok()
        })
    }
}

impl FinalMessage for v5::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
    }

    fn verify(&self, private_key: &x25519::PrivateKey, nonce: u64) -> Result<(), Error> {
        self.gateway_client.verify(private_key, nonce)
    }

    fn private_ips(&self) -> IpPair {
        self.gateway_client.private_ips.into()
    }

    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr> {
        Some(self.gateway_client.private_ips.ipv4)
    }

    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr> {
        Some(self.gateway_client.private_ips.ipv6)
    }

    fn gateway_client_mac(&self) -> Vec<u8> {
        self.gateway_client.mac.to_vec()
    }

    fn credential(&self) -> Option<BandwidthClaim> {
        self.credential.clone().and_then(|c| {
            c.try_into()
                .inspect_err(|err| error!("credential conversion error: {err}"))
                .ok()
        })
    }
}

impl FinalMessage for v6::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
    }

    fn verify(&self, private_key: &x25519::PrivateKey, nonce: u64) -> Result<(), Error> {
        self.gateway_client.verify(private_key, nonce)
    }

    fn private_ips(&self) -> IpPair {
        self.gateway_client.private_ips
    }

    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr> {
        Some(self.gateway_client.private_ips.ipv4)
    }

    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr> {
        Some(self.gateway_client.private_ips.ipv6)
    }

    fn gateway_client_mac(&self) -> Vec<u8> {
        self.gateway_client.mac.to_vec()
    }

    fn credential(&self) -> Option<BandwidthClaim> {
        self.credential.clone()
    }
}

pub trait QueryBandwidthMessage: Versionable + fmt::Debug {
    fn pub_key(&self) -> PeerPublicKey;
}

impl QueryBandwidthMessage for PeerPublicKey {
    fn pub_key(&self) -> PeerPublicKey {
        *self
    }
}

pub trait TopUpMessage: Versionable + fmt::Debug {
    fn pub_key(&self) -> PeerPublicKey;
    fn credential(&self) -> CredentialSpendingData;
}

impl TopUpMessage for v3::topup::TopUpMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    fn credential(&self) -> CredentialSpendingData {
        self.credential.clone()
    }
}

impl TopUpMessage for v4::topup::TopUpMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    fn credential(&self) -> CredentialSpendingData {
        self.credential.clone()
    }
}

impl TopUpMessage for v5::topup::TopUpMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    fn credential(&self) -> CredentialSpendingData {
        self.credential.clone()
    }
}

impl TopUpMessage for v6::topup::TopUpMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    fn credential(&self) -> CredentialSpendingData {
        self.credential.clone()
    }
}

pub trait Id {
    fn id(&self) -> u64;
}

impl Id for v2::response::PendingRegistrationResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v3::response::PendingRegistrationResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v4::response::PendingRegistrationResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v5::response::PendingRegistrationResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v6::response::PendingRegistrationResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v2::response::RegisteredResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v3::response::RegisteredResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v4::response::RegisteredResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v5::response::RegisteredResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v6::response::RegisteredResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v2::response::RemainingBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v3::response::RemainingBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v4::response::RemainingBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v5::response::RemainingBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v6::response::RemainingBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v3::response::TopUpBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v4::response::TopUpBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v5::response::TopUpBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

impl Id for v6::response::TopUpBandwidthResponse {
    fn id(&self) -> u64 {
        self.request_id
    }
}

pub trait PendingRegistrationResponse: Id + UpgradeModeStatus + fmt::Debug {
    fn nonce(&self) -> u64;
    fn verify(&self, gateway_key: &x25519::PrivateKey) -> Result<(), Error>;
    fn pub_key(&self) -> PeerPublicKey;
    fn private_ips(&self) -> IpPair;
    fn finalise_registration(
        &self,
        private_key: &x25519::PrivateKey,
        credential: Option<BandwidthClaim>,
    ) -> Box<dyn FinalMessage + Send + Sync>;
}

impl PendingRegistrationResponse for v2::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(&self, gateway_key: &x25519::PrivateKey) -> Result<(), Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ip.into()
    }

    fn finalise_registration(
        &self,
        private_key: &x25519::PrivateKey,
        credential: Option<BandwidthClaim>,
    ) -> Box<dyn FinalMessage + Send + Sync> {
        Box::new(v2::registration::FinalMessage {
            gateway_client: v2::registration::GatewayClient::new(
                private_key,
                self.pub_key().inner(),
                self.private_ips().ipv4.into(),
                self.nonce(),
            ),
            credential: credential.and_then(|b| b.credential.into_zk_nym().map(|c| *c)),
        })
    }
}

impl PendingRegistrationResponse for v3::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(&self, gateway_key: &x25519::PrivateKey) -> Result<(), Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ip.into()
    }

    fn finalise_registration(
        &self,
        private_key: &x25519::PrivateKey,
        credential: Option<BandwidthClaim>,
    ) -> Box<dyn FinalMessage + Send + Sync> {
        Box::new(v3::registration::FinalMessage {
            gateway_client: v3::registration::GatewayClient::new(
                private_key,
                self.pub_key().inner(),
                self.private_ips().ipv4.into(),
                self.nonce(),
            ),
            credential: credential.and_then(|b| b.credential.into_zk_nym().map(|c| *c)),
        })
    }
}

impl PendingRegistrationResponse for v4::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(&self, gateway_key: &x25519::PrivateKey) -> Result<(), Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        // v4 -> v5 -> v6
        v5::registration::IpPair::from(self.reply.gateway_data.private_ips).into()
    }

    fn finalise_registration(
        &self,
        private_key: &x25519::PrivateKey,
        credential: Option<BandwidthClaim>,
    ) -> Box<dyn FinalMessage + Send + Sync> {
        Box::new(v4::registration::FinalMessage {
            gateway_client: v4::registration::GatewayClient::new(
                private_key,
                self.pub_key().inner(),
                self.reply.gateway_data.private_ips,
                self.nonce(),
            ),
            credential: credential.and_then(|b| b.credential.into_zk_nym().map(|c| *c)),
        })
    }
}

impl PendingRegistrationResponse for v5::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(&self, gateway_key: &x25519::PrivateKey) -> Result<(), Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ips.into()
    }

    fn finalise_registration(
        &self,
        private_key: &x25519::PrivateKey,
        credential: Option<BandwidthClaim>,
    ) -> Box<dyn FinalMessage + Send + Sync> {
        Box::new(v5::registration::FinalMessage {
            gateway_client: v5::registration::GatewayClient::new(
                private_key,
                self.pub_key().inner(),
                self.reply.gateway_data.private_ips,
                self.nonce(),
            ),
            credential: credential.and_then(|b| b.credential.into_zk_nym().map(|c| *c)),
        })
    }
}

impl PendingRegistrationResponse for v6::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(&self, gateway_key: &x25519::PrivateKey) -> Result<(), Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ips
    }

    fn finalise_registration(
        &self,
        private_key: &x25519::PrivateKey,
        credential: Option<BandwidthClaim>,
    ) -> Box<dyn FinalMessage + Send + Sync> {
        Box::new(v6::registration::FinalMessage {
            gateway_client: v6::registration::GatewayClient::new(
                private_key,
                self.pub_key().inner(),
                self.reply.gateway_data.private_ips,
                self.nonce(),
            ),
            credential,
        })
    }
}

pub trait RegisteredResponse: Id + UpgradeModeStatus + fmt::Debug {
    fn private_ips(&self) -> IpPair;
    fn pub_key(&self) -> PeerPublicKey;
    fn wg_port(&self) -> u16;
}

impl RegisteredResponse for v2::response::RegisteredResponse {
    fn private_ips(&self) -> IpPair {
        self.reply.private_ip.into()
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.pub_key
    }

    fn wg_port(&self) -> u16 {
        self.reply.wg_port
    }
}

impl RegisteredResponse for v3::response::RegisteredResponse {
    fn private_ips(&self) -> IpPair {
        self.reply.private_ip.into()
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.pub_key
    }

    fn wg_port(&self) -> u16 {
        self.reply.wg_port
    }
}
impl RegisteredResponse for v4::response::RegisteredResponse {
    fn private_ips(&self) -> IpPair {
        // v4 -> v5 -> v6
        v5::registration::IpPair::from(self.reply.private_ips).into()
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.pub_key
    }

    fn wg_port(&self) -> u16 {
        self.reply.wg_port
    }
}

impl RegisteredResponse for v5::response::RegisteredResponse {
    fn private_ips(&self) -> IpPair {
        self.reply.private_ips.into()
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.pub_key
    }

    fn wg_port(&self) -> u16 {
        self.reply.wg_port
    }
}

impl RegisteredResponse for v6::response::RegisteredResponse {
    fn private_ips(&self) -> IpPair {
        self.reply.private_ips
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.pub_key
    }

    fn wg_port(&self) -> u16 {
        self.reply.wg_port
    }
}

pub trait RemainingBandwidthResponse: Id + UpgradeModeStatus + fmt::Debug {
    fn available_bandwidth(&self) -> Option<i64>;
}

impl RemainingBandwidthResponse for v2::response::RemainingBandwidthResponse {
    fn available_bandwidth(&self) -> Option<i64> {
        self.reply.as_ref().map(|r| r.available_bandwidth)
    }
}

impl RemainingBandwidthResponse for v3::response::RemainingBandwidthResponse {
    fn available_bandwidth(&self) -> Option<i64> {
        self.reply.as_ref().map(|r| r.available_bandwidth)
    }
}

impl RemainingBandwidthResponse for v4::response::RemainingBandwidthResponse {
    fn available_bandwidth(&self) -> Option<i64> {
        self.reply.as_ref().map(|r| r.available_bandwidth)
    }
}

impl RemainingBandwidthResponse for v5::response::RemainingBandwidthResponse {
    fn available_bandwidth(&self) -> Option<i64> {
        self.reply.as_ref().map(|r| r.available_bandwidth)
    }
}

impl RemainingBandwidthResponse for v6::response::RemainingBandwidthResponse {
    fn available_bandwidth(&self) -> Option<i64> {
        self.reply.as_ref().map(|r| r.available_bandwidth)
    }
}

pub trait TopUpBandwidthResponse: Id + UpgradeModeStatus + fmt::Debug {
    fn available_bandwidth(&self) -> i64;
}

impl TopUpBandwidthResponse for v3::response::TopUpBandwidthResponse {
    fn available_bandwidth(&self) -> i64 {
        self.reply.available_bandwidth
    }
}

impl TopUpBandwidthResponse for v4::response::TopUpBandwidthResponse {
    fn available_bandwidth(&self) -> i64 {
        self.reply.available_bandwidth
    }
}

impl TopUpBandwidthResponse for v5::response::TopUpBandwidthResponse {
    fn available_bandwidth(&self) -> i64 {
        self.reply.available_bandwidth
    }
}

impl TopUpBandwidthResponse for v6::response::TopUpBandwidthResponse {
    fn available_bandwidth(&self) -> i64 {
        self.reply.available_bandwidth
    }
}
