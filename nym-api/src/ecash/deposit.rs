// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::Result;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_credentials::IssuanceTicketBook;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_contract_common::deposit::Deposit;

pub async fn validate_deposit(request: &BlindSignRequestBody, deposit: Deposit) -> Result<()> {
    // verify signature with the pubkey used in deposit
    let ed25519 = ed25519::PublicKey::from_base58_string(deposit.bs58_encoded_ed25519_pubkey)?;
    let plaintext =
        IssuanceTicketBook::request_plaintext(&request.inner_sign_request, request.deposit_id);
    ed25519.verify(plaintext, &request.signature)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ecash::error::CoconutError;
    use crate::ecash::tests::voucher_fixture;
    use nym_compact_ecash::{generate_keypair_user, scheme::withdrawal::WithdrawalRequest};
    use rand::rngs::OsRng;
    use time::OffsetDateTime;

    #[tokio::test]
    async fn validate_deposit_test() {
        let mut rng = OsRng;
        let deposit_id = 42;
        let voucher = voucher_fixture(Some(deposit_id));
        let signing_data = voucher.prepare_for_signing();
        let correct_request = voucher.create_blind_sign_request_body(&signing_data);

        let valid_ed25519 = ed25519::KeyPair::new(&mut rng);
        let bs58_encoded_ed25519 = valid_ed25519.public_key().to_base58_string();

        let malformed_deposit = Deposit {
            bs58_encoded_ed25519_pubkey: "invalided25519pubkey".to_string(),
        };

        let err = validate_deposit(&correct_request, malformed_deposit)
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            CoconutError::Ed25519ParseError(
                nym_crypto::asymmetric::identity::Ed25519RecoveryError::MalformedPublicKeyString { .. }
            )
        ));

        let wrong_deposit = Deposit {
            bs58_encoded_ed25519_pubkey: bs58_encoded_ed25519,
        };

        let err = validate_deposit(&correct_request, wrong_deposit)
            .await
            .unwrap_err();
        assert!(matches!(err, CoconutError::SignatureVerificationError(..)));

        //hard-coded values, that generate a correct signature
        let blind_sign_req = WithdrawalRequest::from_bytes(&[
            48, 168, 84, 109, 206, 194, 237, 227, 205, 67, 60, 127, 85, 54, 246, 120, 88, 129, 121,
            50, 255, 133, 50, 54, 155, 172, 179, 52, 16, 250, 6, 209, 67, 54, 251, 20, 37, 124,
            115, 63, 182, 101, 188, 68, 149, 18, 149, 57, 167, 48, 149, 100, 41, 48, 143, 115, 93,
            90, 244, 164, 161, 224, 65, 160, 63, 141, 65, 86, 128, 136, 128, 194, 40, 106, 158, 40,
            235, 242, 51, 108, 3, 109, 120, 11, 100, 82, 188, 61, 41, 12, 232, 54, 162, 243, 43,
            222, 215, 216, 2, 48, 137, 243, 126, 118, 124, 83, 221, 53, 252, 163, 175, 215, 94, 90,
            249, 172, 3, 222, 13, 45, 166, 245, 126, 173, 199, 89, 206, 11, 22, 204, 47, 26, 40,
            191, 217, 139, 75, 101, 45, 5, 62, 251, 52, 36, 117, 101, 166, 63, 48, 152, 195, 163,
            179, 117, 148, 93, 223, 210, 119, 105, 59, 71, 88, 155, 17, 33, 4, 87, 203, 169, 40,
            93, 203, 153, 213, 105, 107, 181, 214, 2, 25, 19, 187, 217, 243, 246, 185, 152, 81,
            118, 11, 169, 100, 74, 88, 215, 37, 32, 31, 123, 5, 222, 103, 255, 236, 74, 37, 222,
            170, 136, 5, 49, 4, 183, 156, 223, 33, 112, 122, 81, 122, 221, 166, 27, 5, 44, 153, 37,
            229, 107, 32, 95, 45, 147, 187, 40, 141, 22, 9, 222, 232, 125, 34, 52, 152, 157, 14,
            228, 200, 183, 29, 62, 24, 201, 228, 103, 119, 89, 186, 79, 116, 75, 53, 2, 32, 219,
            72, 52, 255, 108, 74, 76, 126, 233, 46, 34, 70, 188, 47, 57, 66, 153, 14, 6, 242, 112,
            129, 108, 166, 188, 226, 183, 51, 45, 195, 190, 58, 32, 170, 54, 18, 64, 215, 82, 118,
            243, 66, 186, 137, 175, 230, 172, 174, 226, 104, 188, 123, 239, 77, 180, 32, 225, 73,
            208, 255, 27, 195, 181, 201, 21, 2, 32, 34, 231, 200, 93, 64, 117, 244, 169, 58, 64,
            39, 5, 228, 205, 119, 135, 221, 130, 241, 205, 184, 182, 34, 248, 85, 26, 241, 233, 52,
            244, 17, 15, 32, 157, 211, 145, 238, 16, 101, 55, 132, 233, 11, 249, 129, 41, 226, 250,
            146, 160, 155, 154, 81, 241, 129, 154, 24, 221, 196, 54, 210, 16, 24, 116, 31,
        ])
        .unwrap();
        let expiration_date = OffsetDateTime::from_unix_timestamp(1708300800).unwrap(); // Feb, 19th, 2024
        let ecash_keypair = generate_keypair_user();

        let correct_request = BlindSignRequestBody::new(
            blind_sign_req,
            deposit_id,
            "3MpHDLYMCmuMvZ9zkZXPkTK6nKArvQW3dJA1notoPPxnbBW2ommkR2dkpRWoeWSkUjQSLv1nRyiRzMWbobGLw1eh".parse().unwrap(),
            ecash_keypair.public_key(),
            expiration_date,
        );

        let good_deposit = Deposit {
            bs58_encoded_ed25519_pubkey: "JDTnyotGw3TtbohEamWNjhvGpj3tJz2C4X2Au9PrSTEx".to_string(),
        };

        let res = validate_deposit(&correct_request, good_deposit).await;
        assert!(res.is_ok())
    }
}
