// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tui::config::keybindings::key_event_to_string;
use crate::tui::dispatcher::ActionSender;
use crate::{AppAction, Component, State};
use crossterm::event::KeyEvent;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::{Line, Style, Stylize};
use ratatui::widgets::Block;
use ratatui::Frame;
use std::marker::PhantomData;

pub struct DebugHistory<S, A> {
    last_tick_key_events: Vec<KeyEvent>,

    phantom_state: PhantomData<S>,
    phantom_action: PhantomData<A>,
}

impl<S, A> Component for DebugHistory<S, A>
where
    S: State,
    A: AppAction,
{
    type State = S;
    type Actions = A;

    fn new(_state: &Self::State, _action_sender: ActionSender<Self::Actions>) -> Self
    where
        Self: Sized,
    {
        DebugHistory {
            last_tick_key_events: vec![],
            phantom_state: PhantomData,
            phantom_action: PhantomData,
        }
    }

    fn tick(&mut self) -> bool {
        let was_empty = self.last_tick_key_events.is_empty();
        self.last_tick_key_events.drain(..);

        !was_empty
    }

    fn handle_key(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        self.last_tick_key_events.push(key);
        Ok(())
    }

    fn view(&mut self, frame: &mut Frame, rect: Rect) {
        frame.render_widget(
            Block::default()
                .title_top(
                    Line::from(format!(
                        "{:?}",
                        &self
                            .last_tick_key_events
                            .iter()
                            .map(key_event_to_string)
                            .collect::<Vec<_>>()
                    ))
                    .alignment(Alignment::Right),
                )
                .title_style(Style::default().bold()),
            rect,
        );
    }
}
