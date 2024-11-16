// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::{error::AuthenticatorError, peer_manager::PeerManager};
use defguard_wireguard_rs::net::IpAddrMask;
use defguard_wireguard_rs::{host::Peer, key::Key};
use futures::StreamExt;
use nym_authenticator_requests::{
    traits::{
        AuthenticatorRequest, AuthenticatorVersion, FinalMessage, InitMessage,
        QueryBandwidthMessage, TopUpMessage,
    },
    v1, v2,
    v3::{
        self,
        registration::{
            GatewayClient, PendingRegistrations, PrivateIPs, RegistrationData,
            RemainingBandwidthData,
        },
    },
    CURRENT_VERSION,
};
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, ecash::EcashManager,
    BandwidthFlushingBehaviourConfig, ClientBandwidth, CredentialVerifier,
};
use nym_credentials_interface::CredentialSpendingData;
use nym_crypto::asymmetric::x25519::KeyPair;
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::Storage;
use nym_sdk::mixnet::{InputMessage, MixnetMessageSender, Recipient, TransmissionLane};
use nym_service_provider_requests_common::{Protocol, ServiceProviderType};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use nym_wireguard::WireguardGatewayData;
use nym_wireguard_types::PeerPublicKey;
use rand::{prelude::IteratorRandom, thread_rng};
use tokio::sync::RwLock;
use tokio_stream::wrappers::IntervalStream;

use crate::{config::Config, error::*};

type AuthenticatorHandleResult = Result<(Vec<u8>, Recipient)>;
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

pub(crate) struct MixnetListener<S> {
    // The configuration for the mixnet listener
    pub(crate) config: Config,

    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    // The task handle for the main loop
    pub(crate) task_handle: TaskHandle,

    // Registrations awaiting confirmation
    pub(crate) registred_and_free: RwLock<RegistredAndFree>,

    pub(crate) peer_manager: PeerManager,

    pub(crate) ecash_verifier: Option<Arc<EcashManager<S>>>,

    pub(crate) timeout_check_interval: IntervalStream,
}

impl<S: Storage + Clone + 'static> MixnetListener<S> {
    pub fn new(
        config: Config,
        free_private_network_ips: PrivateIPs,
        wireguard_gateway_data: WireguardGatewayData,
        mixnet_client: nym_sdk::mixnet::MixnetClient,
        task_handle: TaskHandle,
        ecash_verifier: Option<Arc<EcashManager<S>>>,
    ) -> Self {
        let timeout_check_interval =
            IntervalStream::new(tokio::time::interval(DEFAULT_REGISTRATION_TIMEOUT_CHECK));
        MixnetListener {
            config,
            mixnet_client,
            task_handle,
            registred_and_free: RwLock::new(RegistredAndFree::new(free_private_network_ips)),
            peer_manager: PeerManager::new(wireguard_gateway_data),
            ecash_verifier,
            timeout_check_interval,
        }
    }

    fn keypair(&self) -> &Arc<KeyPair> {
        self.peer_manager.wireguard_gateway_data.keypair()
    }

    async fn remove_stale_registrations(&self) -> Result<()> {
        let mut registred_and_free = self.registred_and_free.write().await;
        let registred_values: Vec<_> = registred_and_free
            .registration_in_progres
            .values()
            .cloned()
            .collect();
        for reg in registred_values {
            let ip = registred_and_free
                .free_private_network_ips
                .get_mut(&reg.gateway_data.private_ip)
                .ok_or(AuthenticatorError::InternalDataCorruption(format!(
                    "IP {} should be present",
                    reg.gateway_data.private_ip
                )))?;

            let Some(timestamp) = ip else {
                registred_and_free
                    .registration_in_progres
                    .remove(&reg.gateway_data.pub_key());
                log::debug!(
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
                log::debug!(
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
        reply_to: Recipient,
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
                                private_ip: gateway_data.private_ip,
                                mac: v1::ClientMac::new(gateway_data.mac.to_vec()),
                            },
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                        reply_to,
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
                            gateway_data: registration_data.gateway_data.clone().into(),
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                        reply_to,
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
                            gateway_data: registration_data.gateway_data.clone(),
                            wg_port: registration_data.wg_port,
                        },
                        request_id,
                        reply_to,
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
            let Some(allowed_ip) = peer.allowed_ips.first() else {
                return Err(AuthenticatorError::InternalError(
                    "private ip list should not be empty".to_string(),
                ));
            };
            let bytes = match AuthenticatorVersion::from(protocol) {
                AuthenticatorVersion::V1 => v1::response::AuthenticatorResponse::new_registered(
                    v1::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ip: allowed_ip.ip,
                        wg_port: self.config.authenticator.announced_port,
                    },
                    reply_to,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?,
                AuthenticatorVersion::V2 => v2::response::AuthenticatorResponse::new_registered(
                    v2::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ip: allowed_ip.ip,
                        wg_port: self.config.authenticator.announced_port,
                    },
                    reply_to,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?,
                AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_registered(
                    v3::registration::RegistredData {
                        pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                        private_ip: allowed_ip.ip,
                        wg_port: self.config.authenticator.announced_port,
                    },
                    reply_to,
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
            wg_port: self.config.authenticator.announced_port,
        };
        registred_and_free
            .registration_in_progres
            .insert(remote_public, registration_data.clone());
        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V1 => {
                v1::response::AuthenticatorResponse::new_pending_registration_success(
                    v1::registration::RegistrationData {
                        nonce: registration_data.nonce,
                        gateway_data: v1::GatewayClient {
                            pub_key: gateway_data.pub_key,
                            private_ip: gateway_data.private_ip,
                            mac: v1::ClientMac::new(gateway_data.mac.to_vec()),
                        },
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                    reply_to,
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
                        gateway_data: registration_data.gateway_data.into(),
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                    reply_to,
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
                        gateway_data: registration_data.gateway_data,
                        wg_port: registration_data.wg_port,
                    },
                    request_id,
                    reply_to,
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
        reply_to: Recipient,
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
            .push(IpAddrMask::new(final_message.private_ip(), 32));

        // If gateway does ecash verification and client sends a credential, we do the additional
        // credential verification. Later this will become mandatory.
        if let (Some(ecash_verifier), Some(credential)) =
            (self.ecash_verifier.clone(), final_message.credential())
        {
            let client_id = ecash_verifier
                .storage()
                .insert_wireguard_peer(&peer, true)
                .await?
                .ok_or(AuthenticatorError::InternalError(
                    "peer with ticket shouldn't have been used before without a ticket".to_string(),
                ))?;
            if let Err(e) =
                Self::credential_verification(ecash_verifier.clone(), credential, client_id).await
            {
                ecash_verifier
                    .storage()
                    .remove_wireguard_peer(&peer.public_key.to_string())
                    .await?;
                return Err(e);
            }
            let public_key = peer.public_key.to_string();
            if let Err(e) = self.peer_manager.add_peer(peer, Some(client_id)).await {
                ecash_verifier
                    .storage()
                    .remove_wireguard_peer(&public_key)
                    .await?;
                return Err(e);
            }
        } else {
            self.peer_manager.add_peer(peer, None).await?;
        }
        registred_and_free
            .registration_in_progres
            .remove(&final_message.pub_key());

        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V1 => v1::response::AuthenticatorResponse::new_registered(
                v1::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ip: registration_data.gateway_data.private_ip,
                    wg_port: registration_data.wg_port,
                },
                reply_to,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V2 => v2::response::AuthenticatorResponse::new_registered(
                v2::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ip: registration_data.gateway_data.private_ip,
                    wg_port: registration_data.wg_port,
                },
                reply_to,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_registered(
                v3::registration::RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ip: registration_data.gateway_data.private_ip,
                    wg_port: registration_data.wg_port,
                },
                reply_to,
                request_id,
            )
            .to_bytes()
            .map_err(|err| AuthenticatorError::FailedToSerializeResponsePacket { source: err })?,
            AuthenticatorVersion::UNKNOWN => return Err(AuthenticatorError::UnknownVersion),
        };
        Ok((bytes, reply_to))
    }

    async fn credential_verification(
        ecash_verifier: Arc<EcashManager<S>>,
        credential: CredentialSpendingData,
        client_id: i64,
    ) -> Result<i64> {
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
        let client_bandwidth = ClientBandwidth::new(bandwidth.into());
        let mut verifier = CredentialVerifier::new(
            CredentialSpendingRequest::new(credential),
            ecash_verifier.clone(),
            BandwidthStorageManager::new(
                ecash_verifier.storage().clone(),
                client_bandwidth,
                client_id,
                BandwidthFlushingBehaviourConfig::default(),
                true,
            ),
        );
        Ok(verifier.verify().await?)
    }

    async fn on_query_bandwidth_request(
        &mut self,
        msg: Box<dyn QueryBandwidthMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let bandwidth_data = self.peer_manager.query_bandwidth(msg).await?;
        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V1 => {
                v1::response::AuthenticatorResponse::new_remaining_bandwidth(
                    bandwidth_data.map(|data| v1::registration::RemainingBandwidthData {
                        available_bandwidth: data.available_bandwidth as u64,
                        suspended: false,
                    }),
                    reply_to,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V2 => {
                v2::response::AuthenticatorResponse::new_remaining_bandwidth(
                    bandwidth_data.map(|data| v2::registration::RemainingBandwidthData {
                        available_bandwidth: data.available_bandwidth,
                    }),
                    reply_to,
                    request_id,
                )
                .to_bytes()
                .map_err(|err| {
                    AuthenticatorError::FailedToSerializeResponsePacket { source: err }
                })?
            }
            AuthenticatorVersion::V3 => {
                v3::response::AuthenticatorResponse::new_remaining_bandwidth(
                    bandwidth_data,
                    reply_to,
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
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let Some(ecash_verifier) = self.ecash_verifier.clone() else {
            return Err(AuthenticatorError::UnsupportedOperation);
        };
        let client_id = ecash_verifier
            .storage()
            .get_wireguard_peer(&msg.pub_key().to_string())
            .await?
            .ok_or(AuthenticatorError::MissingClientBandwidthEntry)?
            .client_id
            .ok_or(AuthenticatorError::OldClient)?;
        let bandwidth = ecash_verifier
            .storage()
            .get_available_bandwidth(client_id)
            .await?
            .ok_or(AuthenticatorError::InternalError(
                "bandwidth entry should have just been created".to_string(),
            ))?;

        let client_bandwidth = ClientBandwidth::new(bandwidth.into());
        let mut verifier = CredentialVerifier::new(
            CredentialSpendingRequest::new(msg.credential()),
            ecash_verifier.clone(),
            BandwidthStorageManager::new(
                ecash_verifier.storage().clone(),
                client_bandwidth,
                client_id,
                BandwidthFlushingBehaviourConfig::default(),
                true,
            ),
        );
        let available_bandwidth = verifier.verify().await?;

        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_topup_bandwidth(
                RemainingBandwidthData {
                    available_bandwidth,
                },
                reply_to,
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

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> AuthenticatorHandleResult {
        log::debug!(
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
    async fn handle_response(&self, response: Vec<u8>, recipient: Recipient) -> Result<()> {
        let input_message =
            InputMessage::new_regular(recipient, response, TransmissionLane::General, None);
        self.mixnet_client
            .send(input_message)
            .await
            .map_err(|err| AuthenticatorError::FailedToSendPacketToMixnet { source: err })
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        log::info!("Using authenticator version {}", CURRENT_VERSION);
        let mut task_client = self.task_handle.fork("main_loop");

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("Authenticator [main loop]: received shutdown");
                },
                _ = self.timeout_check_interval.next() => {
                    if let Err(e) = self.remove_stale_registrations().await {
                        log::error!("Could not clear stale registrations. The registration process might get jammed soon - {:?}", e);
                    }
                }
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok((response, recipient)) => {
                                if let Err(err) = self.handle_response(response, recipient).await {
                                    log::error!("Mixnet listener failed to handle response: {err}");
                                }
                            }
                            Err(err) => {
                                log::error!("Error handling reconstructed mixnet message: {err}");
                            }

                        };
                    } else {
                        log::trace!("Authenticator [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        log::debug!("Authenticator: stopping");
        Ok(())
    }
}

fn deserialize_request(reconstructed: &ReconstructedMessage) -> Result<AuthenticatorRequest> {
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
        [version, _] => {
            log::info!("Received packet with invalid version: v{version}");
            Err(AuthenticatorError::InvalidPacketVersion(version))
        }
    }
}
