/// A record contributed by any crate describing how to mutate a reqwest::ClientBuilder.
use crate::ReqwestClientBuilder;

pub struct ConfigRecord {
    /// Lower numbers run earlier.
    pub priority: i32,
    /// A function that takes a builder and returns a mutated builder.
    pub apply: fn(ReqwestClientBuilder) -> ReqwestClientBuilder,
}

inventory::collect!(ConfigRecord);

pub fn default_builder() -> ReqwestClientBuilder {
    let mut b = ReqwestClientBuilder::new();
    let mut records: Vec<&'static ConfigRecord> =
        inventory::iter::<ConfigRecord>
            .into_iter()
            .collect();
    records.sort_by_key(|r| r.priority); // lower runs first
    for r in records {
        b = (r.apply)(b);
    }
    b
}
pub fn build_client() -> reqwest::Result<reqwest::Client> {
    default_builder().build()
}
