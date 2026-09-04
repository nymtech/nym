// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::config::{NodeModes, ServiceProvidersConfig};
use crate::error::{NymNodeError, ServiceProvidersError};
use crate::node::description::load_node_description;
use crate::node::helpers::{
    load_ed25519_identity_public_key, load_key, load_x25519_noise_public_key,
    load_x25519_wireguard_public_key, store_key, store_keypair,
};
use crate::node::http::HttpServerConfig;
use crate::node::http::api::api_requests;
use crate::node::http::helpers::system_info::get_system_info;
use crate::node::http::state::StaticNodeInformation;
use celes::Country;
use nym_bin_common::bin_info_owned;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_crypto::aes::cipher::crypto_common::rand_core::{CryptoRng, OsRng, RngCore};
use nym_crypto::asymmetric::encryption::DHPublicKey;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::keys::KEMEncapsulationKeys;
use nym_network_requester::{
    CustomGatewayDetails, GatewayDetails, GatewayRegistration, set_active_gateway,
    setup_fs_gateways_storage, store_gateway_details,
};
use nym_node_requests::api::SignedData;
use nym_node_requests::api::v1::lewes_protocol::models::{LPHashFunction, LPKEM, LewesProtocol};
use nym_node_requests::api::v1::node::models::NodeRoles;
use nym_noise_keys::VersionedNoiseKeyV1;
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_addressing::Recipient;
use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::trace;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeDescription {
    /// moniker defines a human-readable name for the node.
    pub moniker: String,

    /// website defines an optional website link.
    pub website: String,

    /// security contact defines an optional email for security contact.
    pub security_contact: String,

    /// details define other optional details.
    pub details: String,
}

impl From<NodeDescription> for api_requests::v1::node::models::NodeDescription {
    fn from(description: NodeDescription) -> Self {
        api_requests::v1::node::models::NodeDescription {
            moniker: description.moniker,
            website: description.website,
            security_contact: description.security_contact,
            details: description.details,
        }
    }
}

impl From<NodeDescription> for nym_directory_types::NodeDescription {
    fn from(description: NodeDescription) -> Self {
        nym_directory_types::NodeDescription {
            moniker: description.moniker,
            website: description.website,
            security_contact: description.security_contact,
            details: description.details,
        }
    }
}

// all known information about this node
#[derive(Clone, Debug)]
pub(crate) struct NodeDetails {
    identity_key: ed25519::PublicKey,
    noise_key: x25519::PublicKey,
    cosmos_address: AccountId,

    /// Specifies whether this node operator has agreed to the terms and conditions
    /// as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0>
    accepted_operator_terms_and_conditions: bool,

    ip_addresses: Vec<IpAddr>,
    hostname: Option<String>,
    build_information: BinaryBuildInformationOwned,

    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    location: Option<Country>,
    description: NodeDescription,

    service_providers: ServiceProvidersKeys,
    wireguard_details: WireguardDetails,
    lewes_protocol_details: LewesProtocolDetails,

    modes: NodeModes,
    external_ports: ExternalPorts,
    system_info: api_requests::v1::node::models::HostSystem,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ExternalPorts {
    verloc_port: u16,
    mix_port: u16,
    ws_port: u16,

    // Node if this node does not support wss
    wss_port: Option<u16>,
}

impl ExternalPorts {
    fn to_websockets_http_api_model(self) -> api_requests::v1::gateway::models::WebSockets {
        api_requests::v1::gateway::models::WebSockets {
            ws_port: self.ws_port,
            wss_port: self.wss_port,
        }
    }

    fn to_announced_ports_http_api_model(self) -> api_requests::v2::node::models::AnnouncePorts {
        api_requests::v2::node::models::AnnouncePorts {
            verloc_port: Some(self.verloc_port),
            mix_port: Some(self.mix_port),
        }
    }

    fn to_directory_model(self) -> nym_directory_types::NodePorts {
        nym_directory_types::NodePorts {
            verloc_port: self.verloc_port as u32,
            mix_port: self.mix_port as u32,
            ws_port: self.ws_port as u32,
            wss_port: self.wss_port.map(|port| port as u32),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WireguardDetails {
    enabled: bool,
    tunnel_port: u16,
    metadata_port: u16,
    public_key: x25519::PublicKey,
}

impl WireguardDetails {
    fn to_http_api_model(self) -> Option<api_requests::v1::gateway::models::Wireguard> {
        if !self.enabled {
            return None;
        }

        #[allow(deprecated)]
        Some(api_requests::v1::gateway::models::Wireguard {
            port: self.tunnel_port,
            tunnel_port: self.tunnel_port,
            metadata_port: self.metadata_port,
            public_key: self.public_key.to_string(),
        })
    }

    fn to_directory_model(self) -> nym_directory_types::Wireguard {
        nym_directory_types::Wireguard {
            tunnel_port: self.tunnel_port as u32,
            metadata_port: self.metadata_port as u32,
            public_key: self.public_key.to_bytes().to_vec(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LewesProtocolDetails {
    x25519: DHPublicKey,

    kem_keys: KEMEncapsulationKeys,

    /// LP TCP control address (default: 41264) for establishing LP sessions
    control_port: u16,

    /// LP UDP data address (default: 51264) for Sphinx packets wrapped in LP
    data_port: u16,
}

impl LewesProtocolDetails {
    fn to_directory_model(&self) -> nym_directory_types::LewesProtocolDetails {
        // KEM type -> (hash function -> raw digest bytes), stringifying the enum keys.
        let kem_key_digests = self
            .kem_keys
            .digests()
            .into_iter()
            .map(|(kem, digests)| {
                let digests = digests
                    .into_iter()
                    .map(|(hash_function, digest)| (hash_function.to_string(), digest))
                    .collect();
                (
                    kem.to_string(),
                    nym_directory_types::KemKeyDigests { digests },
                )
            })
            .collect();

        nym_directory_types::LewesProtocolDetails {
            x25519_public_key: self.x25519.as_ref().to_vec(),
            kem_key_digests,
            control_port: self.control_port as u32,
            data_port: self.data_port as u32,
        }
    }

    pub(crate) fn compute_http_api_kem_key_hashes(
        &self,
    ) -> BTreeMap<LPKEM, BTreeMap<LPHashFunction, String>> {
        let digests = self.kem_keys.digests();

        // convert from `nym_kkt_ciphersuite` types into `nym_nodes_requests`
        digests
            .into_iter()
            .map(|(kem, kem_digests)| {
                (
                    kem.into(),
                    kem_digests
                        .into_iter()
                        .map(|(f, digest)| (f.into(), hex::encode(&digest)))
                        .collect(),
                )
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceProvidersKeys {
    // ideally we'd be storing all the keys here, but unfortunately due to how the service providers
    // are currently implemented, they will be loading the data themselves from the provided paths

    // those public keys are just convenience wrappers for http builder and details displayer
    nr_ed25519: ed25519::PublicKey,
    nr_x25519: x25519::PublicKey,

    ipr_ed25519: ed25519::PublicKey,
    ipr_x25519: x25519::PublicKey,

    // TODO: those should be moved to WG section
    auth_ed25519: ed25519::PublicKey,
    auth_x25519: x25519::PublicKey,
}

impl ServiceProvidersKeys {
    fn initialise_client_keys<R: RngCore + CryptoRng>(
        rng: &mut R,
        typ: &str,
        ed25519_paths: nym_pemstore::KeyPairPath,
        x25519_paths: nym_pemstore::KeyPairPath,
        ack_key_path: &Path,
    ) -> Result<(), ServiceProvidersError> {
        let ed25519_keys = ed25519::KeyPair::new(rng);
        let x25519_keys = x25519::KeyPair::new(rng);
        let aes128ctr_key = AckKey::new(rng);

        store_keypair(
            &ed25519_keys,
            &ed25519_paths,
            format!("{typ}-ed25519-identity"),
        )?;
        store_keypair(&x25519_keys, &x25519_paths, format!("{typ}-x25519-dh"))?;
        store_key(&aes128ctr_key, ack_key_path, format!("{typ}-ack-key"))?;

        Ok(())
    }

    pub(crate) async fn initialise_client_gateway_storage(
        storage_path: &Path,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        // insert all required information into the gateways store
        // (I hate that we have to do it, but that's currently the simplest thing to do)
        let storage = setup_fs_gateways_storage(storage_path).await?;
        store_gateway_details(&storage, registration).await?;
        set_active_gateway(&storage, &registration.gateway_id().to_base58_string()).await?;
        Ok(())
    }

    pub async fn initialise_network_requester<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ServiceProvidersConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        trace!("initialising network requester keys");
        Self::initialise_client_keys(
            rng,
            "network-requester",
            config
                .storage_paths
                .network_requester
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .network_requester
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.network_requester.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.network_requester.gateway_registrations,
            registration,
        )
        .await
    }

    pub async fn initialise_ip_packet_router_requester<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ServiceProvidersConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        trace!("initialising ip packet router keys");
        Self::initialise_client_keys(
            rng,
            "ip-packet-router",
            config
                .storage_paths
                .ip_packet_router
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .ip_packet_router
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.ip_packet_router.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.ip_packet_router.gateway_registrations,
            registration,
        )
        .await
    }

    pub async fn initialise_authenticator<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ServiceProvidersConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        trace!("initialising authenticator keys");
        Self::initialise_client_keys(
            rng,
            "authenticator",
            config
                .storage_paths
                .authenticator
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .authenticator
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.authenticator.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.authenticator.gateway_registrations,
            registration,
        )
        .await?;
        Ok(())
    }

    pub async fn initialise(
        config: &ServiceProvidersConfig,
        public_key: ed25519::PublicKey,
    ) -> Result<(), ServiceProvidersError> {
        // generate all the keys for NR, IPR and AUTH
        let mut rng = OsRng;

        let gateway_details = GatewayDetails::Custom(CustomGatewayDetails::new(public_key)).into();

        // NR:
        Self::initialise_network_requester(&mut rng, config, &gateway_details).await?;

        // IPR:
        Self::initialise_ip_packet_router_requester(&mut rng, config, &gateway_details).await?;

        // Authenticator
        Self::initialise_authenticator(&mut rng, config, &gateway_details).await?;

        Ok(())
    }

    pub(crate) fn load(
        config: &ServiceProvidersConfig,
    ) -> Result<ServiceProvidersKeys, ServiceProvidersError> {
        let nr_paths = &config.storage_paths.network_requester;
        let nr_ed25519 = load_key(
            &nr_paths.public_ed25519_identity_key_file,
            "network requester ed25519",
        )?;

        let nr_x25519 = load_key(
            &nr_paths.public_x25519_diffie_hellman_key_file,
            "network requester x25519",
        )?;

        let ipr_paths = &config.storage_paths.ip_packet_router;
        let ipr_ed25519 = load_key(
            &ipr_paths.public_ed25519_identity_key_file,
            "ip packet router ed25519",
        )?;

        let ipr_x25519 = load_key(
            &ipr_paths.public_x25519_diffie_hellman_key_file,
            "ip packet router x25519",
        )?;

        let auth_paths = &config.storage_paths.authenticator;
        let auth_ed25519 = load_key(
            &auth_paths.public_ed25519_identity_key_file,
            "authenticator ed25519",
        )?;

        let auth_x25519 = load_key(
            &auth_paths.public_x25519_diffie_hellman_key_file,
            "authenticator x25519",
        )?;

        Ok(ServiceProvidersKeys {
            nr_ed25519,
            nr_x25519,
            ipr_ed25519,
            ipr_x25519,
            auth_ed25519,
            auth_x25519,
        })
    }
}

impl NodeDetails {
    pub(crate) fn construct(
        config: &Config,
        accepted_operator_terms_and_conditions: bool,
        kem_keys: KEMEncapsulationKeys,
        lp_key: DHPublicKey,
        cosmos_address: AccountId,
    ) -> Result<Self, NymNodeError> {
        let description = load_node_description(&config.storage_paths.description)?;

        // we're unnecessarily loading the public keys for the second time,
        // but the cost is negligible and it keeps the function signature simpler
        // the only exception are the kem keys due to their size and lp key since we only store the private key
        let identity_key = load_ed25519_identity_public_key(
            &config.storage_paths.keys.public_ed25519_identity_key_file,
        )?;
        let noise_key =
            load_x25519_noise_public_key(&config.storage_paths.keys.public_x25519_noise_key_file)?;
        let wireguard_key = load_x25519_wireguard_public_key(
            &config
                .wireguard
                .storage_paths
                .public_diffie_hellman_key_file,
        )?;

        let lewes_protocol_details = LewesProtocolDetails {
            x25519: lp_key,
            kem_keys,
            control_port: config.lp.announced_control_port(),
            data_port: config.lp.announced_data_port(),
        };

        let wireguard_details = WireguardDetails {
            enabled: config.wireguard.enabled,
            tunnel_port: config.wireguard.announced_tunnel_port,
            metadata_port: config.wireguard.announced_metadata_port,
            public_key: wireguard_key,
        };

        let service_providers = ServiceProvidersKeys::load(&config.service_providers)?;

        let external_ports = ExternalPorts {
            verloc_port: config.verloc.external_port(),
            mix_port: config.mixnet.external_port(),
            ws_port: config.gateway_tasks.external_ws_port(),
            wss_port: config.gateway_tasks.announce_wss_port,
        };

        let system_info = get_system_info(
            config.http.expose_system_hardware,
            config.http.expose_crypto_hardware,
        );

        let modes = config.modes;

        Ok(NodeDetails {
            identity_key,
            noise_key,
            cosmos_address,
            accepted_operator_terms_and_conditions,
            ip_addresses: config.host.public_ips.clone(),
            hostname: config.host.hostname.clone(),
            build_information: bin_info_owned!(),
            system_info,
            description,
            modes,
            service_providers,
            wireguard_details,
            lewes_protocol_details,
            location: config.host.location,
            external_ports,
        })
    }

    #[must_use]
    pub(crate) fn fill_http_app_config(
        &self,
        node_config: &Config,
        http_config: HttpServerConfig,
    ) -> HttpServerConfig {
        // mixnode info
        let mixnode_details = api_requests::v1::mixnode::models::Mixnode {};

        // entry gateway info
        let gateway_details = api_requests::v1::gateway::models::Gateway {
            enforces_zk_nyms: node_config.gateway_tasks.enforce_zk_nyms,
            client_interfaces: api_requests::v1::gateway::models::ClientInterfaces {
                wireguard: self.wireguard_details.to_http_api_model(),
                mixnet_websockets: Some(self.external_ports.to_websockets_http_api_model()),
            },
        };

        // exit gateway info
        let nr_details = api_requests::v1::network_requester::models::NetworkRequester {
            encoded_identity_key: self.service_providers.nr_ed25519.to_base58_string(),
            encoded_x25519_key: self.service_providers.nr_x25519.to_base58_string(),
            address: self.exit_network_requester_address().to_string(),
        };

        let ipr_details = api_requests::v1::ip_packet_router::models::IpPacketRouter {
            encoded_identity_key: self.service_providers.ipr_ed25519.to_base58_string(),
            encoded_x25519_key: self.service_providers.ipr_x25519.to_base58_string(),
            address: self.exit_ip_packet_router_address().to_string(),
        };

        let auth_details = api_requests::v1::authenticator::models::Authenticator {
            encoded_identity_key: self.service_providers.auth_ed25519.to_base58_string(),
            encoded_x25519_key: self.service_providers.auth_x25519.to_base58_string(),
            address: self.exit_authenticator_address().to_string(),
        };

        http_config
            .with_mixnode_details(mixnode_details)
            .with_gateway_details(gateway_details)
            .with_network_requester_details(nr_details)
            .with_ip_packet_router_details(ipr_details)
            .with_authenticator_details(auth_details)
    }

    pub(crate) fn build_http_app_static_node_information(
        &self,
        signing_keys: Arc<ed25519::KeyPair>,
        node_config: &Config,
    ) -> StaticNodeInformation {
        let lewes_protocol = LewesProtocol {
            enabled: true,
            control_port: self.lewes_protocol_details.control_port,
            data_port: self.lewes_protocol_details.data_port,
            x25519: self.lewes_protocol_details.x25519,
            kem_keys: self
                .lewes_protocol_details
                .compute_http_api_kem_key_hashes(),
        };

        let x25519_versioned_noise_key = if node_config.mixnet.debug.unsafe_disable_noise {
            None
        } else {
            Some(VersionedNoiseKeyV1 {
                supported_version: nym_noise::LATEST_NOISE_VERSION,
                x25519_pubkey: self.noise_key,
            })
        };

        let system_info = if node_config.http.expose_system_info {
            Some(self.system_info.clone())
        } else {
            None
        };

        let auxiliary_data = api_requests::v2::node::models::AuxiliaryDetailsV2 {
            location: self.location,
            address: self.cosmos_address.to_string(),
            announce_ports: self.external_ports.to_announced_ports_http_api_model(),
            accepted_operator_terms_and_conditions: self.accepted_operator_terms_and_conditions,
        };

        // SAFETY: the only way for this call to fail is if serialisation of LewesProtocol fails.
        // however, that conversion is stable and infallible
        #[allow(clippy::unwrap_used)]
        let signed_lewes_protocol =
            SignedData::new(lewes_protocol, signing_keys.private_key()).unwrap();

        StaticNodeInformation {
            ed25519_identity_keys: signing_keys,
            x25519_versioned_noise_key,
            ip_addresses: self.ip_addresses.clone(),
            hostname: self.hostname.clone(),
            build_information: self.build_information.clone(),
            system_info,
            roles: NodeRoles {
                mixnode_enabled: self.modes.mixnode,
                gateway_enabled: self.modes.entry,
                network_requester_enabled: self.modes.exit,
                ip_packet_router_enabled: self.modes.exit,
            },
            description: self.description.clone().into(),
            auxiliary_data,
            lewes_protocol: signed_lewes_protocol,
        }
    }

    pub(crate) fn exit_network_requester_address(&self) -> Recipient {
        Recipient::new(
            self.service_providers.nr_ed25519,
            self.service_providers.nr_x25519,
            self.identity_key,
        )
    }

    pub(crate) fn exit_ip_packet_router_address(&self) -> Recipient {
        Recipient::new(
            self.service_providers.ipr_ed25519,
            self.service_providers.ipr_x25519,
            self.identity_key,
        )
    }

    pub(crate) fn exit_authenticator_address(&self) -> Recipient {
        Recipient::new(
            self.service_providers.auth_ed25519,
            self.service_providers.auth_x25519,
            self.identity_key,
        )
    }

    pub(crate) fn directory_mixnet_service_providers(
        &self,
    ) -> nym_directory_types::MixnetServiceProviders {
        nym_directory_types::MixnetServiceProviders {
            network_requester: Some(nym_directory_types::NetworkRequester {
                address: self.exit_network_requester_address().to_bytes().to_vec(),
            }),
            internet_packet_router: Some(nym_directory_types::InternetPacketRouter {
                address: self.exit_ip_packet_router_address().to_bytes().to_vec(),
            }),
            authenticator: Some(nym_directory_types::Authenticator {
                address: self.exit_authenticator_address().to_bytes().to_vec(),
            }),
        }
    }

    pub(crate) fn description(&self) -> &NodeDescription {
        &self.description
    }

    pub(crate) fn cosmos_address(&self) -> &AccountId {
        &self.cosmos_address
    }
}

// methods for converting into the directory publications
impl NodeDetails {
    pub(crate) fn directory_node_description(&self) -> nym_directory_types::NodeDescription {
        self.description.clone().into()
    }

    pub(crate) fn directory_wireguard(&self) -> nym_directory_types::Wireguard {
        self.wireguard_details.to_directory_model()
    }

    pub(crate) fn directory_lewes_protocol_details(
        &self,
    ) -> nym_directory_types::LewesProtocolDetails {
        self.lewes_protocol_details.to_directory_model()
    }

    pub(crate) fn directory_node_information(&self) -> nym_directory_types::NodeInformation {
        nym_directory_types::NodeInformation {
            binary_version: self.build_information.build_version.clone(),
            hostname: self.hostname.clone(),
            ip_addresses: self.ip_addresses.iter().map(|ip| ip.to_string()).collect(),
            cosmos_address: self.cosmos_address.to_string(),
            location: self
                .location
                .as_ref()
                .map(|country| country.alpha2.to_string()),
            ports: Some(self.external_ports.to_directory_model()),
            modes: Some(self.directory_node_modes()),
        }
    }

    fn directory_node_modes(&self) -> nym_directory_types::NodeModes {
        nym_directory_types::NodeModes {
            mixnode: self.modes.mixnode,
            entry: self.modes.entry,
            exit: self.modes.exit,
            wireguard_enabled: self.wireguard_details.enabled,
        }
    }
}

#[cfg(test)]
pub(crate) fn mock_node_details() -> NodeDetails {
    let mut rng09 = nym_test_utils::helpers::deterministic_rng_09();
    let mut rng = nym_test_utils::helpers::deterministic_rng();

    let identity = ed25519::KeyPair::new(&mut rng);
    let noise = x25519::KeyPair::new(&mut rng);
    let cosmos_address = AccountId::new("n", &[0u8; 32]).unwrap();
    let nr_ed25519 = ed25519::KeyPair::new(&mut rng);
    let nr_x25519 = x25519::KeyPair::new(&mut rng);
    let ipr_ed25519 = ed25519::KeyPair::new(&mut rng);
    let ipr_x25519 = x25519::KeyPair::new(&mut rng);
    let auth_ed25519 = ed25519::KeyPair::new(&mut rng);
    let auth_x25519 = x25519::KeyPair::new(&mut rng);
    let wireguard_key = x25519::KeyPair::new(&mut rng);

    let lp_key = nym_lp::peer::DHKeyPair::new(&mut rng09);
    let kem_keys = nym_kkt::keys::KEMKeys::new(
        nym_kkt::key_utils::generate_keypair_mceliece(&mut rng09),
        nym_kkt::key_utils::generate_keypair_mlkem(&mut rng09),
    );

    NodeDetails {
        identity_key: *identity.public_key(),
        noise_key: *noise.public_key(),
        cosmos_address,
        accepted_operator_terms_and_conditions: true,
        ip_addresses: vec!["1.1.1.1".parse().unwrap()],
        hostname: None,
        build_information: bin_info_owned!(),
        location: Some(celes::Country::switzerland()),
        description: NodeDescription {
            moniker: "mock_moniker".to_string(),
            website: "https://nymtech.net".to_string(),
            security_contact: "security@nymtech.net".to_string(),
            details: "mock_details".to_string(),
        },
        service_providers: ServiceProvidersKeys {
            nr_ed25519: *nr_ed25519.public_key(),
            nr_x25519: *nr_x25519.public_key(),
            ipr_ed25519: *ipr_ed25519.public_key(),
            ipr_x25519: *ipr_x25519.public_key(),
            auth_ed25519: *auth_ed25519.public_key(),
            auth_x25519: *auth_x25519.public_key(),
        },
        wireguard_details: WireguardDetails {
            enabled: true,
            tunnel_port: 10000,
            metadata_port: 20000,
            public_key: *wireguard_key.public_key(),
        },
        lewes_protocol_details: LewesProtocolDetails {
            x25519: lp_key.pk,
            kem_keys: kem_keys.encapsulation_keys(),
            control_port: 30000,
            data_port: 40000,
        },
        modes: Default::default(),
        external_ports: ExternalPorts {
            verloc_port: 1234,
            mix_port: 2345,
            ws_port: 5678,
            wss_port: None,
        },
        system_info: Default::default(),
    }
}
