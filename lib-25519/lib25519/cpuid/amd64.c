#include <stdio.h>

#define CPUID(func,leaf,a,b,c,d) \
  __asm("cpuid":"=a"(a),"=b"(b),"=c"(c),"=d"(d):"a"(func),"c"(leaf):)

__attribute__((visibility("default")))
void lib25519_cpuid(unsigned int *result,long long resultlen)
{
  unsigned int a,b,c,d;
  unsigned int cpuidmax,extendedcpuidmax;
  int havexgetbv = 0;

  CPUID(0,0,a,b,c,d);
  cpuidmax = a;
  if (resultlen > 0) { *result++ = b; --resultlen; }
  if (resultlen > 0) { *result++ = c; --resultlen; }
  if (resultlen > 0) { *result++ = d; --resultlen; }

  a = b = c = d = 0;
  CPUID(0x80000000,0,a,b,c,d);
  extendedcpuidmax = a;

  a = b = c = d = 0;
  if (extendedcpuidmax >= 0x80000002) CPUID(0x80000002,0,a,b,c,d);
  if (resultlen > 0) { *result++ = a; --resultlen; }
  if (resultlen > 0) { *result++ = b; --resultlen; }
  if (resultlen > 0) { *result++ = c; --resultlen; }
  if (resultlen > 0) { *result++ = d; --resultlen; }

  a = b = c = d = 0;
  if (extendedcpuidmax >= 0x80000003) CPUID(0x80000003,0,a,b,c,d);
  if (resultlen > 0) { *result++ = a; --resultlen; }
  if (resultlen > 0) { *result++ = b; --resultlen; }
  if (resultlen > 0) { *result++ = c; --resultlen; }
  if (resultlen > 0) { *result++ = d; --resultlen; }

  a = b = c = d = 0;
  if (extendedcpuidmax >= 0x80000004) CPUID(0x80000004,0,a,b,c,d);
  if (resultlen > 0) { *result++ = a; --resultlen; }
  if (resultlen > 0) { *result++ = b; --resultlen; }
  if (resultlen > 0) { *result++ = c; --resultlen; }
  if (resultlen > 0) { *result++ = d; --resultlen; }

  a = b = c = d = 0;
  if (cpuidmax >= 1) CPUID(1,0,a,b,c,d);
  if (resultlen > 0) { *result++ = a; --resultlen; }
  if (resultlen > 0) { *result++ = b; --resultlen; }
  if (resultlen > 0) { *result++ = c; --resultlen; }
  if (resultlen > 0) { *result++ = d; --resultlen; }
  /* 27=osxsave; 28=avx */
  if (((1<<27)|(1<<28)) == (((1<<27)|(1<<28)) & c))
    havexgetbv = 1;

  a = b = c = d = 0;
  if (cpuidmax >= 7) CPUID(7,0,a,b,c,d);
  if (resultlen > 0) { *result++ = a; --resultlen; }
  if (resultlen > 0) { *result++ = b; --resultlen; }
  if (resultlen > 0) { *result++ = c; --resultlen; }
  if (resultlen > 0) { *result++ = d; --resultlen; }

  a = b = c = d = 0;
  if (extendedcpuidmax >= 0x80000001) CPUID(0x80000001,0,a,b,c,d);
  if (resultlen > 0) { *result++ = a; --resultlen; }
  if (resultlen > 0) { *result++ = b; --resultlen; }
  if (resultlen > 0) { *result++ = c; --resultlen; }
  if (resultlen > 0) { *result++ = d; --resultlen; }

  a = b = c = d = 0;
  if (havexgetbv) asm(".byte 15;.byte 1;.byte 208":"=a"(a),"=d"(d):"c"(0));
  if (resultlen > 0) { *result++ = a; --resultlen; }

  while (resultlen > 0) { *result++ = 0; --resultlen; }
}
