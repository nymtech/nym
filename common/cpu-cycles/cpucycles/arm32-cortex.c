// version 20230105
// public domain
// djb
// adapted from supercop/cpucycles/cortex.c

#include "cpucycles_internal.h"

long long ticks(void)
{
  unsigned int result;
  asm volatile("mrc p15, 0, %0, c9, c13, 0" : "=r"(result));
  return (unsigned long long) result;
}

static long enable(void)
{
  asm volatile("mcr p15, 0, %0, c9, c12, 0" :: "r"(17));
  asm volatile("mcr p15, 0, %0, c9, c12, 1" :: "r"(0x8000000f));
  asm volatile("mcr p15, 0, %0, c9, c12, 3" :: "r"(0x8000000f));
}

long long ticks_setup(void)
{
  if (!cpucycles_works(enable)) return cpucycles_SKIP;
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_EXTEND32;
}
