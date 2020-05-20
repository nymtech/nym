use crate::parameters::SomeRngTrait;
use crate::{Credential, Params, Result, VerificationKey};

pub type Index = ();
// TODO: perhaps some auxiliary data structure to couple verification key with index
// and another one to couple credential with index
// maybe something like:
// struct CredentialShare {
//     credential: Credential,
//     index: Option<Index>,
// }
// struct VerificationKeyShare {
//     verification_key: VerificationKey,
//     index: Option<Index>,
// }
// ?

/// `aggregate_keys` aggregates verification keys of the signing authorities.
/// Optionally it does so in a threshold manner.
pub fn aggregate_keys<R: SomeRngTrait>(
    params: &Params<R>,
    keys: &[&VerificationKey],
    indices: Option<Vec<Index>>,
) -> Result<VerificationKey> {
    unimplemented!()
}

/// `aggregate_credentials` aggregates Coconut credentials on the same set of attributes
/// that were produced by multiple signing authorities.
/// Optionally it does so in a threshold manner.
pub fn aggregate_credentials<R: SomeRngTrait>(
    params: &Params<R>,
    credentials: &[&Credential],
    indices: Option<Vec<Index>>,
) -> Result<Credential> {
    unimplemented!()
}
