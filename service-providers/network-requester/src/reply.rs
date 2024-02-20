// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sdk::mixnet::InputMessage;
use nym_service_providers_common::interface::{
    ControlRequest, ControlResponse, ProviderInterfaceVersion, RequestVersion,
};
use nym_socks5_requests::{
    ConnectionId, SocketData, Socks5ProviderRequest, Socks5ProviderResponse, Socks5Request,
    Socks5RequestContent, Socks5Response, Socks5ResponseContent,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use std::fmt::{Debug, Formatter};

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
    pub(crate) fn new_provider_response<A: Into<MixnetAddress>>(
        address: A,
        connection_id: ConnectionId,
        msg: Socks5ProviderResponse,
    ) -> Self {
        MixnetMessage {
            address: address.into(),
            data: msg.into_bytes(),
            connection_id,
        }
    }

    pub(crate) fn new_provider_request<A: Into<MixnetAddress>>(
        address: A,
        connection_id: ConnectionId,
        msg: Socks5ProviderRequest,
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
        let msg = Socks5ProviderRequest::new_provider_data(
            request_version.provider_interface,
            Socks5Request::new(request_version.provider_protocol, content),
        );

        Self::new_provider_request(address, connection_id, msg)
    }

    pub(crate) fn new_network_data_response(
        address: MixnetAddress,
        request_version: RequestVersion<Socks5Request>,
        connection_id: ConnectionId,
        content: SocketData,
    ) -> Self {
        // TODO: simplify by providing better constructor for `PlaceholderResponse`
        let res = Socks5Response::new(
            request_version.provider_protocol,
            Socks5ResponseContent::NetworkData { content },
        );
        let msg =
            Socks5ProviderResponse::new_provider_data(request_version.provider_interface, res);

        Self::new_provider_response(address, connection_id, msg)
    }

    #[allow(dead_code)]
    pub(crate) fn new_control_request<A: Into<MixnetAddress>>(
        address: A,
        request_version: ProviderInterfaceVersion,
        content: ControlRequest,
    ) -> Self {
        let msg = Socks5ProviderRequest::new_control(request_version, content);
        // TODO: not sure what to think about 0 connection_id here...
        Self::new_provider_request(address, 0, msg)
    }

    #[allow(dead_code)]
    pub(crate) fn new_control_response<A: Into<MixnetAddress>>(
        address: A,
        response_version: ProviderInterfaceVersion,
        content: ControlResponse,
    ) -> Self {
        let msg = Socks5ProviderResponse::new_control(response_version, content);
        // TODO: not sure what to think about 0 connection_id here...
        Self::new_provider_response(address, 0, msg)
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
        let msg =
            Socks5ProviderResponse::new_provider_data(request_version.provider_interface, res);

        Self::new_provider_response(address, connection_id, msg)
    }

    // TODO: the naming is awful, but naming things is difficult...
    pub(crate) fn new_network_data_response_content(
        address: MixnetAddress,
        request_version: RequestVersion<Socks5Request>,
        seq: u64,
        connection_id: ConnectionId,
        data: Vec<u8>,
        closed_socket: bool,
    ) -> Self {
        let response_content = SocketData::new(seq, connection_id, closed_socket, data);
        Self::new_network_data_response(address, request_version, connection_id, response_content)
    }

    pub(crate) fn data_size(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn into_input_message(self, packet_type: PacketType) -> InputMessage {
        self.address
            .send_back_to(self.data, self.connection_id, packet_type)
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

    pub(super) fn send_back_to(
        self,
        message: Vec<u8>,
        connection_id: u64,
        packet_type: PacketType,
    ) -> InputMessage {
        match self {
            MixnetAddress::Known(recipient) => InputMessage::MessageWrapper {
                message: Box::new(InputMessage::Regular {
                    recipient: *recipient,
                    data: message,
                    lane: TransmissionLane::ConnectionId(connection_id),
                    mix_hops: None,
                }),
                packet_type,
            },
            MixnetAddress::Anonymous(sender_tag) => InputMessage::MessageWrapper {
                message: Box::new(InputMessage::Reply {
                    recipient_tag: sender_tag,
                    data: message,
                    lane: TransmissionLane::ConnectionId(connection_id),
                }),
                packet_type,
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
