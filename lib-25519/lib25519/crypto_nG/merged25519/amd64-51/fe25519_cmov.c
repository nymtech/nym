// linker define fe25519_cmov

#include "fe25519.h"

void fe25519_cmov(fe25519 *f,const fe25519 *g,unsigned char b)
{
  unsigned long long f0 = f->v[0];
  unsigned long long f1 = f->v[1];
  unsigned long long f2 = f->v[2];
  unsigned long long f3 = f->v[3];
  unsigned long long f4 = f->v[4];
  unsigned long long g0 = g->v[0];
  unsigned long long g1 = g->v[1];
  unsigned long long g2 = g->v[2];
  unsigned long long g3 = g->v[3];
  unsigned long long g4 = g->v[4];
  unsigned long long x0 = f0 ^ g0;
  unsigned long long x1 = f1 ^ g1;
  unsigned long long x2 = f2 ^ g2;
  unsigned long long x3 = f3 ^ g3;
  unsigned long long x4 = f4 ^ g4;
  unsigned long long mask = -(unsigned long long) b;
  x0 &= mask;
  x1 &= mask;
  x2 &= mask;
  x3 &= mask;
  x4 &= mask;
  f->v[0] = f0 ^ x0;
  f->v[1] = f1 ^ x1;
  f->v[2] = f2 ^ x2;
  f->v[3] = f3 ^ x3;
  f->v[4] = f4 ^ x4;
}
