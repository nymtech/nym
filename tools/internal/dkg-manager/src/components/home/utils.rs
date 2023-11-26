// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_dkg_common::dealer::ContractDealing;
use nym_coconut_dkg_common::types::{ContractSafeBytes, DealerDetails, TimeConfiguration};
use nym_validator_client::nyxd::cw4::{Member, TotalWeightResponse};
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

pub fn cw4_members_header(total_weight: &TotalWeightResponse) -> Line {
    Line::from(vec![
        Span::styled("CW4 Group Members", Style::default().bold()),
        " (total weight: ".into(),
        Span::styled(
            total_weight.weight.to_string(),
            Style::default().yellow().bold(),
        ),
        ")".into(),
    ])
}

pub fn format_cw4_member(member: &Member) -> Line {
    Line::from(vec![
        Span::styled(&member.addr, Style::default().bold()),
        " (weight: ".into(),
        Span::styled(member.weight.to_string(), Style::default().yellow()),
        ")".into(),
    ])
}

pub fn format_dealing(dealing: &ContractDealing) -> Line {
    fn format_contract_bytes(bytes: &ContractSafeBytes) -> String {
        const MAX_LEN: usize = 32;
        let mut output = "0x".to_string();
        for byte in bytes.0.iter().take(MAX_LEN) {
            output.push_str(&format!("{byte:02X}"));
        }
        output.push_str("...");
        output
    }

    Line::from(vec![
        Span::styled(dealing.dealer.to_string(), Style::default().bold()),
        " submitted: ".into(),
        Span::styled(
            format_contract_bytes(&dealing.dealing),
            Style::default().light_cyan().dim(),
        ),
    ])
}

pub fn format_time_configuration<'a>(tc: TimeConfiguration) -> Vec<Line<'a>> {
    vec![
        Span::styled("Time Configuration", Style::default().bold()).into(),
        Line::from(vec![
            "Public Key Submission: ".into(),
            Span::styled(
                format!("{}secs", tc.public_key_submission_time_secs),
                Style::default().yellow(),
            ),
        ]),
        Line::from(vec![
            "Dealing Exchange: ".into(),
            Span::styled(
                format!("{}secs", tc.dealing_exchange_time_secs),
                Style::default().yellow(),
            ),
        ]),
        Line::from(vec![
            "Verification Key Submission: ".into(),
            Span::styled(
                format!("{}secs", tc.verification_key_submission_time_secs),
                Style::default().yellow(),
            ),
        ]),
        Line::from(vec![
            "Verification Key Validation: ".into(),
            Span::styled(
                format!("{}secs", tc.verification_key_validation_time_secs),
                Style::default().yellow(),
            ),
        ]),
        Line::from(vec![
            "Verification Key Finalization: ".into(),
            Span::styled(
                format!("{}secs", tc.verification_key_finalization_time_secs),
                Style::default().yellow(),
            ),
        ]),
        Line::from(vec![
            "In Progress: ".into(),
            Span::styled(
                format!("{}secs", tc.in_progress_time_secs),
                Style::default().yellow(),
            ),
        ]),
    ]
}
