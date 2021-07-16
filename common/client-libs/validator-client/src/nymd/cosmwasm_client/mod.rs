// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod client;
mod signing_client;
pub mod types;

// pub enum CosmWasmClient {
//     SigningClient(SigningCosmWasmClient),
//     QueryClient(QueryCosmWasmClient),
// }
//
// impl CosmWasmClient {
//     pub fn connect_with_signer() -> Self {
//         todo!()
//     }
//
//     pub fn connect() -> Self {
//         todo!()
//     }
// }

// I initially had a super neat trait-based implementation that got rid of so much duplicate code
// but unfortunately async traits are not mature enough to handle the case of wallet being !Send

// pub struct CosmWasmClient<T> {
//     inner_client: T,
// }
//
// impl<T> CosmWasmClient<T> {
//     pub fn connect_with_signer() -> CosmWasmClient<signing_client::Client> {
//         todo!()
//     }
//
//     pub fn connect() -> CosmWasmClient<client::Client> {
//         todo!()
//     }
// }
//
// impl<T: SigningCosmWasmClient> CosmWasmClient<T> {}
//
// impl<T: QueryCosmWasmClient> CosmWasmClient<T> {}
