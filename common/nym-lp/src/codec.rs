// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use crate::packet::{EncryptedLpPacket, InnerHeader, LpHeader, LpMessage, LpPacket};
use bytes::BytesMut;
use libcrux_psq::Channel;

// needs to be equal or above to the actual overhead
pub(crate) const SANE_ENC_OVERHEAD: usize = 32;

// needs to be equal or below the actual overhead
pub(crate) const SANE_DEC_OVERHEAD: usize = 24;

pub(crate) fn encrypt_data(
    plaintext: &[u8],
    transport: &mut libcrux_psq::session::Transport,
) -> Result<Vec<u8>, LpError> {
    let mut ciphertext = vec![0u8; plaintext.len() + SANE_ENC_OVERHEAD];
    let n = transport.write_message(plaintext, &mut ciphertext)?;

    if plaintext.len() + SANE_ENC_OVERHEAD != n {
        ciphertext.truncate(n);
    }

    Ok(ciphertext)
}

pub(crate) fn decrypt_data(
    ciphertext: &[u8],
    transport: &mut libcrux_psq::session::Transport,
) -> Result<Vec<u8>, LpError> {
    if ciphertext.len() < SANE_DEC_OVERHEAD {
        return Err(LpError::InsufficientBufferSize);
    }
    let mut plaintext = vec![0u8; ciphertext.len() - SANE_DEC_OVERHEAD];

    let (_, n) = transport.read_message(ciphertext, &mut plaintext)?;
    if n != ciphertext.len() - SANE_DEC_OVERHEAD {
        plaintext.truncate(n);
    }
    Ok(plaintext)
}

pub(crate) fn encrypt_lp_packet(
    packet: LpPacket,
    transport: &mut libcrux_psq::session::Transport,
) -> Result<EncryptedLpPacket, LpError> {
    let mut plaintext = BytesMut::with_capacity(InnerHeader::SIZE + packet.message().len());
    packet.header().inner.encode(&mut plaintext);
    packet.message().encode_content(&mut plaintext);

    let ciphertext = encrypt_data(plaintext.as_ref(), transport)?;

    Ok(EncryptedLpPacket::new(packet.header().outer, ciphertext))
}

pub(crate) fn decrypt_lp_packet(
    packet: EncryptedLpPacket,
    transport: &mut libcrux_psq::session::Transport,
) -> Result<LpPacket, LpError> {
    if packet.ciphertext().len() < InnerHeader::SIZE + SANE_DEC_OVERHEAD {
        return Err(LpError::InsufficientBufferSize);
    }

    let plaintext = decrypt_data(packet.ciphertext(), transport)?;

    let inner_header = InnerHeader::parse(&plaintext)?;
    let payload = &plaintext[InnerHeader::SIZE..];
    let message = LpMessage::decode_content(payload, inner_header.message_type)?;

    Ok(LpPacket::new(
        LpHeader {
            outer: packet.outer_header(),
            inner: inner_header,
        },
        message,
    ))
}

#[cfg(test)]
mod tests {
    use crate::LpError;
    use crate::codec::{decrypt_data, decrypt_lp_packet, encrypt_data, encrypt_lp_packet};
    use crate::packet::{EncryptedLpPacket, LpHeader, LpMessage, LpPacket, MessageType};
    use crate::peer::mock_peers;
    use crate::psq::initiator::{build_psq_ciphersuite, build_psq_principal};
    use crate::psq::{PSQ_MSG2_SIZE, psq_msg1_size, responder};
    use libcrux_psq::{Channel, IntoSession};
    use nym_kkt_ciphersuite::KEM;
    use nym_test_utils::helpers::u64_seeded_rng_09;

    fn mock_transport() -> (
        libcrux_psq::session::Transport,
        libcrux_psq::session::Transport,
    ) {
        let kem = KEM::MlKem768;
        let rng1 = u64_seeded_rng_09(1);
        let rng2 = u64_seeded_rng_09(2);
        let (init, resp) = mock_peers();
        let remote_resp = resp.as_remote();
        let encapsulation_key = resp
            .kem_keypairs
            .as_ref()
            .unwrap()
            .encapsulation_key(kem)
            .unwrap();

        let initiator_ciphersuite =
            build_psq_ciphersuite(&init, &remote_resp, &encapsulation_key).unwrap();
        let mut psq_initiator = build_psq_principal(rng1, 1, initiator_ciphersuite).unwrap();

        let responder_ciphersuite = responder::build_psq_ciphersuite(&resp, kem).unwrap();
        let mut psq_responder =
            responder::build_psq_principal(rng2, 1, responder_ciphersuite).unwrap();

        // Send first message
        let mut buf = vec![0u8; psq_msg1_size(kem)];

        let mut payload_buf_responder = vec![0u8; 4096];
        let mut payload_buf_initiator = vec![0u8; 4096];

        let len_i = psq_initiator.write_message(&[], &mut buf).unwrap();
        assert_eq!(len_i, buf.len());

        // Read first message
        let (_, _) = psq_responder
            .read_message(&buf, &mut payload_buf_responder)
            .unwrap();

        // Respond
        let mut buf = [0u8; PSQ_MSG2_SIZE];
        let len_r = psq_responder.write_message(&[], &mut buf).unwrap();
        assert_eq!(len_r, buf.len());

        // Finalize on registration initiator
        let (len_i_deserialized, _) = psq_initiator
            .read_message(&buf, &mut payload_buf_initiator)
            .unwrap();

        // We read the same amount of data.
        assert_eq!(len_r, len_i_deserialized);

        // Ready for transport mode
        assert!(psq_initiator.is_handshake_finished());
        assert!(psq_responder.is_handshake_finished());

        let transport_initiator = psq_initiator
            .into_session()
            .unwrap()
            .transport_channel()
            .unwrap();

        let transport_responder = psq_responder
            .into_session()
            .unwrap()
            .transport_channel()
            .unwrap();

        (transport_initiator, transport_responder)
    }

    #[test]
    fn basic_plain_encryption_test() {
        let (mut init_transport, mut resp_transport) = mock_transport();

        for msg_size in [1usize, 10, 100, 1000, 10000, 65535] {
            let message1 = vec![42u8; msg_size];

            let mut ciphertext = vec![0u8; msg_size + 64];
            let written_init1 = init_transport
                .write_message(&message1, &mut ciphertext)
                .unwrap();

            let init_ciphertext_overhead = written_init1 - msg_size;
            let ciphertext_content = &ciphertext[..written_init1];

            let mut plaintext = vec![0u8; msg_size + 64];
            let (read_resp1, written_resp1) = resp_transport
                .read_message(ciphertext_content, &mut plaintext)
                .unwrap();
            let resp_plaintext_overhead = ciphertext_content.len() - written_resp1;

            assert_eq!(
                written_init1, read_resp1,
                "should work for message {msg_size}"
            );
            let message1_content = &plaintext[..written_resp1];
            assert_eq!(
                message1_content, &message1,
                "should work for message {msg_size}"
            );

            // reverse the communication
            let message2 = vec![43u8; msg_size];

            let mut ciphertext2 = vec![0u8; msg_size + 64];
            let written_resp2 = resp_transport
                .write_message(&message2, &mut ciphertext2)
                .unwrap();

            let resp_ciphertext_overhead = written_resp2 - msg_size;
            let ciphertext_content2 = &ciphertext2[..written_resp2];

            let mut plaintext2 = vec![0u8; msg_size + 64];
            let (read_init2, written_init2) = init_transport
                .read_message(ciphertext_content2, &mut plaintext2)
                .unwrap();
            let init_plaintext_overhead = ciphertext_content2.len() - written_init2;

            assert_eq!(
                written_resp2, read_init2,
                "should work for message {msg_size}"
            );
            let message2_content = &plaintext2[..written_init2];
            assert_eq!(
                message2_content, &message2,
                "should work for message {msg_size}"
            );

            // check consistent overheads
            // enc/enc
            assert_eq!(init_ciphertext_overhead, resp_ciphertext_overhead);

            // dec/dec
            assert_eq!(resp_plaintext_overhead, init_plaintext_overhead);

            // enc/dec
            assert_eq!(init_ciphertext_overhead, resp_plaintext_overhead);
        }
    }

    #[test]
    fn basic_encryption() {
        let (mut init_transport, mut resp_transport) = mock_transport();

        // happy path
        let msg = b"foomp".to_vec();
        let ciphertext = encrypt_data(&msg, &mut init_transport).unwrap();
        let plaintext = decrypt_data(&ciphertext, &mut resp_transport).unwrap();
        assert_eq!(msg, plaintext);

        // incomplete ciphertext
        let msg2 = b"foomp".to_vec();
        let ciphertext2 = encrypt_data(&msg2, &mut init_transport).unwrap();
        let len = ciphertext2.len();
        let dec_err = decrypt_data(&ciphertext2[..len - 1], &mut resp_transport).unwrap_err();
        assert!(matches!(dec_err, LpError::PSQSessionFailure { .. }));

        // too small buffer
        let msg3 = b"foomp".to_vec();
        let ciphertext3 = encrypt_data(&msg3, &mut resp_transport).unwrap();
        let dec_err = decrypt_data(&ciphertext3[..10], &mut init_transport).unwrap_err();
        assert!(matches!(dec_err, LpError::InsufficientBufferSize));
    }

    #[test]
    fn basic_packet_encryption() {
        let (mut init_transport, mut resp_transport) = mock_transport();

        // happy path
        let packet = LpPacket::new(LpHeader::new(123, 0, 1, MessageType::Busy), LpMessage::Busy);

        let ciphertext = encrypt_lp_packet(packet.clone(), &mut init_transport).unwrap();
        assert_eq!(packet.header().outer, ciphertext.outer_header());

        let plaintext = decrypt_lp_packet(ciphertext, &mut resp_transport).unwrap();
        assert_eq!(packet, plaintext);

        // incomplete ciphertext
        let packet = LpPacket::new(LpHeader::new(123, 1, 1, MessageType::Busy), LpMessage::Busy);
        let ciphertext2 = encrypt_lp_packet(packet, &mut init_transport).unwrap();
        let l = ciphertext2.ciphertext().len();
        let malformed_content = ciphertext2.ciphertext()[..l - 1].to_vec();
        let malformed = EncryptedLpPacket::new(ciphertext2.outer_header(), malformed_content);
        let dec_err = decrypt_lp_packet(malformed, &mut resp_transport).unwrap_err();
        assert!(matches!(dec_err, LpError::PSQSessionFailure { .. }));

        // too small buffer
        let packet = LpPacket::new(LpHeader::new(123, 1, 1, MessageType::Busy), LpMessage::Busy);
        let ciphertext3 = encrypt_lp_packet(packet, &mut resp_transport).unwrap();
        let malformed = EncryptedLpPacket::new(ciphertext3.outer_header(), vec![]);
        let dec_err = decrypt_lp_packet(malformed, &mut init_transport).unwrap_err();
        assert!(matches!(dec_err, LpError::InsufficientBufferSize));
    }
}
