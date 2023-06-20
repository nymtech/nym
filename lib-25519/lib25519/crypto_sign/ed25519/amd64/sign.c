#include "randombytes.h"
#include <string.h>
#include "crypto_sign.h"
#include "crypto_hash_sha512.h"
#include "crypto_nG_merged25519.h"
#include "sc25519.h"

void crypto_sign(
    unsigned char *sm,long long *smlen,
    const unsigned char *m,long long mlen,
    const unsigned char *sk
    )
{
  unsigned char pk[32];
  unsigned char azr[96];
  unsigned char nonce[64];
  unsigned char hram[64];
  sc25519 sck, scs, scsk;

  memmove(pk,sk + 32,32);
  /* pk: 32-byte public key A */

  crypto_hash_sha512(azr,sk,32);
  randombytes(azr+64,32);
  azr[0] &= 248;
  azr[31] &= 63;
  azr[31] |= 64;
  /* azr: 32-byte scalar a, 32-byte randomizer z, 32-byte new randomness r */

  crypto_hash_sha512(azr+32,azr+32,64);

  *smlen = mlen + 64;
  memmove(sm + 64,m,mlen);
  memmove(sm + 32,azr + 32,32);
  /* sm: 32-byte uninit, 32-byte z, mlen-byte m */

  crypto_hash_sha512(nonce, sm+32, mlen+32);
  /* nonce: 64-byte H(z,m) */

  sc25519_from64bytes(&sck, nonce);
  sc25519_to32bytes(nonce, &sck);
  crypto_nG_merged25519(sm,nonce);
  /* sm: 32-byte R, 32-byte z, mlen-byte m */
  
  memmove(sm + 32,pk,32);
  /* sm: 32-byte R, 32-byte A, mlen-byte m */

  crypto_hash_sha512(hram,sm,mlen + 64);
  /* hram: 64-byte H(R,A,m) */

  sc25519_from64bytes(&scs, hram);
  sc25519_from32bytes(&scsk, azr);
  sc25519_mul(&scs, &scs, &scsk);
  sc25519_add(&scs, &scs, &sck);
  /* scs: S = nonce + H(R,A,m)a */

  sc25519_to32bytes(sm + 32,&scs);
  /* sm: 32-byte R, 32-byte S, mlen-byte m */
}
