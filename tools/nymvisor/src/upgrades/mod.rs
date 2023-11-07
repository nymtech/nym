// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::{Deserialize, Serialize};
use serde_helpers::{base64, option_offsetdatetime};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use time::OffsetDateTime;
use url::Url;

mod http_upstream;
mod serde_helpers;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UpgradePlan {
    pub current: UpgradeInfo,

    pub next: Vec<UpgradeInfo>,
}

impl UpgradePlan {
    pub(crate) fn new(current: UpgradeInfo) -> Self {
        UpgradePlan {
            current,
            next: vec![],
        }
    }

    pub(crate) fn update_on_disk(&self) -> Result<(), NymvisorError> {
        // 1. copy upgrade-plan.json to upgrade-plan.json.tmp
        // 2. update upgrade-plan.json.tmp
        // 3. move upgrade-plan.json.tmp to upgrade-plan.json

        todo!()
    }

    pub(crate) fn save_new<P: AsRef<Path>>(&self, path: P) -> Result<(), NymvisorError> {
        let path = path.as_ref();
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|source| NymvisorError::UpgradePlanSaveFailure {
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
        let path = path.as_ref();
        std::fs::File::open(path)
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|serde_json_err| io::Error::new(io::ErrorKind::Other, serde_json_err))
            })
            .map_err(|source| NymvisorError::UpgradePlanLoadFailure {
                path: path.to_path_buf(),
                source,
            })
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
    /// The checksum of the file behind the download url.
    #[serde(with = "base64")]
    pub checksum: Vec<u8>,

    /// The algorithm used for computing the checksum
    pub checksum_algorithm: DigestAlgorithm,

    /// Download url for this particular platform
    pub url: Url,
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
        let path = path.as_ref();
        std::fs::File::open(path)
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|serde_json_err| io::Error::new(io::ErrorKind::Other, serde_json_err))
            })
            .map_err(|source| NymvisorError::UpgradeInfoLoadFailure {
                path: path.to_path_buf(),
                source,
            })
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
