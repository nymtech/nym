#include <string.h>
#include "ge.h"
#include "sc.h"
#include "crypto_verify_32.h"
#include "crypto_mGnP.h"

void crypto_mGnP(
  unsigned char *Q,
  const unsigned char *m,
  const unsigned char *n,
  const unsigned char *P
)
{
  unsigned char mcopy[64];
  unsigned char ncopy[64];
  ge_p3 P_internal;
  ge_p2 R;
  int ok;

  ok = ge_frombytes_negate_vartime(&P_internal,P);

  memcpy(mcopy,m,32);
  memset(mcopy+32,0,32);
  sc_reduce(mcopy);
  if (crypto_verify_32(mcopy,m)) ok = 0;

  memcpy(ncopy,n,64);
  sc_reduce(ncopy);

  ge_double_scalarmult_vartime(&R,ncopy,&P_internal,mcopy);
  ge_tobytes(Q,&R);
  Q[32] = ok;
}
