// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ffi::{c_char};
use nym_ffi_shared;
uniffi::include_scaffolding!("bindings");

#[no_mangle]
pub extern "C" fn init_logging() {
    nym_bin_common::logging::setup_logging();
}

// #[no_mangle]
// pub extern "C" fn init_ephemeral() -> i8 {
//     match nym_ffi_shared::init_ephemeral_internal() {
//         Ok(_) => nym_ffi_shared::StatusCode::NoError as i8,
//         Err(_) => nym_ffi_shared::StatusCode::ClientInitError as i8,
//     }
// }
//
// #[no_mangle]
// pub extern "C" fn get_self_address(callback: nym_ffi_shared::CStringCallback) -> i8 {
//     match nym_ffi_shared::get_self_address_internal(callback) {
//         Ok(_) => nym_ffi_shared::StatusCode::NoError as i8,
//         Err(_) => nym_ffi_shared::StatusCode::SelfAddrError as i8,
//     }
// }

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
// pub extern "C" fn listen_for_incoming(callback: nym_ffi_shared::CMessageCallback) -> i8 {
//     match nym_ffi_shared::listen_for_incoming_internal(callback) {
//         Ok(_) => nym_ffi_shared::StatusCode::NoError as i8,
//         Err(_) => nym_ffi_shared::StatusCode::ListenError as i8,
//     }
// }
