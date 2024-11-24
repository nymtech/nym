// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tui::dispatcher::ActionSender;
use crate::{AppAction, State};
use color_eyre::eyre;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use ratatui::Frame;

pub mod common;

// pub trait Props<'a>: From<&'a Self::State> {
//     type State: State;
// }

pub trait Component {
    type State: State;
    type Actions: AppAction;

    fn new(state: &Self::State, action_sender: ActionSender<Self::Actions>) -> Self
    where
        Self: Sized;

    fn update(self, state: &Self::State) -> Self
    where
        Self: Sized,
    {
        let _ = state;
        self
    }

    // returns boolean indicating whether a rerender is needed
    // fn tick(&mut self) -> bool;
    fn tick(&mut self) -> bool {
        false
    }

    fn handle_key(&mut self, key: KeyEvent) -> eyre::Result<()> {
        let _ = key;
        Ok(())
    }

    fn view(&mut self, frame: &mut Frame, rect: Rect);
}
