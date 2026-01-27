// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod v1_1_33 {
    use crate::config::disk_persistence::old_v1_1_33::CommonClientPathsV1_1_33;
    use crate::config::disk_persistence::CommonClientPaths;
    use crate::config::old_config_v1_1_33::OldGatewayEndpointConfigV1_1_33;
    use crate::error::ClientCoreError;

    pub async fn migrate_gateway_details(
        _old_storage_paths: &CommonClientPathsV1_1_33,
        _new_storage_paths: &CommonClientPaths,
        _preloaded_config: Option<OldGatewayEndpointConfigV1_1_33>,
    ) -> Result<(), ClientCoreError> {
        Err(ClientCoreError::UnsupportedMigration(
            "migration of legacy keys has been removed and is no longer supported".into(),
        ))
    }
}
