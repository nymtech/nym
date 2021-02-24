use super::endian::Endian;
use cosmwasm_std::{Api, CanonicalAddr, HumanAddr, StdResult};

pub fn maybe_canonical(
    api: &dyn Api,
    human: Option<HumanAddr>,
) -> StdResult<Option<CanonicalAddr>> {
    human.map(|x| api.canonical_address(&x)).transpose()
}

/// Bound is used to defines the two ends of a range, more explicit than Option<u8>
/// None means that we don't limit that side of the range at all.
/// Include means we use the given bytes as a limit and *include* anything at that exact key
/// Exclude means we use the given bytes as a limit and *exclude* anything at that exact key
#[derive(Clone, Debug)]
pub enum Bound {
    Inclusive(Vec<u8>),
    Exclusive(Vec<u8>),
}

impl Bound {
    /// Turns optional binary, like Option<CanonicalAddr> into an inclusive bound
    pub fn inclusive<T: Into<Vec<u8>>>(limit: T) -> Self {
        Bound::Inclusive(limit.into())
    }

    /// Turns optional binary, like Option<CanonicalAddr> into an exclusive bound
    pub fn exclusive<T: Into<Vec<u8>>>(limit: T) -> Self {
        Bound::Exclusive(limit.into())
    }

    /// Turns an int, like Option<u32> into an inclusive bound
    pub fn inclusive_int<T: Endian>(limit: T) -> Self {
        Bound::Inclusive(limit.to_be_bytes().into())
    }

    /// Turns an int, like Option<u64> into an exclusive bound
    pub fn exclusive_int<T: Endian>(limit: T) -> Self {
        Bound::Exclusive(limit.to_be_bytes().into())
    }
}
