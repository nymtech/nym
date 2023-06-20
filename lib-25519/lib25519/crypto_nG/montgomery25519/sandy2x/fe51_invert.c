// linker define fe51_invert
// linker use fe51_pack

#include "crypto_pow_inv25519.h"
#include "fe51.h"

static void fe51_unpack(fe51 *r, const unsigned char x[32])
{
  r->v[0] = x[0];
  r->v[0] += (unsigned long long)x[1] << 8;
  r->v[0] += (unsigned long long)x[2] << 16;
  r->v[0] += (unsigned long long)x[3] << 24;
  r->v[0] += (unsigned long long)x[4] << 32;
  r->v[0] += (unsigned long long)x[5] << 40;
  r->v[0] += ((unsigned long long)x[6] & 7) << 48;

  r->v[1] = x[6] >> 3;
  r->v[1] += (unsigned long long)x[7] << 5;
  r->v[1] += (unsigned long long)x[8] << 13;
  r->v[1] += (unsigned long long)x[9] << 21;
  r->v[1] += (unsigned long long)x[10] << 29;
  r->v[1] += (unsigned long long)x[11] << 37;
  r->v[1] += ((unsigned long long)x[12] & 63) << 45;
  
  r->v[2] = x[12] >> 6;
  r->v[2] += (unsigned long long)x[13] <<  2;
  r->v[2] += (unsigned long long)x[14] << 10;
  r->v[2] += (unsigned long long)x[15] << 18;
  r->v[2] += (unsigned long long)x[16] << 26;
  r->v[2] += (unsigned long long)x[17] << 34;
  r->v[2] += (unsigned long long)x[18] << 42;
  r->v[2] += ((unsigned long long)x[19] & 1) << 50;
  
  r->v[3] = x[19] >> 1;
  r->v[3] += (unsigned long long)x[20] <<  7;
  r->v[3] += (unsigned long long)x[21] << 15;
  r->v[3] += (unsigned long long)x[22] << 23;
  r->v[3] += (unsigned long long)x[23] << 31;
  r->v[3] += (unsigned long long)x[24] << 39;
  r->v[3] += ((unsigned long long)x[25] & 15) << 47;
  
  r->v[4] = x[25] >> 4;
  r->v[4] += (unsigned long long)x[26] <<  4;
  r->v[4] += (unsigned long long)x[27] << 12;
  r->v[4] += (unsigned long long)x[28] << 20;
  r->v[4] += (unsigned long long)x[29] << 28;
  r->v[4] += (unsigned long long)x[30] << 36;
  r->v[4] += ((unsigned long long)x[31] & 127) << 44;
}

void fe51_invert(fe51 *r, const fe51 *x)
{
  unsigned char s[32];
  fe51_pack(s,x);
  crypto_pow_inv25519(s,s);
  fe51_unpack(r,s);
}
