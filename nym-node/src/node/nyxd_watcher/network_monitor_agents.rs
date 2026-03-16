// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::routing_filter::network_filter::DeclaredNetworkMonitors;
use async_trait::async_trait;
use nym_validator_client::nyxd::cosmwasm::MsgExecuteContract;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::ExecuteMsg;
use nym_validator_client::nyxd::{AccountId, Any, Msg, Name};
use nyxd_scraper_shared::error::ScraperError;
use nyxd_scraper_shared::{DecodedMessage, MsgModule, ParsedTransactionDetails, parse_msg};
use tracing::error;

pub(crate) struct NetworkMonitorAgentsModule {
    pub(crate) contract_address: AccountId,
    pub(crate) network_monitors: DeclaredNetworkMonitors,
}

impl NetworkMonitorAgentsModule {
    pub(crate) fn new(
        contract_address: AccountId,
        network_monitors: DeclaredNetworkMonitors,
    ) -> Self {
        Self {
            contract_address,
            network_monitors,
        }
    }
}

#[async_trait]
impl MsgModule for NetworkMonitorAgentsModule {
    // we're only interested in contract messages
    fn type_url(&self) -> String {
        <MsgExecuteContract as Msg>::Proto::type_url()
    }

    async fn handle_msg(
        &mut self,
        _: usize,
        msg: &Any,
        _: &DecodedMessage,
        tx: &ParsedTransactionDetails,
    ) -> Result<(), ScraperError> {
        // don't process failed transactions
        if !tx.tx_result.code.is_ok() {
            return Ok(());
        }

        // propagate error as this is a critical failure indicating our code is incompatible with
        // the current CometBFT schema so parsing can't proceed
        let execute_msg: MsgExecuteContract = parse_msg(msg)?;

        // not the contract we're interested in
        if execute_msg.contract != self.contract_address {
            return Ok(());
        }

        let exec_msg: ExecuteMsg = match serde_json::from_slice(&execute_msg.msg) {
            Ok(msg) => msg,
            Err(err) => {
                // do NOT propagate error. this just means the contract might have updated.
                // further block processing should continue
                error!(
                    "failed to parse out network monitors contract ExecuteMsg - has the contact schema been updated? error was: {err}"
                );
                return Ok(());
            }
        };

        match exec_msg {
            ExecuteMsg::AuthoriseNetworkMonitor { address } => {
                self.network_monitors.add_known(address)
            }
            ExecuteMsg::RevokeNetworkMonitor { address } => {
                self.network_monitors.remove_known(address)
            }
            ExecuteMsg::RevokeAllNetworkMonitors => self.network_monitors.reset(),

            // we're not interested in those messages
            ExecuteMsg::UpdateAdmin { .. }
            | ExecuteMsg::AuthoriseNetworkMonitorOrchestrator { .. }
            | ExecuteMsg::RevokeNetworkMonitorOrchestrator { .. } => {}
        }

        Ok(())
    }
}
