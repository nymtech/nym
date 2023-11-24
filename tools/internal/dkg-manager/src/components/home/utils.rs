// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_dkg_common::types::DealerDetails;
use ratatui::prelude::*;

pub fn format_dealer(dealer: &DealerDetails) -> Line {
    let key_start = if dealer.bte_public_key_with_proof.len() < 16 {
        dealer.bte_public_key_with_proof.clone()
    } else {
        format!("{}...", &dealer.bte_public_key_with_proof[..16])
    };

    Line::from(vec![
        Span::styled(format!("{}: ", dealer.address), Style::default().bold()),
        "\tindex: ".into(),
        Span::styled(
            format!("{}", dealer.assigned_index),
            Style::default().bold().yellow(),
        ),
        "\t announce address: ".into(),
        Span::styled(
            dealer.announce_address.to_string(),
            Style::default().bold().yellow(),
        ),
        "\t BTE key: ".into(),
        Span::styled(key_start, Style::default().bold().yellow()),
    ])
}
