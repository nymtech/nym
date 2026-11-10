// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The agent whitelist, read out of the same record set the accumulator was recomputed over.
//!
//! Measured entries carry no signature, so the natural question is what authorises them. The
//! answer is that the whitelist is its own entry class in the *same* accumulator, so one
//! successful recompute authenticates the records and the authorisation set together. A
//! client cannot be shown a fabricated whitelist alongside genuine records, which is what
//! closes the whitelist-addition forgery risk: forgery means adding an agent, and adding one
//! changes the digest.

use cosmwasm_std::Addr;
use nym_geolocation_contract_common::{AgentPermissions, GeolocationRecord, Source};
use std::collections::BTreeMap;

/// Whether the agent that wrote a measured entry was still authorised to measure at the
/// verified height.
///
/// The contract enforces the whitelist at write time, so a measured entry that exists at all
/// was authorised when it was written. [`Self::DeAuthorised`] therefore never means "forged";
/// it means the agent was removed, or had `can_measure` withdrawn, at some point after the
/// write. Which of those a consumer wants to honour is a policy question rather than a
/// verification one, so such entries are reported rather than dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementAuthority {
    /// The agent is in the whitelist at the verified height and may measure.
    Authorised,

    /// The agent is absent from the whitelist at the verified height, or is present without
    /// `can_measure`. The entry is still genuine and still committed to the digest.
    DeAuthorised,
}

impl MeasurementAuthority {
    pub fn is_authorised(&self) -> bool {
        matches!(self, MeasurementAuthority::Authorised)
    }
}

/// The agent whitelist exactly as committed at the verified height.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifiedWhitelist {
    agents: BTreeMap<Addr, AgentPermissions>,
}

impl VerifiedWhitelist {
    /// Collect the whitelist entries out of a record set.
    ///
    /// The set MUST already have passed
    /// [`verify_records_against_digest`](crate::verify::verify_records_against_digest). This
    /// reads the authorisation set out of records whose completeness the accumulator has
    /// established; run on an unverified set it authorises nothing, because the caller has no
    /// reason to believe an agent was not simply omitted.
    pub fn from_verified_records(records: &[GeolocationRecord]) -> Self {
        let agents = records
            .iter()
            .filter_map(|record| match record {
                GeolocationRecord::WhitelistedAgent(entry) => {
                    Some((entry.agent.clone(), entry.permissions))
                }
                GeolocationRecord::Location(..) => None,
            })
            .collect();

        VerifiedWhitelist { agents }
    }

    /// The permissions `agent` holds at the verified height, or `None` if it is not
    /// whitelisted at all.
    pub fn get_permissions(&self, agent: &Addr) -> Option<&AgentPermissions> {
        self.agents.get(agent)
    }

    /// Resolve a measured entry's writing agent against the whitelist.
    pub fn measurement_authority(&self, agent: &Addr) -> MeasurementAuthority {
        match self.get_permissions(agent) {
            Some(permissions) if permissions.can_measure => MeasurementAuthority::Authorised,
            _ => MeasurementAuthority::DeAuthorised,
        }
    }

    /// Resolve `source` against the whitelist, or `None` for sources that name no agent.
    ///
    /// Self-declared and admin-override entries are authorised by other means (a subject's
    /// own signature, and the admin role respectively), so the whitelist has nothing to say
    /// about them - which is different from saying they are de-authorised.
    pub fn source_authority(&self, source: &Source) -> Option<MeasurementAuthority> {
        measuring_agent(source).map(|agent| self.measurement_authority(agent))
    }

    pub fn agents(&self) -> impl Iterator<Item = (&Addr, &AgentPermissions)> {
        self.agents.iter()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

/// The measuring agent a source names, or `None` for sources that carry no writer component.
pub fn measuring_agent(source: &Source) -> Option<&Addr> {
    match source {
        Source::Measured { agent, .. } => Some(agent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_records::{record_set, whitelisted_with};
    use nym_geolocation_contract_common::Method;

    fn agent(name: &str) -> Addr {
        Addr::unchecked(name)
    }

    fn measured_source(name: &str) -> Source {
        Source::Measured {
            method: Method::IpInfo,
            agent: agent(name),
        }
    }

    #[test]
    fn the_whitelist_is_read_out_of_the_record_set() {
        let whitelist = VerifiedWhitelist::from_verified_records(&record_set());

        assert_eq!(whitelist.len(), 1);
        assert!(whitelist.get_permissions(&agent("agent-one")).is_some());
        // location records must not leak into the authorisation set
        assert!(whitelist.get_permissions(&agent("agent-two")).is_none());
    }

    #[test]
    fn a_whitelisted_agent_with_can_measure_is_authorised() {
        let whitelist = VerifiedWhitelist::from_verified_records(&record_set());

        assert_eq!(
            whitelist.measurement_authority(&agent("agent-one")),
            MeasurementAuthority::Authorised
        );
    }

    /// The de-authorisation case the design insists is reported, not dropped: the contract
    /// enforced the whitelist at write time, so this state can only arise from a later
    /// removal - never from a forged entry.
    #[test]
    fn an_agent_absent_from_the_whitelist_is_de_authorised() {
        let whitelist = VerifiedWhitelist::from_verified_records(&record_set());

        assert_eq!(
            whitelist.measurement_authority(&agent("agent-removed")),
            MeasurementAuthority::DeAuthorised
        );
    }

    /// Narrowing an agent's permissions is the same class of event as removing it: the
    /// existing measurement stays genuine, but the agent may no longer measure.
    #[test]
    fn a_whitelisted_agent_without_can_measure_is_de_authorised() {
        let records = vec![whitelisted_with("agent-relay", false, true)];
        let whitelist = VerifiedWhitelist::from_verified_records(&records);

        assert!(whitelist.get_permissions(&agent("agent-relay")).is_some());
        assert_eq!(
            whitelist.measurement_authority(&agent("agent-relay")),
            MeasurementAuthority::DeAuthorised
        );
    }

    #[test]
    fn a_measured_source_resolves_against_the_whitelist() {
        let whitelist = VerifiedWhitelist::from_verified_records(&record_set());

        assert_eq!(
            whitelist.source_authority(&measured_source("agent-one")),
            Some(MeasurementAuthority::Authorised)
        );
        assert_eq!(
            whitelist.source_authority(&measured_source("agent-removed")),
            Some(MeasurementAuthority::DeAuthorised)
        );
    }

    /// A self-declared entry is authorised by the subject's own signature, so the whitelist
    /// has nothing to say about it. Reporting it as de-authorised would be wrong.
    #[test]
    fn sources_that_name_no_agent_have_no_whitelist_authority() {
        let whitelist = VerifiedWhitelist::from_verified_records(&record_set());

        assert_eq!(whitelist.source_authority(&Source::SelfDeclared), None);
        assert_eq!(measuring_agent(&Source::SelfDeclared), None);
    }

    /// Every measured entry in a verified set resolves to some authority - the resolution is
    /// total, so no entry can be silently skipped for want of a verdict.
    #[test]
    fn every_measured_entry_in_the_set_resolves() {
        let records = record_set();
        let whitelist = VerifiedWhitelist::from_verified_records(&records);

        let measured: Vec<_> = records
            .iter()
            .filter_map(|record| match record {
                GeolocationRecord::Location(location) => {
                    whitelist.source_authority(&location.source)
                }
                GeolocationRecord::WhitelistedAgent(..) => None,
            })
            .collect();

        assert_eq!(measured.len(), 2);
        assert!(measured.iter().all(|a| a.is_authorised()));
    }
}
