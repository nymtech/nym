// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::hash::Hash;

pub trait Versioned: Clone {
    fn version(&self) -> String;
}

pub trait VersionFilterable<T> {
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
