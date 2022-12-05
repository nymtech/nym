// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::error::Socks5ClientError;
use crate::socks::{
    authentication::{AuthenticationMethods, Authenticator, User},
    server::SphinxSocksServer,
};
use client_core::client::base_client::{BaseClientBuilder, ClientInput, ClientOutput};
use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use gateway_client::bandwidth::BandwidthController;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use std::error::Error;
use task::{wait_for_signal_and_error, ShutdownListener, ShutdownNotifier};

pub mod config;

// Channels used to control the main task from outside
pub type Socks5ControlMessageSender = mpsc::UnboundedSender<Socks5ControlMessage>;
pub type Socks5ControlMessageReceiver = mpsc::UnboundedReceiver<Socks5ControlMessage>;

#[derive(Debug)]
pub enum Socks5ControlMessage {
    /// Tell the main task to stop
    Stop,
}

pub struct NymClient {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,
}

impl NymClient {
    pub fn new(config: Config) -> Self {
        let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
        let key_manager = KeyManager::load_keys(&pathfinder).expect("failed to load stored keys");

        NymClient {
            config,
            key_manager,
        }
    }

    async fn create_bandwidth_controller(config: &Config) -> BandwidthController {
        #[cfg(feature = "coconut")]
        let bandwidth_controller = {
            let details = network_defaults::NymNetworkDetails::new_from_env();
            let client_config = validator_client::Config::try_from_nym_network_details(&details)
                .expect("failed to construct validator client config");
            let client = validator_client::Client::new_query(client_config)
                .expect("Could not construct query client");
            let coconut_api_clients =
                validator_client::CoconutApiClient::all_coconut_api_clients(&client)
                    .await
                    .expect("Could not query api clients");
            BandwidthController::new(
                credential_storage::initialise_storage(config.get_base().get_database_path()).await,
                coconut_api_clients,
            )
        };
        #[cfg(not(feature = "coconut"))]
        let bandwidth_controller = BandwidthController::new(
            credential_storage::initialise_storage(config.get_base().get_database_path()).await,
        )
        .expect("Could not create bandwidth controller");
        bandwidth_controller
    }

    fn start_socks5_listener(
        config: &Config,
        client_input: ClientInput,
        client_output: ClientOutput,
        self_address: Recipient,
        mut shutdown: ShutdownListener,
    ) {
        info!("Starting socks5 listener...");
        let auth_methods = vec![AuthenticationMethods::NoAuth as u8];
        let allowed_users: Vec<User> = Vec::new();

        let ClientInput {
            shared_lane_queue_lengths,
            connection_command_sender,
            input_sender,
        } = client_input;

        let received_buffer_request_sender = client_output.received_buffer_request_sender;

        let authenticator = Authenticator::new(auth_methods, allowed_users);
        let mut sphinx_socks = SphinxSocksServer::new(
            config.get_listening_port(),
            authenticator,
            config.get_provider_mix_address(),
            self_address,
            shared_lane_queue_lengths,
            shutdown.clone(),
        );
        //tokio::spawn(async move {
        //    // Ideally we should have a fully fledged task manager to check for errors in all
        //    // tasks.
        //    // However, pragmatically, we start out by at least reporting errors for some of the
        //    // tasks that interact with the outside world and can fail in normal operation, such as
        //    // network issues.
        //    // TODO: replace this by a generic solution, such as a task manager that stores all
        //    // JoinHandles of all spawned tasks.
        //    if let Err(res) = sphinx_socks
        //        .serve(
        //            input_sender,
        //            received_buffer_request_sender,
        //            connection_command_sender,
        //        )
        //        .await
        //    {
        //        shutdown.send_we_stopped(Box::new(res));
        //    }
        //});

        //let f = async move {
        //    // Ideally we should have a fully fledged task manager to check for errors in all
        //    // tasks.
        //    // However, pragmatically, we start out by at least reporting errors for some of the
        //    // tasks that interact with the outside world and can fail in normal operation, such as
        //    // network issues.
        //    // TODO: replace this by a generic solution, such as a task manager that stores all
        //    // JoinHandles of all spawned tasks.
        //    if let Err(res) = sphinx_socks
        //        .serve(
        //            input_sender,
        //            received_buffer_request_sender,
        //            connection_command_sender,
        //        )
        //        .await
        //    {
        //        shutdown.send_we_stopped(Box::new(res));
        //    }
        //};

        let ff = sphinx_socks
            .serve(
                input_sender,
                received_buffer_request_sender,
                connection_command_sender,
            )
            .boxed();
        //let b = Box::pin(ff);
        //let box_fut = BoxFuture::new(b);
        spawn_with_return(ff);
    }

    /// blocking version of `start` method. Will run forever (or until SIGINT is sent)
    pub async fn run_forever(self) -> Result<(), Box<dyn Error + Send>> {
        let mut shutdown = self
            .start()
            .await
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let res = wait_for_signal_and_error(&mut shutdown).await;

        log::info!("Sending shutdown");
        shutdown.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym-socks5-client");
        res
    }

    // Variant of `run_forever` that listends for remote control messages
    pub async fn run_and_listen(
        self,
        mut receiver: Socks5ControlMessageReceiver,
    ) -> Result<(), Box<dyn Error + Send>> {
        // Start the main task
        let mut shutdown = self
            .start()
            .await
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let res = tokio::select! {
            biased;
            message = receiver.next() => {
                log::debug!("Received message: {:?}", message);
                match message {
                    Some(Socks5ControlMessage::Stop) => {
                        log::info!("Received stop message");
                    }
                    None => {
                        log::info!("Channel closed, stopping");
                    }
                }
                Ok(())
            }
            Some(msg) = shutdown.wait_for_error() => {
                log::info!("Task error: {:?}", msg);
                Err(msg)
            }
            _ = tokio::signal::ctrl_c() => {
                log::info!("Received SIGINT");
                Ok(())
            },
        };

        log::info!("Sending shutdown");
        shutdown.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym-socks5-client");
        res
    }

    pub async fn start(self) -> Result<ShutdownNotifier, Socks5ClientError> {
        let base_builder = BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            Some(Self::create_bandwidth_controller(&self.config).await),
        );

        let self_address = base_builder.as_mix_recipient();
        let mut started_client = base_builder.start_base().await?;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_socks5_listener(
            &self.config,
            client_input,
            client_output,
            self_address,
            started_client.shutdown_notifier.subscribe(),
        );

        info!("Client startup finished!");
        info!("The address of this client is: {}", self_address);

        Ok(started_client.shutdown_notifier)
    }
}

//fn spawn_with_return<F, T, S>(future: F)
//where
//    F: std::future::Future<Output = Result<T, S>> + Send + 'static,
//    F::Output: Send + 'static,
//    //S: 'static,
//    //T: 'static,
//{
//    let f = async move {
//        if let Err(_err) = future.await {
//            println!("error");
//        }
//    };
//    tokio::spawn(f);
//}

fn spawn_with_return<T, E>(future: BoxFuture<'static, Result<T, E>>)
where
    T: 'static,
    E: 'static,
{
    let f = async move {
        if let Err(_err) = future.await {
            println!("error");
        }
    };
    tokio::spawn(f);
}
