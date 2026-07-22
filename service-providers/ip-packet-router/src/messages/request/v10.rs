// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v8::request::{
    IpPacketRequest as IpPacketRequestV8, IpPacketRequestData as IpPacketRequestDataV8,
};
use nym_sdk::mixnet::AnonymousSenderTag;

use super::{ClientVersion, IpPacketRequest};

// Same request wire format as v8; reuse its deserialization and tag as V10.
// A From impl would collide with v8.rs (same concrete types), hence a fn.
pub(crate) fn convert(
    request: IpPacketRequestV8,
    sender_tag: AnonymousSenderTag,
) -> IpPacketRequest {
    let version = ClientVersion::V10;
    match request.data {
        IpPacketRequestDataV8::Data(inner) => IpPacketRequest::Data((inner, version).into()),
        IpPacketRequestDataV8::Control(inner) => {
            IpPacketRequest::Control((*inner, sender_tag, version).into())
        }
    }
}
