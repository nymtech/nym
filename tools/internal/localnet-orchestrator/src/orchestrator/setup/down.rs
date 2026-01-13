// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::container_helpers::{list_containers, remove_container, stop_container};
use crate::orchestrator::context::LocalnetContext;

#[derive(Default)]
pub(crate) struct LocalnetDown {
    container_names: Vec<String>,
}

impl LocalnetOrchestrator {
    async fn get_localnet_container_names(
        &self,
        ctx: &mut LocalnetContext<LocalnetDown>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("establishing list of localnet containers", "🔍");

        let container_list = list_containers(ctx).await?;
        for container in container_list.containers {
            if container.image.contains("localnet") {
                ctx.data.container_names.push(container.name)
            }
        }
        Ok(())
    }

    async fn stop_localnet_containers(
        &self,
        ctx: &mut LocalnetContext<LocalnetDown>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("stopping localnet containers", "🛑");
        let count = ctx.data.container_names.len();

        for (i, container_name) in ctx.data.container_names.iter().enumerate() {
            ctx.set_pb_prefix(format!("[{}/{count}]", i + 1));
            ctx.set_pb_message(format!("stopping {container_name}"));
            stop_container(ctx, container_name).await?;
        }

        Ok(())
    }

    async fn remove_localnet_containers(
        &self,
        ctx: &mut LocalnetContext<LocalnetDown>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("removing localnet containers", "🔥");
        let count = ctx.data.container_names.len();

        for (i, container_name) in ctx.data.container_names.iter().enumerate() {
            ctx.set_pb_prefix(format!("[{}/{count}]", i + 1));
            ctx.set_pb_message(format!("removing {container_name}"));
            remove_container(ctx, container_name).await?;
        }

        Ok(())
    }

    pub(crate) async fn stop_localnet(&self) -> anyhow::Result<()> {
        let mut ctx = LocalnetContext::new(LocalnetDown::default(), 3, "\n stopping the localnet");

        self.get_localnet_container_names(&mut ctx).await?;
        self.stop_localnet_containers(&mut ctx).await?;
        self.remove_localnet_containers(&mut ctx).await?;

        Ok(())
    }
}
