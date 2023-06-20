#include <time.h>
#include <sys/time.h>

long long ticks_setup(void)
{
  return 1000000000;
}

long long ticks(void)
{
  struct timespec t;
  long long result;
  clock_gettime(CLOCK_MONOTONIC,&t);
  result = t.tv_sec;
  result *= 1000000000;
  result += t.tv_nsec;
  return result;
}
