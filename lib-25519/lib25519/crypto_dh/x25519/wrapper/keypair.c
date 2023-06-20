#include "crypto_nG_montgomery25519.h"
#include "randombytes.h"
#include "crypto_dh.h"

void crypto_dh_keypair(unsigned char *pk,unsigned char *sk)
{
  randombytes(sk,crypto_dh_SECRETKEYBYTES);
  crypto_nG_montgomery25519(pk,sk);
}
