use generic_array::{typenum::Unsigned, GenericArray};
use rand::{CryptoRng, RngCore};
use stream_cipher::{Key, NewStreamCipher, Nonce, StreamCipher, SyncStreamCipher};

// TODO: note that this is not the most secure approach here
// we are not using nonces properly but instead "kinda" thinking of them as IVs.
// Nonce require, as the name suggest, being only seen once. Ever.
// While what we are doing here, i.e. generating a pseudo-random IV,
// means that for, for example, 128-bit security, after generating 2^64 IVs
// we are going to have 50% chance of collision. But perhaps that's fine?
// TODO2: ask @AP if what I wrote here even makes sense in the context of what we're doing.
pub type IV<C> = Nonce<C>;

pub fn generate_key<C, R>(rng: &mut R) -> Key<C>
where
    C: NewStreamCipher,
    R: RngCore + CryptoRng,
{
    let mut key = GenericArray::default();
    rng.fill_bytes(&mut key);
    key
}

pub fn random_iv<C, R>(rng: &mut R) -> IV<C>
where
    C: NewStreamCipher,
    R: RngCore + CryptoRng,
{
    let mut iv = GenericArray::default();
    rng.fill_bytes(&mut iv);
    iv
}

pub fn zero_iv<C>() -> IV<C>
where
    C: NewStreamCipher,
{
    GenericArray::default()
}

pub fn iv_from_slice<C>(b: &[u8]) -> &IV<C>
where
    C: NewStreamCipher,
{
    if b.len() != C::NonceSize::to_usize() {
        // `from_slice` would have caused a panic about this issue anyway.
        // Now we at least have slightly more information
        panic!(
            "Tried to convert {} bytes to IV. Expected {}",
            b.len(),
            C::NonceSize::to_usize()
        )
    }
    GenericArray::from_slice(b)
}

// TODO: there's really no way to use more parts of the keystream if it was required at some point.
// However, do we really expect to ever need it?

pub fn encrypt<C>(key: &Key<C>, iv: &IV<C>, data: &[u8]) -> Vec<u8>
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut ciphertext = data.to_vec();
    encrypt_in_place::<C>(key, iv, &mut ciphertext);
    ciphertext
}

pub fn encrypt_in_place<C>(key: &Key<C>, iv: &IV<C>, data: &mut [u8])
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut cipher = C::new(key, iv);
    cipher.encrypt(data)
}

pub fn decrypt<C>(key: &Key<C>, iv: &IV<C>, ciphertext: &[u8]) -> Vec<u8>
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut data = ciphertext.to_vec();
    decrypt_in_place::<C>(key, iv, &mut data);
    data
}

pub fn decrypt_in_place<C>(key: &Key<C>, iv: &IV<C>, data: &mut [u8])
where
    C: SyncStreamCipher + NewStreamCipher,
{
    let mut cipher = C::new(key, iv);
    cipher.decrypt(data)
}
