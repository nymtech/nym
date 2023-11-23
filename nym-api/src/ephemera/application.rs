// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use ephemera::{
    configuration::Configuration,
    ephemera_api::{
        ApiBlock, ApiEphemeraMessage, Application, ApplicationResult, CheckBlockResult,
        RemoveMessages,
    },
};
use log::{debug, error, info};

use crate::ephemera::client::Client;
use crate::ephemera::epoch::Epoch;
use crate::ephemera::peers::members::MembersProvider;
use crate::ephemera::peers::{NymApiEphemeraPeerInfo, NymPeer};
use crate::ephemera::reward::aggregator::RewardsAggregator;
use crate::ephemera::reward::{EphemeraAccess, RewardManager};
use crate::ephemera::Args;
use crate::epoch_operations::MixnodeWithPerformance;
use crate::support::nyxd;
use ephemera::crypto::{EphemeraKeypair, EphemeraPublicKey, Keypair};
use ephemera::ephemera_api::CommandExecutor;
use ephemera::{Ephemera, EphemeraStarterInit};
use nym_task::TaskManager;

pub struct NymApi;

impl NymApi {
    pub(crate) async fn run(
        args: Args,
        ephemera_config: Configuration,
        nyxd_client: nyxd::Client,
        shutdown: &TaskManager,
    ) -> anyhow::Result<RewardManager> {
        //KEYPAIR - Ephemera keypair or Validator keypair
        //Can be a file, keystore etc
        let key_pair = Self::read_nym_api_keypair(&ephemera_config)?;

        //EPHEMERA
        let ephemera = Self::init_ephemera(ephemera_config, nyxd_client.clone()).await?;
        let ephemera_handle = ephemera.handle();

        //REWARDS
        let rewards =
            Self::create_rewards_manager(args, nyxd_client, key_pair, ephemera_handle.api.clone())
                .await?;

        //STARTING
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(ephemera.run(shutdown_listener));

        Ok(rewards)
    }

    pub(crate) async fn init_ephemera(
        ephemera_config: Configuration,
        nyxd_client: nyxd::Client,
    ) -> anyhow::Result<Ephemera<RewardsEphemeraApplication>> {
        info!("Initializing ephemera ...");

        let node_info = ephemera_config.node.clone();

        let keypair = bs58::decode(&node_info.private_key).into_vec().unwrap();
        let keypair = Keypair::from_bytes(&keypair).unwrap();
        let local_peer_id = keypair.public_key().to_base58();

        //Members provider for Ephemera
        let members_provider = MembersProvider::new(nyxd_client.clone());

        let cosmos_address = nyxd_client.client_address().await.to_string();
        let ip_address = format!(
            "/ip4/{}/tcp/{}",
            ephemera_config.node.ip, ephemera_config.libp2p.port
        );

        if !nyxd_client
            .get_ephemera_peers()
            .await?
            .iter()
            .any(|peer_info| peer_info.cosmos_address == cosmos_address)
        {
            let local_peer = NymPeer::new(
                cosmos_address,
                ip_address,
                keypair.public_key(),
                local_peer_id.clone(),
            );
            members_provider.register_peer(local_peer).await?;
        }

        //Application for Ephemera
        let rewards_ephemera_application =
            RewardsEphemeraApplication::init(local_peer_id, nyxd_client.clone()).await?;

        //EPHEMERA
        let ephemera_builder = EphemeraStarterInit::new(ephemera_config)?;
        let ephemera_builder = ephemera_builder.with_application(rewards_ephemera_application);
        let ephemera_builder = ephemera_builder.with_members_provider(members_provider)?;
        let ephemera = ephemera_builder.build();
        Ok(ephemera)
    }

    async fn create_rewards_manager(
        args: Args,
        nyxd_client: nyxd::Client,
        key_pair: Keypair,
        ephemera_api: CommandExecutor,
    ) -> anyhow::Result<RewardManager> {
        let epoch = Epoch::request_epoch(nyxd_client).await?;
        Ok(RewardManager::new(
            args.clone(),
            EphemeraAccess::new(ephemera_api, key_pair).into(),
            Some(RewardsAggregator),
            epoch,
        ))
    }

    fn read_nym_api_keypair(ephemera_config: &Configuration) -> anyhow::Result<Keypair> {
        let key_pair = bs58::decode(&ephemera_config.node.private_key).into_vec()?;
        let key_pair = Keypair::from_bytes(&key_pair)?;
        Ok(key_pair)
    }
}

pub(crate) struct RewardsEphemeraApplicationConfig {
    /// Percentage of messages relative to total number of peers
    pub(crate) peers_rewards_threshold: u64,
}

pub(crate) struct RewardsEphemeraApplication {
    peer_info: NymApiEphemeraPeerInfo,
    app_config: RewardsEphemeraApplicationConfig,
}

impl RewardsEphemeraApplication {
    pub(crate) async fn init(
        local_peer_id: String,
        nyxd_client: nyxd::Client,
    ) -> anyhow::Result<Self> {
        let peer_info = match NymApiEphemeraPeerInfo::from_ephemera_dev_cluster_conf(
            local_peer_id,
            nyxd_client,
        )
        .await
        {
            Ok(info) => info,
            Err(err) => {
                error!("Failed to load peers info: {}", err);
                return Err(err);
            }
        };
        let app_config = RewardsEphemeraApplicationConfig {
            peers_rewards_threshold: peer_info.get_peers_count() as u64,
        };
        Ok(Self {
            peer_info,
            app_config,
        })
    }
}

/// - TODO: We should also check that the messages has expected label(like epoch 100)
///         because next block should have only reward info for correct epoch.
impl Application for RewardsEphemeraApplication {
    /// Perform validation checks:
    /// - Check that the transaction has a valid signature, we don't want to accept garbage messages
    ///   or messages from unknown peers
    fn check_tx(&self, tx: ApiEphemeraMessage) -> ApplicationResult<bool> {
        if serde_json::from_slice::<Vec<MixnodeWithPerformance>>(&tx.data).is_err() {
            error!("Message is not a valid Reward message");
            return Ok(false);
        }
        Ok(true)

        //TODO
        //PS! message label should also be part of the message hash to prevent replay attacks
    }

    /// Agree to accept the block if it contains threshold number of transactions
    /// We trust that transactions are valid(checked by check_tx)
    fn check_block(&self, block: &ApiBlock) -> ApplicationResult<CheckBlockResult> {
        info!("Block message count: {}", block.message_count());

        let block_threshold = ((block.message_count() as f64
            / self.peer_info.get_peers_count() as f64)
            * 100.0) as u64;

        if block_threshold > 100 {
            error!("Block threshold is greater than 100%!. We expected only single message from each peer");
            return Ok(CheckBlockResult::RejectAndRemoveMessages(
                RemoveMessages::All,
            ));
        }

        if block_threshold >= self.app_config.peers_rewards_threshold {
            info!(
                "Block accepted {}:{}",
                block.header.height, block.header.hash
            );
            Ok(CheckBlockResult::Accept)
        } else {
            debug!("Block rejected: not enough messages");
            Ok(CheckBlockResult::Reject)
        }
    }

    /// It is possible to use this method as a callback to get notified when block is committed
    fn deliver_block(&self, _block: ApiBlock) -> ApplicationResult<()> {
        Ok(())
    }
}
