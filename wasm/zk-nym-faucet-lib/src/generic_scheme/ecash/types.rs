// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::wasm_wrapper;
use nym_compact_ecash::GroupParameters;
use wasm_bindgen::prelude::wasm_bindgen;

wasm_wrapper!(GroupParameters, GroupParametersWrapper);
