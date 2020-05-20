pub use aggregate::{aggregate_credentials, aggregate_keys};
pub use randomization::randomize_credential;

pub mod aggregate;
pub mod issue_credential;
pub mod keygen;
pub mod randomization;
pub mod setup;
pub mod show_credential;

pub type Attribute = (); // probably Scalar?
