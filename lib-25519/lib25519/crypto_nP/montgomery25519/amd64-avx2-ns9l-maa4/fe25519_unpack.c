// linker define fe25519_unpack

#include "fe25519.h"

void fe25519_unpack(fe25519 *r, const unsigned char x[32])
{
  /* assuming little-endian */
  r->l[0] = *(unsigned long long *)x;
  r->l[1] = *(((unsigned long long *)x)+1);
  r->l[2] = *(((unsigned long long *)x)+2);
  r->l[3] = *(((unsigned long long *)x)+3);
  r->l[3] &= 0x7fffffffffffffffULL;
}
