// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::hash::Hash;

pub trait Versioned: Clone {
    fn version(&self) -> String;
}

pub trait VersionFilterable<T> {
    #[must_use]
    fn filter_by_version(&self, expected_version: &str) -> Self;
}

impl<T> VersionFilterable<T> for Vec<T>
where
    T: Versioned,
{
    fn filter_by_version(&self, expected_version: &str) -> Self {
        self.iter()
            .filter(|node| {
                version_checker::is_minor_version_compatible(&node.version(), expected_version)
            })
            .cloned()
            .collect()
    }
}

impl<T, K, V> VersionFilterable<T> for HashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: VersionFilterable<T>,
    T: Versioned,
{
    fn filter_by_version(&self, expected_version: &str) -> Self {
        self.iter()
            .map(|(k, v)| (k.clone(), v.filter_by_version(expected_version)))
            .collect()
    }
}
