// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The simulator, run entirely in memory.
//!
//! Every one of these carries a message through the real client pipelines and the real nym-node
//! data pipeline - three mix layers and two gateways of it - without binding a socket.

use std::net::SocketAddr;

use nym_mix_sim::sim::SimHarness;

/// Enough ticks for a message to cross gateway, three mix layers, gateway and land.
const ENOUGH_TICKS: usize = 40;

#[tokio::test]
async fn a_message_reaches_its_recipient() -> anyhow::Result<()> {
    let mut sim = SimHarness::builder().seed([7; 32]).build().await?;
    let (alice, bob) = (sim.clients()[0], sim.clients()[1]);

    sim.send(alice, bob, b"hello bob")?;
    let delivered = sim.step_until(ENOUGH_TICKS, |log| !log.for_client(bob).is_empty());

    assert!(
        delivered,
        "nothing reached client {bob} in {ENOUGH_TICKS} ticks"
    );
    assert_eq!(sim.delivered_to(bob), vec![b"hello bob".to_vec()]);
    assert_eq!(sim.unroutable(), 0, "some datagram went nowhere");

    Ok(())
}

#[tokio::test]
async fn the_same_seed_runs_the_same_way() -> anyhow::Result<()> {
    async fn run() -> anyhow::Result<(Vec<Vec<u8>>, Vec<(SocketAddr, SocketAddr)>, usize)> {
        let mut sim = SimHarness::builder().seed([7; 32]).build().await?;
        let (alice, bob) = (sim.clients()[0], sim.clients()[1]);

        sim.send(alice, bob, b"hello bob")?;
        sim.step_until(ENOUGH_TICKS, |log| !log.for_client(bob).is_empty());

        Ok((sim.delivered_to(bob), sim.route(), sim.ticks()))
    }

    let (first_payloads, first_route, first_ticks) = run().await?;
    let (second_payloads, second_route, second_ticks) = run().await?;

    assert_eq!(first_payloads, second_payloads);
    assert_eq!(
        first_ticks, second_ticks,
        "delivery took a different number of ticks"
    );
    assert_eq!(
        first_route, second_route,
        "the packets took a different route"
    );

    // a route that is only stable because it is empty would pass all three
    assert!(!first_route.is_empty());

    Ok(())
}

#[tokio::test]
async fn a_message_too_big_for_one_packet_is_reassembled() -> anyhow::Result<()> {
    let mut sim = SimHarness::builder().seed([7; 32]).build().await?;
    let (alice, bob) = (sim.clients()[0], sim.clients()[1]);

    // several sphinx payloads' worth, so the chunking stage has to split and the receiver has to
    // put it back together
    let long: Vec<u8> = (0..8192u32).map(|i| i as u8).collect();

    sim.send(alice, bob, &long)?;
    let delivered = sim.step_until(ENOUGH_TICKS, |log| !log.for_client(bob).is_empty());

    assert!(delivered, "the long message never arrived");
    assert_eq!(sim.delivered_to(bob), vec![long]);

    Ok(())
}
