// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

pub mod api;
pub mod client;
mod helpers;

macro_rules! absolute_route {
    ( $name:ident, $parent:expr, $suffix:expr ) => {
        pub fn $name() -> String {
            format!("{}{}", $parent, $suffix)
        }
    };
}

pub mod routes {
    pub const ROOT: &str = "/";
    pub const API: &str = "/api";

    pub mod api {
        pub const V1: &str = "/v1";

        absolute_route!(v1_absolute, super::API, V1);

        pub mod v1 {
            use super::*;

            pub const SWAGGER: &str = "/swagger";
            pub const TICKETBOOK: &str = "/ticketbook";

            // define helper functions to get absolute routes
            absolute_route!(swagger_absolute, v1_absolute(), SWAGGER);
            absolute_route!(ticketbook_absolute, v1_absolute(), TICKETBOOK);

            pub mod ticketbook {
                use super::*;

                pub const OBTAIN: &str = "/obtain";
                pub const OBTAIN_ASYNC: &str = "/obtain-async";
                pub const DEPOSIT_AMOUNT: &str = "/deposit-amount";
                pub const MASTER_KEY: &str = "/master-verification-key";
                pub const PARTIAL_KEYS: &str = "/partial-verification-keys";
                pub const CURRENT_EPOCH: &str = "/current-epoch";
                pub const SHARES: &str = "/shares";

                absolute_route!(
                    obtain_wallet_shares_absolute,
                    ticketbook_absolute(),
                    OBTAIN
                );
                absolute_route!(
                    obtain_async_wallet_shares_absolute,
                    ticketbook_absolute(),
                    OBTAIN_ASYNC
                );
                absolute_route!(
                    current_deposit_amount_absolute,
                    ticketbook_absolute(),
                    DEPOSIT_AMOUNT
                );
                absolute_route!(master_key_absolute, ticketbook_absolute(), MASTER_KEY);
                absolute_route!(partial_keys_absolute, ticketbook_absolute(), PARTIAL_KEYS);
                absolute_route!(current_epoch_absolute, ticketbook_absolute(), CURRENT_EPOCH);
                absolute_route!(shares_absolute, ticketbook_absolute(), SHARES);

                pub mod shares {
                    use super::*;

                    pub const SHARE_BY_ID: &str = "/:share_id";
                    pub const SHARE_BY_DEVICE_AND_CREDENTIAL_ID: &str =
                        "/device/:device_id/credential/:credential_id";

                    absolute_route!(share_by_id_absolute, shares_absolute(), SHARE_BY_ID);
                    absolute_route!(share_by_device_and_credential_id_absolute, shares_absolute(), SHARE_BY_DEVICE_AND_CREDENTIAL_ID);
                }
            }
        }
    }
}
