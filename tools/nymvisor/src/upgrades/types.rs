// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::serde_helpers::{base64, option_offsetdatetime};
use crate::config::GENESIS_DIR;
use crate::error::NymvisorError;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::{fs, io};
use time::OffsetDateTime;
use tracing::error;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub struct UpgradePlan {
    // metadata indicating save location of the underlying file
    #[serde(skip)]
    _save_path: Option<PathBuf>,

    current: UpgradeInfo,

    // TODO: or maybe BTreeMap<OffsetDateTime, UpgradeInfo>, would be more appropriate?
    queued_up: VecDeque<UpgradeInfo>,
}

impl UpgradePlan {
    pub(crate) fn new(current: UpgradeInfo) -> Self {
        UpgradePlan {
            _save_path: None,
            current,
            queued_up: VecDeque::new(),
        }
    }

    fn push_next_upgrade(&mut self, upgrade: UpgradeInfo) {
        self.queued_up.push_back(upgrade);
    }

    pub(crate) fn update_on_disk(&self) -> Result<(), NymvisorError> {
        // it should be impossible to update an existing upgrade plan that wasn't loaded from disk
        assert!(self._save_path.is_some());

        // safety: the except here is fine as this failure implies failure in the underlying logic of the code
        // as opposed to user error
        #[allow(clippy::expect_used)]
        let save_path = self
            ._save_path
            .as_ref()
            .expect("loaded upgrade plan does not have an associate save path!");

        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(save_path)
            .map_err(|source| NymvisorError::UpgradePlanSaveFailure {
                path: save_path.to_path_buf(),
                source,
            })?;

        // we're not using any non-standard serializer and thus the serialization should not ever fail
        #[allow(clippy::expect_used)]
        serde_json::to_writer_pretty(file, self)
            .expect("unexpected UpgradeInfo serialization failure");
        Ok(())
    }

    pub(crate) fn insert_new_upgrade(&mut self, upgrade: UpgradeInfo) -> Result<(), NymvisorError> {
        self.push_next_upgrade(upgrade);
        self.update_on_disk()
    }

    pub(crate) fn current(&self) -> &UpgradeInfo {
        &self.current
    }

    pub(crate) fn set_current(&mut self, new_current: UpgradeInfo) {
        self.current = new_current
    }

    pub(crate) fn next_upgrade(&self) -> Option<&UpgradeInfo> {
        self.queued_up.front()
    }

    pub(crate) fn pop_next_upgrade(&mut self) -> Option<UpgradeInfo> {
        self.queued_up.pop_front()
    }

    pub(crate) fn has_planned(&self, upgrade: &UpgradeInfo) -> bool {
        for planned in &self.queued_up {
            if planned.version == upgrade.version {
                if planned.name != upgrade.name {
                    // TODO: should we maybe return a hard error here instead?
                    error!("we have already a planned upgrade for version {} under name '{}' which differs from provided '{}'", planned.version, planned.name, upgrade.name);
                }
                return true;
            }
        }
        false
    }

    // pub(crate) fn update_current(&mut self) -> Result<(), NymvisorError> {
    //
    // }

    pub(crate) fn save_new<P: AsRef<Path>>(&self, path: P) -> Result<(), NymvisorError> {
        debug_assert!(self._save_path.is_none());

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
        let mut upgrade_plan: UpgradePlan = fs::File::open(path)
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|serde_json_err| io::Error::new(io::ErrorKind::Other, serde_json_err))
            })
            .map_err(|source| NymvisorError::UpgradePlanLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;

        upgrade_plan._save_path = Some(path.to_path_buf());
        Ok(upgrade_plan)
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DigestAlgorithm {
    Sha256,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

        // in case we're saving brand new upgrade info, make sure the parent directory exists
        #[allow(clippy::expect_used)]
        let parent = path
            .parent()
            .expect("attempted to save the upgrade info as the root of the fs");

        fs::create_dir_all(parent).map_err(|source| NymvisorError::PathInitFailure {
            path: parent.to_path_buf(),
            source,
        })?;

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

    pub(crate) fn is_genesis(&self) -> bool {
        self.name == GENESIS_DIR
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UpgradeHistory {
    // metadata indicating save location of the underlying file
    #[serde(skip)]
    _save_path: Option<PathBuf>,

    history: Vec<UpgradeHistoryEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UpgradeHistoryEntry {
    #[serde(with = "time::serde::rfc3339")]
    performed_at: OffsetDateTime,

    info: UpgradeInfo,
}

impl UpgradeHistoryEntry {
    fn new(info: UpgradeInfo) -> Self {
        UpgradeHistoryEntry {
            performed_at: OffsetDateTime::now_utc(),
            info,
        }
    }
}

impl UpgradeHistory {
    pub(crate) fn new() -> Self {
        UpgradeHistory {
            _save_path: None,
            history: vec![],
        }
    }

    pub(crate) fn update_on_disk(&self) -> Result<(), NymvisorError> {
        // it should be impossible to update an existing upgrade history that wasn't loaded from disk
        assert!(self._save_path.is_some());

        // safety: the except here is fine as this failure implies failure in the underlying logic of the code
        // as opposed to user error
        #[allow(clippy::expect_used)]
        let save_path = self
            ._save_path
            .as_ref()
            .expect("loaded upgrade history does not have an associate save path!");

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(save_path)
            .map_err(|source| NymvisorError::UpgradeHistorySaveFailure {
                path: save_path.to_path_buf(),
                source,
            })?;

        // we're not using any non-standard serializer and thus the serialization should not ever fail
        #[allow(clippy::expect_used)]
        serde_json::to_writer_pretty(file, self)
            .expect("unexpected UpgradeHistory serialization failure");
        Ok(())
    }

    fn push_upgrade(&mut self, upgrade: UpgradeInfo) {
        self.history.push(UpgradeHistoryEntry::new(upgrade));
    }

    pub(crate) fn insert_new_upgrade(&mut self, upgrade: UpgradeInfo) -> Result<(), NymvisorError> {
        self.push_upgrade(upgrade);
        self.update_on_disk()
    }

    pub(crate) fn try_load<P: AsRef<Path>>(path: P) -> Result<Self, NymvisorError> {
        let path = path.as_ref();
        std::fs::File::open(path)
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|serde_json_err| io::Error::new(io::ErrorKind::Other, serde_json_err))
            })
            .map_err(|source| NymvisorError::UpgradeHistoryLoadFailure {
                path: path.to_path_buf(),
                source,
            })
    }
}
