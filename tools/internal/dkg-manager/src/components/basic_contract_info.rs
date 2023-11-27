// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contracts_common::ContractBuildInformation;
use nym_validator_client::nyxd::{cw2, Coin};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BasicContractInfo {
    pub name: String,
    pub address: String,
    pub balance: Coin,
    pub cw2_version: Option<cw2::ContractVersion>,
    pub build_info: Option<ContractBuildInformation>,
}

impl BasicContractInfo {
    pub fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
        // is it generic for any currency? no. but it's an internal tool so the approximation is good enough
        let amount_nym = format!(
            "{:.6}{}",
            self.balance.amount as f64 / 1000000.,
            self.balance.denom
        );

        let cw2_formatted = if let Some(cw2_version) = &self.cw2_version {
            format!("{} {}", cw2_version.contract, cw2_version.version)
        } else {
            "some silly sausage never set it".to_string()
        };

        let mut text = vec![
            "Address (part might be hidden): ".into(),
            Span::styled(&self.address, Style::default().yellow().bold()).into(),
            Line::from(vec![
                "Balance: ".into(),
                Span::styled(amount_nym, Style::default().yellow().bold()),
            ]),
            Line::from(vec![
                "CW2 version: ".into(),
                Span::styled(cw2_formatted, Style::default().yellow().bold()),
            ]),
        ];

        if let Some(build_info) = &self.build_info {
            text.push(Line::from(vec![
                "build branch: ".into(),
                Span::styled(&build_info.commit_branch, Style::default().yellow().bold()),
            ]));
            text.push(Line::from(vec![
                "build sha: ".into(),
                Span::styled(&build_info.commit_sha, Style::default().yellow().bold()),
            ]));
        }

        let block = Block::default()
            .title(Line::from(vec![Span::styled(
                &self.name,
                Style::default().add_modifier(Modifier::BOLD),
            )]))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Left);

        f.render_widget(paragraph, rect);

        Ok(())
    }
}
