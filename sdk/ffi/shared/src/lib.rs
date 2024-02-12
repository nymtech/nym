// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
use lazy_static::lazy_static;
use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, ReconstructedMessage, Recipient};
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use std::ffi::{c_char, c_int, CStr, CString};
use std::mem::forget;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

// NYM_CLIENT: Static reference (only init-ed once) to:
//     - Arc: share ownership
//     - Mutex: thread-safe way to share data between threads
//     - Option: init-ed or not
// RUNTIME: Tokio runtime: no need to pass back to C and deal with raw pointers as it was previously
lazy_static! {
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
                anyhow!("couldnt lock NYM_CLIENT");
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

    // send message
    RUNTIME.block_on(async move {
        nym_client.send_plain_message(recipient, message).await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

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

pub fn listen_for_incoming_internal() -> anyhow::Result<(), anyhow::Error> {
    let mut binding = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if binding.is_none() {
        bail!("recipient is null");
    }
    let client = binding
        .as_mut()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    // TODO return message out of this + entire fn
    RUNTIME.block_on(async move {
        let received = wait_for_non_empty_message(client).await?;

        // how to return received out of this? getting const/no-const errors

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
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
