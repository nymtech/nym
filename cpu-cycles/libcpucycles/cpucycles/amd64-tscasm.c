// version 20230105
// public domain
// djb
// adapted from supercop/cpucycles/amd64tscfreq.c

#include "cpucycles_internal.h"

long long ticks(void)
{
  unsigned long long result;
  asm volatile(".byte 15;.byte 49;shlq $32,%%rdx;orq %%rdx,%%rax"
    : "=a"(result) :: "%rdx");
  return result;
}

long long ticks_setup(void)
{
  if (!cpucycles_works(ticks)) return cpucycles_SKIP;
  return cpucycles_MAYBECYCLECOUNTER;
}
