/* WARNING: auto-generated (by autogen-speed); do not edit */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <time.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/resource.h>
#include "cpucycles.h" /* -lcpucycles */
#include "lib25519.h" /* -l25519 */
#include "randombytes.h" /* -lrandombytes_kernel */

static const char *targeto = 0;
static const char *targetp = 0;
static const char *targeti = 0;

#include "limits.inc"

static unsigned char *alignedcalloc(unsigned long long len)
{
  unsigned char *x = (unsigned char *) calloc(1,len + 128);
  if (!x) abort();
  /* will never deallocate so shifting is ok */
  x += 63 & (-(unsigned long) x);
  return x;
}

#define TIMINGS 15
static long long t[TIMINGS+1];

static void t_print(const char *op,long long impl,long long len)
{
  long long median = 0;

  printf("%s",op);
  if (impl >= 0)
    printf(" %lld",impl);
  else
    printf(" selected");
  printf(" %lld",len);
  for (long long i = 0;i < TIMINGS;++i)
    t[i] = t[i+1]-t[i];
  for (long long j = 0;j < TIMINGS;++j) {
    long long belowj = 0;
    long long abovej = 0;
    for (long long i = 0;i < TIMINGS;++i) if (t[i] < t[j]) ++belowj;
    for (long long i = 0;i < TIMINGS;++i) if (t[i] > t[j]) ++abovej;
    if (belowj*2 < TIMINGS && abovej*2 < TIMINGS) {
      median = t[j];
      break;
    }
  }
  printf(" %lld ",median);
  for (long long i = 0;i < TIMINGS;++i)
    printf("%+lld",t[i]-median);
  printf("\n");
  fflush(stdout);
}

#define MAXTEST_BYTES 65536

static void measure_cpucycles(void)
{
  printf("cpucycles selected persecond %lld\n",cpucycles_persecond());
  printf("cpucycles selected implementation %s\n",cpucycles_implementation());

  for (long long i = 0;i <= TIMINGS;++i)
    t[i] = cpucycles();
  t_print("cpucycles",-1,0);
}

static void measure_randombytes(void)
{
  unsigned char *m = alignedcalloc(MAXTEST_BYTES);
  long long mlen = 0;

  while (mlen < MAXTEST_BYTES) {
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      randombytes(m,mlen);
    }
    t_print("randombytes",-1,mlen);
    mlen += 1+mlen/2;
  }
}

static void measure_verify_32(void)
{
  if (targeto && strcmp(targeto,"verify")) return;
  if (targetp && strcmp(targetp,"32")) return;
  unsigned char *x = alignedcalloc(lib25519_verify_32_BYTES);
  unsigned char *y = alignedcalloc(lib25519_verify_32_BYTES);

  for (long long impl = -1;impl < lib25519_numimpl_verify_32();++impl) {
    int (*crypto_verify)(const unsigned char *,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_verify_32_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_verify = lib25519_dispatch_verify_32(impl);
      printf("verify_32 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_verify_32_implementation(impl),lib25519_dispatch_verify_32_compiler(impl));
    } else {
      crypto_verify = lib25519_verify_32;
      printf("verify_32 selected implementation %s compiler %s\n",lib25519_verify_32_implementation(),lib25519_verify_32_compiler());
    }
    randombytes(x,lib25519_verify_32_BYTES);
    randombytes(y,lib25519_verify_32_BYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_verify(x,y);
    }
    t_print("verify_32",impl,lib25519_verify_32_BYTES);
  }
}

static void measure_hashblocks_sha512(void)
{
  if (targeto && strcmp(targeto,"hashblocks")) return;
  if (targetp && strcmp(targetp,"sha512")) return;
  unsigned char *h = alignedcalloc(lib25519_hashblocks_sha512_STATEBYTES);
  unsigned char *m = alignedcalloc(MAXTEST_BYTES);
  long long mlen;

  for (long long impl = -1;impl < lib25519_numimpl_hashblocks_sha512();++impl) {
    int (*crypto_hashblocks)(unsigned char *,const unsigned char *,long long);
    if (targeti && strcmp(targeti,lib25519_dispatch_hashblocks_sha512_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_hashblocks = lib25519_dispatch_hashblocks_sha512(impl);
      printf("hashblocks_sha512 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_hashblocks_sha512_implementation(impl),lib25519_dispatch_hashblocks_sha512_compiler(impl));
    } else {
      crypto_hashblocks = lib25519_hashblocks_sha512;
      printf("hashblocks_sha512 selected implementation %s compiler %s\n",lib25519_hashblocks_sha512_implementation(),lib25519_hashblocks_sha512_compiler());
    }
    randombytes(h,lib25519_hashblocks_sha512_STATEBYTES);
    randombytes(m,MAXTEST_BYTES);
    mlen = 0;
    while (mlen <= MAXTEST_BYTES) {
      randombytes(m,mlen);
      for (long long i = 0;i <= TIMINGS;++i) {
        t[i] = cpucycles();
        crypto_hashblocks(h,m,mlen);
      }
      t_print("hashblocks_sha512",impl,mlen);
      mlen += 1+mlen/2;
    }
  }
}

static void measure_hash_sha512(void)
{
  if (targeto && strcmp(targeto,"hash")) return;
  if (targetp && strcmp(targetp,"sha512")) return;
  unsigned char *h = alignedcalloc(lib25519_hash_sha512_BYTES);
  unsigned char *m = alignedcalloc(MAXTEST_BYTES);
  long long mlen;

  for (long long impl = -1;impl < lib25519_numimpl_hash_sha512();++impl) {
    void (*crypto_hash)(unsigned char *,const unsigned char *,long long);
    if (targeti && strcmp(targeti,lib25519_dispatch_hash_sha512_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_hash = lib25519_dispatch_hash_sha512(impl);
      printf("hash_sha512 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_hash_sha512_implementation(impl),lib25519_dispatch_hash_sha512_compiler(impl));
    } else {
      crypto_hash = lib25519_hash_sha512;
      printf("hash_sha512 selected implementation %s compiler %s\n",lib25519_hash_sha512_implementation(),lib25519_hash_sha512_compiler());
    }
    randombytes(h,lib25519_hash_sha512_BYTES);
    randombytes(m,MAXTEST_BYTES);
    mlen = 0;
    while (mlen <= MAXTEST_BYTES) {
      randombytes(m,mlen);
      for (long long i = 0;i <= TIMINGS;++i) {
        t[i] = cpucycles();
        crypto_hash(h,m,mlen);
      }
      t_print("hash_sha512",impl,mlen);
      mlen += 1+mlen/2;
    }
  }
}

static void measure_pow_inv25519(void)
{
  if (targeto && strcmp(targeto,"pow")) return;
  if (targetp && strcmp(targetp,"inv25519")) return;
  unsigned char *n = alignedcalloc(lib25519_pow_inv25519_BYTES);
  unsigned char *ne = alignedcalloc(lib25519_pow_inv25519_BYTES);

  for (long long impl = -1;impl < lib25519_numimpl_pow_inv25519();++impl) {
    void (*crypto_pow)(unsigned char *,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_pow_inv25519_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_pow = lib25519_dispatch_pow_inv25519(impl);
      printf("pow_inv25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_pow_inv25519_implementation(impl),lib25519_dispatch_pow_inv25519_compiler(impl));
    } else {
      crypto_pow = lib25519_pow_inv25519;
      printf("pow_inv25519 selected implementation %s compiler %s\n",lib25519_pow_inv25519_implementation(),lib25519_pow_inv25519_compiler());
    }
    randombytes(n,lib25519_pow_inv25519_BYTES);
    randombytes(ne,lib25519_pow_inv25519_BYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_pow(ne,n);
    }
    t_print("pow_inv25519",impl,lib25519_pow_inv25519_BYTES);
  }
}

static void measure_nP_montgomery25519(void)
{
  if (targeto && strcmp(targeto,"nP")) return;
  if (targetp && strcmp(targetp,"montgomery25519")) return;
  unsigned char *n = alignedcalloc(lib25519_nP_montgomery25519_SCALARBYTES);
  unsigned char *P = alignedcalloc(lib25519_nP_montgomery25519_POINTBYTES);
  unsigned char *nP = alignedcalloc(lib25519_nP_montgomery25519_POINTBYTES);

  for (long long impl = -1;impl < lib25519_numimpl_nP_montgomery25519();++impl) {
    void (*crypto_nP)(unsigned char *,const unsigned char *,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_nP_montgomery25519_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_nP = lib25519_dispatch_nP_montgomery25519(impl);
      printf("nP_montgomery25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_nP_montgomery25519_implementation(impl),lib25519_dispatch_nP_montgomery25519_compiler(impl));
    } else {
      crypto_nP = lib25519_nP_montgomery25519;
      printf("nP_montgomery25519 selected implementation %s compiler %s\n",lib25519_nP_montgomery25519_implementation(),lib25519_nP_montgomery25519_compiler());
    }
    randombytes(n,lib25519_nP_montgomery25519_SCALARBYTES);
    randombytes(P,lib25519_nP_montgomery25519_POINTBYTES);
    randombytes(nP,lib25519_nP_montgomery25519_POINTBYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_nP(nP,n,P);
    }
    t_print("nP_montgomery25519",impl,lib25519_nP_montgomery25519_POINTBYTES);
  }
}

static void measure_nG_merged25519(void)
{
  if (targeto && strcmp(targeto,"nG")) return;
  if (targetp && strcmp(targetp,"merged25519")) return;
  unsigned char *n = alignedcalloc(lib25519_nP_SCALARBYTES);
  unsigned char *nG = alignedcalloc(lib25519_nP_POINTBYTES);

  for (long long impl = -1;impl < lib25519_numimpl_nG_merged25519();++impl) {
    void (*crypto_nG)(unsigned char *,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_nG_merged25519_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_nG = lib25519_dispatch_nG_merged25519(impl);
      printf("nG_merged25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_nG_merged25519_implementation(impl),lib25519_dispatch_nG_merged25519_compiler(impl));
    } else {
      crypto_nG = lib25519_nG_merged25519;
      printf("nG_merged25519 selected implementation %s compiler %s\n",lib25519_nG_merged25519_implementation(),lib25519_nG_merged25519_compiler());
    }
    randombytes(n,lib25519_nP_SCALARBYTES);
    randombytes(nG,lib25519_nP_POINTBYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_nG(nG,n);
    }
    t_print("nG_merged25519",impl,lib25519_nP_POINTBYTES);
  }
}

static void measure_nG_montgomery25519(void)
{
  if (targeto && strcmp(targeto,"nG")) return;
  if (targetp && strcmp(targetp,"montgomery25519")) return;
  unsigned char *n = alignedcalloc(lib25519_nP_SCALARBYTES);
  unsigned char *nG = alignedcalloc(lib25519_nP_POINTBYTES);

  for (long long impl = -1;impl < lib25519_numimpl_nG_montgomery25519();++impl) {
    void (*crypto_nG)(unsigned char *,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_nG_montgomery25519_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_nG = lib25519_dispatch_nG_montgomery25519(impl);
      printf("nG_montgomery25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_nG_montgomery25519_implementation(impl),lib25519_dispatch_nG_montgomery25519_compiler(impl));
    } else {
      crypto_nG = lib25519_nG_montgomery25519;
      printf("nG_montgomery25519 selected implementation %s compiler %s\n",lib25519_nG_montgomery25519_implementation(),lib25519_nG_montgomery25519_compiler());
    }
    randombytes(n,lib25519_nP_SCALARBYTES);
    randombytes(nG,lib25519_nP_POINTBYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_nG(nG,n);
    }
    t_print("nG_montgomery25519",impl,lib25519_nP_POINTBYTES);
  }
}

static void measure_mGnP_ed25519(void)
{
  if (targeto && strcmp(targeto,"mGnP")) return;
  if (targetp && strcmp(targetp,"ed25519")) return;
  unsigned char *mGnP = alignedcalloc(lib25519_mGnP_ed25519_OUTPUTBYTES);
  unsigned char *m = alignedcalloc(lib25519_mGnP_ed25519_MBYTES);
  unsigned char *n = alignedcalloc(lib25519_mGnP_ed25519_NBYTES);
  unsigned char *P = alignedcalloc(lib25519_mGnP_ed25519_PBYTES);

  for (long long impl = -1;impl < lib25519_numimpl_mGnP_ed25519();++impl) {
    void (*crypto_mGnP)(unsigned char *,const unsigned char *,const unsigned char *,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_mGnP_ed25519_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_mGnP = lib25519_dispatch_mGnP_ed25519(impl);
      printf("mGnP_ed25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_mGnP_ed25519_implementation(impl),lib25519_dispatch_mGnP_ed25519_compiler(impl));
    } else {
      crypto_mGnP = lib25519_mGnP_ed25519;
      printf("mGnP_ed25519 selected implementation %s compiler %s\n",lib25519_mGnP_ed25519_implementation(),lib25519_mGnP_ed25519_compiler());
    }
    randombytes(mGnP,lib25519_mGnP_ed25519_OUTPUTBYTES);
    randombytes(m,lib25519_mGnP_ed25519_MBYTES);
    randombytes(n,lib25519_mGnP_ed25519_NBYTES);
    randombytes(P,lib25519_mGnP_ed25519_PBYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_mGnP(mGnP,m,n,P);
    }
    t_print("mGnP_ed25519",impl,lib25519_mGnP_ed25519_OUTPUTBYTES);
  }
}

static void measure_dh_x25519(void)
{
  if (targeto && strcmp(targeto,"dh")) return;
  if (targetp && strcmp(targetp,"x25519")) return;
  unsigned char *pka = alignedcalloc(lib25519_dh_x25519_PUBLICKEYBYTES);
  unsigned char *ska = alignedcalloc(lib25519_dh_x25519_SECRETKEYBYTES);
  unsigned char *pkb = alignedcalloc(lib25519_dh_x25519_PUBLICKEYBYTES);
  unsigned char *skb = alignedcalloc(lib25519_dh_x25519_SECRETKEYBYTES);
  unsigned char *ka = alignedcalloc(lib25519_dh_x25519_BYTES);

  for (long long impl = -1;impl < lib25519_numimpl_dh_x25519();++impl) {
    void (*crypto_dh_keypair)(unsigned char *,unsigned char *);
    void (*crypto_dh)(unsigned char *,const unsigned char *,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_dh_x25519_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_dh_keypair = lib25519_dispatch_dh_x25519_keypair(impl);
      crypto_dh = lib25519_dispatch_dh_x25519(impl);
      printf("dh_x25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_dh_x25519_implementation(impl),lib25519_dispatch_dh_x25519_compiler(impl));
    } else {
      crypto_dh_keypair = lib25519_dh_x25519_keypair;
      crypto_dh = lib25519_dh_x25519;
      printf("dh_x25519 selected implementation %s compiler %s\n",lib25519_dh_x25519_implementation(),lib25519_dh_x25519_compiler());
    }
    randombytes(pka,lib25519_dh_x25519_PUBLICKEYBYTES);
    randombytes(ska,lib25519_dh_x25519_SECRETKEYBYTES);
    randombytes(pkb,lib25519_dh_x25519_PUBLICKEYBYTES);
    randombytes(skb,lib25519_dh_x25519_SECRETKEYBYTES);
    randombytes(ka,lib25519_dh_x25519_BYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_dh_keypair(pka,ska);
    }
    t_print("dh_x25519_keypair",impl,lib25519_dh_x25519_PUBLICKEYBYTES);
    crypto_dh_keypair(pkb,skb);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_dh(ka,pkb,ska);
    }
    t_print("dh_x25519",impl,lib25519_dh_x25519_BYTES);
  }
}

static void measure_sign_ed25519(void)
{
  if (targeto && strcmp(targeto,"sign")) return;
  if (targetp && strcmp(targetp,"ed25519")) return;
  unsigned char *pk = alignedcalloc(lib25519_sign_ed25519_PUBLICKEYBYTES);
  unsigned char *sk = alignedcalloc(lib25519_sign_ed25519_SECRETKEYBYTES);
  unsigned char *m = alignedcalloc(MAXTEST_BYTES+lib25519_sign_ed25519_BYTES);
  unsigned char *sm = alignedcalloc(MAXTEST_BYTES+lib25519_sign_ed25519_BYTES);
  unsigned char *m2 = alignedcalloc(MAXTEST_BYTES+lib25519_sign_ed25519_BYTES);
  long long mlen;
  long long smlen;
  long long m2len;

  for (long long impl = -1;impl < lib25519_numimpl_sign_ed25519();++impl) {
    void (*crypto_sign_keypair)(unsigned char *,unsigned char *);
    void (*crypto_sign)(unsigned char *,long long *,const unsigned char *,long long,const unsigned char *);
    int (*crypto_sign_open)(unsigned char *,long long *,const unsigned char *,long long,const unsigned char *);
    if (targeti && strcmp(targeti,lib25519_dispatch_sign_ed25519_implementation(impl))) continue;
    if (impl >= 0) {
      crypto_sign_keypair = lib25519_dispatch_sign_ed25519_keypair(impl);
      crypto_sign = lib25519_dispatch_sign_ed25519(impl);
      crypto_sign_open = lib25519_dispatch_sign_ed25519_open(impl);
      printf("sign_ed25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_sign_ed25519_implementation(impl),lib25519_dispatch_sign_ed25519_compiler(impl));
    } else {
      crypto_sign_keypair = lib25519_sign_ed25519_keypair;
      crypto_sign = lib25519_sign_ed25519;
      crypto_sign_open = lib25519_sign_ed25519_open;
      printf("sign_ed25519 selected implementation %s compiler %s\n",lib25519_sign_ed25519_implementation(),lib25519_sign_ed25519_compiler());
    }
    randombytes(pk,lib25519_sign_ed25519_PUBLICKEYBYTES);
    randombytes(sk,lib25519_sign_ed25519_SECRETKEYBYTES);
    randombytes(m,MAXTEST_BYTES+lib25519_sign_ed25519_BYTES);
    randombytes(sm,MAXTEST_BYTES+lib25519_sign_ed25519_BYTES);
    randombytes(m2,MAXTEST_BYTES+lib25519_sign_ed25519_BYTES);
    for (long long i = 0;i <= TIMINGS;++i) {
      t[i] = cpucycles();
      crypto_sign_keypair(pk,sk);
    }
    t_print("sign_ed25519_keypair",impl,lib25519_sign_ed25519_PUBLICKEYBYTES);
    mlen = 0;
    while (mlen <= MAXTEST_BYTES) {
      randombytes(m,mlen);
      for (long long i = 0;i <= TIMINGS;++i) {
        t[i] = cpucycles();
        crypto_sign(sm,&smlen,m,mlen,sk);
      }
      t_print("sign_ed25519",impl,mlen);
      mlen += 1+mlen/4;
    }
    mlen = 0;
    while (mlen <= MAXTEST_BYTES) {
      randombytes(m,mlen);
      lib25519_sign(sm,&smlen,m,mlen,sk);
      for (long long i = 0;i <= TIMINGS;++i) {
        t[i] = cpucycles();
        crypto_sign_open(m2,&m2len,sm,smlen,pk);
      }
      t_print("sign_ed25519_open",impl,mlen);
      /* this is, in principle, not a test program */
      /* but some checks here help validate the data flow above */
      assert(m2len == mlen);
      assert(!memcmp(m,m2,mlen));
      mlen += 1+mlen/4;
    }
  }
}

#include "print_cpuid.inc"

int main(int argc,char **argv)
{
  printf("lib25519 version %s\n",lib25519_version);
  printf("lib25519 arch %s\n",lib25519_arch);
  print_cpuid();

  if (*argv) ++argv;
  if (*argv) {
    targeto = *argv++;
    if (*argv) {
      targetp = *argv++;
      if (*argv) {
        targeti = *argv++;
      }
    }
  }

  measure_cpucycles();
  measure_randombytes();
  limits();
  measure_verify_32();
  measure_hashblocks_sha512();
  measure_hash_sha512();
  measure_pow_inv25519();
  measure_nP_montgomery25519();
  measure_nG_merged25519();
  measure_nG_montgomery25519();
  measure_mGnP_ed25519();
  measure_dh_x25519();
  measure_sign_ed25519();

  return 0;
}
