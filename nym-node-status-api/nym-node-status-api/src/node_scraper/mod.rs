pub(crate) mod description;
pub(crate) mod helpers;
pub(crate) mod scraper;

pub(crate) use description::DescriptionScraper;
pub(crate) use scraper::NodeScraper;

pub(crate) mod models {
    pub(crate) use nym_bridges_types::PersistedClientConfig as BridgeInformation;
}
