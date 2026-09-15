// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Choosing which of a subject's entries to believe.
//!
//! The contract deliberately stores opinions rather than a verdict: several agents may have
//! measured a subject, the subject may have declared its own location, and an admin may have
//! overridden both. Verification establishes that all of them are genuine; it says nothing
//! about which one a consumer should act on. That is a policy question, and different
//! consumers answer it differently - a sweep cadence and a dVPN country filter have unrelated
//! tolerances - so it is a replaceable seam rather than a fixed rule.
//!
//! A policy **selects**, never synthesizes. The result is a reference to one stored entry with
//! its provenance intact, so a consumer can always say which agent, at which height, asserted
//! the value it acted on. A caller wanting a value derived across entries already holds the
//! whole verified set and does not need the seam.

use crate::verified::{
    DecodedLocation, MeasuredEntry, OverrideEntry, SelfDeclaredEntry, SubjectEntries,
    VerifiedGeolocation,
};
use nym_geolocation_contract_common::LocationPayload;
use nym_mixnet_contract_common::NodeId;
use std::collections::BTreeMap;
use time::OffsetDateTime;

/// The entry a policy chose, and which slot it came from.
///
/// Provenance is the variant rather than a separate field, so it cannot drift from the entry
/// it describes, and each variant keeps the evidence specific to its slot - a measurement's
/// authority, a declaration's attestation - rather than flattening them into a common shape
/// that would have to discard one or the other.
#[derive(Debug, PartialEq)]
pub enum ResolvedEntry<'a> {
    /// An admin-set value, authorised by the admin role.
    Override(&'a OverrideEntry),

    /// A third-party measurement, authorised by the agent whitelist.
    Measured(&'a MeasuredEntry),

    /// The subject's own declaration, authorised by its signature.
    SelfDeclared(&'a SelfDeclaredEntry),
}

impl<'a> ResolvedEntry<'a> {
    /// The payload exactly as committed, whichever slot won.
    pub fn location(&self) -> &'a LocationPayload {
        match self {
            ResolvedEntry::Override(entry) => &entry.location,
            ResolvedEntry::Measured(entry) => &entry.location,
            ResolvedEntry::SelfDeclared(entry) => &entry.location,
        }
    }

    /// When the block that wrote this entry was produced.
    pub fn checked_at(&self) -> OffsetDateTime {
        match self {
            ResolvedEntry::Override(entry) => entry.checked_at,
            ResolvedEntry::Measured(entry) => entry.checked_at,
            ResolvedEntry::SelfDeclared(entry) => entry.checked_at,
        }
    }

    /// This build's reading of the winning entry's payload.
    pub fn decoded(&self) -> &'a DecodedLocation {
        match self {
            ResolvedEntry::Override(entry) => &entry.decoded,
            ResolvedEntry::Measured(entry) => &entry.decoded,
            ResolvedEntry::SelfDeclared(entry) => &entry.decoded,
        }
    }
}

/// Applying a policy to a verified set.
///
/// Deliberately here rather than on the client: a [`ResolvedEntry`] borrows the set it came
/// from, so a client method returning one would have to hand back the set as well, or clone
/// the entry and give up the guarantee that the answer *is* a stored record rather than a
/// copy. Resolution is also a per-consumer question, and the client has no business holding
/// one consumer's answer to it.
impl VerifiedGeolocation {
    /// The entry `policy` would act on for `node_id`.
    ///
    /// `None` covers three different situations - no entries for the subject, entries the
    /// policy declined, and entries this build cannot read - which [`Self::get_subject`]
    /// distinguishes for a caller that needs to.
    pub fn resolve(
        &self,
        node_id: NodeId,
        policy: &impl ResolutionPolicy,
    ) -> Option<ResolvedEntry<'_>> {
        policy.resolve(self.get_subject(node_id)?)
    }

    /// Every subject the policy could answer for.
    ///
    /// Subjects it declined are absent rather than present-with-nothing: the map is the set of
    /// answers, and a caller wanting to know why a subject is missing has the full set.
    pub fn resolve_all(
        &self,
        policy: &impl ResolutionPolicy,
    ) -> BTreeMap<NodeId, ResolvedEntry<'_>> {
        self.subjects
            .iter()
            .filter_map(|(node_id, entries)| Some((*node_id, policy.resolve(entries)?)))
            .collect()
    }
}

/// How to pick one entry out of a subject's slots.
///
/// A pure function of the entries. No clock: nothing in the default depends on the current
/// time, and a policy that wants a freshness bound can take its reference instant at
/// construction, where it stays testable, rather than every caller passing one that almost
/// nobody reads.
///
/// `None` means the policy found nothing it was willing to act on - which is different from
/// the subject having no entries, and both are different from a failed verification. A caller
/// that wants to know why still holds the full [`SubjectEntries`].
pub trait ResolutionPolicy {
    fn resolve<'a>(&self, entries: &'a SubjectEntries) -> Option<ResolvedEntry<'a>>;
}

/// The default: an admin override, else the country the most measurements agree on, else a
/// verified self-declaration - skipping any slot this build cannot actually read.
///
/// The unobvious half of the precedence is measured outranking self-declared, and it is the
/// point of the system: a self-declaration is a subject asserting about itself and is
/// unverifiable by a third party, so it belongs as a fallback for subjects nothing has
/// measured rather than as an answer that outranks a measurement.
///
/// # Agreement before freshness
///
/// Among measurements, the country the most of them agree on wins, and only then is the
/// freshest of *those* returned. One recent outlier does not overturn ten older measurements
/// that agree with each other: a lone disagreeing result is more likely a bad lookup than a
/// relocated node, and a node that really has moved accumulates agreeing measurements soon
/// enough.
///
/// Agreement is judged on the two-letter country code alone, never the whole location.
/// Coordinates, city and org differ between providers for a node that has not moved anywhere,
/// so comparing those would find disagreement everywhere and the tally would never mean
/// anything.
///
/// # An unreadable entry cannot win its slot
///
/// Selection needs a location, so an entry this build cannot decode cannot answer and
/// precedence falls through to the next slot that can - with one exception, below. This is
/// the only place the payload version affects anything beyond display, and it is why a
/// partially-decodable measurement set tallies only what it can read: ten version 2 entries
/// agreeing on AT are invisible to a build that reads only version 1, so it would answer from
/// whatever version 1 entries remain. The entries are all still present in
/// [`SubjectEntries`], each carrying its own `decoded`, so that situation is inspectable
/// rather than silent.
///
/// The exception is an undecodable **override**. An override exists to suppress the other
/// slots, so falling through would serve exactly the value an admin acted to suppress. That
/// is worse than answering nothing, so it resolves to `None`.
///
/// # Nothing expires
///
/// An entry that is in the contract is something to fall back on, and stale data beats no
/// data: ageing a measurement out would drop a subject to a weaker source, or to nothing at
/// all, because no agent has swept it recently - a fact about the sweep rather than about the
/// subject. Every entry carries `checked_at`, so a consumer that does need a freshness bound
/// applies its own through a replacement policy instead of one being imposed here.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultResolutionPolicy;

impl ResolutionPolicy for DefaultResolutionPolicy {
    fn resolve<'a>(&self, entries: &'a SubjectEntries) -> Option<ResolvedEntry<'a>> {
        if let Some(entry) = entries.overridden.as_ref() {
            // an override this build cannot read suppresses everything and answers nothing,
            // rather than falling through to the value it was set to override
            return entry
                .decoded
                .location()
                .map(|_| ResolvedEntry::Override(entry));
        }

        if let Some(entry) = select_measured(&entries.measured) {
            return Some(ResolvedEntry::Measured(entry));
        }

        // A self-declaration's only authority is the subject's signature, so one that does not
        // verify has nothing behind it at all - unusable rather than merely weak.
        entries
            .self_declared
            .as_ref()
            .filter(|entry| {
                entry
                    .attestation
                    .as_ref()
                    .is_some_and(|attestation| attestation.status.is_verified())
                    && entry.decoded.location().is_some()
            })
            .map(ResolvedEntry::SelfDeclared)
    }
}

/// The measurement to believe: the freshest one from the country the most measurements agree
/// on, falling back to the freshest readable measurement when no country can be tallied.
///
/// Authority is deliberately not an eligibility test. The contract enforced the whitelist at
/// write time, so a de-authorised agent's measurement was legitimate when it was made, and
/// removing the agent governs future writes rather than past ones. Each entry carries its
/// `authority`, so a stricter consumer can filter.
fn select_measured(measured: &[MeasuredEntry]) -> Option<&MeasuredEntry> {
    let readable = || {
        measured
            .iter()
            .filter(|entry| entry.decoded.location().is_some())
    };

    // count per country, remembering the freshest entry seen for each. A `BTreeMap` so that a
    // tie between two countries resolves by country code rather than by hash order.
    let mut by_country: BTreeMap<String, (usize, &MeasuredEntry)> = BTreeMap::new();
    for entry in readable() {
        let Some(location) = entry.decoded.location() else {
            continue;
        };
        let code = location.two_letter_iso_country_code.trim();
        // an unknown country is not a country: it must not win a tally against real ones
        if code.is_empty() {
            continue;
        }

        by_country
            .entry(code.to_ascii_uppercase())
            .and_modify(|(count, best)| {
                *count += 1;
                if entry.checked_at > best.checked_at {
                    *best = entry;
                }
            })
            .or_insert((1, entry));
    }

    // most agreeing measurements wins; a tie goes to whichever group holds the fresher entry
    let mut winner: Option<(usize, &MeasuredEntry)> = None;
    for (count, entry) in by_country.into_values() {
        let better = match winner {
            None => true,
            Some((best_count, best_entry)) => {
                count > best_count
                    || (count == best_count && entry.checked_at > best_entry.checked_at)
            }
        };
        if better {
            winner = Some((count, entry));
        }
    }

    // nothing carried a country, but something may still be readable and worth returning
    winner
        .map(|(_, entry)| entry)
        .or_else(|| freshest(readable()))
}

/// The freshest entry, keeping the first among equals.
///
/// Records arrive in the contract's key order, `(method, agent)`, so equal timestamps resolve
/// to the lowest key rather than to whichever page happened to land last. Written out rather
/// than using `max_by_key`, which keeps the *last* maximum and would make the winner depend on
/// enumeration order.
fn freshest<'a>(mut entries: impl Iterator<Item = &'a MeasuredEntry>) -> Option<&'a MeasuredEntry> {
    let mut best = entries.next()?;
    for entry in entries {
        if entry.checked_at > best.checked_at {
            best = entry;
        }
    }
    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_records::{
        country_payload, measured_in, measured_with_version, overridden, self_declared_signed,
        signing_keypair, whitelisted,
    };
    use crate::verified::VerifiedGeolocation;
    use cosmwasm_std::Addr;
    use nym_crypto::asymmetric::ed25519;
    use nym_geolocation_contract_common::GeolocationRecord;
    use nym_mixnet_contract_common::NodeId;
    use nym_validator_client::nyxd::Height;
    use std::collections::BTreeMap;

    const SUBJECT: NodeId = 1;

    fn entries_for(records: Vec<GeolocationRecord>) -> SubjectEntries {
        entries_with_identities(records, &BTreeMap::new())
    }

    fn entries_with_identities(
        records: Vec<GeolocationRecord>,
        identities: &BTreeMap<NodeId, ed25519::PublicKey>,
    ) -> SubjectEntries {
        VerifiedGeolocation::from_verified_records(Height::from(100u32), records, identities)
            .get_subject(SUBJECT)
            .cloned()
            .unwrap_or_default()
    }

    fn resolve(entries: &SubjectEntries) -> Option<ResolvedEntry<'_>> {
        DefaultResolutionPolicy.resolve(entries)
    }

    fn country_of(resolved: &ResolvedEntry<'_>) -> String {
        resolved
            .decoded()
            .location()
            .expect("the default only selects readable entries")
            .two_letter_iso_country_code
            .clone()
    }

    // --- precedence ---

    #[test]
    fn an_override_outranks_everything() {
        let kp = signing_keypair(1);
        let entries = entries_with_identities(
            vec![
                overridden(SUBJECT, 1_700_000_000, &country_payload("CH")),
                measured_in(SUBJECT, "agent-one", 1_700_009_999, "AT"),
                self_declared_signed(SUBJECT, &kp, 1_700_009_999, &country_payload("DE")),
            ],
            &BTreeMap::from([(SUBJECT, *kp.public_key())]),
        );

        let resolved = resolve(&entries).expect("an override always answers");
        assert!(matches!(resolved, ResolvedEntry::Override(..)));
        assert_eq!(country_of(&resolved), "CH");
    }

    #[test]
    fn a_measurement_outranks_a_self_declaration() {
        let kp = signing_keypair(1);
        let entries = entries_with_identities(
            vec![
                measured_in(SUBJECT, "agent-one", 1_700_000_000, "AT"),
                self_declared_signed(SUBJECT, &kp, 1_700_009_999, &country_payload("DE")),
            ],
            &BTreeMap::from([(SUBJECT, *kp.public_key())]),
        );

        // even though the declaration is far fresher: a subject asserting about itself is
        // unverifiable by a third party, so it is a fallback rather than a better answer
        let resolved = resolve(&entries).expect("a measurement answers");
        assert!(matches!(resolved, ResolvedEntry::Measured(..)));
        assert_eq!(country_of(&resolved), "AT");
    }

    #[test]
    fn a_verified_self_declaration_answers_when_nothing_measured_the_subject() {
        let kp = signing_keypair(1);
        let entries = entries_with_identities(
            vec![self_declared_signed(
                SUBJECT,
                &kp,
                1_700_000_000,
                &country_payload("DE"),
            )],
            &BTreeMap::from([(SUBJECT, *kp.public_key())]),
        );

        let resolved = resolve(&entries).expect("a verified declaration answers");
        assert!(matches!(resolved, ResolvedEntry::SelfDeclared(..)));
        assert_eq!(country_of(&resolved), "DE");
    }

    /// An unverifiable declaration has nothing behind it: a self-declaration's only authority
    /// is the signature.
    #[test]
    fn an_unverifiable_self_declaration_answers_nothing() {
        let kp = signing_keypair(1);
        // no identity supplied, so the attestation cannot be checked
        let entries = entries_for(vec![self_declared_signed(
            SUBJECT,
            &kp,
            1_700_000_000,
            &country_payload("DE"),
        )]);

        assert!(resolve(&entries).is_none());
    }

    #[test]
    fn a_subject_with_no_entries_answers_nothing() {
        assert!(resolve(&SubjectEntries::default()).is_none());
    }

    // --- agreement among measurements ---

    /// The case that motivates tallying at all: one fresh outlier against many older
    /// measurements that agree with each other.
    #[test]
    fn many_agreeing_measurements_outweigh_one_fresher_outlier() {
        let mut records = vec![measured_in(SUBJECT, "agent-outlier", 1_700_009_999, "CH")];
        for n in 0..10 {
            records.push(measured_in(
                SUBJECT,
                &format!("agent-{n:02}"),
                1_700_000_000 + n,
                "AT",
            ));
        }

        let entries = entries_for(records);
        let resolved = resolve(&entries).expect("a measurement answers");

        assert_eq!(country_of(&resolved), "AT");
        // and within the winning country it is the freshest of those, not just any of them
        assert_eq!(resolved.checked_at().unix_timestamp(), 1_700_000_009);
    }

    #[test]
    fn the_freshest_of_the_agreeing_measurements_is_returned() {
        let entries = entries_for(vec![
            measured_in(SUBJECT, "agent-a", 1_700_000_000, "AT"),
            measured_in(SUBJECT, "agent-b", 1_700_000_500, "AT"),
            measured_in(SUBJECT, "agent-c", 1_700_000_200, "AT"),
        ]);

        let resolved = resolve(&entries).expect("a measurement answers");
        assert_eq!(resolved.checked_at().unix_timestamp(), 1_700_000_500);
    }

    /// With no plurality, the group holding the fresher measurement wins - deterministic
    /// rather than dependent on enumeration order.
    #[test]
    fn a_tied_tally_goes_to_the_group_with_the_fresher_measurement() {
        let entries = entries_for(vec![
            measured_in(SUBJECT, "agent-a", 1_700_000_000, "AT"),
            measured_in(SUBJECT, "agent-b", 1_700_000_900, "CH"),
        ]);

        let resolved = resolve(&entries).expect("a measurement answers");
        assert_eq!(country_of(&resolved), "CH");
    }

    /// Country codes should be uppercase, but a producer emitting lowercase must not split
    /// the vote and hand the result to a genuine minority.
    #[test]
    fn country_codes_are_tallied_case_insensitively() {
        let entries = entries_for(vec![
            measured_in(SUBJECT, "agent-a", 1_700_000_000, "at"),
            measured_in(SUBJECT, "agent-b", 1_700_000_100, "AT"),
            measured_in(SUBJECT, "agent-c", 1_700_000_900, "CH"),
        ]);

        let resolved = resolve(&entries).expect("a measurement answers");
        assert_eq!(country_of(&resolved).to_ascii_uppercase(), "AT");
    }

    /// An unknown country is not a country. Two entries with no country must not out-vote one
    /// that actually determined a location.
    #[test]
    fn entries_with_no_country_do_not_win_the_tally() {
        let entries = entries_for(vec![
            measured_in(SUBJECT, "agent-a", 1_700_000_800, ""),
            measured_in(SUBJECT, "agent-b", 1_700_000_900, ""),
            measured_in(SUBJECT, "agent-c", 1_700_000_000, "AT"),
        ]);

        let resolved = resolve(&entries).expect("a measurement answers");
        assert_eq!(country_of(&resolved), "AT");
    }

    /// Authority is not an eligibility test: the contract enforced the whitelist at write
    /// time, so a since-removed agent's measurement still counts.
    #[test]
    fn a_de_authorised_agents_measurement_still_counts() {
        let entries = entries_for(vec![
            whitelisted("agent-current"),
            measured_in(SUBJECT, "agent-removed", 1_700_000_000, "AT"),
            measured_in(SUBJECT, "agent-removed-too", 1_700_000_100, "AT"),
            measured_in(SUBJECT, "agent-current", 1_700_000_900, "CH"),
        ]);

        let resolved = resolve(&entries).expect("a measurement answers");
        assert_eq!(country_of(&resolved), "AT");
    }

    // --- readability gates selection ---

    /// A build that cannot read a payload cannot answer from it, so precedence falls through.
    #[test]
    fn an_unreadable_measurement_falls_through_to_a_self_declaration() {
        let kp = signing_keypair(1);
        let entries = entries_with_identities(
            vec![
                measured_with_version(SUBJECT, "agent-one", 1_700_009_999, 2, b"v2-bytes"),
                self_declared_signed(SUBJECT, &kp, 1_700_000_000, &country_payload("DE")),
            ],
            &BTreeMap::from([(SUBJECT, *kp.public_key())]),
        );

        let resolved = resolve(&entries).expect("the readable declaration answers");
        assert!(matches!(resolved, ResolvedEntry::SelfDeclared(..)));
        assert_eq!(country_of(&resolved), "DE");
    }

    /// The consequence of the tally worth knowing about: a build reads only what it
    /// understands, so version 2 entries are invisible to its vote even though they are
    /// present in the set.
    #[test]
    fn an_unreadable_measurement_does_not_vote_but_remains_in_the_set() {
        let entries = entries_for(vec![
            measured_with_version(SUBJECT, "agent-a", 1_700_000_000, 2, b"v2-bytes"),
            measured_with_version(SUBJECT, "agent-b", 1_700_000_100, 2, b"v2-bytes"),
            measured_in(SUBJECT, "agent-c", 1_700_000_200, "CH"),
        ]);

        // all three are present; only the readable one can be answered from
        assert_eq!(entries.measured.len(), 3);
        let resolved = resolve(&entries).expect("the readable measurement answers");
        assert_eq!(country_of(&resolved), "CH");
    }

    /// An override exists to suppress the other slots. If it cannot be read, serving the value
    /// it was set to override is worse than answering nothing.
    #[test]
    fn an_unreadable_override_suppresses_rather_than_falls_through() {
        let entries = entries_for(vec![
            overridden(SUBJECT, 1_700_000_000, b"v1-but-not-json"),
            measured_in(SUBJECT, "agent-one", 1_700_000_900, "AT"),
        ]);

        assert!(resolve(&entries).is_none());
    }

    #[test]
    fn measurements_with_no_readable_payload_at_all_answer_nothing() {
        let entries = entries_for(vec![measured_with_version(
            SUBJECT,
            "agent-a",
            1_700_000_000,
            2,
            b"v2-bytes",
        )]);

        assert!(resolve(&entries).is_none());
    }

    // --- the seam is replaceable ---

    /// The whole point of the seam: a consumer that disagrees with the default replaces it
    /// rather than working around it.
    #[test]
    fn a_caller_supplied_policy_replaces_the_default() {
        /// Prefers the subject's own word, inverting the default's precedence.
        struct TrustTheSubject;

        impl ResolutionPolicy for TrustTheSubject {
            fn resolve<'a>(&self, entries: &'a SubjectEntries) -> Option<ResolvedEntry<'a>> {
                entries
                    .self_declared
                    .as_ref()
                    .map(ResolvedEntry::SelfDeclared)
                    .or_else(|| entries.measured.first().map(ResolvedEntry::Measured))
            }
        }

        let kp = signing_keypair(1);
        let entries = entries_with_identities(
            vec![
                measured_in(SUBJECT, "agent-one", 1_700_009_999, "AT"),
                self_declared_signed(SUBJECT, &kp, 1_700_000_000, &country_payload("DE")),
            ],
            &BTreeMap::from([(SUBJECT, *kp.public_key())]),
        );

        assert_eq!(country_of(&resolve(&entries).unwrap()), "AT");
        assert_eq!(
            country_of(&TrustTheSubject.resolve(&entries).unwrap()),
            "DE"
        );
    }

    // --- applying a policy across a verified set ---

    fn verified(records: Vec<GeolocationRecord>) -> VerifiedGeolocation {
        VerifiedGeolocation::from_verified_records(Height::from(100u32), records, &BTreeMap::new())
    }

    #[test]
    fn a_single_subject_resolves_through_the_verified_set() {
        let verified = verified(vec![
            measured_in(SUBJECT, "agent-a", 1_700_000_000, "AT"),
            measured_in(SUBJECT, "agent-b", 1_700_000_500, "AT"),
        ]);

        let resolved = verified
            .resolve(SUBJECT, &DefaultResolutionPolicy)
            .expect("the subject resolves");
        assert_eq!(country_of(&resolved), "AT");

        // a subject the contract holds nothing for is absent, not an error
        assert!(verified.resolve(999, &DefaultResolutionPolicy).is_none());
    }

    #[test]
    fn resolve_all_answers_for_every_subject_it_can() {
        let verified = verified(vec![
            measured_in(1, "agent-a", 1_700_000_000, "AT"),
            measured_in(2, "agent-a", 1_700_000_000, "CH"),
            // this subject has only a payload this build cannot read
            measured_with_version(3, "agent-a", 1_700_000_000, 2, b"v2-bytes"),
        ]);

        let resolved = verified.resolve_all(&DefaultResolutionPolicy);

        assert_eq!(resolved.len(), 2);
        assert_eq!(country_of(&resolved[&1]), "AT");
        assert_eq!(country_of(&resolved[&2]), "CH");
        // declined subjects are absent from the answers, but still in the set
        assert!(!resolved.contains_key(&3));
        assert!(verified.get_subject(3).is_some());
    }

    /// Whichever slot wins, the result is a reference to one stored entry - never a value
    /// assembled from several.
    #[test]
    fn the_resolved_entry_is_one_of_the_stored_entries() {
        let entries = entries_for(vec![
            measured_in(SUBJECT, "agent-a", 1_700_000_000, "AT"),
            measured_in(SUBJECT, "agent-b", 1_700_000_500, "AT"),
        ]);

        let resolved = resolve(&entries).expect("a measurement answers");
        let ResolvedEntry::Measured(entry) = resolved else {
            panic!("expected a measurement");
        };

        assert!(
            entries
                .measured
                .iter()
                .any(|stored| std::ptr::eq(stored, entry)),
            "the policy must select a stored entry, never synthesize one"
        );
        assert_eq!(entry.agent, Addr::unchecked("agent-b"));
    }
}
