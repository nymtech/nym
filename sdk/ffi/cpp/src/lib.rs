// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
// TODO REMOVE when you're working on new CPP branch
#![allow(clippy::all)]
// use nym_ffi_shared;
use std::ffi::{c_char, c_int, CStr, CString};

use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;
use std::mem::forget;
mod types;
use crate::types::types::{CMessageCallback, CStringCallback, ReceivedMessage, StatusCode};

#[no_mangle]
pub extern "C" fn init_logging() {
    nym_bin_common::logging::setup_logging();
}

#[no_mangle]
pub extern "C" fn init_ephemeral() -> c_int {
    match nym_ffi_shared::init_ephemeral_internal() {
        Ok(_) => StatusCode::NoError as c_int,
        Err(_) => StatusCode::ClientInitError as c_int,
    }
}

#[no_mangle]
pub extern "C" fn get_self_address(callback: CStringCallback) -> c_int {
    match nym_ffi_shared::get_self_address_internal(/*callback*/) {
        Ok(addr) => {
            let c_ptr = CString::new(addr).expect("could not convert Nym address to CString");
            let call = CStringCallback::new(callback.callback);
            // as_ptr() keeps ownership in rust unlike into_raw() so no need to free it
            call.trigger(c_ptr.as_ptr());
            StatusCode::NoError as c_int
        }
        Err(_) => StatusCode::SelfAddrError as c_int,
    }
}

#[no_mangle]
pub extern "C" fn send_message(recipient: *const c_char, message: *const c_char) -> c_int {
    let c_str = unsafe {
        if recipient.is_null() {
            return StatusCode::RecipientNullError as c_int;
        }
        let c_str = CStr::from_ptr(recipient);
        c_str
    };
    let r_str = c_str.to_str().unwrap();
    let recipient = r_str.parse().unwrap();
    let c_str = unsafe {
        if message.is_null() {
            return StatusCode::MessageNullError as c_int;
        }
        let c_str = CStr::from_ptr(message);
        c_str
    };
    let message = c_str.to_str().unwrap();

    match nym_ffi_shared::send_message_internal(recipient, message) {
        Ok(_) => StatusCode::NoError as c_int,
        Err(_) => StatusCode::SendMsgError as c_int,
    }
}

#[no_mangle]
pub extern "C" fn reply(recipient: *const c_char, message: *const c_char) -> c_int {
    let recipient = unsafe {
        if recipient.is_null() {
            return StatusCode::RecipientNullError as c_int;
        }
        let r_str = CStr::from_ptr(recipient).to_string_lossy().into_owned();
        AnonymousSenderTag::try_from_base58_string(r_str)
            .expect("could not construct AnonymousSenderTag from supplied value")
    };
    let message = unsafe {
        if message.is_null() {
            return StatusCode::MessageNullError as c_int;
        }
        let c_str = CStr::from_ptr(message);
        let r_str = c_str.to_str().unwrap();
        r_str
    };

    match nym_ffi_shared::reply_internal(recipient, message) {
        Ok(_) => StatusCode::NoError as c_int,
        Err(_) => StatusCode::ReplyError as c_int,
    }
}

#[no_mangle]
pub extern "C" fn listen_for_incoming(callback: CMessageCallback) -> c_int {
    match nym_ffi_shared::listen_for_incoming_internal() {
        Ok(received) => {
            let message_ptr = received.message.as_ptr();
            let message_length = received.message.len();
            let c_string = CString::new(received.sender_tag.unwrap().to_string()).unwrap();
            let sender_ptr = c_string.as_ptr();
            // stop deallocation when out of scope as passing raw ptr to it elsewhere
            forget(received);
            let rec_for_c = ReceivedMessage {
                message: message_ptr,
                size: message_length,
                sender_tag: sender_ptr,
            };
            let call = CMessageCallback::new(callback.callback);
            call.trigger(rec_for_c);

            StatusCode::NoError as c_int
        }
        Err(_) => StatusCode::ListenError as c_int,
    }
}
