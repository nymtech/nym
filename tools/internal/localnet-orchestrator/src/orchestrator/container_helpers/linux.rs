// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::CONTAINER_NETWORK_NAME;
use crate::helpers::exec_fallible_cmd_with_output;
use crate::orchestrator::container_helpers::container_binary;
use crate::orchestrator::context::LocalnetContext;
use crate::serde_helpers::linux::container_network_inspect::NetworkInspect;
use crate::serde_helpers::{ContainerInspect, ContainersList, linux};
use anyhow::Context;

pub(crate) async fn try_inspect_container_network() -> anyhow::Result<Option<NetworkInspect>> {
    let container_bin = container_binary();

    let output = exec_fallible_cmd_with_output(
        container_bin,
        ["network", "inspect", CONTAINER_NETWORK_NAME],
    )
    .await?;
    if !output.status.success() {
        return Ok(None);
    }
    let network_details: NetworkInspect = serde_json::from_slice(&output.stdout)
        .context("failed to deserialise network information")?;
    Ok(Some(network_details))
}

pub(crate) async fn is_container_network_running() -> anyhow::Result<bool> {
    let Some(network_details) = try_inspect_container_network().await? else {
        return Ok(false);
    };
    Ok(network_details.is_running())
}

pub(crate) async fn inspect_container<T>(
    ctx: &LocalnetContext<T>,
    container_name: &str,
) -> anyhow::Result<ContainerInspect> {
    let container_bin = container_binary();

    let output = ctx
        .exec_fallible_cmd_with_output(container_bin, ["inspect", container_name])
        .await?;
    if !output.status.success() {
        return Ok(ContainerInspect::new_empty_container());
    }

    let inspect_info: linux::ContainerInspect = serde_json::from_slice(&output.stdout)
        .context("failed to deserialise container information")?;
    inspect_info.try_into()
}

pub(crate) async fn list_containers<T>(ctx: &LocalnetContext<T>) -> anyhow::Result<ContainersList> {
    let container_bin = container_binary();

    let output = ctx
        .exec_fallible_cmd_with_output(container_bin, ["container", "ls", "-a", "--format", "json"])
        .await?;
    if !output.status.success() {
        return Ok(ContainersList::new_empty());
    }
    // the output is per container so we need to split it
    let output_str = String::from_utf8(output.stdout)?;
    let containers = output_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<linux::ContainerListContainer>(l)
                .context("container info deserialisation failure")
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let containers_list = linux::ContainersList(containers);
    containers_list.try_into()
}
