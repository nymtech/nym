use anyhow::anyhow;
use std::str::FromStr;
use std::sync::Arc;

use clap::Parser;
use log::trace;
use nym_task::TaskManager;
use reqwest::Url;

use crate::ephemera_api::ApplicationResult;
use crate::utilities::codec::{Codec, EphemeraCodec};
use crate::{
    api::application::CheckBlockResult,
    cli::PEERS_CONFIG_FILE,
    config::Configuration,
    crypto::EphemeraKeypair,
    crypto::Keypair,
    ephemera_api::{ApiBlock, ApiEphemeraMessage, Application, Dummy, RawApiEphemeraMessage},
    network::members::ConfigMembersProvider,
    EphemeraStarterInit,
};

#[derive(Clone, Debug)]
pub struct HttpMembersProviderArg {
    pub url: Url,
}

impl FromStr for HttpMembersProviderArg {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HttpMembersProviderArg { url: s.parse()? })
    }
}

#[derive(Parser)]
pub struct RunExternalNodeCmd {
    #[clap(short, long)]
    pub config_file: String,
    #[clap(short, long)]
    pub peers_config: String,
}

impl RunExternalNodeCmd {
    /// # Errors
    /// If the members provider cannot be created.
    ///
    /// # Panics
    /// If the ephemera cannot be created.
    pub async fn execute(&self) -> anyhow::Result<()> {
        let ephemera_conf = match Configuration::try_load(self.config_file.clone()) {
            Ok(conf) => conf,
            Err(err) => anyhow::bail!("Error loading configuration file: {err:?}"),
        };

        let members_provider = Self::config_members_provider_with_path(self.peers_config.clone())?;
        let ephemera = EphemeraStarterInit::new(ephemera_conf.clone())
            .unwrap()
            .with_application(Dummy)
            .with_members_provider(members_provider)?
            .build();

        let mut shutdown = TaskManager::new(10);

        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(ephemera.run(shutdown_listener));

        if let Err(err) = shutdown.catch_interrupt().await {
            Err(anyhow!("Shutdown error {:?}", err))
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn config_members_provider() -> anyhow::Result<ConfigMembersProvider> {
        let peers_conf_path = Configuration::ephemera_root_dir(None)
            .unwrap()
            .join(PEERS_CONFIG_FILE);

        let peers_conf = match ConfigMembersProvider::init(peers_conf_path) {
            Ok(conf) => conf,
            Err(err) => anyhow::bail!("Error loading peers file: {err:?}"),
        };
        Ok(peers_conf)
    }

    #[allow(dead_code)]
    fn config_members_provider_with_path(
        peers_conf_path: String,
    ) -> anyhow::Result<ConfigMembersProvider> {
        let peers_conf = match ConfigMembersProvider::init(peers_conf_path) {
            Ok(conf) => conf,
            Err(err) => anyhow::bail!("Error loading peers file: {err:?}"),
        };
        Ok(peers_conf)
    }
}

pub struct SignatureVerificationApplication {
    keypair: Arc<Keypair>,
}

impl SignatureVerificationApplication {
    #[must_use]
    pub fn new(keypair: Arc<Keypair>) -> Self {
        Self { keypair }
    }

    pub(crate) fn verify_message(&self, msg: ApiEphemeraMessage) -> anyhow::Result<()> {
        let signature = msg.certificate.clone();
        let raw_message: RawApiEphemeraMessage = msg.into();
        let encoded_message = Codec::encode(&raw_message)?;
        if self
            .keypair
            .verify(&encoded_message, &signature.signature.into())
        {
            Ok(())
        } else {
            anyhow::bail!("Invalid signature")
        }
    }
}

impl Application for SignatureVerificationApplication {
    fn check_tx(&self, tx: ApiEphemeraMessage) -> ApplicationResult<bool> {
        trace!("SignatureVerificationApplicationHook::check_tx");
        self.verify_message(tx)?;
        Ok(true)
    }

    fn check_block(&self, _block: &ApiBlock) -> ApplicationResult<CheckBlockResult> {
        Ok(CheckBlockResult::Accept)
    }

    fn deliver_block(&self, _block: ApiBlock) -> ApplicationResult<()> {
        trace!("SignatureVerificationApplicationHook::deliver_block");
        Ok(())
    }
}
