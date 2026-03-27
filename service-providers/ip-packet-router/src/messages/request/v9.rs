// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v8::request::{
    IpPacketRequest as IpPacketRequestV8, IpPacketRequestData as IpPacketRequestDataV8,
};
use nym_sdk::mixnet::AnonymousSenderTag;

use super::{ClientVersion, IpPacketRequest};

// v9 uses the same wire format as v8, so we reuse the v8 deserialization
// and just tag the result with ClientVersion::V9.
//
// We cannot implement From<(IpPacketRequestV8, AnonymousSenderTag)> again
// because v8.rs already has that impl (same concrete types).
pub(crate) fn convert(
    request: IpPacketRequestV8,
    sender_tag: AnonymousSenderTag,
) -> IpPacketRequest {
    let version = ClientVersion::V9;
    match request.data {
        IpPacketRequestDataV8::Data(inner) => IpPacketRequest::Data((inner, version).into()),
        IpPacketRequestDataV8::Control(inner) => {
            IpPacketRequest::Control((*inner, sender_tag, version).into())
        }
    }
}
