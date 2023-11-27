// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::GroupInfo;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

// TODO: move to separate component and get rid of that matching_admin bool
pub fn draw_group_info(
    info: &GroupInfo,
    f: &mut Frame<'_>,
    rect: Rect,
    matching_admin: bool,
) -> anyhow::Result<()> {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "Group Information",
            Style::default().add_modifier(Modifier::BOLD),
        )]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let group_admin = match &info.admin.admin {
        None => Span::styled("<no admin>", Style::default().light_red().bold()),
        Some(admin) => {
            let style = if matching_admin {
                Style::default().light_green().bold()
            } else {
                Style::default().light_red().bold()
            };
            Span::styled(admin, style)
        }
    };

    let mut text = vec![
        Line::from(vec!["Admin: ".into(), group_admin]),
        Line::from(vec![
            "Total Members: ".into(),
            Span::styled(info.members.len().to_string(), Style::new().bold()),
            " (total weight: ".into(),
            Span::styled(
                info.total_weight.weight.to_string(),
                Style::default().yellow().bold(),
            ),
            "):".into(),
        ]),
    ];

    for member in &info.members {
        text.push(Line::from(vec![
            "  - ".into(),
            Span::styled(&member.addr, Style::default().bold()),
            " (weight: ".into(),
            Span::styled(member.weight.to_string(), Style::default().yellow()),
            ")".into(),
        ]))
    }

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, rect);

    Ok(())
}
