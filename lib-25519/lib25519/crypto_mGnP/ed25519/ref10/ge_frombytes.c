#include <string.h>
#include "crypto_verify_32.h"
#include "ge.h"

static const fe d = {
#include "d.h"
} ;

static const fe sqrtm1 = {
#include "sqrtm1.h"
} ;

#include "point26.h"

int ge_frombytes_negate_vartime(ge_p3 *h,const unsigned char *s)
{
  unsigned char scheck[32];
  fe u;
  fe v;
  fe v3;
  fe vxx;
  fe check;
  int ok = 1;

  fe_frombytes(h->Y,s);

  fe_tobytes(scheck,h->Y);
  scheck[31] |= s[31] & 128;
  if (crypto_verify_32(scheck,s)) ok = 0;

  fe_1(h->Z);
  fe_sq(u,h->Y);
  fe_mul(v,u,d);
  fe_sub(u,u,h->Z);       /* u = y^2-1 */
  fe_add(v,v,h->Z);       /* v = dy^2+1 */

  fe_sq(v3,v);
  fe_mul(v3,v3,v);        /* v3 = v^3 */
  fe_sq(h->X,v3);
  fe_mul(h->X,h->X,v);
  fe_mul(h->X,h->X,u);    /* x = uv^7 */

  fe_pow22523(h->X,h->X); /* x = (uv^7)^((q-5)/8) */
  fe_mul(h->X,h->X,v3);
  fe_mul(h->X,h->X,u);    /* x = uv^3(uv^7)^((q-5)/8) */

  fe_sq(vxx,h->X);
  fe_mul(vxx,vxx,v);
  fe_sub(check,vxx,u);    /* vx^2-u */
  if (fe_isnonzero(check)) {
    fe_add(check,vxx,u);  /* vx^2+u */
    if (fe_isnonzero(check)) ok = 0;
    fe_mul(h->X,h->X,sqrtm1);
  }

  if (fe_isnegative(h->X) == (s[31] >> 7))
    fe_neg(h->X,h->X);
  if (!fe_isnonzero(h->X))
    if (s[31] >> 7)
      ok = 0;

  if (!ok) { /* treat all invalid points as point26 */
    memcpy(h->X,point26_x,sizeof point26_x);
    memcpy(h->Y,point26_y,sizeof point26_y);
  }

  fe_mul(h->T,h->X,h->Y);
  return ok;
}
