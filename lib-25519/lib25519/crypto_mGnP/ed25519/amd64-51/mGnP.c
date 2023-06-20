#include "crypto_mGnP.h"

#include "fe25519.h"
#include "sc25519.h"
#include "ge25519.h"
#include "shared-data.h"
#include "crypto_verify_32.h"

static const fe25519 ec2d = {{1859910466990425, 932731440258426, 1072319116312658, 1815898335770999, 633789495995903}};

static void setneutral(ge25519 *r)
{
  fe25519_setint(&r->x,0);
  fe25519_setint(&r->y,1);
  fe25519_setint(&r->z,1);
  fe25519_setint(&r->t,0);
}

static void ge25519_double_scalarmult_precompute(ge25519_pniels *pre1, const ge25519_p3 *p1, unsigned long long PRE1_SIZE)
{
  ge25519_p3 d1;
  ge25519_p1p1 t;
  fe25519 d;
  int i;

  pre1[0] = *(ge25519_pniels *)p1;                                                                         
  ge25519_dbl_p1p1(&t,(ge25519_p2 *)pre1);      ge25519_p1p1_to_p3(&d1, &t);
  /* Convert pre[0] to projective Niels representation */
  d = pre1[0].ysubx;
  fe25519_sub(&pre1[0].ysubx, &pre1[0].xaddy, &pre1[0].ysubx);
  fe25519_add(&pre1[0].xaddy, &pre1[0].xaddy, &d);
  fe25519_mul(&pre1[0].t2d, &pre1[0].t2d, &ec2d);

  for(i=0;i<PRE1_SIZE-1;i++)
  {
    ge25519_pnielsadd_p1p1(&t, &d1, &pre1[i]);  ge25519_p1p1_to_pniels(&pre1[i+1], &t);
  }
}

static void ge25519_double_scalarmult_process(ge25519_p3 *r, const signed char *slide1, const signed char *slide2, const ge25519_pniels *pre1, const ge25519_niels *pre2)
{
  ge25519_pniels neg;
  ge25519_niels nneg;
  ge25519_p1p1 t;
  fe25519 d;
  int i;

  setneutral(r);
  for (i = 255;i >= 0;--i) {
    if (slide1[i] || slide2[i]) goto firstbit;
  }

  for(;i>=0;i--)
  {
    firstbit:

    ge25519_dbl_p1p1(&t, (ge25519_p2 *)r);

    if(slide1[i]>0)
    {
      ge25519_p1p1_to_p3(r, &t);
      ge25519_pnielsadd_p1p1(&t, r, &pre1[slide1[i]/2]);
    }
    else if(slide1[i]<0)
    {
      ge25519_p1p1_to_p3(r, &t);
      neg = pre1[-slide1[i]/2];
      d = neg.ysubx;
      neg.ysubx = neg.xaddy;
      neg.xaddy = d;
      fe25519_neg(&neg.t2d, &neg.t2d);
      ge25519_pnielsadd_p1p1(&t, r, &neg);
    }

    if(slide2[i]>0)
    {
      ge25519_p1p1_to_p3(r, &t);
      ge25519_nielsadd_p1p1(&t, r, &pre2[slide2[i]/2]);
    }
    else if(slide2[i]<0)
    {
      ge25519_p1p1_to_p3(r, &t);
      nneg = pre2[-slide2[i]/2];
      d = nneg.ysubx;
      nneg.ysubx = nneg.xaddy;
      nneg.xaddy = d;
      fe25519_neg(&nneg.t2d, &nneg.t2d);
      ge25519_nielsadd_p1p1(&t, r, &nneg);
    }

    ge25519_p1p1_to_p2((ge25519_p2 *)r, &t);
  }
}

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
