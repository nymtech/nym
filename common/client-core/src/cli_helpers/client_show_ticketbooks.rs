// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::{CliClient, CliClientConfig};
use crate::error::ClientCoreError;
use nym_credential_storage::models::BasicTicketbookInformation;
use nym_credential_storage::storage::Storage;
use nym_ecash_time::ecash_today;
use nym_network_defaults::TICKET_BANDWIDTH_VALUE;
use serde::{Deserialize, Serialize};
use time::Date;

#[derive(Serialize, Deserialize)]
pub struct AvailableTicketbook {
    pub id: i64,
    pub expiration: Date,
    pub issued_tickets: u32,
    pub claimed_tickets: u32,
    pub ticket_size: u64,
}

impl AvailableTicketbook {
    #[cfg(feature = "cli")]
    fn table_row(&self) -> comfy_table::Row {
        let ecash_today = ecash_today().date();

        let issued = self.issued_tickets;
        let si_issued = si_scale::helpers::bibytes2((issued as u64 * self.ticket_size) as f64);

        let claimed = self.claimed_tickets;
        let si_claimed = si_scale::helpers::bibytes2((claimed as u64 * self.ticket_size) as f64);

        let remaining = issued - claimed;
        let si_remaining =
            si_scale::helpers::bibytes2((remaining as u64 * self.ticket_size) as f64);
        let si_size = si_scale::helpers::bibytes2(self.ticket_size as f64);

        let expiration = if self.expiration <= ecash_today {
            comfy_table::Cell::new(format!("EXPIRED ON {}", self.expiration))
                .fg(comfy_table::Color::Red)
                .add_attribute(comfy_table::Attribute::Bold)
        } else {
            comfy_table::Cell::new(self.expiration.to_string())
        };

        vec![
            comfy_table::Cell::new(self.id.to_string()),
            expiration,
            comfy_table::Cell::new(format!("{issued} ({si_issued})")),
            comfy_table::Cell::new(format!("{claimed} ({si_claimed})")),
            comfy_table::Cell::new(format!("{remaining} ({si_remaining})")),
            comfy_table::Cell::new(si_size),
        ]
        .into()
    }
}

impl From<BasicTicketbookInformation> for AvailableTicketbook {
    fn from(value: BasicTicketbookInformation) -> Self {
        AvailableTicketbook {
            id: value.id,
            expiration: value.expiration_date,
            issued_tickets: value.total_tickets,
            claimed_tickets: value.used_tickets,
            ticket_size: TICKET_BANDWIDTH_VALUE,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct AvailableTicketbooks(Vec<AvailableTicketbook>);

#[cfg(feature = "cli")]
impl std::fmt::Display for AvailableTicketbooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut table = comfy_table::Table::new();
        table.set_header(vec![
            "id",
            "expiration",
            "issued tickets (bandwidth)",
            "claimed tickets (bandwidth)",
            "remaining tickets (bandwidth)",
            "ticket size",
        ]);

        for ticketbook in &self.0 {
            table.add_row(ticketbook.table_row());
        }

        writeln!(f, "{table}")?;
        Ok(())
    }
}

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct CommonShowTicketbooksArgs {
    /// Id of client that is going to display the ticketbook information
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,
}

pub async fn show_ticketbooks<C, A>(args: A) -> Result<AvailableTicketbooks, C::Error>
where
    A: AsRef<CommonShowTicketbooksArgs>,
    C: CliClient,
{
    let common_args = args.as_ref();
    let id = &common_args.id;

    let config = C::try_load_current_config(id).await?;
    let paths = config.common_paths();

    let credentials_store =
        nym_credential_storage::initialise_persistent_storage(&paths.credentials_database).await;
    let ticketbooks = credentials_store
        .get_ticketbooks_info()
        .await
        .map_err(|err| ClientCoreError::CredentialStoreError {
            source: Box::new(err),
        })?;

    Ok(AvailableTicketbooks(
        ticketbooks.into_iter().map(Into::into).collect(),
    ))
}
