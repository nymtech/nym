#include <time.h>
#include <sys/time.h>
#include <unistd.h>

long long ticks_setup(void)
{
  return 1000000;
}

long long ticks(void)
{
  struct timeval t;
  long long result;
  gettimeofday(&t,(struct timezone *) 0);
  result = t.tv_sec;
  result *= 1000000;
  result += t.tv_usec;
  return result;
}
