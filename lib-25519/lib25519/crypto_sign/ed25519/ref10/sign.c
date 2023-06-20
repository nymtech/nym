#include "randombytes.h"
#include <string.h>
#include "crypto_sign.h"
#include "crypto_hash_sha512.h"
#include "crypto_nG_merged25519.h"
#include "sc.h"

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

  memmove(pk,sk + 32,32);

  crypto_hash_sha512(azr,sk,32);
  azr[0] &= 248;
  azr[31] &= 63;
  azr[31] |= 64;
  randombytes(azr+64,32);
  crypto_hash_sha512(azr+32,azr+32,64);

  *smlen = mlen + 64;
  memmove(sm + 64,m,mlen);
  memmove(sm + 32,azr + 32,32);
  crypto_hash_sha512(nonce,sm + 32,mlen + 32);
  memmove(sm + 32,pk,32);

  sc_reduce(nonce);
  crypto_nG_merged25519(sm,nonce);

  crypto_hash_sha512(hram,sm,mlen + 64);
  sc_reduce(hram);
  sc_muladd(sm + 32,hram,azr,nonce);
}
