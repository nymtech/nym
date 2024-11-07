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
    latest::{
        registration::{
            FinalMessage, GatewayClient, InitMessage, IpPair, PendingRegistrations, PrivateIPs,
            RegistrationData, RegistredData, RemainingBandwidthData,
        },
        request::{AuthenticatorRequest, AuthenticatorRequestData},
        response::AuthenticatorResponse,
    },
    v1, v2, v3, v4, CURRENT_VERSION,
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
use nym_service_provider_requests_common::ServiceProviderType;
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use nym_wireguard::WireguardGatewayData;
use nym_wireguard_types::PeerPublicKey;
use rand::{prelude::IteratorRandom, thread_rng};
use tokio::sync::RwLock;
use tokio_stream::wrappers::IntervalStream;

use crate::{config::Config, error::*};

type AuthenticatorHandleResult = Result<AuthenticatorResponse>;
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
                .get_mut(&reg.gateway_data.private_ips)
                .ok_or(AuthenticatorError::InternalDataCorruption(format!(
                    "IPs {} should be present",
                    reg.gateway_data.private_ips
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
        init_message: InitMessage,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let remote_public = init_message.pub_key;
        let nonce: u64 = fastrand::u64(..);
        let mut registred_and_free = self.registred_and_free.write().await;
        if let Some(registration_data) = registred_and_free
            .registration_in_progres
            .get(&remote_public)
        {
            return Ok(AuthenticatorResponse::new_pending_registration_success(
                registration_data.clone(),
                request_id,
                reply_to,
            ));
        }

        let peer = self.peer_manager.query_peer(remote_public).await?;
        if let Some(peer) = peer {
            let private_ipv4 = peer
                .allowed_ips
                .iter()
                .find_map(|ip_mask| match ip_mask.ip {
                    std::net::IpAddr::V4(ipv4_addr) => Some(ipv4_addr),
                    _ => None,
                });
            let private_ipv6 = peer
                .allowed_ips
                .iter()
                .find_map(|ip_mask| match ip_mask.ip {
                    std::net::IpAddr::V6(ipv6_addr) => Some(ipv6_addr),
                    _ => None,
                });
            let (Some(allowed_ipv4), Some(allowed_ipv6)) = (private_ipv4, private_ipv6) else {
                return Err(AuthenticatorError::InternalError(
                    "there should be one private IP pair in the list".to_string(),
                ));
            };
            return Ok(AuthenticatorResponse::new_registered(
                RegistredData {
                    pub_key: PeerPublicKey::new(self.keypair().public_key().to_bytes().into()),
                    private_ips: IpPair::new(allowed_ipv4, allowed_ipv6),
                    wg_port: self.config.authenticator.announced_port,
                },
                reply_to,
                request_id,
            ));
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
            gateway_data,
            wg_port: self.config.authenticator.announced_port,
        };
        registred_and_free
            .registration_in_progres
            .insert(remote_public, registration_data.clone());

        Ok(AuthenticatorResponse::new_pending_registration_success(
            registration_data,
            request_id,
            reply_to,
        ))
    }

    async fn on_final_request(
        &mut self,
        final_message: FinalMessage,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let mut registred_and_free = self.registred_and_free.write().await;
        let registration_data = registred_and_free
            .registration_in_progres
            .get(&final_message.gateway_client.pub_key())
            .ok_or(AuthenticatorError::RegistrationNotInProgress)?
            .clone();

        if final_message
            .gateway_client
            .verify(self.keypair().private_key(), registration_data.nonce)
            .is_err()
        {
            return Err(AuthenticatorError::MacVerificationFailure);
        }

        let mut peer = Peer::new(Key::new(final_message.gateway_client.pub_key.to_bytes()));
        peer.allowed_ips.push(IpAddrMask::new(
            final_message.gateway_client.private_ips.ipv4.into(),
            32,
        ));
        peer.allowed_ips.push(IpAddrMask::new(
            final_message.gateway_client.private_ips.ipv6.into(),
            128,
        ));

        // If gateway does ecash verification and client sends a credential, we do the additional
        // credential verification. Later this will become mandatory.
        if let (Some(ecash_verifier), Some(credential)) = (
            self.ecash_verifier.clone(),
            final_message.credential.clone(),
        ) {
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
            .remove(&final_message.gateway_client.pub_key());

        Ok(AuthenticatorResponse::new_registered(
            RegistredData {
                pub_key: registration_data.gateway_data.pub_key,
                private_ips: registration_data.gateway_data.private_ips,
                wg_port: registration_data.wg_port,
            },
            reply_to,
            request_id,
        ))
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
        peer_public_key: PeerPublicKey,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let bandwidth_data = self.peer_manager.query_bandwidth(peer_public_key).await?;
        Ok(AuthenticatorResponse::new_remaining_bandwidth(
            bandwidth_data,
            reply_to,
            request_id,
        ))
    }

    async fn on_topup_bandwidth_request(
        &mut self,
        peer_public_key: PeerPublicKey,
        credential: CredentialSpendingData,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let Some(ecash_verifier) = self.ecash_verifier.clone() else {
            return Err(AuthenticatorError::UnsupportedOperation);
        };
        let client_id = ecash_verifier
            .storage()
            .get_wireguard_peer(&peer_public_key.to_string())
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
        let available_bandwidth = verifier.verify().await?;

        Ok(AuthenticatorResponse::new_topup_bandwidth(
            RemainingBandwidthData {
                available_bandwidth,
            },
            reply_to,
            request_id,
        ))
    }

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> AuthenticatorHandleResult {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

        let request = match deserialize_request(&reconstructed) {
            Err(AuthenticatorError::InvalidPacketVersion(version)) => {
                return self.on_version_mismatch(version, &reconstructed);
            }
            req => req,
        }?;

        match request.data {
            AuthenticatorRequestData::Initial(init_msg) => {
                self.on_initial_request(init_msg, request.request_id, request.reply_to)
                    .await
            }
            AuthenticatorRequestData::Final(final_msg) => {
                self.on_final_request(*final_msg, request.request_id, request.reply_to)
                    .await
            }
            AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                self.on_query_bandwidth_request(
                    peer_public_key,
                    request.request_id,
                    request.reply_to,
                )
                .await
            }
            AuthenticatorRequestData::TopUpBandwidth(topup_message) => {
                self.on_topup_bandwidth_request(
                    topup_message.pub_key,
                    topup_message.credential,
                    request.request_id,
                    request.reply_to,
                )
                .await
            }
        }
    }

    fn on_version_mismatch(
        &self,
        version: u8,
        _reconstructed: &ReconstructedMessage,
    ) -> AuthenticatorHandleResult {
        // If it's possible to parse, do so and return back a response, otherwise just drop
        Err(AuthenticatorError::InvalidPacketVersion(version))
    }

    // When an incoming mixnet message triggers a response that we send back.
    async fn handle_response(&self, response: AuthenticatorResponse) -> Result<()> {
        let recipient = response.recipient();

        let response_packet = response.to_bytes().map_err(|err| {
            log::error!("Failed to serialize response packet");
            AuthenticatorError::FailedToSerializeResponsePacket { source: err }
        })?;

        let input_message =
            InputMessage::new_regular(recipient, response_packet, TransmissionLane::General, None);
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
                            Ok(response) => {
                                if let Err(err) = self.handle_response(response).await {
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
            .map(Into::<v2::request::AuthenticatorRequest>::into)
            .map(Into::<v3::request::AuthenticatorRequest>::into)
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
