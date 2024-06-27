// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::{ProgressCtx, ProgressTracker, RunCommands};
use crate::manager::contract::Account;
use crate::manager::network::LoadedNetwork;
use crate::manager::NetworkManager;
use console::style;
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::{
    construct_gateway_bonding_sign_payload, construct_mixnode_bonding_sign_payload, Addr, Gateway,
    Layer, LayerAssignment, MixNode, MixNodeCostParams, Percent,
};
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use nym_validator_client::nyxd::CosmWasmCoin;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use serde::{Deserialize, Serialize};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use zeroize::Zeroizing;

struct NymNode {
    // host is always 127.0.0.1
    mix_port: u16,
    verloc_port: u16,
    http_port: u16,
    clients_port: u16,
    sphinx_key: String,
    identity_key: String,
    version: String,

    owner: Account,
    bonding_signature: String,
}

impl NymNode {
    fn new_empty() -> NymNode {
        NymNode {
            mix_port: 0,
            verloc_port: 0,
            http_port: 0,
            clients_port: 0,
            sphinx_key: "".to_string(),
            identity_key: "".to_string(),
            version: "".to_string(),
            owner: Account::new(),
            bonding_signature: "".to_string(),
        }
    }

    fn pledge(&self) -> CosmWasmCoin {
        CosmWasmCoin::new(100_000000, "unym")
    }

    fn gateway(&self) -> Gateway {
        Gateway {
            host: "127.0.0.1".to_string(),
            mix_port: self.mix_port,
            clients_port: self.clients_port,
            location: "foomp".to_string(),
            sphinx_key: self.sphinx_key.clone(),
            identity_key: self.identity_key.clone(),
            version: self.version.clone(),
        }
    }

    fn mixnode(&self) -> MixNode {
        MixNode {
            host: "127.0.0.1".to_string(),
            mix_port: self.mix_port,
            verloc_port: self.verloc_port,
            http_api_port: self.http_port,
            sphinx_key: self.sphinx_key.clone(),
            identity_key: self.identity_key.clone(),
            version: self.version.clone(),
        }
    }

    fn cost_params(&self) -> MixNodeCostParams {
        MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
            interval_operating_cost: CosmWasmCoin::new(40_000000, "unym"),
        }
    }

    fn bonding_signature(&self) -> MessageSignature {
        // this is a valid bs58
        self.bonding_signature.parse().unwrap()
    }

    fn mixnode_bonding_payload(&self) -> String {
        let payload = construct_mixnode_bonding_sign_payload(
            0,
            Addr::unchecked(self.owner.address.to_string()),
            None,
            self.pledge(),
            self.mixnode(),
            self.cost_params(),
        );
        payload.to_base58_string().unwrap()
    }

    fn gateway_bonding_payload(&self) -> String {
        let payload = construct_gateway_bonding_sign_payload(
            0,
            Addr::unchecked(self.owner.address.to_string()),
            None,
            self.pledge(),
            self.gateway(),
        );
        payload.to_base58_string().unwrap()
    }
}

struct LocalNodesCtx<'a> {
    nym_node_binary: PathBuf,

    progress: ProgressTracker,
    network: &'a LoadedNetwork,
    admin: DirectSigningHttpRpcNyxdClient,

    mix_nodes: Vec<NymNode>,
    gateway: Option<NymNode>,
}

impl<'a> ProgressCtx for LocalNodesCtx<'a> {
    fn progress_tracker(&self) -> &ProgressTracker {
        &self.progress
    }
}

impl<'a> LocalNodesCtx<'a> {
    fn nym_node_id(&self, node: &NymNode) -> String {
        format!("{}-{}", node.owner.address, self.network.name)
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
            gateway: None,
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
            ctx.gateway = Some(node)
        } else {
            ctx.mix_nodes.push(node)
        }
        Ok(())
    }

    async fn initialise_nym_nodes<'a>(
        &self,
        ctx: &mut LocalNodesCtx<'a>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🔏 {}Initialising local nym-nodes...",
            style("[1/4]").bold().dim()
        ));

        // 3 mixnodes, 1 gateway; maybe at some point make it configurable
        for i in 0..4 {
            let is_gateway = i == 0;
            self.initialise_nym_node(ctx, i, is_gateway).await?;
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
            style("[2/4]").bold().dim()
        ));

        let mut receivers = Vec::new();
        for node in ctx
            .mix_nodes
            .iter()
            .chain(std::iter::once(ctx.gateway.as_ref().unwrap()))
        {
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

        let bonding_fut = if is_gateway {
            owner.bond_gateway(
                node.gateway(),
                node.bonding_signature(),
                node.pledge().into(),
                None,
            )
        } else {
            owner.bond_mixnode(
                node.mixnode(),
                node.cost_params(),
                node.bonding_signature(),
                node.pledge().into(),
                None,
            )
        };
        let res = ctx.async_with_progress(bonding_fut).await?;
        ctx.println(format!(
            "\t{id} bonded in transaction: {}",
            res.transaction_hash
        ));

        Ok(())
    }

    async fn bond_nym_nodes<'a>(&self, ctx: &LocalNodesCtx<'a>) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "⛓️ {}Bonding the local nym-nodes...",
            style("[3/4]").bold().dim()
        ));

        self.bond_node(ctx, ctx.gateway.as_ref().unwrap(), true)
            .await?;
        for mix_node in &ctx.mix_nodes {
            self.bond_node(ctx, mix_node, false).await?;
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
            style("[4/4]").bold().dim()
        ));

        let rewarder = ctx.signing_rewarder()?;

        ctx.set_pb_message("starting epoch transition...");
        let fut = rewarder.begin_epoch_transition(None);
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("reconciling (no) epoch events...");
        let fut = rewarder.reconcile_epoch_events(None, None);
        ctx.async_with_progress(fut).await?;

        ctx.set_pb_message("finally assigning the active set...");
        let fut = rewarder.get_rewarding_parameters();
        let rewarding_params = ctx.async_with_progress(fut).await?;
        let active_set_size = rewarding_params.active_set_size;

        let layer_assignment = vec![
            LayerAssignment::new(1, Layer::One),
            LayerAssignment::new(2, Layer::Two),
            LayerAssignment::new(3, Layer::Three),
        ];
        let fut = rewarder.advance_current_epoch(layer_assignment, active_set_size, None);
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
        for node in ctx
            .mix_nodes
            .iter()
            .chain(std::iter::once(ctx.gateway.as_ref().unwrap()))
        {
            let id = ctx.nym_node_id(node);
            cmds.push(format!(
                "{bin_canon_display} -c {env_canon_display} run --id {id} --local"
            ));
        }

        Ok(RunCommands(cmds))
    }

    fn output_nym_nodes_run_commands(&self, ctx: &LocalNodesCtx, cmds: &RunCommands) {
        ctx.progress.output_run_commands(cmds)
    }

    pub(crate) async fn init_local_nym_nodes<P: AsRef<Path>>(
        &self,
        nym_node_binary: P,
        network: &LoadedNetwork,
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

        self.initialise_nym_nodes(&mut ctx).await?;
        self.transfer_bonding_tokens(&ctx).await?;
        self.bond_nym_nodes(&ctx).await?;
        self.assign_to_active_set(&ctx).await?;
        let cmds = self.prepare_nym_nodes_run_commands(&ctx)?;
        self.output_nym_nodes_run_commands(&ctx, &cmds);

        Ok(cmds)
    }
}
