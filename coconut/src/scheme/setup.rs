use crate::parameters::SomeRngTrait;
use crate::Params;
use crate::Result;

/// `setup` generates the public parameters required by the Coconut scheme.
/// `num_attributes` indicates the maximum number of attributes
/// that can be embed in the credentials.
pub fn setup<R: SomeRngTrait>(num_attributes: u32) -> Result<Params<R>> {
    unimplemented!()
}
