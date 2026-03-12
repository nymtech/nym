// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::MIN_MASTER_UNYM_BALANCE;
use crate::helpers::wasm_code;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::container_helpers::check_container_is_running;
use crate::orchestrator::context::LocalnetContext;
use crate::orchestrator::setup::nym_nodes::{GATEWAYS, MIXNODES};
use anyhow::{Context, bail};
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::nyxd::cosmwasm_client::types::UploadResult;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, QueryHttpRpcNyxdClient};
use std::path::Path;

impl LocalnetOrchestrator {
    pub(crate) fn rpc_query_client(&self) -> anyhow::Result<QueryHttpRpcNyxdClient> {
        let rpc_endpoint = self.localnet_details.localhost_rpc_endpoint()?;
        let network_details = self.localnet_details.nym_network_details()?;

        QueryHttpRpcNyxdClient::connect_with_network_details(rpc_endpoint.as_str(), network_details)
            .context("nyxd query client creation failure")
    }

    pub(crate) fn signing_client(
        &self,
        mnemonic: &bip39::Mnemonic,
    ) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        let rpc_endpoint = self.localnet_details.localhost_rpc_endpoint()?;
        let network_details = self.localnet_details.nym_network_details()?;
        let mnemonic = mnemonic.clone();
        DirectSigningHttpRpcNyxdClient::connect_with_mnemonic_and_network_details(
            rpc_endpoint.as_str(),
            network_details,
            mnemonic,
        )
        .context("nyxd signing client creation failure")
    }

    pub(crate) fn master_signing_client(&self) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        let mnemonic = &self
            .localnet_details
            .nyxd_details()?
            .master_account
            .mnemonic;
        self.signing_client(mnemonic)
    }

    pub(crate) fn mixnet_rewarder_signing_client(
        &self,
    ) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
        let mnemonic = &self
            .localnet_details
            .auxiliary_accounts()?
            .mixnet_rewarder
            .mnemonic;
        self.signing_client(mnemonic)
    }

    pub(crate) async fn check_nyxd_container_is_running<T>(
        &self,
        ctx: &LocalnetContext<T>,
    ) -> anyhow::Result<bool> {
        check_container_is_running(ctx, &self.nyxd_container_name()).await
    }

    pub(crate) async fn check_nym_api_container_is_running<T>(
        &self,
        ctx: &LocalnetContext<T>,
    ) -> anyhow::Result<bool> {
        check_container_is_running(ctx, &self.nym_api_container_name()).await
    }

    pub(crate) async fn check_nym_node_containers_are_running<T>(
        &self,
        ctx: &LocalnetContext<T>,
    ) -> anyhow::Result<bool> {
        let mut running = 0;
        for id in 1..=GATEWAYS + MIXNODES {
            if check_container_is_running(ctx, &self.nym_node_container_name(id as NodeId)).await? {
                running += 1;
            }
        }
        // either ALL containers must be running or NONE of them. we must not be in a zombie state
        if running == 0 {
            return Ok(false);
        }
        if running == GATEWAYS + MIXNODES {
            return Ok(true);
        }
        bail!("only a subset of nym node containers is running! this is not allowed ({running}/4")
    }

    pub(crate) async fn verify_master_account<T>(
        &self,
        ctx: &LocalnetContext<T>,
    ) -> anyhow::Result<()> {
        // essentially perform two checks in one:
        // 1. is the rpc node running at the expected address
        // 2. is the master account really the main one? - we don't need to be incredibly restrictive,
        // i.e. whether it has staked on validators and whatnot. we only care it has sufficient
        // amount of tokens
        let client = self.rpc_query_client()?;
        let address = self
            .localnet_details
            .nyxd_details()?
            .master_account
            .address();

        let balance_fut = client.get_balance(&address, "unym".to_string());
        let balance = ctx
            .async_with_progress(balance_fut)
            .await
            .context(format!("failed to retrieve unym balance of {address}"))?
            .context(format!("{address} does not have any unym"))?;

        if balance.amount < MIN_MASTER_UNYM_BALANCE {
            bail!(
                "the unym balance of {address} ({balance}) is smaller than the minimum value of {MIN_MASTER_UNYM_BALANCE}"
            )
        }

        Ok(())
    }

    pub(crate) async fn upload_contract<P: AsRef<Path>, T>(
        &self,
        ctx: &LocalnetContext<T>,
        path: P,
    ) -> anyhow::Result<UploadResult> {
        let wasm = wasm_code(path)?;
        let admin = self.master_signing_client()?;
        let upload_future = admin.upload(wasm, "localnet contract upload", None);

        ctx.async_with_progress(upload_future)
            .await
            .context("contract upload failure")
    }

    pub(crate) async fn try_build_nym_binaries_docker_image<T>(
        &self,
        ctx: &mut LocalnetContext<T>,
        dockerfile_path: impl AsRef<Path>,
        monorepo_path: impl AsRef<Path>,
        image_tag: &str,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step(
            "building localnet-nym-binaries docker image... this might take few minutes...",
            "🏗️",
        );
        let dockerfile_path = dockerfile_path.as_ref().to_path_buf();
        let dockerfile_path_arg = dockerfile_path
            .to_str()
            .context("invalid Dockerfile path")?;

        let monorepo_path = monorepo_path.as_ref().to_path_buf();
        let monorepo_path_arg = monorepo_path.to_str().context("invalid monorepo path")?;

        ctx.execute_cmd_with_exit_status(
            "docker",
            [
                "build",
                "--platform",
                "linux/amd64",
                "-f",
                dockerfile_path_arg,
                "-t",
                image_tag,
                monorepo_path_arg,
            ],
        )
        .await?;
        Ok(())
    }
}
