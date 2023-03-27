// version 20230105
// public domain
// djb

#include <mach/mach_time.h>
#include "cpucycles_internal.h"

long long ticks(void)
{
  return mach_absolute_time();
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_FINDMULTIPLIER;
}
