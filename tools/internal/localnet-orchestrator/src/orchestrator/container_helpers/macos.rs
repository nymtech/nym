// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::CONTAINER_NETWORK_NAME;
use crate::helpers::exec_cmd_with_output;
use crate::orchestrator::container_helpers::container_binary;
use crate::orchestrator::context::LocalnetContext;
use crate::serde_helpers::macos::{self, container_network_inspect::NetworkInspect};
use crate::serde_helpers::{ContainerInspect, ContainersList};
use anyhow::Context;

pub(crate) async fn inspect_container_network() -> anyhow::Result<NetworkInspect> {
    let container_bin = container_binary();

    let output = exec_cmd_with_output(
        container_bin,
        ["network", "inspect", CONTAINER_NETWORK_NAME],
    )
    .await?;
    let network_details: NetworkInspect = serde_json::from_slice(&output.stdout)
        .context("failed to deserialise network information")?;
    Ok(network_details)
}

pub(crate) async fn is_container_network_running() -> anyhow::Result<bool> {
    let network_details = inspect_container_network().await?;
    Ok(network_details.is_running())
}

pub(crate) async fn inspect_container<T>(
    ctx: &LocalnetContext<T>,
    container_name: &str,
) -> anyhow::Result<ContainerInspect> {
    let container_bin = container_binary();

    let stdout = ctx
        .execute_cmd_with_stdout(container_bin, ["inspect", container_name])
        .await?;
    let inspect_info: macos::ContainerInspect =
        serde_json::from_slice(&stdout).context("failed to deserialise container information")?;
    inspect_info.try_into()
}

pub(crate) async fn list_containers<T>(ctx: &LocalnetContext<T>) -> anyhow::Result<ContainersList> {
    let container_bin = container_binary();

    let stdout = ctx
        .execute_cmd_with_stdout(container_bin, ["ls", "-a", "--format", "json"])
        .await?;
    let containers_list: macos::ContainersList =
        serde_json::from_slice(&stdout).context("failed to deserialise containers list")?;
    containers_list.try_into()
}
