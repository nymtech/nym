// version 20230105
// public domain
// djb

#include "cpucycles_internal.h"

#ifndef __i386__
#error this code is only for 32-bit x86 platforms
#endif

long long ticks(void)
{
  long long result;
  asm volatile(".byte 15;.byte 49" : "=A" (result));
  return result;
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_MAYBECYCLECOUNTER;
}
