// linker define fe25519_invert
// linker use fe25519_pack
// linker use fe25519_unpack

#include "crypto_pow_inv25519.h"
#include "fe25519.h"

void fe25519_invert(fe25519 *r, const fe25519 *x)
{
  unsigned char s[32];
  fe25519_pack(s,x);
  crypto_pow_inv25519(s,s);
  fe25519_unpack(r,s);
}
