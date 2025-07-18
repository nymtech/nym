// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::{ProgressCtx, ProgressTracker, RunCommands};
use crate::manager::network::LoadedNetwork;
use crate::manager::node::NymNode;
use crate::manager::NetworkManager;
use console::style;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::RoleAssignment;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, MixnetSigningClient, PagedMixnetQueryClient,
};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use serde::{Deserialize, Serialize};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use time::OffsetDateTime;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::error;
use zeroize::Zeroizing;

struct LocalNodesCtx<'a> {
    nym_node_binary: PathBuf,

    progress: ProgressTracker,
    network: &'a LoadedNetwork,
    admin: DirectSigningHttpRpcNyxdClient,

    mix_nodes: Vec<NymNode>,
    gateways: Vec<NymNode>,
}

impl ProgressCtx for LocalNodesCtx<'_> {
    fn progress_tracker(&self) -> &ProgressTracker {
        &self.progress
    }
}

impl<'a> LocalNodesCtx<'a> {
    fn nym_node_id(&self, node: &NymNode) -> String {
        format!("{}-{}", self.network.name, node.owner.address)
    }

    fn new(
        nym_node_binary: PathBuf,
        network: &'a LoadedNetwork,
        admin_mnemonic: bip39::Mnemonic,
    ) -> Result<Self, NetworkManagerError> {
        let progress = ProgressTracker::new(format!(
            "\n🚀 setting up new local nym-nodes for network '{}' over {}",
            network.name, network.rpc_endpoint
        ));

        Ok(LocalNodesCtx {
            nym_node_binary,
            network,
            admin: DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
                network.client_config()?,
                network.rpc_endpoint.as_str(),
                admin_mnemonic,
            )?,
            mix_nodes: Vec::new(),
            progress,
            gateways: Vec::new(),
        })
    }

    fn signing_node_owner(
        &self,
        node: &NymNode,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            self.network.client_config()?,
            self.network.rpc_endpoint.as_str(),
            node.owner.mnemonic.clone(),
        )?)
    }

    fn signing_rewarder(&self) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            self.network.client_config()?,
            self.network.rpc_endpoint.as_str(),
            self.network
                .auxiliary_addresses
                .mixnet_rewarder
                .mnemonic
                .clone(),
        )?)
    }

    fn signing_mixnet_contract_admin(
        &self,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            self.network.client_config()?,
            self.network.rpc_endpoint.as_str(),
            self.network.contracts.mixnet.admin_mnemonic.clone(),
        )?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BondingInformation {
    host: String,
    identity_key: ed25519::PublicKey,
}

#[derive(Deserialize)]
struct ReducedSignatureOut {
    encoded_signature: String,
}

impl NetworkManager {
    async fn initialise_nym_node(
        &self,
        ctx: &mut LocalNodesCtx<'_>,
        offset: u16,
        is_gateway: bool,
    ) -> Result<(), NetworkManagerError> {
        let mut node = NymNode::new_empty();
        let env = ctx.network.default_env_file_path();
        let id = ctx.nym_node_id(&node);

        let output_dir = tempfile::tempdir()?;
        let output_file_path = output_dir.path().join("bonding_info.json");

        ctx.set_pb_message(format!("initialising node {id}..."));
        let mix_port = 5000 + offset;
        let verloc_port = 6000 + offset;
        let clients_port = 7000 + offset;
        let http_port = 8000 + offset;

        node.mix_port = mix_port;
        node.verloc_port = verloc_port;
        node.clients_port = clients_port;
        node.http_port = http_port;

        let mut cmd = Command::new(&ctx.nym_node_binary);
        cmd.args([
            "-c",
            &env.display().to_string(),
            "run",
            "--id",
            &id,
            "--init-only",
            "--public-ips",
            "127.0.0.1",
            "--http-bind-address",
            &format!("127.0.0.1:{http_port}"),
            "--mixnet-bind-address",
            &format!("127.0.0.1:{mix_port}"),
            "--verloc-bind-address",
            &format!("127.0.0.1:{verloc_port}"),
            "--entry-bind-address",
            &format!("127.0.0.1:{clients_port}"),
            "--mixnet-announce-port",
            &mix_port.to_string(),
            "--verloc-announce-port",
            &verloc_port.to_string(),
            "--mnemonic",
            &Zeroizing::new(node.owner.mnemonic.to_string()),
            "--local",
            "--accept-operator-terms-and-conditions",
            "--output",
            "json",
            "--bonding-information-output",
            &output_file_path.display().to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true);

        if is_gateway {
            cmd.args(["--mode", "entry"]);
        } else {
            // be explicit about it, even though we don't have to be
            cmd.args(["--mode", "mixnode"]);
        }

        let child = cmd.spawn()?;
        let child_fut = child.wait_with_output();
        let out = ctx.async_with_progress(child_fut).await?;
        if !out.status.success() {
            error!("nym node failure");
            println!("{}", String::from_utf8_lossy(&out.stderr));
            return Err(NetworkManagerError::NymNodeExecutionFailure);
        }

        let output_file = fs::File::open(&output_file_path)?;
        let bonding_info: BondingInformation = serde_json::from_reader(&output_file)?;

        node.identity_key = bonding_info.identity_key.to_string();

        ctx.set_pb_message(format!("generating bonding signature for node {id}..."));

        let msg = node.bonding_payload();

        let child = Command::new(&ctx.nym_node_binary)
            .args([
                "--no-banner",
                "sign",
                "--id",
                &id,
                "--contract-msg",
                &msg,
                "--output",
                "json",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output();

        let out = ctx.async_with_progress(child).await?;
        if !out.status.success() {
            error!("nym node failure");
            println!("{}", String::from_utf8_lossy(&out.stderr));
            return Err(NetworkManagerError::NymNodeExecutionFailure);
        }
        let signature: ReducedSignatureOut = serde_json::from_slice(&out.stdout)?;
        node.bonding_signature = signature.encoded_signature;

        ctx.println(format!(
            "\tinitialised node {} (gateway: {})",
            node.identity_key, is_gateway
        ));

        if is_gateway {
            ctx.gateways.push(node)
        } else {
            ctx.mix_nodes.push(node)
        }
        Ok(())
    }

    async fn check_if_network_is_empty(
        &self,
        ctx: &LocalNodesCtx<'_>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🐽 {}Making sure the network is fresh...",
            style("[0/5]").bold().dim()
        ));

        ctx.set_pb_message("checking network state...");

        let client = ctx.signing_mixnet_contract_admin()?;
        let fut = client.get_all_nymnode_bonds();
        let nym_nodes = ctx.async_with_progress(fut).await?;

        if !nym_nodes.is_empty() {
            return Err(NetworkManagerError::NetworkNotEmpty);
        }

        let fut = client.get_all_mixnode_bonds();
        let mixnodes = ctx.async_with_progress(fut).await?;
        if !mixnodes.is_empty() {
            return Err(NetworkManagerError::NetworkNotEmpty);
        }

        let fut = client.get_all_gateways();
        let gateways = ctx.async_with_progress(fut).await?;
        if !gateways.is_empty() {
            return Err(NetworkManagerError::NetworkNotEmpty);
        }

        Ok(())
    }

    async fn initialise_nym_nodes(
        &self,
        ctx: &mut LocalNodesCtx<'_>,
        mixnodes: u16,
        gateways: u16,
    ) -> Result<(), NetworkManagerError> {
        const OFFSET: u16 = 100;
        if mixnodes > OFFSET {
            panic!("seriously? over 100 mixnodes?")
        }

        ctx.println(format!(
            "🔏 {}Initialising local nym-nodes...",
            style("[1/5]").bold().dim()
        ));

        for i in 0..mixnodes {
            self.initialise_nym_node(ctx, i, false).await?;
        }
        for i in 0..gateways {
            self.initialise_nym_node(ctx, i + OFFSET, true).await?;
        }

        ctx.println("\t✅ all nym nodes got initialised!");

        Ok(())
    }

    async fn transfer_bonding_tokens(
        &self,
        ctx: &LocalNodesCtx<'_>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "💸 {}Transferring tokens to the bond owners...",
            style("[2/5]").bold().dim()
        ));

        let mut receivers = Vec::new();
        for node in ctx.mix_nodes.iter().chain(ctx.gateways.iter()) {
            // send 101nym to the owner
            receivers.push((node.owner.address.clone(), ctx.admin.mix_coins(101_000000)))
        }

        ctx.set_pb_message("attempting to send signer tokens...");

        let send_future = ctx.admin.send_multiple(
            receivers,
            "bond owners token transfer from testnet-manager",
            None,
        );
        let res = ctx.async_with_progress(send_future).await?;

        ctx.println(format!(
            "\t✅ sent tokens in transaction: {} (height {})",
            res.hash, res.height
        ));
        Ok(())
    }

    async fn bond_node(
        &self,
        ctx: &LocalNodesCtx<'_>,
        node: &NymNode,
        is_gateway: bool,
    ) -> Result<(), NetworkManagerError> {
        let prefix = if is_gateway { "[gateway]" } else { "[mixnode]" };
        ctx.set_pb_prefix(prefix);

        let id = ctx.nym_node_id(node);
        ctx.set_pb_message(format!("attempting to bond node {id}..."));

        let owner = ctx.signing_node_owner(node)?;

        let typ = if is_gateway {
            "gateway [as nym-node]"
        } else {
            "mixnode [as nym-node]"
        };

        let bonding_fut = owner.bond_nymnode(
            node.bonding_nym_node(),
            node.cost_params(),
            node.bonding_signature(),
            node.pledge().into(),
            None,
        );

        let res = ctx.async_with_progress(bonding_fut).await?;
        ctx.println(format!(
            "\t{id} ({typ}) bonded in transaction: {}",
            res.transaction_hash
        ));

        Ok(())
    }

    async fn bond_nym_nodes(&self, ctx: &LocalNodesCtx<'_>) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "⛓️ {}Bonding the local nym-nodes...",
            style("[3/5]").bold().dim()
        ));

        for mix_node in &ctx.mix_nodes {
            self.bond_node(ctx, mix_node, false).await?;
        }
        for gateway in &ctx.gateways {
            self.bond_node(ctx, gateway, true).await?;
        }

        ctx.println("\t✅ all nym nodes got bonded!");

        Ok(())
    }

    async fn assign_to_active_set(
        &self,
        ctx: &LocalNodesCtx<'_>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🔌 {}Assigning nodes to the active set...",
            style("[4/5]").bold().dim()
        ));

        // this could be batched in a single tx, but that's too much effort for now
        let rewarder = ctx.signing_rewarder()?;

        ctx.set_pb_message("checking and temporarily adjusting epoch lengths...");
        let fut = rewarder.get_current_interval_details();
        let original_epoch = ctx.async_with_progress(fut).await?;

        let expected_end = original_epoch.interval.current_epoch_end();
        let now = OffsetDateTime::now_utc();
        if expected_end > now {
            loop {
                let now = OffsetDateTime::now_utc();
                let diff = expected_end - now;
                if diff.is_negative() {
                    break;
                }

                let std_diff = diff.unsigned_abs();
                let fut = sleep(std::time::Duration::from_millis(500));
                ctx.set_pb_message(format!(
                    "waiting for {} for the epoch end...",
                    humantime::format_duration(std_diff)
                ));
                ctx.async_with_progress(fut).await;
            }
            // wait extra 10s due to possible block time desync
            ctx.set_pb_message("waiting extra 10s to make sure blocks have advanced".to_string());
            let fut = sleep(std::time::Duration::from_secs(10));
            ctx.async_with_progress(fut).await;
        }

        // TODO: for some reason contract rejects correct admin. won't be debugging it now.
        // let changed_length = if expected_end > now {
        //
        //     // if it's < 10s, just wait
        //     let diff = expected_end - now;
        //
        //     if diff < Duration::seconds(10) {
        //         let std_diff = diff.unsigned_abs();
        //         let fut = sleep(std_diff);
        //         ctx.set_pb_message(format!(
        //             "waiting for {} for the epoch end...",
        //             humantime::format_duration(std_diff)
        //         ));
        //         ctx.async_with_progress(fut).await;
        //         false
        //     } else {
        //         ctx.println(format!(
        //             "🙈 {}Reducing epoch length...",
        //             style("[4.pre/5]").bold().dim()
        //         ));
        //
        //         // just lower the epoch length and later restore it
        //         let admin = ctx.signing_mixnet_contract_admin()?;
        //         let fut = admin.update_interval_config(
        //             original_epoch.interval.epochs_in_interval(),
        //             10,
        //             true,
        //             None,
        //         );
        //         ctx.async_with_progress(fut).await?;
        //         let fut = sleep(std::time::Duration::from_secs(10));
        //         ctx.set_pb_message("waiting for 10s for the epoch end...");
        //         ctx.async_with_progress(fut).await;
        //         true
        //     }
        // } else {
        //     false
        // };

        // reduce epoch length if it would prevent us from the advancing the state

        ctx.set_pb_message("starting epoch transition...");
        let fut = rewarder.begin_epoch_transition(None);
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("reconciling (no) epoch events...");
        let fut = rewarder.reconcile_epoch_events(None, None);
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("finally assigning the active set... exit...");
        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::ExitGateway,
                nodes: vec![],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("finally assigning the active set... entry...");
        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::EntryGateway,
                nodes: vec![4],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("finally assigning the active set... layer1...");
        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Layer1,
                nodes: vec![1],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("finally assigning the active set... layer2...");
        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Layer2,
                nodes: vec![2],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("finally assigning the active set... layer3...");
        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Layer3,
                nodes: vec![3],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("finally assigning the active set... [empty] standby...");
        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Standby,
                nodes: vec![],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        // TODO: for some reason contract rejects correct admin. won't be debugging it now.
        // if changed_length {
        //     ctx.println(format!(
        //         "🙈 {}Restoring epoch length...",
        //         style("[4.post/5]").bold().dim()
        //     ));
        //     ctx.set_pb_message("restoring original epoch length...");
        //     let admin = ctx.signing_mixnet_contract_admin()?;
        //     let fut = admin.update_interval_config(
        //         original_epoch.interval.epochs_in_interval(),
        //         original_epoch.interval.epoch_length_secs(),
        //         true,
        //         None,
        //     );
        //     ctx.async_with_progress(fut).await?;
        // }

        Ok(())
    }

    fn prepare_nym_nodes_run_commands(
        &self,
        ctx: &LocalNodesCtx,
    ) -> Result<RunCommands, NetworkManagerError> {
        let env_file = ctx.network.default_env_file_path();

        let bin_canon = fs::canonicalize(&ctx.nym_node_binary)?;
        let env_canon = fs::canonicalize(env_file)?;
        let bin_canon_display = bin_canon.display();
        let env_canon_display = env_canon.display();

        let mut cmds = Vec::new();
        for mixnode in ctx.mix_nodes.iter() {
            ctx.println(format!(
                "\tpreparing node {} (mixnode)",
                mixnode.identity_key
            ));
            let id = ctx.nym_node_id(mixnode);
            cmds.push(format!(
                "{bin_canon_display} -c {env_canon_display} run --id {id} --local --unsafe-disable-noise"
            ));
        }

        for gateway in ctx.gateways.iter() {
            ctx.println(format!(
                "\tpreparing node {} (gateway)",
                gateway.identity_key
            ));
            let id = ctx.nym_node_id(gateway);
            cmds.push(format!(
                "{bin_canon_display} -c {env_canon_display} run --id {id} --local --unsafe-disable-noise"
            ));
        }

        Ok(RunCommands(cmds))
    }

    fn output_nym_nodes_run_commands(&self, ctx: &LocalNodesCtx, cmds: &RunCommands) {
        ctx.progress.output_run_commands(cmds)
    }

    async fn persist_nodes_in_database(
        &self,
        ctx: &LocalNodesCtx<'_>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "📦 {}Storing the node information in the database",
            style("[5/5]").bold().dim()
        ));

        ctx.set_pb_message("attempting to persist node information...");
        let mix_save_future = self
            .storage
            .persist_mixnodes(&ctx.mix_nodes, ctx.network.id);
        let gw_save_future = self.storage.persist_gateways(&ctx.gateways, ctx.network.id);
        ctx.async_with_progress(mix_save_future).await?;
        ctx.async_with_progress(gw_save_future).await?;

        ctx.println(
            "\t✅ the bonded node information got persisted in the database for future use",
        );

        Ok(())
    }

    pub(crate) async fn init_local_nym_nodes<P: AsRef<Path>>(
        &self,
        nym_node_binary: P,
        network: &LoadedNetwork,
        mixnodes: u16,
        gateways: u16,
    ) -> Result<RunCommands, NetworkManagerError> {
        let mut ctx = LocalNodesCtx::new(
            nym_node_binary.as_ref().to_path_buf(),
            network,
            self.admin.deref().clone(),
        )?;

        let env_file = ctx.network.default_env_file_path();
        if !env_file.exists() {
            return Err(NetworkManagerError::EnvFileNotGenerated);
        }

        self.check_if_network_is_empty(&ctx).await?;
        self.initialise_nym_nodes(&mut ctx, mixnodes, gateways)
            .await?;
        self.transfer_bonding_tokens(&ctx).await?;
        self.bond_nym_nodes(&ctx).await?;
        self.assign_to_active_set(&ctx).await?;
        self.persist_nodes_in_database(&ctx).await?;
        let cmds = self.prepare_nym_nodes_run_commands(&ctx)?;
        self.output_nym_nodes_run_commands(&ctx, &cmds);

        Ok(cmds)
    }
}
