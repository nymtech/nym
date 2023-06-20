#include "crypto_mGnP.h"

#include "sc25519.h"
#include "ge25519.h"
#include "shared-data.h"
#include "crypto_verify_32.h"

void crypto_mGnP(
  unsigned char *Q,
  const unsigned char *m,
  const unsigned char *n,
  const unsigned char *P
)
{
  sc25519 m_internal;
  signed char m_slide[256];
  unsigned char mcheck[32];
  sc25519 n_internal;
  signed char n_slide[256];
  ge25519 P_internal;
  ge25519_pniels P_multiples[P_MULTIPLES];
  ge25519_p3 result;
  int ok;

  sc25519_from32bytes(&m_internal,m);
  sc25519_from64bytes(&n_internal,n);
  ok = ge25519_unpackneg_vartime(&P_internal,P);

  sc25519_to32bytes(mcheck,&m_internal);
  if (crypto_verify_32(mcheck,m)) ok = 0;

  sc25519_slide(m_slide,&m_internal,G_WINDOWSIZE);
  sc25519_slide(n_slide,&n_internal,P_WINDOWSIZE);
  ge25519_double_scalarmult_precompute(P_multiples,&P_internal,P_MULTIPLES);

  ge25519_double_scalarmult_process(&result,n_slide,m_slide,P_multiples,G_multiples);
  ge25519_pack(Q,&result);
  Q[32] = ok;
}
