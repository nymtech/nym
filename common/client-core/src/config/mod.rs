// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_client_core_config_types::disk_persistence;
pub use nym_client_core_config_types::old::{
    old_config_v1_1_13, old_config_v1_1_20, old_config_v1_1_20_2, old_config_v1_1_30,
    old_config_v1_1_33,
};
pub use nym_client_core_config_types::*;



include!(concat!(env!("OUT_DIR"), "/default.rs"));


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_default() {
        let cfg = Config::new("","");
        println!("{}", toml::to_string(&cfg).unwrap());
    }
 
    #[test]
    fn bootstrapped_config() {
        #[cfg(not(feature = "enable-cfg"))]
        {
            let config = new_bootstrapped("id1", "v0.0.0");
            assert_eq!(config.client.id, "id1");
            assert_eq!(config.client.version, "v0.0.0");
            assert_eq!(config.client.disabled_credentials_mode, true);

            assert_eq!(config.debug.topology.use_extended_topology, false);
            assert_eq!(config.debug.stats_reporting.enabled, true);
        }
        #[cfg(feature = "enable-cfg")]
        {
            let config = new_bootstrapped("id2", "v0.0.0-beta");
            assert_eq!(config.client.id, "id2");
            assert_eq!(config.client.version, "v0.0.0-beta");
            assert_eq!(config.client.disabled_credentials_mode, true);
        }
    }
}
