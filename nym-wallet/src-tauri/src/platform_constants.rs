// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// Secondary log viewer `WebviewWindow` is disabled on Windows due to WebView2 freezes.
pub const SECONDARY_LOG_WEBVIEW_SUPPORTED: bool = cfg!(not(target_os = "windows"));

pub const CONFIG_DIR_NAME: &str = "nym-wallet";
pub const CONFIG_FILENAME: &str = "config.toml";
pub const STORAGE_DIR_NAME: &str = "nym-wallet";
pub const WALLET_INFO_FILENAME: &str = "saved-wallet.json";
