// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Specify filenames and other platform specific constants to respect platform conventions, or at
// least, something popular on each respective platform.

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
        // This case is likely to be a unix-y system
        pub const STORAGE_DIR_NAME: &str = "nym-wallet";
        pub const WALLET_INFO_FILENAME: &str = "saved-wallet.json";
    }
}
