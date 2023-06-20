#include "randombytes.h"
#include "crypto_nP.h"
#include "fe25519.h"

#define mladder CRYPTO_SHARED_NAMESPACE(mladder)
extern void mladder(fe25519 *,fe25519 *,const unsigned char *);

void crypto_nP(unsigned char *r,
                      const unsigned char *s,
                      const unsigned char *p)
{
  unsigned char e[32];
  int i;
  for(i=0;i<32;i++) e[i] = s[i];
  e[0] &= 248;
  e[31] &= 127;
  e[31] |= 64; 

  fe25519 t[2];
  fe25519_unpack(t, p);
  mladder(t, t, e);
  fe25519_invert(t+1, t+1);
  fe25519_mul(t, t, t+1);
  fe25519_pack(r, t);
}
