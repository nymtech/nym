pub(crate) mod description;
pub(crate) mod helpers;
pub(crate) mod scraper;

pub(crate) use description::DescriptionScraper;
pub(crate) use scraper::NodeScraper;

pub(crate) mod models {
    pub(crate) use nym_bridges_types::PersistedClientConfig as BridgeInformation;

    /// Outcome of fetching `/api/v1/bridges/client-params` after a successful
    /// `/stats` scrape on an entry/exit gateway.
    ///
    /// Only `Found` / `NotPublished` rewrite the DB (`bonded = true` rows only);
    /// `Unchecked` leaves any previously stored bridges alone (timeouts, 5xx,
    /// bad JSON, or mixnode scrapes that never ask for client-params).
    #[derive(Debug, Clone, Default)]
    pub(crate) enum BridgeScrape {
        Found(BridgeInformation),
        NotPublished,
        #[default]
        Unchecked,
    }
}
