use log::info;

use crate::support::nyxd;
use ephemera::configuration::Configuration;
use ephemera::crypto::{EphemeraKeypair, Keypair};
use ephemera::ephemera_api::CommandExecutor;
use ephemera::membership::HttpMembersProvider;
use ephemera::{Ephemera, EphemeraStarterInit};

use super::application::application::RewardsEphemeraApplication;
use super::epoch::Epoch;
use super::reward::aggregator::RewardsAggregator;
use super::reward::{EphemeraAccess, RewardManager};
use super::Args;

pub(crate) mod application;

pub struct NymApi;

impl NymApi {
    pub(crate) async fn run(
        args: Args,
        ephemera_config: Configuration,
        nyxd_client: nyxd::Client,
    ) -> anyhow::Result<RewardManager> {
        info!(
            "Starting nym api with ephemera {} ...",
            args.ephemera_config
        );
        //KEYPAIR - Ephemera keypair or Validator keypair
        //Can be a file, keystore etc
        let key_pair = Self::read_nym_api_keypair(&ephemera_config)?;

        //EPHEMERA
        let ephemera = Self::init_ephemera(&args, ephemera_config).await?;
        let ephemera_handle = ephemera.handle();

        //REWARDS
        let rewards =
            Self::create_rewards_manager(args, key_pair, nyxd_client, ephemera_handle.api.clone())
                .await;

        //STARTING
        info!("Starting Nym-Api services");
        let _ephemera_task = tokio::spawn(ephemera.run());

        Ok(rewards)
    }

    pub(crate) async fn init_ephemera(
        args: &Args,
        ephemera_config: Configuration,
    ) -> anyhow::Result<Ephemera<RewardsEphemeraApplication>> {
        info!("Initializing ephemera ...");

        //Application for Ephemera
        let rewards_ephemera_application =
            RewardsEphemeraApplication::init(ephemera_config.clone())?;

        //Members provider for Ephemera
        let url = format!("http://{}/contract/peer_info", args.smart_contract_url);
        let members_provider = HttpMembersProvider::new(url);

        //EPHEMERA
        let ephemera_builder = EphemeraStarterInit::new(ephemera_config)?;
        let ephemera_builder = ephemera_builder.with_application(rewards_ephemera_application);
        let ephemera_builder = ephemera_builder.with_members_provider(members_provider)?;
        let ephemera = ephemera_builder.build();
        Ok(ephemera)
    }

    async fn create_rewards_manager(
        args: Args,
        key_pair: Keypair,
        nyxd_client: nyxd::Client,
        ephemera_api: CommandExecutor,
    ) -> RewardManager {
        let epoch = Epoch::request_epoch(args.smart_contract_url.clone()).await;
        let rewards = RewardManager::new(
            nyxd_client,
            args.clone(),
            EphemeraAccess::new(ephemera_api, key_pair).into(),
            Some(RewardsAggregator),
            epoch,
        );
        rewards
    }

    fn read_nym_api_keypair(ephemera_config: &Configuration) -> anyhow::Result<Keypair> {
        let key_pair = bs58::decode(&ephemera_config.node.private_key).into_vec()?;
        let key_pair = Keypair::from_bytes(&key_pair)?;
        Ok(key_pair)
    }
}
