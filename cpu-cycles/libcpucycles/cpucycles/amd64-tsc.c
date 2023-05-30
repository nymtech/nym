// version 20230105
// public domain
// djb

#ifdef _MSC_VER
#include <intrin.h>
#else
#include <x86intrin.h>
#endif

#include "cpucycles_internal.h"

long long ticks(void)
{
  return __rdtsc();
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_MAYBECYCLECOUNTER;
}
