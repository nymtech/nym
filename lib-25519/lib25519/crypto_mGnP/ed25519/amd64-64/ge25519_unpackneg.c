#include "crypto_verify_32.h"
#include "fe25519.h"
#include "ge25519.h"

/* d */
static const fe25519 ecd = {{0x75EB4DCA135978A3, 0x00700A4D4141D8AB, 0x8CC740797779E898, 0x52036CEE2B6FFE73}};
/* sqrt(-1) */
static const fe25519 sqrtm1 = {{0xC4EE1B274A0EA0B0, 0x2F431806AD2FE478, 0x2B4D00993DFBD7A7, 0x2B8324804FC1DF0B}};
static const fe25519 zero = {{0,0,0,0}};

static const fe25519 point26_x = {{0x5bf5acbd527f9b28,0xa4564f8c5508aa23,0x4daaa6d39e2975af,0x6fe31a937f53b071}};
static const fe25519 point26_y = {{26,0,0,0}};

/* return 1 on success, 0 otherwise */
int ge25519_unpackneg_vartime(ge25519_p3 *r, const unsigned char p[32])
{
  int ok = 1;
  unsigned char pcheck[32];
  fe25519 t, chk, num, den, den2, den4, den6;
  unsigned char par = p[31] >> 7;

  fe25519_setint(&r->z,1);
  fe25519_unpack(&r->y, p); 

  fe25519_pack(pcheck,&r->y);
  pcheck[31] |= par<<7;
  if (crypto_verify_32(pcheck,p)) ok = 0;

  fe25519_square(&num, &r->y); /* x = y^2 */
  fe25519_mul(&den, &num, &ecd); /* den = dy^2 */
  fe25519_sub(&num, &num, &r->z); /* x = y^2-1 */
  fe25519_add(&den, &r->z, &den); /* den = dy^2+1 */

  /* Computation of sqrt(num/den)
     1.: computation of num^((p-5)/8)*den^((7p-35)/8) = (num*den^7)^((p-5)/8)
  */
  fe25519_square(&den2, &den);
  fe25519_square(&den4, &den2);
  fe25519_mul(&den6, &den4, &den2);
  fe25519_mul(&t, &den6, &num);
  fe25519_mul(&t, &t, &den);

  fe25519_pow2523(&t, &t);
  /* 2. computation of r->x = t * num * den^3
  */
  fe25519_mul(&t, &t, &num);
  fe25519_mul(&t, &t, &den);
  fe25519_mul(&t, &t, &den);
  fe25519_mul(&r->x, &t, &den);

  /* 3. Check whether sqrt computation gave correct result, multiply by sqrt(-1) if not:
  */
  fe25519_square(&chk, &r->x);
  fe25519_mul(&chk, &chk, &den);
  if (!fe25519_iseq_vartime(&chk, &num))
    fe25519_mul(&r->x, &r->x, &sqrtm1);

  /* 4. Now we have one of the two square roots, except if input was not a square
  */
  fe25519_square(&chk, &r->x);
  fe25519_mul(&chk, &chk, &den);
  if (!fe25519_iseq_vartime(&chk,&num)) ok = 0;

  /* 5. Choose the desired square root according to parity:
  */
  if(fe25519_getparity(&r->x) != (1-par))
    fe25519_sub(&r->x,&zero,&r->x);
  if (par && fe25519_iseq_vartime(&r->x,&zero)) ok = 0;

  if (!ok) { /* treat all invalid points as point26 */
    r->x = point26_x;
    r->y = point26_y;
  }

  fe25519_mul(&r->t, &r->x, &r->y);

  return ok;
}
