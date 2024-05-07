// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use crate::error::Result;
    use crate::scheme::aggregation::{aggregate_verification_keys, aggregate_wallets};
    use crate::scheme::keygen::{
        generate_keypair_user, ttp_keygen, SecretKeyAuth, VerificationKeyAuth,
    };
    use crate::scheme::withdrawal::{issue, issue_verify, withdrawal_request, WithdrawalRequest};
    use crate::scheme::{PartialWallet, PayInfo, Payment, Wallet};
    use crate::setup::Parameters;
    use crate::tests::helpers::{
        generate_coin_indices_signatures, generate_expiration_date_signatures,
    };
    use bls12_381::Scalar;
    use itertools::izip;

    #[test]
    fn main() -> Result<()> {
        let total_coins = 32;
        let params = Parameters::new(total_coins);
        // NOTE: Make sure that the date timestamp are calculated at 00:00:00!!
        let expiration_date = 1703721600; // Dec 28 2023 00:00:00
        let spend_date = Scalar::from(1701907200); // Dec 07 2023 00:00:00
        let user_keypair = generate_keypair_user();

        // generate authorities keys
        let authorities_keypairs = ttp_keygen(2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];
        let secret_keys_authorities: Vec<&SecretKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.secret_key())
            .collect();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3]))?;

        // generate valid dates signatures
        let dates_signatures = generate_expiration_date_signatures(
            expiration_date,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )?;

        // generate coin indices signatures
        let coin_indices_signatures = generate_coin_indices_signatures(
            &params,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )?;

        // request a wallet
        let (req, req_info) =
            withdrawal_request(user_keypair.secret_key(), expiration_date).unwrap();
        let req_bytes = req.to_bytes();
        let req2 = WithdrawalRequest::try_from(req_bytes.as_slice()).unwrap();
        assert_eq!(req, req2);

        // issue partial wallets
        let mut wallet_blinded_signatures = Vec::new();
        for auth_keypair in authorities_keypairs {
            let blind_signature = issue(
                auth_keypair.secret_key(),
                user_keypair.public_key(),
                &req,
                expiration_date,
            );
            wallet_blinded_signatures.push(blind_signature.unwrap());
        }

        let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
            wallet_blinded_signatures.iter(),
            verification_keys_auth.iter()
        )
        .enumerate()
        .map(|(idx, (w, vk))| {
            issue_verify(vk, user_keypair.secret_key(), w, &req_info, idx as u64 + 1).unwrap()
        })
        .collect();

        let partial_wallet = unblinded_wallet_shares.first().unwrap().clone();
        let partial_wallet_bytes = partial_wallet.to_bytes();
        let partial_wallet2 = PartialWallet::try_from(&partial_wallet_bytes[..]).unwrap();
        assert_eq!(partial_wallet, partial_wallet2);

        // Aggregate partial wallets
        let mut aggr_wallet = aggregate_wallets(
            &verification_key,
            user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        )?;

        let wallet_bytes = aggr_wallet.to_bytes();
        let wallet = Wallet::try_from(&wallet_bytes[..]).unwrap();
        assert_eq!(aggr_wallet, wallet);

        // Let's try to spend some coins
        let pay_info = PayInfo {
            pay_info_bytes: [6u8; 72],
        };
        let spend_vv = 1;

        let payment = aggr_wallet.spend(
            &params,
            &verification_key,
            user_keypair.secret_key(),
            &pay_info,
            spend_vv,
            &dates_signatures,
            &coin_indices_signatures,
            spend_date,
        )?;

        assert!(payment
            .spend_verify(&verification_key, &pay_info, spend_date)
            .unwrap());

        let payment_bytes = payment.to_bytes();
        let payment2 = Payment::try_from(&payment_bytes[..]).unwrap();
        assert_eq!(payment, payment2);

        Ok(())
    }
}
