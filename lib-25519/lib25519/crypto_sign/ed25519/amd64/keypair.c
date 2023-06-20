#include <string.h>
#include "randombytes.h"
#include "crypto_sign.h"
#include "crypto_hash_sha512.h"
#include "crypto_nG_merged25519.h"

void crypto_sign_keypair(unsigned char *pk,unsigned char *sk)
{
  unsigned char az[64];

  randombytes(sk,32);
  crypto_hash_sha512(az,sk,32);
  az[0] &= 248;
  az[31] &= 63;
  az[31] |= 64;

  crypto_nG_merged25519(pk,az);
  memmove(sk + 32,pk,32);
}
