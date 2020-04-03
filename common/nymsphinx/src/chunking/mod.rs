use crate::chunking::set::split_into_sets;

pub mod fragment;
pub mod reconstruction;
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
