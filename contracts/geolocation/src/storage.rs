// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::{Addr, DepsMut, Order, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_geolocation_contract_common::constants::storage_keys;
use nym_geolocation_contract_common::{
    AgentPermissions, ContractConfig, EntryKey, GeolocationContractError, GeolocationRecord,
    InitialAgent, LocationEntry, Source, Subject, SubjectClass, WhitelistEntry,
};
use nym_lthash::LtHash16;

pub const GEOLOCATION_CONTRACT_STORAGE: GeolocationStorage = GeolocationStorage::new();

/// The entries store's key: `(subject_class_tag, subject_id, source)`.
///
/// A stock `cw-storage-plus` tuple rather than a custom `PrimaryKey`, which turns out to be
/// all this layout needs. `Source` is not a key-component type, so it travels as the opaque
/// bytes [`Source::to_key_bytes`] produces.
///
/// Ordering rests on subject ids being fixed-width within a class (see
/// [`SubjectClass::id_len`]): `cw-storage-plus` length-prefixes every component but the last,
/// so a variable-width id would sort by length before content and node 10 would precede node
/// 9. The trailing source component is *not* prefixed, so it sorts lexicographically, giving
/// measurements (by method, then agent), then the self-declared slot, then the override.
type EntryStorageKey = (u8, Vec<u8>, Vec<u8>);

/// The prefix covering one subject's contiguous range: everything but the source.
type SubjectPrefix = (u8, Vec<u8>);

fn entry_storage_key(subject: &Subject, source: &Source) -> EntryStorageKey {
    (
        subject.class().tag(),
        subject.id_bytes(),
        source.to_key_bytes(),
    )
}

fn subject_prefix(subject: &Subject) -> SubjectPrefix {
    (subject.class().tag(), subject.id_bytes())
}

pub struct GeolocationStorage {
    /// Admin of the contract; gates privileged operations.
    pub(crate) contract_admin: Admin,

    /// Address of the mixnet contract; used to verify a node id refers to a
    /// real, registered, and bonded node.
    pub(crate) mixnet_contract_address: Item<Addr>,

    /// Runtime configuration set at instantiation.
    pub(crate) config: Item<ContractConfig>,

    /// Location entries, keyed `(subject_class, subject_id, source)`.
    ///
    /// Values are ordinary JSON, unlike the directory contract's compact byte codec, so a
    /// plain `Map` suffices and none of its manual `Path`/`Prefix` handling is needed here.
    entries: Map<EntryStorageKey, LocationEntry>,

    /// The agent whitelist. Digest-committed, because measured entries carry no signature of
    /// their own: a client that could not verify which writers were authorised would have no
    /// way to reject entries laundered through a fabricated whitelist.
    whitelist: Map<Addr, AgentPermissions>,
    // The LtHash digest accumulator (~2 KB) is NOT a `cw-storage-plus` `Item`: serde cannot
    // (de)serialize a `[u8; DIGEST_LEN]` (it only derives arrays up to len 32), and
    // base64-encoding it on every write would be wasteful. It is stored raw under
    // `storage_keys::DIGEST_STATE` via `load_digest` / `save_digest` below.
}

impl GeolocationStorage {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> GeolocationStorage {
        GeolocationStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT_ADDRESS),
            config: Item::new(storage_keys::CONFIG),
            entries: Map::new(storage_keys::ENTRIES),
            whitelist: Map::new(storage_keys::WHITELIST),
        }
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        admin: Addr,
        mixnet_contract_address: Addr,
        initial_whitelist: Vec<InitialAgent>,
        config: ContractConfig,
    ) -> Result<(), GeolocationContractError> {
        // set the mixnet contract address
        self.mixnet_contract_address
            .save(deps.storage, &mixnet_contract_address)?;

        // set the config, having rejected any value that would make the contract inert
        config.validate()?;
        self.config.save(deps.storage, &config)?;

        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

        // add all initial agents
        for agent in initial_whitelist {
            let agent_address = deps.api.addr_validate(&agent.agent)?;
            self.set_whitelisted_agent(deps.storage, agent_address, agent.permissions)?;
        }

        Ok(())
    }

    // ---- entry reads ----

    /// Load a single entry; `None` if that source has written nothing for the subject.
    pub(crate) fn may_load_entry(
        &self,
        store: &dyn Storage,
        subject: &Subject,
        source: &Source,
    ) -> Result<Option<LocationEntry>, GeolocationContractError> {
        Ok(self
            .entries
            .may_load(store, entry_storage_key(subject, source))?)
    }

    #[cfg(any(test, feature = "testable-geolocation-contract"))]
    pub(crate) fn all_entries(
        &self,
        storage: &dyn Storage,
    ) -> Result<Vec<(EntryStorageKey, LocationEntry)>, GeolocationContractError> {
        Ok(self
            .entries
            .range(storage, None, None, Order::Ascending)
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Everything held for one subject, in ascending source order.
    ///
    /// Unpaginated and collected eagerly. A subject holds at most one entry per whitelisted
    /// agent plus a self-declaration plus an override, so the set is small; collecting also
    /// releases the store's immutable borrow, which the bulk deletes below need.
    pub(crate) fn subject_entries(
        &self,
        store: &dyn Storage,
        subject: &Subject,
    ) -> Result<Vec<(Source, LocationEntry)>, GeolocationContractError> {
        self.entries
            .prefix(subject_prefix(subject))
            .range(store, None, None, Order::Ascending)
            .map(|record| {
                let (source, entry) = record?;
                Ok((Source::try_from_key_bytes(&source)?, entry))
            })
            .collect()
    }

    /// Only the measured entries for one subject.
    ///
    /// Filtered in memory rather than by a prefix scan: `source` is a single opaque key
    /// component, so a prefix cannot reach inside it to select the `Measured` tag. Splitting
    /// it into two components to make this a true prefix would buy a range scan over a handful
    /// of items.
    pub(crate) fn subject_measurements(
        &self,
        store: &dyn Storage,
        subject: &Subject,
    ) -> Result<Vec<(Source, LocationEntry)>, GeolocationContractError> {
        Ok(self
            .subject_entries(store, subject)?
            .into_iter()
            .filter(|(source, _)| matches!(source, Source::Measured { .. }))
            .collect())
    }

    // ---- whitelist reads ----

    /// The agent's permissions, or `None` if it is not whitelisted.
    pub(crate) fn may_load_agent_permissions(
        &self,
        store: &dyn Storage,
        agent: &Addr,
    ) -> Result<Option<AgentPermissions>, GeolocationContractError> {
        Ok(self.whitelist.may_load(store, agent.clone())?)
    }

    /// The permissions currently held by `agent`, rejecting a sender that is not whitelisted.
    pub(crate) fn must_load_agent_permissions(
        &self,
        store: &dyn Storage,
        agent: &Addr,
    ) -> Result<AgentPermissions, GeolocationContractError> {
        self.may_load_agent_permissions(store, agent)?
            .ok_or_else(|| GeolocationContractError::NotWhitelisted {
                agent: agent.clone(),
            })
    }

    /// The whole whitelist, in ascending address order. Unpaginated: the set is small and
    /// NYM-controlled.
    pub(crate) fn all_whitelisted_agents(
        &self,
        store: &dyn Storage,
    ) -> Result<Vec<WhitelistEntry>, GeolocationContractError> {
        self.whitelist
            .range(store, None, None, Order::Ascending)
            .map(|record| {
                let (agent, permissions) = record?;
                Ok(WhitelistEntry { agent, permissions })
            })
            .collect()
    }

    // ---- global enumeration ----

    /// Every digest-committed record, across both entry classes.
    ///
    /// This is the definition of what the accumulator is supposed to equal: fold each of
    /// these leaves into an empty digest and the result must be the stored one. It is also
    /// what a client pulls and folds to verify completeness for itself, which is why the
    /// whitelist is in here rather than being treated as configuration.
    ///
    /// Unbounded, and so an internal helper rather than a query handler; the paged query
    /// takes a cursor over the same records.
    pub(crate) fn all_records(
        &self,
        store: &dyn Storage,
    ) -> Result<Vec<GeolocationRecord>, GeolocationContractError> {
        let locations = self
            .entries
            .range(store, None, None, Order::Ascending)
            .map(|record| {
                let (key, entry) = record?;
                let (subject, source) = parse_raw_key(key)?;
                Ok(GeolocationRecord::new_location(subject, source, entry))
            });

        let agents = self
            .whitelist
            .range(store, None, None, Order::Ascending)
            .map(|record| {
                let (agent, permissions) = record?;
                Ok(GeolocationRecord::new_whitelisted_agent(agent, permissions))
            });

        locations.chain(agents).collect()
    }

    // ---- digest accumulator (raw `DIGEST_STATE` key) ----

    /// Load the global LtHash accumulator, or the empty digest if nothing has been
    /// written yet.
    pub(crate) fn load_digest(
        &self,
        store: &dyn Storage,
    ) -> Result<LtHash16, GeolocationContractError> {
        match store.get(storage_keys::DIGEST_STATE.as_bytes()) {
            Some(bytes) => {
                let raw: &[u8; nym_lthash::DIGEST_LEN] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| GeolocationContractError::CorruptDigestState)?;
                Ok(LtHash16::from_bytes(raw))
            }
            None => Ok(LtHash16::new()),
        }
    }

    fn save_digest(&self, store: &mut dyn Storage, digest: &LtHash16) {
        store.set(storage_keys::DIGEST_STATE.as_bytes(), &digest.to_bytes());
    }

    // ---- digest-maintaining mutations ----
    //
    // Every write and delete keeps the accumulator in sync, so the digest always equals the
    // LtHash over the current record set. Two rules make that true and both are easy to
    // violate silently:
    //
    //   - a replacement must subtract the *exact* leaf of the value being replaced, which
    //     means reading the current value first rather than reconstructing what it should
    //     have been. Subtracting anything else corrupts the accumulator irrecoverably, since
    //     there is no way to tell afterwards what was subtracted;
    //   - the leaf is always `GeolocationRecord::digest_leaf`, which commits the key as well
    //     as the value and is independent of how either is stored.
    //
    // The `fold_*` helpers take the accumulator by reference so a batch pays one load and one
    // save no matter how many entries it touches, while each entry keeps its own
    // read-modify-write. LtHash is commutative, so batch ordering does not affect the result
    // and no canonical sort is needed (or wanted).

    fn fold_set_entry(
        &self,
        store: &mut dyn Storage,
        digest: &mut LtHash16,
        subject: Subject,
        source: Source,
        entry: LocationEntry,
    ) -> Result<(), GeolocationContractError> {
        let key = entry_storage_key(&subject, &source);

        // replacing an existing entry: retire its old leaf first. This is also what keeps a
        // key repeated within one batch correct, rather than double-counting it
        if let Some(old) = self.entries.may_load(store, key.clone())? {
            digest.subtract(
                &GeolocationRecord::new_location(subject.clone(), source.clone(), old)
                    .digest_leaf(),
            );
        }

        self.entries.save(store, key, &entry)?;
        digest.add(&GeolocationRecord::new_location(subject, source, entry).digest_leaf());
        Ok(())
    }

    /// Create or replace many entries under a single accumulator load and save.
    pub(crate) fn set_entries(
        &self,
        store: &mut dyn Storage,
        entries: impl IntoIterator<Item = (Subject, Source, LocationEntry)>,
    ) -> Result<(), GeolocationContractError> {
        let mut digest = self.load_digest(store)?;
        for (subject, source, entry) in entries {
            self.fold_set_entry(store, &mut digest, subject, source, entry)?;
        }
        self.save_digest(store, &digest);
        Ok(())
    }

    /// Create or replace a single entry, keeping the digest in sync.
    pub(crate) fn set_entry(
        &self,
        store: &mut dyn Storage,
        subject: Subject,
        source: Source,
        entry: LocationEntry,
    ) -> Result<(), GeolocationContractError> {
        self.set_entries(store, [(subject, source, entry)])
    }

    /// Retire one entry's exact stored leaf and delete it, reporting whether anything was
    /// there to remove.
    fn fold_remove_entry(
        &self,
        store: &mut dyn Storage,
        digest: &mut LtHash16,
        subject: &Subject,
        source: &Source,
    ) -> Result<bool, GeolocationContractError> {
        let key = entry_storage_key(subject, source);
        let Some(old) = self.entries.may_load(store, key.clone())? else {
            return Ok(false);
        };

        digest.subtract(
            &GeolocationRecord::new_location(subject.clone(), source.clone(), old).digest_leaf(),
        );
        self.entries.remove(store, key);
        Ok(true)
    }

    /// Delete many entries under a single accumulator load and save. Idempotent per key:
    /// naming an entry that is not there removes nothing and leaves the digest untouched,
    /// including the stored bytes, since a batch that removed nothing never saves.
    pub(crate) fn remove_entries(
        &self,
        store: &mut dyn Storage,
        keys: impl IntoIterator<Item = EntryKey>,
    ) -> Result<(), GeolocationContractError> {
        let mut digest = self.load_digest(store)?;
        let mut removed_any = false;
        for key in keys {
            if self.fold_remove_entry(store, &mut digest, &key.subject, &key.source)? {
                removed_any = true;
            }
        }

        if removed_any {
            self.save_digest(store, &digest);
        }
        Ok(())
    }

    /// Delete a single entry, keeping the digest in sync. Idempotent: removing an absent
    /// entry leaves the digest untouched.
    pub(crate) fn remove_entry(
        &self,
        store: &mut dyn Storage,
        subject: &Subject,
        source: &Source,
    ) -> Result<(), GeolocationContractError> {
        self.remove_entries(store, [EntryKey::new(subject.clone(), source.clone())])
    }

    /// Delete every entry held for one subject, across all sources, in a single digest
    /// update. Backs the mixnet unbond callback. Idempotent.
    pub(crate) fn remove_all_entries_for_subject(
        &self,
        store: &mut dyn Storage,
        subject: &Subject,
    ) -> Result<usize, GeolocationContractError> {
        // collect first: the scan borrows the store immutably and we then mutate it
        let existing = self.subject_entries(store, subject)?;
        if existing.is_empty() {
            return Ok(0);
        }

        let removed = existing.len();
        let mut digest = self.load_digest(store)?;
        for (source, entry) in existing {
            let key = entry_storage_key(subject, &source);
            digest.subtract(
                &GeolocationRecord::new_location(subject.clone(), source, entry).digest_leaf(),
            );
            self.entries.remove(store, key);
        }
        self.save_digest(store, &digest);
        Ok(removed)
    }

    /// Add an agent to the whitelist, or replace an existing agent's permissions, keeping
    /// the digest in sync.
    pub(crate) fn set_whitelisted_agent(
        &self,
        store: &mut dyn Storage,
        agent: Addr,
        permissions: AgentPermissions,
    ) -> Result<(), GeolocationContractError> {
        let mut digest = self.load_digest(store)?;

        if let Some(old) = self.whitelist.may_load(store, agent.clone())? {
            digest.subtract(
                &GeolocationRecord::new_whitelisted_agent(agent.clone(), old).digest_leaf(),
            );
        }

        self.whitelist.save(store, agent.clone(), &permissions)?;
        digest.add(&GeolocationRecord::new_whitelisted_agent(agent, permissions).digest_leaf());
        self.save_digest(store, &digest);
        Ok(())
    }

    /// Remove an agent from the whitelist, keeping the digest in sync. Idempotent.
    ///
    /// Deliberately leaves the agent's entries in place: authorisation is evaluated at read
    /// time against the current whitelist, so a conforming client stops honouring them the
    /// moment this lands. Purging them afterwards is hygiene, not the security control.
    pub(crate) fn remove_whitelisted_agent(
        &self,
        store: &mut dyn Storage,
        agent: &Addr,
    ) -> Result<(), GeolocationContractError> {
        let Some(old) = self.whitelist.may_load(store, agent.clone())? else {
            return Ok(());
        };

        let mut digest = self.load_digest(store)?;
        digest
            .subtract(&GeolocationRecord::new_whitelisted_agent(agent.clone(), old).digest_leaf());
        self.whitelist.remove(store, agent.clone());
        self.save_digest(store, &digest);
        Ok(())
    }
}

/// Fold every stored record into an empty accumulator - what a verifying client does with the
/// paged enumeration - and assert the maintained digest agrees.
///
/// Lives here rather than in the test module below because the instantiate tests need it too:
/// initial whitelisting is a digest-committed write like any other.
#[cfg(test)]
pub(crate) fn assert_digest_is_refold(store: &dyn Storage) {
    let mut refolded = LtHash16::new();
    #[allow(clippy::unwrap_used)]
    for record in GEOLOCATION_CONTRACT_STORAGE.all_records(store).unwrap() {
        refolded.add(&record.digest_leaf());
    }

    #[allow(clippy::unwrap_used)]
    let maintained = GEOLOCATION_CONTRACT_STORAGE.load_digest(store).unwrap();
    assert_eq!(maintained, refolded,);
}

// The entries store's key: `(subject_class_tag, subject_id, source)`.
pub(crate) fn parse_raw_key(
    key: EntryStorageKey,
) -> Result<(Subject, Source), GeolocationContractError> {
    let (subject_class_tag, subject_id, source) = key;
    let subject_class = SubjectClass::try_from_tag(subject_class_tag)?;
    let subject = Subject::try_from_id_bytes(subject_class, &subject_id)?;
    let source = Source::try_from_key_bytes(&source)?;
    Ok((subject, source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        attested_location_entry, init_contract_tester, location_entry, measured_by,
        GeolocationContractTesterExt,
    };
    use nym_contracts_common_testing::RandExt;
    use nym_geolocation_contract_common::RecordKey;

    // The digest is the contract's only claim about its own completeness, and every way of
    // breaking it is silent: a leaf that is added but never subtracted, or subtracted with
    // bytes other than the ones stored, leaves an accumulator that still hashes, still
    // serves, and simply no longer describes the state. Nothing in the contract notices.
    // These tests are therefore all one assertion in different clothing - that the maintained
    // digest equals a from-scratch fold of whatever is currently stored.

    #[test]
    fn the_digest_tracks_inserts_updates_and_deletes() {
        let mut test = init_contract_tester();

        let agent1 = test.add_dummy_agent();
        assert_digest_is_refold(&test);

        let agent2 = test.add_dummy_agent();
        assert_digest_is_refold(&test);

        test.set_dummy_measurement_from(1, &agent1);
        assert_digest_is_refold(&test);

        test.set_dummy_measurement_from(1, &agent2);
        assert_digest_is_refold(&test);

        test.set_dummy_node_self_declared(1);
        assert_digest_is_refold(&test);

        test.set_dummy_node_override(1);
        assert_digest_is_refold(&test);

        // replacing a value: the old leaf has to be retired with the exact bytes that were
        // stored, so this is the step a reconstructed-rather-than-loaded old value fails
        test.update_dummy_node_measurement(1);
        assert_digest_is_refold(&test);

        // a heartbeat: unchanged location, later `checked_at`. It has to move the digest,
        // otherwise verifying the digest would say nothing about freshness
        let before_heartbeat = test.digest();
        test.node_heartbeat(1);
        let after_heartbeat = test.digest();
        assert_ne!(before_heartbeat, after_heartbeat);
    }

    #[test]
    fn deleting_every_record_returns_the_digest_to_the_identity() {
        let mut test = init_contract_tester();

        let agent1 = test.add_dummy_agent();
        let agent2 = test.add_dummy_agent();
        test.set_dummy_measurement_from(1, &agent1);
        test.set_dummy_measurement_from(1, &agent2);
        test.set_dummy_node_self_declared(1);

        test.set_dummy_node_override(1);

        test.remove_all_agents();
        test.remove_all_locations();

        assert_eq!(test.digest(), LtHash16::new());
    }

    #[test]
    fn a_key_repeated_within_one_batch_is_not_double_counted() {
        let mut test = init_contract_tester();
        let agent = test.add_dummy_agent();
        let source = measured_by(&agent);

        let last = location_entry(b"second", 200);
        test.set_node_entry_batch([
            (1, source.clone(), location_entry(b"first", 100)),
            (1, source.clone(), last.clone()),
        ]);

        // last write wins, and the superseded leaf was retired within the batch rather than
        // left summed into the accumulator alongside it
        assert_eq!(test.node_entry(1, &source), Some(last));
        assert_digest_is_refold(&test);
    }

    #[test]
    fn batch_order_does_not_affect_the_digest() {
        let mut forwards = init_contract_tester();
        let mut backwards = init_contract_tester();

        // one address shared by both, since the two stores have to produce identical leaves
        let agent = forwards.generate_account();
        let records = [
            (1, measured_by(&agent), location_entry(b"a", 100)),
            (
                2,
                Source::SelfDeclared,
                attested_location_entry(b"b", 100, 90),
            ),
            (3, Source::Override, location_entry(b"c", 100)),
        ];

        forwards.set_node_entry_batch(records.clone());

        let mut reversed = records;
        reversed.reverse();
        backwards.set_node_entry_batch(reversed);

        // LtHash is commutative, so agents need no canonical ordering and two agents
        // submitting overlapping batches in different orders converge. Recorded as a test so
        // it is not later "fixed" by imposing a sort
        assert_eq!(forwards.digest(), backwards.digest());
    }

    #[test]
    fn removing_an_absent_record_leaves_the_digest_untouched() {
        let mut test = init_contract_tester();
        let agent = test.add_dummy_agent();
        test.set_dummy_measurement_from(1, &agent);
        let before = test.digest();

        let stranger = test.generate_account();
        test.remove_node_entry(1, &measured_by(&stranger));
        test.remove_node_entry(9, &measured_by(&agent));
        test.remove_all_node_entries(9);
        test.remove_agent(&stranger);

        assert_eq!(test.digest(), before);
        assert_digest_is_refold(&test);
    }

    #[test]
    fn the_enumeration_recovers_the_typed_key_from_storage_bytes() {
        let mut test = init_contract_tester();

        // the re-fold is only meaningful if the key decodes back to what was written: a
        // decoder that lost the subject or the source would still produce leaves, and they
        // would still be self-consistent, so nothing above would catch it
        let agent = test.add_dummy_agent();
        test.set_dummy_measurement_from(9, &agent);
        test.set_dummy_node_self_declared(10);
        test.set_dummy_node_override(10);

        assert_eq!(
            test.all_records()
                .iter()
                .map(|record| record.key())
                .collect::<Vec<_>>(),
            vec![
                // node 9 before node 10: big-endian ids order numerically, where a decimal
                // string would not
                RecordKey::Location(EntryKey::new(Subject::new_nym_node(9), measured_by(&agent))),
                RecordKey::Location(EntryKey::new(
                    Subject::new_nym_node(10),
                    Source::SelfDeclared
                )),
                RecordKey::Location(EntryKey::new(Subject::new_nym_node(10), Source::Override)),
                RecordKey::WhitelistedAgent { agent },
            ]
        );
    }
}
