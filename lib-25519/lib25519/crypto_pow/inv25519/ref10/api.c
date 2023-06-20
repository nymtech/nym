#include "crypto_pow.h"
#include "fe.h"

void crypto_pow(unsigned char *q,const unsigned char *p)
{
  fe x;
  fe_frombytes(x,p);
  fe_invert(x,x);
  fe_tobytes(q,x);
}
