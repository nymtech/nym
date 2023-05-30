// version 20230105
// public domain
// djb

#include "cpucycles_internal.h"

long long ticks_setup(void)
{
  return 1000000;
}

long long ticks(void)
{
  return cpucycles_microseconds();
}
