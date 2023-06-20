// linker define ge25519_scalarmult_base
// linker use ge25519_base_multiples_niels
// linker use sc25519_window4
// linker use base

#include "fe25519.h"
#include "sc25519.h"
#include "ge25519.h"

#define base CRYPTO_SHARED_NAMESPACE(base)
extern void base(ge25519_p3 *,const signed char *,const ge25519_niels *);

void ge25519_scalarmult_base(ge25519_p3 *r, const sc25519 *s)
{
  signed char b[64];
  sc25519_window4(b,s);
  base(r,b,ge25519_base_multiples_niels);
}
