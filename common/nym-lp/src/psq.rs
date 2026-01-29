use libcrux_psq::{
    Channel,
    handshake::{
        RegistrationInitiator, Responder,
        builders::{CiphersuiteBuilder, PrincipalBuilder},
        ciphersuites::CiphersuiteName,
        types::{DHKeyPair, DHPublicKey},
    },
};
use nym_kkt::ciphersuite::{Ciphersuite, DecapsulationKey, EncapsulationKey, KEM, KemKeyPair};
use rand09::rngs::ThreadRng;

use std::fmt::Debug;

const AAD_INITIATOR_OUTER: &[u8] = b"Test Data I Outer";
const AAD_INITIATOR_INNER: &[u8] = b"Test Data I Inner";
const AAD_RESPONDER: &[u8] = b"Test Data R";
const SESSION_CONTEXT: &[u8] = b"Test Context";

pub enum PSQState<'a> {
    Initiator(RegistrationInitiator<'a, ThreadRng>),
    Responder(Responder<'a, ThreadRng>),
}
impl Debug for PSQState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initiator(_) => f.debug_tuple("PSQ Initiator").finish(),
            Self::Responder(_) => f.debug_tuple("PSQ Responder").finish(),
        }
    }
}

pub fn initiator_process<'a>(initiator: &'a mut RegistrationInitiator<ThreadRng>) -> Vec<u8> {
    let mut buffer = vec![0u8; 4096];
    let msg_len = initiator.write_message(b"", &mut buffer).unwrap();
    buffer.resize(msg_len, 0);
    buffer
}

pub fn build_initiator<'a>(
    ciphersuite: &'a Ciphersuite,
    session_context: &'a [u8],
    local_x25519_keys: &'a DHKeyPair,
    remote_x25519_public: &'a DHPublicKey,
    remote_kem_public: &'a EncapsulationKey,
) -> RegistrationInitiator<'a, rand09::rngs::ThreadRng> {
    //georgio: handle errors

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
        .longterm_x25519_keys(local_x25519_keys)
        .peer_longterm_x25519_pk(remote_x25519_public)
        .build_initiator_ciphersuite()
        .unwrap();

    PrincipalBuilder::new(rand09::rng())
        .outer_aad(AAD_INITIATOR_OUTER)
        .inner_aad(AAD_INITIATOR_INNER)
        .context(session_context)
        .build_registration_initiator(initiator_ciphersuite)
        .unwrap()
}

// JS: I have removed the `ciphersuite` argument as it was only matching on the key types,
// which we already obtained matching on the ciphersuite kem type in `LpSession::new`
pub fn build_responder<'a>(
    local_x25519_keys: &'a DHKeyPair,
    local_kem_keys: &'a KemKeyPair,
) -> Responder<'a, rand09::rngs::ThreadRng> {
    let responder_ciphersuite = match local_kem_keys {
        KemKeyPair::MlKem768 {
            encapsulation_key,
            decapsulation_key,
        } => CiphersuiteBuilder::new(
            CiphersuiteName::X25519_MLKEM768_X25519_CHACHA20POLY1305_HKDFSHA256,
        )
        .longterm_mlkem_encapsulation_key(encapsulation_key)
        .longterm_mlkem_decapsulation_key(decapsulation_key),
        KemKeyPair::McEliece {
            encapsulation_key,
            decapsulation_key,
        } => CiphersuiteBuilder::new(
            CiphersuiteName::X25519_CLASSICMCELIECE_X25519_CHACHA20POLY1305_HKDFSHA256,
        )
        .longterm_cmc_encapsulation_key(encapsulation_key)
        .longterm_cmc_decapsulation_key(decapsulation_key),
        KemKeyPair::XWing { .. } => panic!("unsupported"),
        KemKeyPair::X25519 { .. } => panic!("unsupported"),
    }
    .longterm_x25519_keys(local_x25519_keys)
    .build_responder_ciphersuite()
    .unwrap();

    PrincipalBuilder::new(rand09::rng())
        .outer_aad(AAD_RESPONDER)
        .context(SESSION_CONTEXT)
        .build_responder(responder_ciphersuite)
        .unwrap()
}

pub fn responder_process<'a>(
    responder: &'a mut Responder<ThreadRng>,
    initiator_message: &[u8],
) -> Vec<u8> {
    let mut payload = vec![0u8; 4096];
    responder
        .read_message(initiator_message, &mut payload)
        .unwrap();

    let mut buffer = vec![0u8; 4096];
    let msg_len = responder.write_message(b"", &mut buffer).unwrap();
    buffer.resize(msg_len, 0);
    buffer
}
