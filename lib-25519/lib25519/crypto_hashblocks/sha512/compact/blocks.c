#include "crypto_hashblocks.h"

typedef unsigned long long uint64;

static uint64 load_bigendian(const unsigned char *x)
{
  return
      (uint64) (x[7]) \
  | (((uint64) (x[6])) << 8) \
  | (((uint64) (x[5])) << 16) \
  | (((uint64) (x[4])) << 24) \
  | (((uint64) (x[3])) << 32) \
  | (((uint64) (x[2])) << 40) \
  | (((uint64) (x[1])) << 48) \
  | (((uint64) (x[0])) << 56)
  ;
}

static void store_bigendian(unsigned char *x,uint64 u)
{
  x[7] = u; u >>= 8;
  x[6] = u; u >>= 8;
  x[5] = u; u >>= 8;
  x[4] = u; u >>= 8;
  x[3] = u; u >>= 8;
  x[2] = u; u >>= 8;
  x[1] = u; u >>= 8;
  x[0] = u;
}

#define SHR(x,c) ((x) >> (c))
#define ROTR(x,c) (((x) >> (c)) | ((x) << (64 - (c))))

#define sigma0(x) (ROTR(x, 1) ^ ROTR(x, 8) ^ SHR(x,7))
#define sigma1(x) (ROTR(x,19) ^ ROTR(x,61) ^ SHR(x,6))

#define M(w0,w14,w9,w1) w0 = sigma1(w14) + w9 + sigma0(w1) + w0;

static void expand(uint64 *w)
{
  M(w[0] ,w[14],w[9] ,w[1] )
  M(w[1] ,w[15],w[10],w[2] )
  M(w[2] ,w[0] ,w[11],w[3] )
  M(w[3] ,w[1] ,w[12],w[4] )
  M(w[4] ,w[2] ,w[13],w[5] )
  M(w[5] ,w[3] ,w[14],w[6] )
  M(w[6] ,w[4] ,w[15],w[7] )
  M(w[7] ,w[5] ,w[0] ,w[8] )
  M(w[8] ,w[6] ,w[1] ,w[9] )
  M(w[9] ,w[7] ,w[2] ,w[10])
  M(w[10],w[8] ,w[3] ,w[11])
  M(w[11],w[9] ,w[4] ,w[12])
  M(w[12],w[10],w[5] ,w[13])
  M(w[13],w[11],w[6] ,w[14])
  M(w[14],w[12],w[7] ,w[15])
  M(w[15],w[13],w[8] ,w[0] )
}

#define Ch(x,y,z) (z ^ (x & (y ^ z)))
#define Maj(x,y,z) ((x & (y ^ z)) ^ (y & z))
#define Sigma0(x) (ROTR(x,28) ^ ROTR(x,34) ^ ROTR(x,39))
#define Sigma1(x) (ROTR(x,14) ^ ROTR(x,18) ^ ROTR(x,41))

#define F(r0,r1,r2,r3,r4,r5,r6,r7,w,k) \
  r7 += Sigma1(r4) + Ch(r4,r5,r6) + k + w; \
  r3 += r7; \
  r7 += Sigma0(r0) + Maj(r0,r1,r2);

static void handle(uint64 *r,const uint64 *w,const uint64 *c)
{
  F(r[0],r[1],r[2],r[3],r[4],r[5],r[6],r[7],w[0] ,c[0])
  F(r[7],r[0],r[1],r[2],r[3],r[4],r[5],r[6],w[1] ,c[1])
  F(r[6],r[7],r[0],r[1],r[2],r[3],r[4],r[5],w[2] ,c[2])
  F(r[5],r[6],r[7],r[0],r[1],r[2],r[3],r[4],w[3] ,c[3])
  F(r[4],r[5],r[6],r[7],r[0],r[1],r[2],r[3],w[4] ,c[4])
  F(r[3],r[4],r[5],r[6],r[7],r[0],r[1],r[2],w[5] ,c[5])
  F(r[2],r[3],r[4],r[5],r[6],r[7],r[0],r[1],w[6] ,c[6])
  F(r[1],r[2],r[3],r[4],r[5],r[6],r[7],r[0],w[7] ,c[7])
  F(r[0],r[1],r[2],r[3],r[4],r[5],r[6],r[7],w[8] ,c[8])
  F(r[7],r[0],r[1],r[2],r[3],r[4],r[5],r[6],w[9] ,c[9])
  F(r[6],r[7],r[0],r[1],r[2],r[3],r[4],r[5],w[10],c[10])
  F(r[5],r[6],r[7],r[0],r[1],r[2],r[3],r[4],w[11],c[11])
  F(r[4],r[5],r[6],r[7],r[0],r[1],r[2],r[3],w[12],c[12])
  F(r[3],r[4],r[5],r[6],r[7],r[0],r[1],r[2],w[13],c[13])
  F(r[2],r[3],r[4],r[5],r[6],r[7],r[0],r[1],w[14],c[14])
  F(r[1],r[2],r[3],r[4],r[5],r[6],r[7],r[0],w[15],c[15])
}

static const uint64 round[80] = {
  0x428a2f98d728ae22ULL
, 0x7137449123ef65cdULL
, 0xb5c0fbcfec4d3b2fULL
, 0xe9b5dba58189dbbcULL
, 0x3956c25bf348b538ULL
, 0x59f111f1b605d019ULL
, 0x923f82a4af194f9bULL
, 0xab1c5ed5da6d8118ULL
, 0xd807aa98a3030242ULL
, 0x12835b0145706fbeULL
, 0x243185be4ee4b28cULL
, 0x550c7dc3d5ffb4e2ULL
, 0x72be5d74f27b896fULL
, 0x80deb1fe3b1696b1ULL
, 0x9bdc06a725c71235ULL
, 0xc19bf174cf692694ULL
, 0xe49b69c19ef14ad2ULL
, 0xefbe4786384f25e3ULL
, 0x0fc19dc68b8cd5b5ULL
, 0x240ca1cc77ac9c65ULL
, 0x2de92c6f592b0275ULL
, 0x4a7484aa6ea6e483ULL
, 0x5cb0a9dcbd41fbd4ULL
, 0x76f988da831153b5ULL
, 0x983e5152ee66dfabULL
, 0xa831c66d2db43210ULL
, 0xb00327c898fb213fULL
, 0xbf597fc7beef0ee4ULL
, 0xc6e00bf33da88fc2ULL
, 0xd5a79147930aa725ULL
, 0x06ca6351e003826fULL
, 0x142929670a0e6e70ULL
, 0x27b70a8546d22ffcULL
, 0x2e1b21385c26c926ULL
, 0x4d2c6dfc5ac42aedULL
, 0x53380d139d95b3dfULL
, 0x650a73548baf63deULL
, 0x766a0abb3c77b2a8ULL
, 0x81c2c92e47edaee6ULL
, 0x92722c851482353bULL
, 0xa2bfe8a14cf10364ULL
, 0xa81a664bbc423001ULL
, 0xc24b8b70d0f89791ULL
, 0xc76c51a30654be30ULL
, 0xd192e819d6ef5218ULL
, 0xd69906245565a910ULL
, 0xf40e35855771202aULL
, 0x106aa07032bbd1b8ULL
, 0x19a4c116b8d2d0c8ULL
, 0x1e376c085141ab53ULL
, 0x2748774cdf8eeb99ULL
, 0x34b0bcb5e19b48a8ULL
, 0x391c0cb3c5c95a63ULL
, 0x4ed8aa4ae3418acbULL
, 0x5b9cca4f7763e373ULL
, 0x682e6ff3d6b2b8a3ULL
, 0x748f82ee5defb2fcULL
, 0x78a5636f43172f60ULL
, 0x84c87814a1f0ab72ULL
, 0x8cc702081a6439ecULL
, 0x90befffa23631e28ULL
, 0xa4506cebde82bde9ULL
, 0xbef9a3f7b2c67915ULL
, 0xc67178f2e372532bULL
, 0xca273eceea26619cULL
, 0xd186b8c721c0c207ULL
, 0xeada7dd6cde0eb1eULL
, 0xf57d4f7fee6ed178ULL
, 0x06f067aa72176fbaULL
, 0x0a637dc5a2c898a6ULL
, 0x113f9804bef90daeULL
, 0x1b710b35131c471bULL
, 0x28db77f523047d84ULL
, 0x32caab7b40c72493ULL
, 0x3c9ebe0a15c9bebcULL
, 0x431d67c49c100d4cULL
, 0x4cc5d4becb3e42b6ULL
, 0x597f299cfc657e2aULL
, 0x5fcb6fab3ad6faecULL
, 0x6c44198c4a475817ULL
};

int crypto_hashblocks(unsigned char *statebytes,const unsigned char *in,long long inlen)
{
  uint64 w[16];
  uint64 state[8];
  uint64 r[8];
  int i;

  for (i = 0;i < 8;++i)
    state[i] = r[i] = load_bigendian(statebytes+8*i);

  while (inlen >= 128) {
    for (i = 0;i < 16;++i)
      w[i] = load_bigendian(in+8*i);

    handle(r,w,round+0);

    expand(w);

    handle(r,w,round+16);

    expand(w);

    handle(r,w,round+32);

    expand(w);

    handle(r,w,round+48);

    expand(w);

    handle(r,w,round+64);

    for (i = 0;i < 8;++i) {
      uint64 x = r[i]+state[i];
      state[i] = x;
      r[i] = x;
    }
      
    in += 128;
    inlen -= 128;
  }

  for (i = 0;i < 8;++i)
    store_bigendian(statebytes+8*i,state[i]);

  return inlen;
}
