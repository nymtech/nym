// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub const STORAGE_DIR_NAME: &str = "nym-wallet";
        pub const WALLET_INFO_FILENAME: &str = "saved-wallet.json";
    } else if #[cfg(taret_os = "macos")] {
        pub const STORAGE_DIR_NAME: &str = "nym-wallet";
        pub const WALLET_INFO_FILENAME: &str = "saved-wallet.json";
    } else if #[cfg(taret_os = "windows")] {
        pub const STORAGE_DIR_NAME: &str = "NymWallet";
        pub const WALLET_INFO_FILENAME: &str = "saved_wallet.json";
    } else {
        pub const STORAGE_DIR_NAME: &str = "nym-wallet";
        pub const WALLET_INFO_FILENAME: &str = "saved-wallet.json";
    }
}
