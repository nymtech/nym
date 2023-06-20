#include <immintrin.h>
#include "crypto_hashblocks_sha512.h"
#include "crypto_hash.h"

#define blocks crypto_hashblocks_sha512

#define ALIGNED __attribute((aligned(32)))

static const ALIGNED unsigned char iv[64] = {
  0x6a,0x09,0xe6,0x67,0xf3,0xbc,0xc9,0x08,
  0xbb,0x67,0xae,0x85,0x84,0xca,0xa7,0x3b,
  0x3c,0x6e,0xf3,0x72,0xfe,0x94,0xf8,0x2b,
  0xa5,0x4f,0xf5,0x3a,0x5f,0x1d,0x36,0xf1,
  0x51,0x0e,0x52,0x7f,0xad,0xe6,0x82,0xd1,
  0x9b,0x05,0x68,0x8c,0x2b,0x3e,0x6c,0x1f,
  0x1f,0x83,0xd9,0xab,0xfb,0x41,0xbd,0x6b,
  0x5b,0xe0,0xcd,0x19,0x13,0x7e,0x21,0x79
} ;

typedef unsigned long long uint64;

#define load256(x) (_mm256_loadu_si256((void *) (x)))
#define store256(x,y) (_mm256_storeu_si256((void *) (x),y))

void crypto_hash(unsigned char *out,const unsigned char *in,long long inlen)
{
  ALIGNED unsigned char h[64];
  ALIGNED unsigned char padded[256];
  unsigned long long i;
  unsigned long long bytes = inlen;
  __m256i X0,X1;

  X0 = load256(iv);
  X1 = load256(iv + 32);

  store256(h,X0);
  store256(h + 32,X1);

  blocks(h,in,inlen);
  in += inlen;
  inlen &= 127;
  in -= inlen;

  X0 ^= X0;

  if (inlen < 112) {
    store256(padded,X0);
    store256(padded + 32,X0);
    store256(padded + 64,X0);
    store256(padded + 96,X0);

    for (i = 0;i < inlen;++i) padded[i] = in[i];
    padded[inlen] = 0x80;

    padded[119] = bytes >> 61;
    *(uint64 *) (padded + 120) = __builtin_bswap64(bytes << 3);
    blocks(h,padded,128);
  } else {
    store256(padded + 96,X0);
    store256(padded + 128,X0);
    store256(padded + 160,X0);
    store256(padded + 192,X0);
    store256(padded + 224,X0);

    for (i = 0;i < inlen;++i) padded[i] = in[i];
    padded[inlen] = 0x80;

    padded[247] = bytes >> 61;
    *(uint64 *) (padded + 248) = __builtin_bswap64(bytes << 3);
    blocks(h,padded,256);
  }

  X0 = load256(h);
  X1 = load256(h + 32);

  store256(out,X0);
  store256(out + 32,X1);
}
