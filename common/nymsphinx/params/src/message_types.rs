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

// Perhaps this idea could be extended with some binary masking to convey more information
// like to help the receiver to know whether the message is expected to be text-based or binary
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum MessageType {
    WithoutReplySURB = 0,
    WithReplySURB = 1,
    IsReply = 2,
}
