#include "randombytes.h"
#include "crypto_nP.h"
#include "fe25519.h"

#define work_cswap CRYPTO_SHARED_NAMESPACE(work_cswap)
#define ladderstep CRYPTO_SHARED_NAMESPACE(ladderstep)

extern void work_cswap(fe25519 *, unsigned long long);
extern void ladderstep(fe25519 *work);

static void mladder(fe25519 *xr, fe25519 *zr, const unsigned char s[32])
{
  fe25519 work[5];
  unsigned char bit, prevbit=0;
  unsigned long long swap;
  int j;
  int i;

  work[0] = *xr;
  fe25519_setint(work+1,1);
  fe25519_setint(work+2,0);
  work[3] = *xr;
  fe25519_setint(work+4,1);

  j = 6;
  for(i=31;i>=0;i--)
  {
    while(j >= 0)
    {
      bit = 1&(s[i]>>j);
      swap = bit ^ prevbit;
      prevbit = bit;
      work_cswap(work+1, swap);
      ladderstep(work);
      j -= 1;
    }
    j = 7;
  }
  *xr = work[1];
  *zr = work[2];
}

void crypto_nP(unsigned char *nP,
                      const unsigned char *n,
                      const unsigned char *P)
{
  unsigned char e[32];
  int i;
  for(i=0;i<32;i++) e[i] = n[i];
  e[0] &= 248;
  e[31] &= 127;
  e[31] |= 64; 

  fe25519 t;
  fe25519 z;
  fe25519_unpack(&t, P);
  mladder(&t, &z, e);
  fe25519_invert(&z, &z);
  fe25519_mul(&t, &t, &z);
  fe25519_pack(nP, &t);
}
