// version 20230105
// public domain
// djb

// adapted from supercop/cpucycles/riscv.c
// which has code from djb and Romain Dolbeau

#include "cpucycles_internal.h"

#ifndef __riscv_xlen
#error this code is only for riscv platforms
#endif

#if __riscv_xlen != 32
#error this code is only for riscv32 platforms
#endif

long long ticks(void)
{
  unsigned int low, high, newhigh;
  unsigned long long result;

  asm volatile( "start%=:\n"
                "rdcycleh %0\n"
                "rdcycle %1\n"
                "rdcycleh %2\n"
                "bne %0, %2, start%=\n"
                : "=r"(high), "=r"(low), "=r"(newhigh));
  result = high;
  result <<= 32;
  result |= low;
  return result;
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_CYCLECOUNTER;
}
