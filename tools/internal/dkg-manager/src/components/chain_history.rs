// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::Action;
use crate::nyxd::NyxdClient;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

pub struct ContractChainHistory {
    pub nyxd_client: NyxdClient,
}

impl ContractChainHistory {
    pub fn new(nyxd_client: NyxdClient) -> Self {
        ContractChainHistory { nyxd_client }
    }

    pub fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
        let block = Block::default()
            .title("Contract Chain History")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let paragraph = Paragraph::new("Unimplemented")
            .block(block)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Left);

        f.render_widget(paragraph, rect);
        Ok(())
    }

    pub fn update(&mut self, _action: ()) -> anyhow::Result<Option<Action>> {
        todo!()
    }
}
