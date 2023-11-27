// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::DkgInfo;
use nym_coconut_dkg_common::dealer::ContractDealing;
use nym_coconut_dkg_common::types::DealerDetails;
use nym_coconut_dkg_common::verification_key::ContractVKShare;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use std::collections::BTreeMap;

struct FullDealer<'a> {
    info: &'a DealerDetails,
    dealing: Option<&'a ContractDealing>,
    vk_share: Option<&'a ContractVKShare>,
}

impl<'a> FullDealer<'a> {
    fn new(info: &'a DealerDetails) -> Self {
        FullDealer {
            info,
            dealing: None,
            vk_share: None,
        }
    }

    fn format(self) -> Vec<Line<'a>> {
        let dealing = if let Some(_dealing) = self.dealing {
            Span::styled("Submitted", Style::default().bold().green())
        } else {
            Span::styled("UNSUBMITTED", Style::default().bold().red())
        };

        let mut vk_share = if let Some(vk_share) = self.vk_share {
            let verified = if vk_share.verified {
                Span::styled(" share has been verified", Style::default().bold().green())
            } else {
                Span::styled(" share has been UNVERIFIED", Style::default().bold().red())
            };

            vec![
                Line::from(vec![
                    "  - VK share: ".into(),
                    Span::styled("Submitted", Style::default().bold().green()),
                    " for Epoch ".into(),
                    Span::styled(
                        vk_share.epoch_id.to_string(),
                        Style::default().bold().yellow(),
                    ),
                    verified,
                ]),
                Line::from(vec![
                    "  - Assigned index: ".into(),
                    Span::styled(
                        vk_share.node_index.to_string(),
                        Style::default().bold().yellow(),
                    ),
                ]),
            ]
        } else {
            vec![Line::from(vec![
                "  - VK share: ".into(),
                Span::styled("UNSUBMITTED", Style::default().bold().red()),
            ])]
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(format!("{}: ", self.info.address), Style::default().bold()),
                "\tindex: ".into(),
                Span::styled(
                    format!("{}", self.info.assigned_index),
                    Style::default().bold().yellow(),
                ),
                "\t announce address: ".into(),
                Span::styled(
                    self.info.announce_address.to_string(),
                    Style::default().bold().yellow(),
                ),
                "\t BTE key: ".into(),
                Span::styled(
                    format!("{}...", &self.info.bte_public_key_with_proof[..16]),
                    Style::default().bold().yellow(),
                ),
            ]),
            Line::from(vec!["  - dealing: ".into(), dealing]),
        ];

        lines.append(&mut vk_share);
        lines
    }
}

// fn format_contract_bytes(bytes: &ContractSafeBytes) -> String {
//     const MAX_LEN: usize = 32;
//     let mut output = "0x".to_string();
//     for byte in bytes.0.iter().take(MAX_LEN) {
//         output.push_str(&format!("{byte:02X}"));
//     }
//     output.push_str("...");
//     output
// }

pub fn draw_current_dealers_info(
    info: &DkgInfo,
    f: &mut Frame<'_>,
    rect: Rect,
) -> anyhow::Result<()> {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "Current Dealers",
            Style::default().add_modifier(Modifier::BOLD),
        )]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let mut dealers = BTreeMap::new();
    for dealer in &info.dealers {
        dealers.insert(dealer.address.clone(), FullDealer::new(dealer));
    }
    for dealing in &info.epoch_dealings {
        if let Some(dealer) = dealers.get_mut(&dealing.dealer) {
            dealer.dealing = Some(dealing)
        }
    }
    for vk_share in &info.vk_shares {
        if let Some(dealer) = dealers.get_mut(&vk_share.owner) {
            dealer.vk_share = Some(vk_share)
        }
    }

    let mut text = Vec::new();

    for (_, dealer_details) in dealers {
        text.append(&mut dealer_details.format())
    }

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, rect);

    Ok(())
}
