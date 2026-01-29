// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Args;
use nym_node_status_client::models::AttachedTicketMaterials;

#[derive(Args)]
pub struct CredentialArgs {
    #[arg(long)]
    ticket_materials: Option<String>,

    #[arg(long, default_value_t = 1)]
    ticket_materials_revision: u8,
}

impl CredentialArgs {
    pub fn decode_attached_ticket_materials(&self) -> anyhow::Result<AttachedTicketMaterials> {
        let ticket_materials = self
            .ticket_materials
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ticket_materials is required"))?
            .clone();

        Ok(AttachedTicketMaterials::from_serialised_string(
            ticket_materials,
            self.ticket_materials_revision,
        )?)
    }
}
