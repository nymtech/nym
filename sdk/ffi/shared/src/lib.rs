// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
use lazy_static::lazy_static;
use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, ReconstructedMessage};
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use std::ffi::{c_char, CStr, CString};
use std::mem::forget;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

/*
NYM_CLIENT: Static reference (only init-ed once) to:
    - Arc: share ownership
    - Mutex: thread-safe way to share data between threads
    - Option: init-ed or not
RUNTIME: Tokio runtime: no need to pass back to C and deal with raw pointers as it was previously
 */
lazy_static! {
    static ref NYM_CLIENT: Arc<Mutex<Option<MixnetClient>>> = Arc::new(Mutex::new(None));
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

#[derive(Debug)]
pub enum StatusCode {
    NoError = 0,
    ClientInitError = -1,
    ClientUninitialisedError = -2,
    SelfAddrError = -3,
    SendMsgError = -4,
    ReplyError = -5,
    ListenError = -6,
}

pub type CStringCallback = extern "C" fn(*const c_char);
pub type CMessageCallback = extern "C" fn(ReceivedMessage);

// FFI-sanitised ReconstructedMessage
#[repr(C)]
pub struct ReceivedMessage {
    message: *const u8,
    size: usize,
    sender_tag: *const c_char,
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

pub fn get_self_address_internal(callback: CStringCallback) -> anyhow::Result<(), anyhow::Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;
    // get address as cstring
    let c_string = CString::new(nym_client.nym_address().to_string())?;
    // as_ptr() keeps ownership in rust unlike into_raw() so no need to free it
    callback(c_string.as_ptr());
    Ok(())
}

pub fn send_message_internal(
    recipient: *const c_char,
    message: *const c_char,
) -> anyhow::Result<(), anyhow::Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    let c_str = unsafe {
        if recipient.is_null() {
            bail!("recipient is null");
        }
        let c_str = CStr::from_ptr(recipient);
        c_str
    };
    let r_str = c_str.to_str().unwrap();
    let recipient = r_str.parse().unwrap();
    let c_str = unsafe {
        if message.is_null() {
            bail!("message is null");
        }
        let c_str = CStr::from_ptr(message);
        c_str
    };
    let message = c_str.to_str().unwrap();

    // send message
    RUNTIME.block_on(async move {
        nym_client.send_plain_message(recipient, message).await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

pub fn reply_internal(
    recipient: *const c_char,
    message: *const c_char,
) -> anyhow::Result<(), anyhow::Error> {
    let client = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if client.is_none() {
        bail!("Client is not yet initialised");
    }
    let nym_client = client
        .as_ref()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    let recipient = unsafe {
        if recipient.is_null() {
            bail!("recipient is null");
        }
        let r_str = CStr::from_ptr(recipient).to_string_lossy().into_owned();
        AnonymousSenderTag::try_from_base58_string(r_str)
            .expect("could not construct AnonymousSenderTag from supplied value")
    };
    let message = unsafe {
        if message.is_null() {
            bail!("message is null");
        }
        let c_str = CStr::from_ptr(message);
        let r_str = c_str.to_str().unwrap();
        r_str
    };
    RUNTIME.block_on(async move {
        nym_client.send_reply(recipient, message).await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

pub fn listen_for_incoming_internal(callback: CMessageCallback) -> anyhow::Result<(), anyhow::Error> {
    let mut binding = NYM_CLIENT.lock().expect("could not lock NYM_CLIENT");
    if binding.is_none() {
        bail!("recipient is null");
    }
    let client = binding
        .as_mut()
        .ok_or_else(|| anyhow!("could not get client as_ref()"))?;

    RUNTIME.block_on(async move {
        let received = wait_for_non_empty_message(client).await?;
        let message_ptr = received.message.as_ptr();
        let message_length = received.message.len();
        let c_string = CString::new(received.sender_tag.unwrap().to_string())?;
        let sender_ptr = c_string.as_ptr();
        // stop deallocation when out of scope as passing raw ptr to it elsewhere
        forget(received);
        let rec_for_c = ReceivedMessage {
            message: message_ptr,
            size: message_length,
            sender_tag: sender_ptr,
        };
        callback(rec_for_c);
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
                .ok_or_else(|| anyhow!("could not get client as_ref()"));
        }
    }
    bail!("(Rust) did not receive any non-empty message")
}
