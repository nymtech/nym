// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::Action;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub mod home;

pub trait Component {
    fn init(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    // fn handle_events(&mut self, event: Option<Event>) -> anyhow::Result<Option<Action>> {
    //     let r = match event {
    //         Some(Event::Key(key_event)) => self.handle_key_events(key_event)?,
    //         Some(Event::Mouse(mouse_event)) => self.handle_mouse_events(mouse_event)?,
    //         _ => None,
    //     };
    //     Ok(r)
    // }
    #[allow(unused_variables)]
    fn handle_key_events(&mut self, key: KeyEvent) -> anyhow::Result<Option<Action>> {
        Ok(None)
    }
    // #[allow(unused_variables)]
    // fn handle_mouse_events(&mut self, mouse: MouseEvent) -> anyhow::Result<Option<Action>> {
    //     Ok(None)
    // }
    #[allow(unused_variables)]
    fn update(&mut self, action: Action) -> anyhow::Result<Option<Action>> {
        Ok(None)
    }
    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> anyhow::Result<()>;
}
