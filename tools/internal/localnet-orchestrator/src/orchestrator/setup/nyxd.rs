// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{CONTAINER_NETWORK_NAME, LOCALNET_NYXD_IMAGE_NAME};
use crate::orchestrator::LocalnetOrchestrator;
use crate::orchestrator::account::Account;
use crate::orchestrator::container_helpers::{
    check_container_image_exists, get_container_ip_address, load_image_into_container_runtime,
    run_container, run_container_fallible, save_docker_image,
};
use crate::orchestrator::context::LocalnetContext;
use crate::orchestrator::network::NyxdDetails;
use crate::orchestrator::state::LocalnetState;
use anyhow::{Context, bail};
use std::fs;
use std::net::IpAddr;
use tempfile::NamedTempFile;
use tracing::info;
use url::Url;

pub(crate) struct Config {
    pub(crate) nyxd_repo: Url,
    pub(crate) nyxd_dockerfile_path: String,
    pub(crate) custom_dns: Option<String>,
    pub(crate) nyxd_tag: String,
}

struct NyxdSetup {
    config: Config,
    master_account: Account,
    nyxd_image_location: NamedTempFile,
    nyxd_ip: Option<IpAddr>,
}

impl NyxdSetup {
    pub(crate) fn new(config: Config) -> anyhow::Result<Self> {
        Ok(NyxdSetup {
            config,
            nyxd_image_location: NamedTempFile::new()
                .context("failed to create temporary file for nyxd image")?,
            master_account: Account::new(),
            nyxd_ip: None,
        })
    }

    pub(crate) fn image_tag(&self) -> String {
        format!("{LOCALNET_NYXD_IMAGE_NAME}:{}", self.config.nyxd_tag)
    }

    pub(crate) fn image_temp_location_arg(&self) -> anyhow::Result<&str> {
        self.nyxd_image_location
            .path()
            .to_str()
            .context("invalid temporary file location")
    }

    fn into_nyxd_details(self) -> anyhow::Result<NyxdDetails> {
        let ip = self.nyxd_ip.context("nyxd ip is not set")?;
        // for now the port is not configurable (it's not difficult to change that later)
        Ok(NyxdDetails {
            rpc_endpoint: format!("http://{ip}:26657").parse()?,
            master_account: self.master_account,
        })
    }
}

impl LocalnetOrchestrator {
    async fn build_nyxd_docker_image(
        &self,
        ctx: &mut LocalnetContext<NyxdSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step(
            "building nyxd docker image... this might take few minutes...",
            "🏗️",
        );
        let cfg = &ctx.data.config;
        ctx.execute_cmd_with_exit_status(
            "docker",
            [
                "build",
                "--platform",
                "linux/amd64",
                "-f",
                &cfg.nyxd_dockerfile_path,
                &format!("{}#{}", cfg.nyxd_repo, cfg.nyxd_tag),
                "-t",
                &ctx.data.image_tag(),
            ],
        )
        .await?;
        Ok(())
    }

    async fn save_nyxd_docker_image(
        &self,
        ctx: &mut LocalnetContext<NyxdSetup>,
    ) -> anyhow::Result<()> {
        let output_path = ctx.data.image_temp_location_arg()?.to_owned();
        let image_tag = ctx.data.image_tag();

        save_docker_image(ctx, &output_path, &image_tag).await
    }

    async fn load_nyxd_into_container_runtime(
        &self,
        ctx: &mut LocalnetContext<NyxdSetup>,
    ) -> anyhow::Result<()> {
        let image_path = ctx.data.image_temp_location_arg()?.to_owned();
        load_image_into_container_runtime(ctx, &image_path).await
    }

    async fn verify_nyxd_image(&self, ctx: &mut LocalnetContext<NyxdSetup>) -> anyhow::Result<()> {
        ctx.begin_next_step("verifying nyxd container image...", "❔");

        if !check_container_image_exists(ctx, &ctx.data.image_tag()).await? {
            bail!("nyxd image verification failed");
        }
        Ok(())
    }

    async fn check_genesis_exists(
        &self,
        ctx: &mut LocalnetContext<NyxdSetup>,
    ) -> anyhow::Result<bool> {
        let status = run_container_fallible(
            ctx,
            [
                "--name",
                &self.nyxd_container_name(),
                "-v",
                &self.nyxd_volume(),
                "--rm",
                &ctx.data.image_tag(),
                "test",
                "-f",
                "/root/.nyxd/config/genesis.json",
            ],
        )
        .await?;
        Ok(status.success())
    }

    async fn initialise_nyxd_data(
        &self,
        ctx: &mut LocalnetContext<NyxdSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("initialising nyxd data...", "📝");

        ctx.set_pb_prefix("[1/2]");
        ctx.set_pb_message("generating nyxd config...");

        // unfortunately we have to do it manually as scripts embedded in the image
        // (as of v0.60.1) either do not directly expose the genesis mnemonic
        // or assume joining existing consensus as opposed starting from genesis
        // (and yes, technically `sed` could have been replaced by just directly modifying the files
        // on disk, but why break the tradition?)
        //
        // and why is it split into 2 commands?
        // because it made the whole thing easier due to the interactive prompts for key import
        let init_cmd1 = format!(
            r#"
            nyxd init nyx --chain-id nyx

            sed -i "s/\"stake\"/\"unyx\"/" "/root/.nyxd/config/genesis.json"
            sed -i 's/minimum-gas-prices = "0stake"/minimum-gas-prices = "0.025unym"/' "/root/.nyxd/config/app.toml"
            sed -i '0,/enable = false/s//enable = true/g' "/root/.nyxd/config/app.toml"

            sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = \["*"\]/' "/root/.nyxd/config/config.toml"
            sed -i 's/create_empty_blocks = true/create_empty_blocks = false/' "/root/.nyxd/config/config.toml"
            sed -i 's/laddr = "tcp:\/\/127.0.0.1:26657"/laddr = "tcp:\/\/0.0.0.0:26657"/' "/root/.nyxd/config/config.toml"
            sed -i 's/address = "tcp:\/\/localhost:1317"/address = "tcp:\/\/0.0.0.0:1317"/' "/root/.nyxd/config/app.toml"

            sed -i 's/timeout_propose = "3s"/timeout_propose = "500ms"/' "/root/.nyxd/config/config.toml"
            sed -i 's/timeout_propose_delta = "500ms"/timeout_propose_delta = "50ms"/' "/root/.nyxd/config/config.toml"
            sed -i 's/timeout_prevote = "1s"/timeout_prevote = "200ms"/' "/root/.nyxd/config/config.toml"
            sed -i 's/timeout_prevote_delta = "500ms"/timeout_prevote_delta = "50ms"/' "/root/.nyxd/config/config.toml"
            sed -i 's/timeout_precommit = "1s"/timeout_precommit = "200ms"/' "/root/.nyxd/config/config.toml"
            sed -i 's/timeout_precommit_delta = "500ms"/timeout_precommit_delta = "50ms"/' "/root/.nyxd/config/config.toml"
            sed -i 's/timeout_commit = "5s"/timeout_commit = "1s"/' "/root/.nyxd/config/config.toml"

            cat << 'EOF' | nyxd keys add -i {}-admin
            {}

            password
            password
            EOF"
        "#,
            self.localnet_details.human_name, ctx.data.master_account.mnemonic
        );

        run_container(
            ctx,
            [
                "--name",
                &self.nyxd_container_name(),
                "-v",
                &self.nyxd_volume(),
                "--rm",
                &ctx.data.image_tag(),
                "sh",
                "-c",
                &init_cmd1,
            ],
            ctx.data.config.custom_dns.clone(),
        )
        .await?;

        ctx.set_pb_prefix("[2/2]");
        ctx.set_pb_message("generating genesis file...");

        let init_cmd2 = format!(
            r#"
            yes password | nyxd genesis add-genesis-account {}-admin 1000000000000000unym,1000000000000000unyx
            yes password | nyxd genesis gentx {}-admin 100000000000unyx --chain-id nyx
            nyxd genesis collect-gentxs
            nyxd genesis validate-genesis
        "#,
            self.localnet_details.human_name, self.localnet_details.human_name,
        );

        run_container(
            ctx,
            [
                "--name",
                &self.nyxd_container_name(),
                "-v",
                &self.nyxd_volume(),
                "--rm",
                &ctx.data.image_tag(),
                "sh",
                "-c",
                &init_cmd2,
            ],
            ctx.data.config.custom_dns.clone(),
        )
        .await?;
        Ok(())
    }

    async fn start_nyxd(&self, ctx: &mut LocalnetContext<NyxdSetup>) -> anyhow::Result<()> {
        ctx.begin_next_step("spawning nyxd container", "🚀");

        run_container(
            ctx,
            [
                "--name",
                &self.nyxd_container_name(),
                "-v",
                &self.nyxd_volume(),
                "--network",
                CONTAINER_NETWORK_NAME,
                "-p",
                // TEMP: expose tendermint rpc port to make our setup life easier
                "26657:26657",
                "-d",
                &ctx.data.image_tag(),
                "nyxd",
                "start",
            ],
            ctx.data.config.custom_dns.clone(),
        )
        .await?;

        Ok(())
    }

    async fn finalize_nyxd_setup(
        &mut self,
        mut ctx: LocalnetContext<NyxdSetup>,
    ) -> anyhow::Result<()> {
        ctx.begin_next_step("persisting nyxd details", "📝");

        let container_ip = get_container_ip_address(&ctx, &self.nyxd_container_name()).await?;
        ctx.data.nyxd_ip = Some(container_ip);

        let nyxd_details = ctx.data.into_nyxd_details()?;
        self.storage
            .orchestrator()
            .save_nyxd_details(&nyxd_details)
            .await?;

        self.localnet_details.set_nyxd_details(nyxd_details);
        self.state = LocalnetState::RunningNyxd;

        Ok(())
    }

    pub(crate) async fn initialise_nyxd(&mut self, config: Config) -> anyhow::Result<()> {
        let setup = NyxdSetup::new(config)?;
        let mut ctx = LocalnetContext::new(setup, 7, "\ninitialising new nyxd instance");
        fs::create_dir_all(self.storage.nyxd_container_data_directory())
            .context("failed to create nyxd data directory")?;

        // 0.1 check if we have to do anything
        if self.check_nyxd_container_is_running(&ctx).await? {
            info!("nyxd instance for this localnet is already running");
            return Ok(());
        }

        // 0.2 check if container had already been built
        let image_tag = &ctx.data.image_tag();
        if check_container_image_exists(&ctx, image_tag).await? {
            info!(
                "'{image_tag}' container image already exists - skipping docker build and import",
            );
            ctx.skip_steps(4);
        } else {
            // 1. docker build
            self.build_nyxd_docker_image(&mut ctx).await?;

            // 2. docker save
            self.save_nyxd_docker_image(&mut ctx).await?;

            // 3. container load
            self.load_nyxd_into_container_runtime(&mut ctx).await?;

            // 4. container image inspect
            self.verify_nyxd_image(&mut ctx).await?;
        }

        // 5.1 check if genesis.json exists, i.e. chain had been initialised
        if self.check_genesis_exists(&mut ctx).await? {
            info!(
                "'{}' already had its genesis generated - skipping the process",
                self.nyxd_container_name()
            );
            ctx.skip_steps(1);
        } else {
            // 5.2 perform nyxd init, gentx, etc.
            self.initialise_nyxd_data(&mut ctx).await?;
        }

        // 6. start nyxd in the background
        self.start_nyxd(&mut ctx).await?;

        // 7. persist relevant information and update local state
        self.finalize_nyxd_setup(ctx).await?;

        Ok(())
    }
}
