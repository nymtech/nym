mod bindings;
use std::str::FromStr;

use bindings::*;

#[derive(Debug)]
pub struct PublicKeyDh([u8; LIB25519_DH_PUBLICKEYBYTES]);
impl PublicKeyDh {
    pub fn new(k: [u8; LIB25519_DH_PUBLICKEYBYTES]) -> Self {
        Self(k)
    }

    pub fn bytes(&self) -> [u8; LIB25519_DH_PUBLICKEYBYTES] {
        self.0
    }
}

#[derive(Debug)]
pub struct PublicKeySign([u8; LIB25519_SIGN_PUBLICKEYBYTES]);
impl PublicKeySign {
    pub fn new(k: [u8; LIB25519_SIGN_PUBLICKEYBYTES]) -> Self {
        Self(k)
    }

    pub fn bytes(&self) -> [u8; LIB25519_SIGN_PUBLICKEYBYTES] {
        self.0
    }
}

#[derive(Debug)]
pub struct SecretKeyDh([u8; LIB25519_DH_SECRETKEYBYTES]);
impl SecretKeyDh {
    pub fn new(k: [u8; LIB25519_DH_SECRETKEYBYTES]) -> Self {
        Self(k)
    }

    pub fn bytes(&self) -> [u8; LIB25519_DH_SECRETKEYBYTES] {
        self.0
    }
}

#[derive(Debug)]
pub struct SecretKeySign([u8; LIB25519_SIGN_SECRETKEYBYTES]);
impl SecretKeySign {
    pub fn new(k: [u8; LIB25519_SIGN_SECRETKEYBYTES]) -> Self {
        Self(k)
    }

    pub fn bytes(&self) -> [u8; LIB25519_SIGN_SECRETKEYBYTES] {
        self.0
    }
}

#[derive(Debug)]
pub struct SharedSecret([u8; LIB25519_DH_BYTES]);
impl SharedSecret {
    pub fn new(k: [u8; LIB25519_DH_BYTES]) -> Self {
        Self(k)
    }

    pub fn bytes(&self) -> [u8; LIB25519_DH_BYTES] {
        self.0
    }
}

#[derive(Debug)]
pub struct SignedMsg(Vec<u8>);
impl SignedMsg {
    pub fn new(k: Vec<u8>) -> Self {
        Self(k)
    }

    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn len(&self) -> usize {
        self.bytes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct Msg(Vec<u8>);
impl Msg {
    pub fn new(k: Vec<u8>) -> Self {
        Self(k)
    }

    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn len(&self) -> usize {
        self.bytes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl FromStr for Msg {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Msg::new(s.as_bytes().to_vec()))
    }
}

impl PartialEq for Msg {
    fn eq(&self, other: &Msg) -> bool {
        self.bytes() == other.bytes()
    }
}

/// 1. X25519 key generation:
///
/// This function randomly generates Alice's secret key sk[0], sk[1], ...,
/// sk[lib25519_dh_SECRETKEYBYTES-1] and Alice's corresponding public key
/// pk[0], pk[1], ..., pk[lib25519_dh_PUBLICKEYBYTES-1].
pub fn lib25519_dh_keypair() -> (PublicKeyDh, SecretKeyDh) {
    let mut pk = [0u8; LIB25519_DH_PUBLICKEYBYTES];
    let mut sk = [0u8; LIB25519_DH_SECRETKEYBYTES];

    unsafe { lib25519_dh_x25519_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) }

    (PublicKeyDh::new(pk), SecretKeyDh::new(sk))
}

/// 2. X25519 shared-secret generation:
///
/// This function computes the X25519 secret k[0], k[1], ...,
/// k[lib25519_dh_BYTES-1] shared between Alice and Bob, given Bob's public
/// key pk[0], pk[1], ..., pk[lib25519_dh_PUBLICKEYBYTES-1] and Alice's
/// secret key sk[0], sk[1], ..., sk[lib25519_dh_SECRETKEYBYTES-1].
///
/// lib25519_dh_PUBLICKEYBYTES, lib25519_dh_SECRETKEYBYTES, and
/// lib25519_dh_BYTES are guaranteed to be 32, but callers wishing to allow
/// easy substitution of other DH systems should not rely on this guarantee.
pub fn lib25519_dh(pk: &PublicKeyDh, sk: &SecretKeyDh) -> SharedSecret {
    let mut s = [0u8; LIB25519_DH_SECRETKEYBYTES];
    unsafe { lib25519_dh_x25519(s.as_mut_ptr(), pk.bytes().as_ptr(), sk.bytes().as_ptr()) }
    SharedSecret::new(s)
}

/// 3. Ed25519 key generation:
///
/// This function randomly generates a secret key sk[0], sk[1], ...,
/// sk[lib25519_sign_SECRETKEYBYTES-1] and a corresponding public key
/// pk[0], pk[1], ..., pk[lib25519_sign_PUBLICKEYBYTES-1].
///
/// lib25519_sign_PUBLICKEYBYTES is guaranteed to be 32, and
/// lib25519_sign_SECRETKEYBYTES is guaranteed to be 64, but callers wishing
/// to allow easy substitution of other signature systems should not rely on
/// these guarantees.
pub fn lib25519_sign_keypair() -> (PublicKeySign, SecretKeySign) {
    let mut pk = [0u8; LIB25519_SIGN_PUBLICKEYBYTES];
    let mut sk = [0u8; LIB25519_SIGN_SECRETKEYBYTES];

    unsafe { lib25519_sign_ed25519_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) }

    (PublicKeySign::new(pk), SecretKeySign::new(sk))
}

/// 4. Ed25519 signing:
///
/// This function signs a message m[0], ..., m[mlen-1] using the signer's
/// secret key sk[0], sk[1], ..., sk[lib25519_sign_SECRETKEYBYTES-1], puts
/// the length of the signed message into smlen, and puts the signed message
/// into sm[0], sm[1], ..., sm[smlen-1].
///
/// The maximum possible length smlen is mlen+lib25519_sign_BYTES. The caller
/// must allocate at least mlen+lib25519_sign_BYTES for sm.
///
/// lib25519_sign_SECRETKEYBYTES is guaranteed to be 64, lib25519_sign_BYTES
/// is guaranteed to be 64, and signed messages are always exactly 64 bytes
/// longer than messages, but callers wishing to allow easy substitution of
/// other signature systems should not rely on these guarantees.
pub fn lib25519_sign(sk: &SecretKeySign, msg: &Msg) -> SignedMsg {
    let mut sm = vec![0u8; msg.len() + LIB25519_SIGN_BYTES];
    let mut smlen = sm.len() as i64;
    let mlen = msg.len() as i64;
    unsafe {
        lib25519_sign_ed25519(
            sm.as_mut_ptr(),
            &mut smlen,
            msg.bytes().as_ptr(),
            mlen,
            sk.bytes().as_ptr(),
        )
    }
    SignedMsg::new(sm)
}

/// 5. Ed25519 signature verification and message recovery:
///
/// This function verifies the signed message in sm[0], ..., sm[smlen-1]
/// using the signer's public key pk[0], pk[1], ...,
/// pk[lib25519_sign_PUBLICKEYBYTES-1]. This function puts the length of the
/// message into mlen and puts the message into m[0], m[1], ..., m[mlen-1].
/// It then returns 0.
///
/// The maximum possible length mlen is smlen. The caller must allocate at
/// least smlen bytes for m (not just some guess for the number of bytes
/// expected in m).
///
/// If the signature fails verification, lib25519_sign_open instead returns
/// -1. It also sets mlen to -1 and clears m[0], m[1], ..., m[smlen-1], but
/// callers should note that other signature software does not necessarily
/// do this; callers should always check the return value.
///
/// lib25519_sign_PUBLICKEYBYTES is guaranteed to be 32, but callers wishing
/// to allow easy substitution of other signature systems should not rely on
/// this guarantee.
pub fn lib25519_sign_open(pk: &PublicKeySign, sm: &SignedMsg) -> Msg {
    let mut m = vec![0u8; sm.len()];
    let mut mlen = 0;
    let smlen = sm.len() as i64;

    let result = unsafe {
        lib25519_sign_ed25519_open(
            m.as_mut_ptr(),
            &mut mlen,
            sm.bytes().as_ptr(),
            smlen,
            pk.bytes().as_ptr(),
        )
    };

    debug_assert!(result == 0);

    m.truncate(mlen as usize);

    Msg::new(m)
}

#[cfg(test)]
mod test {
    use crate::bindings::*;
    use std::str::FromStr;

    #[test]
    fn lib25519_dh_keypair() {
        let (pk, _sk) = crate::lib25519_dh_keypair();
        assert_ne!(pk.bytes(), [0u8; LIB25519_DH_PUBLICKEYBYTES])
    }

    #[test]
    fn lib25519_dh() {
        let (pk, sk) = crate::lib25519_dh_keypair();
        let s = crate::lib25519_dh(&pk, &sk);
        assert_ne!(s.bytes(), [0u8; LIB25519_DH_PUBLICKEYBYTES])
    }

    #[test]
    fn lib25519_sign_keypair() {
        let (pk, _sk) = crate::lib25519_sign_keypair();
        assert_ne!(pk.bytes(), [0u8; LIB25519_DH_PUBLICKEYBYTES])
    }

    #[test]
    fn lib25519_sign_open() {
        let (pk, sk) = crate::lib25519_sign_keypair();
        let msg = crate::Msg::from_str("super secret message").unwrap();
        let sm = crate::lib25519_sign(&sk, &msg);
        assert_eq!(sm.bytes().len(), msg.len() + LIB25519_SIGN_BYTES);
        assert_ne!(sm.bytes(), vec![0u8; msg.len() + LIB25519_SIGN_BYTES]);

        let omsg = crate::lib25519_sign_open(&pk, &sm);
        assert_eq!(omsg, msg)
    }
}
