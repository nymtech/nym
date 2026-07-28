// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::checkpoint::Checkpoint;
use crate::anchor::checkpoint::NYX_TRUSTING_PERIOD;
use crate::anchor::helpers::get_trusted_directory_digest;
use crate::anchor::{DirectoryTrustAnchor, TrustedDigest};
use crate::error::DirectoryClientError;
use async_trait::async_trait;
use cosmrs::AccountId;
use cosmrs::tendermint::AppHash;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt, ValidatorSet};
use std::collections::BTreeMap;
use std::time::Duration;
use tendermint_light_client::light_client::Options;
use tendermint_light_client::types::{
    Hash, SignedHeader, Time, TrustThreshold, TrustedBlockState, UntrustedBlockState,
};
use tendermint_light_client::verifier::{ProdVerifier, Verdict, Verifier};
use tokio::sync::Mutex;
use tracing::debug;

/// Sane defaults for the Nym mainnet: trust threshold 1/3 (required for skip/bisection
/// verification), the [`NYX_TRUSTING_PERIOD`], and a 5-second clock-drift allowance.
pub fn nyx_default_options() -> Options {
    Options {
        trust_threshold: TrustThreshold::ONE_THIRD,
        trusting_period: NYX_TRUSTING_PERIOD,
        clock_drift: Duration::from_secs(5),
    }
}

/// Owned state from which `TrustedBlockState<'_>` is constructed on demand.
#[derive(Clone)]
struct TrustedAnchorState {
    chain_id: cosmrs::tendermint::chain::Id,
    header_time: Time,
    height: Height,
    next_validators: ValidatorSet,
    next_validators_hash: Hash,
}

impl From<Checkpoint> for TrustedAnchorState {
    fn from(checkpoint: Checkpoint) -> Self {
        TrustedAnchorState {
            chain_id: checkpoint.signed_header.header.chain_id,
            header_time: checkpoint.signed_header.header.time,
            height: checkpoint.height,
            next_validators_hash: checkpoint.signed_header.header.next_validators_hash,
            next_validators: checkpoint.next_validators,
        }
    }
}

impl TrustedAnchorState {
    fn as_trusted_block_state(&self) -> TrustedBlockState<'_> {
        TrustedBlockState {
            chain_id: &self.chain_id,
            header_time: self.header_time,
            height: self.height,
            next_validators: &self.next_validators,
            next_validators_hash: self.next_validators_hash,
        }
    }

    fn advance(&mut self, signed_header: SignedHeader, next_validators: ValidatorSet) {
        self.chain_id = signed_header.header.chain_id.clone();
        self.header_time = signed_header.header.time;
        self.height = signed_header.header.height;
        self.next_validators_hash = signed_header.header.next_validators_hash;
        self.next_validators = next_validators;
    }
}

struct LightClientAnchorState {
    /// The immutable pinned checkpoint. Used as the base for verifying heights the advancing
    /// head has already passed (the verifier only moves forward, so we cannot re-verify a
    /// below-head height from `trusted`).
    checkpoint: TrustedAnchorState,

    /// The furthest-verified state, advanced forward by monotonically-increasing queries.
    trusted: TrustedAnchorState,

    /// Cache: query height `H` -> `header[H+1].app_hash` (the app state committed at `H`).
    app_hash_cache: BTreeMap<Height, AppHash>,
}

pub struct LightClientAnchor<C> {
    client: C,

    directory_contract: AccountId,

    // we only need Mutex to be able to take &self without mutable reference
    // there's no concurrent access anywhere
    state: Mutex<LightClientAnchorState>,

    options: Options,

    verifier: ProdVerifier,
}

impl<C> LightClientAnchor<C> {
    pub fn new(
        client: C,
        directory_contract: AccountId,
        checkpoint: Checkpoint,
        options: Options,
    ) -> Self {
        let mut app_hash_cache = BTreeMap::new();
        // the checkpoint's own header commits state at `checkpoint.height - 1`, so we can serve
        // that one height directly without any verification.
        if checkpoint.height.value() > 1 {
            app_hash_cache.insert(
                Height::from(checkpoint.height.value() as u32 - 1),
                checkpoint.signed_header.header.app_hash.clone(),
            );
        }
        let trusted: TrustedAnchorState = checkpoint.into();
        Self {
            client,
            directory_contract,
            state: Mutex::new(LightClientAnchorState {
                checkpoint: trusted.clone(),
                trusted,
                app_hash_cache,
            }),
            options,
            verifier: ProdVerifier::default(),
        }
    }
}

impl<C> LightClientAnchor<C>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    /// Verify the header at `target` directly against `base` via the Tendermint light-client rule.
    ///
    /// Returns `Some((signed_header, next_validators))` on success (`next_validators` is the
    /// set at `target + 1`, ready to become the new trusted state's next-validators),
    /// `None` when the trusted validator overlap is insufficient (caller should bisect),
    /// `Err` on hard verification failures or RPC errors.
    async fn verify_hop(
        &self,
        base: &TrustedAnchorState,
        target: Height,
    ) -> Result<Option<(SignedHeader, ValidatorSet)>, DirectoryClientError> {
        let commit_res = self.client.commit(target).await?;
        if !commit_res.canonical {
            return Err(DirectoryClientError::NonCanonicalCommit(target.value()));
        }
        let validators = ValidatorSet::without_proposer(
            self.client.get_all_validators(target).await?.validators,
        );
        // the new trusted state at `target` must carry the validator set of `target + 1` as its
        // next-validators (that is what skip verification checks the next commit's overlap against).
        let next = Height::from(target.value() as u32 + 1);
        let next_validators =
            ValidatorSet::without_proposer(self.client.get_all_validators(next).await?.validators);

        // pass `next_validators` so the verifier ties it to the verified header's
        // `next_validators_hash` (`next_validators_match`); otherwise the RPC-supplied set we
        // store for the next skip hop would be trusted blindly.
        let untrusted = UntrustedBlockState {
            signed_header: &commit_res.signed_header,
            validators: &validators,
            next_validators: Some(&next_validators),
        };
        let trusted = base.as_trusted_block_state();
        let now = Time::now();

        match self
            .verifier
            .verify_update_header(untrusted, trusted, &self.options, now)
        {
            Verdict::Success => Ok(Some((commit_res.signed_header, next_validators))),
            Verdict::NotEnoughTrust(_) => Ok(None),
            Verdict::Invalid(err) => Err(DirectoryClientError::LightClientVerificationFailed(
                err.to_string(),
            )),
        }
    }

    /// Advance `base` forward to `target` using skip verification with bisection, caching every
    /// verified header's app hash into `cache` along the way.
    ///
    /// Attempts to verify `target` directly (O(1) for a stable validator set). On
    /// `NotEnoughTrust` it bisects: verifies the midpoint, advances `base` to it in place, then
    /// retries the target. Depth is O(log(target - base)). `base` is `&mut` so the below-head
    /// walk (over a local checkpoint clone) makes progress without touching the persisted head.
    async fn walk_to(
        &self,
        base: &mut TrustedAnchorState,
        cache: &mut BTreeMap<Height, AppHash>,
        target: Height,
    ) -> Result<(), DirectoryClientError> {
        let current = base.height;
        if current >= target {
            return Ok(());
        }
        debug!("light-client: advancing from {current} to {target}",);

        if let Some((signed_header, next_validators)) = self.verify_hop(base, target).await? {
            // `header[target]` commits the app state at `target - 1` (CometBFT off-by-one); this
            // holds for any verified header, including bisection midpoints.
            cache.insert(
                Height::from(target.value() as u32 - 1),
                signed_header.header.app_hash.clone(),
            );
            base.advance(signed_header, next_validators);
            return Ok(());
        }

        // NotEnoughTrust: bisect.
        let mid = Height::from((current.value() as u32 + target.value() as u32) / 2);
        debug!("light-client: bisecting [{current}, {target}] via midpoint {mid}");
        Box::pin(self.walk_to(base, cache, mid)).await?;
        Box::pin(self.walk_to(base, cache, target)).await
    }

    /// Ensure `header[target]` is verified and its app hash cached.
    ///
    /// Forward of the head: advance the persisted head. At or below the head (a height the head
    /// already passed but never cached): walk a local clone of the checkpoint up to `target`,
    /// since the verifier cannot re-verify backwards from the head.
    async fn advance_to(
        &self,
        state: &mut LightClientAnchorState,
        target: Height,
    ) -> Result<(), DirectoryClientError> {
        if state.trusted.height >= target {
            if target <= state.checkpoint.height {
                return Err(DirectoryClientError::HeightBelowCheckpoint {
                    requested: target.value().saturating_sub(1),
                    checkpoint: state.checkpoint.height.value(),
                });
            }
            let mut local = state.checkpoint.clone();
            self.walk_to(&mut local, &mut state.app_hash_cache, target)
                .await
        } else {
            self.walk_to(&mut state.trusted, &mut state.app_hash_cache, target)
                .await
        }
    }
}

#[async_trait]
impl<C> DirectoryTrustAnchor for LightClientAnchor<C>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, DirectoryClientError> {
        // the app_hash committing state at H lives in header[H+1] (CometBFT off-by-one)
        let target = Height::from(height.value() as u32 + 1);
        let mut state = self.state.lock().await;

        if let Some(cached) = state.app_hash_cache.get(&height) {
            return Ok(cached.clone());
        }

        self.advance_to(&mut state, target).await?;

        state.app_hash_cache.get(&height).cloned().ok_or_else(|| {
            DirectoryClientError::LightClientVerificationFailed(format!(
                "app_hash for height {height} not in cache after advance"
            ))
        })
    }

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, DirectoryClientError> {
        let app_hash = self.trusted_app_hash(height).await?;

        get_trusted_directory_digest(&self.client, &self.directory_contract, height, app_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmrs::rpc::endpoint::{commit, validators};
    use cosmrs::tendermint::validator::Info;
    use cosmrs::tendermint::{PublicKey, vote};
    use nym_directory_attestation::source::mock::mock_contract;
    use nym_validator_client::nyxd::{Paging, Response};
    use nym_validator_client::rpc::mocks::MockRpcClient;

    // the app_hash committing state at H lives in header[H+1] (CometBFT off-by-one), so the
    // heights below are one apart: checkpoint at 24499896, adjacent block 24499897,
    // and a 10-block skip target 24499906. Fixtures are real `nyx` mainnet RPC responses.

    use crate::test_support::{CHECKPOINT_HEIGHT, checkpoint, checkpoint_fixtures};

    const CHECKPOINT: u32 = CHECKPOINT_HEIGHT;
    const FAR_FUTURE: Duration = Duration::from_secs(100000000);

    // adjacent block 24499897 with its next-height (24499898) validator set
    fn adjacent_fixtures() -> (commit::Response, validators::Response) {
        // commit response at height 24499897
        let commit = commit::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"signed_header":{"header":{"version":{"block":"11"},"chain_id":"nyx","height":"24499897","time":"2026-07-02T13:42:16.360156178Z","last_block_id":{"hash":"1BDC0588F03C5DF679C16FBD5D8145734FCF4C1ACBD36D7AA37B6232F26E8842","parts":{"total":1,"hash":"B6C5B357EF913752F0F7763B71D2374BD5E7334F2E5EE4D1755F8397191F6886"}},"last_commit_hash":"86B5B2E752438630EC32B08FB6489B5ADE40A4489678FA34410489C5D9BDCD87","data_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","next_validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","consensus_hash":"048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F","app_hash":"4DA8720ED589322E5CDE433DBBC390CEA4B1E7C28982D8C404438393DB655FE6","last_results_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","evidence_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","proposer_address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1"},"commit":{"height":"24499897","round":0,"block_id":{"hash":"4FD17927DA1EC2F1F1A0076683D11FB93916486C64CE6CDBAE7F9AECEC988C8B","parts":{"total":1,"hash":"041BDB7BB55C5EF32606A01F7A823B0F8E91C4D526F8904F1A9AEF67BF3064C3"}},"signatures":[{"block_id_flag":2,"validator_address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","timestamp":"2026-07-02T13:42:21.978100665Z","signature":"ArO1WcKzkIG6mwWjOV6i2snoogZ3n/v/Q4Y2NOSr8TzSOEWE8g6myIegFvypGDwIKKTNnBMsm9kwvnTuccZAAQ=="},{"block_id_flag":2,"validator_address":"47601B18F0F434375F7219AC5297E156459D2A8C","timestamp":"2026-07-02T13:42:21.998424331Z","signature":"AiMJPZdzSY1h54xK6RgnvfXOMwSfWy85UQ7Y1IEXm/o/QhjcHTlNW0dGDrqDmzBEbM+5E75i26DrLAnHxQOfCw=="},{"block_id_flag":2,"validator_address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","timestamp":"2026-07-02T13:42:22.032434073Z","signature":"7gZHaPBLcwOBHWa4RbIrwBE4ihdrbsk7oRPOh+iqY6/3ps9BMdQcE8BFh91uyo3FoQ8SS4AoNF4pbCkoyaJjDg=="},{"block_id_flag":2,"validator_address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","timestamp":"2026-07-02T13:42:21.993747794Z","signature":"NsEXFMl6C/Lr+viExXqfduwWJakZQh53iAH8vcERpg+gz3vepPpQzOnMqfZEkJl/sO/La6FOb7eV0DwrDjPTBA=="},{"block_id_flag":2,"validator_address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","timestamp":"2026-07-02T13:42:21.994973721Z","signature":"hwGVEBeIH09RsTqD3LL4kihiSzUUybFnNgmIH7cIlXHxA9vAIZIMSg/y4D4W3PvxeXM4nDAnLCwSjqaf3dZ7Aw=="},{"block_id_flag":2,"validator_address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","timestamp":"2026-07-02T13:42:22.006772439Z","signature":"OzP1BIHdcl1bpmjBXXa9DTRcrh1epnGkpTKcGRCQU174KaQG42qOHdc9yHBVGmBQXK35QExaQuKHuHYzrC+qBw=="},{"block_id_flag":2,"validator_address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","timestamp":"2026-07-02T13:42:21.998777566Z","signature":"Gj2NztvqyFgY2aU4YBybzczIV5/Fde9AyoNRkFsOyQX3kekbVt7en1bPKWuyGQkiftyeSZW1/SUg3KpJQ67xBg=="},{"block_id_flag":2,"validator_address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","timestamp":"2026-07-02T13:42:21.999322735Z","signature":"qkDrWV9mKPUPVhyhptG28R/d9jrF/dJiaxE5bkBjz3e5HpaCS+aLVVdGXtqz3Fg+MBcz64pW+zBwc3uzkthMAQ=="},{"block_id_flag":2,"validator_address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","timestamp":"2026-07-02T13:42:21.99187637Z","signature":"iVpE1pDEuPe6WYFgQJW56cn1XeT4/UnWVHrlCEhZ0eSbBBNlnETjffRD2z6A+xlEuiAcLZcWFOm6Wae6yvD/Cg=="},{"block_id_flag":2,"validator_address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","timestamp":"2026-07-02T13:42:21.990447937Z","signature":"NRRUCm9u0sY3gtgBK6k4+6vMr3scJ4OqVzREKY0oJHjwSOALHSEKPObmKPkq/AeQRGbJsaveMc0EIStY40QjAQ=="},{"block_id_flag":2,"validator_address":"1354CE3615325D1820E451ED8AE09A057BB22753","timestamp":"2026-07-02T13:42:21.977962776Z","signature":"e1h2ARu254Kc3pLWMXkb5WCAVA4kdKXhukVLtBsJpWS2FGqlZP4o7GzUJlFsxe+vgnqu0k+kLzvOFkySEA8EDg=="},{"block_id_flag":2,"validator_address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","timestamp":"2026-07-02T13:42:21.984915752Z","signature":"KxE4qgeqrKK/ONarnkZVWWQjsuvqZJsE1KPcUjyfehvJdLBydhMPSpHqSHA2LqmDYZzId/np2+NwUAkoUH82BQ=="},{"block_id_flag":2,"validator_address":"519AD7739408413E80010AECFDF1B509A580D0C0","timestamp":"2026-07-02T13:42:21.976441296Z","signature":"UYL3WT9gM6PzutjugEUjjlLj4FgHbK1BUIvZ5BnlHgsQXp0k2jNceaMZxlqQmtacG5oZNfXMYoGuU9vTSQiACw=="},{"block_id_flag":2,"validator_address":"F15247741FAFBF85DB50C741E21E824D6D90059E","timestamp":"2026-07-02T13:42:27.702978552Z","signature":"olsRT+gyPzs4/0VWKJf70WsRvuHopAvuPogizYJ29p5ikAV6fFfTFCp8SX1ra4/zj79wI3qpLfFYE3/WV7PZAQ=="},{"block_id_flag":2,"validator_address":"3363E8F97B02ECC00289E72173D827543047ACDA","timestamp":"2026-07-02T13:42:22.048806264Z","signature":"Ps4J2qznxOQKgq3Z0bpn4SH/ivooQqAqexkCETwsJKSbLUyDjn1iaLxAVb5K0gTC+Fl7mxR6DSdJlxmx+dmfAg=="},{"block_id_flag":2,"validator_address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","timestamp":"2026-07-02T13:42:21.984788162Z","signature":"3MG+NWoligZfkxLgG8tFi2QjXIix+oywBxarCHFwGnGksvjSbMOY9GiZorKsnllAhUv7fupKlfFcK7Nu3A8HCg=="}]}},"canonical":true}}"#).unwrap();

        // validators at height 24499898
        let next = validators::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"block_height":"24499898","validators":[{"address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","pub_key":{"type":"tendermint/PubKeyEd25519","value":"fWg+2+R4FRJAEOAdZJnxav5Nt1ckULfUYxwSor/WVzg="},"voting_power":"1799415","proposer_priority":"-8836954"},{"address":"47601B18F0F434375F7219AC5297E156459D2A8C","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/6i7POj/PPpoAeCnYAliUopKxPW+fx+YVt9iocB+N7E="},"voting_power":"1798054","proposer_priority":"-1195599"},{"address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","pub_key":{"type":"tendermint/PubKeyEd25519","value":"A3qll5g0IyGUft3ePAdiRWcFIMwBJR5mfLTCIOmpKzY="},"voting_power":"1798054","proposer_priority":"1634884"},{"address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"WVEQK6+phIZfZNVCtyNbeB1deIyGQuUDEwRdJQCeJqs="},"voting_power":"1798053","proposer_priority":"10408204"},{"address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","pub_key":{"type":"tendermint/PubKeyEd25519","value":"W2ek87VdjheHyYIsDbLwcY0ElBjKQDZ7QBXxcLfgtXE="},"voting_power":"1798052","proposer_priority":"-7955845"},{"address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"3QVGlX6R4tv9jO7u2gyU4fi8IM5V8FLyggN4tckct3I="},"voting_power":"1798048","proposer_priority":"-4748269"},{"address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"00qwGl9hr3K3Bv0z9FFCfxfDNbwwYrPKLzrC4Hj+atM="},"voting_power":"1798046","proposer_priority":"9308304"},{"address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"1qvOrXd0UxQBCPewUYch5SfOmpW7l0N/WCh9Pa2Fv1s="},"voting_power":"1798041","proposer_priority":"11464"},{"address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","pub_key":{"type":"tendermint/PubKeyEd25519","value":"Qerh8g8MKIv1Y+tP4iNofYOGA89fdgNJJLX77FJC/GU="},"voting_power":"1798029","proposer_priority":"75287"},{"address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","pub_key":{"type":"tendermint/PubKeyEd25519","value":"ZDsrSs5naDnL0ZXibIN5WP+C/cqsPd8chk2QIHi3D6c="},"voting_power":"1788057","proposer_priority":"11366891"},{"address":"1354CE3615325D1820E451ED8AE09A057BB22753","pub_key":{"type":"tendermint/PubKeyEd25519","value":"udITsIl01Vog3jm6cZjK9vlUetFf3xd8OVck5MJZwZs="},"voting_power":"1556809","proposer_priority":"-7595100"},{"address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"n6XDWPG7i9pZNoMEbmLxsOMggu8sBfI+ChM24W6dxq4="},"voting_power":"1513945","proposer_priority":"11798340"},{"address":"519AD7739408413E80010AECFDF1B509A580D0C0","pub_key":{"type":"tendermint/PubKeyEd25519","value":"hzrYFXAbynbIpJs8OGf5PAt/vH9GI2lbRyiRtYo46SI="},"voting_power":"1467666","proposer_priority":"12366767"},{"address":"F15247741FAFBF85DB50C741E21E824D6D90059E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/O3LQ8ipc7OO8vwVrivviE3+H8HxfbeKyUcjACznpew="},"voting_power":"1424749","proposer_priority":"-9415034"},{"address":"3363E8F97B02ECC00289E72173D827543047ACDA","pub_key":{"type":"tendermint/PubKeyEd25519","value":"mPnu910hOOa1tAQ7pbOLFDxvllbQUmrbtGjqQrYg1nM="},"voting_power":"1140040","proposer_priority":"-8422364"},{"address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"HIYOoPElWdXDpddcgiI2+ipY++J4DDoqyAtThP44m84="},"voting_power":"759540","proposer_priority":"-8800968"}],"count":"16","total":"16"}}"#).unwrap();
        (commit, next)
    }

    // skip target 24499906 with its own (24499906) and next-height (24499907) validator sets
    fn skip_fixtures() -> (commit::Response, validators::Response, validators::Response) {
        // commit response at height 24499906
        let commit = commit::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"signed_header":{"header":{"version":{"block":"11"},"chain_id":"nyx","height":"24499906","time":"2026-07-02T13:43:07.309667988Z","last_block_id":{"hash":"257EAE923A9611D07E7EC3819F905DDD152118003DE0CBDFCD40860B1EA100B1","parts":{"total":1,"hash":"D8103B6005B7F8D0BB64463C0E9D28DA55DC794BF0C00CA7F1CCC9DA40F17C66"}},"last_commit_hash":"84774BAB81B70CA7E0797D6D3B618BE9C8ED09F777F9DE2C7C7C174407437B00","data_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","next_validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","consensus_hash":"048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F","app_hash":"C10836704A752BED417375A47DBF4A68E64A4271ACDA6F393F2FB0DF17E4B76C","last_results_hash":"EB044CE2748D32999260C6A24F61BC6D602DD8B2F581530E39C1CD2B265DC48E","evidence_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","proposer_address":"D72D363E94A7C20E7A3A274F1A074E577F04432A"},"commit":{"height":"24499906","round":0,"block_id":{"hash":"F614D1799B7C0BA3F586F67BB14BC0DF624E6794A492026F4E4F5FD71BAFD38B","parts":{"total":1,"hash":"C7A639E3878126EB1163046E41C9A728211507C6AE8ADCA954BF3DDA4048032C"}},"signatures":[{"block_id_flag":2,"validator_address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","timestamp":"2026-07-02T13:43:12.950940834Z","signature":"3ZDaZtme2yQ+tjQYk03Uv+iLGf3tQvzlAxnAOL93ZoQcRX2K9Ac6zCeiaq2pFeXW24abBme4kWPeqnynzLdYDg=="},{"block_id_flag":2,"validator_address":"47601B18F0F434375F7219AC5297E156459D2A8C","timestamp":"2026-07-02T13:43:12.96063193Z","signature":"oi9u3+enZJuWvlyh/m5Ob9hbJgkLPI8wT8Lppr7qYgYlCMhHGf0bqXPGKzA6tXY7JEr+qE7IkI546n5w86fJCw=="},{"block_id_flag":2,"validator_address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","timestamp":"2026-07-02T13:43:13.001591763Z","signature":"Pq08S+0ti2DcNauzO+b7vRnVek2AWPStvUjBk2JrxUPOKMs1ZBvpTVTxPEplmh4eb5R+C9IQs8mOg41Pl2SYBA=="},{"block_id_flag":2,"validator_address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","timestamp":"2026-07-02T13:43:12.966139369Z","signature":"hi/capuyNKMaRqFDBlagslay3fokkVbEpnYSD1chLkcCLeRe6BQjGdOnN4RaGGwuLL72s1EttjzYZdJSISW9AQ=="},{"block_id_flag":2,"validator_address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","timestamp":"2026-07-02T13:43:12.969374205Z","signature":"AfCFuJnGS4OvWnXg6cuYvkV3JTg1YNZp737dqGhVCmt+TM4QX7GOZ8RqgXp7ixWB0r6/5AqHS95kRV0U9lrkDQ=="},{"block_id_flag":2,"validator_address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","timestamp":"2026-07-02T13:43:12.968599748Z","signature":"rMkSWee1jh9QBDeyINIY0BXuacn0/NGlAZuOF8mCo2NTv2xzPE+rakBhthq+2b6XdAy3SGIfLPdNDoQsR/vNBA=="},{"block_id_flag":2,"validator_address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","timestamp":"2026-07-02T13:43:12.899661903Z","signature":"sEdnvqtzVq0HPe60w5R3iUswEDIljhJSptQ0IceyBOV6dL2DrVVVEN9N/DgF3IhY5hUXUGmg/gU9Y+mYb/BDBA=="},{"block_id_flag":2,"validator_address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","timestamp":"2026-07-02T13:43:12.958904119Z","signature":"RKQHhcws8X8ZBCSZ2OsxDWQzkKgq2x6Or8HbU0VIyvBNZN38b6/rkfx1yhNrRZ6AgloLi+9A8PsCi614wH7TBQ=="},{"block_id_flag":2,"validator_address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","timestamp":"2026-07-02T13:43:12.968635064Z","signature":"sKWljqmYoHmW9e7tMESsgk/NjZbBYd9RbPNocQjEhUT4amSJ/4gELyqbPSNeAqTnH1IdIAzpE1vJ2NBoCD3ZAQ=="},{"block_id_flag":2,"validator_address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","timestamp":"2026-07-02T13:43:12.967515421Z","signature":"gs2KV6Qi27W+qkXPfrsPTP8RA5b1G7AtRkful8em8w7PLOc6QuPd25SWk5Bb+RDpZJ0z/aA1k6/vuTxoEVDdDA=="},{"block_id_flag":2,"validator_address":"1354CE3615325D1820E451ED8AE09A057BB22753","timestamp":"2026-07-02T13:43:12.959629171Z","signature":"ImhDKkT8GTUHAtnW7qKehU9KzRVQh/AYKJMgPDW5kyOIH2avhVlPyOSz+r+HND8RERKBCgb3V+uFsOe0mGcMBg=="},{"block_id_flag":2,"validator_address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","timestamp":"2026-07-02T13:43:12.955609942Z","signature":"cQD+QhXUl+/F3rrJFpl2N5hJ0Xtt30DkopQva4FupwNvYGtI28ymOZKiubnnrE1MJVLZUSS5mwhPbltYpmv9CA=="},{"block_id_flag":2,"validator_address":"519AD7739408413E80010AECFDF1B509A580D0C0","timestamp":"2026-07-02T13:43:12.929281055Z","signature":"hgNwz8ZgsKNkJa/qDLIyIcajqBT6jxXioExqaAphqVoLz9AsqrRD1dfPbDWGvsVdTbBhsfW20wNJfbOUiL1mCg=="},{"block_id_flag":2,"validator_address":"F15247741FAFBF85DB50C741E21E824D6D90059E","timestamp":"2026-07-02T13:43:18.683809456Z","signature":"DSr/NwLVvMdnRrp4KCXJZLxz/nKD2RvVUNyGcs+CBEPaXQKFAJpF7yZ1YxFJJF23EN5eGPr4HIz9eLXSD5BxBw=="},{"block_id_flag":2,"validator_address":"3363E8F97B02ECC00289E72173D827543047ACDA","timestamp":"2026-07-02T13:43:12.962684789Z","signature":"ahdJx8+ZQk/uktEUCFQJ3qODy4+/mb8cdmlM7M8ZCwS2u6INPrYEvScHeepkxiSG2avOWu5i1HiJpwLmrcIoDA=="},{"block_id_flag":2,"validator_address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","timestamp":"2026-07-02T13:43:12.960181022Z","signature":"TCu6DIRfohg++T8BTZPtKjGGISFyaTVbCPNeqFovhiHsIBm3ov8vN78yB5SzfoNsEfpu5nu3x7uvGJ2L9VtHBg=="}]}},"canonical":true}}"#).unwrap();

        // validators response at height 24499906
        let validators = validators::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"block_height":"24499906","validators":[{"address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","pub_key":{"type":"tendermint/PubKeyEd25519","value":"fWg+2+R4FRJAEOAdZJnxav5Nt1ckULfUYxwSor/WVzg="},"voting_power":"1799415","proposer_priority":"5558366"},{"address":"47601B18F0F434375F7219AC5297E156459D2A8C","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/6i7POj/PPpoAeCnYAliUopKxPW+fx+YVt9iocB+N7E="},"voting_power":"1798054","proposer_priority":"13188833"},{"address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","pub_key":{"type":"tendermint/PubKeyEd25519","value":"A3qll5g0IyGUft3ePAdiRWcFIMwBJR5mfLTCIOmpKzY="},"voting_power":"1798054","proposer_priority":"-9815282"},{"address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"WVEQK6+phIZfZNVCtyNbeB1deIyGQuUDEwRdJQCeJqs="},"voting_power":"1798053","proposer_priority":"-1041970"},{"address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","pub_key":{"type":"tendermint/PubKeyEd25519","value":"W2ek87VdjheHyYIsDbLwcY0ElBjKQDZ7QBXxcLfgtXE="},"voting_power":"1798052","proposer_priority":"6428571"},{"address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"3QVGlX6R4tv9jO7u2gyU4fi8IM5V8FLyggN4tckct3I="},"voting_power":"1798048","proposer_priority":"9636115"},{"address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"00qwGl9hr3K3Bv0z9FFCfxfDNbwwYrPKLzrC4Hj+atM="},"voting_power":"1798046","proposer_priority":"-2141926"},{"address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"1qvOrXd0UxQBCPewUYch5SfOmpW7l0N/WCh9Pa2Fv1s="},"voting_power":"1798041","proposer_priority":"-11438806"},{"address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","pub_key":{"type":"tendermint/PubKeyEd25519","value":"Qerh8g8MKIv1Y+tP4iNofYOGA89fdgNJJLX77FJC/GU="},"voting_power":"1798029","proposer_priority":"-11375079"},{"address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","pub_key":{"type":"tendermint/PubKeyEd25519","value":"ZDsrSs5naDnL0ZXibIN5WP+C/cqsPd8chk2QIHi3D6c="},"voting_power":"1788057","proposer_priority":"-163251"},{"address":"1354CE3615325D1820E451ED8AE09A057BB22753","pub_key":{"type":"tendermint/PubKeyEd25519","value":"udITsIl01Vog3jm6cZjK9vlUetFf3xd8OVck5MJZwZs="},"voting_power":"1556809","proposer_priority":"4859372"},{"address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"n6XDWPG7i9pZNoMEbmLxsOMggu8sBfI+ChM24W6dxq4="},"voting_power":"1513945","proposer_priority":"-1924698"},{"address":"519AD7739408413E80010AECFDF1B509A580D0C0","pub_key":{"type":"tendermint/PubKeyEd25519","value":"hzrYFXAbynbIpJs8OGf5PAt/vH9GI2lbRyiRtYo46SI="},"voting_power":"1467666","proposer_priority":"-1726503"},{"address":"F15247741FAFBF85DB50C741E21E824D6D90059E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/O3LQ8ipc7OO8vwVrivviE3+H8HxfbeKyUcjACznpew="},"voting_power":"1424749","proposer_priority":"1982958"},{"address":"3363E8F97B02ECC00289E72173D827543047ACDA","pub_key":{"type":"tendermint/PubKeyEd25519","value":"mPnu910hOOa1tAQ7pbOLFDxvllbQUmrbtGjqQrYg1nM="},"voting_power":"1140040","proposer_priority":"697956"},{"address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"HIYOoPElWdXDpddcgiI2+ipY++J4DDoqyAtThP44m84="},"voting_power":"759540","proposer_priority":"-2724648"}],"count":"16","total":"16"}}"#).unwrap();

        // validators response at height 24499907
        let next_validators = validators::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"block_height":"24499907","validators":[{"address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","pub_key":{"type":"tendermint/PubKeyEd25519","value":"fWg+2+R4FRJAEOAdZJnxav5Nt1ckULfUYxwSor/WVzg="},"voting_power":"1799415","proposer_priority":"7357781"},{"address":"47601B18F0F434375F7219AC5297E156459D2A8C","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/6i7POj/PPpoAeCnYAliUopKxPW+fx+YVt9iocB+N7E="},"voting_power":"1798054","proposer_priority":"-10847711"},{"address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","pub_key":{"type":"tendermint/PubKeyEd25519","value":"A3qll5g0IyGUft3ePAdiRWcFIMwBJR5mfLTCIOmpKzY="},"voting_power":"1798054","proposer_priority":"-8017228"},{"address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"WVEQK6+phIZfZNVCtyNbeB1deIyGQuUDEwRdJQCeJqs="},"voting_power":"1798053","proposer_priority":"756083"},{"address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","pub_key":{"type":"tendermint/PubKeyEd25519","value":"W2ek87VdjheHyYIsDbLwcY0ElBjKQDZ7QBXxcLfgtXE="},"voting_power":"1798052","proposer_priority":"8226623"},{"address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"3QVGlX6R4tv9jO7u2gyU4fi8IM5V8FLyggN4tckct3I="},"voting_power":"1798048","proposer_priority":"11434163"},{"address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"00qwGl9hr3K3Bv0z9FFCfxfDNbwwYrPKLzrC4Hj+atM="},"voting_power":"1798046","proposer_priority":"-343880"},{"address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"1qvOrXd0UxQBCPewUYch5SfOmpW7l0N/WCh9Pa2Fv1s="},"voting_power":"1798041","proposer_priority":"-9640765"},{"address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","pub_key":{"type":"tendermint/PubKeyEd25519","value":"Qerh8g8MKIv1Y+tP4iNofYOGA89fdgNJJLX77FJC/GU="},"voting_power":"1798029","proposer_priority":"-9577050"},{"address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","pub_key":{"type":"tendermint/PubKeyEd25519","value":"ZDsrSs5naDnL0ZXibIN5WP+C/cqsPd8chk2QIHi3D6c="},"voting_power":"1788057","proposer_priority":"1624806"},{"address":"1354CE3615325D1820E451ED8AE09A057BB22753","pub_key":{"type":"tendermint/PubKeyEd25519","value":"udITsIl01Vog3jm6cZjK9vlUetFf3xd8OVck5MJZwZs="},"voting_power":"1556809","proposer_priority":"6416181"},{"address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"n6XDWPG7i9pZNoMEbmLxsOMggu8sBfI+ChM24W6dxq4="},"voting_power":"1513945","proposer_priority":"-410753"},{"address":"519AD7739408413E80010AECFDF1B509A580D0C0","pub_key":{"type":"tendermint/PubKeyEd25519","value":"hzrYFXAbynbIpJs8OGf5PAt/vH9GI2lbRyiRtYo46SI="},"voting_power":"1467666","proposer_priority":"-258837"},{"address":"F15247741FAFBF85DB50C741E21E824D6D90059E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/O3LQ8ipc7OO8vwVrivviE3+H8HxfbeKyUcjACznpew="},"voting_power":"1424749","proposer_priority":"3407707"},{"address":"3363E8F97B02ECC00289E72173D827543047ACDA","pub_key":{"type":"tendermint/PubKeyEd25519","value":"mPnu910hOOa1tAQ7pbOLFDxvllbQUmrbtGjqQrYg1nM="},"voting_power":"1140040","proposer_priority":"1837996"},{"address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"HIYOoPElWdXDpddcgiI2+ipY++J4DDoqyAtThP44m84="},"voting_power":"759540","proposer_priority":"-1965108"}],"count":"16","total":"16"}}"#).unwrap();

        (commit, validators, next_validators)
    }

    // --- helpers ---

    fn test_options(trusting_period: Duration) -> Options {
        Options {
            trust_threshold: TrustThreshold::ONE_THIRD,
            trusting_period,
            clock_drift: Duration::from_secs(5),
        }
    }

    // a fabricated, non-signing validator used only to inflate the trusted set's total voting
    // power. Its signature is never verified because it never appears in any commit; only its
    // voting power (added to the denominator) matters.
    fn fake_validator(seed: u8, power: u64) -> Info {
        let pk = PublicKey::from_raw_ed25519(&[seed; 32]).unwrap();
        Info::new(pk, vote::Power::try_from(power).unwrap())
    }

    // a checkpoint whose `next_validators` is padded with a dominant fake validator. For any
    // SKIP hop the overlap is computed against this set (trusted blindly - never hash-checked),
    // so real/(real + fake) drops below 1/3 and the hop returns `NotEnoughTrust`, forcing
    // bisection. The real `signed_header` (and thus `next_validators_hash`) is untouched, so the
    // adjacent hop - which checks the untrusted header's `validators_hash` against the real
    // `next_validators_hash` - still verifies.
    fn tampered_checkpoint() -> Checkpoint {
        let (commit, validators, next_validators) = checkpoint_fixtures();
        let mut padded = next_validators.validators;
        // the real set totals ~25M; a single 100M fake pushes real/(real + fake) well below 1/3
        padded.push(fake_validator(1, 100_000_000));
        Checkpoint {
            height: Height::from(CHECKPOINT),
            signed_header: commit.signed_header,
            validators: ValidatorSet::without_proposer(validators.validators),
            next_validators: ValidatorSet::without_proposer(padded),
        }
    }

    /// A mock RPC serving every fixture (checkpoint, adjacent block, skip block).
    fn full_mock() -> MockRpcClient {
        let (c896, v896, v897) = checkpoint_fixtures();
        let (c897, v898) = adjacent_fixtures();
        let (c906, v906, v907) = skip_fixtures();

        let mut mock = MockRpcClient::default();
        mock.with_commit_response(24499896u32, Ok(c896))
            .with_commit_response(24499897u32, Ok(c897))
            .with_commit_response(24499906u32, Ok(c906))
            .with_validators_response(24499896u32, Paging::All, Ok(v896))
            .with_validators_response(24499897u32, Paging::All, Ok(v897))
            .with_validators_response(24499898u32, Paging::All, Ok(v898))
            .with_validators_response(24499906u32, Paging::All, Ok(v906))
            .with_validators_response(24499907u32, Paging::All, Ok(v907));
        mock
    }

    fn build_anchor(client: MockRpcClient, options: Options) -> LightClientAnchor<MockRpcClient> {
        LightClientAnchor::new(client, mock_contract(0), checkpoint(), options)
    }

    // bisection support: the skip target 24499898 (whose direct hop from the tampered checkpoint
    // fails the overlap check) and its next-height (24499899) validator set. These two are the
    // only fixtures missing from the set above.
    fn bisection_fixtures() -> (commit::Response, validators::Response) {
        // commit response at height 24499898
        let commit = commit::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"signed_header":{"header":{"version":{"block":"11"},"chain_id":"nyx","height":"24499898","time":"2026-07-02T13:42:21.994973721Z","last_block_id":{"hash":"4FD17927DA1EC2F1F1A0076683D11FB93916486C64CE6CDBAE7F9AECEC988C8B","parts":{"total":1,"hash":"041BDB7BB55C5EF32606A01F7A823B0F8E91C4D526F8904F1A9AEF67BF3064C3"}},"last_commit_hash":"5B1EFF800594E213933D24803E3C601417CC7225B2EB4AE79E9F81369A32261D","data_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","next_validators_hash":"3C8E7E6CB54A4A6FF5D81247F50D4582F43208D534D60AACE4C91B961778A853","consensus_hash":"048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F","app_hash":"5B7062490F0489795076EF6C8DF0258EEF7812441685C4A38A25AC5CFACA3755","last_results_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","evidence_hash":"E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855","proposer_address":"F15247741FAFBF85DB50C741E21E824D6D90059E"},"commit":{"height":"24499898","round":0,"block_id":{"hash":"8A64E6BFE714C8D71E6DDAD9FCD75CE1C093DD4D1908F19F9346C31853C9123F","parts":{"total":1,"hash":"5100B9F5C78545757866D13FD604660B78957EA03FE5C8AE9FB9AFCFC2324874"}},"signatures":[{"block_id_flag":2,"validator_address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","timestamp":"2026-07-02T13:42:27.648280649Z","signature":"68xtFW/7eGG7Kkj7R49n9nki5h1/r3cMBy3G7nXsW27vwxtRJJQBTMYUXtKhJasqiIQY9xz3IOlXp9AayZW+Bw=="},{"block_id_flag":2,"validator_address":"47601B18F0F434375F7219AC5297E156459D2A8C","timestamp":"2026-07-02T13:42:27.701100968Z","signature":"JICjR+kiODDMxnOQnPkkEHENEFPmGinthfT1fmzaUipoNmYf+46W/QCiBmQ7jrYHTTyrIEhUg/bGWWgGVq8QCA=="},{"block_id_flag":2,"validator_address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","timestamp":"2026-07-02T13:42:27.745537811Z","signature":"Z/Whnw1VgGV742WeJvPqlJmO+goLZuzuQGRrlOfXkW51GfWW+Bg4udbUZBTAvPBAIOeZ0PguzxstXDvSWibPAA=="},{"block_id_flag":2,"validator_address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","timestamp":"2026-07-02T13:42:27.712880745Z","signature":"Sf4+pmXrrapSIxr43+sDJHhlSkfQyW42sezYw206Bm3yKBHjAN7N5dyV0jihCPsjjQbIopvIQkKOGOAfAA6pDQ=="},{"block_id_flag":2,"validator_address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","timestamp":"2026-07-02T13:42:27.717160215Z","signature":"96dj3va9pECHZPpoRdQ9Q2ih6aniBET0x/jtXLoatWsNxZOHGRZnopdW+CeYoNK/4sGsUNNALFh87wH1VmSGDg=="},{"block_id_flag":2,"validator_address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","timestamp":"2026-07-02T13:42:27.711153061Z","signature":"UryAO2kPnq0F00WM8w9twL4cMXzOryInOlzV05jY8eZWukDd4Tr9C8TyBCtUSrVEhTh6L4LfG6oSAL5w8lq2DA=="},{"block_id_flag":2,"validator_address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","timestamp":"2026-07-02T13:42:27.722303202Z","signature":"yJQDC5EooKa2cTO+8NM7V51iICkcGNd2Pqw+c1MFLx7NuExnmceG0rXbkPfNie8n3Stmh2XWYsrCLV0tTbIjCg=="},{"block_id_flag":2,"validator_address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","timestamp":"2026-07-02T13:42:27.708597406Z","signature":"vU+bSoeOjv8HAbNyGUQysSpYlhbnHiYlREvF38hAv/xph3khor2XJMK6raNIXTurU/5rPf37tGn647WrWwsKDQ=="},{"block_id_flag":2,"validator_address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","timestamp":"2026-07-02T13:42:27.673800385Z","signature":"vRD5D8sBEDPDxCRiFKppNW8M5EEInnDTfwNdcgSrDxnPONSU5W41ti/L29VwHi1gyJwQxtRL4mmI92yTr1cZDA=="},{"block_id_flag":2,"validator_address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","timestamp":"2026-07-02T13:42:27.711567881Z","signature":"qqSpzdntIea330YBgPApjll6eL3dCiFtT51eHjP5eGkSryp8v4ryK7SYjoUgs8/6XuUzapHE27bdFWTFwrrsAw=="},{"block_id_flag":2,"validator_address":"1354CE3615325D1820E451ED8AE09A057BB22753","timestamp":"2026-07-02T13:42:27.699368355Z","signature":"2JI215z5mBcsRki483zIazPV5AbSEG3w4i62PWRgS4eMAM682PUY8mgvk1AqDmHO2j0Suvpn1Ex0hcSScOiqCA=="},{"block_id_flag":2,"validator_address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","timestamp":"2026-07-02T13:42:27.707288357Z","signature":"7tsFW5qWQw43RyoUO+2GwVk9KD3KC2lTiOgrOnEyHAhXhVx9savYpjy0Zq8buGxR6aCsc9QbLsCHCqK8b12JCQ=="},{"block_id_flag":2,"validator_address":"519AD7739408413E80010AECFDF1B509A580D0C0","timestamp":"2026-07-02T13:42:27.63146239Z","signature":"w6Fz0WQw+MAvdwj0v9BVVzjFQeyi0q6Un10n4dvmkzw+MTzcaMjzcnPA0iHHMORviBQIcJ54dOq/Qifm4zZwDQ=="},{"block_id_flag":2,"validator_address":"F15247741FAFBF85DB50C741E21E824D6D90059E","timestamp":"2026-07-02T13:42:33.430748285Z","signature":"XDt91v5cYqj3oBXGNqujLd515XE9AVylLVW3k3E5z0ZoCDh+e/8HHXVc3ej0MzN0YCX8pyvT7XyqkL5pgRx3AQ=="},{"block_id_flag":2,"validator_address":"3363E8F97B02ECC00289E72173D827543047ACDA","timestamp":"2026-07-02T13:42:27.769696182Z","signature":"Z28GmRw7+Ll23KaQg44WNXnEi7JNvqe2LhUhcoTvvJxCyp8JihyNJxyXRxFSsYETdOw71r63A/D2DU4TDuDRCQ=="},{"block_id_flag":2,"validator_address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","timestamp":"2026-07-02T13:42:27.706618261Z","signature":"xkc9Qs0j6tTFWuk4SNepEtXesrdNGwYllbbKlqYMOhj7sPrDk24SL7BNjJNxdNN7vtiqSr2kJnddVA0ABgWEDQ=="}]}},"canonical":true}}"#).unwrap();

        // validators response for height 24499899
        let next = validators::Response::from_string(r#"{"jsonrpc":"2.0","id":-1,"result":{"block_height":"24499899","validators":[{"address":"9A5783B0CB39B4AE670E0F9215D3C720B56506D1","pub_key":{"type":"tendermint/PubKeyEd25519","value":"fWg+2+R4FRJAEOAdZJnxav5Nt1ckULfUYxwSor/WVzg="},"voting_power":"1799415","proposer_priority":"-7037539"},{"address":"47601B18F0F434375F7219AC5297E156459D2A8C","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/6i7POj/PPpoAeCnYAliUopKxPW+fx+YVt9iocB+N7E="},"voting_power":"1798054","proposer_priority":"602455"},{"address":"4E4A4575F97EDCE249812A7AD125414AFCD86933","pub_key":{"type":"tendermint/PubKeyEd25519","value":"A3qll5g0IyGUft3ePAdiRWcFIMwBJR5mfLTCIOmpKzY="},"voting_power":"1798054","proposer_priority":"3432938"},{"address":"73837BE389D82E7881B504A43F40ADF4855E3B4D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"WVEQK6+phIZfZNVCtyNbeB1deIyGQuUDEwRdJQCeJqs="},"voting_power":"1798053","proposer_priority":"12206257"},{"address":"A43138580D4EF4571A6E4A5C0CDEC3243EAA7276","pub_key":{"type":"tendermint/PubKeyEd25519","value":"W2ek87VdjheHyYIsDbLwcY0ElBjKQDZ7QBXxcLfgtXE="},"voting_power":"1798052","proposer_priority":"-6157793"},{"address":"1DB464D43981AA325BC0CE4ACA3EB12EAC076A5D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"3QVGlX6R4tv9jO7u2gyU4fi8IM5V8FLyggN4tckct3I="},"voting_power":"1798048","proposer_priority":"-2950221"},{"address":"AA71546DB40A211CDB8B78D8DEB6F750A611336D","pub_key":{"type":"tendermint/PubKeyEd25519","value":"00qwGl9hr3K3Bv0z9FFCfxfDNbwwYrPKLzrC4Hj+atM="},"voting_power":"1798046","proposer_priority":"11106350"},{"address":"D72D363E94A7C20E7A3A274F1A074E577F04432A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"1qvOrXd0UxQBCPewUYch5SfOmpW7l0N/WCh9Pa2Fv1s="},"voting_power":"1798041","proposer_priority":"1809505"},{"address":"6EF6EE46207C59ACA4CDD011FD00A0D8F4172BED","pub_key":{"type":"tendermint/PubKeyEd25519","value":"Qerh8g8MKIv1Y+tP4iNofYOGA89fdgNJJLX77FJC/GU="},"voting_power":"1798029","proposer_priority":"1873316"},{"address":"24CF61027DF3E26A774EBD6A527DDE7F28D1CB32","pub_key":{"type":"tendermint/PubKeyEd25519","value":"ZDsrSs5naDnL0ZXibIN5WP+C/cqsPd8chk2QIHi3D6c="},"voting_power":"1788057","proposer_priority":"13154948"},{"address":"1354CE3615325D1820E451ED8AE09A057BB22753","pub_key":{"type":"tendermint/PubKeyEd25519","value":"udITsIl01Vog3jm6cZjK9vlUetFf3xd8OVck5MJZwZs="},"voting_power":"1556809","proposer_priority":"-6038291"},{"address":"D5CFFB5F5F7647A983FBEB4089891AC7402CB43A","pub_key":{"type":"tendermint/PubKeyEd25519","value":"n6XDWPG7i9pZNoMEbmLxsOMggu8sBfI+ChM24W6dxq4="},"voting_power":"1513945","proposer_priority":"13312285"},{"address":"519AD7739408413E80010AECFDF1B509A580D0C0","pub_key":{"type":"tendermint/PubKeyEd25519","value":"hzrYFXAbynbIpJs8OGf5PAt/vH9GI2lbRyiRtYo46SI="},"voting_power":"1467666","proposer_priority":"-12000165"},{"address":"F15247741FAFBF85DB50C741E21E824D6D90059E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"/O3LQ8ipc7OO8vwVrivviE3+H8HxfbeKyUcjACznpew="},"voting_power":"1424749","proposer_priority":"-7990285"},{"address":"3363E8F97B02ECC00289E72173D827543047ACDA","pub_key":{"type":"tendermint/PubKeyEd25519","value":"mPnu910hOOa1tAQ7pbOLFDxvllbQUmrbtGjqQrYg1nM="},"voting_power":"1140040","proposer_priority":"-7282324"},{"address":"25219C7188D73816F8B2B7B153F83FA06A9A699E","pub_key":{"type":"tendermint/PubKeyEd25519","value":"HIYOoPElWdXDpddcgiI2+ipY++J4DDoqyAtThP44m84="},"voting_power":"759540","proposer_priority":"-8041428"}],"count":"16","total":"16"}}"#).unwrap();

        (commit, next)
    }

    // `full_mock` plus the two extra fixtures needed to walk 24499896 -> 24499898 via bisection.
    fn full_mock_with_bisection() -> MockRpcClient {
        let (c898, v899) = bisection_fixtures();
        let mut mock = full_mock();
        mock.with_commit_response(24499898u32, Ok(c898))
            .with_validators_response(24499899u32, Paging::All, Ok(v899));
        mock
    }

    #[tokio::test]
    async fn direct_verification_advances_state_and_caches_app_hash() {
        let anchor = build_anchor(full_mock(), test_options(FAR_FUTURE));
        // querying H=checkpoint verifies the adjacent header[H+1]=24499897
        let got = anchor
            .trusted_app_hash(Height::from(CHECKPOINT))
            .await
            .unwrap();
        assert_eq!(got, adjacent_fixtures().0.signed_header.header.app_hash);

        let state = anchor.state.lock().await;
        assert_eq!(state.trusted.height, Height::from(24499897u32));
        assert!(state.app_hash_cache.contains_key(&Height::from(CHECKPOINT)));
    }

    #[tokio::test]
    async fn skip_verification_resolves_in_a_single_hop() {
        let client = full_mock();
        let probe = client.clone();
        let anchor = build_anchor(client, test_options(FAR_FUTURE));
        // H=24499905 verifies header[24499906], a 10-block skip from the checkpoint
        let got = anchor
            .trusted_app_hash(Height::from(24499905u32))
            .await
            .unwrap();
        assert_eq!(got, skip_fixtures().0.signed_header.header.app_hash);
        // a single commit fetch (the target only) proves no bisection midpoints were fetched
        assert_eq!(probe.commit_calls(), vec![Height::from(24499906u32)]);
    }

    #[tokio::test]
    async fn repeated_query_is_served_from_cache() {
        let client = full_mock();
        let probe = client.clone();
        let anchor = build_anchor(client, test_options(FAR_FUTURE));

        let first = anchor
            .trusted_app_hash(Height::from(CHECKPOINT))
            .await
            .unwrap();
        let calls_after_first = probe.commit_calls().len();

        let second = anchor
            .trusted_app_hash(Height::from(CHECKPOINT))
            .await
            .unwrap();
        assert_eq!(first, second);
        // the second query hit the cache: no further RPC calls
        assert_eq!(probe.commit_calls().len(), calls_after_first);
    }

    #[tokio::test]
    async fn stale_checkpoint_fails_verification() {
        // a 1ns trusting period makes the (days-old) checkpoint immediately expired
        let anchor = build_anchor(full_mock(), test_options(Duration::from_nanos(1)));
        let err = anchor
            .trusted_app_hash(Height::from(CHECKPOINT))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::LightClientVerificationFailed(_)
        ));
    }

    // regression: a height the advancing head already passed is re-verified from the checkpoint
    // rather than returning the old spurious "not in cache after advance" error
    #[tokio::test]
    async fn below_head_query_reverifies_from_checkpoint() {
        let anchor = build_anchor(full_mock(), test_options(FAR_FUTURE));
        // advance the head to 24499906
        anchor
            .trusted_app_hash(Height::from(24499905u32))
            .await
            .unwrap();
        // now query a height below the head (the checkpoint), never cached during the skip
        let got = anchor
            .trusted_app_hash(Height::from(CHECKPOINT))
            .await
            .unwrap();
        assert_eq!(got, adjacent_fixtures().0.signed_header.header.app_hash);
    }

    // a height below the pinned checkpoint is unverifiable
    #[tokio::test]
    async fn below_checkpoint_query_is_rejected() {
        let anchor = build_anchor(full_mock(), test_options(FAR_FUTURE));
        let err = anchor
            .trusted_app_hash(Height::from(24499800u32))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DirectoryClientError::HeightBelowCheckpoint { .. }
        ));
    }

    // 6.3
    #[tokio::test]
    async fn insufficient_overlap_triggers_bisection() {
        let client = full_mock_with_bisection();
        let probe = client.clone();
        // note the TAMPERED checkpoint: its padded next_validators make every skip hop fail
        let anchor = LightClientAnchor::new(
            client,
            mock_contract(0),
            tampered_checkpoint(),
            test_options(FAR_FUTURE),
        );

        // query 24499897 -> target 24499898. The direct skip 24499896 -> 24499898 fails the
        // overlap check against the padded checkpoint set, so the anchor bisects to midpoint
        // 24499897 (adjacent, verified against the real next_validators_hash), advances there,
        // then retries 24499898 from 24499897.
        let got = anchor
            .trusted_app_hash(Height::from(24499897u32))
            .await
            .unwrap();
        assert_eq!(got, bisection_fixtures().0.signed_header.header.app_hash);

        // the call order is the bisection signature: the target was attempted (898), failed, the
        // midpoint was fetched (897), then the target was retried from the midpoint (898)
        assert_eq!(
            probe.commit_calls(),
            vec![
                Height::from(24499898u32),
                Height::from(24499897u32),
                Height::from(24499898u32),
            ]
        );
    }
}
