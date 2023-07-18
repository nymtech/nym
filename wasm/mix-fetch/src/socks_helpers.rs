// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_providers_common::interface::ProviderInterfaceVersion;
use nym_socks5_requests::{
    ConnectionId, RemoteAddress, SocketData, Socks5ProtocolVersion, Socks5ProviderRequest,
};
use wasm_client_core::Recipient;

pub(crate) const PROVIDER_INTERFACE_VERSION: ProviderInterfaceVersion =
    ProviderInterfaceVersion::new_current();
pub(crate) const SOCKS5_PROTOCOL_VERSION: Socks5ProtocolVersion =
    Socks5ProtocolVersion::new_current();

// for now explicitly attach return address, we can worry about surbs later
pub(crate) fn socks5_connect_request(
    conn_id: ConnectionId,
    remote_addr: RemoteAddress,
    return_address: Recipient,
) -> Vec<u8> {
    // Create SOCKS connect request
    let request_content = nym_socks5_requests::request::Socks5Request::new_connect(
        SOCKS5_PROTOCOL_VERSION,
        conn_id,
        remote_addr,
        Some(return_address),
    );

    // and wrap it in provider request
    Socks5ProviderRequest::new_provider_data(PROVIDER_INTERFACE_VERSION, request_content)
        .into_bytes()
}

pub(crate) fn socks5_data_request(
    conn_id: ConnectionId,
    local_closed: bool,
    message_sequence: u64,
    data: Vec<u8>,
) -> Vec<u8> {
    // Create SOCKS send request
    let request_content = nym_socks5_requests::request::Socks5Request::new_send(
        SOCKS5_PROTOCOL_VERSION,
        SocketData::new(message_sequence, conn_id, local_closed, data),
    );

    // and wrap it in provider request
    Socks5ProviderRequest::new_provider_data(PROVIDER_INTERFACE_VERSION, request_content)
        .into_bytes()
}
