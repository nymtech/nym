use crate::order::OrderSignature;
use validator_client::nyxd::error::NyxdError;
use validator_client::nyxd::wallet::{AccountData, DirectSecp256k1HdWallet};

/// Signs an order message to purchase Nym with Bity
pub fn sign_order(
    wallet: &DirectSecp256k1HdWallet,
    signer: &AccountData,
    message: String,
) -> Result<OrderSignature, NyxdError> {
    Ok(OrderSignature {
        account_id: signer.address().clone(),
        public_key: signer.public_key(),
        signature_as_hex: wallet
            .sign_raw_with_account(signer, &message.into_bytes())?
            .to_string(),
    })
}
