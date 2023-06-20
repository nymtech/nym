// linker define ge_tobytes
// linker use fe_invert fe_mul fe_tobytes fe_isnegative

#include "ge.h"

void ge_tobytes(unsigned char *s,const ge_p2 *h)
{
  fe recip;
  fe x;
  fe y;

  fe_invert(recip,h->Z);
  fe_mul(x,h->X,recip);
  fe_mul(y,h->Y,recip);
  fe_tobytes(s,y);
  s[31] ^= fe_isnegative(x) << 7;
}
