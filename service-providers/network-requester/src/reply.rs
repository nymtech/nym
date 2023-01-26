// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use service_providers_common::interface::RequestVersion;
use socks5_requests::{
    ConnectionId, NetworkData, PlaceholderRequest, PlaceholderResponse, Socks5Request,
    Socks5RequestContent, Socks5Response, Socks5ResponseContent,
};
use std::fmt::{Debug, Formatter};
use websocket_requests::requests::ClientRequest;

/// Generic data this service provider will send back to the mixnet via its connected native client.
/// It includes serialized socks5 proxy responses to its connected clients
/// as well as socks5 proxy requests to the stats collector.
pub(crate) struct MixnetMessage {
    pub(crate) address: MixnetAddress,
    pub(crate) data: Vec<u8>,
    pub(crate) connection_id: ConnectionId,
}

impl Debug for MixnetMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} bytes to {:?} on connection_id {}",
            self.data.len(),
            self.address,
            self.connection_id
        )
    }
}

impl MixnetMessage {
    pub(crate) fn new_provider_data_response<A: Into<MixnetAddress>>(
        address: A,
        connection_id: ConnectionId,
        msg: PlaceholderResponse,
    ) -> Self {
        MixnetMessage {
            address: address.into(),
            data: msg.into_bytes(),
            connection_id,
        }
    }

    pub(crate) fn new_provider_data_request<A: Into<MixnetAddress>>(
        address: A,
        connection_id: ConnectionId,
        msg: PlaceholderRequest,
    ) -> Self {
        MixnetMessage {
            address: address.into(),
            data: msg.into_bytes(),
            connection_id,
        }
    }

    pub(crate) fn new_network_data_request<A: Into<MixnetAddress>>(
        address: A,
        request_version: RequestVersion<Socks5Request>,
        connection_id: ConnectionId,
        content: Socks5RequestContent,
    ) -> Self {
        let msg = PlaceholderRequest::new_provider_data(
            request_version.provider_interface,
            Socks5Request::new(request_version.provider_protocol, content),
        );

        Self::new_provider_data_request(address, connection_id, msg)
    }

    pub(crate) fn new_network_data_response(
        address: MixnetAddress,
        request_version: RequestVersion<Socks5Request>,
        connection_id: ConnectionId,
        content: NetworkData,
    ) -> Self {
        // TODO: simplify by providing better constructor for `PlaceholderResponse`
        let res = Socks5Response::new(
            request_version.provider_protocol,
            Socks5ResponseContent::NetworkData(content),
        );
        let msg = PlaceholderResponse::new_provider_data(request_version.provider_interface, res);

        Self::new_provider_data_response(address, connection_id, msg)
    }

    pub(crate) fn new_connection_error(
        address: MixnetAddress,
        request_version: RequestVersion<Socks5Request>,
        connection_id: ConnectionId,
        error_message: String,
    ) -> Self {
        // TODO: simplify by providing better constructor for `PlaceholderResponse`
        let res = Socks5Response::new_connection_error(
            request_version.provider_protocol,
            connection_id,
            error_message,
        );
        let msg = PlaceholderResponse::new_provider_data(request_version.provider_interface, res);

        Self::new_provider_data_response(address, connection_id, msg)
    }

    // TODO: the naming is awful, but naming things is difficult...
    pub(crate) fn new_network_data_response_content(
        address: MixnetAddress,
        request_version: RequestVersion<Socks5Request>,
        connection_id: ConnectionId,
        data: Vec<u8>,
        closed_socket: bool,
    ) -> Self {
        let response_content = NetworkData::new(connection_id, data, closed_socket);
        Self::new_network_data_response(address, request_version, connection_id, response_content)
    }

    pub(crate) fn data_size(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn into_client_request(self) -> ClientRequest {
        self.address.send_back_to(self.data, self.connection_id)
    }
}

/// A return address is a way to send a message back to the original sender. It can be either
/// an explicitly known Recipient, or a surb AnonymousSenderTag.
#[derive(Debug, Clone)]
pub enum MixnetAddress {
    Known(Box<Recipient>),
    Anonymous(AnonymousSenderTag),
}
impl MixnetAddress {
    pub fn new(
        explicit_return_address: Option<Recipient>,
        implicit_tag: Option<AnonymousSenderTag>,
    ) -> Option<Self> {
        // if somehow we received both, always prefer the explicit address since it's way easier to use
        if let Some(recipient) = explicit_return_address {
            return Some(MixnetAddress::Known(Box::new(recipient)));
        }
        if let Some(sender_tag) = implicit_tag {
            return Some(MixnetAddress::Anonymous(sender_tag));
        }
        None
    }

    pub(super) fn send_back_to(self, message: Vec<u8>, connection_id: u64) -> ClientRequest {
        match self {
            MixnetAddress::Known(recipient) => ClientRequest::Send {
                recipient: *recipient,
                message,
                connection_id: Some(connection_id),
            },
            MixnetAddress::Anonymous(sender_tag) => ClientRequest::Reply {
                message,
                sender_tag,
                connection_id: Some(connection_id),
            },
        }
    }
}

impl From<Recipient> for MixnetAddress {
    fn from(recipient: Recipient) -> Self {
        MixnetAddress::Known(Box::new(recipient))
    }
}

impl From<AnonymousSenderTag> for MixnetAddress {
    fn from(sender_tag: AnonymousSenderTag) -> Self {
        MixnetAddress::Anonymous(sender_tag)
    }
}
