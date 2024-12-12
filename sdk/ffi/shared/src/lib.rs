// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Error, Result};
use lazy_static::lazy_static;
use nym_sdk::mixnet::{
    MixnetClient, MixnetClientBuilder, MixnetMessageSender, Recipient, ReconstructedMessage,
    StoragePaths,
};
use nym_sdk::tcp_proxy::{NymProxyClient, NymProxyServer};
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

// NYM_CLIENT/PROXIES: Static thread-safe reference (init once) to Option<Client>s.
// RUNTIME: Tokio runtime: no need to pass across FFI boundary and deal with raw pointers.
lazy_static! {
    static ref NYM_PROXY_CLIENT: Arc<Mutex<Option<NymProxyClient>>> = Arc::new(Mutex::new(None));
    static ref NYM_PROXY_SERVER: Arc<Mutex<Option<NymProxyServer>>> = Arc::new(Mutex::new(None));
    static ref NYM_CLIENT: Arc<Mutex<Option<MixnetClient>>> = Arc::new(Mutex::new(None));
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

// TODO create get_client() to use in fns and remove code repetition
fn get_client_as_ref() -> bool {
    NYM_CLIENT.lock().unwrap().as_ref().is_some()
}

fn get_client_guard() -> bool {
    todo!()
}

pub fn init_ephemeral_internal() -> Result<(), Error> {
    if NYM_CLIENT.lock().unwrap().as_ref().is_some() {
        bail!("client already exists");
    } else {
        RUNTIME.block_on(async move {
            let init_client = MixnetClient::connect_new().await?;
            let mut client = NYM_CLIENT.try_lock();
            if let Ok(ref mut client) = client {
                **client = Some(init_client);
            } else {
                return Err(anyhow!("couldnt lock ephemeral NYM_CLIENT"));
            }
            Ok::<(), Error>(())
        })?;
    }
    // if get_client_as_ref() {
    //     RUNTIME.block_on(async move {
    //         let init_client = MixnetClient::connect_new().await?;
    //         let mut client = NYM_CLIENT.try_lock();
    //         if let Ok(ref mut client) = client {
    //             **client = Some(init_client);
    //         } else {
    //             return Err(anyhow!("couldnt lock ephemeral NYM_CLIENT"));
    //         }
    //         Ok::<(), Error>(())
    //     })?;
    // } else {
    //     bail!("client already exists: no need to reinitialise");
    // }
    Ok(())
}

pub fn init_default_storage_internal(config_dir: PathBuf) -> Result<(), Error> {
    if NYM_CLIENT.lock().unwrap().as_ref().is_some() {
        bail!("client already exists");
    } else {
        RUNTIME.block_on(async move {
            let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();
            let init_client = MixnetClientBuilder::new_with_default_storage(storage_paths)
                .await?
                .build()?
                .connect_to_mixnet()
                .await?;
            let mut client = NYM_CLIENT.try_lock();
            if let Ok(ref mut client) = client {
                **client = Some(init_client);
            } else {
                return Err(anyhow!("couldnt lock NYM_CLIENT"));
            }
            Ok::<(), Error>(())
        })?;
    }
    Ok(())
}

pub fn get_self_address_internal() -> Result<String, Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    Ok(nym_client.nym_address().to_string())
}

// TODO split sender

pub fn send_message_internal(
    recipient: Recipient,
    message: &str,
    // TODO add Option<surb_amount>, if Some(surb_amount) call send_message() instead with specified #, else send_plain_message as this uses the default
) -> Result<(), Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    RUNTIME.block_on(async move {
        nym_client.send_plain_message(recipient, message).await?;
        Ok::<(), Error>(())
    })?;
    Ok(())
}

// TODO send_raw_message_internal

pub fn reply_internal(recipient: AnonymousSenderTag, message: &str) -> Result<(), Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    RUNTIME.block_on(async move {
        nym_client.send_reply(recipient, message).await?;
        Ok::<(), Error>(())
    })?;
    Ok(())
}

pub fn listen_for_incoming_internal() -> Result<ReconstructedMessage, Error> {
    let mut binding = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if binding.is_none() {
        bail!("recipient is null");
    }
    let client = binding
        .as_mut()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    let message = RUNTIME.block_on(async move {
        let received = wait_for_non_empty_message(client).await?;
        Ok::<ReconstructedMessage, Error>(ReconstructedMessage {
            message: received.message,
            sender_tag: received.sender_tag,
        })
    })?;

    Ok(message)
}

pub async fn wait_for_non_empty_message(client: &mut MixnetClient) -> Result<ReconstructedMessage> {
    while let Some(mut new_message) = client.wait_for_messages().await {
        if !new_message.is_empty() {
            return new_message
                .pop()
                .ok_or_else(|| anyhow!("could not get non empty message"));
        }
    }
    bail!("(Rust) did not receive any non-empty message")
}

pub fn proxy_client_new_internal(
    server_address: Recipient,
    listen_address: &str,
    listen_port: &str,
    close_timeout: u64,
    env: Option<String>,
    pool_size: usize,
) -> Result<(), Error> {
    if NYM_PROXY_CLIENT.lock().unwrap().as_ref().is_some() {
        bail!("proxy client already exists");
    } else {
        RUNTIME.block_on(async move {
            let init_proxy_client = NymProxyClient::new(
                server_address,
                listen_address,
                listen_port,
                close_timeout,
                env,
                pool_size,
            )
            .await?;
            let mut client = NYM_PROXY_CLIENT.try_lock();
            if let Ok(ref mut client) = client {
                **client = Some(init_proxy_client);
            } else {
                return Err(anyhow!("couldnt lock NYM_PROXY_CLIENT"));
            }
            Ok::<(), Error>(())
        })?;
    }
    Ok(())
}

pub fn proxy_client_new_defaults_internal(
    server_address: Recipient,
    env: Option<String>,
) -> Result<(), Error> {
    if NYM_PROXY_CLIENT.lock().unwrap().as_ref().is_some() {
        bail!("proxy client already exists");
    } else {
        RUNTIME.block_on(async move {
            let init_proxy_client = NymProxyClient::new_with_defaults(server_address, env).await?;
            let mut client = NYM_PROXY_CLIENT.try_lock();
            if let Ok(ref mut client) = client {
                **client = Some(init_proxy_client);
            } else {
                return Err(anyhow!("couldn't lock PROXY_CLIENT"));
            }
            Ok::<(), Error>(())
        })?;
    }
    Ok(())
}

pub fn proxy_client_run_internal() -> Result<(), Error> {
    let proxy_client = NYM_PROXY_CLIENT
        .lock()
        .expect("could not lock NYM_PROXY_CLIENT");
    if proxy_client.is_none() {
        bail!("Client is not yet initialised");
    }
    let proxy = proxy_client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get proxy_client as_ref()"))?;
    RUNTIME.block_on(async move {
        proxy.run().await?;
        Ok::<(), Error>(())
    })?;
    Ok(())
}

pub fn proxy_server_new_internal(
    upstream_address: &str,
    config_dir: &str,
    env: Option<String>,
) -> Result<(), Error> {
    if NYM_PROXY_SERVER.lock().unwrap().as_ref().is_some() {
        bail!("proxy client already exists");
    } else {
        RUNTIME.block_on(async move {
            let init_proxy_server = NymProxyServer::new(upstream_address, config_dir, env).await?;
            let mut client = NYM_PROXY_SERVER.try_lock();
            if let Ok(ref mut client) = client {
                **client = Some(init_proxy_server);
            } else {
                return Err(anyhow!("couldn't lock PROXY_SERVER"));
            }
            Ok::<(), Error>(())
        })?;
    }
    Ok(())
}

pub fn proxy_server_run_internal() -> Result<(), Error> {
    let mut proxy_server = NYM_PROXY_SERVER
        .lock()
        .expect("could not lock NYM_PROXY_CLIENT");
    if proxy_server.is_none() {
        bail!("Server is not yet initialised");
    }
    let proxy = proxy_server
        .as_mut()
        .ok_or_else(|| anyhow!("could not get proxy_client as_ref()"))?;
    RUNTIME.block_on(async move {
        proxy.run_with_shutdown().await?;
        Ok::<(), Error>(())
    })?;
    Ok(())
}

pub fn proxy_server_address_internal() -> Result<Recipient, Error> {
    let mut proxy_server = NYM_PROXY_SERVER
        .lock()
        .expect("could not lock NYM_PROXY_CLIENT");
    if proxy_server.is_none() {
        bail!("Server is not yet initialised");
    }
    let proxy = proxy_server
        .as_mut()
        .ok_or_else(|| anyhow!("could not get proxy_client as_ref()"))?;
    Ok(proxy.nym_address().to_owned())
}
