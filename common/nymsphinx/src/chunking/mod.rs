use crate::chunking::set::split_into_sets;
use addressing::AddressTypeError;
use sphinx::route::{Destination, Node};
use std::net::SocketAddr;
use std::time;
use topology::{NymTopology, NymTopologyError};

pub mod fragment;
pub mod set;

/// The idea behind the process of chunking is to incur as little data overhead as possible due
/// to very computationally costly sphinx encapsulation procedure.
///
/// To achieve this, the underlying message is split into so-called "sets", which are further
/// subdivided into the base unit of "fragment" that is directly encapsulated by a Sphinx packet.
/// This allows to encapsulate messages of arbitrary length.
///
/// Each message, regardless of its size, consists of at least a single `Set` that has at least
/// a single `Fragment`.
///
/// Each `Fragment` can have variable, yet fully deterministic, length,
/// that depends on its position in the set as well as total number of sets. This is further
/// explained in `fragment.rs` file.  
///
/// Similarly, each `Set` can have a variable number of `Fragment`s inside. However, that
/// value is more restrictive: if it's the last set into which the message was split
/// (or implicitly the only one), it has no lower bound on the number of `Fragment`s.
/// (Apart from the restriction of containing at least a single one). If the set is located
/// somewhere in the middle, *it must be* full. Finally, regardless of its position, it must also be
/// true that it contains no more than `u8::max_value()`, i.e. 255 `Fragment`s.
/// Again, the reasoning for this is further explained in `set.rs` file. However, you might
/// also want to look at `fragment.rs` to understand the full context behind that design choice.
///
/// Both of those concepts as well as their structures, i.e. `Set` and `Fragment`
/// are further explained in the respective files.

#[derive(PartialEq, Debug)]
pub enum ChunkingError {
    InvalidPayloadLengthError,
    TooBigMessageToSplit,
    MalformedHeaderError,
    NoValidProvidersError,
    NoValidRoutesAvailableError,
    InvalidTopologyError,
    TooShortFragmentData,
    MalformedFragmentData,
    UnexpectedFragmentCount,
}

impl From<topology::NymTopologyError> for ChunkingError {
    fn from(_: NymTopologyError) -> Self {
        use ChunkingError::*;
        NoValidRoutesAvailableError
    }
}

// this will later be completely removed when `addressing` crate is moved into this crate
impl From<AddressTypeError> for ChunkingError {
    fn from(_: AddressTypeError) -> Self {
        use ChunkingError::*;
        InvalidTopologyError
    }
}

// the user of this library can either prepare payloads for sphinx packets that he needs to
// encapsulate themselves by creating [sphinx] headers.
// or alternatively provide network topology and get bytes ready to be sent over the network

/// Takes the entire message and splits it into bytes chunks that will fit into sphinx packets
/// directly. After receiving they can be combined using `reconstruction::MessageReconstructor`
/// to obtain the original message back.
pub fn split_and_prepare_payloads(message: &[u8]) -> Vec<Vec<u8>> {
    let fragmented_messages = split_into_sets(message);
    fragmented_messages
        .into_iter()
        .flat_map(|fragment_set| fragment_set.into_iter())
        .map(|fragment| fragment.into_bytes())
        .collect()
}

// note that for very long messages, this function will take a very long time to complete
// due to expensive sphinx wrapping operations

/// Takes the entire message and splits it into bytes chunks that are then encapsulated into
/// sphinx packets usingn provided network topology. The resultant bytes chunks can be sent
/// directly on the wire to the `SocketAddr` part of the tuple.
/// After being received back by a client they can be combined using `reconstruction::MessageReconstructor`
/// to obtain the original message back.
pub fn split_and_encapsulate_message<T: NymTopology>(
    message: &[u8],
    // TODO: in the future this will require also a specific provider of this particular recipient
    recipient: Destination,
    average_delay: time::Duration,
    topology: &T,
) -> Result<Vec<(SocketAddr, Vec<u8>)>, ChunkingError> {
    let ready_payloads = split_and_prepare_payloads(message);

    let mut providers = topology.providers();
    if providers.is_empty() {
        return Err(ChunkingError::NoValidProvidersError);
    }
    let provider: Node = providers.pop().unwrap().into();

    let mut encapsulated_sphinx_packets = Vec::new();
    for message_fragment in ready_payloads {
        let route = topology.route_to(provider.clone())?;
        let delays =
            sphinx::header::delays::generate_from_average_duration(route.len(), average_delay);
        let packet =
            sphinx::SphinxPacket::new(message_fragment, &route[..], &recipient, &delays).unwrap(); // this cannot fail unless there's an underlying bug which we must find and fix anyway
        let first_node_address = addressing::socket_address_from_encoded_bytes(
            route.first().unwrap().address.to_bytes(),
        )?;

        encapsulated_sphinx_packets.push((first_node_address, packet.to_bytes()));
    }

    Ok(encapsulated_sphinx_packets)
}
