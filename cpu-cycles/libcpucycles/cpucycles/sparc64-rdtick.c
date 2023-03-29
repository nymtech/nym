// version 20230105
// public domain
// djb

// adapted from supercop/cpucycles/sparccpuinfo.c

#include "cpucycles_internal.h"

#if defined(__sparcv8) || defined(__sparcv8plus)
#error this code is only for sparc64 platforms
#endif

long long ticks(void)
{
  long long result;
  asm volatile("rd %%tick,%0" : "=r" (result));
  return result;
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_CYCLECOUNTER;
}
