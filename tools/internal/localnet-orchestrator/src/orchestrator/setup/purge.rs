// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::monorepo_root_path;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::container_helpers::{
    default_nym_binaries_image_tag, remove_container_image,
};
use crate::orchestrator::context::LocalnetContext;
use anyhow::Context;
use std::path::PathBuf;

struct LocalnetPurge {
    remove_images: bool,
    remove_cache: bool,
    monorepo_root: PathBuf,
}

impl LocalnetPurge {
    fn new(config: Config) -> anyhow::Result<LocalnetPurge> {
        let monorepo_root = monorepo_root_path(config.monorepo_root)?;

        Ok(LocalnetPurge {
            remove_images: config.remove_images,
            remove_cache: config.remove_cache,
            monorepo_root,
        })
    }
}

pub(crate) struct Config {
    pub(crate) remove_images: bool,
    pub(crate) remove_cache: bool,
    pub(crate) monorepo_root: Option<PathBuf>,
}

impl LocalnetOrchestrator {
    async fn remove_built_images(
        &self,
        ctx: &mut LocalnetContext<LocalnetPurge>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("removing all built images", "🔥");
        if !ctx.data.remove_images {
            ctx.println("\t NOT ENABLED - SKIPPING");
            return Ok(());
        }

        let nym_binaries_tag = default_nym_binaries_image_tag(&ctx.data.monorepo_root)?;

        // TODO: we need to dynamically determine tag for this
        // LOCALNET_NYXD_IMAGE_NAME.to_string()

        for tag in [nym_binaries_tag] {
            ctx.execute_cmd_with_stdout("docker", ["image", "rm", &tag])
                .await?;
            remove_container_image(ctx, &tag).await?;
        }

        Ok(())
    }

    fn remove_build_cache(&self, ctx: &mut LocalnetContext<LocalnetPurge>) -> anyhow::Result<()> {
        ctx.begin_next_step("removing build cache", "🔥");
        if !ctx.data.remove_cache {
            ctx.println("\t NOT ENABLED - SKIPPING");
            return Ok(());
        }

        self.storage.data_cache().clear()
    }

    async fn remove_storage_data(
        self,
        ctx: &mut LocalnetContext<LocalnetPurge>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("removing storage data", "🔥");

        std::fs::remove_dir_all(self.storage.localnet_directory())
            .context("missing main storage directory")?;
        let storage_db = self.storage.into_orchestrator_storage();
        let db_path = storage_db.stop().await?;
        std::fs::remove_file(db_path).context("missing database path")?;

        Ok(())
    }

    pub(crate) async fn purge_localnet(self, config: Config) -> anyhow::Result<()> {
        let purge = LocalnetPurge::new(config)?;
        let mut ctx = LocalnetContext::new(purge, 3, "\npurging localnet");

        // 1. stop the localnet
        self.stop_localnet().await?;

        // 2. remove docker (and container) images
        self.remove_built_images(&mut ctx).await?;

        // 3. remove build cache
        self.remove_build_cache(&mut ctx)?;

        // 4. remove all storage dir
        self.remove_storage_data(&mut ctx).await
    }
}
