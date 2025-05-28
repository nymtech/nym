use std::marker::PhantomData;

use nym_crypto::asymmetric::ed25519;
use rand::prelude::*;

use libcrux_kem::MlKem768PublicKey;
use libcrux_psq::{impls::MlKem768, traits::PSQ};
use libcrux_traits::kem::KEM;

use crate::error::KKTError;

const REQ_STR: &[u8] = "KEM_REQ".as_bytes();
const RES_STR: &[u8] = "KEM_RES".as_bytes();

const REQ_LEN: usize = REQ_STR.len() + KKT_TAG_LEN;
const RES_LEN_MLKEM768: usize = REQ_LEN + MlKem768PublicKey::len();

pub const KKT_TAG_LEN: usize = 16;
pub const KKT_REQ_LEN: usize = REQ_LEN + ed25519::SIGNATURE_LENGTH;
pub const KKT_RES_LEN_MLKEM768: usize = RES_LEN_MLKEM768 + ed25519::SIGNATURE_LENGTH;

pub struct KKTInitiator<'a, T: PSQ> {
    signing_key: &'a ed25519::PrivateKey,
    _t: PhantomData<T>,
}

impl<'a, T: PSQ> KKTInitiator<'a, T> {
    pub fn init(signing_key: &'a ed25519::PrivateKey) -> Self {
        Self {
            signing_key,
            _t: PhantomData,
        }
    }
    pub fn request_kem_pk(
        &self,
        request_buffer: &mut [u8; KKT_REQ_LEN],
        tag_buffer: &mut [u8; KKT_TAG_LEN],
    ) {
        // request_buffer[0..7] <- request string
        request_buffer[0..REQ_STR.len()].copy_from_slice(REQ_STR);

        // tag_buffer <- generate tag
        rand::rng().fill_bytes(tag_buffer);

        // request_buffer[7..23] <- tag
        request_buffer[REQ_STR.len()..REQ_LEN].copy_from_slice(tag_buffer);

        // sig <- Sign request_buffer[0..23]
        let sig = self.signing_key.sign(&request_buffer[0..REQ_LEN]);

        // request_buffer[23..87] <- sig
        request_buffer[REQ_LEN..].copy_from_slice(&sig.to_bytes());
    }
}

impl<'a> KKTInitiator<'a, MlKem768> {
    pub fn ingest_response_kem_pk<T: PSQ>(
        &self,
        response: &[u8],
        tag: &[u8; KKT_TAG_LEN],
        responder_verification_key: &ed25519::PublicKey,
    ) -> Result<libcrux_kem::PublicKey, KKTError> {
        // TODO: Refactor asserts into errors
        // Check size of message
        assert_eq!(response.len(), KKT_RES_LEN_MLKEM768);
        // Check if the response string is there at the start of the message
        assert_eq!(&response[0..RES_STR.len()], RES_STR);
        // Check if the tag is the one we expect
        assert_eq!(
            &response[RES_STR.len()..RES_STR.len() + KKT_TAG_LEN],
            &tag[..]
        );

        // Attempt to reconstruct a signature from the received bytes
        match ed25519::Signature::from_bytes(&response[RES_LEN_MLKEM768..]) {
            Ok(sig) => {
                // Attempt to verify signature
                match responder_verification_key.verify(&response[0..RES_LEN_MLKEM768], &sig) {
                    Ok(()) => {
                        // Extract key from bytes
                        // (has to be an owned sized array, unless I'm missing some function that works with slices)
                        let mut key_bytes: [u8; MlKem768PublicKey::len()] =
                            [0u8; MlKem768PublicKey::len()];
                        key_bytes.copy_from_slice(
                            &response[RES_STR.len() + KKT_TAG_LEN..RES_LEN_MLKEM768],
                        );

                        // Create key from bytes and return it
                        Ok(libcrux_kem::PublicKey::MlKem768(MlKem768PublicKey::from(
                            key_bytes,
                        )))
                    }
                    Err(_) => Err(KKTError::SigVerifError),
                }
            }
            Err(_) => Err(KKTError::SigConstructorError),
        }
    }
}

pub struct KKTResponder<'a, T: PSQ> {
    signing_key: &'a ed25519::PrivateKey,
    kem_public_key: &'a <T::InnerKEM as KEM>::EncapsulationKey,
}

impl<'a> KKTResponder<'a, MlKem768> {
    pub fn init(
        signing_key: &'a ed25519::PrivateKey,
        kem_public_key: &'a libcrux_kem::PublicKey,
    ) -> Self {
        Self {
            signing_key,
            kem_public_key,
        }
    }

    pub fn respond_kem_pk(
        &self,
        response_buffer: &mut [u8; KKT_RES_LEN_MLKEM768],
        initiator_verification_key: &ed25519::PublicKey,
        request: &[u8],
    ) -> Result<(), KKTError> {
        // TODO: Refactor asserts into errors
        // Check request size
        assert_eq!(request.len(), KKT_REQ_LEN);
        // Check request string
        assert_eq!(&request[0..REQ_STR.len()], REQ_STR);

        match ed25519::Signature::from_bytes(&request[REQ_LEN..]) {
            Ok(sig) => match initiator_verification_key.verify(&request[0..REQ_LEN], &sig) {
                Ok(()) => {
                    // response_buffer[0..7] <- RES_STR (7 bytes)
                    response_buffer[0..RES_STR.len()].copy_from_slice(RES_STR);

                    // response_buffer[7..23] <- tag (16 bytes, sent by initiator)
                    response_buffer[RES_STR.len()..REQ_LEN]
                        .copy_from_slice(&request[REQ_STR.len()..REQ_LEN]);

                    // response_buffer[23..1207] <- MlKem768 Public Key (1184 bytes)
                    response_buffer[REQ_LEN..RES_LEN_MLKEM768]
                        .copy_from_slice(self.kem_public_key.encode().as_slice());

                    // sign response_buffer[0..1207]
                    let sig = self.signing_key.sign(&response_buffer[0..RES_LEN_MLKEM768]);

                    // response_buffer[1207..1271] <- signature (64 bytes)
                    response_buffer[RES_LEN_MLKEM768..].copy_from_slice(&sig.to_bytes());

                    Ok(())
                }
                Err(_) => Err(KKTError::SigConstructorError),
            },
            Err(_) => Err(KKTError::SigVerifError),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::kkt::{KKT_REQ_LEN, KKT_RES_LEN_MLKEM768, KKT_TAG_LEN, REQ_LEN, RES_LEN_MLKEM768};

    use super::{KKTInitiator, KKTResponder};
    use libcrux_psq::impls::MlKem768;
    use nym_crypto::asymmetric::ed25519;
    use rand::prelude::*;

    #[test]
    fn test_kkt_e2e() {
        // generate ed25519 keys
        let mut secret_initiator: [u8; 32] = [0u8; 32];
        rand::rng().fill_bytes(&mut secret_initiator);
        let initiator_ed25519_keypair = ed25519::KeyPair::from_secret(secret_initiator, 0);

        let mut secret_responder: [u8; 32] = [0u8; 32];
        rand::rng().fill_bytes(&mut secret_responder);
        let responder_ed25519_keypair = ed25519::KeyPair::from_secret(secret_responder, 1);

        // generate kem keypair
        let (_, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rand::rng()).unwrap();

        // initialize parties
        let initiator: KKTInitiator<MlKem768> =
            KKTInitiator::init(initiator_ed25519_keypair.private_key());

        let responder: KKTResponder<MlKem768> = KKTResponder::init(
            responder_ed25519_keypair.private_key(),
            &responder_kem_public_key,
        );

        // create buffers
        let mut request_buffer: [u8; KKT_REQ_LEN] = [0u8; KKT_REQ_LEN];
        let mut response_buffer: [u8; KKT_RES_LEN_MLKEM768] = [0u8; KKT_RES_LEN_MLKEM768];
        let mut tag_buffer: [u8; KKT_TAG_LEN] = [0u8; KKT_TAG_LEN];

        // generate request
        initiator.request_kem_pk(&mut request_buffer, &mut tag_buffer);

        // ingest request, generate response
        responder
            .respond_kem_pk(
                &mut response_buffer,
                initiator_ed25519_keypair.public_key(),
                &request_buffer,
            )
            .unwrap();

        // ingest response
        let received_responder_key = initiator
            .ingest_response_kem_pk::<MlKem768>(
                &response_buffer,
                &tag_buffer,
                responder_ed25519_keypair.public_key(),
            )
            .unwrap();

        // check if the public key received is the same one that we generated at the start
        assert_eq!(
            responder_kem_public_key.encode(),
            received_responder_key.encode()
        );
    }
}
