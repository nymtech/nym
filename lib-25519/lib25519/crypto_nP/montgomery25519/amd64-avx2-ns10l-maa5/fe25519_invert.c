// linker define fe25519_invert
// linker use fe25519_from_5l fe25519_pack fe25519_unpack fe25519_to_10l fe25519_10l_to_5l

#include "crypto_pow_inv25519.h"
#include "fe25519.h"

void fe25519_invert(fe25519_5l *r, const fe25519_5l *x)
{
  // XXX: can streamline this
  fe25519 t;
  fe25519_10l u;
  unsigned char s[32];
  fe25519_from_5l(&t,x);
  fe25519_pack(s,&t);
  crypto_pow_inv25519(s,s);
  fe25519_unpack(&t,s);
  fe25519_to_10l(&u,&t);
  fe25519_10l_to_5l(r,&u);
}
