// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::monorepo_root_path;
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::container_helpers::{
    check_container_image_exists, default_nym_binaries_image_tag,
    load_image_into_container_runtime, save_docker_image,
};
use crate::orchestrator::context::LocalnetContext;
use anyhow::{Context, bail};
use std::path::PathBuf;
use tempfile::NamedTempFile;

pub(crate) struct Config {
    pub(crate) custom_tag: Option<String>,
    pub(crate) monorepo_root: Option<PathBuf>,
}

struct ImageRebuild {
    monorepo_root: PathBuf,
    nym_binaries_image_location: NamedTempFile,

    tag: String,
}

impl ImageRebuild {
    fn new(config: Config) -> anyhow::Result<Self> {
        let monorepo_root = monorepo_root_path(config.monorepo_root)?;

        let tag = config
            .custom_tag
            .unwrap_or(default_nym_binaries_image_tag(&monorepo_root)?);

        Ok(ImageRebuild {
            monorepo_root,
            nym_binaries_image_location: NamedTempFile::new()?,
            tag,
        })
    }

    fn monorepo_root_canon(&self) -> anyhow::Result<PathBuf> {
        Ok(self.monorepo_root.canonicalize()?)
    }

    fn nym_binaries_dockerfile_location_canon(&self) -> anyhow::Result<PathBuf> {
        Ok(self
            .monorepo_root
            .join("docker")
            .join("localnet")
            .join("nym-binaries-localnet.Dockerfile")
            .canonicalize()?)
    }

    fn image_temp_location_arg(&self) -> anyhow::Result<&str> {
        self.nym_binaries_image_location
            .path()
            .to_str()
            .context("invalid temporary file location")
    }
}

impl LocalnetOrchestrator {
    pub(crate) async fn rebuild_binaries_image(&self, config: Config) -> anyhow::Result<()> {
        let rebuild = ImageRebuild::new(config)?;
        let mut ctx = LocalnetContext::new(rebuild, 4, "\nrebuilding nym-binaries image");

        let dockerfile_path = ctx.data.nym_binaries_dockerfile_location_canon()?;
        let monorepo_path = ctx.data.monorepo_root_canon()?;
        let image_location = ctx.data.image_temp_location_arg()?.to_owned();
        let image_tag = ctx.data.tag.clone();

        // 1. docker build
        self.try_build_nym_binaries_docker_image(
            &mut ctx,
            dockerfile_path,
            monorepo_path,
            &image_tag,
        )
        .await?;

        // 2. docker save
        save_docker_image(&mut ctx, &image_location, &image_tag).await?;

        // 3. container load
        load_image_into_container_runtime(&mut ctx, &image_location).await?;

        // 4. container image inspect
        if !check_container_image_exists(&ctx, &image_tag).await? {
            bail!("localnet-nym-binaries image verification failed");
        }

        Ok(())
    }
}
