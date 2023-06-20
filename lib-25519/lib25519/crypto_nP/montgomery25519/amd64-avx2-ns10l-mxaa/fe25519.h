#ifndef FE25519_H
#define FE25519_H

#define fe25519                CRYPTO_NAMESPACE(fe25519)
#define fe25519_10l            CRYPTO_NAMESPACE(fe25519_10l)
#define fe25519_freeze         CRYPTO_SHARED_NAMESPACE(fe25519_freeze)
#define fe25519_unpack         CRYPTO_NAMESPACE(fe25519_unpack)
#define fe25519_pack           CRYPTO_NAMESPACE(fe25519_pack)
#define fe25519_mul            CRYPTO_SHARED_NAMESPACE(fe25519_mul)
#define fe25519_nsquare        CRYPTO_SHARED_NAMESPACE(fe25519_nsquare)
#define fe25519_invert         CRYPTO_NAMESPACE(fe25519_invert)
#define fe25519_to_10l         CRYPTO_NAMESPACE(fe25519_to_10l)
#define fe25519_from_10l       CRYPTO_NAMESPACE(fe25519_from_10l)

#define fe25519_square(x,y) fe25519_nsquare(x,y,1)

typedef struct 
{
  unsigned long long l[4]; 
}
fe25519;

typedef struct {
  unsigned long long l[10]; 
}
fe25519_10l;

void fe25519_freeze(fe25519 *r);

void fe25519_unpack(fe25519 *r, const unsigned char x[32]);

void fe25519_pack(unsigned char r[32], const fe25519 *x);

void fe25519_mul(fe25519 *r, const fe25519 *x, const fe25519 *y);

void fe25519_nsquare(fe25519 *r, const fe25519 *x, long long n);

void fe25519_invert(fe25519 *r, const fe25519 *x);

void fe25519_to_10l(fe25519_10l *r, const fe25519 *x);

void fe25519_from_10l(fe25519 *r, const fe25519_10l *x);

#endif
