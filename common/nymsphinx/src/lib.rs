use addressing::AddressTypeError;
use rand::{thread_rng, Rng};
use sphinx::route::{Destination, Node};
use std::convert::TryInto;
use std::net::SocketAddr;
use std::time;
use topology::{NymTopology, NymTopologyError};

// max 255 fragments, each having max size of max sphinx packet size minus fragmented header overhead
pub const MAX_MESSAGE_LENGTH: usize =
    u8::max_value() as usize * (sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH - 6);

#[derive(PartialEq, Clone, Debug)]
struct NymSphinxPacket {
    header: NymSphinxHeader,
    payload: Vec<u8>,
}

#[derive(PartialEq, Debug)]
pub enum NymSphinxError {
    TooBigMessageToSplit,
    NoValidProvidersError,
    NoValidRoutesAvailableError,
    InvalidTopologyError,
    TooShortMessage,
    TooLongPayload,
    TooShortPayload,
    UnexpectedFragmentCount,
}

impl From<topology::NymTopologyError> for NymSphinxError {
    fn from(_: NymTopologyError) -> Self {
        use NymSphinxError::*;
        NoValidRoutesAvailableError
    }
}

// this will later be completely removed as `addressing` is moved into this crate
impl From<AddressTypeError> for NymSphinxError {
    fn from(_: AddressTypeError) -> Self {
        use NymSphinxError::*;
        InvalidTopologyError
    }
}

//<R: Rng>(rng: &mut R)

impl NymSphinxPacket {
    fn new_fragmented(message: &[u8], id: i32, total_fragments: u8, current_fragment: u8) -> Self {
        assert!(id >= 0);
        NymSphinxPacket {
            header: NymSphinxHeader::new_fragmented(id, total_fragments, current_fragment),
            payload: message.to_vec(),
        }
    }

    fn new_unfragmented(message: &[u8]) -> Self {
        NymSphinxPacket {
            header: NymSphinxHeader::new_unfragmented(),
            payload: message.to_vec(),
        }
    }

    fn fragmented_payload_max_len() -> usize {
        sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH - NymSphinxHeader::fragmented_len()
    }

    fn unfragmented_payload_max_len() -> usize {
        sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH - NymSphinxHeader::unfragmented_len()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.header
            .to_bytes()
            .into_iter()
            .chain(self.payload.into_iter())
            .collect()
    }

    fn try_from_bytes(b: &[u8]) -> Result<Self, NymSphinxError> {
        // check if it's fragmented - if it was - the whole first byte is set to 0
        // otherwise first bit is set to 1
        if b.is_empty() {
            return Err(NymSphinxError::TooShortMessage);
        }
        if b[0] == 0 {
            if b.len()
                > NymSphinxPacket::unfragmented_payload_max_len()
                    + NymSphinxHeader::unfragmented_len()
            {
                return Err(NymSphinxError::TooLongPayload);
            }
            Ok(NymSphinxPacket {
                header: NymSphinxHeader::new_unfragmented(),
                payload: b[NymSphinxHeader::unfragmented_len()..].to_vec(),
            })
        } else {
            if b.len() < NymSphinxHeader::fragmented_len() {
                return Err(NymSphinxError::TooShortMessage);
            }
            if b.len()
                > NymSphinxPacket::fragmented_payload_max_len() + NymSphinxHeader::fragmented_len()
            {
                return Err(NymSphinxError::TooLongPayload);
            }

            let frag_id = i32::from_be_bytes(b[0..4].try_into().unwrap());
            // sanity check for the fragmentation flag
            assert!(((frag_id >> 31) & 1) != 0);

            let id = frag_id & !(1 << 31);
            let total_fragments = b[4];
            let current_fragment = b[5];

            if total_fragments != current_fragment
                && b.len()
                    != NymSphinxPacket::fragmented_payload_max_len()
                        + NymSphinxHeader::fragmented_len()
            {
                return Err(NymSphinxError::TooShortPayload);
            }

            Ok(NymSphinxPacket {
                header: NymSphinxHeader::new_fragmented(id, total_fragments, current_fragment),
                payload: b[NymSphinxHeader::fragmented_len()..].to_vec(),
            })
        }
    }
}

// The header is represented as follows:
// IF || 31 bit ID
// TF || CF
// note that if IF is not set, then the remaining bytes in the header are used as payload
#[derive(PartialEq, Clone, Debug)]
struct NymSphinxHeader {
    is_fragmented: bool,
    // id is a value in the 0, I32_MAX range
    id: i32,
    // since payload is always fragmented into packets of constant length
    // (apart from possibly the last one), there's no need to use offsets like ipv4/ipv6.
    // just enumerate the fragments.
    total_fragments: u8,
    current_fragment: u8,
}

impl NymSphinxHeader {
    fn new_fragmented(id: i32, total_fragments: u8, current_fragment: u8) -> Self {
        assert!(id >= 0);
        NymSphinxHeader {
            is_fragmented: true,
            id,
            total_fragments,
            current_fragment,
        }
    }

    fn new_unfragmented() -> Self {
        NymSphinxHeader {
            is_fragmented: false,
            // The below fields are meaningless if the message is not fragmented
            id: Default::default(),
            total_fragments: 1,
            current_fragment: 1,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        if self.is_fragmented {
            let frag_id = self.id | (1 << 31);
            let frag_id_bytes = frag_id.to_be_bytes();
            frag_id_bytes
                .iter()
                .cloned()
                .chain(std::iter::once(self.total_fragments))
                .chain(std::iter::once(self.current_fragment))
                .collect()
        } else {
            [0].to_vec()
        }
    }

    fn fragmented_len() -> usize {
        6
    }

    fn unfragmented_len() -> usize {
        // if it's unfragmented, we only need a single bit to represent the fragmentation flag.
        // however, we operate on bytes hence we need a full byte for it
        1
    }
}

fn prepare_payloads(message: &[u8]) -> Vec<NymSphinxPacket> {
    if message.len() <= NymSphinxPacket::unfragmented_payload_max_len() {
        // no need to fragment it
        vec![NymSphinxPacket::new_unfragmented(message)]
    } else {
        let mut rng = thread_rng();
        // Technically this method of generating fragmentation_id is not perfectly uniform,
        // as 0 has lower probability of being chosen compared to any other value.
        // But is it really an issue considering this doesn't need to be cryptographically secure?
        let fragmentation_id = rng.gen::<i32>().abs(); // as u32; // this guarantees the number fits in 31 bits
        let num_fragments = (message.len() as f64
            / NymSphinxPacket::fragmented_payload_max_len() as f64)
            .ceil() as usize;

        if num_fragments > u8::max_value() as usize {
            panic!("todo implement behaviour for this case")
        }

        let mut packets = Vec::with_capacity(num_fragments);
        let fsize = NymSphinxPacket::fragmented_payload_max_len();
        for i in 0..num_fragments {
            let lb = i * fsize;
            let ub = if i == (num_fragments - 1) {
                // final fragment - read until the end
                message.len()
            } else {
                (i + 1) * fsize
            };
            packets.push(NymSphinxPacket::new_fragmented(
                &message[lb..ub],
                fragmentation_id,
                num_fragments as u8,
                (i + 1) as u8,
            ))
        }

        packets
    }
}

// the user of this library can either prepare payloads for sphinx packets that he needs to
// encapsulate themselves by creating headers.
// or alternatively provide network topology and get bytes ready to be sent over the network

pub fn split_and_prepare_payloads(message: &[u8]) -> Result<Vec<Vec<u8>>, NymSphinxError> {
    if message.len() > MAX_MESSAGE_LENGTH {
        // TODO: perhaps to make messages arbitrary long, for last header include fragmentation id of next part
        return Err(NymSphinxError::TooBigMessageToSplit);
    }

    let fragmented_messages = prepare_payloads(message);
    Ok(fragmented_messages
        .into_iter()
        .map(|frag| frag.into_bytes())
        .collect())
}

pub fn split_and_encapsulate_message<T: NymTopology>(
    message: &[u8],
    // TODO: in the future this will require also a specific provider of this particular recipient
    recipient: Destination,
    average_delay: time::Duration,
    topology: &T,
) -> Result<Vec<(SocketAddr, Vec<u8>)>, NymSphinxError> {
    if message.len() > MAX_MESSAGE_LENGTH {
        return Err(NymSphinxError::TooBigMessageToSplit);
    }

    let fragmented_messages = prepare_payloads(message);

    let mut providers = topology.providers();
    if providers.is_empty() {
        return Err(NymSphinxError::NoValidProvidersError);
    }
    let provider: Node = providers.pop().unwrap().into();

    let mut encapsulated_sphinx_packets = Vec::new();
    for fragment in fragmented_messages {
        let route = topology.route_to(provider.clone())?;
        let delays =
            sphinx::header::delays::generate_from_average_duration(route.len(), average_delay);
        let packet =
            sphinx::SphinxPacket::new(fragment.into_bytes(), &route[..], &recipient, &delays)
                .unwrap(); // this cannot fail unless there's an underlying bug which we must find and fix anyway
        let first_node_address = addressing::socket_address_from_encoded_bytes(
            route.first().unwrap().address.to_bytes(),
        )?;

        encapsulated_sphinx_packets.push((first_node_address, packet.to_bytes()));
    }

    Ok(encapsulated_sphinx_packets)
}

type ReconstructionBuffer = Vec<Option<NymSphinxPacket>>;

pub struct MessageReconstructor {
    // TODO: some cleaner thread that if message is incomplete and we haven't received any fragments
    // in X time, we assume they were lost and message can't be restored
    reconstructed_messages: HashMap<i32, ReconstructionBuffer>,
}

impl MessageReconstructor {
    fn new() -> Self {
        MessageReconstructor {
            reconstructed_messages: HashMap::new(),
        }
    }

    fn is_message_fully_received(&self, id: i32) -> bool {
        self.reconstructed_messages
            .get(&id)
            .map(|buf| !buf.contains(&None))
            .unwrap_or_else(|| false)
    }

    pub fn new_fragment(&mut self, frag_data: Vec<u8>) -> Option<Vec<u8>> {
        let fragment = NymSphinxPacket::try_from_bytes(&frag_data).ok()?;

        let msg_id = fragment.header.id;
        let message_len = fragment.header.total_fragments as usize;
        let frag_index = fragment.header.current_fragment as usize - 1;

        let buf = self
            .reconstructed_messages
            .entry(msg_id)
            .or_insert(Vec::with_capacity(message_len));
        buf.resize(message_len, None);

        if buf[frag_index].is_some() {
            // TODO: how to behave in this case?
            panic!("We already received this fragment before!");
        }
        buf[frag_index] = Some(fragment);

        if self.is_message_fully_received(msg_id) {
            let received_buffer = self.reconstructed_messages.remove(&msg_id).unwrap();
            let original_message: Vec<_> = received_buffer
                .into_iter()
                .map(|frag| frag.unwrap())
                .flat_map(|frag| frag.payload.into_iter())
                .collect();
            Some(original_message)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod nym_sphinx_packet {
    use super::*;
    use rand::RngCore;

    #[test]
    fn can_be_converted_to_and_from_bytes_for_unfragmented_payload() {
        let mut rng = thread_rng();

        let mlen = NymSphinxPacket::unfragmented_payload_max_len() - 20;
        let mut valid_message = vec![0u8; mlen];
        rng.fill_bytes(&mut valid_message);

        let valid_unfragmented_packet = NymSphinxPacket {
            header: NymSphinxHeader::new_unfragmented(),
            payload: valid_message,
        };
        let packet_bytes = valid_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            valid_unfragmented_packet,
            NymSphinxPacket::try_from_bytes(&packet_bytes).unwrap()
        );

        let empty_unfragmented_packet = NymSphinxPacket {
            header: NymSphinxHeader::new_unfragmented(),
            payload: Vec::new(),
        };
        let packet_bytes = empty_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            empty_unfragmented_packet,
            NymSphinxPacket::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut full_message = vec![0u8; NymSphinxPacket::fragmented_payload_max_len()];
        rng.fill_bytes(&mut full_message);

        let full_unfragmented_packet = NymSphinxPacket {
            header: NymSphinxHeader::new_unfragmented(),
            payload: full_message,
        };
        let packet_bytes = full_unfragmented_packet.clone().into_bytes();
        assert_eq!(
            full_unfragmented_packet,
            NymSphinxPacket::try_from_bytes(&packet_bytes).unwrap()
        );
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_long_unfragmented_payload() {
        let mut rng = thread_rng();

        let mlen = NymSphinxPacket::unfragmented_payload_max_len() + 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = NymSphinxPacket {
            header: NymSphinxHeader::new_unfragmented(),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            NymSphinxPacket::try_from_bytes(&packet_bytes),
            Err(NymSphinxError::TooLongPayload)
        );
    }

    #[test]
    fn can_be_converted_to_and_from_bytes_for_fragmented_payload() {
        let mut rng = thread_rng();

        let mut msg = vec![0u8; NymSphinxPacket::fragmented_payload_max_len()];
        rng.fill_bytes(&mut msg);

        let non_last_packet = NymSphinxPacket {
            header: NymSphinxHeader::new_fragmented(12345, 10, 5),
            payload: msg,
        };
        let packet_bytes = non_last_packet.clone().into_bytes();
        assert_eq!(
            non_last_packet,
            NymSphinxPacket::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut msg = vec![0u8; NymSphinxPacket::fragmented_payload_max_len()];
        rng.fill_bytes(&mut msg);

        let last_full_packet = NymSphinxPacket {
            header: NymSphinxHeader::new_fragmented(12345, 10, 10),
            payload: msg,
        };
        let packet_bytes = last_full_packet.clone().into_bytes();
        assert_eq!(
            last_full_packet,
            NymSphinxPacket::try_from_bytes(&packet_bytes).unwrap()
        );

        let mut msg = vec![0u8; NymSphinxPacket::fragmented_payload_max_len() - 20];
        rng.fill_bytes(&mut msg);

        let last_non_full_packet = NymSphinxPacket {
            header: NymSphinxHeader::new_fragmented(12345, 10, 10),
            payload: msg,
        };
        let packet_bytes = last_non_full_packet.clone().into_bytes();
        assert_eq!(
            last_non_full_packet,
            NymSphinxPacket::try_from_bytes(&packet_bytes).unwrap()
        );
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_long_fragmented_payload() {
        let mut rng = thread_rng();

        let mlen = NymSphinxPacket::fragmented_payload_max_len() + 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = NymSphinxPacket {
            header: NymSphinxHeader::new_fragmented(12345, 10, 5),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            NymSphinxPacket::try_from_bytes(&packet_bytes),
            Err(NymSphinxError::TooLongPayload)
        );
    }

    #[test]
    fn conversion_from_bytes_fails_for_too_short_fragmented_payload_if_not_last() {
        let mut rng = thread_rng();

        let mlen = NymSphinxPacket::fragmented_payload_max_len() - 1;
        let mut message = vec![0u8; mlen];
        rng.fill_bytes(&mut message);

        let packet = NymSphinxPacket {
            header: NymSphinxHeader::new_fragmented(12345, 10, 5),
            payload: message,
        };

        let packet_bytes = packet.into_bytes();
        assert_eq!(
            NymSphinxPacket::try_from_bytes(&packet_bytes),
            Err(NymSphinxError::TooShortPayload)
        );
    }
}

#[cfg(test)]
mod preparing_payload {
    use super::*;
    use rand::RngCore;

    #[test]
    fn correctly_works_for_messages_shorter_than_unfragmented_length() {
        let mut rng = thread_rng();

        let mlen = NymSphinxPacket::unfragmented_payload_max_len() - 20;
        let mut message = vec![0u8; mlen];

        // use random message content to make sure we wouldn't end up in edge case due to say all
        // bytes being 0 or 1 or whatever. This gives way higher probability of not getting that
        rng.fill_bytes(&mut message);

        let fragments = prepare_payloads(&message);
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].payload, message.to_vec());
        assert!(!fragments[0].header.is_fragmented);
    }

    #[test]
    fn correctly_works_for_messages_longer_than_unfragmented_length() {
        let mut rng = thread_rng();

        let mlen = NymSphinxPacket::fragmented_payload_max_len() * 15 + 531;
        let mut message = vec![0u8; mlen];

        rng.fill_bytes(&mut message);
        let fragments = prepare_payloads(&message);
        assert_eq!(fragments.len(), 16);

        for i in 0..16 {
            assert_eq!(fragments[i].header.current_fragment, (i + 1) as u8);
            assert_eq!(fragments[i].header.total_fragments, 16);
            assert!(fragments[i].header.is_fragmented);

            if fragments[i].header.total_fragments == fragments[i].header.current_fragment {
                assert_eq!(fragments[i].payload.len(), 531);
                assert_eq!(
                    fragments[i].payload,
                    &message[i * NymSphinxPacket::fragmented_payload_max_len()..]
                );
            } else {
                assert_eq!(
                    fragments[i].payload.len(),
                    NymSphinxPacket::fragmented_payload_max_len(),
                );
                assert_eq!(
                    fragments[i].payload,
                    &message[i * NymSphinxPacket::fragmented_payload_max_len()
                        ..(i + 1) * NymSphinxPacket::fragmented_payload_max_len()]
                );
                assert_eq!(fragments[i].header.id, fragments[i + 1].header.id)
            }
        }
    }
}
