use std::{
    fmt,
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use nym_authenticator_requests::{
    v2, v3, v4,
    v5::{self, registration::IpPair},
};

use nym_credentials_interface::CredentialSpendingData;
use nym_crypto::asymmetric::x25519::PrivateKey;
use nym_sdk::mixnet::{
    IncludedSurbs, MixnetClientSender, MixnetMessageSender, Recipient, ReconstructedMessage,
    TransmissionLane,
};
use nym_service_provider_requests_common::ServiceProviderType;
use nym_wireguard_types::PeerPublicKey;
use tracing::{debug, error};

mod error;
mod mixnet_listener;

pub use crate::{
    error::{Error, Result},
    mixnet_listener::{
        AuthClientMixnetListener, AuthClientMixnetListenerHandle, MixnetMessageBroadcastReceiver,
    },
};

pub trait Versionable {
    fn version(&self) -> AuthenticatorVersion;
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

pub trait InitMessage: Versionable + fmt::Debug {
    fn pub_key(&self) -> PeerPublicKey;
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

pub trait FinalMessage: Versionable + fmt::Debug {
    fn gateway_client_pub_key(&self) -> PeerPublicKey;
    fn gateway_client_ipv4(&self) -> Option<Ipv4Addr>;
    fn gateway_client_ipv6(&self) -> Option<Ipv6Addr>;
    fn gateway_client_mac(&self) -> Vec<u8>;
    fn credential(&self) -> Option<CredentialSpendingData>;
}

impl FinalMessage for v2::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
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

    fn credential(&self) -> Option<CredentialSpendingData> {
        self.credential.clone()
    }
}

impl FinalMessage for v3::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
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

    fn credential(&self) -> Option<CredentialSpendingData> {
        self.credential.clone()
    }
}

impl FinalMessage for v4::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
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

    fn credential(&self) -> Option<CredentialSpendingData> {
        self.credential.clone()
    }
}

impl FinalMessage for v5::registration::FinalMessage {
    fn gateway_client_pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
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

    fn credential(&self) -> Option<CredentialSpendingData> {
        self.credential.clone()
    }
}

// Temporary solution for lacking a query message wrapper in monorepo
#[derive(Debug)]
pub struct QueryMessageImpl {
    pub pub_key: PeerPublicKey,
    pub version: AuthenticatorVersion,
}

impl Versionable for QueryMessageImpl {
    fn version(&self) -> AuthenticatorVersion {
        self.version
    }
}

pub trait QueryMessage: Versionable + fmt::Debug {
    fn pub_key(&self) -> PeerPublicKey;
}

impl QueryMessage for QueryMessageImpl {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
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

#[derive(Debug)]
pub enum ClientMessage {
    Initial(Box<dyn InitMessage + Send + Sync + 'static>),
    Final(Box<dyn FinalMessage + Send + Sync + 'static>),
    Query(Box<dyn QueryMessage + Send + Sync + 'static>),
    TopUp(Box<dyn TopUpMessage + Send + Sync + 'static>),
}

impl ClientMessage {
    // check if message is wasteful e.g. contains a credential
    pub fn is_wasteful(&self) -> bool {
        match self {
            Self::Final(msg) => msg.credential().is_some(),
            Self::TopUp(_) => true,
            Self::Initial(_) | Self::Query(_) => false,
        }
    }

    fn version(&self) -> AuthenticatorVersion {
        match self {
            ClientMessage::Initial(msg) => msg.version(),
            ClientMessage::Final(msg) => msg.version(),
            ClientMessage::Query(msg) => msg.version(),
            ClientMessage::TopUp(msg) => msg.version(),
        }
    }

    pub fn bytes(&self, reply_to: Recipient) -> Result<(Vec<u8>, u64)> {
        match self.version() {
            AuthenticatorVersion::V2 => {
                use v2::{
                    registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
                    request::AuthenticatorRequest,
                };
                match self {
                    ClientMessage::Initial(init_message) => {
                        let (req, id) = AuthenticatorRequest::new_initial_request(
                            InitMessage {
                                pub_key: init_message.pub_key(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Final(final_message) => {
                        let (req, id) = AuthenticatorRequest::new_final_request(
                            FinalMessage {
                                gateway_client: GatewayClient {
                                    pub_key: final_message.gateway_client_pub_key(),
                                    private_ip: final_message
                                        .gateway_client_ipv4()
                                        .ok_or(Error::UnsupportedMessage)?
                                        .into(),
                                    mac: ClientMac::new(final_message.gateway_client_mac()),
                                },
                                credential: final_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) = AuthenticatorRequest::new_query_request(
                            query_message.pub_key(),
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    _ => Err(Error::UnsupportedMessage),
                }
            }
            AuthenticatorVersion::V3 => {
                use v3::{
                    registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
                    request::AuthenticatorRequest,
                    topup::TopUpMessage,
                };
                match self {
                    ClientMessage::Initial(init_message) => {
                        let (req, id) = AuthenticatorRequest::new_initial_request(
                            InitMessage {
                                pub_key: init_message.pub_key(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Final(final_message) => {
                        let (req, id) = AuthenticatorRequest::new_final_request(
                            FinalMessage {
                                gateway_client: GatewayClient {
                                    pub_key: final_message.gateway_client_pub_key(),
                                    private_ip: final_message
                                        .gateway_client_ipv4()
                                        .ok_or(Error::UnsupportedMessage)?
                                        .into(),
                                    mac: ClientMac::new(final_message.gateway_client_mac()),
                                },
                                credential: final_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) = AuthenticatorRequest::new_query_request(
                            query_message.pub_key(),
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::TopUp(top_up_message) => {
                        let (req, id) = AuthenticatorRequest::new_topup_request(
                            TopUpMessage {
                                pub_key: top_up_message.pub_key(),
                                credential: top_up_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                }
            }
            AuthenticatorVersion::V4 => {
                use v4::{
                    registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
                    request::AuthenticatorRequest,
                    topup::TopUpMessage,
                };
                match self {
                    ClientMessage::Initial(init_message) => {
                        let (req, id) = AuthenticatorRequest::new_initial_request(
                            InitMessage {
                                pub_key: init_message.pub_key(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Final(final_message) => {
                        let (req, id) = AuthenticatorRequest::new_final_request(
                            FinalMessage {
                                gateway_client: GatewayClient {
                                    pub_key: final_message.gateway_client_pub_key(),
                                    private_ips: IpPair {
                                        ipv4: final_message
                                            .gateway_client_ipv4()
                                            .ok_or(Error::UnsupportedMessage)?,
                                        ipv6: final_message
                                            .gateway_client_ipv6()
                                            .ok_or(Error::UnsupportedMessage)?,
                                    }
                                    .into(),
                                    mac: ClientMac::new(final_message.gateway_client_mac()),
                                },
                                credential: final_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) = AuthenticatorRequest::new_query_request(
                            query_message.pub_key(),
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::TopUp(top_up_message) => {
                        let (req, id) = AuthenticatorRequest::new_topup_request(
                            TopUpMessage {
                                pub_key: top_up_message.pub_key(),
                                credential: top_up_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                }
            }
            AuthenticatorVersion::V5 => {
                use v5::{
                    registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
                    request::AuthenticatorRequest,
                    topup::TopUpMessage,
                };
                match self {
                    ClientMessage::Initial(init_message) => {
                        let (req, id) = AuthenticatorRequest::new_initial_request(InitMessage {
                            pub_key: init_message.pub_key(),
                        });
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Final(final_message) => {
                        let (req, id) = AuthenticatorRequest::new_final_request(FinalMessage {
                            gateway_client: GatewayClient {
                                pub_key: final_message.gateway_client_pub_key(),
                                private_ips: IpPair {
                                    ipv4: final_message
                                        .gateway_client_ipv4()
                                        .ok_or(Error::UnsupportedMessage)?,
                                    ipv6: final_message
                                        .gateway_client_ipv6()
                                        .ok_or(Error::UnsupportedMessage)?,
                                },
                                mac: ClientMac::new(final_message.gateway_client_mac()),
                            },
                            credential: final_message.credential(),
                        });
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) =
                            AuthenticatorRequest::new_query_request(query_message.pub_key());
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::TopUp(top_up_message) => {
                        let (req, id) = AuthenticatorRequest::new_topup_request(TopUpMessage {
                            pub_key: top_up_message.pub_key(),
                            credential: top_up_message.credential(),
                        });
                        Ok((req.to_bytes()?, id))
                    }
                }
            }
            AuthenticatorVersion::UNKNOWN => Err(Error::UnknownVersion),
        }
    }

    pub fn use_surbs(&self) -> bool {
        match self.version() {
            AuthenticatorVersion::V2 | AuthenticatorVersion::V3 | AuthenticatorVersion::V4 => false,
            AuthenticatorVersion::V5 => true,
            AuthenticatorVersion::UNKNOWN => true,
        }
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

pub trait PendingRegistrationResponse: Id + fmt::Debug {
    fn nonce(&self) -> u64;
    fn verify(
        &self,
        gateway_key: &PrivateKey,
    ) -> std::result::Result<(), nym_authenticator_requests::Error>;
    fn pub_key(&self) -> PeerPublicKey;
    fn private_ips(&self) -> IpPair;
}

impl PendingRegistrationResponse for v2::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(
        &self,
        gateway_key: &PrivateKey,
    ) -> std::result::Result<(), nym_authenticator_requests::Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ip.into()
    }
}

impl PendingRegistrationResponse for v3::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(
        &self,
        gateway_key: &PrivateKey,
    ) -> std::result::Result<(), nym_authenticator_requests::Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ip.into()
    }
}

impl PendingRegistrationResponse for v4::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(
        &self,
        gateway_key: &PrivateKey,
    ) -> std::result::Result<(), nym_authenticator_requests::Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ips.into()
    }
}

impl PendingRegistrationResponse for v5::response::PendingRegistrationResponse {
    fn nonce(&self) -> u64 {
        self.reply.nonce
    }

    fn verify(
        &self,
        gateway_key: &PrivateKey,
    ) -> std::result::Result<(), nym_authenticator_requests::Error> {
        self.reply.gateway_data.verify(gateway_key, self.nonce())
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.gateway_data.pub_key
    }

    fn private_ips(&self) -> IpPair {
        self.reply.gateway_data.private_ips
    }
}

pub trait RegisteredResponse: Id + fmt::Debug {
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
        self.reply.private_ips.into()
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
        self.reply.private_ips
    }

    fn pub_key(&self) -> PeerPublicKey {
        self.reply.pub_key
    }

    fn wg_port(&self) -> u16 {
        self.reply.wg_port
    }
}

pub trait RemainingBandwidthResponse: Id + fmt::Debug {
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

pub trait TopUpBandwidthResponse: Id + fmt::Debug {
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

#[derive(Debug)]
pub enum AuthenticatorResponse {
    PendingRegistration(Box<dyn PendingRegistrationResponse + Send + Sync + 'static>),
    Registered(Box<dyn RegisteredResponse + Send + Sync + 'static>),
    RemainingBandwidth(Box<dyn RemainingBandwidthResponse + Send + Sync + 'static>),
    TopUpBandwidth(Box<dyn TopUpBandwidthResponse + Send + Sync + 'static>),
}

impl Id for AuthenticatorResponse {
    fn id(&self) -> u64 {
        match self {
            AuthenticatorResponse::PendingRegistration(pending_registration_response) => {
                pending_registration_response.id()
            }
            AuthenticatorResponse::Registered(registered_response) => registered_response.id(),
            AuthenticatorResponse::RemainingBandwidth(remaining_bandwidth_response) => {
                remaining_bandwidth_response.id()
            }
            AuthenticatorResponse::TopUpBandwidth(top_up_bandwidth_response) => {
                top_up_bandwidth_response.id()
            }
        }
    }
}

impl From<v2::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v2::response::AuthenticatorResponse) -> Self {
        match value.data {
            v2::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v2::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v2::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
        }
    }
}

impl From<v3::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v3::response::AuthenticatorResponse) -> Self {
        match value.data {
            v3::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v3::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v3::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
            v3::response::AuthenticatorResponseData::TopUpBandwidth(top_up_bandwidth_response) => {
                Self::TopUpBandwidth(Box::new(top_up_bandwidth_response))
            }
        }
    }
}

impl From<v4::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v4::response::AuthenticatorResponse) -> Self {
        match value.data {
            v4::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v4::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v4::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
            v4::response::AuthenticatorResponseData::TopUpBandwidth(top_up_bandwidth_response) => {
                Self::TopUpBandwidth(Box::new(top_up_bandwidth_response))
            }
        }
    }
}

impl From<v5::response::AuthenticatorResponse> for AuthenticatorResponse {
    fn from(value: v5::response::AuthenticatorResponse) -> Self {
        match value.data {
            v5::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(Box::new(pending_registration_response)),
            v5::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(Box::new(registered_response))
            }
            v5::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(Box::new(remaining_bandwidth_response)),
            v5::response::AuthenticatorResponseData::TopUpBandwidth(top_up_bandwidth_response) => {
                Self::TopUpBandwidth(Box::new(top_up_bandwidth_response))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuthenticatorVersion {
    V2,
    V3,
    V4,
    V5,
    UNKNOWN,
}

impl AuthenticatorVersion {
    pub const LATEST: AuthenticatorVersion = AuthenticatorVersion::V5;
}

impl fmt::Display for AuthenticatorVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V2 => write!(f, "v2"),
            Self::V3 => write!(f, "v3"),
            Self::V4 => write!(f, "v4"),
            Self::V5 => write!(f, "v5"),
            Self::UNKNOWN => write!(f, "unknown"),
        }
    }
}

impl From<u8> for AuthenticatorVersion {
    fn from(value: u8) -> Self {
        if value == 2 {
            Self::V2
        } else if value == 3 {
            Self::V3
        } else if value == 4 {
            Self::V4
        } else if value == 5 {
            Self::V5
        } else {
            Self::UNKNOWN
        }
    }
}

impl From<&str> for AuthenticatorVersion {
    fn from(value: &str) -> Self {
        let Ok(semver) = semver::Version::parse(value) else {
            return Self::UNKNOWN;
        };

        semver.into()
    }
}

impl From<Option<&String>> for AuthenticatorVersion {
    fn from(value: Option<&String>) -> Self {
        match value {
            None => Self::UNKNOWN,
            Some(value) => value.as_str().into(),
        }
    }
}

impl From<String> for AuthenticatorVersion {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<Option<String>> for AuthenticatorVersion {
    fn from(value: Option<String>) -> Self {
        value.as_ref().into()
    }
}

impl From<semver::Version> for AuthenticatorVersion {
    fn from(semver: semver::Version) -> Self {
        if semver.major < 1 {
            return Self::UNKNOWN;
        }
        if semver.minor < 1 {
            return Self::UNKNOWN;
        }
        if semver.minor == 1 && semver.patch == 9 {
            return Self::V2;
        }
        if semver.minor == 1 && semver.patch >= 10 {
            return Self::V3;
        }
        if semver.minor < 6 {
            return Self::V4;
        }
        if semver.minor == 6 && semver.patch == 0 {
            return Self::V4;
        }
        if semver.minor == 6 && semver.patch >= 1 {
            return Self::V5;
        }
        Self::LATEST
    }
}

#[derive(Clone)]
pub struct AuthenticatorClient {
    auth_mix_client: AuthenticatorMixnetClient,
    auth_recipient: Recipient,
    auth_version: AuthenticatorVersion,
}

impl AuthenticatorClient {
    pub fn new(
        auth_mix_client: AuthenticatorMixnetClient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
    ) -> Self {
        Self {
            auth_mix_client,
            auth_recipient,
            auth_version,
        }
    }

    pub fn auth_recipient(&self) -> Recipient {
        self.auth_recipient
    }

    pub fn auth_version(&self) -> AuthenticatorVersion {
        self.auth_version
    }

    pub async fn send(&mut self, message: &ClientMessage) -> Result<AuthenticatorResponse> {
        self.auth_mix_client
            .send(message, self.auth_recipient)
            .await
    }
}

pub struct AuthenticatorMixnetClient {
    mixnet_listener: MixnetMessageBroadcastReceiver,
    mixnet_sender: MixnetClientSender,
    our_nym_address: Recipient,
}

impl Clone for AuthenticatorMixnetClient {
    fn clone(&self) -> Self {
        Self {
            mixnet_listener: self.mixnet_listener.resubscribe(),
            mixnet_sender: self.mixnet_sender.clone(),
            our_nym_address: self.our_nym_address,
        }
    }
}

impl AuthenticatorMixnetClient {
    pub async fn new(
        mixnet_sender: MixnetClientSender,
        mixnet_listener: MixnetMessageBroadcastReceiver,
        our_nym_address: Recipient,
    ) -> Self {
        Self {
            mixnet_listener,
            mixnet_sender,
            our_nym_address,
        }
    }

    pub async fn send(
        &mut self,
        message: &ClientMessage,
        authenticator_address: Recipient,
    ) -> Result<AuthenticatorResponse> {
        self.send_inner(message, authenticator_address).await
    }

    async fn send_inner(
        &mut self,
        message: &ClientMessage,
        authenticator_address: Recipient,
    ) -> Result<AuthenticatorResponse> {
        let request_id = self
            .send_connect_request(message, authenticator_address)
            .await?;

        debug!("Waiting for reply...");
        self.listen_for_connect_response(request_id).await
    }

    async fn send_connect_request(
        &self,
        message: &ClientMessage,
        authenticator_address: Recipient,
    ) -> Result<u64> {
        let (data, request_id) = message.bytes(self.our_nym_address)?;

        // We use 20 surbs for the connect request because typically the
        // authenticator mixnet client on the nym-node is configured to have a min
        // threshold of 10 surbs that it reserves for itself to request additional
        // surbs.
        let surbs = if message.use_surbs() {
            match &message {
                ClientMessage::Initial(_) => IncludedSurbs::new(20),
                _ => IncludedSurbs::new(1),
            }
        } else {
            IncludedSurbs::ExposeSelfAddress
        };
        let input_message = create_input_message(authenticator_address, data, surbs);

        self.mixnet_sender
            .send(input_message)
            .await
            .map_err(|e| Error::SendMixnetMessage(Box::new(e)))?;

        Ok(request_id)
    }

    async fn listen_for_connect_response(
        &mut self,
        request_id: u64,
    ) -> Result<AuthenticatorResponse> {
        let timeout = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    error!("Timed out waiting for reply to connect request");
                    return Err(Error::TimeoutWaitingForConnectResponse);
                }
                msg = self.mixnet_listener.recv() => match msg {
                    Err(_) => {
                        return Err(Error::NoMixnetMessagesReceived);
                    }
                    Ok(msg) => {
                        if !check_if_authenticator_message(&msg) {
                            debug!("Received non-authenticator message while waiting for connect response");
                            continue;
                        }
                        // Confirm that the version is correct
                        let version = check_auth_message_version(&msg)?;

                        // Then we deserialize the message
                        debug!("AuthClient: got message while waiting for connect response with version {version:?}");
                        let ret: Result<AuthenticatorResponse> = match version {
                            AuthenticatorVersion::V2 => v2::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::V4 => v4::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::V5 => v5::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::UNKNOWN => Err(Error::UnknownVersion),
                        };
                        let Ok(response) = ret else {
                            // This is ok, it's likely just one of our self-pings
                            debug!("Failed to deserialize reconstructed message");
                            continue;
                        };

                        if response.id() == request_id {
                            debug!("Got response with matching id");
                            return Ok(response);
                        }
                    }
                }
            }
        }
    }
}

fn check_if_authenticator_message(message: &ReconstructedMessage) -> bool {
    if let Some(msg_type) = message.message.get(1) {
        ServiceProviderType::Authenticator as u8 == *msg_type
    } else {
        false
    }
}

fn check_auth_message_version(message: &ReconstructedMessage) -> Result<AuthenticatorVersion> {
    // Assuing it's an Authenticator message, it will have a version as its first byte
    if let Some(&version) = message.message.first() {
        Ok(version.into())
    } else {
        Err(Error::NoVersionInMessage)
    }
}

fn create_input_message(
    recipient: Recipient,
    data: Vec<u8>,
    surbs: IncludedSurbs,
) -> nym_sdk::mixnet::InputMessage {
    match surbs {
        IncludedSurbs::Amount(surbs) => nym_sdk::mixnet::InputMessage::new_anonymous(
            recipient,
            data,
            surbs,
            TransmissionLane::General,
            None,
            None
        ),
        IncludedSurbs::ExposeSelfAddress => nym_sdk::mixnet::InputMessage::new_regular(
            recipient,
            data,
            TransmissionLane::General,
            None,
            None
        ),
    }
}
