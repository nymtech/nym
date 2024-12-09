// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use std::cmp::Ordering;
use std::ops::{Add, Sub};

#[cw_serde]
pub struct HistoricalNymNodeVersion {
    /// Version of the nym node that is going to be used for determining the version score of a node.
    /// note: value stored here is pre-validated `semver::Version`
    pub semver: String,

    /// Block height of when this version has been added to the contract
    pub introduced_at_height: u64,

    /// The absolute version difference as compared against the first version introduced into the contract.
    pub difference_since_genesis: TotalVersionDifference,
}

impl HistoricalNymNodeVersion {
    pub fn genesis(semver: String, height: u64) -> HistoricalNymNodeVersion {
        HistoricalNymNodeVersion {
            semver,
            introduced_at_height: height,
            difference_since_genesis: Default::default(),
        }
    }

    // SAFETY: the value stored in the contract is always valid
    // if you manually construct that struct with invalid value, it's on you.
    #[allow(clippy::unwrap_used)]
    pub fn semver_unchecked(&self) -> semver::Version {
        self.semver.parse().unwrap()
    }

    /// Return [`TotalVersionDifference`] for a new release version that is going to be pushed right after this one
    /// this function cannot be called against 2 arbitrary versions
    #[inline]
    pub fn difference_against_new_current(
        &self,
        new_version: &semver::Version,
    ) -> TotalVersionDifference {
        let self_semver = self.semver_unchecked();
        let mut new_absolute = self.difference_since_genesis;
        if new_version.major > self_semver.major {
            new_absolute.major += 1
        } else if new_version.minor > self_semver.minor {
            new_absolute.minor += 1
        } else if new_version.patch > self_semver.patch {
            new_absolute.patch += 1
        } else if new_version.pre != self_semver.pre {
            new_absolute.prerelease += 1
        }
        new_absolute
    }

    pub fn relative_difference(&self, other: &Self) -> TotalVersionDifference {
        if self.difference_since_genesis > other.difference_since_genesis {
            self.difference_since_genesis - other.difference_since_genesis
        } else {
            other.difference_since_genesis - self.difference_since_genesis
        }
    }

    pub fn difference_against_legacy(
        &self,
        legacy_version: &semver::Version,
    ) -> TotalVersionDifference {
        let current = self.semver_unchecked();
        let major_diff = (current.major as i64 - legacy_version.major as i64).unsigned_abs() as u32;
        let minor_diff = (current.minor as i64 - legacy_version.minor as i64).unsigned_abs() as u32;
        let patch_diff = (current.patch as i64 - legacy_version.patch as i64).unsigned_abs() as u32;
        let prerelease_diff = if current.pre == legacy_version.pre {
            0
        } else {
            1
        };

        let mut diff = TotalVersionDifference::default();
        // if there's a major increase, ignore minor and patch and treat it as 0
        if major_diff != 0 {
            diff.major += major_diff;
            return diff;
        }

        // if there's a minor increase, ignore patch and treat is as 0
        if minor_diff != 0 {
            diff.minor += minor_diff;
            return diff;
        }

        diff.patch = patch_diff;
        diff.prerelease = prerelease_diff;
        diff
    }
}

#[cw_serde]
#[derive(Default, Copy, PartialOrd, Ord, Eq)]
pub struct TotalVersionDifference {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: u32,
}

impl Add for TotalVersionDifference {
    type Output = TotalVersionDifference;
    fn add(self, rhs: TotalVersionDifference) -> Self::Output {
        TotalVersionDifference {
            major: self.major.add(rhs.major),
            minor: self.minor.add(rhs.minor),
            patch: self.patch.add(rhs.patch),
            prerelease: self.prerelease.add(rhs.prerelease),
        }
    }
}

impl Sub for TotalVersionDifference {
    type Output = TotalVersionDifference;
    fn sub(self, rhs: TotalVersionDifference) -> Self::Output {
        TotalVersionDifference {
            major: self.major.sub(rhs.major),
            minor: self.minor.sub(rhs.minor),
            patch: self.patch.sub(rhs.patch),
            prerelease: self.prerelease.sub(rhs.prerelease),
        }
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

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<u32>,
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

/// Defines weights for calculating numbers of versions behind the current release.
#[cw_serde]
#[derive(Copy)]
pub struct OutdatedVersionWeights {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: u32,
}

fn is_one_semver_difference(this: &semver::Version, other: &semver::Version) -> bool {
    let major_diff = (this.major as i64 - other.major as i64).unsigned_abs() as u32;
    let minor_diff = (this.minor as i64 - other.minor as i64).unsigned_abs() as u32;
    let patch_diff = (this.patch as i64 - other.patch as i64).unsigned_abs() as u32;
    let prerelease_diff = if this.pre == other.pre { 0 } else { 1 };

    if major_diff == 1 {
        return true;
    }

    if major_diff == 0 && minor_diff == 1 {
        return true;
    }

    if major_diff == 0 && minor_diff == 0 && patch_diff == 1 {
        return true;
    }

    prerelease_diff == 1
}

impl OutdatedVersionWeights {
    pub fn difference_to_versions_behind_factor(&self, diff: TotalVersionDifference) -> u32 {
        diff.major * self.major
            + diff.minor * self.minor
            + diff.patch * self.patch
            + diff.prerelease * self.prerelease
    }

    // INVARIANT: release chain is sorted
    // do NOT call this method directly from inside the contract. it's too inefficient
    // it relies on some external caching.
    pub fn versions_behind_factor(
        &self,
        node_version: &semver::Version,
        release_chain: &[HistoricalNymNodeVersionEntry],
    ) -> u32 {
        let Some(latest) = release_chain.last() else {
            return 0;
        };

        let latest_semver = latest.version_information.semver_unchecked();

        // if you're more recent than the latest, you get the benefit of the doubt, the release might have not yet been commited to the chain
        // but only if you're only a single semver ahead, otherwise you get penalty equivalent of being major version behind for cheating
        if node_version > &latest_semver {
            return if is_one_semver_difference(node_version, &latest_semver) {
                0
            } else {
                self.major
            };
        }

        // find your position in the release chain, if we fail, we assume that the node comes from before the changes were introduced
        // in which case we simply calculate the absolute difference between the genesis entry and add up the total difference
        let version_diff = match release_chain
            .iter()
            .find(|h| h.version_information.semver == node_version.to_string())
        {
            Some(version_chain_entry) => {
                // determine the difference against the current
                version_chain_entry
                    .version_information
                    .relative_difference(&latest.version_information)
            }
            None => {
                // SAFETY: since we managed to get 'last' entry, it means the release chain is not empty,
                // so we must be able to obtain the first entry
                #[allow(clippy::unwrap_used)]
                let genesis = release_chain.first().unwrap();

                let difference_from_genesis = genesis
                    .version_information
                    .difference_against_legacy(node_version);
                difference_from_genesis + latest.version_information.difference_since_genesis
            }
        };

        self.difference_to_versions_behind_factor(version_diff)
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
    use std::ops::Deref;

    // simple wrapper for tests
    struct ReleaseChain {
        inner: Vec<HistoricalNymNodeVersionEntry>,
    }

    impl Deref for ReleaseChain {
        type Target = [HistoricalNymNodeVersionEntry];
        fn deref(&self) -> &Self::Target {
            self.inner.deref()
        }
    }

    impl ReleaseChain {
        fn new(initial: &str) -> Self {
            ReleaseChain {
                inner: vec![HistoricalNymNodeVersionEntry {
                    id: 0,
                    version_information: HistoricalNymNodeVersion {
                        semver: initial.to_string(),
                        introduced_at_height: 123,
                        difference_since_genesis: TotalVersionDifference::default(),
                    },
                }],
            }
        }

        fn with_release(mut self, raw: &str) -> Self {
            self.push_new(raw);
            self
        }

        fn push_new(&mut self, raw: &str) {
            let latest = self.inner.last().unwrap();
            let new_version: semver::Version = raw.parse().unwrap();

            let new_absolute = latest
                .version_information
                .difference_against_new_current(&new_version);

            self.inner.push(HistoricalNymNodeVersionEntry {
                id: latest.id + 1,
                version_information: HistoricalNymNodeVersion {
                    semver: new_version.to_string(),
                    introduced_at_height: latest.version_information.introduced_at_height + 1,
                    difference_since_genesis: new_absolute,
                },
            })
        }
    }

    #[test]
    fn versions_behind_factor() {
        // helper to compact the parsing
        fn s(raw: &str) -> semver::Version {
            raw.parse().unwrap()
        }

        let weights = OutdatedVersionWeights::default();

        // no releases:
        let res = weights.versions_behind_factor(&s("1.1.13"), &[]);
        assert_eq!(0, res);

        // ###############################
        // single released version (1.1.13)
        // ###############################
        let mut release_chain = ReleaseChain::new("1.1.13");

        // "legacy" versions
        let res = weights.versions_behind_factor(&s("1.0.12"), &release_chain);
        assert_eq!(10, res);
        let res = weights.versions_behind_factor(&s("1.0.4"), &release_chain);
        assert_eq!(10, res);
        let res = weights.versions_behind_factor(&s("1.0.1"), &release_chain);
        assert_eq!(10, res);
        let res = weights.versions_behind_factor(&s("0.1.12"), &release_chain);
        assert_eq!(100, res);

        let res = weights.versions_behind_factor(&s("1.1.12"), &release_chain);
        assert_eq!(1, res);
        let res = weights.versions_behind_factor(&s("1.1.11"), &release_chain);
        assert_eq!(2, res);
        let res = weights.versions_behind_factor(&s("1.1.9"), &release_chain);
        assert_eq!(4, res);

        // current version
        let res = weights.versions_behind_factor(&s("1.1.13"), &release_chain);
        assert_eq!(0, res);

        // "ahead" versions
        let res = weights.versions_behind_factor(&s("1.1.14"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("1.2.0"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("2.0.0"), &release_chain);
        assert_eq!(0, res);

        // cheating ahead:
        let res = weights.versions_behind_factor(&s("1.1.15"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("1.3.0"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("3.0.0"), &release_chain);
        assert_eq!(100, res);

        // ###############################
        // small patch release chain (1.1.13 => 1.1.14 => 1.1.15 => 1.1.16)
        // ###############################
        release_chain.push_new("1.1.14");
        release_chain.push_new("1.1.15");
        release_chain.push_new("1.1.16");

        // "legacy" versions
        let res = weights.versions_behind_factor(&s("1.0.12"), &release_chain);
        assert_eq!(13, res);
        let res = weights.versions_behind_factor(&s("1.0.4"), &release_chain);
        assert_eq!(13, res);
        let res = weights.versions_behind_factor(&s("1.0.1"), &release_chain);
        assert_eq!(13, res);
        let res = weights.versions_behind_factor(&s("0.1.12"), &release_chain);
        assert_eq!(103, res);

        let res = weights.versions_behind_factor(&s("1.1.12"), &release_chain);
        assert_eq!(4, res);
        let res = weights.versions_behind_factor(&s("1.1.11"), &release_chain);
        assert_eq!(5, res);
        let res = weights.versions_behind_factor(&s("1.1.9"), &release_chain);
        assert_eq!(7, res);

        // current version
        let res = weights.versions_behind_factor(&s("1.1.16"), &release_chain);
        assert_eq!(0, res);

        // present in the chain
        let res = weights.versions_behind_factor(&s("1.1.15"), &release_chain);
        assert_eq!(1, res);
        let res = weights.versions_behind_factor(&s("1.1.14"), &release_chain);
        assert_eq!(2, res);
        let res = weights.versions_behind_factor(&s("1.1.13"), &release_chain);
        assert_eq!(3, res);

        // "ahead" versions
        let res = weights.versions_behind_factor(&s("1.1.17"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("1.2.0"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("2.0.0"), &release_chain);
        assert_eq!(0, res);

        // cheating ahead:
        let res = weights.versions_behind_factor(&s("1.1.18"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("1.3.0"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("3.0.0"), &release_chain);
        assert_eq!(100, res);

        // ###############################
        // small minor release chain (1.2.0 => 1.3.0 => 1.4.0)
        // ###############################
        let release_chain = ReleaseChain::new("1.2.0")
            .with_release("1.3.0")
            .with_release("1.4.0");

        // "legacy" versions
        let res = weights.versions_behind_factor(&s("1.0.12"), &release_chain);
        assert_eq!(40, res);
        let res = weights.versions_behind_factor(&s("1.0.4"), &release_chain);
        assert_eq!(40, res);
        let res = weights.versions_behind_factor(&s("1.0.1"), &release_chain);
        assert_eq!(40, res);
        let res = weights.versions_behind_factor(&s("0.1.12"), &release_chain);
        assert_eq!(120, res);

        let res = weights.versions_behind_factor(&s("1.1.12"), &release_chain);
        assert_eq!(30, res);
        let res = weights.versions_behind_factor(&s("1.1.11"), &release_chain);
        assert_eq!(30, res);
        let res = weights.versions_behind_factor(&s("1.1.9"), &release_chain);
        assert_eq!(30, res);

        // current version
        let res = weights.versions_behind_factor(&s("1.4.0"), &release_chain);
        assert_eq!(0, res);

        // present in the chain
        let res = weights.versions_behind_factor(&s("1.2.0"), &release_chain);
        assert_eq!(20, res);
        let res = weights.versions_behind_factor(&s("1.3.0"), &release_chain);
        assert_eq!(10, res);

        // weird in between
        let res = weights.versions_behind_factor(&s("1.2.1"), &release_chain);
        assert_eq!(21, res);
        let res = weights.versions_behind_factor(&s("1.3.3"), &release_chain);
        assert_eq!(30, res);

        // "ahead" versions
        let res = weights.versions_behind_factor(&s("1.4.1"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("1.5.0"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("2.0.0"), &release_chain);
        assert_eq!(0, res);

        // cheating ahead:
        let res = weights.versions_behind_factor(&s("1.4.2"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("1.6.0"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("3.0.0"), &release_chain);
        assert_eq!(100, res);

        // ###############################
        // mixed release chain (1.1.13 => 1.2.0 => 1.2.1 => 1.3.0 => 1.3.1 => 1.3.2 => 1.4.0)
        // ###############################
        let release_chain = ReleaseChain::new("1.1.13")
            .with_release("1.2.0")
            .with_release("1.2.1")
            .with_release("1.3.0")
            .with_release("1.3.1")
            .with_release("1.3.1-important")
            .with_release("1.3.2")
            .with_release("1.4.0");

        // "legacy" versions
        let res = weights.versions_behind_factor(&s("1.0.12"), &release_chain);
        assert_eq!(44, res);
        let res = weights.versions_behind_factor(&s("1.0.4"), &release_chain);
        assert_eq!(44, res);
        let res = weights.versions_behind_factor(&s("1.0.1"), &release_chain);
        assert_eq!(44, res);
        let res = weights.versions_behind_factor(&s("0.1.12"), &release_chain);
        assert_eq!(134, res);

        let res = weights.versions_behind_factor(&s("1.1.12"), &release_chain);
        assert_eq!(35, res);
        let res = weights.versions_behind_factor(&s("1.1.11"), &release_chain);
        assert_eq!(36, res);
        let res = weights.versions_behind_factor(&s("1.1.9"), &release_chain);
        assert_eq!(38, res);

        // current version
        let res = weights.versions_behind_factor(&s("1.4.0"), &release_chain);
        assert_eq!(0, res);

        // present in the chain
        let res = weights.versions_behind_factor(&s("1.1.13"), &release_chain);
        assert_eq!(34, res);
        let res = weights.versions_behind_factor(&s("1.2.0"), &release_chain);
        assert_eq!(24, res);
        let res = weights.versions_behind_factor(&s("1.2.1"), &release_chain);
        assert_eq!(23, res);
        let res = weights.versions_behind_factor(&s("1.3.0"), &release_chain);
        assert_eq!(13, res);
        let res = weights.versions_behind_factor(&s("1.3.1"), &release_chain);
        assert_eq!(12, res);
        let res = weights.versions_behind_factor(&s("1.3.1-important"), &release_chain);
        assert_eq!(11, res);
        let res = weights.versions_behind_factor(&s("1.3.2"), &release_chain);
        assert_eq!(10, res);

        // weird in between
        let res = weights.versions_behind_factor(&s("1.2.3"), &release_chain);
        assert_eq!(44, res);
        let res = weights.versions_behind_factor(&s("1.3.69"), &release_chain);
        assert_eq!(54, res);

        // "ahead" versions
        let res = weights.versions_behind_factor(&s("1.4.1"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("1.5.0"), &release_chain);
        assert_eq!(0, res);
        let res = weights.versions_behind_factor(&s("2.0.0"), &release_chain);
        assert_eq!(0, res);

        // cheating ahead:
        let res = weights.versions_behind_factor(&s("1.4.2"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("1.6.0"), &release_chain);
        assert_eq!(100, res);
        let res = weights.versions_behind_factor(&s("3.0.0"), &release_chain);
        assert_eq!(100, res);
    }
}
