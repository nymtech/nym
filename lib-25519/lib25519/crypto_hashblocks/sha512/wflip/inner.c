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

int inner(unsigned char *statebytes,const unsigned char *in,unsigned int inlen,const uint64 *constants)
{
  uint64 state[8];
  uint64 r0,r1,r2,r3,r4,r5,r6,r7;
  uint64 w0,w1,w2,w3,w4,w5,w6,w7,w8,w9,w10,w11,w12,w13,w14,w15;
  uint64 w0_next,w1_next,w2_next,w3_next,w4_next,w5_next,w6_next,w7_next;
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
    w0 = load_bigendian(in);
    w1 = load_bigendian(in+8);
    w2 = load_bigendian(in+16);
    w3 = load_bigendian(in+24);
    w4 = load_bigendian(in+32);
    w5 = load_bigendian(in+40);
    w6 = load_bigendian(in+48);
    w7 = load_bigendian(in+56);
    w0_next = load_bigendian(in+64);
    w1_next = load_bigendian(in+72);
    w2_next = load_bigendian(in+80);
    w3_next = load_bigendian(in+88);
    w4_next = load_bigendian(in+96);
    w5_next = load_bigendian(in+104);
    w6_next = load_bigendian(in+112);
    w7_next = load_bigendian(in+120);

    i = 0;

    for (;;) {
      r7 += w0 + constants[0] + Sigma1(r4) + Ch(r4,r5,r6);
      r3 += r7;
      r7 += Sigma0(r0) + Maj(r0,r1,r2);
      r6 += w1 + constants[1] + Sigma1(r3) + Ch(r3,r4,r5);
      r2 += r6;
      r6 += Sigma0(r7) + Maj(r7,r0,r1);
      r5 += w2 + constants[2] + Sigma1(r2) + Ch(r2,r3,r4);
      r1 += r5;
      r5 += Sigma0(r6) + Maj(r6,r7,r0);
      r4 += w3 + constants[3] + Sigma1(r1) + Ch(r1,r2,r3);
      r0 += r4;
      r4 += Sigma0(r5) + Maj(r5,r6,r7);
      r3 += w4 + constants[4] + Sigma1(r0) + Ch(r0,r1,r2);
      r7 += r3;
      r3 += Sigma0(r4) + Maj(r4,r5,r6);
      r2 += w5 + constants[5] + Sigma1(r7) + Ch(r7,r0,r1);
      r6 += r2;
      r2 += Sigma0(r3) + Maj(r3,r4,r5);
      r1 += w6 + constants[6] + Sigma1(r6) + Ch(r6,r7,r0);
      r5 += r1;
      r1 += Sigma0(r2) + Maj(r2,r3,r4);
      r0 += w7 + constants[7] + Sigma1(r5) + Ch(r5,r6,r7);
      r4 += r0;
      r0 += Sigma0(r1) + Maj(r1,r2,r3);

      if (i == 72) break;

      if (i == 64) {
        w0 = w0_next;
        w1 = w1_next;
        w2 = w2_next;
        w3 = w3_next;
        w4 = w4_next;
        w5 = w5_next;
        w6 = w6_next;
        w7 = w7_next;
      } else {
        w8 = w0;
        w9 = w1;
        w10 = w2;
        w11 = w3;
        w12 = w4;
        w13 = w5;
        w14 = w6;
        w15 = w7;

        w0 = w0_next;
        w1 = w1_next;
        w2 = w2_next;
        w3 = w3_next;
        w4 = w4_next;
        w5 = w5_next;
        w6 = w6_next;
        w7 = w7_next;

        w8  += sigma1(w6)  + w1  + sigma0(w9);
        w9  += sigma1(w7)  + w2  + sigma0(w10);
        w10 += sigma1(w8)  + w3  + sigma0(w11);
        w11 += sigma1(w9)  + w4  + sigma0(w12);
        w12 += sigma1(w10) + w5  + sigma0(w13);
        w13 += sigma1(w11) + w6  + sigma0(w14);
        w14 += sigma1(w12) + w7  + sigma0(w15);
        w15 += sigma1(w13) + w8  + sigma0(w0);
  
        w0_next = w8;
        w1_next = w9;
        w2_next = w10;
        w3_next = w11;
        w4_next = w12;
        w5_next = w13;
        w6_next = w14;
        w7_next = w15;
      }

      i += 8;
      constants += 8;
    }

    constants -= 72;

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
