// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::{ProgressCtx, ProgressTracker, RunCommands};
use crate::manager::network::LoadedNetwork;
use crate::manager::node::NymNode;
use crate::manager::NetworkManager;
use console::style;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::RoleAssignment;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use serde::{Deserialize, Serialize};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use zeroize::Zeroizing;

struct LocalNodesCtx<'a> {
    nym_node_binary: PathBuf,

    progress: ProgressTracker,
    network: &'a LoadedNetwork,
    admin: DirectSigningHttpRpcNyxdClient,

    mix_nodes: Vec<NymNode>,
    gateways: Vec<NymNode>,
}

impl<'a> ProgressCtx for LocalNodesCtx<'a> {
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
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "node_type")]
pub enum BondingInformationV1 {
    Mixnode(MixnodeBondingInformation),
    Gateway(GatewayBondingInformation),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MixnodeBondingInformation {
    pub(crate) version: String,
    pub(crate) host: String,
    pub(crate) identity_key: String,
    pub(crate) sphinx_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GatewayBondingInformation {
    pub(crate) version: String,
    pub(crate) host: String,
    pub(crate) location: String,
    pub(crate) identity_key: String,
    pub(crate) sphinx_key: String,
}

#[derive(Deserialize)]
struct ReducedSignatureOut {
    encoded_signature: String,
}

impl NetworkManager {
    async fn initialise_nym_node<'a>(
        &self,
        ctx: &mut LocalNodesCtx<'a>,
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
            "--mnemonic",
            &Zeroizing::new(node.owner.mnemonic.to_string()),
            "--local",
            "--output",
            "json",
            "--bonding-information-output",
            &output_file_path.display().to_string(),
        ])
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);

        if is_gateway {
            cmd.args(["--mode", "entry"]);
        }

        let mut child = cmd.spawn()?;
        let child_fut = child.wait();
        let out = ctx.async_with_progress(child_fut).await?;
        if !out.success() {
            return Err(NetworkManagerError::NymNodeExecutionFailure);
        }

        let output_file = fs::File::open(&output_file_path)?;
        let bonding_info: BondingInformationV1 = serde_json::from_reader(&output_file)?;

        match bonding_info {
            BondingInformationV1::Mixnode(bonding_info) => {
                node.identity_key = bonding_info.identity_key;
                node.sphinx_key = bonding_info.sphinx_key;
                node.version = bonding_info.version;
            }
            BondingInformationV1::Gateway(bonding_info) => {
                node.identity_key = bonding_info.identity_key;
                node.sphinx_key = bonding_info.sphinx_key;
                node.version = bonding_info.version;
            }
        }

        ctx.set_pb_message(format!("generating bonding signature for node {id}..."));

        let msg = if is_gateway {
            node.gateway_bonding_payload()
        } else {
            node.mixnode_bonding_payload()
        };

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
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output();
        let out = ctx.async_with_progress(child).await?;
        if !out.status.success() {
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

    async fn initialise_nym_nodes<'a>(
        &self,
        ctx: &mut LocalNodesCtx<'a>,
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

    async fn transfer_bonding_tokens<'a>(
        &self,
        ctx: &LocalNodesCtx<'a>,
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

    async fn bond_node<'a>(
        &self,
        ctx: &LocalNodesCtx<'a>,
        node: &NymNode,
        is_gateway: bool,
    ) -> Result<(), NetworkManagerError> {
        let prefix = if is_gateway { "[gateway]" } else { "[mixnode]" };
        ctx.set_pb_prefix(prefix);

        let id = ctx.nym_node_id(node);
        ctx.set_pb_message(format!("attempting to bond node {id}..."));

        let owner = ctx.signing_node_owner(node)?;

        let (bonding_fut, typ) = if is_gateway {
            (
                owner.bond_gateway(
                    node.gateway(),
                    node.bonding_signature(),
                    node.pledge().into(),
                    None,
                ),
                "gateway",
            )
        } else {
            (
                owner.bond_mixnode(
                    node.mixnode(),
                    node.cost_params(),
                    node.bonding_signature(),
                    node.pledge().into(),
                    None,
                ),
                "mixnode",
            )
        };
        let res = ctx.async_with_progress(bonding_fut).await?;
        ctx.println(format!(
            "\t{id} ({typ}) bonded in transaction: {}",
            res.transaction_hash
        ));

        Ok(())
    }

    async fn bond_nym_nodes<'a>(&self, ctx: &LocalNodesCtx<'a>) -> Result<(), NetworkManagerError> {
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

    async fn assign_to_active_set<'a>(
        &self,
        ctx: &LocalNodesCtx<'a>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🔌 {}Assigning mixnodes to the active set...",
            style("[4/5]").bold().dim()
        ));

        let rewarder = ctx.signing_rewarder()?;

        ctx.set_pb_message("starting epoch transition...");
        let fut = rewarder.begin_epoch_transition(None);
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("reconciling (no) epoch events...");
        let fut = rewarder.reconcile_epoch_events(None, None);
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("[BROKEN] finally assigning the active set...");
        // let fut = rewarder.get_rewarding_parameters();
        // let rewarding_params = ctx.async_with_progress(fut).await?;
        // let active_set_size = rewarding_params.active_set_size;

        let unused_variable = "this has to be fixed up and refactored....";
        /*
                fn generate_role_assignment_messages(
            &self,
            rewarded_set: RewardedSet,
        ) -> Vec<(ExecuteMsg, Vec<Coin>)> {
            // currently we just assign all of them together,
            // but the contract is ready to handle them separately should we need it
            // if the tx is too big
            let mut msgs = Vec::new();
            for (role, nodes) in [
                (Role::ExitGateway, rewarded_set.exit_gateways),
                (Role::EntryGateway, rewarded_set.entry_gateways),
                (Role::Layer1, rewarded_set.layer1),
                (Role::Layer2, rewarded_set.layer2),
                (Role::Layer3, rewarded_set.layer3),
                (Role::Standby, rewarded_set.standby),
            ] {
                msgs.push((
                    ExecuteMsg::AssignRoles {
                        assignment: RoleAssignment { role, nodes },
                    },
                    Vec::new(),
                ));
            }
            msgs
        }
             */

        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::ExitGateway,
                nodes: vec![4],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::EntryGateway,
                nodes: vec![],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Layer1,
                nodes: vec![1],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Layer2,
                nodes: vec![2],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Layer3,
                nodes: vec![3],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

        let fut = rewarder.assign_roles(
            RoleAssignment {
                role: Role::Standby,
                nodes: vec![],
            },
            None,
        );
        ctx.async_with_progress(fut).await?;

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
                "{bin_canon_display} -c {env_canon_display} run --id {id} --local"
            ));
        }

        for gateway in ctx.gateways.iter() {
            ctx.println(format!(
                "\tpreparing node {} (gateway)",
                gateway.identity_key
            ));
            let id = ctx.nym_node_id(gateway);
            cmds.push(format!(
                "{bin_canon_display} -c {env_canon_display} run --id {id} --local"
            ));
        }

        Ok(RunCommands(cmds))
    }

    fn output_nym_nodes_run_commands(&self, ctx: &LocalNodesCtx, cmds: &RunCommands) {
        ctx.progress.output_run_commands(cmds)
    }

    async fn persist_nodes_in_database<'a>(
        &self,
        ctx: &LocalNodesCtx<'a>,
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
