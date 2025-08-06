// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::Id;
use itertools::Itertools;
use std::fmt::Display;

// from the docs:
// For channel messages, either the name or integer ID of the channel.
// For direct messages, either a list containing integer user IDs
// or a list containing string Zulip API email addresses.
pub enum ToDirect {
    ByIds(Vec<Id>),
    ByNames(Vec<String>),
}

impl Display for ToDirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToDirect::ByIds(ids) => write!(f, "[{}]", ids.iter().join(",")),
            ToDirect::ByNames(names) => {
                write!(f, "[{}]", names.join(","))
            }
        }
    }
}

impl From<Vec<String>> for ToDirect {
    fn from(names: Vec<String>) -> Self {
        ToDirect::ByNames(names)
    }
}

impl From<&[String]> for ToDirect {
    fn from(names: &[String]) -> Self {
        names.to_vec().into()
    }
}

impl From<&[&str]> for ToDirect {
    fn from(names: &[&str]) -> Self {
        names
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into()
    }
}

impl<const N: usize> From<&[&str; N]> for ToDirect {
    fn from(names: &[&str; N]) -> Self {
        names.as_slice().into()
    }
}

impl From<Vec<&str>> for ToDirect {
    fn from(names: Vec<&str>) -> Self {
        names.as_slice().into()
    }
}

impl From<String> for ToDirect {
    fn from(name: String) -> Self {
        ToDirect::ByNames(vec![name])
    }
}

impl From<&str> for ToDirect {
    fn from(name: &str) -> Self {
        name.to_string().into()
    }
}

impl From<Id> for ToDirect {
    fn from(id: Id) -> Self {
        ToDirect::ByIds(vec![id])
    }
}

impl From<&[Id]> for ToDirect {
    fn from(ids: &[Id]) -> Self {
        ids.to_vec().into()
    }
}

impl<const N: usize> From<&[Id; N]> for ToDirect {
    fn from(ids: &[Id; N]) -> Self {
        ids.as_slice().into()
    }
}

impl From<Vec<Id>> for ToDirect {
    fn from(ids: Vec<Id>) -> Self {
        ToDirect::ByIds(ids)
    }
}

pub enum ToChannel {
    ByName(String),
    ById(Id),
}

impl Display for ToChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToChannel::ByName(name) => name.fmt(f),
            ToChannel::ById(id) => id.fmt(f),
        }
    }
}

impl From<String> for ToChannel {
    fn from(name: String) -> Self {
        ToChannel::ByName(name)
    }
}

impl From<&str> for ToChannel {
    fn from(name: &str) -> Self {
        name.to_string().into()
    }
}

impl From<Id> for ToChannel {
    fn from(id: Id) -> Self {
        ToChannel::ById(id)
    }
}
