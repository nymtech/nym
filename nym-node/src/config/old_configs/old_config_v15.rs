// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{Config, Nyx};
use crate::error::NymNodeError;
use nym_config::defaults::mainnet;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument, warn};
use url::Url;

#[allow(unused_imports)]
pub use unchanged_v15_types::*;

// (while some of those are technically unused, they might be needed in future migrations,
// thus allow them to exist)
#[allow(dead_code)]
pub mod unchanged_v15_types {
    use crate::config::old_configs::old_config_v14::NyxV14;
    use crate::config::persistence::NymNodePaths;
    use crate::config::{
        Debug, DirectoryConfig, GatewayTasksConfig, Host, Http, LpConfig, MetricsConfig, Mixnet,
        NodeModes, ServiceProvidersConfig, Verloc, Wireguard,
    };
    use nym_bin_common::logging::LoggingSettings;

    pub type NyxV15 = NyxV14;

    // these sections are still what the current config holds. Whichever of them changes next has
    // to leave a copy of its present shape here.
    pub type NodeModesV15 = NodeModes;
    pub type HostV15 = Host;
    pub type MixnetV15 = Mixnet;
    pub type NymNodePathsV15 = NymNodePaths;
    pub type HttpV15 = Http;
    pub type VerlocV15 = Verloc;
    pub type WireguardV15 = Wireguard;
    pub type LpConfigV15 = LpConfig;
    pub type GatewayTasksConfigV15 = GatewayTasksConfig;
    pub type ServiceProvidersConfigV15 = ServiceProvidersConfig;
    pub type MetricsConfigV15 = MetricsConfig;
    pub type DirectoryConfigV15 = DirectoryConfig;
    pub type LoggingSettingsV15 = LoggingSettings;
    pub type DebugV15 = Debug;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV15 {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    /// Human-readable ID of this particular node.
    pub id: String,

    /// Current modes of this nym-node.
    pub modes: NodeModesV15,

    pub host: HostV15,

    pub mixnet: MixnetV15,

    /// Storage paths to persistent nym-node data, such as its long term keys.
    pub storage_paths: NymNodePathsV15,

    #[serde(default)]
    pub nyx: NyxV15,

    #[serde(default)]
    pub http: HttpV15,

    #[serde(default)]
    pub verloc: VerlocV15,

    pub wireguard: WireguardV15,

    #[serde(default)]
    pub lp: LpConfigV15,

    #[serde(alias = "entry_gateway")]
    pub gateway_tasks: GatewayTasksConfigV15,

    #[serde(alias = "exit_gateway")]
    pub service_providers: ServiceProvidersConfigV15,

    #[serde(default)]
    pub metrics: MetricsConfigV15,

    #[serde(default)]
    pub directory: DirectoryConfigV15,

    #[serde(default)]
    pub logging: LoggingSettingsV15,

    #[serde(default)]
    pub debug: DebugV15,
}

impl ConfigV15 {
    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: ConfigV15 =
            read_config_from_toml_file(path).map_err(|source| NymNodeError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }
}

/// Put a node that only uses the well-known mainnet endpoints on the lite one, with the main one
/// as its fallback. Any other set of addresses is left alone.
fn upgrade_nyx(old: NyxV15) -> Nyx {
    // SAFETY:
    // our hardcoded values should always be valid
    #[allow(clippy::expect_used)]
    let lite: Url = mainnet::NYXD_QUERY_LITE
        .parse()
        .expect("invalid default nyxd lite URL");
    #[allow(clippy::expect_used)]
    let main: Url = mainnet::NYXD_URL.parse().expect("invalid default nyxd URL");

    let only_known_mainnet_endpoints =
        !old.nyxd_urls.is_empty() && old.nyxd_urls.iter().all(|url| url == &lite || url == &main);

    if !only_known_mainnet_endpoints {
        return Nyx {
            nyxd_websocket_url: old.nyxd_websocket_url,
            nyxd_urls: old.nyxd_urls,
            nyxd_fallback_urls: Vec::new(),
        };
    }

    if old.nyxd_urls == [lite.clone()] {
        info!("nyxd_fallback_urls set to {main}");
    } else {
        let previous: Vec<_> = old.nyxd_urls.iter().map(Url::as_str).collect();
        warn!(
            "nyxd_urls changed from {previous:?} to {lite}: {main} is now only queried as a fallback (nyxd_fallback_urls)"
        );
    }

    Nyx {
        nyxd_websocket_url: old.nyxd_websocket_url,
        nyxd_urls: vec![lite],
        nyxd_fallback_urls: vec![main],
    }
}

#[instrument(skip_all)]
pub async fn try_upgrade_config_v15<P: AsRef<Path>>(
    path: P,
    prev_config: Option<ConfigV15>,
) -> Result<Config, NymNodeError> {
    debug!("attempting to load v15 config...");

    let old_cfg = if let Some(prev_config) = prev_config {
        prev_config
    } else {
        ConfigV15::read_from_path(&path)?
    };

    info!("migrating the old config (v15)...");

    let cfg = Config {
        save_path: old_cfg.save_path,
        id: old_cfg.id,
        modes: old_cfg.modes,
        host: old_cfg.host,
        mixnet: old_cfg.mixnet,
        storage_paths: old_cfg.storage_paths,
        // \/ CHANGED
        nyx: upgrade_nyx(old_cfg.nyx),
        // /\ CHANGED
        http: old_cfg.http,
        verloc: old_cfg.verloc,
        wireguard: old_cfg.wireguard,
        lp: old_cfg.lp,
        gateway_tasks: old_cfg.gateway_tasks,
        service_providers: old_cfg.service_providers,
        metrics: old_cfg.metrics,
        directory: old_cfg.directory,
        logging: old_cfg.logging,
        debug: old_cfg.debug,
    };
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use crate::config::upgrade_helpers::try_load_current_config;
    use nym_config::NymConfigTemplate;

    const LITE: &str = "https://blockstream.nymtech.net/";
    const MAIN: &str = "https://rpc.nymtech.net/";
    const OWN: &str = "https://nyxd.operator.example/";
    const WEBSOCKET: &str = "wss://rpc.nymtech.net/websocket";

    fn urls(raw: &[&str]) -> Vec<Url> {
        raw.iter()
            .map(|url| url.parse().expect("the test url is valid"))
            .collect()
    }

    fn upgraded(nyxd_urls: &[&str]) -> Nyx {
        upgrade_nyx(NyxV15 {
            nyxd_websocket_url: WEBSOCKET.parse().expect("the test url is valid"),
            nyxd_urls: urls(nyxd_urls),
        })
    }

    // Every deployed mainnet node queries the lite endpoint alone. Without the main one behind it
    // the watcher has nowhere to turn when its lite backend fails.
    #[test]
    fn a_node_on_the_lite_mainnet_endpoint_gains_the_main_one_as_fallback() {
        let nyx = upgraded(&[LITE]);

        assert_eq!(urls(&[LITE]), nyx.nyxd_urls);
        assert_eq!(urls(&[MAIN]), nyx.nyxd_fallback_urls);
    }

    // Operators who pointed their node at the main endpoint to work around the lite one go back
    // to the lite one, keeping the endpoint they chose as the fallback.
    #[test]
    fn a_node_on_the_main_mainnet_endpoint_goes_back_to_the_lite_one() {
        for nyxd_urls in [&[MAIN][..], &[MAIN, LITE], &[LITE, MAIN]] {
            let nyx = upgraded(nyxd_urls);

            assert_eq!(urls(&[LITE]), nyx.nyxd_urls, "from {nyxd_urls:?}");
            assert_eq!(urls(&[MAIN]), nyx.nyxd_fallback_urls, "from {nyxd_urls:?}");
        }
    }

    // Anything but the two well-known mainnet endpoints is a deliberate setup, or another network
    // altogether: neither may be handed mainnet addresses.
    #[test]
    fn a_node_with_endpoints_of_its_own_is_left_alone() {
        for nyxd_urls in [&[OWN][..], &[LITE, OWN], &[OWN, MAIN], &[]] {
            let nyx = upgraded(nyxd_urls);

            assert_eq!(urls(nyxd_urls), nyx.nyxd_urls, "from {nyxd_urls:?}");
            assert!(nyx.nyxd_fallback_urls.is_empty(), "from {nyxd_urls:?}");
        }
    }

    // The websocket has neither a fallback nor a way to recover yet, so it stays wherever the
    // operator put it.
    #[test]
    fn the_websocket_endpoint_is_never_touched() {
        for nyxd_urls in [&[LITE][..], &[MAIN], &[OWN]] {
            assert_eq!(WEBSOCKET, upgraded(nyxd_urls).nyxd_websocket_url.as_str());
        }
    }

    // A file written by the previous release has to be refused as the current format, upgraded
    // and written back with the fallback in it. Otherwise no deployed node would ever get one.
    #[tokio::test]
    async fn a_config_file_from_the_previous_release_is_upgraded_on_load() {
        let dir = std::env::temp_dir().join(format!(
            "nym-node-v15-upgrade-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is past the epoch")
                .as_nanos()
        ));
        let config_path = dir.join("config.toml");

        let mut cfg = ConfigBuilder::new("test-node".to_string(), config_path.clone(), dir.clone())
            .build()
            .expect("the default config builds");
        cfg.nyx.nyxd_urls = urls(&[MAIN]);
        cfg.nyx.nyxd_fallback_urls = Vec::new();

        // the previous release wrote the very same file, just without the fallback entry
        let current = cfg.format_to_string();
        let entry_start = current
            .find("nyxd_fallback_urls = [")
            .expect("the current format has the fallback entry");
        let entry_end = entry_start
            + current[entry_start..]
                .find(']')
                .expect("the fallback entry is closed")
            + 1;
        let previous = format!("{}{}", &current[..entry_start], &current[entry_end..]);

        std::fs::create_dir_all(&dir).expect("the temp directory is writable");
        std::fs::write(&config_path, previous).expect("the temp directory is writable");

        let loaded = try_load_current_config(&config_path)
            .await
            .expect("the previous format must upgrade");
        let rewritten = std::fs::read_to_string(&config_path).expect("the config was saved");
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(urls(&[LITE]), loaded.nyx.nyxd_urls);
        assert_eq!(urls(&[MAIN]), loaded.nyx.nyxd_fallback_urls);
        assert!(
            rewritten.contains("nyxd_fallback_urls = ["),
            "the upgraded config was not written back:\n{rewritten}"
        );
    }
}
