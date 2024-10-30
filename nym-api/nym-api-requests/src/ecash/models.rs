// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::helpers::issued_credential_plaintext;
use crate::helpers::PlaceholderJsonSchemaImpl;
use cosmrs::AccountId;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::Bytable;
use nym_credentials_interface::TicketType;
use nym_credentials_interface::{
    BlindedSignature, CompactEcashError, CredentialSpendingData, PublicKeyUser,
    VerificationKeyAuth, WithdrawalRequest,
};
use nym_crypto::asymmetric::{ed25519, identity};
use nym_ticketbooks_merkle::{IssuedTicketbook, IssuedTicketbooksFullMerkleProof};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use thiserror::Error;
use time::Date;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct VerifyEcashTicketBody {
    /// The cryptographic material required for spending the underlying credential.
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub credential: CredentialSpendingData,

    /// Cosmos address of the sender of the credential
    #[schemars(with = "String")]
    pub gateway_cosmos_addr: AccountId,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct VerifyEcashCredentialBody {
    /// The cryptographic material required for spending the underlying credential.
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub credential: CredentialSpendingData,

    /// Cosmos address of the sender of the credential
    #[schemars(with = "String")]
    pub gateway_cosmos_addr: AccountId,

    /// Multisig proposal for releasing funds for the provided bandwidth credential
    pub proposal_id: u64,
}

impl VerifyEcashCredentialBody {
    pub fn new(
        credential: CredentialSpendingData,
        gateway_cosmos_addr: AccountId,
        proposal_id: u64,
    ) -> VerifyEcashCredentialBody {
        VerifyEcashCredentialBody {
            credential,
            gateway_cosmos_addr,
            proposal_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EcashTicketVerificationResponse {
    pub verified: Result<(), EcashTicketVerificationRejection>,
}

impl EcashTicketVerificationResponse {
    pub fn reject(reason: EcashTicketVerificationRejection) -> Self {
        EcashTicketVerificationResponse {
            verified: Err(reason),
        }
    }
}

#[derive(Debug, Error, Serialize, Deserialize, JsonSchema, ToSchema)]
pub enum EcashTicketVerificationRejection {
    #[error("invalid ticket spent date. expected either today's ({today}) or yesterday's* ({yesterday}) date but got {received} instead\n*assuming it's before 1AM UTC")]
    InvalidSpentDate {
        #[schemars(with = "String")]
        #[serde(with = "crate::helpers::date_serde")]
        today: Date,
        #[schemars(with = "String")]
        #[serde(with = "crate::helpers::date_serde")]
        yesterday: Date,
        #[schemars(with = "String")]
        #[serde(with = "crate::helpers::date_serde")]
        received: Date,
    },

    #[error("this ticket has already been received before")]
    ReplayedTicket,

    #[error("this ticket has already been spent before")]
    DoubleSpend,

    #[error("failed to verify the provided ticket")]
    InvalidTicket,

    #[error(
        "the received payment contained more than a single ticket. that's currently not supported"
    )]
    MultipleTickets,
}

//  All strings are base58 encoded representations of structs
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, JsonSchema)]
pub struct BlindSignRequestBody {
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub inner_sign_request: WithdrawalRequest,

    /// the id of the associated deposit
    pub deposit_id: u32,

    /// Signature on the inner sign request and the tx hash
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signature: identity::Signature,

    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub ecash_pubkey: PublicKeyUser,

    #[schemars(with = "String")]
    #[serde(with = "crate::helpers::date_serde")]
    pub expiration_date: Date,

    #[schemars(with = "String")]
    pub ticketbook_type: TicketType,
}

impl BlindSignRequestBody {
    pub fn new(
        inner_sign_request: WithdrawalRequest,
        deposit_id: u32,
        signature: identity::Signature,
        ecash_pubkey: PublicKeyUser,
        expiration_date: Date,
        ticketbook_type: TicketType,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            inner_sign_request,
            deposit_id,
            signature,
            ecash_pubkey,
            expiration_date,
            ticketbook_type,
        }
    }

    pub fn encode_join_commitments(&self) -> Vec<u8> {
        self.inner_sign_request
            .get_private_attributes_commitments()
            .iter()
            .flat_map(|c| c.to_byte_vec())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BlindedSignatureResponse {
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub blinded_signature: BlindedSignature,
}

impl BlindedSignatureResponse {
    pub fn new(blinded_signature: BlindedSignature) -> BlindedSignatureResponse {
        BlindedSignatureResponse { blinded_signature }
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, CompactEcashError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.blinded_signature.to_bytes().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CompactEcashError> {
        Ok(BlindedSignatureResponse {
            blinded_signature: BlindedSignature::from_bytes(bytes)?,
        })
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MasterVerificationKeyResponse {
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub key: VerificationKeyAuth,
}

impl MasterVerificationKeyResponse {
    pub fn new(key: VerificationKeyAuth) -> MasterVerificationKeyResponse {
        MasterVerificationKeyResponse { key }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct VerificationKeyResponse {
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub key: VerificationKeyAuth,
}

impl VerificationKeyResponse {
    pub fn new(key: VerificationKeyAuth) -> VerificationKeyResponse {
        VerificationKeyResponse { key }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CosmosAddressResponse {
    #[schemars(with = "String")]
    pub addr: AccountId,
}

impl CosmosAddressResponse {
    pub fn new(addr: AccountId) -> CosmosAddressResponse {
        CosmosAddressResponse { addr }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PartialExpirationDateSignatureResponse {
    pub epoch_id: u64,

    #[schemars(with = "String")]
    #[serde(with = "crate::helpers::date_serde")]
    #[schema(value_type = String)]
    pub expiration_date: Date,
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signatures: Vec<AnnotatedExpirationDateSignature>,
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct PartialCoinIndicesSignatureResponse {
    pub epoch_id: u64,
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signatures: Vec<AnnotatedCoinIndexSignature>,
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct AggregatedExpirationDateSignatureResponse {
    pub epoch_id: u64,

    #[schemars(with = "String")]
    #[serde(with = "crate::helpers::date_serde")]
    pub expiration_date: Date,

    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signatures: Vec<AnnotatedExpirationDateSignature>,
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct AggregatedCoinIndicesSignatureResponse {
    pub epoch_id: u64,
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signatures: Vec<AnnotatedCoinIndexSignature>,
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct Pagination<T> {
    /// last_key is the last value returned in the previous query.
    /// it's used to indicate the start of the next (this) page.
    /// the value itself is not included in the response.
    pub last_key: Option<T>,

    /// limit is the total number of results to be returned in the result page.
    /// If left empty it will default to a value to be set by each app.
    pub limit: Option<u32>,
}

#[deprecated]
#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsRequestBody {
    /// Explicit ids of the credentials to retrieve. Note: it can't be set alongside pagination.
    pub credential_ids: Vec<i64>,

    /// Pagination settings for retrieving credentials. Note: it can't be set alongside explicit ids.
    pub pagination: Option<Pagination<i64>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, PartialEq)]
pub struct SerialNumberWrapper(
    #[serde(with = "nym_serde_helpers::bs58")]
    #[schemars(with = "String")]
    Vec<u8>,
);

impl Deref for SerialNumberWrapper {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for SerialNumberWrapper {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<Vec<u8>> for SerialNumberWrapper {
    fn from(value: Vec<u8>) -> Self {
        SerialNumberWrapper(value)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, PartialEq)]
pub struct BatchRedeemTicketsBody {
    #[serde(with = "nym_serde_helpers::bs58")]
    #[schemars(with = "String")]
    pub digest: Vec<u8>,
    pub included_serial_numbers: Vec<SerialNumberWrapper>,
    pub proposal_id: u64,
    #[schemars(with = "String")]
    pub gateway_cosmos_addr: AccountId,
}

impl BatchRedeemTicketsBody {
    pub fn make_digest<I, T>(serial_numbers: I) -> Vec<u8>
    where
        I: Iterator<Item = T>,
        T: AsRef<[u8]>,
    {
        let mut hasher = sha2::Sha256::new();
        for sn in serial_numbers {
            hasher.update(sn)
        }
        hasher.finalize().to_vec()
    }

    pub fn new(
        digest: Vec<u8>,
        proposal_id: u64,
        serial_numbers: Vec<impl Into<SerialNumberWrapper>>,
        redeemer: AccountId,
    ) -> Self {
        BatchRedeemTicketsBody {
            digest,
            included_serial_numbers: serial_numbers.into_iter().map(Into::into).collect(),
            proposal_id,
            gateway_cosmos_addr: redeemer,
        }
    }

    pub fn verify_digest(&self) -> bool {
        Self::make_digest(self.included_serial_numbers.iter()) == self.digest
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct EcashBatchTicketRedemptionResponse {
    pub proposal_accepted: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpentCredentialsResponse {
    #[serde(with = "nym_serde_helpers::base64")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub bitmap: Vec<u8>,
}

impl SpentCredentialsResponse {
    pub fn new(bitmap: Vec<u8>) -> SpentCredentialsResponse {
        SpentCredentialsResponse { bitmap }
    }
}

pub type DepositId = u32;

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommitedDeposit {
    pub deposit_id: DepositId,
    pub merkle_index: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbooksForResponseBody {
    #[schemars(with = "String")]
    #[serde(with = "crate::helpers::date_serde")]
    pub expiration_date: Date,
    pub deposits: Vec<CommitedDeposit>,
    pub merkle_root: Option<[u8; 32]>,
}

impl IssuedTicketbooksForResponseBody {
    pub fn plaintext(&self) -> Vec<u8> {
        #[allow(clippy::unwrap_used)]
        serde_json::to_vec(self).unwrap()
    }

    pub fn sign(self, key: &ed25519::PrivateKey) -> IssuedTicketbooksForResponse {
        IssuedTicketbooksForResponse {
            signature: key.sign(self.plaintext()),
            body: self,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbooksForResponse {
    pub body: IssuedTicketbooksForResponseBody,

    /// Signature on the body    
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signature: identity::Signature,
}

impl IssuedTicketbooksForResponse {
    pub fn verify_signature(&self, pub_key: &ed25519::PublicKey) -> bool {
        pub_key
            .verify(self.body.plaintext(), &self.signature)
            .is_ok()
    }
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbooksChallengeRequest {
    #[schemars(with = "String")]
    #[serde(with = "crate::helpers::date_serde")]
    pub expiration_date: Date,
    pub deposits: Vec<DepositId>,
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbooksChallengeResponseBody {
    pub partial_ticketbooks: BTreeMap<DepositId, IssuedTicketbook>,
    pub merkle_proof: IssuedTicketbooksFullMerkleProof,
}

impl IssuedTicketbooksChallengeResponseBody {
    pub fn plaintext(&self) -> Vec<u8> {
        #[allow(clippy::unwrap_used)]
        serde_json::to_vec(self).unwrap()
    }

    pub fn sign(self, key: &ed25519::PrivateKey) -> IssuedTicketbooksChallengeResponse {
        IssuedTicketbooksChallengeResponse {
            signature: key.sign(self.plaintext()),
            body: self,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbooksChallengeResponse {
    pub body: IssuedTicketbooksChallengeResponseBody,

    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signature: identity::Signature,
}

impl IssuedTicketbooksChallengeResponse {
    pub fn verify_signature(&self, pub_key: &ed25519::PublicKey) -> bool {
        pub_key
            .verify(self.body.plaintext(), &self.signature)
            .is_ok()
    }
}

#[deprecated]
#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EpochCredentialsResponse {
    pub epoch_id: u64,
    pub first_epoch_credential_id: Option<i64>,
    pub total_issued: u32,
}

#[deprecated]
#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCredentialsResponse {
    // note: BTreeMap returns ordered results, so it's fine to use it with pagination
    pub credentials: BTreeMap<i64, IssuedTicketbookBody>,
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCredentialResponse {
    pub credential: Option<IssuedTicketbookBody>,
}

#[deprecated]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, JsonSchema, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbookBody {
    pub credential: IssuedTicketbookDeprecated,
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub signature: identity::Signature,
}

#[deprecated]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbookDeprecated {
    pub id: i64,
    pub epoch_id: u32,
    pub deposit_id: u32,

    // NOTE: if we find creation of this guy takes too long,
    // change `BlindedSignature` to `BlindedSignatureBytes`
    // so that nym-api wouldn't need to parse the value out of its storage
    #[schemars(with = "PlaceholderJsonSchemaImpl")]
    pub blinded_partial_credential: BlindedSignature,
    pub encoded_private_attributes_commitments: Vec<Vec<u8>>,

    #[schemars(with = "String")]
    #[serde(with = "crate::helpers::date_serde")]
    pub expiration_date: Date,

    #[schemars(with = "String")]
    pub ticketbook_type: TicketType,
}

impl IssuedTicketbookDeprecated {
    // this method doesn't have to be reversible so just naively concatenate everything
    pub fn signable_plaintext(&self) -> Vec<u8> {
        issued_credential_plaintext(
            self.epoch_id,
            self.deposit_id,
            &self.blinded_partial_credential,
            &self.encoded_private_attributes_commitments,
            self.expiration_date,
            self.ticketbook_type,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // had some issues with `Date` and serde...
    // so might as well leave this unit test in case we do something to the helper
    #[test]
    fn aggregated_expiration_date_signature_responses_deserialises_correctly() {
        let raw = r#"{"epoch_id":0,"expiration_date":"2024-08-03","signatures":[{"signature":{"h":"9379384af5236bffd7d6c533782d74863f892cf6e0fc8b4b64647e8bd5bccfe63df639a4a556d21826ab7ef54e4adafe","s":"83124c907935eba10c791cbc2cedf8714b68c6a266efead7bc4897c3b9652680bd151493689577ac790b9ccef1112f9b"},"expiration_timestamp":1722643200,"spending_timestamp":1720137600},{"signature":{"h":"817b9fa0eef617ce751963d9c853df56fdf38a643b9ef8649b658d7ec3da61d49d9279f22509c66dd07051132a418c62","s":"af9fd22c2fde4cb36a0ee021c5a756f97f871ea404d639af1be845b277bed583f715d228d3556e79d15eba23a5b0d5e0"},"expiration_timestamp":1722643200,"spending_timestamp":1720224000},{"signature":{"h":"89f04f5ae1a1532551c2e17166775eac3c5ebba0198eff2919073efd733d6e17e0a6496e31061b0797f51b69d0fa17fc","s":"b752cdb2f4d8a4254d24d5773a8490de1ad84207cbe693638d1fdcbffd8426be96fae3586f19c570beb005bb6bf27b9b"},"expiration_timestamp":1722643200,"spending_timestamp":1720310400},{"signature":{"h":"b1072da2612a6ebc0c02c7f673898dfea88eeeeb076a9dd2ab981d47a8176b87804ca6090cfb5b0c128f06adc6fdf0de","s":"8746a62f6333c3948f1fbb05543f0095cd44f610926c71f339511bf3bb97d98fd019d44c4a4dfc6710a9d5c718c5bfc2"},"expiration_timestamp":1722643200,"spending_timestamp":1720396800},{"signature":{"h":"8d266e7e56af7364d79c4190b24d5f0a601d7771c359815f642115123f5418d0304a46226d70e873b10a051f15178630","s":"a11d67c3293b224abb1256050031b32236484dc0f0fe66a9363d322e6b8c9b24719f4d4da0584d6106eb0f59fa6bb3c8"},"expiration_timestamp":1722643200,"spending_timestamp":1720483200},{"signature":{"h":"8ee31ef821949be316528b7b75b81baa6d0cf2846b615ff6d34e478f25746a07287ae6944279550879735740803c7922","s":"aa26b0df4445d8a10a226f1c70908451d211f787458c0743ba891c46bbbbde3bc59777f4edb9cabdd5fd181d772cfa43"},"expiration_timestamp":1722643200,"spending_timestamp":1720569600},{"signature":{"h":"b8fbacde3f9d507fb5bf6d5f96a33dd5c9c40cc9a062e92f9cdc6a2bddf8cbd873ffcc5112de816f98fbf47b74330abb","s":"a791aee82bdcf3cc4203ce657fbd2c3e9c2f0decd032c4ab91713d3cfd0ac57ebb9bb20e133d9a8a4ba1b8692a3b2706"},"expiration_timestamp":1722643200,"spending_timestamp":1720656000},{"signature":{"h":"b9ed48b47c1a1e6a1ae847ffaad01e0b7d22b2c9f9075beedf4924958ca044579f1e9ca18e268e2a0ef743c214934692","s":"a3e9191d0c3651c656a376e99f3d8c121a742ba55522765479ce866df8aaecc0281ae08003439debf7977f23e450a45f"},"expiration_timestamp":1722643200,"spending_timestamp":1720742400},{"signature":{"h":"a6f1fbdcc28953f318f067cde6e3d6ad0362fd46a501a74492be0884308cf306a7f30006181710f8883a380a8d4885f7","s":"9260d035ea97fc1b93d2db3802678005c95a4acdb0ce0c947cb0b9866ba4028a3b833d742f02ed0227ed33cee4ed17b4"},"expiration_timestamp":1722643200,"spending_timestamp":1720828800},{"signature":{"h":"95109fd35f7808084f224aa1fde645728c0c163751fe24d97e0002a7c38dc5a3a592dcb770893ddd25fc67f84e29e0a1","s":"b85279c4566aa365d208b57817ce96f1a1245895fbe25363202da89559fc454a2cfb09972fc3d1c468b96e8bc11b424c"},"expiration_timestamp":1722643200,"spending_timestamp":1720915200},{"signature":{"h":"8f4b7f93871d8f995a248d31664b02e1fe7107d67512f248a8f550d477846ca9f9862a5d44f5da458c6b83f4d8e62494","s":"b87e46a5d25de574eb4e16cf24199b2e0a658e0c1ef965b632165ed2d695637df76d373484dc93cec30e2f0a29ac91f8"},"expiration_timestamp":1722643200,"spending_timestamp":1721001600},{"signature":{"h":"b740491d3527f8fa0b8bed76d234d0737c09a049f653bb1e8a552b29f568aabd077604b08e7337d44f551fbda565e52e","s":"a08634a2560b3a79c8b56ff5387ef76c397c277810d78cc6dfac2161673df761d0ba61e6075e8a4781ec1b2ba7715b93"},"expiration_timestamp":1722643200,"spending_timestamp":1721088000},{"signature":{"h":"8d4b92fe48257d2574223fbe12afcdabd00db976e8d20faaddb5fb570373edb2a15af4ad9a7c10ceda3bb1304b0210aa","s":"8f8592c45a1cca753e5d0b04ffc2947fd27c7716f2751e632f2bc1bf0e73b179ce8080325951beb0886c5cf0fca79372"},"expiration_timestamp":1722643200,"spending_timestamp":1721174400},{"signature":{"h":"891d412e631041d07115cacf847575462d2389402a0e243a2f106b5e1c8f1431fa8e1aedc55cf943aef9fa8125d105a1","s":"ad2038ab95b823a8119bcc473be22101c4b58fd44ad375235c3664341fc617fe129702aec8765d8dc32ac845aec58a6a"},"expiration_timestamp":1722643200,"spending_timestamp":1721260800},{"signature":{"h":"84b97558e7889af167ba01ed608418ddabfc7fa71ba8f9cb045027f994fe9b5f9bb0ccc15822d1c73a37a48b041cd759","s":"adc370bc182ff9c7057591a3d4afcb21c714093e480374a07608c9f9d2baeff8c14fc5de3f0f457ea340e445ecc28487"},"expiration_timestamp":1722643200,"spending_timestamp":1721347200},{"signature":{"h":"b29b245acf66082c0f4e52cbf9db00ecb9cee070ddb1f5b56a9e85e2563dae04186ed03b7b590e0a9f4b4304ae1cd370","s":"8196445495ed1ee098bbe6083ece3522d4669e23dc4f3e2226e0e63294f1bc791e66380deedd21e5679e3019e93c7727"},"expiration_timestamp":1722643200,"spending_timestamp":1721433600},{"signature":{"h":"9750b9bdf1e86e8e4e6de44392f2160059da64838357a16e64155a5b7989fe7958ae95501e7c1f4039cf8b372ac3b214","s":"80707aa94af054fedcb4947809dab7a9317e303a12b7fb30858115c95bf8e46fec7ea9b0ce5097b75d8f7905c205f757"},"expiration_timestamp":1722643200,"spending_timestamp":1721520000},{"signature":{"h":"828be64b5cd147b5c43e68a2084cb77a762967d5621769136a0894c3d1ee50890412120d7c765cb3c835435d57ac4438","s":"956ca67f8f8591e72b8feeb53cbe52a0f69202313aa729c09760fb2a19b346fda1b7afd326822889fb228ca77a503072"},"expiration_timestamp":1722643200,"spending_timestamp":1721606400},{"signature":{"h":"83ac2c778f0b847af8a4bcc37bd5c2734544fe4427ee7ec5c4690738005cc805864fd5944266417ba2795d7333bd8c4a","s":"88936b7f61a0febfbedc8c0c40dad8f9c4eacb41d276ba00b7d03602ca69614faacca0d945713a272e9748dd6dd71f81"},"expiration_timestamp":1722643200,"spending_timestamp":1721692800},{"signature":{"h":"911c126da663478023efd1157586681937ac7f6c5a33b34a102b3611a63780b244ecdf05ecad09701e65ad20c5246f6a","s":"a1188f3b6ced776890b038e2d07b6af67f1a2c08f54a32d48dbfce61be3295e8ea910b6197b111a6ff16dd668eeda9f5"},"expiration_timestamp":1722643200,"spending_timestamp":1721779200},{"signature":{"h":"8c03c749b424e8fb3f26d51e45d21a7e9f59e6836f89f4a1f27b952e2bc8c038f3372f593dfb3bd284feb36918c2f194","s":"b20d3fd194f73142f0dfc68981519b76aa013e6743b2bdf63d41f3aa1e754d1f74c46012adf0332723f861e5aaa69dba"},"expiration_timestamp":1722643200,"spending_timestamp":1721865600},{"signature":{"h":"872de60a19f3ece61a70519b1f7a63476d13a20721ff00ae923a984bd710703af55112092e209970f074a8950b9e246e","s":"854906bbce5a0c2e47fa829441a03f5c3ac9357dc6d107547198f17505cd17852806e6d69b78264921a8c13c4a9fe50c"},"expiration_timestamp":1722643200,"spending_timestamp":1721952000},{"signature":{"h":"a2a57fd2b675fccccae34f9ff465f36638ee3809291c9b37b5830015a1a7c4d873528105f030a05efd16c6411672fe88","s":"b42f31e4ed05d3b026a62c5df97e04568c0c92d3fafaf95fa732ce69fbe7ee091ac025d04e3ce526b3474edaaf278398"},"expiration_timestamp":1722643200,"spending_timestamp":1722038400},{"signature":{"h":"b9be127e812602ac7a612897fd95e5869b360046993139590c82c2844654b6a5a72e9d74ed38eea6ee99ba0978382a3b","s":"b027db49bb9b0ec3931fc1cb4a6a739c968c45a59698d199056ebfcbeca7ddb363a123e55d05fbf5635d915be2a16ffb"},"expiration_timestamp":1722643200,"spending_timestamp":1722124800},{"signature":{"h":"8c40b2a5e17e1f9cdbe85eaaa419126374d35a255b8338474d442d0d40a4e10c3f2668f579155dfdad3fd403523683df","s":"a40cec4b00c9a975db6c307f25425f3a4d0e49eca2c05d5122630c54bd5d114f74e4abb78a7261752a9dfd4c020f8b91"},"expiration_timestamp":1722643200,"spending_timestamp":1722211200},{"signature":{"h":"b72821bc38167e7f422dcc669c19e0aeffb8247192c4bd50da1503b6aa28041bb5605598639bdb8b945dbc804fc852aa","s":"9443cad7c9d6194b601daf21cbc5fe3019d3350dd725f6d9e3ae366162f93e3b738779617f77ec146f0a05f9a40b602b"},"expiration_timestamp":1722643200,"spending_timestamp":1722297600},{"signature":{"h":"8fe36994ac7e92b5ad0869f2b1bfa3247aac27599459cce9d94cf84eeead8bba1ca4b294d16db3588a0eb9d9b28d4051","s":"8eae0ef0e966f9f473bd829f18e28cf9730447452040aaef114701f409ff237965f203f7394a7d61ca745a4b2f058d9b"},"expiration_timestamp":1722643200,"spending_timestamp":1722384000},{"signature":{"h":"a9f91a83a1db0c5609fcacd1290f1650e4e5ab98c3dd9f0b8fa1c54664c12ed17478ae6e598f0e900ace30addf1b0559","s":"801658058f8de9ba1c25518905196d7e4e0bc5e26a7a810c9dce3ac141f01c538c62808b4728ff683dee78670b2846f9"},"expiration_timestamp":1722643200,"spending_timestamp":1722470400},{"signature":{"h":"94ecd95c1d5ae03270079476aad6334dafdcf615157a6a34b05bb755a047bc2515e33a7e099f9fddc4eec3def8904cdf","s":"b59ff6ded76926a9ae97447ba00eac53d9aa0e1168218d990c5b97cc128a4f5a0c42f961f84cad8ad344630b2a15f3a9"},"expiration_timestamp":1722643200,"spending_timestamp":1722556800},{"signature":{"h":"ace778e70775d349c1e679e5ccef634a533738e7af8b51daa45ccbd920f976d15c4ba744d6ae8fdf711b84b20703cef6","s":"80581faceda2bc35b81240017e1ce8f121476c5ba4d75acddfc58ba6aaa2f12d626eaaab185548e6dd716b6e582740b8"},"expiration_timestamp":1722643200,"spending_timestamp":1722643200}]}"#;
        let _: AggregatedExpirationDateSignatureResponse = serde_json::from_str(raw).unwrap();
    }

    #[test]
    fn batch_redemption_body_roundtrip() {
        let sn = vec![b"foomp".to_vec(), b"bar".to_vec()];
        let gateway: AccountId = "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf".parse().unwrap();
        let digest = [42u8; 32].to_vec();

        let req = BatchRedeemTicketsBody::new(digest, 69, sn, gateway);
        let bytes = serde_json::to_vec(&req).unwrap();

        let de: BatchRedeemTicketsBody = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(req, de);
    }
}
