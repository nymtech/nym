// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::HomeAction;
use crate::{action::Action, app::Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::ops::Deref;
use tui_logger::TuiWidgetEvent;

#[derive(Clone, Debug)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

impl Deref for KeyBindings {
    type Target = HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut inner_home = HashMap::new();
        let mut inner_logger = HashMap::new();
        let mut inner_chain = HashMap::new();

        // those are disgusting but can't be bothered to refactor it for global keybindings
        // GLOBAL:
        inner_home.insert(unchecked_keys("<tab>"), Action::NextTab);
        inner_logger.insert(unchecked_keys("<tab>"), Action::NextTab);
        inner_chain.insert(unchecked_keys("<tab>"), Action::NextTab);

        inner_home.insert(unchecked_keys("<Ctrl-d>"), Action::Quit);
        inner_home.insert(unchecked_keys("<Ctrl-c>"), Action::Quit);
        inner_logger.insert(unchecked_keys("<Ctrl-d>"), Action::Quit);
        inner_logger.insert(unchecked_keys("<Ctrl-c>"), Action::Quit);
        inner_chain.insert(unchecked_keys("<Ctrl-d>"), Action::Quit);
        inner_chain.insert(unchecked_keys("<Ctrl-c>"), Action::Quit);

        // HOME
        inner_home.insert(
            unchecked_keys("<Ctrl-h>"),
            HomeAction::ToggleShowHelp.into(),
        );
        inner_home.insert(
            unchecked_keys("<Ctrl-r>"),
            HomeAction::ScheduleContractRefresh.into(),
        );
        inner_home.insert(unchecked_keys("</>"), HomeAction::StartInput.into());

        // LOGGER
        inner_logger.insert(unchecked_keys("<space>"), TuiWidgetEvent::SpaceKey.into());
        inner_logger.insert(unchecked_keys("<esc>"), TuiWidgetEvent::EscapeKey.into());
        inner_logger.insert(
            unchecked_keys("<pageup>"),
            TuiWidgetEvent::PrevPageKey.into(),
        );
        inner_logger.insert(
            unchecked_keys("<pagedown>"),
            TuiWidgetEvent::NextPageKey.into(),
        );
        inner_logger.insert(unchecked_keys("<up>"), TuiWidgetEvent::UpKey.into());
        inner_logger.insert(unchecked_keys("<down>"), TuiWidgetEvent::DownKey.into());
        inner_logger.insert(unchecked_keys("<left>"), TuiWidgetEvent::LeftKey.into());
        inner_logger.insert(unchecked_keys("<right>"), TuiWidgetEvent::RightKey.into());
        inner_logger.insert(unchecked_keys("<+>"), TuiWidgetEvent::PlusKey.into());
        inner_logger.insert(unchecked_keys("<->"), TuiWidgetEvent::MinusKey.into());
        inner_logger.insert(unchecked_keys("<h>"), TuiWidgetEvent::HideKey.into());
        inner_logger.insert(unchecked_keys("<f>"), TuiWidgetEvent::FocusKey.into());

        let mut inner = HashMap::new();
        inner.insert(Mode::Home, inner_home);
        inner.insert(Mode::Logger, inner_logger);
        inner.insert(Mode::ChainHistory, inner_chain);
        KeyBindings(inner)
    }
}

fn unchecked_keys(raw: &str) -> Vec<KeyEvent> {
    parse_key_sequence(raw).expect("failed to parse the key sequence")
}

pub fn parse_key_sequence(raw: &str) -> Result<Vec<KeyEvent>, String> {
    if raw.chars().filter(|c| *c == '>').count() != raw.chars().filter(|c| *c == '<').count() {
        return Err(format!("Unable to parse `{}`", raw));
    }
    let raw = if !raw.contains("><") {
        let raw = raw.strip_prefix('<').unwrap_or(raw);
        let raw = raw.strip_prefix('>').unwrap_or(raw);
        raw
    } else {
        raw
    };
    let sequences = raw
        .split("><")
        .map(|seq| {
            if let Some(s) = seq.strip_prefix('<') {
                s
            } else if let Some(s) = seq.strip_suffix('>') {
                s
            } else {
                seq
            }
        })
        .collect::<Vec<_>>();

    sequences.into_iter().map(parse_key_event).collect()
}

fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.to_ascii_lowercase();
    let (remaining, modifiers) = extract_modifiers(&raw_lower);
    parse_key_code_with_modifiers(remaining, modifiers)
}

fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let c = match raw {
        "esc" => KeyCode::Esc,
        "enter" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "space" => KeyCode::Char(' '),
        "hyphen" => KeyCode::Char('-'),
        "minus" => KeyCode::Char('-'),
        "tab" => KeyCode::Tab,
        c if c.len() == 1 => {
            let mut c = c.chars().next().unwrap();
            if modifiers.contains(KeyModifiers::SHIFT) {
                c = c.to_ascii_uppercase();
            }
            KeyCode::Char(c)
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };
    Ok(KeyEvent::new(c, modifiers))
}

fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        match current {
            rest if rest.starts_with("ctrl-") => {
                modifiers.insert(KeyModifiers::CONTROL);
                current = &rest[5..];
            }
            rest if rest.starts_with("alt-") => {
                modifiers.insert(KeyModifiers::ALT);
                current = &rest[4..];
            }
            rest if rest.starts_with("shift-") => {
                modifiers.insert(KeyModifiers::SHIFT);
                current = &rest[6..];
            }
            _ => break, // break out of the loop if no known prefix is detected
        };
    }

    (current, modifiers)
}
