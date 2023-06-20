#include "crypto_pow.h"
#include "fe.h"
#include "fe51.h"

void crypto_pow(unsigned char *q,const unsigned char *p)
{
  fe x;
  fe51 x51;
  fe_frombytes(x,p);
  x51.v[0] = (x[1] << 26) + x[0];
  x51.v[1] = (x[3] << 26) + x[2];
  x51.v[2] = (x[5] << 26) + x[4];
  x51.v[3] = (x[7] << 26) + x[6];
  x51.v[4] = (x[9] << 26) + x[8];
  fe51_invert(&x51,&x51);
  fe51_pack(q,&x51);
}
