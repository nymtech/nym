// linker define fe_invert
// linker use fe_tobytes
// linker use fe_frombytes

#include "crypto_pow_inv25519.h"
#include "fe.h"

void fe_invert(fe out,const fe z)
{
  unsigned char r[32];
  fe_tobytes(r,z);
  crypto_pow_inv25519(r,r);
  fe_frombytes(out,r);
}
