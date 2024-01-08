// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context};
use semver::{Prerelease, Version};
use std::collections::HashSet;
use std::path::Path;

pub(crate) trait ReleasePackage: Sized {
    fn type_name() -> &'static str;

    fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self>;

    fn save_changes(&mut self) -> anyhow::Result<()>;

    fn get_current_version(&self) -> anyhow::Result<Version>;

    fn set_version(&mut self, version: &Version);

    fn remove_suffix(&mut self) -> anyhow::Result<()> {
        let version = self.get_current_version()?;
        let updated = version.try_remove_prerelease()?;

        println!("\t>>> changing version from {version} to {updated}");
        self.set_version(&updated);
        Ok(())
    }

    fn bump_version(&mut self, pre_release: bool) -> anyhow::Result<()> {
        let version = self.get_current_version()?;
        let updated = if pre_release {
            if version.pre.is_empty() {
                let patch_updated = version.try_bump_patch_without_pre()?;
                patch_updated.set_initial_release_candidate()?
            } else {
                version.try_bump_prerelease()?
            }
        } else {
            version.try_bump_patch_without_pre()?
        };

        println!("\t>>> changing version from {version} to {updated}");
        self.set_version(&updated);
        Ok(())
    }

    fn update_nym_dependencies(&mut self, _: &HashSet<String>, _: bool) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(crate) trait VersionBumpExt: Sized {
    fn try_bump_prerelease(&self) -> anyhow::Result<Self>;
    fn try_bump_patch_without_pre(&self) -> anyhow::Result<Self>;

    fn set_initial_release_candidate(&self) -> anyhow::Result<Self>;
    fn try_remove_prerelease(&self) -> anyhow::Result<Self>;
}

pub(crate) fn try_bump_raw_prerelease(raw: &str) -> anyhow::Result<Prerelease> {
    // ugh that's disgusting
    let (rc_prefix, pre_version) = raw
        .split_once('.')
        .context("the prerelease version does not contain a valid rc.X suffix")?;

    let parsed_version: u32 = pre_version.parse()?;
    let updated_version = parsed_version + 1;

    Ok(format!("{rc_prefix}.{updated_version}").parse()?)
}

impl VersionBumpExt for Version {
    fn try_bump_prerelease(&self) -> anyhow::Result<Self> {
        if self.pre.is_empty() {
            bail!("the current version ({self}) does not have pre-release data set - are you sure you followed the release process correctly?")
        }

        Ok(Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
            pre: try_bump_raw_prerelease(self.pre.as_str())?,
            build: self.build.clone(),
        })
    }

    fn try_bump_patch_without_pre(&self) -> anyhow::Result<Self> {
        if !self.pre.is_empty() {
            bail!("the current version ({self}) has pre-release data set - are you sure you followed the release process correctly?")
        }

        let mut updated = self.clone();
        updated.patch += 1;
        Ok(updated)
    }

    fn set_initial_release_candidate(&self) -> anyhow::Result<Self> {
        if !self.pre.is_empty() {
            bail!("the current version ({self}) has pre-release data set - are you sure you followed the release process correctly?")
        }
        Ok(Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
            pre: Prerelease::new("rc.0")?,
            build: self.build.clone(),
        })
    }

    fn try_remove_prerelease(&self) -> anyhow::Result<Self> {
        if self.pre.is_empty() {
            bail!("the current version ({self}) does not have pre-release data set - are you sure you followed the release process correctly?")
        }
        Ok(Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
            pre: Default::default(),
            build: self.build.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    #[test]
    fn bump_patch_version() {
        let cases = [
            ("1.0.0", Some("1.0.1")),
            ("2.3.0", Some("2.3.1")),
            ("1.0.0-rc.0", None),
            ("1.0.0-rc.foomp", None),
        ];

        for (raw, expected) in cases {
            let updated = Version::parse(raw).unwrap().try_bump_patch_without_pre();
            if let Some(expected) = expected {
                let expected = Version::parse(expected).unwrap();
                assert_eq!(expected, updated.unwrap())
            } else {
                assert!(updated.is_err())
            }
        }
    }

    #[test]
    fn bump_rc_version() {
        let cases = [
            ("1.0.0", None),
            ("1.0.0-rc.0", Some("1.0.0-rc.1")),
            ("1.0.0-rc.-1", None),
            ("1.2.3-rc.42", Some("1.2.3-rc.43")),
            ("1.2.3-rc42", None),
            ("1.0.0-rc.foomp", None),
        ];

        for (raw, expected) in cases {
            let updated = Version::parse(raw).unwrap().try_bump_prerelease();
            if let Some(expected) = expected {
                let expected = Version::parse(expected).unwrap();
                assert_eq!(expected, updated.unwrap())
            } else {
                assert!(updated.is_err())
            }
        }
    }
}
