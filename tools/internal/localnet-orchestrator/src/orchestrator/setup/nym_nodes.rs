// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    CONTAINER_NETWORK_NAME, LOCALNET_NYM_BINARIES_IMAGE_NAME, NYM_NODE_HTTP_BEARER,
};
use crate::helpers::{
    monorepo_root_path, nym_api_cache_refresh_script, retrieve_current_nymnode_version,
};
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::account::Account;
use crate::orchestrator::container_helpers::{
    exec_container, get_container_ip_address, run_container, run_container_fut,
};
use crate::orchestrator::context::LocalnetContext;
use crate::orchestrator::nym_node::LocalnetNymNode;
use crate::orchestrator::state::LocalnetState;
use anyhow::{Context, bail};
use itertools::Itertools;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::{NodeId, RoleAssignment};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::models::NodeRefreshBody;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, MixnetSigningClient, PagedMixnetQueryClient,
};
use std::collections::BTreeMap;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tracing::info;

// for now just bond 3 mixnodes and 1 gateway
// in the future this could be made configurable
pub(crate) const GATEWAYS: usize = 1;
pub(crate) const MIXNODES: usize = 3;

pub(crate) struct Config {
    pub(crate) monorepo_root: Option<PathBuf>,
    pub(crate) custom_dns: Option<String>,
    pub(crate) open_proxy: bool,
}

pub(crate) struct NymNodeSetup {
    monorepo_root: PathBuf,
    custom_dns: Option<String>,
    open_proxy: bool,

    nodes: BTreeMap<NodeId, LocalnetNymNode>,
}

impl NymNodeSetup {
    pub(crate) fn new(config: Config) -> anyhow::Result<Self> {
        let monorepo_root = monorepo_root_path(config.monorepo_root)?;

        Ok(NymNodeSetup {
            monorepo_root,
            custom_dns: config.custom_dns,
            open_proxy: config.open_proxy,
            nodes: Default::default(),
        })
    }

    pub(crate) fn nym_binaries_image_tag(&self) -> anyhow::Result<String> {
        let version = retrieve_current_nymnode_version(&self.monorepo_root)?;
        Ok(format!("{LOCALNET_NYM_BINARIES_IMAGE_NAME}:{version}"))
    }

    fn next_node_id(&self) -> NodeId {
        let last_id = self
            .nodes
            .last_key_value()
            .map(|(k, _)| k.to_owned())
            .unwrap_or_default();

        // node ids are meant to start from 1
        last_id + 1
    }
}

impl LocalnetOrchestrator {
    fn mixnet_admin_signer(&self) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        let mnemonic = &self.localnet_details.contracts()?.mixnet.admin.mnemonic;
        self.signing_client(mnemonic)
    }

    fn node_signer(
        &self,
        node: &LocalnetNymNode,
    ) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        self.signing_client(&node.owner.mnemonic)
    }

    async fn validate_mixnet_contract_state(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("verifying mixnet contract state...", "🤔");

        let client = self.rpc_query_client()?;
        let fut = client.get_all_nymnode_bonds();
        let nym_nodes = ctx.async_with_progress(fut).await?;

        if !nym_nodes.is_empty() {
            bail!("attempted to bond nodes on a non-empty network")
        }

        // for good measure also check legacy nodes in case some tests were messing with those
        let fut = client.get_all_mixnode_bonds();
        let mixnodes = ctx.async_with_progress(fut).await?;

        if !mixnodes.is_empty() {
            bail!("attempted to bond nodes on a non-empty network")
        }

        let fut = client.get_all_gateways();
        let gateways = ctx.async_with_progress(fut).await?;

        if !gateways.is_empty() {
            bail!("attempted to bond nodes on a non-empty network")
        }

        Ok(())
    }

    async fn init_nym_node(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
        gateway: bool,
    ) -> anyhow::Result<()> {
        let account = Account::new();
        let node_id = ctx.data.next_node_id();

        fs::create_dir_all(self.storage.nym_node_container_data_directory(node_id))?;

        let name = self.nym_node_container_name(node_id);
        let nym_api = self.localnet_details.nym_api_endpoint()?;
        let nyxd = self.localnet_details.rpc_endpoint()?;
        let volume = self.nym_node_volume(node_id);
        let image_tag = ctx.data.nym_binaries_image_tag()?;
        let mnemonic = account.mnemonic.to_string();

        let mut args = vec![
            "--name",
            &name,
            "-v",
            &volume,
            "--network",
            CONTAINER_NETWORK_NAME,
            "--rm",
            &image_tag,
            "nym-node",
            "run",
            "--init-only",
            "--accept-operator-terms-and-conditions",
            "--unsafe-disable-replay-protection",
            // TODO: try to enable noise
            "--unsafe-disable-noise",
            // "--local" might actually not be needed. TBD
            "--local",
            "--http-access-token",
            NYM_NODE_HTTP_BEARER,
            // NOTE: this is a placeholder that will be changed once container is set to run
            // 'properly'
            "--public-ips",
            "1.2.3.4",
            "--mnemonic",
            &mnemonic,
            "--nym-api-urls",
            nym_api.as_str(),
            "--nyxd-urls",
            nyxd.as_str(),
            "--wireguard-userspace",
            "true",
        ];

        if gateway {
            // gw: --wireguard-enabled, --mode exit
            args.push("--wireguard-enabled");
            args.push("true");
            args.push("--mode");
            args.push("exit-gateway");
        } else {
            // not strictly needed
            args.push("--mode");
            args.push("mixnode");
        }

        run_container(ctx, args, ctx.data.custom_dns.clone()).await?;

        // 2. retrieve current identity key
        let private_key_path = self.storage.nym_node_ed25519_private_key_path(node_id);
        let private_key: ed25519::PrivateKey = nym_pemstore::load_key(&private_key_path)?;
        let keypair: ed25519::KeyPair = private_key.into();

        let details = LocalnetNymNode {
            id: node_id,
            gateway,
            identity: keypair,
            owner: account,
        };

        ctx.data.nodes.insert(node_id, details);

        Ok(())
    }

    async fn init_nym_nodes(&self, ctx: &mut LocalnetContext<NymNodeSetup>) -> anyhow::Result<()> {
        ctx.begin_next_step("initialising nym-nodes storage data...", "🔏");

        let total = MIXNODES + GATEWAYS;

        for i in 0..GATEWAYS {
            ctx.set_pb_prefix(format!("[{}/{total}]", i + 1));
            ctx.set_pb_message("initialising a gateway...");
            self.init_nym_node(ctx, true).await?;
        }

        for i in 0..MIXNODES {
            ctx.set_pb_prefix(format!("[{}/{total}]", GATEWAYS + i + 1));
            ctx.set_pb_message("initialising a mixnode...");
            self.init_nym_node(ctx, false).await?;
        }

        Ok(())
    }

    async fn transfer_bonding_tokens(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("sending initial tokens to node owners...", "💸");

        let mut receivers = Vec::new();

        // make sure to send minimum bond (100nym) + minimum amount needed for verifying zk-nyms
        for node in ctx.data.nodes.values() {
            receivers.push((node.owner.address(), ctx.unyms(1000_000000)));
        }

        let signing_client = self.master_signing_client()?;
        let send_fut =
            signing_client.send_multiple(receivers, "localnet nym-nodes token seeding", None);
        let res = ctx.async_with_progress(send_fut).await?;
        ctx.println(format!(
            "\t✅ sent tokens in transaction: {} (height {})",
            res.hash, res.height
        ));

        Ok(())
    }

    async fn start_nym_node_container(
        &self,
        ctx: &LocalnetContext<NymNodeSetup>,
        node: &LocalnetNymNode,
    ) -> anyhow::Result<()> {
        // 1. generate the .env file (we need valid contract addresses which can't be set via cli args)
        let content = self.localnet_details.env_file_content()?;
        let env_path = self
            .storage
            .nym_node_container_data_directory(node.id)
            .join("localnet.env");
        fs::write(env_path, &content)?;

        let mut args = Vec::new();

        let mut run_cmd = r#"CONTAINER_IP=$(hostname -i);
nym-node -c /root/.nym/nym-nodes/default-nym-node/localnet.env run --accept-operator-terms-and-conditions --public-ips ${CONTAINER_IP} --local --unsafe-disable-noise --wireguard-userspace true --unsafe-disable-replay-protection"#.to_string();

        if ctx.data.open_proxy {
            run_cmd.push_str(" --open-proxy=true");
        };

        args.push("--name".to_string());
        args.push(self.nym_node_container_name(node.id));
        args.push("-v".to_string());
        args.push(self.nym_node_volume(node.id));
        args.push("--network".to_string());
        args.push(CONTAINER_NETWORK_NAME.to_string());
        args.push("-d".to_string());
        args.push(ctx.data.nym_binaries_image_tag()?);
        args.push("sh".to_string());
        args.push("-c".to_string());
        args.push(run_cmd);

        // 2. start the container
        run_container(ctx, args, ctx.data.custom_dns.clone()).await?;

        Ok(())
    }

    async fn start_nym_nodes_containers(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("starting nym-nodes containers...", "🚀");

        let total = ctx.data.nodes.len();
        for (i, node) in ctx.data.nodes.values().enumerate() {
            ctx.set_pb_prefix(format!("[{}/{total}]", i + 1));
            ctx.set_pb_message("starting node container...");
            self.start_nym_node_container(ctx, node).await?;
        }

        Ok(())
    }

    async fn bond_nym_node(
        &self,
        ctx: &LocalnetContext<NymNodeSetup>,
        node: &LocalnetNymNode,
    ) -> anyhow::Result<()> {
        let container_name = self.nym_node_container_name(node.id);

        // 1. get node container ip
        let node_ip = get_container_ip_address(ctx, &container_name).await?;

        // 2. prepare bonding signature
        let payload = node.node_bonding_payload(node_ip);

        let stdout = exec_container(
            ctx,
            [
                &self.nym_node_container_name(node.id),
                "nym-node",
                "--no-banner",
                "sign",
                "--contract-msg",
                &payload,
                "--output",
                "json",
            ],
        )
        .await?;

        let details: serde_json::Value =
            serde_json::from_slice(&stdout).context("failed to parse signature details")?;
        let signature = details
            .get("encoded_signature")
            .context("failed to retrieve ed25519 signature")?;
        let signature_str = signature
            .as_str()
            .context("failed to retrieve ed25519 signature - not a string")?;
        let parsed_signature = signature_str
            .parse()
            .context("failed to parse ed25519 signature")?;

        // 3. call the contract with bonding message
        let client = self.node_signer(node)?;

        let fut = client.bond_nymnode(
            node.bonding_nym_node(node_ip),
            node.cost_params(),
            parsed_signature,
            node.pledge().into(),
            None,
        );
        let res = ctx.async_with_progress(fut).await?;
        ctx.println(format!(
            "\t node {} bonded in transaction: {}",
            node.identity.public_key(),
            res.transaction_hash,
        ));

        Ok(())
    }

    async fn bond_nym_nodes(&self, ctx: &mut LocalnetContext<NymNodeSetup>) -> anyhow::Result<()> {
        ctx.begin_next_step("starting nym-node bonding...", "⛓️");

        let total = ctx.data.nodes.len();
        for (i, node) in ctx.data.nodes.values().enumerate() {
            ctx.set_pb_prefix(format!("[{}/{total}]", i + 1));
            ctx.set_pb_message("bonding nym-node...");
            self.bond_nym_node(ctx, node).await?;
        }

        Ok(())
    }

    // that step is super flaky as nym-api might potentially pick up epoch changes and interject
    // first we reduce the epoch length to 1s to essentially force it to finish immediately
    // so that we could send all the rewarding txs to update the active set for the following epoch
    // finally we restore the expected epoch duration
    async fn assign_to_active_set(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("attempting to assign nodes to the active set...", "🔌");

        let rewarder = self.mixnet_rewarder_signing_client()?;
        let mixnet_admin = self.mixnet_admin_signer()?;

        let original_epoch = ctx
            .async_with_progress(rewarder.get_current_interval_details())
            .await?;

        // ideally we'd make **all** the changes within a single tx to reduce the time chunk in which
        // nym-api could cause us problems, but I'm not sure if we can guarantee correct ordering within
        // the mempool, so we spread it throughout 3 blocks instead.
        // but given ~1s block times, it should be fine
        ctx.println_with_emoji("\treducing epoch length...", "🙈");

        let fut = mixnet_admin.update_interval_config(
            original_epoch.interval.epochs_in_interval(),
            1,
            true,
            None,
        );
        ctx.async_with_progress(fut).await?;

        let exec_msgs = vec![
            // 1. start epoch transition
            (
                nym_mixnet_contract_common::ExecuteMsg::BeginEpochTransition {},
                vec![],
            ),
            // (nothing to reward)
            // 2. reconcile events
            (
                nym_mixnet_contract_common::ExecuteMsg::ReconcileEpochEvents { limit: None },
                vec![],
            ),
            // 3. assign (empty) exit
            (
                nym_mixnet_contract_common::ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::ExitGateway,
                        nodes: vec![],
                    },
                },
                vec![],
            ),
            // 4. assign entry
            (
                nym_mixnet_contract_common::ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::EntryGateway,
                        nodes: vec![1],
                    },
                },
                vec![],
            ),
            // 5. assign layer1
            (
                nym_mixnet_contract_common::ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Layer1,
                        nodes: vec![2],
                    },
                },
                vec![],
            ),
            // 6. assign layer2
            (
                nym_mixnet_contract_common::ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Layer2,
                        nodes: vec![3],
                    },
                },
                vec![],
            ),
            // 7. assign layer3
            (
                nym_mixnet_contract_common::ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Layer3,
                        nodes: vec![4],
                    },
                },
                vec![],
            ),
            // 8. assign (empty) standby
            (
                nym_mixnet_contract_common::ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Standby,
                        nodes: vec![],
                    },
                },
                vec![],
            ),
        ];

        ctx.println_with_emoji("\tadvancing epoch and assigning active set...", "🔌");
        let contract = &self.localnet_details.contracts()?.mixnet.address;
        let fut = rewarder.execute_multiple(
            contract,
            exec_msgs,
            None,
            "hacking our way through the mixnet contract!",
        );
        ctx.async_with_progress(fut).await?;

        ctx.println_with_emoji("\trestoring the original epoch length...", "🙈");
        let fut = mixnet_admin.update_interval_config(
            original_epoch.interval.epochs_in_interval(),
            original_epoch.interval.epoch_length_secs(),
            true,
            None,
        );
        ctx.async_with_progress(fut).await?;

        Ok(())
    }

    async fn force_refresh_nym_api_mixnet_and_describe_caches(
        &mut self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("refreshing nym-api state [mixnet/described]...", "⏳");

        // we need to do the following:
        // 1. call `/v1/utility/mixnet-cache-timestamp` to get current cache ts
        // 2. call `/v1/utility/refresh-mixnet-cache` to make the api start refreshing the cache
        // 3. poll `/v1/utility/mixnet-cache-timestamp` until the timestamp changes
        // 4. for each nym-node call `/v1/nym-nodes/refresh-described`

        let nym_api_endpoint = self.localnet_details.nym_api_endpoint()?;
        let cache_timestamp_route = nym_api_endpoint.join("/v1/utility/mixnet-cache-timestamp")?;
        let cache_refresh_route = nym_api_endpoint.join("/v1/utility/refresh-mixnet-cache")?;
        let refresh_described_route = nym_api_endpoint.join("/v1/nym-nodes/refresh-described")?;

        ctx.set_pb_prefix("[1/2]");
        ctx.set_pb_message("trying to force refresh mixnet contract cache...");

        let refresh_cache =
            nym_api_cache_refresh_script(cache_timestamp_route, cache_refresh_route);

        run_container(
            ctx,
            [
                "--network",
                CONTAINER_NETWORK_NAME,
                "--rm",
                &ctx.data.nym_binaries_image_tag()?,
                "sh",
                "-c",
                &refresh_cache,
            ],
            ctx.data.custom_dns.clone(),
        )
        .await?;

        ctx.set_pb_prefix("[2/2]");
        ctx.set_pb_message("trying to force refresh described cache...");

        let mut refresh_futures = JoinSet::new();
        for (i, node) in ctx.data.nodes.values().enumerate() {
            ctx.set_pb_prefix(format!("[{}/5]", i + 2));

            let refresh_request = NodeRefreshBody::new(node.identity.private_key());
            let refresh_request_json = serde_json::to_string(&refresh_request)?;

            let refresh_cmd = format!(
                r#"
set -euo pipefail

curl --fail-with-body -s -X POST {refresh_described_route} \
  -H "Content-Type: application/json" \
  -d '{refresh_request_json}' > /dev/null

"#
            );
            let image_tag = ctx.data.nym_binaries_image_tag()?;

            let future = run_container_fut([
                "--network".to_string(),
                CONTAINER_NETWORK_NAME.to_string(),
                "--rm".to_string(),
                image_tag,
                "sh".to_string(),
                "-c".to_string(),
                refresh_cmd,
            ]);

            refresh_futures.spawn(future);
        }

        for res in ctx.async_with_progress(refresh_futures.join_all()).await {
            res.context("cache refresh failure")?;
        }

        Ok(())
    }

    async fn insert_fake_network_monitor_runs(
        &self,
        ctx: &LocalnetContext<NymNodeSetup>,
        timestamps: Range<i64>,
    ) -> anyhow::Result<()> {
        let mut query = r#"
        BEGIN;

        INSERT INTO monitor_run(timestamp)
        VALUES
        "#
        .to_string();

        let values = timestamps
            .map(|result_ts| format!("({result_ts})"))
            .join(",\n");

        query.push_str(&values);
        query.push_str(";\nCOMMIT;");

        exec_container(
            ctx,
            [
                &self.nym_api_container_name(),
                "sqlite3",
                "/root/.nym/nym-api/default/data/db.sqlite",
                &query,
            ],
        )
        .await?;

        Ok(())
    }

    async fn insert_fake_network_monitor_results_for_node(
        &self,
        ctx: &LocalnetContext<NymNodeSetup>,
        node: &LocalnetNymNode,
        timestamps: Range<i64>,
    ) -> anyhow::Result<()> {
        // target result (for node_id = 1, identity = 'DwxvqcjzCfvBWECZcW38Zf767CoFkcqxPKzSJZC4nSG4'):
        /*
        BEGIN;

        INSERT OR IGNORE INTO gateway_details (node_id, identity)
        VALUES (1, 'DwxvqcjzCfvBWECZcW38Zf767CoFkcqxPKzSJZC4nSG4');

        INSERT INTO gateway_status (gateway_details_id, reliability, timestamp)
        VALUES ((SELECT id FROM gateway_details WHERE node_id = 1), 100, 1764782010);

        INSERT INTO gateway_status (gateway_details_id, reliability, timestamp)
        VALUES ((SELECT id FROM gateway_details WHERE node_id = 1), 100, 1764782011);

        COMMIT;
                 */

        let node_id = node.id;
        let identity = node.identity.public_key().to_base58_string();

        let insert_details = if node.gateway {
            "INSERT OR IGNORE INTO gateway_details (node_id, identity)"
        } else {
            "INSERT OR IGNORE INTO mixnode_details (mix_id, identity_key)"
        };
        let id_select = if node.gateway {
            format!("SELECT id FROM gateway_details WHERE node_id = {node_id}")
        } else {
            format!("SELECT id FROM mixnode_details WHERE mix_id = {node_id}")
        };
        let insert_status = if node.gateway {
            "INSERT INTO gateway_status (gateway_details_id, reliability, timestamp)\n"
        } else {
            "INSERT INTO mixnode_status (mixnode_details_id, reliability, timestamp)\n"
        };

        let mut query = format!(
            r#"
BEGIN;
{insert_details}
VALUES ({node_id}, '{identity}');

"#
        );

        for result_ts in timestamps {
            query.push_str(insert_status);
            query.push_str(&format!("VALUES (({id_select}), 100, {result_ts});\n"))
        }

        query.push_str("\nCOMMIT;");

        exec_container(
            ctx,
            [
                &self.nym_api_container_name(),
                "sqlite3",
                "/root/.nym/nym-api/default/data/db.sqlite",
                &query,
            ],
        )
        .await?;

        Ok(())
    }

    async fn insert_fake_network_monitor_results(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("inserting fake network monitor measurements...", "🥷");

        let now = OffsetDateTime::now_utc().unix_timestamp();
        let start = now - 100;
        let ts_range = start..now;

        ctx.set_pb_message("inserting base monitor results...");
        self.insert_fake_network_monitor_runs(ctx, ts_range.clone())
            .await?;

        for node in ctx.data.nodes.values() {
            ctx.set_pb_message(format!("inserting fake results for node {}...", node.id));
            self.insert_fake_network_monitor_results_for_node(ctx, node, ts_range.clone())
                .await?;
        }

        Ok(())
    }

    async fn force_refresh_nym_api_annotations_cache(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("refreshing nym-api state [annotations]...", "⏳");

        // we need to do the following:
        // 1. call `/v1/utility/node-annotations-cache-timestamp` to get current cache ts
        // 2. call `/v1/utility/refresh-node-annotations-cache` to make the api start refreshing the cache
        // 3. poll `/v1/utility/node-annotations-cache-timestamp` until the timestamp changes

        let nym_api_endpoint = self.localnet_details.nym_api_endpoint()?;
        let cache_timestamp_route =
            nym_api_endpoint.join("/v1/utility/node-annotations-cache-timestamp")?;
        let cache_refresh_route =
            nym_api_endpoint.join("/v1/utility/refresh-node-annotations-cache")?;

        let refresh_cache =
            nym_api_cache_refresh_script(cache_timestamp_route, cache_refresh_route);

        run_container(
            ctx,
            [
                "--network",
                CONTAINER_NETWORK_NAME,
                "--rm",
                &ctx.data.nym_binaries_image_tag()?,
                "sh",
                "-c",
                &refresh_cache,
            ],
            ctx.data.custom_dns.clone(),
        )
        .await?;

        Ok(())
    }

    async fn setup_gateway_forwarding(
        &self,
        ctx: &mut LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("setting up gateway forwarding rules", "🔀");

        // for now ignore ipv6 - they seem to be having their own set of issues
        const IP_RULES: &str = r#"
set -euo pipefail

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward
echo 1 > /proc/sys/net/ipv6/conf/all/forwarding

# Add NAT masquerade for outbound traffic
iptables -t nat -C POSTROUTING -o eth0 -j MASQUERADE 2>/dev/null || iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
ip6tables -t nat -C POSTROUTING -o eth0 -j MASQUERADE 2>/dev/null || ip6tables -t nat -A POSTROUTING -o eth0 -j MASQUERADE

# nymtun0
iptables  -C FORWARD -i nymtun0 -o eth0 -j ACCEPT 2>/dev/null || iptables  -I FORWARD 1 -i nymtun0 -o eth0 -j ACCEPT
iptables  -C FORWARD -i eth0 -o nymtun0 -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || iptables  -I FORWARD 2 -i eth0 -o nymtun0 -m state --state RELATED,ESTABLISHED -j ACCEPT

ip6tables  -C FORWARD -i nymtun0 -o eth0 -j ACCEPT 2>/dev/null || ip6tables  -I FORWARD 1 -i nymtun0 -o eth0 -j ACCEPT
ip6tables  -C FORWARD -i eth0 -o nymtun0 -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || ip6tables  -I FORWARD 2 -i eth0 -o nymtun0 -m state --state RELATED,ESTABLISHED -j ACCEPT

# nymwg
iptables  -C FORWARD -i nymwg -o eth0 -j ACCEPT 2>/dev/null || iptables  -I FORWARD 1 -i nymwg -o eth0 -j ACCEPT
iptables  -C FORWARD -i eth0 -o nymwg -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || iptables  -I FORWARD 2 -i eth0 -o nymwg -m state --state RELATED,ESTABLISHED -j ACCEPT

ip6tables  -C FORWARD -i nymwg -o eth0 -j ACCEPT 2>/dev/null || ip6tables  -I FORWARD 1 -i nymwg -o eth0 -j ACCEPT
ip6tables  -C FORWARD -i eth0 -o nymwg -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || ip6tables  -I FORWARD 2 -i eth0 -o nymwg -m state --state RELATED,ESTABLISHED -j ACCEPT

# DNS + ICMP
iptables -C INPUT -p icmp --icmp-type echo-request -j ACCEPT 2>/dev/null || iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT
iptables -C OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT 2>/dev/null || iptables -A OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT

iptables -C INPUT -p udp --dport 53 -j ACCEPT 2>/dev/null || iptables -A INPUT -p udp --dport 53 -j ACCEPT
iptables -C INPUT -p tcp --dport 53 -j ACCEPT 2>/dev/null || iptables -A INPUT -p tcp --dport 53 -j ACCEPT
        "#;

        for node in ctx.data.nodes.values() {
            if node.gateway {
                exec_container(
                    ctx,
                    [&self.nym_node_container_name(node.id), "sh", "-c", IP_RULES],
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn finalize_nym_nodes_setup(
        &mut self,
        mut ctx: LocalnetContext<NymNodeSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("persisting nym nodes details", "📝");

        let network_name = &self.localnet_details.human_name;

        for node in ctx.data.nodes.values() {
            self.storage
                .orchestrator()
                .save_nym_node_details(network_name, node)
                .await?;
        }

        self.state = LocalnetState::RunningNymNodes;
        Ok(())
    }

    pub(crate) async fn initialise_nym_nodes(&mut self, config: Config) -> anyhow::Result<()> {
        let setup = NymNodeSetup::new(config)?;
        let mut ctx = LocalnetContext::new(setup, 11, "\ninitialising nym nodes");

        // 0 check if we have to do anything
        if self.check_nym_node_containers_are_running(&ctx).await? {
            info!("nym node instances for this localnet are already running");
            return Ok(());
        }

        // no need to build containers as we're using the same one as nym-api which MUST be running

        // 1. ensure the current mixnet contract is empty, i.e. no nodes are bonded
        self.validate_mixnet_contract_state(&mut ctx).await?;

        // 2. run init on all nodes to create initial data
        self.init_nym_nodes(&mut ctx).await?;

        // 3. send tokens needed for bonding for all nodes
        self.transfer_bonding_tokens(&mut ctx).await?;

        // 4. start nym-nodes to get their proper container addresses to use for bonding
        self.start_nym_nodes_containers(&mut ctx).await?;

        // 5. perform the bonding of all the nodes
        self.bond_nym_nodes(&mut ctx).await?;

        // 6. hack the mixnet contract by forcing epoch transition to assign the new nodes to the active set
        self.assign_to_active_set(&mut ctx).await?;

        // 7. force refresh state of nym-api to fully recognise new nodes
        self.force_refresh_nym_api_mixnet_and_describe_caches(&mut ctx)
            .await?;

        // 8. insert some fake monitoring results to bump up nodes performance without waiting
        // for NM to go around
        self.insert_fake_network_monitor_results(&mut ctx).await?;

        // 9. force refresh node annotations to update node scores served
        self.force_refresh_nym_api_annotations_cache(&mut ctx)
            .await?;

        // 10. set forwarding rules on gateways (at this point the nodes must have been running
        // for sufficiently long for the relevant interfaces to have been created)
        self.setup_gateway_forwarding(&mut ctx).await?;

        // 11. persist relevant information and update local state
        self.finalize_nym_nodes_setup(ctx).await?;

        Ok(())
    }
}
