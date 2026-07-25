// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A minimal hand-rolled lightwalletd gRPC client.
//!
//! Only the three calls this example needs are implemented, against
//! hand-written `prost` messages rather than generated code. That keeps the
//! example free of a `build.rs` and a `protoc` install; `prost` skips unknown
//! fields on decode, so declaring a subset of each message's fields is safe
//! and forward-compatible.
//!
//! Field numbers and the service path are from lightwalletd's
//! `compact_formats.proto` / `service.proto`
//! (<https://github.com/zcash/lightwalletd>).

use tonic::codegen::http::uri::PathAndQuery;
use tonic::transport::Channel;
use tonic::{Request, Status};
use tonic_prost::ProstCodec;

const GET_LATEST_BLOCK: &str = "/cash.z.wallet.sdk.rpc.CompactTxStreamer/GetLatestBlock";
const GET_BLOCK_RANGE: &str = "/cash.z.wallet.sdk.rpc.CompactTxStreamer/GetBlockRange";

/// `ChainSpec` — an intentionally empty request message.
#[derive(Clone, PartialEq, prost::Message)]
pub struct ChainSpec {}

/// `BlockID`: a height, a hash, or both.
#[derive(Clone, PartialEq, prost::Message)]
pub struct BlockId {
    #[prost(uint64, tag = "1")]
    pub height: u64,
    #[prost(bytes = "vec", tag = "2")]
    pub hash: Vec<u8>,
}

impl BlockId {
    fn at(height: u64) -> Self {
        Self {
            height,
            hash: Vec::new(),
        }
    }
}

/// `BlockRange`. Note lightwalletd treats both ends as **inclusive**.
#[derive(Clone, PartialEq, prost::Message)]
pub struct BlockRange {
    #[prost(message, optional, tag = "1")]
    pub start: Option<BlockId>,
    #[prost(message, optional, tag = "2")]
    pub end: Option<BlockId>,
}

/// A shielded transaction inside a compact block (subset of `CompactTx`).
#[derive(Clone, PartialEq, prost::Message)]
pub struct CompactTx {
    #[prost(uint64, tag = "1")]
    pub index: u64,
    #[prost(bytes = "vec", tag = "2")]
    pub hash: Vec<u8>,
}

/// A compact block (subset of `CompactBlock`; other fields are skipped on
/// decode).
#[derive(Clone, PartialEq, prost::Message)]
pub struct CompactBlock {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub hash: Vec<u8>,
    #[prost(message, repeated, tag = "7")]
    pub vtx: Vec<CompactTx>,
}

/// What one `GetBlockRange` call returned.
#[derive(Debug, Default)]
pub struct FetchedRange {
    /// Heights received, in the order the server streamed them.
    pub heights: Vec<u64>,
    /// Total shielded transactions across those blocks.
    pub transactions: usize,
}

/// A lightwalletd client. Cloning is cheap — the underlying [`Channel`] is
/// reference-counted and multiplexes over one HTTP/2 connection.
#[derive(Clone, Debug)]
pub struct Lightwalletd {
    inner: tonic::client::Grpc<Channel>,
}

impl Lightwalletd {
    /// Connect to a lightwalletd endpoint (e.g. `https://zec.rocks:443`).
    pub async fn connect(endpoint: String) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_shared(endpoint)?
            .tls_config(tonic::transport::ClientTlsConfig::new().with_webpki_roots())?
            .connect()
            .await?;
        Ok(Self {
            inner: tonic::client::Grpc::new(channel),
        })
    }

    /// Current chain tip height.
    pub async fn tip(&mut self) -> Result<u64, Status> {
        self.inner.ready().await.map_err(connect_error)?;
        let response = self
            .inner
            .unary(
                Request::new(ChainSpec {}),
                PathAndQuery::from_static(GET_LATEST_BLOCK),
                ProstCodec::<ChainSpec, BlockId>::default(),
            )
            .await?;
        Ok(response.into_inner().height)
    }

    /// Stream every compact block in the half-open range `[start, end)`.
    ///
    /// The half-open range is converted to lightwalletd's inclusive
    /// `BlockRange` on the wire.
    pub async fn block_range(&mut self, start: u64, end: u64) -> Result<FetchedRange, Status> {
        debug_assert!(start < end, "empty range {start}..{end}");
        let request = BlockRange {
            start: Some(BlockId::at(start)),
            end: Some(BlockId::at(end - 1)),
        };

        self.inner.ready().await.map_err(connect_error)?;
        let mut stream = self
            .inner
            .server_streaming(
                Request::new(request),
                PathAndQuery::from_static(GET_BLOCK_RANGE),
                ProstCodec::<BlockRange, CompactBlock>::default(),
            )
            .await?
            .into_inner();

        let mut fetched = FetchedRange::default();
        while let Some(block) = stream.message().await? {
            fetched.transactions += block.vtx.len();
            fetched.heights.push(block.height);
        }
        Ok(fetched)
    }
}

fn connect_error(e: impl std::fmt::Display) -> Status {
    Status::unavailable(format!("lightwalletd connection not ready: {e}"))
}
