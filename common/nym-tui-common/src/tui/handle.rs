// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use color_eyre::eyre;
use crossterm::event::EventStream;
use ratatui::backend::CrosstermBackend as Backend;
use ratatui::crossterm::{
    self, cursor,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::Stream;

pub struct TuiHandle {
    pub terminal: ratatui::Terminal<Backend<std::io::Stderr>>,
    pub crossterm_events: EventStream,
}

impl TuiHandle {
    pub fn new() -> Result<TuiHandle, eyre::Error> {
        let terminal = ratatui::Terminal::new(Backend::new(std::io::stderr()))?;
        let crossterm_events = EventStream::new();

        Ok(TuiHandle {
            terminal,
            crossterm_events,
        })
    }

    pub fn enter(&self) -> Result<(), eyre::Error> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), EnterAlternateScreen, cursor::Hide)?;
        Ok(())
    }

    pub fn exit(&mut self) -> eyre::Result<()> {
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.terminal.flush()?;
            crossterm::execute!(std::io::stderr(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }
}

impl Stream for TuiHandle {
    type Item = <EventStream as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.crossterm_events).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.crossterm_events.size_hint()
    }
}

impl Deref for TuiHandle {
    type Target = ratatui::Terminal<Backend<std::io::Stderr>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TuiHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for TuiHandle {
    fn drop(&mut self) {
        // well. at this point we can't do much, we'll just go straight into the panic handler
        #[allow(clippy::expect_used)]
        self.exit().expect("failed to teardown the terminal")
    }
}
