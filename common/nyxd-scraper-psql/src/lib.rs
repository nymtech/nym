// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::block_storage::PostgresScraperStorage;
use nyxd_scraper_shared::NyxdScraper;

pub use nyxd_scraper_shared::constants;
pub use nyxd_scraper_shared::error::ScraperError;
pub use nyxd_scraper_shared::{
    BlockModule, MsgModule, NyxdScraperTransaction, ParsedTransactionResponse, PruningOptions,
    PruningStrategy, StartingBlockOpts, TxModule,
};
pub use storage::models;

pub mod error;
pub mod storage;

pub type PostgresNyxdScraper = NyxdScraper<PostgresScraperStorage>;

// TODO: for now just use exactly the same config
pub use nyxd_scraper_shared::Config;
