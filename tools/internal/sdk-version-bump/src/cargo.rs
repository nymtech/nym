// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ReleasePackage;
use anyhow::Context;
use cargo_edit::LocalManifest;
use semver::Version;
use std::path::Path;

pub struct CargoPackage {
    inner: LocalManifest,
}

impl ReleasePackage for CargoPackage {
    fn type_name() -> &'static str {
        "Cargo.toml"
    }

    fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let inner = LocalManifest::find(Some(path.as_ref()))?;
        println!("\t>>> located the file at {}", inner.path.display());

        Ok(CargoPackage { inner })
    }

    fn save_changes(&mut self) -> anyhow::Result<()> {
        self.inner.write()
    }

    fn get_current_version(&self) -> anyhow::Result<Version> {
        self.inner
            .manifest
            .data
            .get("package")
            .context("no package")?
            .get("version")
            .context("no version")?
            .as_str()
            .context("not a valid str")?
            .parse()
            .map_err(Into::into)
    }

    fn set_version(&mut self, version: &Version) {
        self.inner.set_package_version(version);
    }
}
