// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use nym_network_defaults::NymNetworkDetails;
use nym_task::TaskManager;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use tendermint_rpc::WebSocketClient;
use tracing::info;

mod tasks;

pub struct Rewarder {
    config: Config,
}

impl Rewarder {
    pub fn new(config: Config) -> Self {
        Rewarder { config }
    }

    pub async fn run(mut self) -> Result<(), NymRewarderError> {
        info!("Starting nym validators rewarder");

        // setup shutdowns
        let task_manager = TaskManager::new(5);

        //
        //

        // let client_config =
        //     nyxd::Config::try_from_nym_network_details(&NymNetworkDetails::new_from_env())?;
        //
        // let client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
        //     client_config,
        //     self.config.base.upstream_nyxd.as_str(),
        //     // note: the clone here is fine as the mnemonic itself implements ZeroizeOnDrop
        //     self.config.base.mnemonic.clone(),
        // )?;

        let (client, driver) = WebSocketClient::new("https://rpc.nymtech.net/")
            .await
            .unwrap();
        /*
        /// #[tokio::main]
        /// async fn main() {
        ///     let (client, driver) = WebSocketClient::new("ws://127.0.0.1:26657/websocket")
        ///         .await
        ///         .unwrap();
        ///     let driver_handle = tokio::spawn(async move { driver.run().await });
        ///
        ///     // Standard client functionality
        ///     let tx = format!("some-key=some-value");
        ///     client.broadcast_tx_async(Transaction::from(tx.into_bytes())).await.unwrap();
        ///
        ///     // Subscription functionality
        ///     let mut subs = client.subscribe(EventType::NewBlock.into())
        ///         .await
        ///         .unwrap();
        ///
        ///     // Grab 5 NewBlock events
        ///     let mut ev_count = 5_i32;
        ///
        ///     while let Some(res) = subs.next().await {
        ///         let ev = res.unwrap();
        ///         println!("Got event: {:?}", ev);
        ///         ev_count -= 1;
        ///         if ev_count < 0 {
        ///             break;
        ///         }
        ///     }
        ///
        ///     // Signal to the driver to terminate.
        ///     client.close().unwrap();
        ///     // Await the driver's termination to ensure proper connection closure.
        ///     let _ = driver_handle.await.unwrap();
        /// }
        /// ```
         */

        /*
           task 1:
           on timer:
               - go to DKG contract
               - get all coconut signers
               - for each of them get the info, verify, etc

           task 2:
           on timer (or maybe per block?):
               - query abci endpoint for VP
               - also maybe missed blocks, etc

        */

        todo!()
    }
}
