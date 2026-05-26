// Copyright 2023-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::load::CachedNodeLoad;
use crate::node::http::state::metrics::MetricsAppState;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_credential_verification::UpgradeModeState;
use nym_crypto::asymmetric::ed25519;
use nym_node_metrics::NymNodeMetrics;
use nym_node_requests::api::SignedLewesProtocol;
use nym_node_requests::api::v1::node::models::{HostSystem, NodeDescription, NodeRoles};
use nym_node_requests::api::v2::node::models::AuxiliaryDetailsV2;
use nym_noise_keys::VersionedNoiseKeyV1;
use nym_verloc::measurements::SharedVerlocStats;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use url::Url;

pub mod load;
pub mod metrics;

pub(crate) struct StaticNodeInformation {
    pub(crate) ed25519_identity_keys: Arc<ed25519::KeyPair>,
    pub(crate) x25519_versioned_noise_key: Option<VersionedNoiseKeyV1>,
    pub(crate) ip_addresses: Vec<IpAddr>,
    pub(crate) hostname: Option<String>,

    // TODO: move other fields here too
    pub(crate) build_information: BinaryBuildInformationOwned,
    pub(crate) system_info: Option<HostSystem>,
    pub(crate) roles: NodeRoles,
    pub(crate) description: NodeDescription,
    pub(crate) auxiliary_data: AuxiliaryDetailsV2,
    pub(crate) lewes_protocol: SignedLewesProtocol,
}

#[derive(Clone)]
pub(crate) struct UpgradeModeApiState {
    pub(crate) node_state: UpgradeModeState,
    pub(crate) attestation_url: Url,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) startup_time: Instant,

    pub(crate) static_information: Arc<StaticNodeInformation>,

    pub(crate) x25519_sphinx_keys: ActiveSphinxKeys,

    pub(crate) cached_load: CachedNodeLoad,

    pub(crate) metrics: MetricsAppState,

    pub(crate) upgrade_mode_state: UpgradeModeApiState,
}

impl AppState {
    pub(crate) fn new(
        static_information: StaticNodeInformation,
        x25519_sphinx_keys: ActiveSphinxKeys,
        metrics: NymNodeMetrics,
        verloc: SharedVerlocStats,
        upgrade_mode_attestation_url: Url,
        upgrade_mode_state: UpgradeModeState,
        load_cache_ttl: Duration,
    ) -> Self {
        AppState {
            static_information: Arc::new(static_information),
            x25519_sphinx_keys,

            // is it 100% accurate?
            // no.
            // does it have to be?
            // also no.
            startup_time: Instant::now(),
            cached_load: CachedNodeLoad::new(load_cache_ttl),
            metrics: MetricsAppState { metrics, verloc },
            upgrade_mode_state: UpgradeModeApiState {
                node_state: upgrade_mode_state,
                attestation_url: upgrade_mode_attestation_url,
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn dummy() -> Self {
        use crate::node::key_rotation::key::SphinxPrivateKey;
        use nym_crypto::asymmetric::x25519;
        use rand::rngs::OsRng;

        let mut rng = nym_test_utils::helpers::deterministic_rng();
        let ed25519_keys = ed25519::KeyPair::new(&mut rng);
        let x25519_pub: x25519::DHPublicKey = x25519::PrivateKey::new(&mut rng).public_key().into();
        let lp = nym_node_requests::api::v1::lewes_protocol::models::LewesProtocol::new(
            false,
            0,
            0,
            x25519_pub,
            std::collections::BTreeMap::new(),
        );
        let signed =
            nym_node_requests::api::SignedData::new(lp, ed25519_keys.private_key()).unwrap();

        let attester_pk = *ed25519_keys.public_key();
        let static_information = StaticNodeInformation {
            ed25519_identity_keys: Arc::new(ed25519_keys),
            x25519_versioned_noise_key: None,
            ip_addresses: vec![],
            hostname: None,
            build_information: nym_bin_common::bin_info_owned!(),
            system_info: None,
            roles: Default::default(),
            description: Default::default(),
            auxiliary_data: Default::default(),
            lewes_protocol: signed,
        };
        let active_sphinx = ActiveSphinxKeys::new_fresh(SphinxPrivateKey::new(&mut OsRng, 0));

        AppState::new(
            static_information,
            active_sphinx,
            NymNodeMetrics::new(),
            SharedVerlocStats::default(),
            Url::parse("https://attestation.test").unwrap(),
            UpgradeModeState::new(attester_pk),
            Duration::from_secs(60),
        )
    }
}
