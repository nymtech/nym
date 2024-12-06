// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use std::cmp::Ordering;

#[cw_serde]
pub struct HistoricalNymNodeVersion {
    /// Version of the nym node that is going to be used for determining the version score of a node.
    /// note: value stored here is pre-validated `semver::Version`
    pub semver: String,

    /// Block height of when this version has been added to the contract
    pub introduced_at_height: u64,
}

impl HistoricalNymNodeVersion {
    // SAFETY: the value stored in the contract is always valid
    // if you manually construct that struct with invalid value, it's on you.
    #[allow(clippy::unwrap_used)]
    pub fn semver_unchecked(&self) -> semver::Version {
        self.semver.parse().unwrap()
    }
}

#[cw_serde]
pub struct HistoricalNymNodeVersionEntry {
    /// The unique, ordered, id of this particular entry
    pub id: u32,

    /// Data associated with this particular version
    pub version_information: HistoricalNymNodeVersion,
}

impl From<(u32, HistoricalNymNodeVersion)> for HistoricalNymNodeVersionEntry {
    fn from((id, version_information): (u32, HistoricalNymNodeVersion)) -> Self {
        HistoricalNymNodeVersionEntry {
            id,
            version_information,
        }
    }
}

impl PartialOrd for HistoricalNymNodeVersionEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // we only care about id for the purposes of ordering as they should have unique data
        self.id.partial_cmp(&other.id)
    }
}

#[cw_serde]
pub struct NymNodeVersionHistoryResponse {
    pub history: Vec<HistoricalNymNodeVersionEntry>,
}

#[cw_serde]
pub struct CurrentNymNodeVersionResponse {
    pub version: Option<HistoricalNymNodeVersionEntry>,
}

#[cw_serde]
pub struct ConfigScoreParams {
    /// Defines weights for calculating numbers of versions behind the current release.
    pub version_weights: OutdatedVersionWeights,

    /// Defines the parameters of the formula for calculating the version score
    pub version_score_formula_params: VersionScoreFormulaParams,
}

impl ConfigScoreParams {
    // INVARIANT: release chain is sorted
    pub fn config_score(
        node_version: &semver::Version,
        release_chain: &[HistoricalNymNodeVersion],
    ) -> Decimal {
    }
}

/// Defines weights for calculating numbers of versions behind the current release.
#[cw_serde]
#[derive(Copy)]
pub struct OutdatedVersionWeights {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: u32,
}

impl OutdatedVersionWeights {
    pub fn versions_behind_factor(
        &self,
        version: &semver::Version,
        current: &semver::Version,
    ) -> u32 {
        let major_diff = (current.major as i64 - version.major as i64).unsigned_abs() as u32;
        let minor_diff = (current.minor as i64 - version.minor as i64).unsigned_abs() as u32;
        let patch_diff = (current.patch as i64 - version.patch as i64).unsigned_abs() as u32;
        let prerelease_diff = if current.pre == version.pre { 0 } else { 1 };

        // if there's a major increase, ignore minor and patch and treat it as 0
        if major_diff != 0 {
            return major_diff * self.major
                + current.minor as u32 * self.minor
                + current.patch as u32 * self.patch
                + prerelease_diff * self.prerelease;
        }

        // if there's a minor increase, ignore patch and treat is as 0
        if minor_diff != 0 {
            return minor_diff * self.minor
                + current.patch as u32 * self.patch
                + prerelease_diff * self.prerelease;
        }

        patch_diff * self.patch + prerelease_diff * self.prerelease
    }
}

impl Default for OutdatedVersionWeights {
    fn default() -> Self {
        OutdatedVersionWeights {
            major: 100,
            minor: 10,
            patch: 1,
            prerelease: 1,
        }
    }
}

/// Given the formula of version_score = penalty ^ (versions_behind_factor ^ penalty_scaling)
/// define the relevant parameters
#[cw_serde]
#[derive(Copy)]
pub struct VersionScoreFormulaParams {
    pub penalty: Decimal,
    pub penalty_scaling: Decimal,
}

impl Default for VersionScoreFormulaParams {
    fn default() -> Self {
        #[allow(clippy::unwrap_used)]
        VersionScoreFormulaParams {
            penalty: "0.995".parse().unwrap(),
            penalty_scaling: "1.65".parse().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_behind_factor() {
        // helper to compact the parsing
        fn s(raw: &str) -> semver::Version {
            s.parse().unwrap()
        }

        let weights = OutdatedVersionWeights::default();

        // current
        assert_eq!(0, weights.versions_behind_factor(&s("1.2.3"), &s("1.2.3")));
        assert_eq!(1, weights.versions_behind_factor(&s("1.2.2"), &s("1.2.3")));
        assert_eq!(2, weights.versions_behind_factor(&s("1.2.1"), &s("1.2.3")));
        assert_eq!(13, weights.versions_behind_factor(&s("1.1.3"), &s("1.2.3")));
        assert_eq!(13, weights.versions_behind_factor(&s("1.1.2"), &s("1.2.3")));
        assert_eq!(13, weights.versions_behind_factor(&s("1.1.0"), &s("1.2.3")));

        assert_eq!(10, weights.versions_behind_factor(&s("1.1.0"), &s("1.2.0")));
        assert_eq!(
            100,
            weights.versions_behind_factor(&s("1.1.0"), &s("2.0.0"))
        );
        assert_eq!(
            110,
            weights.versions_behind_factor(&s("1.1.0"), &s("2.1.0"))
        );
        assert_eq!(
            113,
            weights.versions_behind_factor(&s("1.1.0"), &s("2.1.3"))
        );
    }
}
