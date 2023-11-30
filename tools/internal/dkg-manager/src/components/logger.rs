// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::{Action, LoggerAction};
use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;
use std::collections::HashMap;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget, TuiWidgetState};

pub struct Logger {
    state: TuiWidgetState,
    pub keymap: HashMap<KeyEvent, Action>,
}

impl Logger {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Logger {
            state: TuiWidgetState::new(),
            keymap: Default::default(),
        }
    }

    pub fn keymap(mut self, keymap: HashMap<KeyEvent, Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn tick(&mut self) {
        //
    }

    pub fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()> {
        let block = Block::default().borders(Borders::ALL);
        let inner_area = block.inner(rect);
        f.render_widget(block, rect);

        let tui_sm = TuiLoggerSmartWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_info(Style::default().fg(Color::Green))
            .style_debug(Style::default().fg(Color::Cyan))
            .style_trace(Style::default().fg(Color::Magenta))
            .output_separator(':')
            .output_timestamp(Some("%F %H:%M:%S%.3f".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Long))
            .output_target(true)
            .output_file(true)
            .output_line(true)
            .state(&self.state);
        f.render_widget(tui_sm, inner_area);

        Ok(())
    }

    pub fn update(&mut self, action: LoggerAction) -> anyhow::Result<Option<Action>> {
        match action {
            LoggerAction::WidgetKeyEvent(event) => self.state.transition(&event),
        };

        Ok(None)
    }

    pub fn handle_key_events(&mut self, _key: KeyEvent) -> anyhow::Result<Option<Action>> {
        Ok(None)
    }
}
