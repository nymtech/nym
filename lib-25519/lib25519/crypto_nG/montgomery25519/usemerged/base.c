#include "crypto_nG_merged25519.h"
#include "crypto_nG.h"

void crypto_nG(unsigned char *nG,const unsigned char *n)
{
  unsigned char e[32];
  for (int i = 0;i < 32;++i) e[i] = n[i];
  e[0] &= 248;
  e[31] &= 127;
  e[31] |= 192;
  crypto_nG_merged25519(nG,e);
}

