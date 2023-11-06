// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cargo::CargoPackage;
use crate::helpers::ReleasePackage;
use crate::json::PackageJson;
use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

mod cargo;
mod helpers;
mod json;
pub mod json_types;

fn default_root() -> PathBuf {
    env::current_dir().unwrap()
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
    RemoveSuffix,

    /// Will update the versions of all relevant packages from `X.Y.Z` into `X.Y.(Z+1)-rc.0`.
    /// It will also update the `@nymproject/...` dependencies from `">=X.Y.Z-rc.0 || ^X"` to `">=X.Y.(Z+1)-rc.0 || ^X"`
    BumpVersion {
        #[arg(long)]
        /// If enabled, the packages will only have their rc version bumped and the dependencies won't get updated at all
        pre_release: bool,
    },
}

fn remove_suffix<Pkg: ReleasePackage>(root: &Path, path: impl AsRef<Path>) {
    let path = root.join(path);
    println!(
        ">>> [{}] UPDATING PACKAGE {}: ",
        Pkg::type_name(),
        path.display()
    );

    if let Err(err) = { remove_suffix_inner::<Pkg>(path) } {
        println!("\t>>> ❌ FAILURE: {err}");
    } else {
        println!("\t>>> ✅ SUCCESS");
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
) {
    let path = root.join(path);
    println!(
        ">>> [{}] UPDATING PACKAGE {}: ",
        Pkg::type_name(),
        path.display()
    );
    if let Err(err) = { bump_version_inner::<Pkg>(path, dependencies_to_update, pre_release) } {
        println!("\t>>> ❌ FAILURE: {err}");
    } else {
        println!("\t>>> ✅ SUCCESS");
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

    if !pre_release {
        package.update_nym_dependencies(dependencies_to_update)?;
    }

    println!("\t>>> saving the package file...");
    package.save_changes()
}

#[derive(Default)]
struct InternalPackages {
    root: PathBuf,
    cargo: HashSet<String>,
    json: HashSet<String>,

    internal_js_dependencies: HashSet<String>,
}

impl InternalPackages {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        InternalPackages {
            root: root.as_ref().to_path_buf(),
            cargo: Default::default(),
            json: Default::default(),
            internal_js_dependencies: Default::default(),
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

    pub fn remove_suffix(&self) {
        for cargo_package in &self.cargo {
            remove_suffix::<CargoPackage>(&self.root, cargo_package);
        }

        for package_json in &self.json {
            remove_suffix::<PackageJson>(&self.root, package_json);
        }
    }

    pub fn bump_version(&self, pre_release: bool) {
        for cargo_package in &self.cargo {
            bump_version::<CargoPackage>(
                &self.root,
                cargo_package,
                &Default::default(),
                pre_release,
            );
        }

        for package_json in &self.json {
            bump_version::<PackageJson>(
                &self.root,
                package_json,
                &self.internal_js_dependencies,
                pre_release,
            );
        }
    }
}

fn initialise_internal_packages<P: AsRef<Path>>(root: P) -> InternalPackages {
    let mut packages = InternalPackages::new(root);

    // cargo packages that will have their Cargo.toml modified
    packages.register_cargo("wasm/mix-fetch");
    packages.register_cargo("wasm/client");
    packages.register_cargo("wasm/node-tester");
    packages.register_cargo("wasm/full-nym-wasm");
    packages.register_cargo("nym-browser-extension/storage");

    // js packages that will have their package.json modified
    packages.register_json("nym-wallet");
    packages.register_json("sdk/typescript/docs");
    packages.register_json("sdk/typescript/examples/chat-app/parcel");
    packages.register_json("sdk/typescript/examples/chat-app/plain-html");
    packages.register_json("sdk/typescript/examples/chat-app/react-webpack-with-theme-example");
    packages.register_json("sdk/typescript/examples/chrome-extension");
    packages.register_json("sdk/typescript/examples/firefox-extension");
    packages.register_json("sdk/typescript/examples/mix-fetch/browser");
    packages.register_json("sdk/typescript/examples/node-tester/parcel");
    packages.register_json("sdk/typescript/examples/node-tester/plain-html");
    packages.register_json("sdk/typescript/examples/node-tester/react");
    packages.register_json("sdk/typescript/packages/mix-fetch");
    packages.register_json("sdk/typescript/packages/mix-fetch-node");
    packages.register_json("sdk/typescript/packages/mix-fetch/internal-dev");
    packages.register_json("sdk/typescript/packages/mix-fetch/internal-dev/parcel");
    packages.register_json("sdk/typescript/packages/node-tester");
    packages.register_json("sdk/typescript/packages/nodejs-client");
    packages.register_json("sdk/typescript/packages/sdk");
    packages.register_json("sdk/typescript/packages/sdk-react");
    packages.register_json("sdk/typescript/codegen/contract-clients");

    // dependencies that will have their versions adjusted in the above packages
    packages.register_known_js_dependency("@nymproject/mix-fetch");
    packages.register_known_js_dependency("@nymproject/mix-fetch-full-fat");
    packages.register_known_js_dependency("@nymproject/mui-theme");
    packages.register_known_js_dependency("@nymproject/node-tester");
    packages.register_known_js_dependency("@nymproject/react-components");
    packages.register_known_js_dependency("@nymproject/sdk");
    packages.register_known_js_dependency("@nymproject/sdk-full-fat");
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

    match args.command {
        Commands::RemoveSuffix => packages.remove_suffix(),
        Commands::BumpVersion { pre_release } => packages.bump_version(pre_release),
    }

    Ok(())
}
