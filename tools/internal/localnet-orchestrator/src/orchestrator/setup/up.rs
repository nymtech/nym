// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::setup::{cosmwasm_contracts, nym_api, nym_nodes, nyxd};

pub(crate) struct Config {
    pub(crate) nyxd_setup: nyxd::Config,
    pub(crate) contracts_setup: cosmwasm_contracts::Config,
    pub(crate) nym_api_setup: nym_api::Config,
    pub(crate) nym_nodes_setup: nym_nodes::Config,
}

impl LocalnetOrchestrator {
    pub(crate) async fn start_localnet(&mut self, config: Config) -> anyhow::Result<()> {
        // 1. start nyxd
        self.initialise_nyxd(config.nyxd_setup).await?;

        // 2. upload contracts
        self.initialise_contracts(config.contracts_setup).await?;

        // 3. start nym-api (and setup DKG)
        self.initialise_nym_api(config.nym_api_setup).await?;

        // 4. launch nym-nodes
        self.initialise_nym_nodes(config.nym_nodes_setup).await?;

        // ???

        // 5. profit!
        Ok(())
    }
}
