use std::{
    thread::{self, sleep},
    time::Duration,
};

use nym_lp::packet::utils::format_debug_bytes;
use nym_lp_data::traits::{Pipeline, PipelineDriver, ProcessingPipeline, StreamOptions};

use crate::common::{
    KcpReliability, KekwObfuscation, LpFraming, LpTransport, MockChunking, ReallyOddObfuscation,
    SphinxSecurity,
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

    let options = StreamOptions {
        reliability: true,
        security: true,
        obfuscation: true,
    };

    let output = mock_pipeline.process(Vec::new(), options, 1);

    assert!(output.is_empty());
}

#[test]
fn testy_mc_testface() {
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

    let options = StreamOptions {
        reliability: true,
        security: true,
        obfuscation: true,
    };

    let input = "Never gonna give you up
Never gonna let you down
Never gonna run around
And desert you
Never gonna make you cry
Never gonna say goodbye
Never gonna tell a lie
And hurt you
"
    .as_bytes()
    .to_vec();
    println!("input:");
    println!("{}", format_debug_bytes(&input).unwrap());

    let output = mock_pipeline.process(input, options, 1);
    println!("output:");
    println!("{output:#?}");
}

#[test]
fn driver_test() {
    let packet_size = 64;
    let security_layer_nb_frames = 2;

    let mut timestamp = 2;

    let mock_pipeline = Pipeline {
        chunking: MockChunking,
        reliability: KcpReliability,
        security: SphinxSecurity {
            nb_frames: security_layer_nb_frames,
        },
        obfuscation: ReallyOddObfuscation::new(timestamp),
        framing: LpFraming,
        transport: LpTransport,
        packet_size,
    };

    let options = StreamOptions {
        reliability: true,
        security: true,
        obfuscation: true,
    };

    let mut mock_pipeline_driver =
        PipelineDriver::new(mock_pipeline).with_processing_options(options);

    let input_sender = mock_pipeline_driver.input_sender();
    thread::spawn(move || {
        let input = [vec![b'a'; 68], vec![b'b'; 4], vec![b'c'; 100]];
        println!("input:");
        for pkt in &input {
            println!("{}", format_debug_bytes(pkt).unwrap());
        }
        for payload in input {
            input_sender.send(payload).unwrap()
        }
    });

    loop {
        sleep(Duration::from_millis(10));
        let output = mock_pipeline_driver.tick(timestamp);
        println!("timestamp : {timestamp}");
        if !output.is_empty() {
            println!("output:");
            for pkt in output {
                println!("{pkt:#?}");
            }
        }
        timestamp += 1;
        if timestamp > 20 {
            break;
        }
    }
}
