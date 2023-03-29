#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <time.h>
#include <sys/time.h>
#include <sys/types.h>
#include <cpucycles.h>

#define TIMINGS 63
static long long t[TIMINGS+1];

static void t_print(void)
{
  long long median = 0;
  long long i,j;

  for (i = 0;i < TIMINGS;++i)
    t[i] = t[i+1]-t[i];
  for (j = 0;j < TIMINGS;++j) {
    long long belowj = 0;
    long long abovej = 0;
    for (i = 0;i < TIMINGS;++i) if (t[i] < t[j]) ++belowj;
    for (i = 0;i < TIMINGS;++i) if (t[i] > t[j]) ++abovej;
    if (belowj*2 < TIMINGS && abovej*2 < TIMINGS) {
      median = t[j];
      break;
    }
  }
  printf(" %lld ",median);
  for (i = 0;i < TIMINGS;++i)
    printf("%+lld",t[i]-median);
  printf("\n");
  fflush(stdout);
}

static long long microseconds(void)
{
  struct timeval t;
  long long result;
  gettimeofday(&t,(struct timezone *) 0);
  result = t.tv_sec;
  result *= 1000000;
  result += t.tv_usec;
  return result;
}

static volatile int v;

static void measure_cpucycles(void)
{
  long long loops,i,j;

  printf("cpucycles persecond %lld\n",cpucycles_persecond());
  printf("cpucycles implementation %s\n",cpucycles_implementation());

  for (i = 0;i <= TIMINGS;++i)
    t[i] = cpucycles();
  printf("cpucycles median"); t_print();

  for (loops = 1024;loops <= 1048576;loops *= 2) {
    long long t00,t01,t10,t11;
    long long m0,m1;
    double ratiobelow,ratioabove;

    t00 = cpucycles();
    m0 = microseconds();
    t01 = cpucycles();

    for (j = 0;j < loops;++j) v = 0;

    t10 = cpucycles();
    m1 = microseconds();
    t11 = cpucycles();

    if (t01 < t00) continue;
    if (t10 < t01) continue;
    if (t11 < t10) continue;
    if (m1 <= m0+2) continue;

    ratiobelow = floor((1000000.0*(t10-t01))/(m1+1-m0));
    ratioabove = ceil((1000000.0*(t11-t00))/(m1-m0-1));

    printf("cpucycles observed persecond %.0lf...%.0lf with %lld loops %lld microseconds\n",ratiobelow,ratioabove,loops,m1-m0);
  }
}

int main(int argc,char **argv)
{
  cpucycles_tracesetup();
  printf("cpucycles version %s\n",cpucycles_version());
  measure_cpucycles();
  return 0;
}
