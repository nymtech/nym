/*
gcc has __builtin_cpu_supports("avx2")
but implemented it incorrectly until 2018:
https://gcc.gnu.org/bugzilla/show_bug.cgi?id=85100

as of 2022, many machines still have buggy versions of gcc

furthermore, why is checking just for avx2 enough?
has intel guaranteed that it will never introduce
a cpu with avx2 instructions and without (e.g.) sse4.2?

so manually check cpuid and xgetbv here
and include all the "lower" instruction sets
rather than trying to guess which ones are implied
*/

#define CPUID(func,leaf,a,b,c,d) \
  __asm("cpuid":"=a"(a),"=b"(b),"=c"(c),"=d"(d):"a"(func),"c"(leaf):)

#define WANT_1_3 ((1<<23)|(1<<25)|(1<<26))
/* 23=mmx; 25=sse; 26=sse2 */

#define WANT_1_2 ((1<<0)|(1<<9)|(1<<19)|(1<<20)|(1<<27)|(1<<28))
/* 0=sse3; 9=ssse3; 19=sse41; 20=sse42; 27=osxsave; 28=avx */

#define WANT_7_1 ((1<<3)|(1<<5)|(1<<8))
/* 3=bmi1; 5=avx2; 8=bmi2 */

#define WANT_XCR ((1<<1)|(1<<2))
/* 1=xmm; 2=ymm */

int supports(void)
{
  unsigned int cpuidmax,id0,id1,id2;
  unsigned int familymodelstepping;
  unsigned int feature0,feature1,feature2,feature3;
  unsigned int xcrlow,xcrhigh;

  CPUID(0,0,cpuidmax,id0,id1,id2);
  if (cpuidmax < 7) return 0;

  CPUID(1,0,familymodelstepping,feature1,feature2,feature3);
  if (WANT_1_2 != (WANT_1_2 & feature2)) return 0;
  if (WANT_1_3 != (WANT_1_3 & feature3)) return 0;

  CPUID(7,0,feature0,feature1,feature2,feature3);
  if (WANT_7_1 != (WANT_7_1 & feature1)) return 0;

  asm(".byte 15;.byte 1;.byte 208":"=a"(xcrlow),"=d"(xcrhigh):"c"(0));
  if (WANT_XCR != (WANT_XCR & xcrlow)) return 0;

  return 1;
}
