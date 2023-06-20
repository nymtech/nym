#include "crypto_pow.h"
#include "fe25519.h"

void crypto_pow(unsigned char *q,const unsigned char *p)
{
  fe25519 x;
  fe25519_unpack(&x,p);
  fe25519_invert(&x,&x);
  fe25519_pack(q,&x);
}
