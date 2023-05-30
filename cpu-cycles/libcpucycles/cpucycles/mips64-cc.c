// version 20230105
// public domain
// djb
// adapted from supercop/cpucycles/mips.c

// mips32 release 2 instruction rdhwr
// 7c02103b: read hwr#2 (cycle count) into $2
// 7c02183b: read hwr#3 (cycle-count multiplier) into $2

#include "cpucycles_internal.h"

static unsigned int multiplier = 0;

static long long multiplier_set(void)
{
  asm volatile(".long 0x7c02183b; move %0,$2" : "=r"(multiplier) : : "$2");
  return multiplier;
}

long long ticks(void)
{
  unsigned int result;
  asm volatile(".long 0x7c02103b; move %0,$2" : "=r"(result) :: "$2");
  result *= multiplier;
  return (unsigned long long) result;
}

long long ticks_setup(void)
{
  if (!cpucycles_works(multiplier_set)) return cpucycles_SKIP;
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_EXTEND32;
}
