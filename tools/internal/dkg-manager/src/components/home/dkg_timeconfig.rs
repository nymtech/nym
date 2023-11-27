// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_dkg_common::types::TimeConfiguration;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

pub fn draw_dkg_timeconfig(
    config: TimeConfiguration,
    f: &mut Frame<'_>,
    rect: Rect,
) -> anyhow::Result<()> {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "DKG Time Configuration",
            Style::default().add_modifier(Modifier::BOLD),
        )]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(format_time_configuration(config))
        .block(block)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, rect);

    Ok(())
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
