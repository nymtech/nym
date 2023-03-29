// version 20230105
// public domain
// djb

// adapted from supercop/cpucycles/riscv.c
// which has code from djb and Romain Dolbeau

#include "cpucycles_internal.h"

#ifndef __riscv_xlen
#error this code is only for riscv platforms
#endif

#if __riscv_xlen != 64
#error this code is only for riscv64 platforms
#endif

long long ticks(void)
{
  long long result;
  asm volatile("rdcycle %0" : "=r" (result));
  return result;
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_CYCLECOUNTER;
}
