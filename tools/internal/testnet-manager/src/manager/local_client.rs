// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use crate::helpers::{ProgressCtx, ProgressTracker};
use crate::manager::network::LoadedNetwork;
use crate::manager::NetworkManager;
use console::style;
use nym_config::{must_get_home, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, NYM_DIR};
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::NymApiClient;
use rand::{thread_rng, RngCore};
use std::fs;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::time::sleep;
use url::Url;

struct LocalClientCtx<'a> {
    nym_client_binary: PathBuf,
    client_id: String,
    gateway: Option<String>,

    progress: ProgressTracker,
    network: &'a LoadedNetwork,
}

impl<'a> ProgressCtx for LocalClientCtx<'a> {
    fn progress_tracker(&self) -> &ProgressTracker {
        &self.progress
    }
}

impl<'a> LocalClientCtx<'a> {
    fn new(
        nym_client_binary: PathBuf,
        gateway: Option<String>,
        network: &'a LoadedNetwork,
    ) -> Result<Self, NetworkManagerError> {
        let progress = ProgressTracker::new(format!(
            "\n🚀 setting up new local nym-client for network '{}' over {}",
            network.name, network.rpc_endpoint
        ));
        let mut rng = thread_rng();
        let client_id = format!("{}-client-{}", network.name, rng.next_u32());

        Ok(LocalClientCtx {
            nym_client_binary,
            network,
            progress,
            client_id,
            gateway,
        })
    }

    // hehe, that's disgusting, but it's not meant to be used by users
    fn nym_api_url(&self) -> Result<Url, NetworkManagerError> {
        let env_file = fs::read_to_string(self.network.default_env_file_path())?;
        for entry in env_file.lines() {
            if let Some(raw_url) = entry.strip_prefix("NYM_API=") {
                return Ok(raw_url.parse()?);
            }
        }
        Err(NetworkManagerError::NymApiEndpointMissing)
    }
}

impl NetworkManager {
    fn nym_client_config(&self, client_id: &str) -> PathBuf {
        must_get_home()
            .join(NYM_DIR)
            .join("clients")
            .join(client_id)
            .join(DEFAULT_CONFIG_DIR)
            .join(DEFAULT_CONFIG_FILENAME)
    }

    async fn wait_for_api_gateway<'a>(
        &self,
        ctx: &LocalClientCtx<'a>,
    ) -> Result<SocketAddr, NetworkManagerError> {
        // create api client
        // hehe, that's disgusting, but it's not meant to be used by users
        let api_url = ctx.nym_api_url()?;
        ctx.set_pb_message(format!(
            "⌛waiting for any gateway to appear in the directory ({api_url})..."
        ));

        let api_client = NymApiClient::new(api_url);

        let wait_fut = async {
            let inner_fut = async {
                loop {
                    let mut gateways = match api_client.nym_api.get_basic_gateways(None).await {
                        Ok(gateways) => gateways,
                        Err(err) => {
                            ctx.println(format!(
                                "❌ {} {err}",
                                style("[API QUERY FAILURE]: ").bold().dim()
                            ));
                            continue;
                        }
                    };

                    // if we explicitly specified some identity, find THIS node
                    if let Some(identity) = ctx.gateway.as_ref() {
                        if let Some(node) = gateways
                            .nodes
                            .iter()
                            .find(|gw| &gw.ed25519_identity_pubkey == identity)
                        {
                            return SocketAddr::new(
                                node.ip_addresses[0],
                                node.entry.clone().unwrap().ws_port,
                            );
                        }
                    }

                    // otherwise look for ANY node
                    if let Some(node) = gateways.nodes.pop() {
                        return SocketAddr::new(node.ip_addresses[0], node.entry.unwrap().ws_port);
                    }
                    sleep(Duration::from_secs(10)).await;
                }
            };
            tokio::time::timeout(Duration::from_secs(240), inner_fut).await
        };

        match ctx.async_with_progress(wait_fut).await {
            Ok(endpoint) => {
                ctx.println(format!(
                    "\twe finally got a gateway in the directory! it's at: {endpoint}"
                ));
                Ok(endpoint)
            }
            Err(_) => Err(NetworkManagerError::ApiGatewayWaitTimeout),
        }
    }

    async fn wait_for_gateway_endpoint<'a>(
        &self,
        ctx: &LocalClientCtx<'a>,
        gateway: SocketAddr,
    ) -> Result<(), NetworkManagerError> {
        ctx.set_pb_message(format!(
            "⌛waiting for gateway at {gateway} to start receiving traffic..."
        ));

        let wait_fut = async {
            let inner_fut = async {
                loop {
                    if TcpStream::connect(gateway).await.is_ok() {
                        break;
                    }
                    sleep(Duration::from_secs(10)).await;
                }
            };
            tokio::time::timeout(Duration::from_secs(240), inner_fut).await
        };

        if ctx.async_with_progress(wait_fut).await.is_err() {
            return Err(NetworkManagerError::GatewayWaitTimeout);
        }

        ctx.println(format!(
            "\tthe gateway at {gateway} has finally come online"
        ));

        Ok(())
    }

    async fn wait_for_gateway<'a>(
        &self,
        ctx: &LocalClientCtx<'a>,
    ) -> Result<(), NetworkManagerError> {
        let endpoint = self.wait_for_api_gateway(ctx).await?;
        self.wait_for_gateway_endpoint(ctx, endpoint).await
    }

    async fn prepare_nym_client<'a>(
        &self,
        ctx: &LocalClientCtx<'a>,
    ) -> Result<(), NetworkManagerError> {
        ctx.println(format!(
            "🔏 {}Initialising local nym-client...",
            style("[1/1]").bold().dim()
        ));

        let env = ctx.network.default_env_file_path();
        let id = &ctx.client_id;

        self.wait_for_gateway(ctx).await?;
        let mut rng = thread_rng();
        let mut port = rng.next_u32();
        port = (port + 1000) % (u16::MAX as u32);

        ctx.set_pb_message(format!("initialising client {id}..."));
        ctx.println(format!("\tinitialising client {id}..."));
        let mut cmd = Command::new(&ctx.nym_client_binary);
        cmd.args([
            "-c",
            &env.display().to_string(),
            "init",
            "--id",
            id,
            "--enabled-credentials-mode",
            "true",
            "--port",
            &port.to_string(),
        ])
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);

        if let Some(gateway) = &ctx.gateway {
            cmd.args(["--gateway", gateway]);
        }

        let mut child = cmd.spawn()?;

        let child_fut = child.wait();
        let out = ctx.async_with_progress(child_fut).await?;
        if !out.success() {
            return Err(NetworkManagerError::NymClientExecutionFailure);
        }

        ctx.println(format!("\tupdating client {id} config..."));

        let config_path = self.nym_client_config(id);
        let mut config_file = OpenOptions::new().append(true).open(config_path)?;

        // make the client ignore the performance of the nodes since we're not running network monitor
        writeln!(
            config_file,
            r#"

[debug.topology]
minimum_mixnode_performance = 0
minimum_gateway_performance = 0
"#
        )?;

        ctx.println(format!("\t✅client {id} is ready to use!"));

        Ok(())
    }

    fn prepare_client_run_command(
        &self,
        ctx: &LocalClientCtx,
    ) -> Result<String, NetworkManagerError> {
        let env_file = ctx.network.default_env_file_path();

        let bin_canon = fs::canonicalize(&ctx.nym_client_binary)?;
        let env_canon = fs::canonicalize(env_file)?;
        let bin_canon_display = bin_canon.display();
        let env_canon_display = env_canon.display();

        let id = &ctx.client_id;

        Ok(format!(
            "{bin_canon_display} -c {env_canon_display} run --id {id}"
        ))
    }

    pub(crate) async fn init_local_nym_client<P: AsRef<Path>>(
        &self,
        nym_client_binary: P,
        network: &LoadedNetwork,
        gateway: Option<String>,
    ) -> Result<String, NetworkManagerError> {
        let ctx = LocalClientCtx::new(nym_client_binary.as_ref().to_path_buf(), gateway, network)?;

        let env_file = ctx.network.default_env_file_path();
        if !env_file.exists() {
            return Err(NetworkManagerError::EnvFileNotGenerated);
        }

        self.prepare_nym_client(&ctx).await?;
        let cmd = self.prepare_client_run_command(&ctx)?;

        ctx.println("🏇 run the binary with the following commands:");
        ctx.println(&cmd);

        Ok(cmd)
    }
}
