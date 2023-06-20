#include <immintrin.h>
#include "inner.h"

#define uint64 crypto_uint64

static uint64 load_bigendian(const unsigned char *x)
{
  return __builtin_bswap64(*(uint64 *) x);
}

static void store_bigendian(unsigned char *x,uint64 u)
{
  *(uint64 *) x = __builtin_bswap64(u);
}

#define SHR(x,c) ((x) >> (c))
#define ROTR(x,c) (((x) >> (c)) | ((x) << (64 - (c))))
#define sigma0(x) (ROTR(x, 1) ^ ROTR(x, 8) ^ SHR(x,7))
#define sigma1(x) (ROTR(x,19) ^ ROTR(x,61) ^ SHR(x,6))

#define Ch(x,y,z) (z ^ (x & (y ^ z)))
#define Maj(x,y,z) ((x & (y ^ z)) ^ (y & z))
#define Sigma0(x) (ROTR(x,28) ^ ROTR(x,34) ^ ROTR(x,39))
#define Sigma1(x) (ROTR(x,14) ^ ROTR(x,18) ^ ROTR(x,41))

#define ALIGNED __attribute((aligned(32)))

#define load64(x) (*(uint64 *) (x))

#define store256(x,y) (*(volatile __m256i *) (x) = (y))

#define bigendian64 _mm256_set_epi8(8,9,10,11,12,13,14,15,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0,1,2,3,4,5,6,7)

#define PREEXPANDx4(X0,X9,X1) \
      X0 = _mm256_add_epi64(X0, \
            _mm256_srli_epi64(X1,1) ^ _mm256_slli_epi64(X1,63) ^ \
            _mm256_srli_epi64(X1,8) ^ _mm256_slli_epi64(X1,56) ^ \
            _mm256_srli_epi64(X1,7) \
            ); \
      X0 = _mm256_add_epi64(X0,X9);

#define POSTEXPANDx4(X0,W0,W2,W14) \
      W0 = ( \
        _mm256_extracti128_si256(X0,0)); \
      W0 = _mm_add_epi64(W0, \
        _mm_srli_epi64(W14,19) ^ _mm_slli_epi64(W14,45) ^ \
        _mm_srli_epi64(W14,61) ^ _mm_slli_epi64(W14,3) ^ \
        _mm_srli_epi64(W14,6)); \
      W2 = ( \
        _mm256_extracti128_si256(X0,1)); \
      W2 = _mm_add_epi64(W2, \
        _mm_srli_epi64(W0,19) ^ _mm_slli_epi64(W0,45) ^ \
        _mm_srli_epi64(W0,61) ^ _mm_slli_epi64(W0,3) ^ \
        _mm_srli_epi64(W0,6)); \
      X0 = _mm256_insertf128_si256(_mm256_castsi128_si256(W0),W2,1);

#define ROUND0(i,r0,r1,r2,r3,r4,r5,r6,r7) \
      r7 += load64(&wc[i]); \
      r7 += Ch(r4,r5,r6); \
      r7 += Sigma1(r4); \
      r3 += r7; \
      r7 += Maj(r2,r0,r1); \
      r7 += Sigma0(r0); \

#define ROUND1(i,r0,r1,r2,r3,r4,r5,r6,r7) \
      r7 += load64(&wc[i]); \
      r7 += Ch(r4,r5,r6); \
      r7 += Sigma1(r4); \
      r3 += r7; \
      r7 += Maj(r0,r1,r2); \
      r7 += Sigma0(r0); \

int inner(unsigned char *statebytes,const unsigned char *in,unsigned int inlen,const uint64 *constants)
{
  ALIGNED uint64 state[8];
  ALIGNED uint64 w[20];
  ALIGNED uint64 wc[16]; /* w[i]+constants[i] */
  uint64 r0,r1,r2,r3,r4,r5,r6,r7;
  __m128i W0,W2,W4,W6,W8,W10,W12,W14;
  __m256i X0,X1,X4,X5,X8,X9,X12,X13;
  __m256i D0,D4,D8,D12;
  int i;

  state[0] = r0 = load_bigendian(statebytes);
  state[1] = r1 = load_bigendian(statebytes+8);
  state[2] = r2 = load_bigendian(statebytes+16);
  state[3] = r3 = load_bigendian(statebytes+24);
  state[4] = r4 = load_bigendian(statebytes+32);
  state[5] = r5 = load_bigendian(statebytes+40);
  state[6] = r6 = load_bigendian(statebytes+48);
  state[7] = r7 = load_bigendian(statebytes+56);

  do {
        X0 = _mm256_loadu_si256((void *) (in+0));
        X0 = _mm256_shuffle_epi8(X0,bigendian64);
        D0 = _mm256_loadu_si256((void *) &constants[0]);
        D0 = _mm256_add_epi64(X0,D0);
        store256(&wc[0],D0);
        store256(&w[0],X0);
        store256(&w[16],X0);

        X4 = _mm256_loadu_si256((void *) (in+32));
        X4 = _mm256_shuffle_epi8(X4,bigendian64);
        D4 = _mm256_loadu_si256((void *) &constants[4]);
        D4 = _mm256_add_epi64(X4,D4);
        store256(&wc[4],D4);
        store256(&w[4],X4);

      ROUND0(0,r0,r1,r2,r3,r4,r5,r6,r7)
      ROUND1(1,r7,r0,r1,r2,r3,r4,r5,r6)

        X8 = _mm256_loadu_si256((void *) (in+64));
        X8 = _mm256_shuffle_epi8(X8,bigendian64);
        D8 = _mm256_loadu_si256((void *) &constants[8]);
        D8 = _mm256_add_epi64(X8,D8);
        store256(&wc[8],D8);
        store256(&w[8],X8);

      ROUND0(2,r6,r7,r0,r1,r2,r3,r4,r5)
      ROUND1(3,r5,r6,r7,r0,r1,r2,r3,r4)

      ROUND0(4,r4,r5,r6,r7,r0,r1,r2,r3)
      ROUND1(5,r3,r4,r5,r6,r7,r0,r1,r2)

        X12 = _mm256_loadu_si256((void *) (in+96));
        X12 = _mm256_shuffle_epi8(X12,bigendian64);
        D12 = _mm256_loadu_si256((void *) &constants[12]);
        D12 = _mm256_add_epi64(X12,D12);
        store256(&wc[12],D12);
        store256(&w[12],X12);

      ROUND0(6,r2,r3,r4,r5,r6,r7,r0,r1)
      ROUND1(7,r1,r2,r3,r4,r5,r6,r7,r0)

      ROUND0(8,r0,r1,r2,r3,r4,r5,r6,r7)
      ROUND1(9,r7,r0,r1,r2,r3,r4,r5,r6)


    for (i = 4;i > 0;--i) {

          constants += 16;

          X1 = _mm256_loadu_si256((void *) (w+1));
          X9 = _mm256_loadu_si256((void *) (w+9));
          PREEXPANDx4(X0,X9,X1)

          W14 = _mm_loadu_si128((void *) (w+14));
          POSTEXPANDx4(X0,W0,W2,W14)

          D0 = _mm256_loadu_si256((void *) &constants[0]);
          D0 = _mm256_add_epi64(X0,D0);
          store256(&wc[0],D0);
          store256(w+16,X0);
          store256(w+0,X0);

      ROUND0(10,r6,r7,r0,r1,r2,r3,r4,r5)
      ROUND1(11,r5,r6,r7,r0,r1,r2,r3,r4)

      ROUND0(12,r4,r5,r6,r7,r0,r1,r2,r3)
      ROUND1(13,r3,r4,r5,r6,r7,r0,r1,r2)

          X5 = _mm256_loadu_si256((void *) (w+5));
          X13 = _mm256_loadu_si256((void *) (w+13));
          PREEXPANDx4(X4,X13,X5)

          W2 = _mm_loadu_si128((void *) (w+2));
          POSTEXPANDx4(X4,W4,W6,W2)

          D4 = _mm256_loadu_si256((void *) &constants[4]);
          D4 = _mm256_add_epi64(X4,D4);
          store256(&wc[4],D4);
          store256(w+4,X4);

      ROUND0(14,r2,r3,r4,r5,r6,r7,r0,r1)
      ROUND1(15,r1,r2,r3,r4,r5,r6,r7,r0)

      ROUND0(0,r0,r1,r2,r3,r4,r5,r6,r7)
      ROUND1(1,r7,r0,r1,r2,r3,r4,r5,r6)

          X9 = _mm256_loadu_si256((void *) (w+9));
          X1 = _mm256_loadu_si256((void *) (w+1));
          PREEXPANDx4(X8,X1,X9)

          W6 = _mm_loadu_si128((void *) (w+6));
          POSTEXPANDx4(X8,W8,W10,W6)

          D8 = _mm256_loadu_si256((void *) &constants[8]);
          D8 = _mm256_add_epi64(X8,D8);
          store256(&wc[8],D8);
          store256(w+8,X8);

      ROUND0(2,r6,r7,r0,r1,r2,r3,r4,r5)
      ROUND1(3,r5,r6,r7,r0,r1,r2,r3,r4)

      ROUND0(4,r4,r5,r6,r7,r0,r1,r2,r3)
      ROUND1(5,r3,r4,r5,r6,r7,r0,r1,r2)

          X13 = _mm256_loadu_si256((void *) (w+13));
          X5 = _mm256_loadu_si256((void *) (w+5));
          PREEXPANDx4(X12,X5,X13)

          W10 = _mm_loadu_si128((void *) (w+10));
          POSTEXPANDx4(X12,W12,W14,W10)

          D12 = _mm256_loadu_si256((void *) &constants[12]);
          D12 = _mm256_add_epi64(X12,D12);
          store256(&wc[12],D12);
          store256(w+12,X12);

      ROUND0(6,r2,r3,r4,r5,r6,r7,r0,r1)
      ROUND1(7,r1,r2,r3,r4,r5,r6,r7,r0)

      ROUND0(8,r0,r1,r2,r3,r4,r5,r6,r7)
      ROUND1(9,r7,r0,r1,r2,r3,r4,r5,r6)

    }

    {
      ROUND0(10,r6,r7,r0,r1,r2,r3,r4,r5)
      ROUND1(11,r5,r6,r7,r0,r1,r2,r3,r4)

      ROUND0(12,r4,r5,r6,r7,r0,r1,r2,r3)
      ROUND1(13,r3,r4,r5,r6,r7,r0,r1,r2)

      ROUND0(14,r2,r3,r4,r5,r6,r7,r0,r1)
      ROUND1(15,r1,r2,r3,r4,r5,r6,r7,r0)
    }


    constants -= 64;

    r0 += state[0]; state[0] = r0;
    r1 += state[1]; state[1] = r1;
    r2 += state[2]; state[2] = r2;
    r3 += state[3]; state[3] = r3;
    r4 += state[4]; state[4] = r4;
    r5 += state[5]; state[5] = r5;
    r6 += state[6]; state[6] = r6;
    r7 += state[7]; state[7] = r7;

    in += 128;
    inlen -= 128;
  } while (inlen >= 128);

  for (i = 0;i < 8;++i)
    store_bigendian(statebytes+8*i,state[i]);

  return inlen;
}
