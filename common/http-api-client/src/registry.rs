

/// A record contributed by any crate describing how to mutate a reqwest::ClientBuilder.
pub use reqwest;

pub struct ConfigRecord {
    /// Lower numbers run earlier.
    pub priority: i32,
    /// Optional namespace/scope to filter which defaults apply.
    pub scope: Option<&'static str>,
    /// A function that takes a builder and returns a mutated builder.
    pub apply: fn(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
}

// Collect all records linked into the final binary.
inventory::collect!(ConfigRecord);

/// Start a ClientBuilder and apply all registered defaults for the given `scope`.
pub fn default_builder(scope: Option<&str>) -> reqwest::ClientBuilder {
    let mut b = reqwest::ClientBuilder::new();
    // Collect and sort by priority for deterministic order.
    let mut items: Vec<&'static ConfigRecord> = inventory::iter::<ConfigRecord>.into_iter().collect();
    items.sort_by_key(|r| r.priority);
    for rec in items {
        let pass = match (rec.scope, scope) {
            (None, _) => true,
            (Some(s), Some(q)) => s == q,
            (Some(_), None) => false,
        };
        if pass {
            b = (rec.apply)(b);
        }
    }
    b
}

/// Build a reqwest::Client with defaults for the given `scope` (or all global defaults if None).
pub fn build_client(scope: Option<&str>) -> reqwest::Result<reqwest::Client> {
    default_builder(scope).build()
}