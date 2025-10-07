// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::node::internal_service_providers::authenticator::{
    config::Config, error::AuthenticatorError, peer_manager::PeerManager,
    seen_credential_cache::SeenCredentialCache,
};
use defguard_wireguard_rs::net::IpAddrMask;
use defguard_wireguard_rs::{host::Peer, key::Key};
use futures::StreamExt;
use nym_authenticator_requests::{
    latest::registration::RegistrationData, v4::registration::IpPair,
};
use nym_authenticator_requests::{
    latest::registration::{GatewayClient, PendingRegistrations, PrivateIPs},
    traits::{
        AuthenticatorRequest, AuthenticatorVersion, FinalMessage, InitMessage,
        QueryBandwidthMessage, TopUpMessage,
    },
    v1, v2, v3, v4, v5, CURRENT_VERSION,
};
use nym_credential_verification::ecash::traits::EcashManager;
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth, CredentialVerifier,
};
use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_crypto::asymmetric::x25519::KeyPair;
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::models::PersistedBandwidth;
use nym_sdk::mixnet::{
    AnonymousSenderTag, InputMessage, MixnetMessageSender, Recipient, TransmissionLane,
};
use nym_service_provider_requests_common::{Protocol, ServiceProviderType};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::ShutdownToken;
use nym_wireguard::WireguardGatewayData;
use nym_wireguard_types::PeerPublicKey;
use rand::{prelude::IteratorRandom, thread_rng};
use tokio::sync::RwLock;
use tokio_stream::wrappers::IntervalStream;

type AuthenticatorHandleResult = Result<(Vec<u8>, Option<Recipient>), AuthenticatorError>;
const DEFAULT_REGISTRATION_TIMEOUT_CHECK: Duration = Duration::from_secs(60); // 1 minute

pub(crate) struct RegistredAndFree {
    registration_in_progres: PendingRegistrations,
    free_private_network_ips: PrivateIPs,
}

impl RegistredAndFree {
    pub(crate) fn new(free_private_network_ips: PrivateIPs) -> Self {
        RegistredAndFree {
            registration_in_progres: Default::default(),
            free_private_network_ips,
        }
    }
}

pub(crate) struct MixnetListener {
    // The configuration for the mixnet listener
    pub(crate) config: Config,

    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    // Registrations awaiting confirmation
    pub(crate) registred_and_free: RwLock<RegistredAndFree>,

    pub(crate) peer_manager: PeerManager,

    pub(crate) ecash_verifier: Arc<dyn EcashManager + Send + Sync>,

    pub(crate) timeout_check_interval: IntervalStream,

    pub(crate) seen_credential_cache: SeenCredentialCache,
}

impl MixnetListener {
    pub fn new(
        config: Config,
        free_private_network_ips: PrivateIPs,
        wireguard_gateway_data: WireguardGatewayData,
        mixnet_client: nym_sdk::mixnet::MixnetClient,
        ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
    ) -> Self {
        let timeout_check_interval =
            IntervalStream::new(tokio::time::interval(DEFAULT_REGISTRATION_TIMEOUT_CHECK));
        MixnetListener {
            config,
            mixnet_client,
            registred_and_free: RwLock::new(RegistredAndFree::new(free_private_network_ips)),
            peer_manager: PeerManager::new(wireguard_gateway_data),
            ecash_verifier,
            timeout_check_interval,
            seen_credential_cache: SeenCredentialCache::new(),
        }
    }

    fn keypair(&self) -> &Arc<KeyPair> {
        self.peer_manager.wireguard_gateway_data.keypair()
    }

    async fn remove_stale_registrations(&self) -> Result<(), AuthenticatorError> {
        let mut registred_and_free = self.registred_and_free.write().await;
        let registred_values: Vec<_> = registred_and_free
            .registration_in_progres
            .values()
            .cloned()
            .collect();
        for reg in registred_values {
            let ip = registred_and_free
                .free_private_network_ips
                .get_mut(&reg.gateway_data.private_ips)
                .ok_or(AuthenticatorError::InternalDataCorruption(format!(
                    "IPs {} should be present",
                    reg.gateway_data.private_ips
                )))?;

            let Some(timestamp) = ip else {
                registred_and_free
                    .registration_in_progres
                    .remove(&reg.gateway_data.pub_key());
                tracing::debug!(
                    "Removed stale registration of {}",
                    reg.gateway_data.pub_key()
                );
                continue;
            };
            let duration = SystemTime::now().duration_since(*timestamp).map_err(|_| {
                AuthenticatorError::InternalDataCorruption(
                    "set timestamp shouldn't have been set in the future".to_string(),
                )
            })?;
            if duration > DEFAULT_REGISTRATION_TIMEOUT_CHECK {
                *ip = None;
                registred_and_free
                    .registration_in_progres
                    .remove(&reg.gateway_data.pub_key());
                tracing::debug!(
                    "Removed stale registration of {}",
                    reg.gateway_data.pub_key()
                );
            }
        }
        Ok(())
    }

    async fn on_initial_request(
        &mut self,
        init_message: Box<dyn InitMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let remote_public = init_message.pub_key();
        let nonce: u64 = fastrand::u64(..);
        let mut registred_and_free = self.registred_and_free.write().await;
        if let Some(registration_data) = registred_and_free
            .registration_in_progres
            .get(&remote_public)
        {
            let gateway_data = registration_data.gateway_data.clone();
            let bytes = match AuthenticatorVersion::from(protocol) {
                AuthenticatorVersion::V1 => {
                    v1::response::AuthenticatorResponse::new_pending_registration_success(
                        v1::registration::RegistrationData {
                            nonce: registration_data.nonce,
                            gateway_data: v1::GatewayClient {
                                pub_key: gateway_data.pub_key,
                                private_ip: gateway_data.private_ips.ipv4.into(),
                                mac: v1::ClientMac::new(gateway_data.mac.to_vec()),
                            },
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                        reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    )
                    .to_bytes()
                    .map_err(|err| {
                        AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                    })?
                }
                AuthenticatorVersion::V2 => {
                    v2::response::AuthenticatorResponse::new_pending_registration_success(
                        v2::registration::RegistrationData {
                            nonce: registration_data.nonce,
                            gateway_data: v2::registration::GatewayClient::new(
                                self.keypair().private_key(),
                                remote_public.inner(),
                                registration_data.gateway_data.private_ips.ipv4.into(),
                                registration_data.nonce,
                            ),
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                        reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    )
                    .to_bytes()
                    .map_err(|err| {
                        AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                    })?
                }
                AuthenticatorVersion::V3 => {
                    v3::response::AuthenticatorResponse::new_pending_registration_success(
                        v3::registration::RegistrationData {
                            nonce: registration_data.nonce,
                            gateway_data: v3::registration::GatewayClient::new(
                                self.keypair().private_key(),
                                remote_public.inner(),
                                registration_data.gateway_data.private_ips.ipv4.into(),
                                registration_data.nonce,
                            ),
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                        reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    )
                    .to_bytes()
                    .map_err(|err| {
                        AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                    })?
                }
                AuthenticatorVersion::V4 => {
                    v4::response::AuthenticatorResponse::new_pending_registration_success(
                        v4::registration::RegistrationData {
                            nonce: registration_data.nonce,
                            gateway_data: registration_data.gateway_data.clone().into(),
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                        reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    )
                    .to_bytes()
                    .map_err(|err| {
                        AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                    })?
                }
                AuthenticatorVersion::V5 => {
                    v5::response::AuthenticatorResponse::new_pending_registration_success(
                        v5::registration::RegistrationData {
                            nonce: registration_data.nonce,
                            gateway_data: registration_data.gateway_data.clone(),
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                    )
                    .to_bytes()
                    .map_err(|err| {
                        AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                    })?
                }
                AuthenticatorVersion::UNKNOWN => return Err(AuthenticatorError::UnknownVersion),
            };
            return Ok((bytes, reply_to));
        }

        let peer = self.peer_manager.query_peer(remote_public).await?;
        if let Some(peer) = peer {
            let allowed_ipv4 = peer
                .allowed_ips
                .iter()
                .find_map(|ip_mask| match ip_mask.ip {
                    std::net::IpAddr::V4(ipv4_addr) => Some(ipv4_addr),
                    _ => None,
                })
                .ok_or(AuthenticatorError::InternalError(
                    "there should be one private IPv4 in the list".to_string(),
                ))?;
            let allowed_ipv6 = peer
                .allowed_ips
                .iter()
                .find_map(|ip_mask| match ip_mask.ip {
                    std::net::IpAddr::V6(ipv6_addr) => Some(ipv6_addr),
                    _ => None,
                })
                .unwrap_or(IpPair::from(IpAddr::from(allowed_ipv4)).ipv6);
            let bytes = match AuthenticatorVersion::from(protocol) {
                AuthenticatorVersion::V1 => v1::response::AuthenticatorResponse::new_registered(
                    v1::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ip: allowed_ipv4.into(),
                        wg_port: self.config.authenticator.tunnel_announced_port,
                    },
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?,
                AuthenticatorVersion::V2 => v2::response::AuthenticatorResponse::new_registered(
                    v2::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ip: allowed_ipv4.into(),
                        wg_port: self.config.authenticator.tunnel_announced_port,
                    },
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?,
                AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_registered(
                    v3::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ip: allowed_ipv4.into(),
                        wg_port: self.config.authenticator.tunnel_announced_port,
                    },
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?,
                AuthenticatorVersion::V4 => v4::response::AuthenticatorResponse::new_registered(
                    v4::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ips: (allowed_ipv4, allowed_ipv6).into(),
                        wg_port: self.config.authenticator.tunnel_announced_port,
                    },
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?,
                AuthenticatorVersion::V5 => v5::response::AuthenticatorResponse::new_registered(
                    v5::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ips: (allowed_ipv4, allowed_ipv6).into(),
                        wg_port: self.config.authenticator.tunnel_announced_port,
                    },
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?,
                AuthenticatorVersion::UNKNOWN => return Err(AuthenticatorError::UnknownVersion),
            };
            return Ok((bytes, reply_to));
        }

        let private_ip_ref = registred_and_free
            .free_private_network_ips
            .iter_mut()
            .filter(|r| r.1.is_none())
            .choose(&mut thread_rng())
            .ok_or(AuthenticatorError::NoFreeIp)?;
        let private_ips = *private_ip_ref.0;
        // mark it as used, even though it's not final
        *private_ip_ref.1 = Some(SystemTime::now());
        let gateway_data = GatewayClient::new(
            self.keypair().private_key(),
            remote_public.inner(),
            *private_ip_ref.0,
            nonce,
        );
        let registration_data = RegistrationData {
            nonce,
            gateway_data: gateway_data.clone(),
            wg_port: self.config.authenticator.tunnel_announced_port,
        };
        registred_and_free
            .registration_in_progres
            .insert(remote_public, registration_data.clone());
        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V1 => {
                v1::response::AuthenticatorResponse::new_pending_registration_success(
                    v1::registration::RegistrationData {
                        nonce: registration_data.nonce,
                        gateway_data: v1::registration::GatewayClient::new(
                            self.keypair().private_key(),
                            remote_public.inner(),
                            private_ips.ipv4.into(),
                            nonce,
                        ),
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V2 => {
                v2::response::AuthenticatorResponse::new_pending_registration_success(
                    v2::registration::RegistrationData {
                        nonce: registration_data.nonce,
                        gateway_data: v2::registration::GatewayClient::new(
                            self.keypair().private_key(),
                            remote_public.inner(),
                            private_ips.ipv4.into(),
                            nonce,
                        ),
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V3 => {
                v3::response::AuthenticatorResponse::new_pending_registration_success(
                    v3::registration::RegistrationData {
                        nonce: registration_data.nonce,
                        gateway_data: v3::registration::GatewayClient::new(
                            self.keypair().private_key(),
                            remote_public.inner(),
                            private_ips.ipv4.into(),
                            nonce,
                        ),
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V4 => {
                v4::response::AuthenticatorResponse::new_pending_registration_success(
                    v4::registration::RegistrationData {
                        nonce: registration_data.nonce,
                        gateway_data: registration_data.gateway_data.into(),
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V5 => {
                v5::response::AuthenticatorResponse::new_pending_registration_success(
                    v5::registration::RegistrationData {
                        nonce: registration_data.nonce,
                        gateway_data: registration_data.gateway_data,
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::UNKNOWN => return Err(AuthenticatorError::UnknownVersion),
        };

        Ok((bytes, reply_to))
    }

    async fn on_final_request(
        &mut self,
        final_message: Box<dyn FinalMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let mut registred_and_free = self.registred_and_free.write().await;
        let registration_data = registred_and_free
            .registration_in_progres
            .get(&final_message.pub_key())
            .ok_or(AuthenticatorError::RegistrationNotInProgress)?
            .clone();

        if final_message
            .verify(self.keypair().private_key(), registration_data.nonce)
            .is_err()
        {
            return Err(AuthenticatorError::MacVerificationFailure);
        }

        let mut peer = Peer::new(Key::new(final_message.pub_key().to_bytes()));
        peer.allowed_ips
            .push(IpAddrMask::new(final_message.private_ips().ipv4.into(), 32));
        peer.allowed_ips.push(IpAddrMask::new(
            final_message.private_ips().ipv6.into(),
            128,
        ));

        let Some(credential) = final_message.credential() else {
            return Err(AuthenticatorError::NoCredentialReceived);
        };
        let client_id = self
            .ecash_verifier
            .storage()
            .insert_wireguard_peer(
                &peer,
                TicketType::try_from_encoded(credential.payment.t_type)
                    .map_err(|_| AuthenticatorError::InvalidCredentialType)?
                    .into(),
            )
            .await?;
        if let Err(e) =
            credential_verification(self.ecash_verifier.clone(), credential, client_id).await
        {
            self.ecash_verifier
                .storage()
                .remove_wireguard_peer(&peer.public_key.to_string())
                .await?;
            return Err(e);
        }
        let public_key = peer.public_key.to_string();
        if let Err(e) = self.peer_manager.add_peer(peer).await {
            self.ecash_verifier
                .storage()
                .remove_wireguard_peer(&public_key)
                .await?;
            return Err(e);
        }

        registred_and_free
            .registration_in_progres
            .remove(&final_message.pub_key());

        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V1 => v1::response::AuthenticatorResponse::new_registered(
                v1::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ip: registration_data.gateway_data.private_ips.ipv4.into(),
                    wg_port: registration_data.wg_port,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V2 => v2::response::AuthenticatorResponse::new_registered(
                v2::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ip: registration_data.gateway_data.private_ips.ipv4.into(),
                    wg_port: registration_data.wg_port,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_registered(
                v3::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ip: registration_data.gateway_data.private_ips.ipv4.into(),
                    wg_port: registration_data.wg_port,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V4 => v4::response::AuthenticatorResponse::new_registered(
                v4::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ips: registration_data.gateway_data.private_ips.into(),
                    wg_port: registration_data.wg_port,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V5 => v5::response::AuthenticatorResponse::new_registered(
                v5::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ips: registration_data.gateway_data.private_ips,
                    wg_port: registration_data.wg_port,
                },
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::UNKNOWN => return Err(AuthenticatorError::UnknownVersion),
        };
        Ok((bytes, reply_to))
    }

    async fn on_query_bandwidth_request(
        &mut self,
        msg: Box<dyn QueryBandwidthMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let available_bandwidth = self.peer_manager.query_bandwidth(msg.pub_key()).await?;
        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V1 => {
                v1::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v1::registration::RemainingBandwidthData {
                        available_bandwidth: available_bandwidth as u64,
                        suspended: false,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V2 => {
                v2::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v2::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V3 => {
                v3::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v3::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V4 => {
                v4::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v4::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V5 => {
                v5::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v5::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::UNKNOWN => return Err(AuthenticatorError::UnknownVersion),
        };
        Ok((bytes, reply_to))
    }

    async fn on_topup_bandwidth_request(
        &mut self,
        msg: Box<dyn TopUpMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let available_bandwidth = if self.received_retry(msg.as_ref()) {
            // don't process the credential and just return the current bandwidth
            self.peer_manager.query_bandwidth(msg.pub_key()).await?
        } else {
            let mut verifier = self
                .peer_manager
                .query_verifier_by_key(msg.pub_key(), msg.credential())
                .await?;
            let available_bandwidth = verifier.verify().await?;
            self.seen_credential_cache
                .insert_credential(msg.credential(), msg.pub_key());
            available_bandwidth
        };

        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V5 => v5::response::AuthenticatorResponse::new_topup_bandwidth(
                v5::registration::RemainingBandwidthData {
                    available_bandwidth,
                },
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V4 => v4::response::AuthenticatorResponse::new_topup_bandwidth(
                v4::registration::RemainingBandwidthData {
                    available_bandwidth,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_topup_bandwidth(
                v3::registration::RemainingBandwidthData {
                    available_bandwidth,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V1 | AuthenticatorVersion::V2 | AuthenticatorVersion::UNKNOWN => {
                return Err(AuthenticatorError::UnknownVersion)
            }
        };

        Ok((bytes, reply_to))
    }

    fn received_retry(&self, msg: &(dyn TopUpMessage + Send + Sync + 'static)) -> bool {
        if let Some(peer_pub_key) = self
            .seen_credential_cache
            .get_peer_pub_key(&msg.credential())
        {
            // check if the same peer sent the same credential twice, probably because of a retry
            peer_pub_key == msg.pub_key()
        } else {
            false
        }
    }

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> AuthenticatorHandleResult {
        tracing::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

        let request = deserialize_request(&reconstructed)?;

        match request {
            AuthenticatorRequest::Initial {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_initial_request(msg, protocol, request_id, reply_to)
                    .await
            }
            AuthenticatorRequest::Final {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_final_request(msg, protocol, request_id, reply_to)
                    .await
            }
            AuthenticatorRequest::QueryBandwidth {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_query_bandwidth_request(msg, protocol, request_id, reply_to)
                    .await
            }
            AuthenticatorRequest::TopUpBandwidth {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_topup_bandwidth_request(msg, protocol, request_id, reply_to)
                    .await
            }
        }
    }

    // When an incoming mixnet message triggers a response that we send back.
    async fn handle_response(
        &self,
        response: Vec<u8>,
        recipient: Option<Recipient>,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Result<(), AuthenticatorError> {
        let input_message = create_input_message(recipient, sender_tag, response)?;
        self.mixnet_client.send(input_message).await.map_err(|err| {
            AuthenticatorError::FailedToSendPacketToMixnet {
                source: Box::new(err),
            }
        })
    }

    pub(crate) async fn run(
        mut self,
        shutdown_token: ShutdownToken,
    ) -> Result<(), AuthenticatorError> {
        tracing::info!("Using authenticator version {CURRENT_VERSION}");

        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    tracing::debug!("Authenticator [main loop]: received shutdown");
                    break;
                },
                _ = self.timeout_check_interval.next() => {
                    if let Err(e) = self.remove_stale_registrations().await {
                        tracing::error!("Could not clear stale registrations. The registration process might get jammed soon - {e:?}");
                    }
                    self.seen_credential_cache.remove_stale();
                }
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        let sender_tag = msg.sender_tag;
                        match self.on_reconstructed_message(msg).await {
                            Ok((response, recipient)) => {
                                if let Err(err) = self.handle_response(response, recipient, sender_tag).await {
                                    tracing::error!("Mixnet listener failed to handle response: {err}");
                                }
                            }
                            Err(err) => {
                                tracing::error!("Error handling reconstructed mixnet message: {err}");
                            }

                        };
                    } else {
                        tracing::trace!("Authenticator [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        tracing::debug!("Authenticator: stopping");
        Ok(())
    }
}

pub async fn credential_storage_preparation(
    ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
    client_id: i64,
) -> Result<PersistedBandwidth, AuthenticatorError> {
    ecash_verifier
        .storage()
        .create_bandwidth_entry(client_id)
        .await?;
    let bandwidth = ecash_verifier
        .storage()
        .get_available_bandwidth(client_id)
        .await?
        .ok_or(AuthenticatorError::InternalError(
            "bandwidth entry should have just been created".to_string(),
        ))?;
    Ok(bandwidth)
}

async fn credential_verification(
    ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
    credential: CredentialSpendingData,
    client_id: i64,
) -> Result<i64, AuthenticatorError> {
    let bandwidth = credential_storage_preparation(ecash_verifier.clone(), client_id).await?;
    let client_bandwidth = ClientBandwidth::new(bandwidth.into());
    let mut verifier = CredentialVerifier::new(
        CredentialSpendingRequest::new(credential),
        ecash_verifier.clone(),
        BandwidthStorageManager::new(
            ecash_verifier.storage(),
            client_bandwidth,
            client_id,
            BandwidthFlushingBehaviourConfig::default(),
            true,
        ),
    );
    Ok(verifier.verify().await?)
}

fn deserialize_request(
    reconstructed: &ReconstructedMessage,
) -> Result<AuthenticatorRequest, AuthenticatorError> {
    let request_version = *reconstructed
        .message
        .first_chunk::<2>()
        .ok_or(AuthenticatorError::ShortPacket)?;

    // Check version of the request and convert to the latest version if necessary
    match request_version {
        [1, _] => v1::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
            .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err })
            .map(Into::into),
        [2, request_type] => {
            if request_type == ServiceProviderType::Authenticator as u8 {
                v2::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                    .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket {
                        source: err,
                    })
                    .map(Into::<v3::request::AuthenticatorRequest>::into)
                    .map(Into::into)
            } else {
                Err(AuthenticatorError::InvalidPacketType(request_type))
            }
        }
        [3, request_type] => {
            if request_type == ServiceProviderType::Authenticator as u8 {
                v3::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                    .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket {
                        source: err,
                    })
                    .map(Into::into)
            } else {
                Err(AuthenticatorError::InvalidPacketType(request_type))
            }
        }
        [4, request_type] => {
            if request_type == ServiceProviderType::Authenticator as u8 {
                v4::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                    .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket {
                        source: err,
                    })
                    .map(Into::into)
            } else {
                Err(AuthenticatorError::InvalidPacketType(request_type))
            }
        }
        [5, request_type] => {
            if request_type == ServiceProviderType::Authenticator as u8 {
                v5::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                    .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket {
                        source: err,
                    })
                    .map(Into::into)
            } else {
                Err(AuthenticatorError::InvalidPacketType(request_type))
            }
        }
        [version, _] => {
            tracing::info!("Received packet with invalid version: v{version}");
            Err(AuthenticatorError::InvalidPacketVersion(version))
        }
    }
}

fn create_input_message(
    nym_address: Option<Recipient>,
    reply_to_tag: Option<AnonymousSenderTag>,
    response_packet: Vec<u8>,
) -> Result<InputMessage, AuthenticatorError> {
    let lane = TransmissionLane::General;
    let packet_type = None;
    if let Some(reply_to_tag) = reply_to_tag {
        tracing::debug!("Creating message using SURB");
        Ok(InputMessage::new_reply(
            reply_to_tag,
            response_packet,
            lane,
            packet_type,
        ))
    } else if let Some(nym_address) = nym_address {
        tracing::debug!("Creating message using nym_address");
        Ok(InputMessage::new_regular(
            nym_address,
            response_packet,
            lane,
            packet_type,
            None,
        ))
    } else {
        tracing::error!("No nym-address or sender tag provided");
        Err(AuthenticatorError::MissingReplyToForOldClient)
    }
}
