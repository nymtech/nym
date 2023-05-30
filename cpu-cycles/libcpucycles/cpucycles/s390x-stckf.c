// version 20230106
// public domain
// djb

// adapted from sparc64-rdtick.c

#include "cpucycles_internal.h"

long long ticks(void)
{
  long long result;
  asm volatile("stckf 0(%0)" :: "a"(&result) : "memory","cc");
  return result;
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return 4096000000; // manual says 2^12 per microsecond
}
