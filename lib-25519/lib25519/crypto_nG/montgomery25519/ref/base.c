#include "crypto_nP_montgomery25519.h"
#include "crypto_nG.h"

static const unsigned char G[32] = {9};

void crypto_nG(unsigned char *nG,const unsigned char *n)
{
  crypto_nP_montgomery25519(nG,n,G);
}
