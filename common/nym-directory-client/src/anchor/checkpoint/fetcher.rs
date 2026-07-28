// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::checkpoint::provider::CheckpointProvider;
use crate::anchor::checkpoint::{Checkpoint, SignedCheckpoint};
use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519;
use tracing::warn;

/// Fetches the raw checkpoint datum (JSON body) from a URL. The transport is injected so this
/// crate stays free of a concrete HTTP dependency: the producer (nym-api) supplies a real
/// client, tests supply a canned one. Each implementor names its own concrete transport error
/// via [`CheckpointFetcher::Error`]; the provider only needs to `Display` it for a log line.
#[async_trait]
pub trait CheckpointFetcher: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Fetch and deserialize the [`SignedCheckpoint`] published at internal `url`. The real client can do
    /// this in one step (e.g. `reqwest`'s `.json()`); a deserialization failure is surfaced as
    /// [`CheckpointFetcher::Error`]. This does NOT verify the root signature - that is the
    /// provider's job.
    async fn fetch(&self) -> Result<SignedCheckpoint, Self::Error>;
}

/// Lowest-priority source: a root-signed datum published at a well-known, env-overridable HTTPS
/// URL. The source is untrusted - it provides availability only, and the datum is root-verified
/// before use exactly like the hardcoded one.
pub struct HttpsCheckpointProvider<F> {
    fetcher: F,
    root: ed25519::PublicKey,
}

impl<F: CheckpointFetcher> HttpsCheckpointProvider<F> {
    pub fn new(fetcher: F, root: ed25519::PublicKey) -> Self {
        HttpsCheckpointProvider { fetcher, root }
    }
}

#[async_trait]
impl<F: CheckpointFetcher> CheckpointProvider for HttpsCheckpointProvider<F> {
    async fn candidate(&self) -> Option<Checkpoint> {
        match self.fetcher.fetch().await {
            Ok(signed) => signed.verify_from_source(&self.root, "https"),
            Err(err) => {
                warn!("failed to fetch checkpoint: {err}");
                None
            }
        }
    }
}

/// A canned [`CheckpointFetcher`]: `Some(signed)` serves it, `None` fails the transport.
#[cfg(test)]
pub(crate) struct MockFetcher(pub(crate) Option<SignedCheckpoint>);

#[cfg(test)]
#[async_trait]
impl CheckpointFetcher for MockFetcher {
    type Error = std::io::Error;

    async fn fetch(&self) -> Result<SignedCheckpoint, std::io::Error> {
        self.0
            .clone()
            .ok_or_else(|| std::io::Error::other("mock transport failure"))
    }
}

#[cfg(test)]
pub(crate) fn mock_https_provider(
    fetcher: MockFetcher,
    root: &ed25519::KeyPair,
) -> HttpsCheckpointProvider<MockFetcher> {
    HttpsCheckpointProvider::new(fetcher, *root.public_key())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::signed_checkpoint;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use time::OffsetDateTime;
    use time::macros::datetime;

    const MINTED_AT: OffsetDateTime = datetime!(2026-07-02 13:42:10+00:00);

    #[tokio::test]
    async fn https_transport_failure_is_ignored() {
        let root = dummy_ed25519_keypair(1);
        let https = mock_https_provider(MockFetcher(None), &root);
        assert!(https.candidate().await.is_none());
    }

    #[tokio::test]
    async fn https_with_a_bad_root_signature_is_ignored() {
        let real_root = dummy_ed25519_keypair(1);
        let impostor = dummy_ed25519_keypair(2);
        // datum signed by the impostor but served over the real-root HTTPS source
        let https = mock_https_provider(
            MockFetcher(Some(signed_checkpoint(&impostor, MINTED_AT))),
            &real_root,
        );
        assert!(https.candidate().await.is_none());
    }
}
