// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod utils;

// Future consideration: currently in a lot of places, the payloads have randomised content
// which is not a perfect testing strategy as it might not detect some edge cases I never would
// have assumed could be possible. A better approach would be to research some Fuzz testing
// library like: https://github.com/rust-fuzz/afl.rs and use that instead for the inputs.

// perhaps it might be useful down the line for interaction testing between client,mixes,etc?

// re-export sub-crates
pub use nymsphinx_acknowledgements as acknowledgements;
pub use nymsphinx_addressing as addressing;
pub use nymsphinx_chunking as chunking;
pub use nymsphinx_framing as framing;
pub use nymsphinx_types::*;
