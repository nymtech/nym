// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    CONTAINER_NETWORK_NAME, LOCALNET_NYM_API_CONTAINER_NAME_SUFFIX,
    LOCALNET_NYM_BINARIES_IMAGE_NAME, LOCALNET_NYM_NODE_CONTAINER_NAME_SUFFIX,
    LOCALNET_NYXD_CONTAINER_NAME_SUFFIX,
};
use crate::helpers::{exec_cmd_with_output, retrieve_current_nymnode_version};
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::context::LocalnetContext;
use nym_mixnet_contract_common::NodeId;
use std::ffi::{OsStr, OsString};
use std::net::IpAddr;
use std::path::Path;
use std::process::ExitStatus;
use tracing::info;

#[cfg(target_os = "linux")]
pub(crate) use linux::*;

#[cfg(target_os = "macos")]
pub(crate) use macos::*;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

impl LocalnetOrchestrator {
    pub(crate) fn nyxd_container_name(&self) -> String {
        format!(
            "{}-{}",
            self.localnet_details.human_name, LOCALNET_NYXD_CONTAINER_NAME_SUFFIX
        )
    }

    pub(crate) fn nym_api_container_name(&self) -> String {
        format!(
            "{}-{}",
            self.localnet_details.human_name, LOCALNET_NYM_API_CONTAINER_NAME_SUFFIX
        )
    }

    pub(crate) fn nym_node_container_name(&self, id: NodeId) -> String {
        self.nym_node_name(id)
    }

    pub(crate) fn nym_node_name(&self, id: NodeId) -> String {
        format!(
            "{}-{}-{id}",
            self.localnet_details.human_name, LOCALNET_NYM_NODE_CONTAINER_NAME_SUFFIX
        )
    }

    #[allow(clippy::unwrap_used)]
    pub(crate) fn nyxd_volume(&self) -> String {
        // SAFETY: directory had been sanitised before getting here
        format!(
            "{}:/root/.nyxd",
            self.storage
                .nyxd_container_data_directory()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
        )
    }

    #[allow(clippy::unwrap_used)]
    pub(crate) fn nym_api_volume(&self) -> String {
        // SAFETY: directory had been sanitised before getting here
        format!(
            "{}:/root/.nym/nym-api/default",
            self.storage
                .nym_api_container_data_directory()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
        )
    }

    #[allow(clippy::unwrap_used)]
    pub(crate) fn nym_node_volume(&self, id: NodeId) -> String {
        // SAFETY: directory had been sanitised before getting here
        format!(
            "{}:/root/.nym/nym-nodes/default-nym-node",
            self.storage
                .nym_node_container_data_directory(id)
                .canonicalize()
                .unwrap()
                .to_string_lossy()
        )
    }

    #[allow(clippy::unwrap_used)]
    pub(crate) fn kernel_configs_volume(&self) -> String {
        // SAFETY: directory had been sanitised before getting here
        format!(
            "{}:/root/kernel-configs",
            self.storage
                .data_cache()
                .kernel_configs_directory()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
        )
    }
}

#[allow(clippy::panic)]
pub(crate) fn container_binary() -> &'static str {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            "container"
        } else if #[cfg(target_os = "linux")] {
            "nerdctl"
        } else {
            panic!("unsupported platform")
        }
    }
}

pub(crate) async fn save_docker_image<T>(
    ctx: &mut LocalnetContext<T>,
    output_path: &str,
    image_tag: &str,
) -> anyhow::Result<()> {
    ctx.begin_next_step("saving the docker image to a temporary file...", "💾️");

    ctx.execute_cmd_with_exit_status("docker", ["save", "-o", output_path, image_tag])
        .await?;
    Ok(())
}

pub(crate) async fn load_image_into_container_runtime<T>(
    ctx: &mut LocalnetContext<T>,
    saved_image_path: &str,
) -> anyhow::Result<()> {
    let container_bin = container_binary();
    ctx.begin_next_step("inserting docker image into the container runtime...", "📩");

    ctx.execute_cmd_with_exit_status(
        container_bin,
        ["image", "load", "--input", saved_image_path],
    )
    .await?;

    Ok(())
}

pub(crate) async fn remove_container_image<T>(
    ctx: &LocalnetContext<T>,
    image_tag: &str,
) -> anyhow::Result<()> {
    let container_bin = container_binary();

    ctx.execute_cmd_with_stdout(container_bin, ["image", "rm", image_tag])
        .await?;
    Ok(())
}

pub(crate) async fn check_container_image_exists<T>(
    ctx: &LocalnetContext<T>,
    image_tag: &str,
) -> anyhow::Result<bool> {
    let container_bin = container_binary();

    let status = ctx
        .exec_fallible_cmd_with_exit_status(container_bin, ["image", "inspect", image_tag])
        .await?;

    Ok(status.success())
}

pub(crate) async fn stop_container<T>(
    ctx: &LocalnetContext<T>,
    container_name: &str,
) -> anyhow::Result<()> {
    let container_bin = container_binary();

    ctx.execute_cmd_with_stdout(container_bin, ["stop", container_name])
        .await?;
    Ok(())
}

pub(crate) async fn remove_container<T>(
    ctx: &LocalnetContext<T>,
    container_name: &str,
) -> anyhow::Result<()> {
    let container_bin = container_binary();

    ctx.execute_cmd_with_stdout(container_bin, ["rm", container_name])
        .await?;
    Ok(())
}

pub(crate) async fn check_container_is_running<T>(
    ctx: &LocalnetContext<T>,
    container_name: &str,
) -> anyhow::Result<bool> {
    let container_info = inspect_container(ctx, container_name).await?;
    Ok(container_info.is_running())
}

pub(crate) async fn get_container_ip_address<T>(
    ctx: &LocalnetContext<T>,
    container_name: &str,
) -> anyhow::Result<IpAddr> {
    let container_info = inspect_container(ctx, container_name).await?;
    container_info.container_ip()
}

pub(crate) async fn create_container_network() -> anyhow::Result<()> {
    let container_bin = container_binary();

    info!("creating {CONTAINER_NETWORK_NAME} network");
    exec_cmd_with_output(container_bin, ["network", "create", CONTAINER_NETWORK_NAME]).await?;
    Ok(())
}

async fn run_container_cmd<T>(
    ctx: &LocalnetContext<T>,
    sub_cmd: OsString,
    mut args: Vec<OsString>,
) -> anyhow::Result<Vec<u8>> {
    let container_bin = container_binary();
    args.insert(0, sub_cmd);

    ctx.execute_cmd_with_stdout(container_bin, args).await
}

async fn run_container_cmd_fallible<T>(
    ctx: &LocalnetContext<T>,
    sub_cmd: OsString,
    mut args: Vec<OsString>,
) -> anyhow::Result<ExitStatus> {
    let container_bin = container_binary();
    args.insert(0, sub_cmd);

    ctx.exec_fallible_cmd_with_exit_status(container_bin, args)
        .await
}

pub(crate) fn attach_run_container_args<S, I>(base_args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd_args: Vec<OsString> = Vec::new();
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
           cmd_args.push("--arch".into());
           cmd_args.push("amd64".into());
        } else if #[cfg(target_os = "linux")] {
            cmd_args.push("--runtime".into());
            cmd_args.push("io.containerd.kata.v2".into());
            cmd_args.push("--device".into());
            cmd_args.push("/dev/net/tun".into());
            cmd_args.push("--privileged".into());
            cmd_args.push("--security-opt".into());
            cmd_args.push("privileged-without-host-devices".into());
        }
    }

    for arg in base_args {
        cmd_args.push(arg.as_ref().into());
    }
    cmd_args
}

pub(crate) async fn run_container<T, S, I>(
    ctx: &LocalnetContext<T>,
    args: I,
    dns: Option<String>,
) -> anyhow::Result<Vec<u8>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd_args = attach_run_container_args(args);
    if let Some(dns) = dns {
        // --dns $DNS
        cmd_args.insert(0, "--dns".into());
        cmd_args.insert(1, dns.into());
    }

    run_container_cmd(ctx, "run".into(), cmd_args).await
}

// no progress bar
pub(crate) async fn run_container_fut<S, I>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let container_bin = container_binary();

    let mut cmd_args: Vec<OsString> = Vec::new();
    cmd_args.push("run".into());

    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
           cmd_args.push("--arch".into());
           cmd_args.push("amd64".into());
        }
    }

    for arg in args {
        cmd_args.push(arg.as_ref().into());
    }

    exec_cmd_with_output(container_bin, cmd_args).await?;
    Ok(())
}

pub(crate) async fn run_container_fallible<T, S, I>(
    ctx: &LocalnetContext<T>,
    args: I,
) -> anyhow::Result<ExitStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_container_cmd_fallible(
        ctx,
        "run".into(),
        args.into_iter()
            .map(|a| a.as_ref().to_os_string())
            .collect(),
    )
    .await
}

pub(crate) async fn exec_container<T, S, I>(
    ctx: &LocalnetContext<T>,
    args: I,
) -> anyhow::Result<Vec<u8>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_container_cmd(
        ctx,
        "exec".into(),
        args.into_iter()
            .map(|a| a.as_ref().to_os_string())
            .collect(),
    )
    .await
}

pub(crate) fn default_nym_binaries_image_tag(
    monorepo_root: impl AsRef<Path>,
) -> anyhow::Result<String> {
    let version = retrieve_current_nymnode_version(monorepo_root)?;
    Ok(format!("{LOCALNET_NYM_BINARIES_IMAGE_NAME}:{version}"))
}
