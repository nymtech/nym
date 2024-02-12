// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ffi::{c_char, CString};
use nym_ffi_shared;
use thiserror;
use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, ReconstructedMessage, Recipient};
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

// #[no_mangle]
// pub extern "C" fn send_message(recipient: *const c_char, message: *const c_char) -> i8 {
//     match nym_ffi_shared::send_message_internal(recipient, message) {
//         Ok(_) => nym_ffi_shared::StatusCode::NoError as i8,
//         Err(_) => nym_ffi_shared::StatusCode::SendMsgError as i8,
//     }
// }
//
// #[no_mangle]
// pub extern "C" fn reply(recipient: *const c_char, message: *const c_char) -> i8 {
//     match nym_ffi_shared::reply_internal(recipient, message) {
//         Ok(_) => nym_ffi_shared::StatusCode::NoError as i8,
//         Err(_) => nym_ffi_shared::StatusCode::ReplyError as i8,
//     }
// }
//
// #[no_mangle]
// fn listen_for_incoming(callback: nym_ffi_shared::CMessageCallback) -> i8 {
//     match nym_ffi_shared::listen_for_incoming_internal(callback) {
//         Ok(_) => nym_ffi_shared::StatusCode::NoError as i8,
//         Err(_) => nym_ffi_shared::StatusCode::ListenError as i8,
//     }
// }
