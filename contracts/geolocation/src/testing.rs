// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// fine in test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::contract::{execute, instantiate, migrate, query};
use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
use cosmwasm_std::{Addr, Storage};
use mixnet_contract::testable_mixnet_contract::{
    EmbeddedMixnetContractExt, MixnetContract, MixnetContractSiblings,
};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractTester, ContractTesterBuilder, DenomExt,
    PermissionedFn, QueryFn, RandExt, TestableNymContract,
};
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::constants::{storage_keys, PAYLOAD_VERSION_1};
use nym_geolocation_contract_common::{
    AgentPermissions, ExecuteMsg, GeolocationContractError, GeolocationRecord, InstantiateMsg,
    LocationAttestation, LocationEntry, LocationPayload, Measurement, Method, MigrateMsg,
    NymNodeLocation, QueryMsg, Source, Subject,
};
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;

pub struct GeolocationContract;

impl TestableNymContract for GeolocationContract {
    const NAME: &'static str = "nym-geolocation-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = GeolocationContractError;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError> {
        instantiate
    }

    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError> {
        execute
    }

    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError> {
        query
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        migrate
    }

    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        let builder = ContractTesterBuilder::new().instantiate::<MixnetContract>(None);

        // we just instantiated it
        let mixnet_address = builder
            .well_known_contracts
            .get(MixnetContract::NAME)
            .unwrap()
            .clone();

        builder
            .instantiate::<Self>(Some(InstantiateMsg {
                mixnet_contract_address: mixnet_address.to_string(),
                initial_whitelist: vec![],
                max_skew_secs: None,
                max_batch_size: None,
                max_payload_size: None,
            }))
            .build()
    }
}

pub fn init_contract_tester() -> ContractTester<GeolocationContract> {
    let mut tester = GeolocationContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN);

    let geolocation_address = tester.contract_address.clone();
    tester
        .set_mixnet_sibling_contracts(
            MixnetContractSiblings::default()
                .with_clear_all()
                .with_geolocation_contract(geolocation_address),
        )
        .expect("should be able to patch mixnet contract state");

    tester
}

/// The only measured source there is: an ipinfo lookup performed by `agent`.
pub fn measured_by(agent: &Addr) -> Source {
    Source::Measured {
        method: Method::IpInfo,
        agent: agent.clone(),
    }
}

/// An entry with opaque `content` and no attestation, as a measurement or an override
/// produces. The content is never parsed by anything under test, so gibberish is the point.
pub fn location_entry(content: &[u8], checked_at: u64) -> LocationEntry {
    LocationEntry {
        payload: LocationPayload {
            version: PAYLOAD_VERSION_1,
            content: content.to_vec().into(),
        },
        checked_at,
        attestation: None,
    }
}

/// A self-declared entry, carrying the node's attestation. The signature is a placeholder:
/// nothing below the transaction layer verifies it, and the digest leaf commits it as bytes.
pub fn attested_location_entry(content: &[u8], checked_at: u64, declared_at: u64) -> LocationEntry {
    LocationEntry {
        attestation: Some(LocationAttestation {
            declared_at,
            signature: vec![7u8; 64].into(),
        }),
        ..location_entry(content, checked_at)
    }
}

/// One measurement of a node, as an agent would submit it. `content` is never parsed by the
/// contract, so gibberish is the point.
pub fn node_measurement(node_id: NodeId, content: &[u8]) -> Measurement {
    Measurement {
        subject: Subject::new_nym_node(node_id),
        method: Method::IpInfo,
        payload: LocationPayload {
            version: PAYLOAD_VERSION_1,
            content: content.to_vec().into(),
        },
    }
}

/// A self-declaration signed by the node's own identity key, so it verifies against the key in
/// that node's mixnet bond. Pair with `bond_dummy_nymnode_with_keypair`, which is the only way
/// to get hold of a bonded node's private key.
///
/// The signature covers the payload's own bytes, so a caller cannot accidentally sign one
/// serialisation and submit another.
pub fn signed_declaration(
    keypair: &ed25519::KeyPair,
    node_id: NodeId,
    declared_at: u64,
    content: &[u8],
) -> NymNodeLocation {
    let payload = LocationPayload {
        version: PAYLOAD_VERSION_1,
        content: content.to_vec().into(),
    };
    let signature = keypair
        .private_key()
        .sign(payload.self_declaration_signing_payload(node_id, declared_at));

    NymNodeLocation {
        node_id,
        declared_at,
        payload,
        signature: signature.to_bytes().to_vec().into(),
    }
}

pub trait GeolocationContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = GeolocationContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + BankExt
    + RandExt
    + Storage
    + ArbitraryContractStorageReader
    + ArbitraryContractStorageWriter
    + EmbeddedMixnetContractExt
    + Sized
{
    fn digest(&self) -> LtHash16 {
        GEOLOCATION_CONTRACT_STORAGE.load_digest(self).unwrap()
    }

    fn add_dummy_agent(&mut self) -> Addr {
        self.add_agent_with_permissions(AgentPermissions {
            can_measure: true,
            can_relay_self_declared: true,
        })
    }

    fn add_agent_with_permissions(&mut self, permissions: AgentPermissions) -> Addr {
        let addr = self.generate_account();
        GEOLOCATION_CONTRACT_STORAGE
            .set_whitelisted_agent(self, addr.clone(), permissions)
            .unwrap();
        addr
    }

    fn set_node_measurement(
        &mut self,
        node_id: NodeId,
        source: Source,
        content: &[u8],
        checked_at: u64,
    ) {
        self.set_node_entry(node_id, source, location_entry(content, checked_at))
    }

    /// Write one entry through the digest wrapper.
    fn set_node_entry(&mut self, node_id: NodeId, source: Source, entry: LocationEntry) {
        GEOLOCATION_CONTRACT_STORAGE
            .set_entry(self, Subject::new_nym_node(node_id), source, entry)
            .unwrap();
    }

    /// Write several entries under a single accumulator load and save, the way
    /// `SubmitMeasurements` will. Repeated keys within one batch are allowed and resolve to
    /// the last write.
    fn set_node_entry_batch(
        &mut self,
        entries: impl IntoIterator<Item = (NodeId, Source, LocationEntry)>,
    ) {
        GEOLOCATION_CONTRACT_STORAGE
            .set_entries(
                self,
                entries.into_iter().map(|(node_id, source, entry)| {
                    (Subject::new_nym_node(node_id), source, entry)
                }),
            )
            .unwrap();
    }

    fn set_node_measurement_content(&mut self, node_id: NodeId, content: &[u8]) {
        let agent = self.get_agent();
        self.set_node_measurement(node_id, measured_by(&agent), content, 1234)
    }

    fn set_dummy_measurement_from(&mut self, node_id: NodeId, agent: &Addr) {
        let content = format!("dummy-measurement-for-node-{node_id}-from-{agent}");
        self.set_node_measurement(node_id, measured_by(agent), content.as_bytes(), 1234)
    }

    fn set_dummy_node_measurement(&mut self, node_id: NodeId) {
        let content = format!("dummy-measurement-for-node-{node_id}");
        self.set_node_measurement_content(node_id, content.as_bytes());
    }

    /// Everything held for one node, across all sources, in ascending source order.
    fn node_entries(&self, node_id: NodeId) -> Vec<(Source, LocationEntry)> {
        GEOLOCATION_CONTRACT_STORAGE
            .subject_entries(self, &Subject::new_nym_node(node_id))
            .unwrap()
    }

    fn node_measurements(&self, node_id: NodeId) -> Vec<(Source, LocationEntry)> {
        self.node_entries(node_id)
            .into_iter()
            .filter(|(source, _)| source.is_measured())
            .collect()
    }

    fn node_heartbeat(&mut self, node_id: NodeId) {
        let (source, mut entry) = self.node_measurements(node_id).first().unwrap().clone();
        entry.checked_at += 1;
        GEOLOCATION_CONTRACT_STORAGE
            .set_entry(self, Subject::new_nym_node(node_id), source, entry)
            .unwrap();
    }

    fn update_dummy_node_measurement(&mut self, node_id: NodeId) {
        let (source, mut entry) = self.node_measurements(node_id).first().unwrap().clone();
        entry.checked_at += 1;
        let mut old_payload = entry.payload.content.to_vec();
        old_payload.push(42);
        entry.payload.content = old_payload.into();
        GEOLOCATION_CONTRACT_STORAGE
            .set_entry(self, Subject::new_nym_node(node_id), source, entry)
            .unwrap();
    }

    fn set_dummy_node_override(&mut self, node_id: NodeId) {
        let content = format!("dummy-node-override-for-{node_id}");
        self.set_node_measurement(node_id, Source::Override, content.as_bytes(), 1234);
    }

    fn set_dummy_node_self_declared(&mut self, node_id: NodeId) {
        let content = format!("dummy-node-self-declared-for-{node_id}");
        self.set_node_entry(
            node_id,
            Source::SelfDeclared,
            attested_location_entry(content.as_bytes(), 1234, 1000),
        )
    }

    /// A single entry as stored, or `None` if that source has written nothing for the node.
    fn node_entry(&self, node_id: NodeId, source: &Source) -> Option<LocationEntry> {
        GEOLOCATION_CONTRACT_STORAGE
            .may_load_entry(self, &Subject::new_nym_node(node_id), source)
            .unwrap()
    }

    /// What `agent` measured for this node, or `None` if it has written nothing for it. The
    /// single-entry counterpart of [`Self::node_measurements`].
    fn measurement_by(&self, node_id: NodeId, agent: &Addr) -> Option<LocationEntry> {
        self.node_entry(node_id, &measured_by(agent))
    }

    /// Fold every stored record from scratch and assert the maintained digest agrees.
    ///
    /// The assertion after any mutation: an accumulator that has drifted still hashes, still
    /// compares equal to itself, and is wrong in a way nothing else notices.
    fn assert_digest_is_refold(&self) {
        crate::storage::assert_digest_is_refold(self)
    }

    /// Every digest-committed record, across both entry classes - the set a verifying client
    /// folds to recompute the digest.
    fn all_records(&self) -> Vec<GeolocationRecord> {
        GEOLOCATION_CONTRACT_STORAGE.all_records(self).unwrap()
    }

    fn remove_node_entry(&mut self, node_id: NodeId, source: &Source) {
        GEOLOCATION_CONTRACT_STORAGE
            .remove_entry(self, &Subject::new_nym_node(node_id), source)
            .unwrap();
    }

    fn remove_all_node_entries(&mut self, node_id: NodeId) {
        GEOLOCATION_CONTRACT_STORAGE
            .remove_all_entries_for_subject(self, &Subject::new_nym_node(node_id))
            .unwrap();
    }

    fn remove_agent(&mut self, agent: &Addr) {
        GEOLOCATION_CONTRACT_STORAGE
            .remove_whitelisted_agent(self, agent)
            .unwrap();
    }

    fn get_agent(&self) -> Addr {
        GEOLOCATION_CONTRACT_STORAGE
            .all_whitelisted_agents(self)
            .unwrap()
            .first()
            .expect("no agents set")
            .agent
            .clone()
    }

    fn remove_all_locations(&mut self) {
        let entries = GEOLOCATION_CONTRACT_STORAGE
            .entries_paged(self, None, usize::MAX)
            .unwrap();
        for record in entries {
            GEOLOCATION_CONTRACT_STORAGE
                .remove_entry(self, &record.subject, &record.source)
                .unwrap();
        }
    }

    fn remove_all_agents(&mut self) {
        let agents = GEOLOCATION_CONTRACT_STORAGE
            .all_whitelisted_agents(self)
            .unwrap();
        for agent in agents {
            self.remove_agent(&agent.agent);
        }
    }
}

impl GeolocationContractTesterExt for ContractTester<GeolocationContract> {}
