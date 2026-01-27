use libcrux_psq::{
    Channel,
    handshake::{
        RegistrationInitiator, Responder,
        builders::{CiphersuiteBuilder, PrincipalBuilder},
        ciphersuites::CiphersuiteName,
        types::{DHKeyPair, DHPrivateKey, DHPublicKey},
    },
};
use nym_crypto::asymmetric::x25519;
use nym_kkt::ciphersuite::{Ciphersuite, DecapsulationKey, EncapsulationKey};

const AAD_INITIATOR_OUTER: &[u8; 17] = b"Test Data I Outer";
const AAD_INITIATOR_INNER: &[u8; 17] = b"Test Data I Inner";
const AAD_RESPONDER: &[u8; 11] = b"Test Data R";
const SESSION_CONTEXT: &[u8; 12] = b"Test Context";

pub fn initiator_process(
    ciphersuite: &Ciphersuite,
    session_context: &[u8],
    inner_aad: &[u8],
    outer_aad: &[u8],
    local_x25519_private_key: &x25519::PrivateKey,
    remote_x25519_public: &x25519::PublicKey,
    remote_kem_public: &EncapsulationKey,
) -> Vec<u8> {
    //georgio: handle errors
    let initiator_libcrux_x25519_private_key =
        DHPrivateKey::from_bytes(local_x25519_private_key.as_bytes()).unwrap();

    let initiator_x25519_keypair = DHKeyPair::from(initiator_libcrux_x25519_private_key);

    let responder_x25519_public_key = DHPublicKey::from_bytes(remote_x25519_public.as_bytes());

    let initiator_cbuilder = match ciphersuite.kem() {
        nym_kkt::ciphersuite::KEM::MlKem768 => match remote_kem_public {
            EncapsulationKey::MlKem768(ml_kem_public_key) => CiphersuiteBuilder::new(
                CiphersuiteName::X25519_MLKEM768_X25519_CHACHA20POLY1305_HKDFSHA256,
            )
            .peer_longterm_mlkem_pk(ml_kem_public_key),
            _ => panic!(
                "wrong key type passed (remote_kem_public should be EncapsulationKey::MlKem768)"
            ),
        },
        nym_kkt::ciphersuite::KEM::McEliece => match remote_kem_public {
            EncapsulationKey::McEliece(mceliece_public_key) => CiphersuiteBuilder::new(
                CiphersuiteName::X25519_CLASSICMCELIECE_X25519_CHACHA20POLY1305_HKDFSHA256,
            )
            .peer_longterm_cmc_pk(mceliece_public_key),
            _ => panic!(
                "wrong key type passed (remote_kem_public should be EncapsulationKey::McEliece)"
            ),
        },
        _ => panic!("undefined"),
    };
    let initiator_ciphersuite = initiator_cbuilder
        .longterm_x25519_keys(&initiator_x25519_keypair)
        .peer_longterm_x25519_pk(&responder_x25519_public_key)
        .build_initiator_ciphersuite()
        .unwrap();

    let mut initiator = PrincipalBuilder::new(rand09::rng())
        .outer_aad(outer_aad)
        .inner_aad(inner_aad)
        .context(session_context)
        .build_registration_initiator(initiator_ciphersuite)
        .unwrap();

    let mut buffer = vec![0u8; 4096];
    let msg_len = initiator.write_message(b"", &mut buffer).unwrap();
    buffer.resize(msg_len, 0);
    buffer
}
pub fn build_initiator(
    ciphersuite: &Ciphersuite,
    session_context: &[u8],
    inner_aad: &[u8],
    outer_aad: &[u8],
    local_x25519_private_key: &x25519::PrivateKey,
    remote_x25519_public: &x25519::PublicKey,
    remote_kem_public: &EncapsulationKey,
) -> RegistrationInitiator {
    //georgio: handle errors
    let initiator_libcrux_x25519_private_key =
        DHPrivateKey::from_bytes(local_x25519_private_key.as_bytes()).unwrap();

    let initiator_x25519_keypair = DHKeyPair::from(initiator_libcrux_x25519_private_key);

    let responder_x25519_public_key = DHPublicKey::from_bytes(remote_x25519_public.as_bytes());

    let initiator_cbuilder = match ciphersuite.kem() {
        nym_kkt::ciphersuite::KEM::MlKem768 => match remote_kem_public {
            EncapsulationKey::MlKem768(ml_kem_public_key) => CiphersuiteBuilder::new(
                CiphersuiteName::X25519_MLKEM768_X25519_CHACHA20POLY1305_HKDFSHA256,
            )
            .peer_longterm_mlkem_pk(ml_kem_public_key),
            _ => panic!(
                "wrong key type passed (remote_kem_public should be EncapsulationKey::MlKem768)"
            ),
        },
        nym_kkt::ciphersuite::KEM::McEliece => match remote_kem_public {
            EncapsulationKey::McEliece(mceliece_public_key) => CiphersuiteBuilder::new(
                CiphersuiteName::X25519_CLASSICMCELIECE_X25519_CHACHA20POLY1305_HKDFSHA256,
            )
            .peer_longterm_cmc_pk(mceliece_public_key),
            _ => panic!(
                "wrong key type passed (remote_kem_public should be EncapsulationKey::McEliece)"
            ),
        },
        _ => panic!("undefined"),
    };
    let initiator_ciphersuite = initiator_cbuilder
        .longterm_x25519_keys(&initiator_x25519_keypair)
        .peer_longterm_x25519_pk(&responder_x25519_public_key)
        .build_initiator_ciphersuite()
        .unwrap();

    let mut initiator = PrincipalBuilder::new(rand09::rng())
        .outer_aad(outer_aad)
        .inner_aad(inner_aad)
        .context(session_context)
        .build_registration_initiator(initiator_ciphersuite)
        .unwrap();
    initiator
}

pub fn build_responder<'a, R>(
    ciphersuite: &Ciphersuite,
    session_context: &[u8],
    inner_aad: &[u8],
    outer_aad: &[u8],
    local_x25519_private_key: &x25519::PrivateKey,
    local_kem_decapsulation_key: &DecapsulationKey,
    local_kem_encapsulation_key: &EncapsulationKey,
) -> Responder<'a, R>
where
    R: rand09::CryptoRng,
{
    let responder_libcrux_x25519_private_key =
        DHPrivateKey::from_bytes(local_x25519_private_key.as_bytes()).unwrap();

    let responder_x25519_keypair = DHKeyPair::from(responder_libcrux_x25519_private_key);

    let responder_cbuilder = match ciphersuite.kem() {
        nym_kkt::ciphersuite::KEM::MlKem768 => {
            match (local_kem_decapsulation_key, local_kem_encapsulation_key) {
                (
                    DecapsulationKey::MlKem768(ml_kem_private_key),
                    EncapsulationKey::MlKem768(ml_kem_public_key),
                ) => CiphersuiteBuilder::new(
                    CiphersuiteName::X25519_MLKEM768_X25519_CHACHA20POLY1305_HKDFSHA256,
                )
                .longterm_mlkem_encapsulation_key(ml_kem_public_key)
                .longterm_mlkem_decapsulation_key(ml_kem_private_key),
                _ => panic!(
                    "wrong key type passed (local_kem_encapsulation_key should be EncapsulationKey::MlKem768 and local_kem_decapsulation_key should be DecapsulationKey::MlKem768)"
                ),
            }
        }
        nym_kkt::ciphersuite::KEM::McEliece => {
            match (local_kem_decapsulation_key, local_kem_encapsulation_key) {
                (
                    DecapsulationKey::McEliece(mceliece_private_key),
                    EncapsulationKey::McEliece(mceliece_public_key),
                ) => CiphersuiteBuilder::new(
                    CiphersuiteName::X25519_CLASSICMCELIECE_X25519_CHACHA20POLY1305_HKDFSHA256,
                )
                .longterm_cmc_encapsulation_key(mceliece_public_key)
                .longterm_cmc_decapsulation_key(mceliece_private_key),
                _ => panic!(
                    "wrong key type passed (local_kem_encapsulation_key should be EncapsulationKey::McEliece and local_kem_decapsulation_key should be DecapsulationKey::McEliece)"
                ),
            }
        }
        _ => panic!("undefined"),
    };
    let responder_ciphersuite = responder_cbuilder
        .longterm_x25519_keys(&responder_x25519_keypair)
        .build_responder_ciphersuite()
        .unwrap();

    let mut responder = PrincipalBuilder::new(rand09::rng())
        .outer_aad(AAD_RESPONDER)
        .context(SESSION_CONTEXT)
        .build_responder(responder_ciphersuite)
        .unwrap();
    responder
}

pub fn responder_process(
    ciphersuite: &Ciphersuite,
    session_context: &[u8],
    inner_aad: &[u8],
    outer_aad: &[u8],
    local_x25519_private_key: &x25519::PrivateKey,
    local_kem_decapsulation_key: &DecapsulationKey,
    local_kem_encapsulation_key: &EncapsulationKey,
    initiator_message: &[u8],
) -> Vec<u8> {
    let responder_libcrux_x25519_private_key =
        DHPrivateKey::from_bytes(local_x25519_private_key.as_bytes()).unwrap();

    let responder_x25519_keypair = DHKeyPair::from(responder_libcrux_x25519_private_key);

    let responder_cbuilder = match ciphersuite.kem() {
        nym_kkt::ciphersuite::KEM::MlKem768 => {
            match (local_kem_decapsulation_key, local_kem_encapsulation_key) {
                (
                    DecapsulationKey::MlKem768(ml_kem_private_key),
                    EncapsulationKey::MlKem768(ml_kem_public_key),
                ) => CiphersuiteBuilder::new(
                    CiphersuiteName::X25519_MLKEM768_X25519_CHACHA20POLY1305_HKDFSHA256,
                )
                .longterm_mlkem_encapsulation_key(ml_kem_public_key)
                .longterm_mlkem_decapsulation_key(ml_kem_private_key),
                _ => panic!(
                    "wrong key type passed (local_kem_encapsulation_key should be EncapsulationKey::MlKem768 and local_kem_decapsulation_key should be DecapsulationKey::MlKem768)"
                ),
            }
        }
        nym_kkt::ciphersuite::KEM::McEliece => {
            match (local_kem_decapsulation_key, local_kem_encapsulation_key) {
                (
                    DecapsulationKey::McEliece(mceliece_private_key),
                    EncapsulationKey::McEliece(mceliece_public_key),
                ) => CiphersuiteBuilder::new(
                    CiphersuiteName::X25519_CLASSICMCELIECE_X25519_CHACHA20POLY1305_HKDFSHA256,
                )
                .longterm_cmc_encapsulation_key(mceliece_public_key)
                .longterm_cmc_decapsulation_key(mceliece_private_key),
                _ => panic!(
                    "wrong key type passed (local_kem_encapsulation_key should be EncapsulationKey::McEliece and local_kem_decapsulation_key should be DecapsulationKey::McEliece)"
                ),
            }
        }
        _ => panic!("undefined"),
    };
    let responder_ciphersuite = responder_cbuilder
        .longterm_x25519_keys(&responder_x25519_keypair)
        .build_responder_ciphersuite()
        .unwrap();

    let mut responder = PrincipalBuilder::new(rand09::rng())
        .outer_aad(outer_aad)
        .inner_aad(inner_aad)
        .context(session_context)
        .build_responder(responder_ciphersuite)
        .unwrap();

    let mut payload = vec![0u8; 4096];
    responder
        .read_message(&initiator_message, &mut payload)
        .unwrap();

    let mut buffer = vec![0u8; 4096];
    let msg_len = responder.write_message(b"", &mut buffer).unwrap();
    buffer.resize(msg_len, 0);
    buffer
}
