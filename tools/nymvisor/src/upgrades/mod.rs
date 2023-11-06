// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::Path;
use time::OffsetDateTime;

mod http_upstream;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UpgradePlan {
    pub current: UpgradeInfo,

    pub next: Vec<UpgradeInfo>,
}

impl UpgradePlan {
    pub(crate) fn update_on_disk(&self) -> Result<(), NymvisorError> {
        // 1. copy upgrade-plan.json to upgrade-plan.json.tmp
        // 2. update upgrade-plan.json.tmp
        // 3. move upgrade-plan.json.tmp to upgrade-plan.json

        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DigestAlgorithm {
    Sha256,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct DownloadUrl {
    // TODO: get it from base64
    /// The checksum of the file behind the download url.
    pub checksum: Vec<u8>,

    /// The algorithm used for computing the checksum
    pub checksum_algorithm: DigestAlgorithm,

    /// Download url for this particular platform
    pub url: String,
}

mod option_offsetdatetime {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::OffsetDateTime;

    pub fn serialize<S>(value: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a>(#[serde(with = "time::serde::rfc3339")] &'a OffsetDateTime);

        value.as_ref().map(Helper).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "time::serde::rfc3339")] OffsetDateTime);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UpgradeInfo {
    /// Specifies whether this upgrade requires manual intervention and cannot be done automatically by the nymvisor.
    pub manual: bool,

    /// Name of this upgrade, for example `2023.4-galaxy`
    pub name: String,

    /// Additional information about this release
    pub notes: String,

    /// Optional rfc3339 datetime of the publish date of the release,
    #[serde(with = "option_offsetdatetime")]
    pub publish_date: Option<OffsetDateTime>,

    /// Version of this upgrade, for example `1.1.69`
    pub version: String,

    /// Platform specific download urls, for example `linux-x86_64`
    pub platforms: HashMap<String, DownloadUrl>,

    /// Time when the upgrade should happen.
    #[serde(with = "time::serde::rfc3339")]
    pub upgrade_time: OffsetDateTime,

    /// Optional build information of the upgraded binary for additional verification
    pub binary_details: Option<BinaryBuildInformationOwned>,
}

impl UpgradeInfo {
    pub(crate) fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), NymvisorError> {
        let path = path.as_ref();
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|source| NymvisorError::UpgradeInfoSaveFailure {
                name: self.name.clone(),
                path: path.to_path_buf(),
                source,
            })?;

        // we're not using any non-standard serializer and thus the serialization should not ever fail
        #[allow(clippy::expect_used)]
        serde_json::to_writer_pretty(file, self)
            .expect("unexpected UpgradeInfo serialization failure");
        Ok(())
    }

    pub(crate) fn try_load<P: AsRef<Path>>(path: P) -> Result<Self, NymvisorError> {
        let file = std::fs::File::open(path)?;
        serde_json::from_reader(file).map_err(Into::into)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UpgradeHistory(Vec<UpgradeHistoryEntry>);

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UpgradeHistoryEntry {
    #[serde(with = "time::serde::rfc3339")]
    performed_at: OffsetDateTime,
    info: UpgradeInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
        let a = UpgradeInfo {
            manual: false,
            name: "".to_string(),
            notes: "".to_string(),
            publish_date: None,
            version: "".to_string(),
            platforms: Default::default(),
            upgrade_time: OffsetDateTime::now_utc(),
            binary_details: None,
        };

        let aa = serde_json::to_string_pretty(&a).unwrap();
        println!("{aa}")
    }
}
