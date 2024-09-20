// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet::Recipient;
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
uniffi::include_scaffolding!("bindings");

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
enum GoWrapError {
    #[error("Couldn't init client")]
    ClientInitError {},
    // #[error("Client is uninitialised: init client first")]
    // ClientUninitialisedError {},
    #[error("Error getting self address")]
    SelfAddrError {},
    #[error("Error sending message")]
    SendMsgError {},
    #[error("Error sending reply")]
    ReplyError {},
    #[error("Could not start listening")]
    ListenError {},
    #[error("Couldn't init proxy client")]
    ProxyInitError {},
    // #[error("Proxy client is uninitialised: init proxy client first")]
    // ProxyUninitialisedError {},
    #[error("Couldn't run proxy client")]
    ProxyRunError {},
    #[error("Couldn't init proxy server")]
    ServerInitError {},
    #[error("Couldn't get proxy server address")]
    AddressGetterError {},
    #[error("Couldn't run proxy server")]
    ServerRunError {},
}

#[no_mangle]
fn init_logging() {
    nym_bin_common::logging::setup_logging();
}

#[no_mangle]
fn init_ephemeral() -> Result<(), GoWrapError> {
    match nym_ffi_shared::init_ephemeral_internal() {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ClientInitError {}),
    }
}

#[no_mangle]
fn get_self_address() -> Result<String, GoWrapError> {
    match nym_ffi_shared::get_self_address_internal() {
        Ok(addr) => Ok(addr),
        Err(..) => Err(GoWrapError::SelfAddrError {}),
    }
}

#[no_mangle]
fn send_message(recipient: String, message: String) -> Result<(), GoWrapError> {
    let nym_recipient_type =
        Recipient::try_from_base58_string(recipient).expect("couldn't create Recipient");
    match nym_ffi_shared::send_message_internal(nym_recipient_type, &message) {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::SendMsgError {}),
    }
}

#[no_mangle]
fn reply(recipient: Vec<u8>, message: String) -> Result<(), GoWrapError> {
    let mut sized_array: [u8; 16] = [0; 16];
    sized_array.copy_from_slice(&recipient[..16]);
    let anon_recipient_type: AnonymousSenderTag = AnonymousSenderTag::from_bytes(sized_array);
    match nym_ffi_shared::reply_internal(anon_recipient_type, &message) {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ReplyError {}),
    }
}

pub struct IncomingMessage {
    message: String,
    sender: Vec<u8>,
}

#[no_mangle]
fn listen_for_incoming() -> Result<IncomingMessage, GoWrapError> {
    match nym_ffi_shared::listen_for_incoming_internal() {
        Ok(received) => {
            let message = String::from_utf8_lossy(&received.message).to_string();
            let sender = received.sender_tag.unwrap().to_bytes().to_vec();
            let incoming = IncomingMessage { message, sender };
            Ok(incoming)
        }
        Err(_) => Err(GoWrapError::ListenError {}),
    }
}

#[no_mangle]
fn new_proxy_client(
    server_address: String,
    listen_address: String,
    listen_port: String,
    close_timeout: u64,
    env: Option<String>,
) -> Result<(), GoWrapError> {
    let server_nym_addr =
        Recipient::try_from_base58_string(server_address).expect("couldn't create Recipient");
    match nym_ffi_shared::proxy_client_new_internal(
        server_nym_addr,
        &listen_address,
        &listen_port,
        close_timeout,
        env,
    ) {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ProxyInitError {}),
    }
}

// TODO new proxy client w defaults

fn run_proxy_client() -> Result<(), GoWrapError> {
    match nym_ffi_shared::proxy_client_run_internal() {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ProxyRunError {}),
    }
}

// server
// new
fn new_proxy_server(
    upstream_address: String,
    config_dir: String,
    env: Option<String>,
) -> Result<(), GoWrapError> {
    match nym_ffi_shared::proxy_server_new_internal(&upstream_address, &config_dir, env) {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ServerInitError {}),
    }
}

// get addr
fn proxy_server_address() -> Result<String, GoWrapError> {
    match nym_ffi_shared::proxy_server_address_internal() {
        Ok(address) => Ok(address.to_string()),
        Err(_) => Err(GoWrapError::AddressGetterError {}),
    }
}

// run
fn run_proxy_server() -> Result<(), GoWrapError> {
    match nym_ffi_shared::proxy_server_run_internal() {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ServerRunError {}),
    }
}
