// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
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

// NYM_CLIENT/PROXIES: Static reference (only init-ed once) to:
//     - Arc: share ownership
//     - Mutex: thread-safe way to share data between threads
//     - Option: init-ed or not
// RUNTIME: Tokio runtime: no need to pass back to C and deal with raw pointers as it was previously
lazy_static! {
    static ref NYM_PROXY_CLIENT: Arc<Mutex<Option<NymProxyClient>>> = Arc::new(Mutex::new(None));
    static ref NYM_PROXY_SERVER: Arc<Mutex<Option<NymProxyServer>>> = Arc::new(Mutex::new(None));
    static ref NYM_CLIENT: Arc<Mutex<Option<MixnetClient>>> = Arc::new(Mutex::new(None));
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

pub fn init_ephemeral_internal() -> anyhow::Result<(), anyhow::Error> {
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
            Ok::<(), anyhow::Error>(())
        })?;
    }
    Ok(())
}

pub fn init_default_storage_internal(config_dir: PathBuf) -> anyhow::Result<(), anyhow::Error> {
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
            Ok::<(), anyhow::Error>(())
        })?;
    }
    Ok(())
}

pub fn get_self_address_internal() -> anyhow::Result<String, anyhow::Error> {
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
) -> anyhow::Result<(), anyhow::Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    RUNTIME.block_on(async move {
        nym_client.send_plain_message(recipient, message).await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

// TODO send_raw_message_internal

pub fn reply_internal(
    recipient: AnonymousSenderTag,
    message: &str,
) -> anyhow::Result<(), anyhow::Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    RUNTIME.block_on(async move {
        nym_client.send_reply(recipient, message).await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

pub fn listen_for_incoming_internal() -> anyhow::Result<ReconstructedMessage, anyhow::Error> {
    let mut binding = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if binding.is_none() {
        bail!("recipient is null");
    }
    let client = binding
        .as_mut()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    let message = RUNTIME.block_on(async move {
        let received = wait_for_non_empty_message(client).await?;
        Ok::<ReconstructedMessage, anyhow::Error>(ReconstructedMessage {
            message: received.message,
            sender_tag: received.sender_tag,
        })
    })?;

    Ok(message)
}

pub async fn wait_for_non_empty_message(
    client: &mut MixnetClient,
) -> anyhow::Result<ReconstructedMessage> {
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
) -> anyhow::Result<(), anyhow::Error> {
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
            )
            .await?;
            let mut client = NYM_PROXY_CLIENT.try_lock();
            if let Ok(ref mut client) = client {
                **client = Some(init_proxy_client);
            } else {
                return Err(anyhow!("couldnt lock NYM_PROXY_CLIENT"));
            }
            Ok::<(), anyhow::Error>(())
        })?;
    }
    Ok(())
}

pub fn proxy_client_new_defaults_internal(
    server_address: Recipient,
    env: Option<String>,
) -> anyhow::Result<(), anyhow::Error> {
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
            Ok::<(), anyhow::Error>(())
        })?;
    }
    Ok(())
}

pub fn proxy_client_run_internal() -> anyhow::Result<(), anyhow::Error> {
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
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

pub fn proxy_server_new_internal(
    upstream_address: &str,
    config_dir: &str,
    env: Option<String>,
) -> anyhow::Result<(), anyhow::Error> {
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
            Ok::<(), anyhow::Error>(())
        })?;
    }
    Ok(())
}

pub fn proxy_server_run_internal() -> anyhow::Result<(), anyhow::Error> {
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
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

pub fn proxy_server_address_internal() -> anyhow::Result<Recipient, anyhow::Error> {
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
