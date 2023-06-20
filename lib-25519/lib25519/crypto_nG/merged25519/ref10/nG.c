#include <string.h>
#include "randombytes.h"
#include "crypto_nG.h"
#include "crypto_hash_sha512.h"
#include "ge.h"

void crypto_nG(unsigned char *pk,const unsigned char *sk)
{
  unsigned char e[32];
  ge_p3 A;
  fe ZplusY;
  fe ZminusY;
  fe recip;
  fe x;
  fe y;
  int wantmont;

  for (int i = 0;i < 32;++i) e[i] = sk[i];
  wantmont = e[31]>>7;
  e[31] &= 127;

  ge_scalarmult_base(&A,e);

  // A has X,Y,Z,T representing X/Z,Y/Z in edwards form
  // if wantmont: output (Z+Y)/(Z-Y)
  // else: output Y/Z, with a bit reflecting X/Z

  // doing this in constant time (at minor expense)
  // in case wantmont is secret

  fe_add(ZplusY,A.Z,A.Y);
  fe_sub(ZminusY,A.Z,A.Y);
  fe_cmov(A.Y,ZplusY,wantmont);
  fe_cmov(A.Z,ZminusY,wantmont);

  fe_invert(recip,A.Z);
  fe_mul(y,A.Y,recip);
  fe_tobytes(pk,y);

  fe_mul(x,A.X,recip);
  pk[31] ^= ((1-wantmont) & fe_isnegative(x)) << 7;
}
