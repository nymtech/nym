// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::error::ClientCoreError;
use crate::make_bincode_serializer;
use bincode::Options;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use tokio_util::{
    bytes::Buf,
    bytes::BytesMut,
    codec::{Decoder, Encoder},
};

pub type InputMessageSender = tokio_util::sync::PollSender<InputMessage>;
pub type InputMessageReceiver = tokio::sync::mpsc::Receiver<InputMessage>;

const LENGHT_ENCODING_PREFIX_SIZE: usize = 4;

#[derive(Serialize, Deserialize, Debug)]
pub enum InputMessage {
    /// Fire an already prepared mix packets into the network.
    /// No guarantees are made about it. For example no retransmssion
    /// will be attempted if it gets dropped.
    Premade {
        msgs: Vec<MixPacket>,
        lane: TransmissionLane,
    },

    /// The simplest message variant where no additional information is attached.
    /// You're simply sending your `data` to specified `recipient` without any tagging.
    ///
    /// Ends up with `NymMessage::Plain` variant
    Regular {
        recipient: Recipient,
        data: Vec<u8>,
        lane: TransmissionLane,
        max_retransmissions: Option<u32>,
    },

    /// Creates a message used for a duplex anonymous communication where the recipient
    /// will never learn of our true identity. This is achieved by carefully sending `reply_surbs`.
    ///
    /// Note that if reply_surbs is set to zero then
    /// this variant requires the client having sent some reply_surbs in the past
    /// (and thus the recipient also knowing our sender tag).
    ///
    /// Ends up with `NymMessage::Repliable` variant
    Anonymous {
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
        lane: TransmissionLane,
        max_retransmissions: Option<u32>,
    },

    /// Attempt to use our internally received and stored `ReplySurb` to send the message back
    /// to specified recipient whilst not knowing its full identity (or even gateway).
    ///
    /// Ends up with `NymMessage::Reply` variant
    Reply {
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
        max_retransmissions: Option<u32>,
    },

    MessageWrapper {
        message: Box<InputMessage>,
        packet_type: PacketType,
    },
}

impl InputMessage {
    pub fn simple(data: &[u8], recipient: Recipient) -> Self {
        InputMessage::new_regular(recipient, data.to_vec(), TransmissionLane::General, None)
    }

    pub fn new_premade(
        msgs: Vec<MixPacket>,
        lane: TransmissionLane,
        packet_type: PacketType,
    ) -> Self {
        let message = InputMessage::Premade { msgs, lane };
        if packet_type == PacketType::Mix {
            message
        } else {
            InputMessage::new_wrapper(message, packet_type)
        }
    }

    pub fn new_wrapper(message: InputMessage, packet_type: PacketType) -> Self {
        InputMessage::MessageWrapper {
            message: Box::new(message),
            packet_type,
        }
    }

    pub fn new_regular(
        recipient: Recipient,
        data: Vec<u8>,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
    ) -> Self {
        let message = InputMessage::Regular {
            recipient,
            data,
            lane,
            max_retransmissions: None,
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    pub fn new_anonymous(
        recipient: Recipient,
        data: Vec<u8>,
        reply_surbs: u32,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
    ) -> Self {
        let message = InputMessage::Anonymous {
            recipient,
            data,
            reply_surbs,
            lane,
            max_retransmissions: None,
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    pub fn new_reply(
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
        packet_type: Option<PacketType>,
    ) -> Self {
        let message = InputMessage::Reply {
            recipient_tag,
            data,
            lane,
            // \/ set it to SOME sane default so that if we run out of surbs and constantly
            // fail to request more, we wouldn't be stuck in limbo
            max_retransmissions: Some(10),
        };
        if let Some(packet_type) = packet_type {
            InputMessage::new_wrapper(message, packet_type)
        } else {
            message
        }
    }

    pub fn lane(&self) -> &TransmissionLane {
        match self {
            InputMessage::Regular { lane, .. }
            | InputMessage::Anonymous { lane, .. }
            | InputMessage::Reply { lane, .. }
            | InputMessage::Premade { lane, .. } => lane,
            InputMessage::MessageWrapper { message, .. } => message.lane(),
        }
    }

    pub fn set_max_retransmissions(&mut self, max_retransmissions: u32) -> &mut Self {
        match self {
            InputMessage::Regular {
                max_retransmissions: m,
                ..
            }
            | InputMessage::Anonymous {
                max_retransmissions: m,
                ..
            }
            | InputMessage::Reply {
                max_retransmissions: m,
                ..
            } => {
                *m = Some(max_retransmissions);
            }
            InputMessage::Premade { .. } => {}
            InputMessage::MessageWrapper { message, .. } => {
                message.set_max_retransmissions(max_retransmissions);
            }
        }

        self
    }

    pub fn with_max_retransmissions(mut self, max_retransmissions: u32) -> Self {
        self.set_max_retransmissions(max_retransmissions);
        self
    }
    #[allow(clippy::expect_used)]
    pub fn serialized_size(&self) -> u64 {
        make_bincode_serializer()
            .serialized_size(self)
            .expect("failed to get serialized InputMessage size")
            + LENGHT_ENCODING_PREFIX_SIZE as u64
    }
}

pub struct AdressedInputMessageCodec(pub Recipient);

impl Encoder<&[u8]> for AdressedInputMessageCodec {
    type Error = ClientCoreError;

    fn encode(&mut self, item: &[u8], buf: &mut BytesMut) -> Result<(), Self::Error> {
        let mut codec = InputMessageCodec;
        let input_message = InputMessage::simple(item, self.0);
        codec.encode(input_message, buf)?;
        Ok(())
    }
}

pub struct InputMessageCodec;

impl Encoder<InputMessage> for InputMessageCodec {
    type Error = ClientCoreError;

    fn encode(&mut self, item: InputMessage, buf: &mut BytesMut) -> Result<(), Self::Error> {
        #[allow(clippy::expect_used)]
        let encoded = make_bincode_serializer().serialize(&item)?;
        let encoded_len = encoded.len() as u32;
        let mut encoded_with_len = encoded_len.to_le_bytes().to_vec();
        encoded_with_len.extend(encoded);
        buf.reserve(encoded_with_len.len());
        buf.extend_from_slice(&encoded_with_len);
        Ok(())
    }
}

impl Decoder for InputMessageCodec {
    type Item = InputMessage;
    type Error = ClientCoreError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.len() < LENGHT_ENCODING_PREFIX_SIZE {
            return Ok(None);
        }
        #[allow(clippy::expect_used)]
        let len = u32::from_le_bytes(buf[0..LENGHT_ENCODING_PREFIX_SIZE].try_into()?) as usize;
        if buf.len() < len + LENGHT_ENCODING_PREFIX_SIZE {
            return Ok(None);
        }

        let decoded = make_bincode_serializer()
            .deserialize(&buf[LENGHT_ENCODING_PREFIX_SIZE..len + LENGHT_ENCODING_PREFIX_SIZE])?;

        buf.advance(len + LENGHT_ENCODING_PREFIX_SIZE);

        Ok(Some(decoded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_sphinx::addressing::clients::Recipient;
    use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
    use nym_sphinx::params::PacketType;
    use rand::SeedableRng;

    fn test_recipient() -> Recipient {
        Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap()
    }

    fn test_sender_tag() -> AnonymousSenderTag {
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        AnonymousSenderTag::new_random(&mut rng)
    }

    #[test]
    fn encode_decode_all_variants() {
        let mut codec = InputMessageCodec;
        {
            let mut buf = BytesMut::new();
            let msg = InputMessage::new_anonymous(
                test_recipient(),
                vec![1, 2, 3, 4, 5],
                3,
                TransmissionLane::General,
                None,
            );
            codec.encode(msg, &mut buf).unwrap();
            let decoded = codec
                .decode(&mut buf)
                .unwrap()
                .expect("Should decode message");

            match decoded {
                InputMessage::Anonymous {
                    data, reply_surbs, ..
                } => {
                    assert_eq!(data, vec![1, 2, 3, 4, 5]);
                    assert_eq!(reply_surbs, 3);
                }
                _ => panic!("Expected Anonymous variant"),
            }
        }

        {
            let mut buf = BytesMut::new();
            let msg = InputMessage::new_reply(
                test_sender_tag(),
                vec![6, 7, 8],
                TransmissionLane::General,
                None,
            );
            codec.encode(msg, &mut buf).unwrap();
            let decoded = codec
                .decode(&mut buf)
                .unwrap()
                .expect("Should decode message");

            match decoded {
                InputMessage::Reply { data, .. } => {
                    assert_eq!(data, vec![6, 7, 8]);
                }
                _ => panic!("Expected Reply variant"),
            }
        }

        {
            let mut buf = BytesMut::new();
            let inner = InputMessage::new_anonymous(
                test_recipient(),
                vec![9, 10],
                2,
                TransmissionLane::General,
                None,
            );
            let msg = InputMessage::new_wrapper(inner, PacketType::Mix);
            codec.encode(msg, &mut buf).unwrap();
            let decoded = codec
                .decode(&mut buf)
                .unwrap()
                .expect("Should decode message");

            match decoded {
                InputMessage::MessageWrapper {
                    message,
                    packet_type,
                } => {
                    assert_eq!(packet_type, PacketType::Mix);
                    match *message {
                        InputMessage::Anonymous {
                            data, reply_surbs, ..
                        } => {
                            assert_eq!(data, vec![9, 10]);
                            assert_eq!(reply_surbs, 2);
                        }
                        _ => panic!("Expected Anonymous inner message"),
                    }
                }
                _ => panic!("Expected MessageWrapper variant"),
            }
        }
    }

    #[test]
    fn encode_decode_sequential_messages() {
        let mut codec = InputMessageCodec;
        let mut buf = BytesMut::new();

        codec
            .encode(
                InputMessage::new_anonymous(
                    test_recipient(),
                    vec![1, 2, 3],
                    1,
                    TransmissionLane::General,
                    None,
                ),
                &mut buf,
            )
            .unwrap();

        codec
            .encode(
                InputMessage::new_anonymous(
                    test_recipient(),
                    vec![4, 5, 6, 7],
                    2,
                    TransmissionLane::General,
                    None,
                ),
                &mut buf,
            )
            .unwrap();

        codec
            .encode(
                InputMessage::new_anonymous(
                    test_recipient(),
                    vec![8, 9],
                    3,
                    TransmissionLane::General,
                    None,
                ),
                &mut buf,
            )
            .unwrap();

        let decoded1 = codec
            .decode(&mut buf)
            .unwrap()
            .expect("Should decode first message");
        match decoded1 {
            InputMessage::Anonymous {
                data, reply_surbs, ..
            } => {
                assert_eq!(data, vec![1, 2, 3]);
                assert_eq!(reply_surbs, 1);
            }
            _ => panic!("Wrong variant"),
        }

        let decoded2 = codec
            .decode(&mut buf)
            .unwrap()
            .expect("Should decode second message");
        match decoded2 {
            InputMessage::Anonymous {
                data, reply_surbs, ..
            } => {
                assert_eq!(data, vec![4, 5, 6, 7]);
                assert_eq!(reply_surbs, 2);
            }
            _ => panic!("Wrong variant"),
        }

        let decoded3 = codec
            .decode(&mut buf)
            .unwrap()
            .expect("Should decode third message");
        match decoded3 {
            InputMessage::Anonymous {
                data, reply_surbs, ..
            } => {
                assert_eq!(data, vec![8, 9]);
                assert_eq!(reply_surbs, 3);
            }
            _ => panic!("Wrong variant"),
        }

        // Buffer should be empty
        let decoded4 = codec.decode(&mut buf).unwrap();
        assert!(decoded4.is_none(), "Should have no more messages");
        assert_eq!(buf.len(), 0, "Buffer should be empty");
    }

    #[test]
    fn partial_message_handling() {
        let mut codec = InputMessageCodec;
        let mut buf = BytesMut::new();
        // Empty @ beginning
        assert!(codec.decode(&mut buf).unwrap().is_none());

        let mut buf = BytesMut::from(&[0x10, 0x00][..]);
        assert!(codec.decode(&mut buf).unwrap().is_none());
        assert_eq!(buf.len(), 2, "Buffer should be unchanged");

        let mut full_buf = BytesMut::new();
        codec
            .encode(
                InputMessage::new_anonymous(
                    test_recipient(),
                    vec![1, 2, 3, 4, 5],
                    2,
                    TransmissionLane::General,
                    None,
                ),
                &mut full_buf,
            )
            .unwrap();

        // Only first half of the message
        let partial_len = full_buf.len() / 2;
        let mut partial_buf = full_buf.split_to(partial_len);

        assert!(codec.decode(&mut partial_buf).unwrap().is_none());
        assert_eq!(partial_buf.len(), partial_len, "Buffer should be unchanged");

        partial_buf.unsplit(full_buf);
        let decoded = codec.decode(&mut partial_buf).unwrap();
        assert!(decoded.is_some(), "Should decode complete message");
        match decoded.unwrap() {
            InputMessage::Anonymous { data, .. } => {
                assert_eq!(data, vec![1, 2, 3, 4, 5]);
            }
            _ => panic!("Expected Anonymous variant"),
        }
    }

    #[test]
    fn addressed_codec_compatibility() {
        let recipient = test_recipient();
        let data = b"test message payload";

        let mut addressed_codec = AdressedInputMessageCodec(recipient);
        let mut buf = BytesMut::new();
        addressed_codec.encode(data.as_ref(), &mut buf).unwrap();

        let mut input_codec = InputMessageCodec;
        let decoded = input_codec
            .decode(&mut buf)
            .unwrap()
            .expect("Should decode");

        match decoded {
            InputMessage::Regular {
                data: decoded_data,
                recipient: decoded_recipient,
                lane,
                ..
            } => {
                assert_eq!(decoded_data, data, "Data should match");
                assert_eq!(decoded_recipient, recipient, "Recipient should match");
                assert_eq!(lane, TransmissionLane::General, "Should use General lane");
            }
            _ => panic!("Expected Regular variant"),
        }
    }
}
