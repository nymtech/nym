// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[derive(Clone, Copy, Debug)]
pub enum Platform {
    Apple,
    Unspecified,
}
impl Platform {
    pub fn api_path_component(&self) -> &'static str {
        match self {
            Platform::Apple => crate::routes::APPLE,
            Platform::Unspecified => "",
        }
    }
}
