// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::LazyLock;

use crate::error::NymTuiError;
#[cfg(feature = "logger")]
use tui_logger::TuiWidgetEvent;

static KEY_MODIFIERS: LazyLock<HashMap<&'static str, KeyModifiers>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("shift", KeyModifiers::SHIFT);
    m.insert("ctrl", KeyModifiers::CONTROL);
    m.insert("alt", KeyModifiers::ALT);
    m.insert("super", KeyModifiers::SUPER);
    m.insert("hyper", KeyModifiers::HYPER);
    m.insert("meta", KeyModifiers::META);
    m
});

static SPECIAL_KEYS: LazyLock<HashMap<&'static str, KeyCode>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("backspace", KeyCode::Backspace);
    m.insert("enter", KeyCode::Enter);
    m.insert("left", KeyCode::Left);
    m.insert("right", KeyCode::Right);
    m.insert("up", KeyCode::Up);
    m.insert("down", KeyCode::Down);
    m.insert("home", KeyCode::Home);
    m.insert("end", KeyCode::End);
    m.insert("pageup", KeyCode::PageUp);
    m.insert("pagedown", KeyCode::PageDown);
    m.insert("tab", KeyCode::Tab);
    m.insert("backtab", KeyCode::BackTab);
    m.insert("delete", KeyCode::Delete);
    m.insert("insert", KeyCode::Insert);
    m.insert("null", KeyCode::Null);
    m.insert("esc", KeyCode::Esc);
    m.insert("space", KeyCode::Char(' '));
    m.insert("f1", KeyCode::F(1));
    m.insert("f2", KeyCode::F(2));
    m.insert("f3", KeyCode::F(3));
    m.insert("f4", KeyCode::F(4));
    m.insert("f5", KeyCode::F(5));
    m.insert("f6", KeyCode::F(6));
    m.insert("f7", KeyCode::F(7));
    m.insert("f8", KeyCode::F(8));
    m.insert("f9", KeyCode::F(9));
    m.insert("f10", KeyCode::F(10));
    m.insert("f11", KeyCode::F(11));
    m.insert("f12", KeyCode::F(12));
    m
});

#[cfg(feature = "logger")]
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct LoggerKeybindings {
    // TODO: give them better names
    tui_logger_space_key: KeyBinding,
    tui_logger_up_key: KeyBinding,
    tui_logger_down_key: KeyBinding,
    tui_logger_left_key: KeyBinding,
    tui_logger_right_key: KeyBinding,
    tui_logger_plus_key: KeyBinding,
    tui_logger_minus_key: KeyBinding,
    tui_logger_hide_key: KeyBinding,
    tui_logger_focus_key: KeyBinding,
    tui_logger_prev_page_key: KeyBinding,
    tui_logger_next_page_key: KeyBinding,
    tui_logger_escape_key: KeyBinding,
}

#[cfg(feature = "logger")]
impl Default for LoggerKeybindings {
    fn default() -> Self {
        LoggerKeybindings {
            tui_logger_space_key: KeyBinding::new(KeyCode::Char(' ')),
            tui_logger_up_key: KeyBinding::new(KeyCode::Up),
            tui_logger_down_key: KeyBinding::new(KeyCode::Down),
            tui_logger_left_key: KeyBinding::new(KeyCode::Left),
            tui_logger_right_key: KeyBinding::new(KeyCode::Right),
            tui_logger_plus_key: KeyBinding::new(KeyCode::Char('+')),
            tui_logger_minus_key: KeyBinding::new(KeyCode::Char('-')),
            tui_logger_hide_key: KeyBinding::new(KeyCode::Char('h')),
            tui_logger_focus_key: KeyBinding::new(KeyCode::Char('f')),
            tui_logger_prev_page_key: KeyBinding::new(KeyCode::PageUp),
            tui_logger_next_page_key: KeyBinding::new(KeyCode::PageDown),
            tui_logger_escape_key: KeyBinding::new(KeyCode::Esc),
        }
    }
}

#[cfg(feature = "logger")]
impl LoggerKeybindings {
    pub fn tui_logger_event(&self, key: KeyBinding) -> Option<TuiWidgetEvent> {
        if key == self.tui_logger_space_key {
            Some(TuiWidgetEvent::SpaceKey)
        } else if key == self.tui_logger_up_key {
            Some(TuiWidgetEvent::UpKey)
        } else if key == self.tui_logger_down_key {
            Some(TuiWidgetEvent::DownKey)
        } else if key == self.tui_logger_left_key {
            Some(TuiWidgetEvent::LeftKey)
        } else if key == self.tui_logger_right_key {
            Some(TuiWidgetEvent::RightKey)
        } else if key == self.tui_logger_plus_key {
            Some(TuiWidgetEvent::PlusKey)
        } else if key == self.tui_logger_minus_key {
            Some(TuiWidgetEvent::MinusKey)
        } else if key == self.tui_logger_hide_key {
            Some(TuiWidgetEvent::HideKey)
        } else if key == self.tui_logger_focus_key {
            Some(TuiWidgetEvent::FocusKey)
        } else if key == self.tui_logger_prev_page_key {
            Some(TuiWidgetEvent::PrevPageKey)
        } else if key == self.tui_logger_next_page_key {
            Some(TuiWidgetEvent::NextPageKey)
        } else if key == self.tui_logger_escape_key {
            Some(TuiWidgetEvent::EscapeKey)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifier: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode) -> Self {
        KeyBinding {
            code,
            modifier: KeyModifiers::NONE,
        }
    }

    pub fn new_with_modifier(code: KeyCode, modifier: KeyModifiers) -> Self {
        KeyBinding { code, modifier }
    }
}

impl From<KeyEvent> for KeyBinding {
    fn from(value: KeyEvent) -> Self {
        KeyBinding {
            code: value.code,
            modifier: value.modifiers,
        }
    }
}

impl Display for KeyBinding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.modifier.contains(KeyModifiers::SHIFT) {
            write!(f, "shift+")?;
        }
        if self.modifier.contains(KeyModifiers::CONTROL) {
            write!(f, "ctrl+")?;
        }
        if self.modifier.contains(KeyModifiers::ALT) {
            write!(f, "alt+")?;
        }
        if self.modifier.contains(KeyModifiers::SUPER) {
            write!(f, "super+")?;
        }
        if self.modifier.contains(KeyModifiers::HYPER) {
            write!(f, "hyper+")?;
        }
        if self.modifier.contains(KeyModifiers::META) {
            write!(f, "meta+")?;
        }
        match self.code {
            KeyCode::Backspace => write!(f, "backspace"),
            KeyCode::Enter => write!(f, "enter"),
            KeyCode::Left => write!(f, "left"),
            KeyCode::Right => write!(f, "right"),
            KeyCode::Up => write!(f, "up"),
            KeyCode::Down => write!(f, "down"),
            KeyCode::Home => write!(f, "home"),
            KeyCode::End => write!(f, "end"),
            KeyCode::PageUp => write!(f, "pageup"),
            KeyCode::PageDown => write!(f, "pagedown"),
            KeyCode::Tab => write!(f, "tab"),
            KeyCode::BackTab => write!(f, "backtab"),
            KeyCode::Delete => write!(f, "delete"),
            KeyCode::Insert => write!(f, "insert"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Null => write!(f, "null"),
            KeyCode::Esc => write!(f, "esc"),
            _ => write!(f, "unknown"),
        }
    }
}

impl From<KeyBinding> for String {
    fn from(value: KeyBinding) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for KeyBinding {
    type Error = NymTuiError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for KeyBinding {
    type Err = NymTuiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.rsplit_once('+') {
            Some((modifiers, value)) => Ok(Self {
                code: parse_keycode(value)?,
                modifier: parse_modifiers(modifiers)?,
            }),
            None => Ok(Self {
                code: parse_keycode(s)?,
                modifier: KeyModifiers::NONE,
            }),
        }
    }
}

fn parse_keycode(value: &str) -> Result<KeyCode, NymTuiError> {
    Ok(if value.len() == 1 {
        KeyCode::Char(
            char::from_str(value)
                .map_err(|source| NymTuiError::InvalidCharacter {
                    str: value.to_string(),
                    source,
                })?
                .to_ascii_lowercase(),
        )
    } else {
        SPECIAL_KEYS
            .get(value)
            .cloned()
            .ok_or_else(|| NymTuiError::UnknownKeyBinding {
                value: value.to_string(),
            })?
    })
}

fn parse_modifiers(modifiers: &str) -> Result<KeyModifiers, NymTuiError> {
    modifiers
        .split('+')
        .try_fold(KeyModifiers::NONE, |modifiers, token| {
            KEY_MODIFIERS
                .get(token)
                .map(|modifier| modifiers | *modifier)
                .ok_or_else(|| NymTuiError::UnknownKeyModifier {
                    value: token.to_string(),
                })
        })
}

pub fn key_event_to_string(key_event: &KeyEvent) -> String {
    KeyBinding::from(*key_event).to_string()
}
