use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, Criterion, criterion_group, criterion_main};

use crypto::asymmetric::{encryption, identity};
use crypto::asymmetric::encryption::{KeyPair, PrivateKey};
use crypto::asymmetric::identity::PublicKey;
use mixnet_contract_common::Layer;
use nymsphinx::{delays, Node, NODE_ADDRESS_LENGTH, NodeAddressBytes, NymsphinxPayloadBuilder, PAYLOAD_OVERHEAD_SIZE, SphinxPacket};
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::acknowledgements::surb_ack::SurbAck;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::builder::SphinxPacketBuilder;
use nymsphinx::chunking::fragment::{Fragment, FragmentHeader, FragmentIdentifier};
use nymsphinx::cover::generate_loop_cover_packet;
use nymsphinx::crypto::keygen;
use nymsphinx::params::packet_sizes::PacketSize::RegularPacket;
use nymsphinx::params::PacketSize;
use topology::{gateway, mix, MixLayer, NymTopology};

const REGULAR_PACKET_SIZE: usize = PAYLOAD_OVERHEAD_SIZE + 2 * 1024;
const EXTENDED_PACKET_SIZE_8: usize = PAYLOAD_OVERHEAD_SIZE + 8 * 1024;
const EXTENDED_PACKET_SIZE_16: usize = PAYLOAD_OVERHEAD_SIZE + 16 * 1024;
const EXTENDED_PACKET_SIZE_32: usize = PAYLOAD_OVERHEAD_SIZE + 32 * 1024;

struct BenchCase {
    packet_size: PacketSize,
}

fn feature_topology(sender_gateway_id: PublicKey, recipient_gateway_id: PublicKey) -> (NymTopology, KeyPair) {
    let mut rng = rand::thread_rng();

    let gateway1 = gateway::Node {
        owner: "N/A".to_string(),
        stake: 1000,
        location: "N/A".to_string(),
        host: "1.1.1.1".parse().unwrap(),
        mix_host: "1.1.1.1:1789".parse().unwrap(),
        clients_port: 8888,
        identity_key: sender_gateway_id,
        sphinx_key: encryption::PublicKey::from_base58_string(
            "C7cown6dYCLZpLiMFC1PaBmhvLvmJmLDJGeRTbPD45bX",
        )
            .unwrap(),
        version: "0.x.0".to_string(),
    };

    let gateway2 = gateway::Node {
        identity_key: recipient_gateway_id,
        ..gateway1.clone()
    };

    let node1_enc_keys = KeyPair::new(&mut rng);
    let node1 = mix::Node {
        mix_id: 42,
        owner: "N/A".to_string(),
        host: "3.3.3.3".parse().unwrap(),
        mix_host: "3.3.3.3:1789".parse().unwrap(),
        identity_key: identity::PublicKey::from_base58_string(
            "3ebjp1Fb9hdcS1AR6AZihgeJiMHkB5jjJUsvqNnfQwU7",
        )
            .unwrap(),
        sphinx_key: *node1_enc_keys.public_key(),
        layer: Layer::One,
        version: "0.x.0".to_string(),
    };

    let node2 = mix::Node {
        owner: "Alice".to_string(),
        ..node1.clone()
    };

    let node3 = mix::Node {
        owner: "Bob".to_string(),
        ..node1.clone()
    };

    let mut mixes: HashMap<MixLayer, Vec<mix::Node>> = HashMap::new();
    mixes.insert(1, vec![node1]);
    mixes.insert(2, vec![node2]);
    mixes.insert(3, vec![node3]);

    let topology = NymTopology::new(mixes, vec![gateway1, gateway2]);
    (topology, node1_enc_keys)
}

fn make_packet_copy(packet: &SphinxPacket) -> SphinxPacket {
    SphinxPacket::from_bytes(&packet.to_bytes()).unwrap()
}

fn bench_loop_packet_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-sphinx");
    // group.sample_size(200);
    group.measurement_time(Duration::from_secs(500));

    let mut rng = rand::thread_rng();
    let case = BenchCase {
        packet_size: RegularPacket,
    };

    // create sender
    let sender_client_id_pair = identity::KeyPair::new(&mut rng);
    let sender_client_enc_pair = encryption::KeyPair::new(&mut rng);
    let sender_gateway_id_pair = identity::KeyPair::new(&mut rng);
    let packet_sender = Recipient::new(
        *sender_client_id_pair.public_key(),
        *sender_client_enc_pair.public_key(),
        *sender_gateway_id_pair.public_key(),
    );

    // build topology
    let (topology, node_keypair) = feature_topology(*sender_gateway_id_pair.public_key(), *sender_gateway_id_pair.public_key());
    // generate the encryption key for the ack
    let ack_key = AckKey::new(&mut rng);

    group.bench_function(
        &format!(
            "[Sphinx] create_loop_cover_packet_with_payload_size_{}",
            case.packet_size.payload_size(),
        ),
        |b| {
            b.iter(|| {
                generate_loop_cover_packet(
                    &mut rng,
                    &topology,
                    &ack_key,
                    &packet_sender,
                    Duration::from_millis(50),
                    Duration::from_millis(50),
                    case.packet_size)
            })
        },
    );

    // let's create the packet to later benchmark the processing
    let packet = generate_loop_cover_packet(
        &mut rng,
        &topology,
        &ack_key,
        &packet_sender,
        Duration::from_millis(50),
        Duration::from_millis(50),
        case.packet_size).unwrap();

    group.bench_function(
        &format!(
            "[Sphinx] process_loop_cover_packet_with_payload_size_{}",
            case.packet_size.payload_size(),
        ),
        |b| {
            b.iter(|| {
                make_packet_copy(&packet.sphinx_packet).process(&node_keypair.private_key().into())
            })
        },
    );
    // let new_packet = packet.sphinx_packet.process(&node_keypair.private_key().into());
}

fn bench_new_no_surb(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-sphinx");
    // group.sample_size(200);
    group.measurement_time(Duration::from_secs(500));

    let mut rng = rand::thread_rng();
    let case = BenchCase {
        packet_size: RegularPacket,
    };

    // create sender
    let sender_client_id_pair = identity::KeyPair::new(&mut rng);
    let sender_client_enc_pair = encryption::KeyPair::new(&mut rng);
    let sender_gateway_id_pair = identity::KeyPair::new(&mut rng);
    let packet_sender = Recipient::new(
        *sender_client_id_pair.public_key(),
        *sender_client_enc_pair.public_key(),
        *sender_gateway_id_pair.public_key(),
    );

    // create recipient
    let recipient_client_id_pair = identity::KeyPair::new(&mut rng);
    let recipient_client_enc_pair = encryption::KeyPair::new(&mut rng);
    let recipient_gateway_id_pair = identity::KeyPair::new(&mut rng);
    let packet_recipient = Recipient::new(
        *recipient_client_id_pair.public_key(),
        *recipient_client_enc_pair.public_key(),
        *recipient_gateway_id_pair.public_key(),
    );

    // build topology
    let (topology, node_keypair) = feature_topology(*sender_gateway_id_pair.public_key(), *recipient_gateway_id_pair.public_key());

    // generate pseudorandom route for the packet
    let route = topology.random_route_to_gateway(
        &mut rng,
        3,
        packet_recipient.gateway(),
    ).unwrap();

    // generate some payload
    let mlen = 40;
    let mut msg = vec![0u8; mlen];
    let fragment = Fragment {
        header: FragmentHeader::try_new(
            12345,
            u8::max_value(),
            u8::max_value(),
            None,
            Some(1234),
        )
            .unwrap(),
        payload: msg,
    };


    let ack_key = AckKey::new(&mut rng);
    let surb_ack = SurbAck::construct(
        &mut rng,
        &packet_sender,
        &ack_key,
        fragment.fragment_identifier().to_bytes(),
        Duration::from_millis(50),
        &topology,
    ).unwrap();


    let packet_payload = NymsphinxPayloadBuilder::new(fragment, surb_ack)
        .build_regular(&mut rng, packet_recipient.encryption_key());

    let delays = delays::generate_from_average_duration(route.len(), Duration::from_millis(50));
    let destination = packet_recipient.as_sphinx_destination();

    group.bench_function(
        &format!(
            "[Sphinx] create_packet_no_reply_surbs_with_payload_size_{}",
            case.packet_size.payload_size(),
        ),
        |b| {
            b.iter(|| {
                SphinxPacketBuilder::new()
                    .with_payload_size(case.packet_size.payload_size())
                    .build_packet(packet_payload.clone(), &route, &destination, &delays)
            })
        },
    );

    // let's create the packet to later benchmark the processing
    let sphinx_packet = SphinxPacketBuilder::new()
        .with_payload_size(case.packet_size.payload_size())
        .build_packet(packet_payload.clone(), &route, &destination, &delays)
        .unwrap();

    group.bench_function(
        &format!(
            "[Sphinx] process_packet_with_payload_size_{}",
            case.packet_size.payload_size(),
        ),
        |b| {
            b.iter(|| {
                make_packet_copy(&sphinx_packet).process(&node_keypair.private_key().into())
            })
        },
    );
}

criterion_group!(sphinx, bench_loop_packet_create, bench_new_no_surb);
criterion_main!(sphinx);