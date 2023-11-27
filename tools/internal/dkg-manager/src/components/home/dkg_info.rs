// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::DkgInfo;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn draw_dkg_info(info: &DkgInfo, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            "DKG Information",
            Style::default().add_modifier(Modifier::BOLD),
        )]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let end =
        OffsetDateTime::from_unix_timestamp(info.epoch.finish_timestamp.seconds() as i64).unwrap();

    let remaining = end - OffsetDateTime::now_utc();
    let remaining_str = if remaining.is_negative() {
        format!("-{} secs", remaining.whole_seconds())
    } else {
        format!("{} secs", remaining.whole_seconds())
    };

    let threshold_str = if let Some(val) = info.threshold {
        val.to_string()
    } else {
        "not yet determined".to_string()
    };

    let dealers = info.dealers.len();
    let dealings = info.epoch_dealings.len();
    let dealings_color = if dealings < dealers {
        Color::Red
    } else {
        Color::Green
    };

    let shares = info.vk_shares.len();
    let shares_color = if shares < dealings {
        Color::Red
    } else {
        Color::Green
    };

    let text = vec![
        Line::from(vec![
            format!("DKG Epoch {} State: ", info.epoch.epoch_id).into(),
            Span::styled(info.epoch.state.to_string(), Style::default().bold()),
        ]),
        Line::from(vec![
            "Epoch end time: ".into(),
            Span::styled(
                end.format(&Rfc3339).unwrap().to_string(),
                Style::default().bold(),
            ),
            " (".into(),
            Span::styled(remaining_str, Style::default().light_green()),
            " remaining)".into(),
        ]),
        Line::from(vec![
            "Threshold: ".into(),
            Span::styled(threshold_str, Style::default().bold()),
        ]),
        Line::from(vec![
            "Registered Dealers: ".into(),
            Span::styled(dealers.to_string(), Style::default().bold()),
            " (".into(),
            Span::styled(info.past_dealers.len().to_string(), Style::default().bold()),
            " in the past)".into(),
        ]),
        Line::from(vec![
            "Submitted Dealings: ".into(),
            Span::styled(
                dealings.to_string(),
                Style::default().bold().fg(dealings_color),
            ),
        ]),
        Line::from(vec![
            "Submitted VK Shares: ".into(),
            Span::styled(shares.to_string(), Style::default().bold().fg(shares_color)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, rect);

    Ok(())
}
