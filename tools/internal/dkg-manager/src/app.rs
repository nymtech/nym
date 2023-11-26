// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::ActionSender;
use crate::cli::Args;
use crate::components::logger::Logger;
use crate::keybindings::KeyBindings;
use crate::nyxd::setup_nyxd_client;
use crate::utils::key_event_to_string;
use crate::{action::Action, components::home::Home};
use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,

    Logger,
}

pub enum Tab {
    Home,
    Logger,
}

pub struct App {
    pub keybindings: KeyBindings,
    pub home: Home,
    pub logger: Logger,

    pub action_tx: ActionSender,

    pub should_quit: bool,
    pub mode: Mode,

    pub active_tab: usize,
    pub tab_titles: Vec<&'static str>,

    pub last_tick_key_events: Vec<KeyEvent>,
}

impl App {
    pub async fn new(args: Args) -> anyhow::Result<(Self, UnboundedReceiver<Action>)> {
        let nyxd_client = setup_nyxd_client(args)?;

        let keybindings = KeyBindings::default();

        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let action_sender = ActionSender(action_tx);

        let home = Home::new(nyxd_client, action_sender.clone()).await?;
        let logger = Logger::new();

        let mode = Mode::Home;
        Ok((
            Self {
                keybindings,
                home,

                logger,
                action_tx: action_sender,
                should_quit: false,
                mode,
                last_tick_key_events: Vec::new(),
                active_tab: 0,
                tab_titles: vec!["🥥 Contract Information 🥥", "📝 Logs 📝"],
            },
            action_rx,
        ))
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tab_titles.len();

        // TEMP: currently we have two modes so it's easy to keep track of it
        if self.mode == Mode::Home {
            self.mode = Mode::Logger
        } else {
            self.mode = Mode::Home
        }
    }

    pub fn previous_tab(&mut self) {
        if self.active_tab > 0 {
            self.active_tab -= 1;
        } else {
            self.active_tab = self.tab_titles.len() - 1;
        }

        // TEMP: currently we have two modes so it's easy to keep track of it
        if self.mode == Mode::Home {
            self.mode = Mode::Logger
        } else {
            self.mode = Mode::Home
        }
    }

    pub fn action_sender(&self) -> ActionSender {
        self.action_tx.clone()
    }

    pub fn draw(&mut self, f: &mut Frame<'_>) -> anyhow::Result<()> {
        let frame_size = f.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(frame_size);

        let block = Block::default();
        f.render_widget(block, frame_size);
        let titles = self
            .tab_titles
            .iter()
            .map(|t| Line::from(Span::styled(*t, Style::default().bold())))
            .collect();

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .select(self.active_tab)
            .style(Style::default().cyan())
            .highlight_style(Style::default().bold().light_cyan().on_black());
        // .highlight_style(Style::default().bold().on_black());
        f.render_widget(tabs, chunks[0]);

        match self.active_tab {
            0 => {
                self.home.render_tick();
                self.home.draw(f, chunks[1])?;
            }
            1 => {
                self.logger.draw(f, chunks[1])?;
            }
            _ => unreachable!(),
        };

        f.render_widget(
            self.history_widget(),
            Rect {
                x: frame_size.x + 1,
                y: frame_size.height.saturating_sub(1),
                width: frame_size.width.saturating_sub(2),
                height: 1,
            },
        );

        Ok(())
    }

    pub fn history_widget(&self) -> Block {
        Block::default()
            .title(
                ratatui::widgets::block::Title::from(format!(
                    "{:?}",
                    &self
                        .last_tick_key_events
                        .iter()
                        .map(key_event_to_string)
                        .collect::<Vec<_>>()
                ))
                .alignment(Alignment::Right),
            )
            .title_style(Style::default().bold())
    }

    pub fn key_event(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if let Some(keymap) = self.keybindings.get(&self.mode) {
            if let Some(action) = keymap.get(&vec![key]) {
                info!("Got action: {action:?}");
                self.action_tx.send(action.clone())?;
            } else {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    self.action_tx.send(action.clone())?;
                }
            }
        }

        if let Some(action) = self.home.handle_key_events(key)? {
            self.action_tx.send(action)?;
        }
        Ok(())
    }

    pub fn tick(&mut self) {
        self.last_tick_key_events.drain(..);
        self.home.tick();
        self.logger.tick();
    }

    pub fn update(&mut self, action: Action) -> anyhow::Result<()> {
        debug!("handling action {action:?}");

        let next = match action {
            Action::Quit => {
                self.should_quit = true;
                None
            }
            Action::NextTab => {
                self.next_tab();
                None
            }
            Action::PreviousTab => {
                self.previous_tab();
                None
            }
            Action::Error(err) => {
                error!("unhandled error action: {err}");
                None
            }
            Action::HomeAction(home_action) => self.home.update(home_action)?,
            Action::LoggerAction(logger_action) => self.logger.update(logger_action)?,
        };

        if let Some(action) = next {
            self.action_sender().send(action)?
        }
        Ok(())
    }
}
