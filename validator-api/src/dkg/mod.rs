// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::dealing_processing::Processor;
use crate::dkg::events::Dispatcher;
use crate::dkg::main_loop::ProcessingLoop;
use crate::dkg::networking::receiver::Listener;
use crate::dkg::state::{DkgState, StateAccessor};
use crate::Client;
use crypto::asymmetric::identity;
use dkg::bte;
use futures::channel::mpsc;
use log::{error, info};
use rand::rngs::OsRng;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use validator_client::nymd::{SigningCosmWasmClient, SigningNymdClient};

mod dealing_processing;
pub(crate) mod error;
pub(crate) mod events;
mod main_loop;
pub(crate) mod networking;
mod smart_contract;
pub(crate) mod state;

pub(crate) struct Config {
    // not entirely sure what should go here just yet
    network_address: SocketAddr,
    contract_polling_interval: Duration,

    // note: perhaps it should go behind some wrapper so that if the epoch is updated,
    // the value stored on the disk would be overwritten
    decryption_key: bte::DecryptionKey,
    public_key: bte::PublicKeyWithProof,

    identity: identity::KeyPair,
}

async fn run_dkg<C>(config: Config, nyxd_client: Client<C>) -> anyhow::Result<()>
where
    C: SigningCosmWasmClient + Send + Sync + 'static,
{
    info!("initialising dkg...");
    let (dispatcher_sender, dispatcher_receiver) = mpsc::unbounded();
    let (contracts_events_sender, contracts_events_receiver) = mpsc::unbounded();
    let (dealing_sender, dealing_receiver) = mpsc::unbounded();

    // TODO: change it to attempt to load from a file first
    let state = DkgState::initialise_fresh(
        &nyxd_client,
        config.identity,
        config.decryption_key,
        config.public_key,
    )
    .await
    .map_err(|err| {
        error!("failed to initialise dkg state - {}", err);
        err
    })?;

    let state_accessor = StateAccessor::new(state.clone(), dispatcher_sender.clone());

    let mut event_dispatcher =
        Dispatcher::new(dispatcher_receiver, dealing_sender, contracts_events_sender);
    let mut dealing_processor = Processor::new(state.clone(), dealing_receiver);
    let contract_watcher = smart_contract::Watcher::new(
        nyxd_client.clone(),
        state_accessor.clone(),
        config.contract_polling_interval,
    );
    let publisher = smart_contract::Publisher::new(nyxd_client);
    let mut net_listener = Listener::new(config.network_address.clone(), state_accessor);

    let mut processing_loop = ProcessingLoop::new(
        state,
        dispatcher_sender,
        contracts_events_receiver,
        publisher,
        config.network_address,
    );

    tokio::spawn(async move { event_dispatcher.run().await });
    tokio::spawn(async move { net_listener.run().await });
    tokio::spawn(async move { dealing_processor.run().await });
    tokio::spawn(async move { contract_watcher.run().await });
    processing_loop.run().await;

    Ok(())
}

// upon startup, the following tasks will need to be spawned:
// - smart contract watcher
// - main loop processing
// - dealing processor
// - network listener
// - event dispatcher
// (possibly): network sender (if listens for events, otherwise under control of main loop)
// (possibly): contract publisher (if listens for events, otherwise under control of main loop)

// this only exists for purposes of local testing so that I wouldn't need to setup the entire valid API instance
// including validator connections

// in "proper" main, this would have been constructed elsewhere, but we just want to have something to work with now
fn make_client() -> Client<SigningNymdClient> {
    let mnemonic = std::env::var("MNEMONIC").unwrap();

    let validator_url = "http://localhost:26657".parse().unwrap();

    // this one is irrelevant as we don't need to call it
    let api_url = "http://localhost:8080".parse().unwrap();

    let contract_address = "nymt14hj2tavq8fpesdwxxcu44rty3hh90vhuysqrsr"
        .parse()
        .unwrap();

    let client_config = validator_client::Config::new(
        config::defaults::all::Network::QA,
        validator_url,
        api_url,
        None,
        None,
        None,
    )
    .with_coconut_dkg_contract(Some(contract_address));

    let inner = validator_client::Client::new_signing(client_config, mnemonic.parse().unwrap())
        .expect("Failed to connect to nymd!");

    Client(Arc::new(RwLock::new(inner)))
}

// note: for time being the entire dkg is only concerned about deriving a single shared scalar
// and not the entire coconut keypair
pub(crate) async fn dkg_only_main() -> anyhow::Result<()> {
    const POLLING_RATE: Duration = Duration::from_secs(10);
    let port = std::env::var("PORT").unwrap();
    let network_address = format!("localhost:{}", port)
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();

    let client = make_client();

    let mut rng = OsRng;
    let mut rng_07 = rand_07::rngs::OsRng;
    let dkg_params = dkg::bte::setup();

    let bte_keys = dkg::bte::keygen(&dkg_params, &mut rng);
    let ed25519_keys = identity::KeyPair::new(&mut rng_07);

    let dkg_config = Config {
        network_address,
        contract_polling_interval: POLLING_RATE,
        decryption_key: bte_keys.0,
        public_key: bte_keys.1,
        identity: ed25519_keys,
    };

    run_dkg(dkg_config, client).await
}
