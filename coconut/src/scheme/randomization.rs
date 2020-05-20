use crate::parameters::SomeRngTrait;
use crate::{Credential, Params};

/// `randomize_credential` randomizes the Coconut credential such that it becomes indistinguishable
/// from a fresh credential on different attributes.
pub fn randomize_credential<R: SomeRngTrait>(
    params: &Params<R>,
    credential: &Credential,
) -> Credential {
    unimplemented!()
}
