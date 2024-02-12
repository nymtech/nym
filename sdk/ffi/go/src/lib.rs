// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ffi::{c_char, CString};
use std::mem::forget;
use nym_ffi_shared;
use thiserror;
use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, ReconstructedMessage, Recipient};
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
uniffi::include_scaffolding!("bindings");

#[derive(Debug, thiserror::Error)]
enum GoWrapError {
    #[error("Couldn't init client")]
    ClientInitError{},
    #[error("Client is uninitialised: init client first")]
    ClientUninitialisedError{},
    #[error("Error getting self address")]
    SelfAddrError{},
    #[error("Error sending message")]
    SendMsgError{},
    #[error("Error sending reply")]
    ReplyError{},
    #[error("Could not start listening")]
    ListenError{},
}

#[no_mangle]
fn init_logging() {
    nym_bin_common::logging::setup_logging();
}

#[no_mangle]
fn init_ephemeral() -> Result<(), GoWrapError> {
    match nym_ffi_shared::init_ephemeral_internal() {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ClientInitError{})
    }
}

#[no_mangle]
fn get_self_address() -> Result<String, GoWrapError> {
    match nym_ffi_shared::get_self_address_internal() {
        Ok(addr) => Ok(String::from(addr)),
        Err(..) => Err(GoWrapError::SelfAddrError{})
    }
}

#[no_mangle]
fn send_message(recipient: String, message: String) -> Result<(), GoWrapError> {
    let nym_recipient_type = Recipient::try_from_base58_string(recipient).unwrap();
    match nym_ffi_shared::send_message_internal(nym_recipient_type, &message) {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::SendMsgError{}),
    }
}

#[no_mangle]
fn reply(recipient: String, message: String) -> Result<(), GoWrapError> {
    let anon_recipient_type: AnonymousSenderTag = AnonymousSenderTag::try_from_base58_string(recipient).expect("could not construct AnonymousSenderTag from supplied value");
    match nym_ffi_shared::reply_internal(anon_recipient_type, &message) {
        Ok(_) => Ok(()),
        Err(_) => Err(GoWrapError::ReplyError{}),
    }
}

pub struct IncomingMessage {
    message: String,
    sender: String
}

#[no_mangle]
fn listen_for_incoming() -> Result<IncomingMessage, GoWrapError> {
    match nym_ffi_shared::listen_for_incoming_internal() {
        Ok(received) => {
            let message = String::from_utf8_lossy(&received.message).to_string();
            // maybe change this to raw bytes to send over TODO
            let sender = received.sender_tag.unwrap().to_base58_string();
            let incoming = IncomingMessage {
                message,
                sender
            };
            Ok(incoming)
        },
        Err(_) => Err(GoWrapError::ListenError{}),
    }
}
