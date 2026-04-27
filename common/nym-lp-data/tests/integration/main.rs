// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::clients::{traits::ClientWrappingPipeline, types::Pipeline};

use crate::common::{
    KcpReliability, KekwObfuscation, LpFraming, LpTransport, MockChunking, SphinxSecurity,
};

mod common;

#[test]
fn empty_input_yields_empty_output() {
    let packet_size = 64;
    let security_layer_nb_frames = 2;

    let mut mock_pipeline = Pipeline {
        chunking: MockChunking,
        reliability: KcpReliability,
        security: SphinxSecurity {
            nb_frames: security_layer_nb_frames,
        },
        obfuscation: KekwObfuscation,
        framing: LpFraming,
        transport: LpTransport,
        packet_size,
    };

    let output = mock_pipeline.process(None, 1);

    assert!(output.is_empty());
}

// TODO More test to come later
