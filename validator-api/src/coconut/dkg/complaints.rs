// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dkg::error::DkgError;

pub(crate) enum ComplaintReason {
    MalformedBTEPublicKey,
    DealingVerificationError(DkgError),
    MalformedDealing(DkgError),
}

// pub(crate) async fn complaint_period(
//     dkg_client: &DkgClient,
//     state: &mut State,
// ) -> Result<(), CoconutError> {
//     let dkg_params = dkg::bte::setup();
//     let threshold = state
//         .threshold()
//         .expect("We should have a tentative threshold by now");
//     let dealings = dkg_client.get_dealings().await?;
//     for contract_dealing in dealings {
//         match Dealing::try_from_bytes(&contract_dealing.dealing) {
//             Ok(dealing) => {
//                 if let Err(err) =
//                     dealing.verify(&dkg_params, threshold, &state.current_receivers(), None)
//                 {
//                     state.remove_good_dealer(&contract_dealing.dealer);
//                     state.add_bad_dealer(
//                         contract_dealing.dealer,
//                         ComplaintReason::DealingVerificationError(err),
//                     );
//                 }
//             }
//             Err(err) => {
//                 state.remove_good_dealer(&contract_dealing.dealer);
//                 state.add_bad_dealer(
//                     contract_dealing.dealer,
//                     ComplaintReason::MalformedDealing(err),
//                 );
//             }
//         }
//     }
//
//     Ok(())
// }
