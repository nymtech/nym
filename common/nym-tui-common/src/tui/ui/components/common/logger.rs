// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::tui::config::keybindings::LoggerKeybindings;
use crate::tui::dispatcher::store::State;
use crate::tui::dispatcher::ActionSender;
use crate::tui::ui::components::Component;
use crate::AppAction;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::widgets::Block;
use ratatui::Frame;
use std::marker::PhantomData;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget, TuiWidgetState};

pub struct Props {
    pub keybindings: LoggerKeybindings,
}

pub struct Logger<S, A> {
    props: Props,
    widget_state: TuiWidgetState,

    phantom_state: PhantomData<S>,
    phantom_action: PhantomData<A>,
}

impl<S, A> Component for Logger<S, A>
where
    S: State,
    for<'a> Props: From<&'a S>,
    A: AppAction,
{
    type State = S;
    type Actions = A;

    fn new(state: &Self::State, _action_sender: ActionSender<Self::Actions>) -> Self
    where
        Self: Sized,
    {
        Logger {
            props: Props::from(state),
            widget_state: TuiWidgetState::new(),
            phantom_state: PhantomData,
            phantom_action: PhantomData,
        }
    }

    fn tick(&mut self) -> bool {
        true
    }

    fn handle_key(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        if let Some(tui_event) = self.props.keybindings.tui_logger_event(key.into()) {
            self.widget_state.transition(tui_event)
        }
        Ok(())
    }

    fn view(&mut self, frame: &mut Frame, rect: Rect) {
        let border = Block::bordered();
        let inner_area = border.inner(rect);
        frame.render_widget(border, rect);

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
            .state(&self.widget_state);
        frame.render_widget(tui_sm, inner_area);
    }
}
