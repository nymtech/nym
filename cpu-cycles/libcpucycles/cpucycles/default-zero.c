// version 20230105
// public domain
// djb

#include "cpucycles_internal.h"

long long ticks_setup(void)
{
  return cpucycles_SKIP;
}

long long ticks(void)
{
  return 0;
}
