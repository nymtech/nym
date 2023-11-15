// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::try_bump_raw_prerelease;
use crate::json_types::DepsSet;
use crate::{json_types, ReleasePackage};
use anyhow::{bail, Context};
use semver::{Comparator, Prerelease, Version, VersionReq};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub struct PackageJson {
    path: PathBuf,
    inner: json_types::Package,
}

fn update_dependencies(
    deps: &mut DepsSet,
    names: &HashSet<String>,
    pre_release: bool,
) -> anyhow::Result<()> {
    for (package, version) in deps.iter_mut() {
        if names.contains(package) {
            let updated = if pre_release {
                try_bump_prerelease_version_req(version)?
            } else {
                try_bump_minor_version_req(version)?
            };

            println!("\t\t>>> updating '{package}' from {version} to {updated}");
            *version = updated
        }
    }
    Ok(())
}

impl ReleasePackage for PackageJson {
    fn type_name() -> &'static str {
        "package.json"
    }

    fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = find(Some(path.as_ref()))?;
        println!("\t>>> located the file at {}", path.display());

        let inner = json_types::Package::from_path(&path)?;
        Ok(PackageJson { path, inner })
    }

    fn save_changes(&mut self) -> anyhow::Result<()> {
        let file = std::fs::File::create(&self.path)?;
        Ok(serde_json::to_writer_pretty(file, &self.inner)?)
    }

    fn get_current_version(&self) -> anyhow::Result<Version> {
        self.inner.version.parse().map_err(Into::into)
    }

    fn set_version(&mut self, version: &Version) {
        self.inner.version = version.to_string()
    }

    fn update_nym_dependencies(
        &mut self,
        names: &HashSet<String>,
        pre_release: bool,
    ) -> anyhow::Result<()> {
        println!("\t>>> updating @nymproject dependencies...");
        update_dependencies(&mut self.inner.dependencies, names, pre_release)?;

        println!("\t>>> updating @nymproject peerDependencies...");
        update_dependencies(&mut self.inner.peer_dependencies, names, pre_release)?;

        println!("\t>>> updating @nymproject devDependencies...");
        update_dependencies(&mut self.inner.dev_dependencies, names, pre_release)?;

        println!("\t>>> updating @nymproject optionalDependencies...");
        update_dependencies(&mut self.inner.optional_dependencies, names, pre_release)?;

        println!("\t>>> updating @nymproject bundledDependencies...");
        update_dependencies(&mut self.inner.bundled_dependencies, names, pre_release)?;

        Ok(())
    }
}

pub fn find(specified: Option<&Path>) -> anyhow::Result<PathBuf> {
    match specified {
        Some(path)
            if fs::metadata(path)
                .with_context(|| "Failed to get cargo file metadata")?
                .is_file() =>
        {
            Ok(path.to_owned())
        }
        Some(path) => find_package_path(path),
        None => find_package_path(
            &env::current_dir().with_context(|| "Failed to get current directory")?,
        ),
    }
}

pub(crate) fn find_package_path(dir: &Path) -> anyhow::Result<PathBuf> {
    const MANIFEST_FILENAME: &str = "package.json";
    for path in dir.ancestors() {
        let manifest = path.join(MANIFEST_FILENAME);
        if std::fs::metadata(&manifest).is_ok() {
            return Ok(manifest);
        }
    }
    anyhow::bail!("Unable to find package.json for {}", dir.display());
}

// expected structure: `>=X.Y.Z-rc.W || ^X`
fn try_bump_minor_version_req(raw_req: &str) -> anyhow::Result<String> {
    let (req, major) = raw_req.split_once("||").context(format!(
        "'{raw_req}' is not a valid semver version requirement - we expect '`>=X.Y.Z-rc.W || ^X`'"
    ))?;
    let parsed_req = VersionReq::parse(req)?;
    let parsed_major = VersionReq::parse(major)?;
    if parsed_req.comparators.len() != 1 {
        bail!("wrong number of version requirements present in {parsed_req}")
    }

    let updated = VersionReq {
        comparators: vec![Comparator {
            op: parsed_req.comparators[0].op,
            major: parsed_req.comparators[0].major,
            minor: parsed_req.comparators[0].minor,
            patch: parsed_req.comparators[0].patch.map(|p| p + 1),
            pre: Prerelease::new("rc.0")?,
        }],
    };

    Ok(format!("{updated} || {parsed_major}"))
}

// expected structure: `>=X.Y.Z-rc.W || ^X`
fn try_bump_prerelease_version_req(raw_req: &str) -> anyhow::Result<String> {
    let (req, major) = raw_req.split_once("||").context(format!(
        "'{raw_req}' is not a valid semver version requirement - we expect '`>=X.Y.Z-rc.W || ^X`'"
    ))?;
    let parsed_req = VersionReq::parse(req)?;
    let parsed_major = VersionReq::parse(major)?;
    if parsed_req.comparators.len() != 1 {
        bail!("wrong number of version requirements present in {parsed_req}")
    }

    let updated = VersionReq {
        comparators: vec![Comparator {
            op: parsed_req.comparators[0].op,
            major: parsed_req.comparators[0].major,
            minor: parsed_req.comparators[0].minor,
            patch: parsed_req.comparators[0].patch,
            pre: try_bump_raw_prerelease(parsed_req.comparators[0].pre.as_str())?,
        }],
    };

    Ok(format!("{updated} || {parsed_major}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumping_version_req() {
        let cases = [
            (">=1.2.0-rc.0 || ^1", Some(">=1.2.1-rc.0 || ^1")),
            (">=1.2.0-rc.0 || 1", Some(">=1.2.1-rc.0 || ^1")),
            (">=1.2.0-rc.5 || ^1", Some(">=1.2.1-rc.0 || ^1")),
            (">=1.2.0-rc.5 || 1", Some(">=1.2.1-rc.0 || ^1")),
            (">=1.2.0-rc.0", None),
            ("1.0.0-rc.0", None),
        ];

        for (raw, expected) in cases {
            let updated = try_bump_minor_version_req(raw);
            if let Some(expected) = expected {
                assert_eq!(expected, updated.unwrap())
            } else {
                assert!(updated.is_err())
            }
        }
    }
}
