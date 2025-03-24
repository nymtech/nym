// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod client;

use crate::config::upgrade_helpers::try_load_current_config;
use crate::node::NymNode;
use crate::throughput_test::client::ThroughputTestingClient;
use std::path::PathBuf;
use std::time::Duration;
use tokio::runtime;
use tokio::runtime::Runtime;

pub struct ThroughputTest {
    node_runtime: Runtime,
    clients_runtime: Runtime,
}

impl ThroughputTest {
    fn new(senders: usize) -> anyhow::Result<Self> {
        Ok(ThroughputTest {
            node_runtime: runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("nym-node-pool")
                .build()?,
            clients_runtime: runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(senders)
                .thread_name("testing-clients-pool")
                .build()?,
        })
    }

    fn prepare_nymnode(&self, config_path: PathBuf) -> anyhow::Result<NymNode> {
        self.node_runtime.block_on(async {
            let config = try_load_current_config(config_path).await?;

            let nym_node = NymNode::new(config).await?;
            Ok(nym_node)
        })
    }
}

pub struct NodeWrapper {
    //
}

pub struct ClientsWrapper {}

pub(crate) fn test_mixing_throughput(config_path: PathBuf, senders: usize) -> anyhow::Result<()> {
    let mut tester = ThroughputTest::new(senders)?;

    let nym_node = tester.prepare_nymnode(config_path)?;
    let listener = nym_node.config().mixnet.bind_address;
    let sphinx_keys = nym_node.x25519_sphinx_keys();

    // for now try one client
    let token = nym_node.shutdown_token("dummy-load-client");

    tester.clients_runtime.spawn(async move {
        println!("client create start");
        let c = ThroughputTestingClient::try_create(
            Duration::from_millis(1000),
            10,
            &sphinx_keys,
            listener,
            token,
        )
        .await
        .expect("todo: expect");
        println!("created");
        c.run().await;
    });

    let node_future = tester
        .node_runtime
        .block_on(async move { nym_node.run_minimal_mixnet_processing().await });

    // before we can create the clients, the mix processing has to be running
    // (as clients will attempt to establish connection)

    println!("listening on {}", listener);

    Ok(())
}
