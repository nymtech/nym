// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `zcash-sync` — Zcash compact-block sync over a two-hop dVPN tunnel.
//!
//! Times syncing the last N compact blocks (default 10_000, `--blocks <N>`) from
//! a public `lightwalletd` (`zec.rocks:443`, gRPC over TLS) both directly and
//! through a two-hop tunnel, and compares the throughput. Uses a hand-written
//! `tonic` gRPC client (no proto/build.rs): only the message fields needed to
//! drive `GetLatestBlock` and count `GetBlockRange` are declared.
//!
//! Build with `--release`: boringtun is much slower in debug, which dominates the
//! through-tunnel timing at the default block count.
//!
//! Usage:
//!   MNEMONIC="<funded mnemonic>" \
//!   cargo run --release -p nym-smoldvpn --example zcash-sync [-- --blocks <N> <options>]

use std::process::ExitCode;
use std::time::{Duration, Instant};

use http::uri::PathAndQuery;
use nym_smoldvpn::Tunnel;
use tonic::client::Grpc;
use tonic::transport::{Channel, Endpoint};
use tonic::Request;
use tonic_prost::ProstCodec;
use tracing::{error, info};

#[path = "common/mod.rs"]
mod common;
use common::{BoxError, DirectConnector, TlsWrap};

const LWD: &str = "zec.rocks";
const DEFAULT_BLOCKS: u64 = 10_000;
const GET_LATEST_BLOCK: &str = "/cash.z.wallet.sdk.rpc.CompactTxStreamer/GetLatestBlock";
const GET_BLOCK_RANGE: &str = "/cash.z.wallet.sdk.rpc.CompactTxStreamer/GetBlockRange";

// --- Minimal lightwalletd protocol messages (only the fields we use) --------

#[derive(Clone, PartialEq, prost::Message)]
struct ChainSpec {}

#[derive(Clone, PartialEq, prost::Message)]
struct BlockId {
    #[prost(uint64, tag = "1")]
    height: u64,
    #[prost(bytes = "vec", tag = "2")]
    hash: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct BlockRange {
    #[prost(message, optional, tag = "1")]
    start: Option<BlockId>,
    #[prost(message, optional, tag = "2")]
    end: Option<BlockId>,
}

// We only need to count streamed blocks, so decode just the height. In
// lightwalletd's compact_formats.proto `height` is field 2 (field 4 is
// `prevHash`, a length-delimited bytes field — decoding it as a varint fails).
#[derive(Clone, PartialEq, prost::Message)]
struct CompactBlock {
    #[prost(uint64, tag = "2")]
    height: u64,
}

/// Build a TLS gRPC channel to `zec.rocks:443` using `connector` as the
/// underlying transport (direct socket or tunnel socket).
async fn grpc_channel<C, S>(connector: C) -> Result<Channel, BoxError>
where
    C: tower::Service<http::Uri, Response = hyper_util::rt::TokioIo<S>> + Clone + Send + 'static,
    C::Future: Send + 'static,
    C::Error: Into<BoxError>,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let channel = Endpoint::from_static("https://zec.rocks:443")
        .connect_with_connector(TlsWrap::h2(connector))
        .await?;
    Ok(channel)
}

/// Sync the last `n_blocks` compact blocks over `channel`; returns
/// (blocks_streamed, elapsed) timing only the block-range streaming.
async fn sync_last_blocks(channel: Channel, n_blocks: u64) -> Result<(u64, Duration), BoxError> {
    let mut grpc = Grpc::new(channel);
    grpc.ready().await?;

    // Chain tip.
    let latest: BlockId = grpc
        .unary(
            Request::new(ChainSpec {}),
            PathAndQuery::from_static(GET_LATEST_BLOCK),
            ProstCodec::<ChainSpec, BlockId>::default(),
        )
        .await?
        .into_inner();
    let top = latest.height;
    let start = top.saturating_sub(n_blocks.saturating_sub(1));

    let range = BlockRange {
        start: Some(BlockId {
            height: start,
            hash: Vec::new(),
        }),
        end: Some(BlockId {
            height: top,
            hash: Vec::new(),
        }),
    };

    grpc.ready().await?;
    let t0 = Instant::now();
    let mut stream = grpc
        .server_streaming(
            Request::new(range),
            PathAndQuery::from_static(GET_BLOCK_RANGE),
            ProstCodec::<BlockRange, CompactBlock>::default(),
        )
        .await?
        .into_inner();

    let mut count = 0u64;
    while let Some(_block) = stream.message().await? {
        count += 1;
    }
    Ok((count, t0.elapsed()))
}

fn report(label: &str, blocks: u64, elapsed: Duration) {
    let secs = elapsed.as_secs_f64();
    let rate = if secs > 0.0 {
        blocks as f64 / secs
    } else {
        0.0
    };
    info!("{label}: {blocks} blocks in {secs:.2}s ({rate:.0} blocks/s)");
}

async fn run() -> Result<(), BoxError> {
    common::init_logging();
    common::init_crypto();
    let cli = common::parse_cli()?;

    // 1. Real IP + baseline sync (no tunnel).
    info!(
        "real IP (no tunnel): {}",
        common::fmt_ipinfo(&common::ipinfo_direct().await?)
    );
    let n_blocks = cli.blocks.unwrap_or(DEFAULT_BLOCKS);
    info!("syncing last {n_blocks} blocks from {LWD} directly …");
    let (out_blocks, out_time) =
        sync_last_blocks(grpc_channel(DirectConnector).await?, n_blocks).await?;
    report("direct", out_blocks, out_time);

    // 2. Provision + bring up the requested tunnel.
    info!("provisioning a {} tunnel …", common::describe(&cli));
    let session = common::new_session("zcash-sync").await;
    let result = async {
        // Register (cache-served when possible) + bring up the tunnel, gated
        // on WireGuard establishment with stale-cache fallback.
        let (reg, tunnel): (_, Tunnel) = common::connect(&session, &cli).await?;
        common::print_gateway("entry", &reg.entry.gateway);
        if let Some(exit) = reg.exit.as_ref() {
            common::print_gateway("exit", &exit.gateway);
        }

        // Exit IP display probe (the tunnel is already established).
        let ip = common::ipinfo_display(&tunnel).await?;
        info!("IP through the tunnel: {}", common::fmt_ipinfo(&ip));

        // 3. Sync the same range through the tunnel.
        info!("syncing last {n_blocks} blocks from {LWD} through the tunnel …");
        let (in_blocks, in_time) =
            sync_last_blocks(grpc_channel(tunnel.connector()).await?, n_blocks).await?;
        report("tunnel", in_blocks, in_time);

        // 4. Comparison.
        let slowdown = in_time.as_secs_f64() / out_time.as_secs_f64().max(1e-9);
        info!("comparison:");
        report("direct", out_blocks, out_time);
        report("tunnel", in_blocks, in_time);
        info!("tunnel took {slowdown:.2}x the direct time");

        let _ = tokio::time::timeout(Duration::from_secs(5), tunnel.shutdown()).await;
        Ok::<(), BoxError>(())
    }
    .await;

    // Close the session's credential store cleanly (checkpoints the sqlite WAL;
    // stored tickets are retained) whether the flow succeeded or failed.
    session.shutdown().await;
    result?;

    info!("PASS: synced {n_blocks} blocks inside and outside the tunnel");
    std::process::exit(0);
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        error!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
