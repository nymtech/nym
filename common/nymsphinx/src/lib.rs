use rand::{thread_rng, Rng};

struct NymSphinxPacket {
    header: NymSphinxHeader,
    payload: Vec<u8>,
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

    fn fragmented_max_len() -> usize {
        sphinx::constants::PAYLOAD_SIZE - NymSphinxHeader::fragmented_len()
    }

    fn unfragmented_max_len() -> usize {
        sphinx::constants::PAYLOAD_SIZE - NymSphinxHeader::unfragmented_len()
    }
}

// The header is represented as follows:
// IF || 31 bit ID
// TF || CF
// note that if IF is not set, then the remaining bytes in the header are used as payload
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
            total_fragments: Default::default(),
            current_fragment: Default::default(),
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
    if message.len() <= NymSphinxPacket::unfragmented_max_len() {
        // no need to fragment it
        vec![NymSphinxPacket::new_unfragmented(message)]
    } else {
        let mut rng = thread_rng();
        let fragmentation_id = rng.gen::<i32>().abs();
        let num_fragments =
            (message.len() as f64 / NymSphinxPacket::fragmented_max_len() as f64).ceil() as usize;

        if num_fragments > u8::max_value() as usize {
            panic!("todo implement behaviour for this case")
        }

        let mut packets = Vec::with_capacity(num_fragments);
        let fsize = NymSphinxPacket::fragmented_max_len();
        for i in 0..num_fragments {
            let lb = i * fsize;
            let ub = if i == (num_fragments - 1) {
                // final fragment
                i * fsize + message.len() % NymSphinxPacket::fragmented_max_len()
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

// allow each messsage to be at most 2^8 payload sizes (is this enough?)
pub fn split_and_encapsulate_message(message: &[u8]) -> Vec<Vec<u8>> {
    unimplemented!()
}

struct MessageReconstructor {
    buffer: Vec<String>,
}

#[cfg(test)]
mod preparing_payload {
    use super::*;
    use rand::RngCore;

    #[test]
    fn correctly_works_for_messages_shorter_than_unfragmented_length() {
        let mut rng = thread_rng();

        let mlen = NymSphinxPacket::unfragmented_max_len() - 20;
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

        let mlen = NymSphinxPacket::fragmented_max_len() * 15 + 531;
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
                    &message[i * NymSphinxPacket::fragmented_max_len()..]
                );
            } else {
                assert_eq!(
                    fragments[i].payload.len(),
                    NymSphinxPacket::fragmented_max_len(),
                );
                assert_eq!(
                    fragments[i].payload,
                    &message[i * NymSphinxPacket::fragmented_max_len()
                        ..(i + 1) * NymSphinxPacket::fragmented_max_len()]
                );
                assert_eq!(fragments[i].header.id, fragments[i + 1].header.id)
            }
        }
    }
}
