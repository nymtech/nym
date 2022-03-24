use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::Wallet;

pub struct PayInfo {}

pub fn spend(wallet: &Wallet, verification_key: &VerificationKeyAuth, skUser: &SecretKeyUser, payInfo: &PayInfo) {
    //
}

pub fn spend_verify() {}
