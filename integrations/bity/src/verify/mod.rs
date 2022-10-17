use thiserror::Error;

use nym_cli_commands::validator::signature::helpers::secp256k1_verify_with_public_key;

use crate::order::Order;

#[derive(Error, Debug)]
pub enum VerifyOrderError {
    #[error("{source}")]
    K256Error {
        #[from]
        source: k256::ecdsa::Error,
    },
    #[error("{source}")]
    ErrorReport {
        #[from]
        source: eyre::Report,
    },
    #[error("Account id does not match public key")]
    AccountIdDoesNotMatchPubKey,
    #[error("Unsupported key type. Only secp256k1 is currently supported.")]
    UnsupportedKeyType,
    #[error("Signature error - {0}")]
    SignatureError(k256::ecdsa::signature::Error),
    #[error("Account id is not a Nyx mainnet account")]
    AccountIdPrefixIncorrect,
}

/// Verifies an order
pub fn verify_order(order: Order) -> Result<(), VerifyOrderError> {
    let account_id = order.signature.public_key.account_id("n")?;

    if order.signature.account_id.prefix() != "n" || order.account_id.prefix() != "n" {
        return Err(VerifyOrderError::AccountIdPrefixIncorrect);
    }

    // the account id in the order must match the account id derived from the public key
    if account_id != order.signature.account_id {
        return Err(VerifyOrderError::AccountIdDoesNotMatchPubKey);
    }

    // the user provided account id in the order must match the derived account id
    if account_id != order.account_id || account_id != order.signature.account_id {
        return Err(VerifyOrderError::AccountIdDoesNotMatchPubKey);
    }

    if order.signature.public_key.type_url() != cosmrs::crypto::PublicKey::SECP256K1_TYPE_URL {
        return Err(VerifyOrderError::UnsupportedKeyType);
    }

    match secp256k1_verify_with_public_key(
        &order.signature.public_key.to_bytes(),
        order.signature.signature_as_hex,
        order.message,
    ) {
        Ok(()) => Ok(()),
        Err(e) => Err(VerifyOrderError::SignatureError(e)),
    }
}
