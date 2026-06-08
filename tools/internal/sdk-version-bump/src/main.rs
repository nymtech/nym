// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cargo::CargoPackage;
use crate::helpers::ReleasePackage;
use crate::json::PackageJson;
use clap::{Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};

mod cargo;
mod helpers;
mod json;
pub mod json_types;

fn default_root() -> PathBuf {
    env::current_dir().unwrap()
}

struct Summary {
    cargo_results: HashMap<String, anyhow::Result<()>>,

    json_results: HashMap<String, anyhow::Result<()>>,
}

impl Summary {
    fn new(
        cargo_results: HashMap<String, anyhow::Result<()>>,
        json_results: HashMap<String, anyhow::Result<()>>,
    ) -> Self {
        Summary {
            cargo_results,
            json_results,
        }
    }

    fn print(&self) {
        let cargo_ok = self.cargo_results.values().filter(|p| p.is_ok()).count();
        let json_ok = self.json_results.values().filter(|p| p.is_ok()).count();

        println!("SUMMARY");
        println!("inspected {} cargo packages", self.cargo_results.len());
        println!("updated {cargo_ok} cargo packages");
        for (package, res) in &self.cargo_results {
            if let Err(err) = res {
                println!(
                    "\t>>> ❌ FAILURE: cargo package '{package}' failed to get updated: {err}"
                );
            }
        }

        println!("inspected {} json packages", self.json_results.len());
        println!("updated {json_ok} json packages");
        for (package, res) in &self.json_results {
            if let Err(err) = res {
                println!("\t>>> ❌ FAILURE: json package '{package}' failed to get updated: {err}");
            }
        }
    }
}

#[derive(Parser)]
struct Args {
    #[arg(default_value=default_root().into_os_string())]
    root: PathBuf,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Will strip any `-rc.X` suffixes from the package versions
    RemoveSuffix {
        /// If set, only TypeScript (package.json) packages are touched and the
        /// Rust (Cargo.toml) packages are left alone. Use when shipping a
        /// TS-only release without bumping `wasm/smolmix`.
        #[arg(long)]
        ts_only: bool,
    },

    /// Will update the versions of all relevant packages from `X.Y.Z` into `X.Y.(Z+1)-rc.0`.
    /// It will also update the `@nymproject/...` dependencies from `">=X.Y.Z-rc.0 || ^X"` to `">=X.Y.(Z+1)-rc.0 || ^X"`
    BumpVersion {
        #[arg(long)]
        /// If enabled, the packages will only have their rc version bumped and the dependencies
        /// will get updated from `">=X.Y.Z-rc.W || ^X"` to `">=X.Y.Z-rc.(W+1) || ^X"`
        pre_release: bool,

        /// If set, only TypeScript (package.json) packages are bumped and the
        /// Rust (Cargo.toml) packages are left alone. Use this when the TS SDK
        /// needs a release without an underlying smolmix-wasm change. Default
        /// is parity (Rust + TS together).
        #[arg(long)]
        ts_only: bool,
    },
}

fn remove_suffix<Pkg: ReleasePackage>(root: &Path, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = root.join(path);
    println!(
        ">>> [{}] UPDATING PACKAGE {}: ",
        Pkg::type_name(),
        path.display()
    );

    if let Err(err) = { remove_suffix_inner::<Pkg>(path) } {
        println!("\t>>> ❌ FAILURE: {err}");
        Err(err)
    } else {
        println!("\t>>> ✅ SUCCESS");
        Ok(())
    }
}

fn remove_suffix_inner<Pkg: ReleasePackage>(path: impl AsRef<Path>) -> anyhow::Result<()> {
    println!("\t>>> opening the package file...");
    let mut package = Pkg::open(path)?;
    package.remove_suffix()?;

    println!("\t>>> saving the package file...");
    package.save_changes()
}

fn bump_version<Pkg: ReleasePackage>(
    root: &Path,
    path: impl AsRef<Path>,
    dependencies_to_update: &HashSet<String>,
    pre_release: bool,
) -> anyhow::Result<()> {
    let path = root.join(path);
    println!(
        ">>> [{}] UPDATING PACKAGE {}: ",
        Pkg::type_name(),
        path.display()
    );
    if let Err(err) = { bump_version_inner::<Pkg>(path, dependencies_to_update, pre_release) } {
        println!("\t>>> ❌ FAILURE: {err}");
        Err(err)
    } else {
        println!("\t>>> ✅ SUCCESS");
        Ok(())
    }
}

fn bump_version_inner<Pkg: ReleasePackage>(
    path: impl AsRef<Path>,
    dependencies_to_update: &HashSet<String>,
    pre_release: bool,
) -> anyhow::Result<()> {
    println!("\t>>> opening the package file...");
    let mut package = Pkg::open(path)?;
    package.bump_version(pre_release)?;

    package.update_nym_dependencies(dependencies_to_update, pre_release)?;

    println!("\t>>> saving the package file...");
    package.save_changes()
}

#[derive(Default)]
struct InternalPackages {
    root: PathBuf,
    cargo: HashSet<String>,
    json: HashSet<String>,

    /// All `@nymproject/...` dep names whose specifiers should get bumped
    /// alongside the source package versions.
    internal_js_dependencies: HashSet<String>,

    /// Subset of `internal_js_dependencies` whose source-of-truth is a Cargo
    /// crate (the wasm-pack-generated `pkg/package.json`). Under `--ts-only`,
    /// the Cargo crate isn't bumped, so its dep specifiers must NOT be bumped
    /// either — otherwise downstream package.jsons end up pinning a version
    /// that was never published.
    cargo_derived_js_dependencies: HashSet<String>,
}

impl InternalPackages {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        InternalPackages {
            root: root.as_ref().to_path_buf(),
            cargo: Default::default(),
            json: Default::default(),
            internal_js_dependencies: Default::default(),
            cargo_derived_js_dependencies: Default::default(),
        }
    }

    pub fn register_cargo<S: Into<String>>(&mut self, relative_path: S) {
        self.cargo.insert(relative_path.into());
    }

    pub fn register_json<S: Into<String>>(&mut self, relative_path: S) {
        self.json.insert(relative_path.into());
    }

    pub fn register_known_js_dependency<S: Into<String>>(&mut self, name: S) {
        self.internal_js_dependencies.insert(name.into());
    }

    /// Register a JS dep whose version is driven by a Cargo crate (and thus
    /// regenerated from `Cargo.toml` by wasm-pack). Implicitly also adds it to
    /// the general known-deps set so it gets bumped during a full release.
    pub fn register_cargo_derived_js_dependency<S: Into<String>>(&mut self, name: S) {
        let name = name.into();
        self.internal_js_dependencies.insert(name.clone());
        self.cargo_derived_js_dependencies.insert(name);
    }

    pub fn remove_suffix(&self, ts_only: bool) -> Summary {
        let mut cargo_results = HashMap::new();
        if !ts_only {
            for cargo_package in &self.cargo {
                let res = remove_suffix::<CargoPackage>(&self.root, cargo_package);
                cargo_results.insert(cargo_package.clone(), res);
            }
        }

        let mut json_results = HashMap::new();
        for package_json in &self.json {
            let res = remove_suffix::<PackageJson>(&self.root, package_json);
            json_results.insert(package_json.clone(), res);
        }

        Summary::new(cargo_results, json_results)
    }

    pub fn bump_version(&self, pre_release: bool, ts_only: bool) -> Summary {
        let mut cargo_results = HashMap::new();
        if !ts_only {
            for cargo_package in &self.cargo {
                let res = bump_version::<CargoPackage>(
                    &self.root,
                    cargo_package,
                    &Default::default(),
                    pre_release,
                );
                cargo_results.insert(cargo_package.clone(), res);
            }
        }

        // Under --ts-only the wasm/Cargo crates aren't getting a new version,
        // so we must NOT bump downstream `>=X.Y.Z || ^X` specifiers that point
        // at them either.
        let deps_to_update: HashSet<String> = if ts_only {
            self.internal_js_dependencies
                .difference(&self.cargo_derived_js_dependencies)
                .cloned()
                .collect()
        } else {
            self.internal_js_dependencies.clone()
        };

        let mut json_results = HashMap::new();
        for package_json in &self.json {
            let res = bump_version::<PackageJson>(
                &self.root,
                package_json,
                &deps_to_update,
                pre_release,
            );
            json_results.insert(package_json.clone(), res);
        }

        Summary::new(cargo_results, json_results)
    }
}

fn initialise_internal_packages<P: AsRef<Path>>(root: P) -> InternalPackages {
    let mut packages = InternalPackages::new(root);

    // cargo packages that will have their Cargo.toml modified.
    // `wasm/smolmix` replaces the legacy `wasm/mix-fetch`; smolmix is the
    // only wasm crate that ships TS-SDK-facing bindings now.
    packages.register_cargo("wasm/smolmix");
    packages.register_cargo("wasm/client");

    // js packages that will have their package.json modified.
    packages.register_json("nym-wallet");
    packages.register_json("sdk/typescript/docs");
    packages.register_json("sdk/typescript/examples/chat-app/parcel");
    packages.register_json("sdk/typescript/examples/chat-app/plain-html");
    packages.register_json("sdk/typescript/examples/chat-app/react-webpack-with-theme-example");
    packages.register_json("sdk/typescript/examples/mix-fetch/browser");
    packages.register_json("sdk/typescript/packages/mix-tunnel");
    packages.register_json("sdk/typescript/packages/mix-fetch");
    packages.register_json("sdk/typescript/packages/mix-dns");
    packages.register_json("sdk/typescript/packages/mix-websocket");
    packages.register_json("sdk/typescript/packages/mix-fetch/internal-dev");
    packages.register_json("sdk/typescript/packages/mix-fetch/internal-dev/parcel");
    packages.register_json("sdk/typescript/packages/nodejs-client");
    packages.register_json("sdk/typescript/packages/sdk");
    packages.register_json("sdk/typescript/packages/sdk-react");
    packages.register_json("sdk/typescript/codegen/contract-clients");

    // dependencies that will have their versions adjusted in the above packages.

    // WASM packages. Their package.json is generated by wasm-pack from the
    // corresponding Cargo crate, so they're tracked as cargo-derived: a
    // `--ts-only` bump leaves their dep specifiers alone.
    packages.register_cargo_derived_js_dependency("@nymproject/nym-client-wasm");
    packages.register_cargo_derived_js_dependency("@nymproject/smolmix-wasm");

    // smolmix-based TS SDKs.
    packages.register_known_js_dependency("@nymproject/mix-tunnel");
    packages.register_known_js_dependency("@nymproject/mix-fetch");
    packages.register_known_js_dependency("@nymproject/mix-dns");
    packages.register_known_js_dependency("@nymproject/mix-websocket");

    // Other SDK packages.
    packages.register_known_js_dependency("@nymproject/mui-theme");
    packages.register_known_js_dependency("@nymproject/react-components");
    packages.register_known_js_dependency("@nymproject/sdk");
    packages.register_known_js_dependency("@nymproject/sdk-react");
    packages.register_known_js_dependency("@nymproject/react");
    packages.register_known_js_dependency("@nymproject/nym-validator-client");
    packages.register_known_js_dependency("@nymproject/ts-sdk-docs");
    packages.register_known_js_dependency("@nymproject/contract-clients");

    packages
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let packages = initialise_internal_packages(args.root);

    let summary = match args.command {
        Commands::RemoveSuffix { ts_only } => packages.remove_suffix(ts_only),
        Commands::BumpVersion {
            pre_release,
            ts_only,
        } => packages.bump_version(pre_release, ts_only),
    };

    summary.print();

    Ok(())
}
