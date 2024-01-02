// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::serde_helpers::{hex, option_offsetdatetime};
use crate::error::NymvisorError;
use crate::helpers::{calculate_file_checksum, init_path};
use crate::upgrades::download::os_arch;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::{fs, io};
use time::OffsetDateTime;
use tracing::error;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpgradePlan {
    // metadata indicating save location of the underlying file
    #[serde(skip)]
    _save_path: Option<PathBuf>,

    current: UpgradeInfo,

    // TODO: or maybe BTreeMap<OffsetDateTime, UpgradeInfo>, would be more appropriate?
    queued_up: Vec<UpgradeInfo>,
}

impl UpgradePlan {
    pub(crate) fn new(current: UpgradeInfo) -> Self {
        UpgradePlan {
            _save_path: None,
            current,
            queued_up: Vec::new(),
        }
    }

    fn push_next_upgrade(&mut self, upgrade: UpgradeInfo) {
        self.queued_up.push(upgrade);

        // we could be fancy and try to determine the correct index for the insertion point
        // or we could just do a naive thing of sorting the elements by the upgrade time.
        // is it less efficient? sure
        // does it matter? no because we'll have at most few elements here
        // so the overhead will be in the order of nanoseconds/microseconds
        self.queued_up.sort_by_key(|u| u.upgrade_time)
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
        self.queued_up.first()
    }

    pub(crate) fn pop_next_upgrade(&mut self) -> Option<UpgradeInfo> {
        // yes, yes. VecDeque would have been perfect for this instead,
        // but again, we'll hardly ever have more than 2-3 elements here so it doesn't matter
        if !self.queued_up.is_empty() {
            Some(self.queued_up.remove(0))
        } else {
            None
        }
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

    pub(crate) fn has_planned_by_name(&self, upgrade_name: &str) -> bool {
        for planned in &self.queued_up {
            if planned.name == upgrade_name {
                return true;
            }
        }
        false
    }

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
#[serde(rename_all = "camelCase")]
pub enum DigestAlgorithm {
    Sha256,
    Sha512,
}

impl Display for DigestAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DigestAlgorithm::Sha256 => write!(f, "sha256"),
            DigestAlgorithm::Sha512 => write!(f, "sha512"),
        }
    }
}

impl DigestAlgorithm {
    pub fn calculate_file_checksum<P: AsRef<Path>>(
        &self,
        filepath: P,
    ) -> Result<Vec<u8>, NymvisorError> {
        let path = filepath.as_ref();
        match self {
            DigestAlgorithm::Sha256 => calculate_file_checksum::<Sha256, _>(path),
            DigestAlgorithm::Sha512 => calculate_file_checksum::<Sha512, _>(path),
        }
        .map_err(|source| NymvisorError::ChecksumCalculationFailure {
            path: path.to_path_buf(),
            source,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadUrl {
    /// The hex-encoded checksum of the file behind the download url.
    #[serde(with = "hex")]
    pub checksum: Vec<u8>,

    /// The algorithm used for computing the checksum
    pub checksum_algorithm: DigestAlgorithm,

    /// Download url for this particular platform
    pub url: Url,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeInfo {
    /// Specifies whether this upgrade requires manual intervention and cannot be done automatically by the nymvisor.
    pub manual: bool,

    /// Name of this upgrade, for example `2023.4-galaxy`
    pub name: String,

    /// Additional information about this release
    pub notes: String,

    /// Optional rfc3339 datetime of the publishing date of the release,
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

        init_path(parent)?;

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
        fs::File::open(path)
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|serde_json_err| io::Error::new(io::ErrorKind::Other, serde_json_err))
            })
            .map_err(|source| NymvisorError::UpgradeInfoLoadFailure {
                path: path.to_path_buf(),
                source,
            })
    }

    // pub(crate) fn is_genesis(&self) -> bool {
    //     self.name == GENESIS_DIR
    // }

    pub(crate) fn get_download_url(&self) -> Result<&DownloadUrl, NymvisorError> {
        if let Some(download_url) = self.platforms.get(&os_arch()) {
            return Ok(download_url);
        }
        self.platforms
            .get("any")
            .ok_or(NymvisorError::NoDownloadUrls {
                upgrade_name: self.name.clone(),
                arch: os_arch(),
                available: self.platforms.keys().cloned().collect(),
            })
    }

    /// Check whether the loaded (presumably `current`) upgrade-info matches the provided current version information.
    pub(crate) fn ensure_matches(
        &self,
        current_info: &CurrentVersionInfo,
    ) -> Result<(), NymvisorError> {
        if self.name != current_info.name || self.version != current_info.version {
            return Err(NymvisorError::UnexpectedCurrentVersionInfo {
                current_info: Box::new(self.clone()),
                current_version_info: Box::new(current_info.clone()),
            });
        }

        if let Some(bin_info) = &self.binary_details {
            if bin_info != &current_info.binary_details {
                return Err(NymvisorError::UnexpectedCurrentVersionInfo {
                    current_info: Box::new(self.clone()),
                    current_version_info: Box::new(current_info.clone()),
                });
            }
        }

        Ok(())
    }

    pub(crate) fn ensure_matches_bin_info(
        &self,
        info: &BinaryBuildInformationOwned,
    ) -> Result<(), NymvisorError> {
        if let Some(self_info) = &self.binary_details {
            if self_info != info {
                return Err(NymvisorError::UnexpectedDaemonBuild {
                    daemon_info: Box::new(info.clone()),
                    stored_info: Box::new(self_info.clone()),
                });
            }
        }
        if self.version != info.build_version {
            return Err(NymvisorError::UnexpectedUpgradeDaemonVersion {
                upgrade_name: self.name.clone(),
                daemon_version: info.build_version.clone(),
                expected: self.version.clone(),
            });
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeHistory {
    // metadata indicating save location of the underlying file
    #[serde(skip)]
    _save_path: Option<PathBuf>,

    history: Vec<UpgradeHistoryEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
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
    pub(crate) fn new<P: AsRef<Path>>(save_path: P) -> Self {
        UpgradeHistory {
            _save_path: Some(save_path.as_ref().to_path_buf()),
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
        let mut history: UpgradeHistory = fs::File::open(path)
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|serde_json_err| io::Error::new(io::ErrorKind::Other, serde_json_err))
            })
            .map_err(|source| NymvisorError::UpgradeHistoryLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;

        history._save_path = Some(path.to_path_buf());
        Ok(history)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurrentVersionInfo {
    /// Name of the current version, for example `2023.4-galaxy`
    pub name: String,

    /// Version of this upgrade, for example `1.1.69`
    pub version: String,

    /// Time when the upgrade has happened.
    #[serde(with = "time::serde::rfc3339")]
    pub upgrade_time: OffsetDateTime,

    /// Build information of the expected current binary for additional verification
    pub binary_details: BinaryBuildInformationOwned,
}

impl CurrentVersionInfo {
    pub(crate) fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), NymvisorError> {
        let path = path.as_ref();

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|source| NymvisorError::CurrentVersionInfoSaveFailure {
                path: path.to_path_buf(),
                source,
            })?;

        // we're not using any non-standard serializer and thus the serialization should not ever fail
        #[allow(clippy::expect_used)]
        serde_json::to_writer_pretty(file, self)
            .expect("unexpected CurrentVersionInfo serialization failure");
        Ok(())
    }

    pub(crate) fn try_load<P: AsRef<Path>>(path: P) -> Result<Self, NymvisorError> {
        let path = path.as_ref();
        fs::File::open(path)
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|serde_json_err| io::Error::new(io::ErrorKind::Other, serde_json_err))
            })
            .map_err(|source| NymvisorError::CurrentVersionInfoLoadFailure {
                path: path.to_path_buf(),
                source,
            })
    }
}
