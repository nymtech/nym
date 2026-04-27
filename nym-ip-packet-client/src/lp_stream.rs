use bytes::BytesMut;
use nym_ip_packet_requests::SPHINX_STREAM_VERSION_THRESHOLD;
use nym_lp::packet::frame::{
    LpFrame, LpFrameHeader, LpFrameKind, SphinxStreamFrameAttributes, SphinxStreamMsgType,
};
use nym_sdk::mixnet::ReconstructedMessage;
use tracing::trace;

/// Whether the "current" IPR client is operating at a version where the node expects
/// non-stream mixnet IPR messages to be LP Stream framed (see `SPHINX_STREAM_VERSION_THRESHOLD`).
pub(crate) fn current_requires_sphinx_stream_transport() -> bool {
    crate::current::VERSION >= SPHINX_STREAM_VERSION_THRESHOLD
}

pub fn maybe_unwrap_lp_stream_payload(data: &[u8]) -> &[u8] {
    if data.len() < LpFrameHeader::SIZE {
        return data;
    }
    let Ok(header) = LpFrameHeader::parse(data) else {
        trace!("expected LP header but failed to parse; treating as raw payload");
        return data;
    };
    if header.kind == LpFrameKind::SphinxStream {
        &data[LpFrameHeader::SIZE..]
    } else {
        trace!(kind = ?header.kind, "lp header parsed but not a sphinx stream frame; treating as raw payload");
        data
    }
}

pub fn maybe_unwrap_lp_stream_payload_from_reconstructed(message: &ReconstructedMessage) -> &[u8] {
    maybe_unwrap_lp_stream_payload(&message.message)
}

pub fn encode_stream_frame(stream_id: u64, sequence_num: u32, payload: Vec<u8>) -> Vec<u8> {
    let attrs = SphinxStreamFrameAttributes {
        stream_id,
        msg_type: SphinxStreamMsgType::Data,
        sequence_num,
    };
    let frame = LpFrame::new_stream(attrs, payload);
    let mut buf = BytesMut::with_capacity(LpFrameHeader::SIZE + frame.content.len());
    frame.encode(&mut buf);
    buf.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_lp::packet::frame::SphinxStreamFrameAttributes;

    #[test]
    fn stream_frame_roundtrip_unwraps_payload() {
        let stream_id = 0x0123_4567_89ab_cdef;
        let seq = 42u32;
        let payload = b"hello-ipr".to_vec();

        let framed = encode_stream_frame(stream_id, seq, payload.clone());

        let header = LpFrameHeader::parse(&framed).expect("valid lp header");
        assert_eq!(header.kind, LpFrameKind::SphinxStream);

        let attrs =
            SphinxStreamFrameAttributes::parse(&header.frame_attributes).expect("valid attrs");
        assert_eq!(attrs.stream_id, stream_id);
        assert_eq!(attrs.sequence_num, seq);
        assert_eq!(attrs.msg_type, SphinxStreamMsgType::Data);

        let unwrapped = maybe_unwrap_lp_stream_payload(&framed);
        assert_eq!(unwrapped, payload.as_slice());
    }

    #[test]
    fn unwrap_noops_on_non_stream_or_malformed_data() {
        let raw = b"\x09\x00\x01\x02\x03";
        assert_eq!(maybe_unwrap_lp_stream_payload(raw), raw);

        // malformed header: not enough bytes for LP header
        let short = b"\x00\x01";
        assert_eq!(maybe_unwrap_lp_stream_payload(short), short);
    }
}
