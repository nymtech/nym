/* WARNING: auto-generated (by autogen-test); do not edit */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <time.h>
#include <assert.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <fcntl.h>
#include <sys/resource.h>
#include "crypto_uint8.h"
#include "crypto_uint32.h"
#include "crypto_uint64.h"
#include "lib25519.h" /* -l25519 */
#include "randombytes.h"

static const char *targeto = 0;
static const char *targetp = 0;
static const char *targeti = 0;

static int ok = 1;

#define fail ((ok = 0),printf)

/* ----- kernelrandombytes */

static int kernelrandombytes_fd = -1;

static void kernelrandombytes_setup(void)
{
  kernelrandombytes_fd = open("/dev/urandom",O_RDONLY);
  if (kernelrandombytes_fd == -1) {
    fprintf(stderr,"lib25519-test: fatal: unable to open /dev/urandom: %s",strerror(errno));
    exit(111);
  }
}

static void kernelrandombytes(unsigned char *x,long long xlen)
{
  int i;

  while (xlen > 0) {
    if (xlen < 1048576) i = xlen; else i = 1048576;

    i = read(kernelrandombytes_fd,x,i);
    if (i < 1) {
      sleep(1);
      continue;
    }

    x += i;
    xlen -= i;
  }
}

/* ----- rng and hash, from supercop/try-anything.c */

typedef crypto_uint8 u8;
typedef crypto_uint32 u32;
typedef crypto_uint64 u64;

#define FOR(i,n) for (i = 0;i < n;++i)

static u32 L32(u32 x,int c) { return (x << c) | ((x&0xffffffff) >> (32 - c)); }

static u32 ld32(const u8 *x)
{
  u32 u = x[3];
  u = (u<<8)|x[2];
  u = (u<<8)|x[1];
  return (u<<8)|x[0];
}

static void st32(u8 *x,u32 u)
{
  int i;
  FOR(i,4) { x[i] = u; u >>= 8; }
}

static const u8 sigma[17] = "expand 32-byte k";

static void core_salsa(u8 *out,const u8 *in,const u8 *k)
{
  u32 w[16],x[16],y[16],t[4];
  int i,j,m;

  FOR(i,4) {
    x[5*i] = ld32(sigma+4*i);
    x[1+i] = ld32(k+4*i);
    x[6+i] = ld32(in+4*i);
    x[11+i] = ld32(k+16+4*i);
  }

  FOR(i,16) y[i] = x[i];

  FOR(i,20) {
    FOR(j,4) {
      FOR(m,4) t[m] = x[(5*j+4*m)%16];
      t[1] ^= L32(t[0]+t[3], 7);
      t[2] ^= L32(t[1]+t[0], 9);
      t[3] ^= L32(t[2]+t[1],13);
      t[0] ^= L32(t[3]+t[2],18);
      FOR(m,4) w[4*j+(j+m)%4] = t[m];
    }
    FOR(m,16) x[m] = w[m];
  }

  FOR(i,16) st32(out + 4 * i,x[i] + y[i]);
}

static void salsa20(u8 *c,u64 b,const u8 *n,const u8 *k)
{
  u8 z[16],x[64];
  u32 u,i;
  if (!b) return;
  FOR(i,16) z[i] = 0;
  FOR(i,8) z[i] = n[i];
  while (b >= 64) {
    core_salsa(x,z,k);
    FOR(i,64) c[i] = x[i];
    u = 1;
    for (i = 8;i < 16;++i) {
      u += (u32) z[i];
      z[i] = u;
      u >>= 8;
    }
    b -= 64;
    c += 64;
  }
  if (b) {
    core_salsa(x,z,k);
    FOR(i,b) c[i] = x[i];
  }
}

static void increment(u8 *n)
{
  if (!++n[0])
    if (!++n[1])
      if (!++n[2])
        if (!++n[3])
          if (!++n[4])
            if (!++n[5])
              if (!++n[6])
                if (!++n[7])
                  ;
}

static unsigned char testvector_n[8];

static void testvector_clear(void)
{
  memset(testvector_n,0,sizeof testvector_n);
}

static void testvector(unsigned char *x,unsigned long long xlen)
{
  const static unsigned char testvector_k[33] = "generate inputs for test vectors";
  salsa20(x,xlen,testvector_n,testvector_k);
  increment(testvector_n);
}

static unsigned long long myrandom(void)
{
  unsigned char x[8];
  unsigned long long result;
  testvector(x,8);
  result = x[7];
  result = (result<<8)|x[6];
  result = (result<<8)|x[5];
  result = (result<<8)|x[4];
  result = (result<<8)|x[3];
  result = (result<<8)|x[2];
  result = (result<<8)|x[1];
  result = (result<<8)|x[0];
  return result;
}

static unsigned char canary_n[8];

static void canary(unsigned char *x,unsigned long long xlen)
{
  const static unsigned char canary_k[33] = "generate pad to catch overwrites";
  salsa20(x,xlen,canary_n,canary_k);
  increment(canary_n);
}

static void double_canary(unsigned char *x2,unsigned char *x,unsigned long long xlen)
{
  canary(x - 16,16);
  canary(x + xlen,16);
  memcpy(x2 - 16,x - 16,16);
  memcpy(x2 + xlen,x + xlen,16);
}

static void input_prepare(unsigned char *x2,unsigned char *x,unsigned long long xlen)
{
  testvector(x,xlen);
  canary(x - 16,16);
  canary(x + xlen,16);
  memcpy(x2 - 16,x - 16,xlen + 32);
}

static void input_compare(const unsigned char *x2,const unsigned char *x,unsigned long long xlen,const char *fun)
{
  if (memcmp(x2 - 16,x - 16,xlen + 32)) {
    fail("failure: %s overwrites input\n",fun);
  }
}

static void output_prepare(unsigned char *x2,unsigned char *x,unsigned long long xlen)
{
  canary(x - 16,xlen + 32);
  memcpy(x2 - 16,x - 16,xlen + 32);
}

static void output_compare(const unsigned char *x2,const unsigned char *x,unsigned long long xlen,const char *fun)
{
  if (memcmp(x2 - 16,x - 16,16)) {
    fail("failure: %s writes before output\n",fun);
  }
  if (memcmp(x2 + xlen,x + xlen,16)) {
    fail("failure: %s writes after output\n",fun);
  }
}

/* ----- knownrandombytes */

static const int knownrandombytes_is_only_for_testing_not_for_cryptographic_use = 1;
#define knownrandombytes randombytes

#define QUARTERROUND(a,b,c,d) \
  a += b; d = L32(d^a,16); \
  c += d; b = L32(b^c,12); \
  a += b; d = L32(d^a, 8); \
  c += d; b = L32(b^c, 7);

static void core_chacha(u8 *out,const u8 *in,const u8 *k)
{
  u32 x[16],y[16];
  int i,j;
  FOR(i,4) {
    x[i] = ld32(sigma+4*i);
    x[12+i] = ld32(in+4*i);
  }
  FOR(i,8) x[4+i] = ld32(k+4*i);
  FOR(i,16) y[i] = x[i];
  FOR(i,10) {
    FOR(j,4) { QUARTERROUND(x[j],x[j+4],x[j+8],x[j+12]) }
    FOR(j,4) { QUARTERROUND(x[j],x[((j+1)&3)+4],x[((j+2)&3)+8],x[((j+3)&3)+12]) }
  }
  FOR(i,16) st32(out+4*i,x[i]+y[i]);
}

static void chacha20(u8 *c,u64 b,const u8 *n,const u8 *k)
{
  u8 z[16],x[64];
  u32 u,i;
  if (!b) return;
  FOR(i,16) z[i] = 0;
  FOR(i,8) z[i+8] = n[i];
  while (b >= 64) {
    core_chacha(x,z,k);
    FOR(i,64) c[i] = x[i];
    u = 1;
    FOR(i,8) {
      u += (u32) z[i];
      z[i] = u;
      u >>= 8;
    }
    b -= 64;
    c += 64;
  }
  if (b) {
    core_chacha(x,z,k);
    FOR(i,b) c[i] = x[i];
  }
}

#define crypto_rng_OUTPUTBYTES 736

static int crypto_rng(
        unsigned char *r, /* random output */
        unsigned char *n, /* new key */
  const unsigned char *g  /* old key */
)
{
  static const unsigned char nonce[8] = {0};
  unsigned char x[32+crypto_rng_OUTPUTBYTES];
  chacha20(x,sizeof x,nonce,g);
  memcpy(n,x,32);
  memcpy(r,x+32,crypto_rng_OUTPUTBYTES);
  return 0;
}

static unsigned char knownrandombytes_g[32];
static unsigned char knownrandombytes_r[crypto_rng_OUTPUTBYTES];
static unsigned long long knownrandombytes_pos = crypto_rng_OUTPUTBYTES;

static void knownrandombytes_clear(void)
{
  memset(knownrandombytes_g,0,sizeof knownrandombytes_g);
  memset(knownrandombytes_r,0,sizeof knownrandombytes_r);
  knownrandombytes_pos = crypto_rng_OUTPUTBYTES;
}

void knownrandombytes(unsigned char *x,long long xlen)
{
  assert(knownrandombytes_is_only_for_testing_not_for_cryptographic_use);

  while (xlen > 0) {
    if (knownrandombytes_pos == crypto_rng_OUTPUTBYTES) {
      crypto_rng(knownrandombytes_r,knownrandombytes_g,knownrandombytes_g);
      knownrandombytes_pos = 0;
    }
    *x++ = knownrandombytes_r[knownrandombytes_pos]; xlen -= 1;
    knownrandombytes_r[knownrandombytes_pos++] = 0;
  }
}

/* ----- checksums */

static unsigned char checksum_state[64];
static char checksum_hex[65];

static void checksum_expected(const char *expected)
{
  long long i;
  for (i = 0;i < 32;++i) {
    checksum_hex[2 * i] = "0123456789abcdef"[15 & (checksum_state[i] >> 4)];
    checksum_hex[2 * i + 1] = "0123456789abcdef"[15 & checksum_state[i]];
  }
  checksum_hex[2 * i] = 0;

  if (strcmp(checksum_hex,expected))
    fail("failure: checksum mismatch: %s expected %s\n",checksum_hex,expected);
}

static void checksum_clear(void)
{
  memset(checksum_state,0,sizeof checksum_state);
  knownrandombytes_clear();
  testvector_clear();
  /* not necessary to clear canary */
}

static void checksum(const unsigned char *x,unsigned long long xlen)
{
  u8 block[16];
  int i;
  while (xlen >= 16) {
    core_salsa(checksum_state,x,checksum_state);
    x += 16;
    xlen -= 16;
  }
  FOR(i,16) block[i] = 0;
  FOR(i,xlen) block[i] = x[i];
  block[xlen] = 1;
  checksum_state[0] ^= 1;
  core_salsa(checksum_state,block,checksum_state);
}

#include "limits.inc"

static unsigned char *alignedcalloc(unsigned long long len)
{
  unsigned char *x = (unsigned char *) calloc(1,len + 256);
  long long i;
  if (!x) abort();
  /* will never deallocate so shifting is ok */
  for (i = 0;i < len + 256;++i) x[i] = random();
  x += 64;
  x += 63 & (-(unsigned long) x);
  for (i = 0;i < len;++i) x[i] = 0;
  return x;
}

/* ----- catching SIGILL, SIGBUS, SIGSEGV, etc. */

static void forked(void (*test)(long long),long long impl)
{
  fflush(stdout);
  pid_t child = fork();
  int childstatus = -1;
  if (child == -1) {
    fprintf(stderr,"fatal: fork failed: %s",strerror(errno));
    exit(111);
  }
  if (child == 0) {
    ok = 1;
    limits();
    test(impl);
    if (!ok) exit(100);
    exit(0);
  }
  if (waitpid(child,&childstatus,0) != child) {
    fprintf(stderr,"fatal: wait failed: %s",strerror(errno));
    exit(111);
  }
  if (childstatus)
    fail("failure: process failed, status %d\n",childstatus);
  fflush(stdout);
}

/* ----- verify, derived from supercop/crypto_verify/try.c */

static int (*crypto_verify)(const unsigned char *,const unsigned char *);
#define crypto_verify_BYTES lib25519_verify_BYTES

static unsigned char *test_verify_x;
static unsigned char *test_verify_y;

static void test_verify_check(void)
{
  unsigned char *x = test_verify_x;
  unsigned char *y = test_verify_y;
  int r = crypto_verify(x,y);

  if (r == 0) {
    if (memcmp(x,y,crypto_verify_BYTES))
      fail("failure: different strings pass verify\n");
  } else if (r == -1) {
    if (!memcmp(x,y,crypto_verify_BYTES))
      fail("failure: equal strings fail verify\n");
  } else {
    fail("failure: weird return value\n");
  }
}

void test_verify_impl(long long impl)
{
  unsigned char *x = test_verify_x;
  unsigned char *y = test_verify_y;

  if (targeti && strcmp(targeti,lib25519_dispatch_verify_implementation(impl))) return;
  if (impl >= 0) {
    crypto_verify = lib25519_dispatch_verify(impl);
    printf("verify %lld implementation %s compiler %s\n",impl,lib25519_dispatch_verify_implementation(impl),lib25519_dispatch_verify_compiler(impl));
  } else {
    crypto_verify = lib25519_verify;
    printf("verify selected implementation %s compiler %s\n",lib25519_verify_implementation(),lib25519_verify_compiler());
  }

  kernelrandombytes(x,crypto_verify_BYTES);
  kernelrandombytes(y,crypto_verify_BYTES);
  test_verify_check();
  memcpy(y,x,crypto_verify_BYTES);
  test_verify_check();
  y[myrandom() % crypto_verify_BYTES] = myrandom();
  test_verify_check();
  y[myrandom() % crypto_verify_BYTES] = myrandom();
  test_verify_check();
  y[myrandom() % crypto_verify_BYTES] = myrandom();
  test_verify_check();
}

static void test_verify(void)
{
  if (targeto && strcmp(targeto,"verify")) return;
  if (targetp && strcmp(targetp,"32")) return;

  test_verify_x = alignedcalloc(crypto_verify_BYTES);
  test_verify_y = alignedcalloc(crypto_verify_BYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("verify offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_verify();++impl)
      forked(test_verify_impl,impl);
    ++test_verify_x;
    ++test_verify_y;
  }
}

/* ----- hashblocks, derived from supercop/crypto_hashblocks/try.c */
static const char *hashblocks_sha512_checksums[] = {
  "f0bc623a9033f9f648336540e11e85be21aeb60905c7d8808d10ea20b39d58d1",
  "f1a2c46c9ce7fa4cd22f180907d77b6f7189badef4b9a1b5284d6fb9db859b76",
} ;

static int (*crypto_hashblocks)(unsigned char *,const unsigned char *,long long);
#define crypto_hashblocks_STATEBYTES lib25519_hashblocks_sha512_STATEBYTES
#define crypto_hashblocks_BLOCKBYTES lib25519_hashblocks_sha512_BLOCKBYTES

static unsigned char *test_hashblocks_sha512_h;
static unsigned char *test_hashblocks_sha512_m;
static unsigned char *test_hashblocks_sha512_h2;
static unsigned char *test_hashblocks_sha512_m2;

static void test_hashblocks_sha512_impl(long long impl)
{
  unsigned char *h = test_hashblocks_sha512_h;
  unsigned char *m = test_hashblocks_sha512_m;
  unsigned char *h2 = test_hashblocks_sha512_h2;
  unsigned char *m2 = test_hashblocks_sha512_m2;
  long long hlen = crypto_hashblocks_STATEBYTES;
  long long mlen;

  if (targeti && strcmp(targeti,lib25519_dispatch_hashblocks_sha512_implementation(impl))) return;
  if (impl >= 0) {
    crypto_hashblocks = lib25519_dispatch_hashblocks_sha512(impl);
    printf("hashblocks_sha512 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_hashblocks_sha512_implementation(impl),lib25519_dispatch_hashblocks_sha512_compiler(impl));
  } else {
    crypto_hashblocks = lib25519_hashblocks_sha512;
    printf("hashblocks_sha512 selected implementation %s compiler %s\n",lib25519_hashblocks_sha512_implementation(),lib25519_hashblocks_sha512_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 32768 : 4096;
    long long maxtest = checksumbig ? 4096 : 128;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {
      int result;
      mlen = myrandom() % (maxtest + 1);

      input_prepare(m2,m,mlen);
      input_prepare(h2,h,hlen);
      result = crypto_hashblocks(h,m,mlen);
      if (result != mlen % crypto_hashblocks_BLOCKBYTES) fail("failure: crypto_hashblocks returns unexpected value\n");
      checksum(h,hlen);
      output_compare(h2,h,hlen,"crypto_hashblocks");
      input_compare(m2,m,mlen,"crypto_hashblocks");

      double_canary(h2,h,hlen);
      double_canary(m2,m,mlen);
      result = crypto_hashblocks(h2,m2,mlen);
      if (result != mlen % crypto_hashblocks_BLOCKBYTES) fail("failure: crypto_hashblocks returns unexpected value\n");
      if (memcmp(h2,h,hlen) != 0) fail("failure: crypto_hashblocks is nondeterministic\n");
    }
    checksum_expected(hashblocks_sha512_checksums[checksumbig]);
  }
}

static void test_hashblocks_sha512(void)
{
  if (targeto && strcmp(targeto,"hashblocks")) return;
  if (targetp && strcmp(targetp,"sha512")) return;
  test_hashblocks_sha512_h = alignedcalloc(crypto_hashblocks_STATEBYTES);
  test_hashblocks_sha512_m = alignedcalloc(4096);
  test_hashblocks_sha512_h2 = alignedcalloc(crypto_hashblocks_STATEBYTES);
  test_hashblocks_sha512_m2 = alignedcalloc(4096);

  for (long long offset = 0;offset < 2;++offset) {
    printf("hashblocks_sha512 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_hashblocks_sha512();++impl)
      forked(test_hashblocks_sha512_impl,impl);
    ++test_hashblocks_sha512_h;
    ++test_hashblocks_sha512_m;
    ++test_hashblocks_sha512_h2;
    ++test_hashblocks_sha512_m2;
  }
}
#undef crypto_hashblocks_STATEBYTES
#undef crypto_hashblocks_BLOCKBYTES


/* ----- hash, derived from supercop/crypto_hash/try.c */
static const char *hash_sha512_checksums[] = {
  "8220572f58bd4730be165c9739d8d4b0fd2e0229dbe01e25b4aed23f00f23b70",
  "c1e322b7cbfc941260c5508967ba05bce22eeee94d425e708b7c3301ea1d5e2e",
} ;

static void (*crypto_hash)(unsigned char *,const unsigned char *,long long);
#define crypto_hash_BYTES lib25519_hash_sha512_BYTES

static unsigned char *test_hash_sha512_h;
static unsigned char *test_hash_sha512_m;
static unsigned char *test_hash_sha512_h2;
static unsigned char *test_hash_sha512_m2;

static void test_hash_sha512_impl(long long impl)
{
  unsigned char *h = test_hash_sha512_h;
  unsigned char *m = test_hash_sha512_m;
  unsigned char *h2 = test_hash_sha512_h2;
  unsigned char *m2 = test_hash_sha512_m2;
  long long hlen = crypto_hash_BYTES;
  long long mlen;

  if (targeti && strcmp(targeti,lib25519_dispatch_hash_sha512_implementation(impl))) return;
  if (impl >= 0) {
    crypto_hash = lib25519_dispatch_hash_sha512(impl);
    printf("hash_sha512 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_hash_sha512_implementation(impl),lib25519_dispatch_hash_sha512_compiler(impl));
  } else {
    crypto_hash = lib25519_hash_sha512;
    printf("hash_sha512 selected implementation %s compiler %s\n",lib25519_hash_sha512_implementation(),lib25519_hash_sha512_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 512 : 64;
    long long maxtest = checksumbig ? 4096 : 128;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {
      mlen = myrandom() % (maxtest + 1);

      output_prepare(h2,h,hlen);
      input_prepare(m2,m,mlen);
      crypto_hash(h,m,mlen);
      checksum(h,hlen);
      output_compare(h2,h,hlen,"crypto_hash");
      input_compare(m2,m,mlen,"crypto_hash");

      double_canary(h2,h,hlen);
      double_canary(m2,m,mlen);
      crypto_hash(h2,m2,mlen);
      if (memcmp(h2,h,hlen) != 0) fail("failure: crypto_hash is nondeterministic\n");

      double_canary(h2,h,hlen);
      double_canary(m2,m,mlen);
      crypto_hash(m2,m2,mlen);
      if (memcmp(m2,h,hlen) != 0) fail("failure: crypto_hash does not handle m=h overlap\n");
      memcpy(m2,m,mlen);
    }
    checksum_expected(hash_sha512_checksums[checksumbig]);
  }
}

static void test_hash_sha512(void)
{
  if (targeto && strcmp(targeto,"hash")) return;
  if (targetp && strcmp(targetp,"sha512")) return;
  test_hash_sha512_h = alignedcalloc(crypto_hash_BYTES);
  test_hash_sha512_m = alignedcalloc(4096);
  test_hash_sha512_h2 = alignedcalloc(crypto_hash_BYTES);
  test_hash_sha512_m2 = alignedcalloc(4096);

  for (long long offset = 0;offset < 2;++offset) {
    printf("hash_sha512 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_hash_sha512();++impl)
      forked(test_hash_sha512_impl,impl);
    ++test_hash_sha512_h;
    ++test_hash_sha512_m;
    ++test_hash_sha512_h2;
    ++test_hash_sha512_m2;
  }
}
#undef crypto_hash_BYTES


/* ----- pow, derived from supercop/crypto_pow/try.c */
static const char *pow_inv25519_checksums[] = {
  "ad2062946e82718da820226504991a85c5fe56bdbff959c1313f837ee13b37be",
  "59b3045a01e1fca2a86a0280aee8b985c5e040afdc0d3e85ed87eb97a46a4dd6",
} ;

static void (*crypto_pow)(unsigned char *,const unsigned char *);
#define crypto_pow_BYTES lib25519_pow_inv25519_BYTES

static unsigned char *test_pow_inv25519_q;
static unsigned char *test_pow_inv25519_p;
static unsigned char *test_pow_inv25519_q2;
static unsigned char *test_pow_inv25519_p2;

#define precomputed_pow_inv25519_NUM 296

static const unsigned char precomputed_pow_inv25519_q[precomputed_pow_inv25519_NUM][crypto_pow_BYTES] = {
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,63},
  {73,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,95},
  {150,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,25},
  {155,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,106},
  {141,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,47},
  {18,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,71},
  {203,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,12},
  {38,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,58},
  {68,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,117},
  {11,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59},
  {61,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,82},
  {208,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,93},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,87},
  {77,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90},
  {137,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,35},
  {20,202,107,40,175,161,188,134,242,26,202,107,40,175,161,188,134,242,26,202,107,40,175,161,188,134,242,26,202,107,40,47},
  {92,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,70},
  {47,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12},
  {19,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,29},
  {195,155,222,244,166,55,189,233,77,111,122,211,155,222,244,166,55,189,233,77,111,122,211,155,222,244,166,55,189,233,77,111},
  {162,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,58},
  {30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,5},
  {124,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,93},
  {6,237,37,180,151,208,94,66,123,9,237,37,180,151,208,94,66,123,9,237,37,180,151,208,94,66,123,9,237,37,180,23},
  {21,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,105},
  {189,114,79,35,44,247,52,194,114,79,35,44,247,52,194,114,79,35,44,247,52,194,114,79,35,44,247,52,194,114,79,35},
  {232,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,46},
  {104,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,78},
  {95,156,149,188,163,80,140,36,177,208,177,85,156,131,239,91,4,68,92,196,88,28,142,134,216,34,78,221,208,159,17,87},
  {38,221,244,121,160,140,135,138,91,246,171,251,42,229,30,53,137,92,183,112,97,134,244,149,52,224,229,52,101,97,145,127},
  {31,73,133,189,148,129,160,37,174,162,214,165,201,252,137,0,218,176,230,165,23,126,230,255,196,214,236,207,2,77,81,37},
  {19,82,242,135,133,161,110,239,185,137,126,205,221,147,170,22,90,141,0,210,119,232,26,163,149,111,118,253,222,3,226,73},
  {166,52,194,117,195,131,147,130,49,176,243,246,89,219,228,148,224,102,213,222,219,41,109,11,190,118,174,188,216,106,209,7},
  {64,83,70,138,197,143,41,181,1,61,160,174,192,213,164,235,142,132,203,134,232,145,182,128,27,160,170,67,19,240,15,76},
  {61,237,173,151,232,124,229,83,142,240,16,116,25,67,58,230,223,197,172,201,53,68,215,224,68,88,216,73,128,27,86,102},
  {79,136,214,213,186,148,59,147,202,128,21,64,171,141,222,175,233,198,247,208,187,227,246,47,84,98,142,3,140,220,155,104},
  {185,216,110,172,45,205,198,159,142,243,151,169,151,195,36,173,182,208,242,83,225,213,58,26,140,107,163,60,166,34,213,122},
  {52,192,155,149,157,69,10,35,45,123,136,36,198,171,92,77,61,235,133,155,107,234,184,43,148,43,78,72,236,127,77,1},
  {8,234,10,87,166,164,61,70,11,230,67,7,129,10,190,96,223,225,34,242,217,231,116,111,105,145,107,181,110,7,188,75},
  {252,236,57,250,166,229,172,78,226,121,142,145,209,165,31,76,181,206,99,67,25,29,185,105,201,93,213,246,132,121,178,46},
  {153,117,135,100,29,207,138,247,147,39,254,150,219,28,79,248,19,76,43,171,33,194,44,19,22,157,198,93,162,18,218,106},
  {57,223,157,125,238,224,147,49,254,234,178,208,159,240,225,64,132,52,197,214,65,81,250,170,171,134,212,82,97,17,143,57},
  {123,212,179,176,52,203,182,207,40,108,123,109,127,207,180,121,253,77,50,4,27,72,51,207,112,244,196,13,208,89,48,125},
  {86,220,213,222,72,124,82,5,213,248,34,204,214,253,164,251,177,11,65,6,226,153,121,182,210,190,64,10,88,14,13,43},
  {222,250,103,37,175,182,130,65,162,45,72,167,27,141,5,219,173,216,146,111,157,137,38,20,53,154,250,237,253,72,140,28},
  {183,98,239,209,78,92,190,97,188,253,134,65,208,227,201,65,240,85,75,192,135,66,252,186,18,230,191,68,201,128,7,82},
  {137,66,22,136,66,175,64,253,202,146,254,187,66,23,123,156,58,120,188,62,152,88,156,206,64,111,227,2,43,57,111,69},
  {163,122,63,119,3,151,5,212,188,30,80,65,104,9,118,149,121,77,179,29,167,6,121,68,20,203,120,37,24,177,88,49},
  {56,226,92,118,151,194,129,204,37,225,15,244,57,216,81,219,121,73,114,239,129,93,102,147,174,164,240,101,116,121,20,8},
  {223,183,166,239,48,31,130,116,116,213,206,52,173,119,219,186,152,3,115,75,165,101,149,137,53,78,253,172,172,134,204,22},
  {101,171,79,128,35,224,66,21,99,25,159,196,245,236,13,244,223,24,155,47,0,8,164,106,111,169,154,42,127,39,154,32},
  {60,71,103,7,132,234,55,209,32,107,75,14,162,135,37,217,36,215,112,162,8,0,165,37,116,10,2,107,239,105,15,44},
  {120,48,72,22,125,57,58,161,110,110,133,193,209,44,210,149,126,151,247,43,137,146,69,87,217,126,123,71,34,181,180,41},
  {26,234,188,172,72,26,124,128,66,220,35,67,32,250,233,98,180,240,142,120,38,245,98,68,87,247,130,59,42,107,59,102},
  {87,227,76,166,177,115,247,116,70,93,201,181,94,78,216,36,244,172,46,34,177,17,8,6,214,230,219,224,179,76,121,123},
  {247,116,252,18,43,132,167,183,1,40,32,155,43,255,203,237,11,9,60,95,92,99,181,125,155,84,13,23,190,90,109,120},
  {25,81,186,29,90,22,34,66,129,41,152,231,153,155,246,70,108,195,67,53,45,126,6,236,70,61,233,207,56,126,191,67},
  {128,12,149,140,116,186,133,136,238,4,7,76,128,24,44,7,32,8,97,255,209,208,246,68,147,223,143,19,17,152,219,26},
  {44,252,229,127,155,27,91,26,34,187,23,211,87,55,95,37,86,36,46,145,164,68,237,53,14,109,181,57,104,233,232,46},
  {246,169,6,227,244,135,36,153,153,180,43,150,213,82,228,200,25,224,132,223,114,42,238,88,133,171,232,76,139,176,32,37},
  {233,161,230,75,99,10,248,239,134,179,75,112,5,184,9,150,164,194,77,105,9,62,177,27,220,255,121,153,166,197,196,14},
  {10,175,30,186,99,204,243,120,203,70,66,90,131,167,0,32,126,65,92,129,132,11,117,118,170,34,208,91,216,226,223,66},
  {113,101,139,242,230,170,63,23,240,172,125,206,13,249,18,91,234,177,30,206,94,173,181,169,117,42,108,207,40,26,209,61},
  {171,26,203,222,6,85,103,210,224,27,211,48,141,214,146,10,136,83,2,40,31,239,213,59,57,124,216,115,62,224,175,89},
  {204,107,141,96,205,160,154,128,57,30,225,94,158,123,47,156,116,66,2,71,134,159,89,50,95,21,82,240,2,251,112,40},
  {226,145,63,94,151,88,160,212,211,138,37,204,192,45,231,173,191,161,182,15,154,32,44,216,245,254,159,75,249,101,252,60},
  {24,0,179,212,28,171,145,205,100,98,74,136,173,166,5,71,245,245,134,228,226,81,250,177,85,93,34,216,87,56,39,8},
  {124,187,175,19,110,205,24,229,223,52,182,185,61,183,101,187,33,36,136,71,247,139,158,124,21,82,165,203,138,93,137,87},
  {198,198,211,121,255,219,233,129,21,15,70,214,117,210,246,255,141,73,177,65,9,198,234,94,196,241,32,208,245,146,188,115},
  {58,85,123,151,193,188,6,23,14,144,215,82,124,64,185,14,61,224,129,70,177,145,124,171,48,115,158,228,40,88,226,119},
  {224,235,122,124,59,65,184,174,22,86,227,250,241,159,196,106,218,9,141,235,156,50,177,253,134,98,5,22,95,73,184,0},
  {240,44,29,72,44,33,188,97,152,219,129,79,124,138,167,64,144,200,229,60,6,3,242,73,196,157,70,161,25,6,89,17},
  {36,66,230,237,167,158,139,23,31,250,125,86,224,127,46,232,226,180,215,204,34,96,249,42,199,166,207,107,147,129,146,123},
  {98,218,163,46,248,246,95,42,196,92,16,252,105,79,233,115,249,87,128,31,207,51,90,86,246,189,111,79,124,228,140,10},
  {122,211,53,191,141,31,35,162,90,37,77,135,241,82,64,222,160,108,43,134,121,118,172,103,15,160,201,241,51,217,64,28},
  {145,8,253,77,245,5,120,31,74,60,119,213,196,168,200,250,125,18,48,99,3,110,27,113,160,14,220,76,165,239,65,32},
  {37,94,195,214,119,156,42,247,187,218,195,228,212,126,144,11,226,177,229,62,100,157,41,250,74,199,49,99,76,76,56,21},
  {203,165,234,77,253,45,200,2,126,15,141,112,82,28,182,95,73,13,130,89,202,6,194,120,69,171,129,243,86,140,179,104},
  {250,175,158,192,173,162,118,136,152,75,129,43,252,232,27,56,147,20,108,184,122,112,9,22,249,193,34,179,116,82,243,13},
  {139,217,27,238,113,28,242,233,76,16,196,238,215,96,153,61,0,64,134,198,20,205,154,208,84,164,135,54,153,95,4,126},
  {50,64,140,178,127,243,229,133,181,245,231,178,255,28,163,116,143,244,167,202,224,102,167,125,23,128,31,132,114,244,143,96},
  {68,2,137,26,149,197,182,41,179,89,168,46,148,42,174,46,105,67,5,236,229,53,91,235,166,66,164,68,226,71,196,20},
  {218,173,13,120,122,94,145,16,70,118,129,50,34,108,85,233,165,114,255,45,136,23,229,92,106,144,137,2,33,252,29,54},
  {230,33,152,96,68,5,80,48,41,141,126,170,102,237,107,144,102,211,248,193,103,55,52,34,219,94,12,203,154,12,80,103},
  {19,44,152,227,217,20,167,250,84,55,67,224,45,145,57,180,192,170,237,160,232,123,136,47,197,82,40,170,162,17,184,49},
  {148,135,170,165,51,197,56,93,128,90,55,248,90,88,150,59,254,77,206,190,194,65,214,56,198,175,24,104,8,181,243,44},
  {226,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,68},
  {169,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,10},
  {199,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69},
  {34,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,115},
  {219,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,79},
  {96,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,91},
  {82,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,21},
  {87,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,31},
  {164,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,42},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,63},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,63},
  {73,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,95},
  {150,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,25},
  {155,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,106},
  {141,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,47},
  {18,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,71},
  {203,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,12},
  {38,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,58},
  {68,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,117},
  {11,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59},
  {61,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,82},
  {208,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,93},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,87},
  {77,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90},
  {137,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,35},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,63},
  {73,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,95},
  {150,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,25},
  {155,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,106},
  {141,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,47},
  {18,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,71},
  {203,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,12},
  {38,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,58},
  {68,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,117},
  {11,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59},
  {61,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,82},
  {208,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,93},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,87},
  {77,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90},
  {137,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,35},
  {20,202,107,40,175,161,188,134,242,26,202,107,40,175,161,188,134,242,26,202,107,40,175,161,188,134,242,26,202,107,40,47},
  {92,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,70},
  {47,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12,195,48,12},
  {19,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,29},
  {195,155,222,244,166,55,189,233,77,111,122,211,155,222,244,166,55,189,233,77,111,122,211,155,222,244,166,55,189,233,77,111},
  {162,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,58},
  {30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,133,235,81,184,30,5},
  {124,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,157,216,137,93},
  {6,237,37,180,151,208,94,66,123,9,237,37,180,151,208,94,66,123,9,237,37,180,151,208,94,66,123,9,237,37,180,23},
  {21,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,105},
  {189,114,79,35,44,247,52,194,114,79,35,44,247,52,194,114,79,35,44,247,52,194,114,79,35,44,247,52,194,114,79,35},
  {232,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,238,46},
  {104,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,206,57,231,156,115,78},
  {255,18,128,72,228,210,22,236,211,24,133,166,9,89,225,162,36,191,150,60,252,28,47,47,158,5,5,216,186,24,207,103},
  {119,103,199,145,13,212,68,180,32,154,11,70,33,64,99,183,127,26,61,109,5,129,168,139,16,89,194,52,24,154,5,41},
  {1,148,159,247,183,146,107,28,108,130,211,75,103,155,178,82,249,162,224,151,203,209,46,231,108,64,61,45,150,202,45,0},
  {99,139,94,252,32,74,144,89,201,238,12,147,88,178,109,115,251,249,27,173,176,90,202,53,157,93,69,69,158,207,96,85},
  {249,65,66,22,31,224,118,102,169,219,18,109,120,237,101,211,149,123,65,235,107,9,99,16,84,18,117,37,154,188,46,103},
  {184,10,155,17,51,214,119,30,59,251,85,113,123,29,15,114,107,99,233,184,99,141,154,85,48,88,114,7,174,248,1,43},
  {183,19,131,62,107,60,139,25,159,81,252,127,241,150,167,10,86,224,76,177,199,197,197,80,104,31,181,26,122,42,200,107},
  {239,161,210,117,214,15,240,116,5,0,54,223,151,211,12,138,144,42,18,214,168,102,214,85,16,247,188,192,117,63,57,1},
  {61,54,212,216,74,218,83,219,62,35,165,95,17,3,73,161,75,101,254,69,125,245,244,141,119,123,245,253,111,14,25,93},
  {220,221,46,159,19,71,130,150,192,107,243,94,161,132,21,7,54,106,156,135,175,130,59,60,200,36,73,123,86,75,50,15},
  {99,164,73,245,216,117,26,54,100,89,185,145,92,75,162,224,247,89,82,97,42,146,188,162,234,188,53,50,205,27,164,125},
  {200,172,196,243,100,101,206,90,55,146,193,161,10,181,97,24,25,169,142,207,183,245,50,177,35,33,223,137,121,214,8,11},
  {27,50,144,247,44,248,235,222,217,45,60,206,47,125,224,19,173,200,182,17,66,77,178,222,9,208,215,92,217,235,16,51},
  {186,241,50,238,253,46,48,201,94,82,83,204,249,196,0,182,83,208,171,167,81,86,62,151,251,33,215,165,135,188,241,83},
  {177,22,78,2,251,64,132,103,11,6,223,232,100,240,122,253,31,213,144,42,88,64,152,99,119,1,85,61,153,42,246,28},
  {87,151,81,173,173,39,155,66,15,101,16,184,251,55,76,209,228,154,157,212,35,136,110,6,9,206,124,153,240,250,51,21},
  {26,237,10,109,22,27,15,145,91,164,252,40,180,129,77,1,209,54,255,9,230,144,68,179,94,150,255,52,26,192,119,12},
  {10,112,77,248,215,20,81,248,239,85,249,63,227,118,43,8,62,101,27,56,86,116,15,149,157,61,205,251,137,165,102,37},
  {124,192,215,171,163,91,214,7,214,103,28,62,221,11,42,56,169,24,161,174,234,129,174,74,144,189,57,77,183,143,230,54},
  {1,253,23,133,180,163,138,250,3,6,56,24,193,174,56,245,134,226,111,175,140,177,93,247,195,15,184,46,27,183,168,84},
  {25,127,208,50,168,132,165,21,56,59,46,58,37,56,196,112,79,108,118,165,211,183,6,64,229,168,173,55,122,100,74,77},
  {225,186,121,98,173,21,21,215,110,75,191,20,71,113,25,23,202,27,131,16,127,51,113,82,20,236,251,108,6,13,170,112},
  {48,45,229,134,192,180,23,34,163,128,199,99,190,108,58,236,202,55,204,18,197,103,175,253,182,237,16,122,224,228,25,51},
  {218,53,247,81,139,46,27,171,129,162,207,255,164,59,252,164,108,253,4,12,247,226,70,241,122,188,72,211,252,122,178,13},
  {170,226,249,137,116,167,226,120,193,243,124,147,88,138,15,162,48,71,145,111,254,119,63,233,128,23,10,92,67,118,30,30},
  {16,203,185,193,38,83,190,92,116,115,246,106,35,158,198,3,90,188,79,129,38,19,63,45,251,135,198,66,86,155,225,121},
  {209,40,101,130,26,183,234,127,32,20,146,116,8,112,194,56,141,99,210,83,17,213,186,22,156,223,63,106,4,205,119,120},
  {44,211,20,43,212,157,190,57,72,130,253,234,26,138,128,90,95,4,179,17,35,81,110,212,39,183,62,124,81,174,211,111},
  {44,36,50,74,131,169,121,80,198,48,15,109,17,179,219,187,206,100,32,234,143,15,167,198,211,146,163,181,67,145,34,30},
  {65,202,102,109,103,80,115,84,86,255,69,144,130,156,246,146,171,184,205,92,6,56,1,209,45,189,61,184,217,216,238,107},
  {92,107,139,15,200,95,209,61,18,98,255,105,11,30,179,203,165,173,126,12,101,216,5,82,113,163,14,42,63,71,54,95},
  {92,183,137,31,254,74,14,236,77,43,136,33,36,188,185,136,35,197,175,200,235,28,59,241,185,27,235,124,94,75,147,7},
  {172,154,20,26,177,187,189,219,238,205,20,135,222,148,10,180,45,77,167,230,85,220,108,93,190,152,222,115,194,46,238,38},
  {194,21,95,5,118,195,213,133,77,184,255,154,248,106,211,69,190,97,64,75,113,2,168,123,145,239,139,148,112,167,185,35},
  {32,117,101,93,21,29,45,200,131,115,0,171,69,249,81,151,27,172,135,163,158,170,85,244,177,208,78,112,29,130,208,1},
  {107,175,215,13,154,66,2,195,79,10,245,244,111,49,54,97,158,174,22,157,61,39,123,146,193,47,231,23,119,172,49,68},
  {218,44,117,234,3,221,97,148,64,22,1,104,162,171,212,147,56,101,247,120,8,181,246,246,147,242,168,40,240,191,81,91},
  {215,89,254,152,127,20,89,20,92,33,137,154,119,181,225,180,43,64,75,37,242,39,86,167,15,137,5,170,48,126,146,76},
  {146,95,192,36,105,238,38,17,66,210,53,159,29,45,172,13,19,239,216,103,210,227,95,146,76,141,226,75,237,224,140,45},
  {229,167,235,58,38,253,227,229,144,252,65,147,155,192,1,200,234,201,60,53,72,205,125,195,152,107,147,58,133,244,226,57},
  {88,244,110,33,254,212,210,200,74,219,192,138,123,42,22,161,158,67,146,169,244,26,194,146,134,59,88,139,28,13,232,113},
  {64,221,81,158,3,36,181,87,193,60,136,163,35,100,143,129,179,228,250,156,164,142,21,4,77,93,75,0,244,129,177,40},
  {99,201,196,169,55,97,250,40,141,136,16,157,64,175,118,170,88,177,140,160,7,30,114,150,4,66,180,241,24,77,251,24},
  {174,170,250,224,154,6,139,55,176,83,195,77,81,163,12,64,169,193,158,42,136,115,151,254,220,204,20,51,143,148,100,67},
  {30,77,35,14,207,56,225,204,214,84,214,43,10,5,131,1,232,164,48,194,28,249,10,221,177,149,235,251,97,74,211,10},
  {107,245,0,118,252,117,202,19,202,116,222,202,138,82,165,96,61,18,186,171,168,23,115,16,230,51,114,68,15,167,254,10},
  {214,120,72,144,158,69,169,197,194,31,128,253,42,9,89,244,196,88,93,126,65,2,167,187,5,242,252,242,184,214,242,5},
  {47,76,234,34,222,81,77,135,195,76,208,112,156,219,20,178,30,155,71,83,60,90,163,191,143,71,145,161,191,195,87,15},
  {58,222,210,172,117,119,57,61,224,20,29,150,221,44,144,70,140,51,231,175,210,153,37,88,146,194,225,169,73,158,19,113},
  {250,236,86,132,69,38,205,167,12,237,206,138,141,97,254,200,84,201,57,37,204,224,28,138,118,116,237,160,16,107,229,5},
  {70,94,251,66,100,123,54,49,59,133,51,209,145,175,125,17,204,29,7,212,133,213,29,183,143,195,185,173,47,3,222,85},
  {131,210,117,143,82,41,161,167,161,158,111,95,35,69,168,11,197,154,116,202,90,82,255,64,166,92,246,17,178,212,91,67},
  {157,55,39,243,142,130,111,45,166,84,112,203,27,239,172,76,214,75,31,106,240,173,202,191,74,78,251,83,156,95,158,43},
  {87,207,2,124,244,96,162,120,146,89,164,148,95,183,218,84,166,224,100,150,225,179,52,188,200,94,137,234,35,31,9,3},
  {162,220,223,48,214,174,3,100,223,97,188,82,171,187,14,131,68,24,30,37,238,210,191,242,76,102,108,1,27,62,67,0},
  {30,123,115,55,197,27,49,167,26,61,86,4,186,21,50,55,147,200,8,105,28,242,64,218,157,1,113,131,215,111,207,111},
  {119,12,187,139,37,59,1,93,187,48,178,41,39,84,133,181,224,15,62,13,53,161,23,77,89,10,6,206,230,145,233,84},
  {244,146,147,38,236,43,58,202,100,24,224,35,134,96,36,251,137,254,123,44,1,78,253,67,213,155,22,127,86,117,55,113},
  {136,220,123,37,85,109,34,40,194,73,21,222,241,128,141,227,72,46,31,43,36,254,235,144,206,97,253,136,172,31,194,111},
  {131,33,61,243,99,20,154,34,215,107,210,27,150,102,170,27,7,217,146,76,109,111,189,253,67,210,93,36,75,209,72,86},
  {247,154,210,2,116,207,58,196,149,233,253,154,162,199,226,96,240,183,87,121,236,7,252,165,160,114,145,199,44,124,246,118},
  {153,123,171,215,228,176,169,26,86,118,67,64,128,172,246,201,4,70,71,252,163,227,127,42,24,49,155,199,77,129,4,45},
  {102,56,102,102,190,156,31,135,141,175,224,246,95,126,250,255,95,195,96,39,9,143,141,99,172,48,106,85,180,81,171,105},
  {13,25,54,57,100,228,51,49,195,8,83,158,55,126,92,14,44,9,29,168,137,83,213,19,194,203,51,242,110,168,217,123},
  {126,111,105,209,219,21,23,61,189,188,198,71,33,4,103,204,17,238,164,23,50,76,21,77,119,218,251,72,121,47,196,16},
  {154,191,14,215,80,28,117,152,174,124,147,150,211,165,123,33,244,66,101,67,128,44,23,64,71,244,213,17,103,240,254,44},
  {89,145,129,24,248,62,149,212,214,140,43,190,89,228,1,213,214,167,111,57,89,200,78,150,104,191,130,246,83,119,194,71},
  {242,118,80,48,12,130,98,141,232,17,141,251,79,212,185,116,63,245,235,89,160,19,152,149,206,94,219,197,89,73,22,2},
  {239,138,220,36,139,47,198,44,90,0,48,81,44,12,104,228,54,9,240,224,5,20,247,183,178,146,78,154,237,252,122,7},
  {60,113,93,225,240,169,197,176,14,86,252,209,74,154,11,127,32,135,115,151,143,37,124,35,150,114,120,154,23,216,167,39},
  {23,180,18,131,215,44,27,48,91,253,165,223,233,204,200,233,114,107,194,7,218,183,23,167,130,77,48,18,162,130,204,16},
  {231,235,59,35,147,122,136,163,184,162,214,191,159,112,70,221,175,159,182,89,172,92,124,5,118,13,239,84,174,235,227,40},
  {89,198,80,48,244,56,47,214,227,188,88,156,193,94,9,89,75,209,109,178,3,109,160,247,107,98,103,234,213,41,188,13},
  {120,109,244,14,120,248,5,253,209,142,9,226,88,17,109,141,148,174,208,247,226,254,102,22,49,239,234,213,17,172,70,17},
  {20,179,156,100,59,90,228,162,230,74,170,58,239,231,51,159,90,52,134,152,179,190,187,121,146,54,2,136,4,16,81,100},
  {19,160,110,39,86,191,120,66,143,156,210,157,236,183,79,168,239,218,176,4,252,132,170,55,119,151,50,59,110,207,54,8},
  {149,77,244,53,53,179,126,41,219,42,231,103,64,15,240,173,90,157,174,130,89,199,42,206,228,212,76,252,106,140,56,21},
  {1,181,30,167,122,145,55,57,251,166,148,170,197,8,104,79,141,210,253,40,34,8,85,24,41,74,122,67,62,236,0,24},
  {95,11,182,96,11,182,96,11,182,96,11,182,96,11,182,96,11,182,96,11,182,96,11,182,96,11,182,96,11,182,96,11},
  {109,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,49},
  {78,208,23,244,5,125,65,95,208,23,244,5,125,65,95,208,23,244,5,125,65,95,208,23,244,5,125,65,95,208,23,116},
  {223,121,158,231,121,158,231,121,158,231,121,158,231,121,158,231,121,158,231,121,158,231,121,158,231,121,158,231,121,158,231,57},
  {123,12,206,199,224,124,12,206,199,224,124,12,206,199,224,124,12,206,199,224,124,12,206,199,224,124,12,206,199,224,124,12},
  {191,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,92},
  {246,150,111,249,150,111,249,150,111,249,150,111,249,150,111,249,150,111,249,150,111,249,150,111,249,150,111,249,150,111,249,22},
  {227,26,202,107,40,175,161,188,134,242,26,202,107,40,175,161,188,134,242,26,202,107,40,175,161,188,134,242,26,202,107,104},
  {237,89,55,152,34,159,117,131,41,242,89,55,152,34,159,117,131,41,242,89,55,152,34,159,117,131,41,242,89,55,152,34},
  {50,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,46},
  {114,197,87,124,197,87,124,197,87,124,197,87,124,197,87,124,197,87,124,197,87,124,197,87,124,197,87,124,197,87,124,69},
  {208,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,210,18},
  {139,108,178,201,38,155,108,178,201,38,155,108,178,201,38,155,108,178,201,38,155,108,178,201,38,155,108,178,201,38,155,108},
  {253,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,19},
  {133,49,198,24,99,140,49,198,24,99,140,49,198,24,99,140,49,198,24,99,140,49,198,24,99,140,49,198,24,99,140,49},
  {5,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,17,81},
  {48,141,176,220,211,8,203,61,141,176,220,211,8,203,61,141,176,220,211,8,203,61,141,176,220,211,8,203,61,141,176,92},
  {216,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,22},
  {231,18,218,75,104,47,161,189,132,246,18,218,75,104,47,161,189,132,246,18,218,75,104,47,161,189,132,246,18,218,75,104},
  {113,98,39,118,98,39,118,98,39,118,98,39,118,98,39,118,98,39,118,98,39,118,98,39,118,98,39,118,98,39,118,34},
  {207,122,20,174,71,225,122,20,174,71,225,122,20,174,71,225,122,20,174,71,225,122,20,174,71,225,122,20,174,71,225,122},
  {75,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,69},
  {42,100,33,11,89,200,66,22,178,144,133,44,100,33,11,89,200,66,22,178,144,133,44,100,33,11,89,200,66,22,178,16},
  {218,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,98},
  {190,243,60,207,243,60,207,243,60,207,243,60,207,243,60,207,243,60,207,243,60,207,243,60,207,243,60,207,243,60,207,115},
  {145,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,57},
  {217,53,148,215,80,94,67,121,13,229,53,148,215,80,94,67,121,13,229,53,148,215,80,94,67,121,13,229,53,148,215,80},
  {100,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,92},
  {160,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,165,37},
  {250,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,39},
  {29,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34,34},
  {176,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,45},
  {226,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,196,78,236,68},
  {169,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,10},
  {199,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69,23,93,116,209,69},
  {34,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,51,115},
  {219,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,79},
  {96,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,219,182,109,91},
  {82,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,21},
  {87,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,31},
  {164,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,42},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,63},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,63},
  {73,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,95},
  {150,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,153,25},
  {155,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,170,106},
  {141,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,47},
  {18,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,199,113,28,71},
  {203,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,204,12},
  {38,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,186,232,162,139,46,58},
  {68,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,85,117},
  {11,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59,177,19,59},
  {61,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,146,36,73,82},
  {208,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,221,93},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,87},
  {77,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90,90},
  {137,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,227,56,142,35},
} ;

static const unsigned char precomputed_pow_inv25519_p[precomputed_pow_inv25519_NUM][crypto_pow_BYTES] = {
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {8,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {11,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {14,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {17,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {18,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {19,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {22,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {23,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {24,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {26,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {27,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {28,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {29,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {30,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {31,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {224,235,122,124,59,65,184,174,22,86,227,250,241,159,196,106,218,9,141,235,156,50,177,253,134,98,5,22,95,73,184,0},
  {93,11,17,172,237,85,188,115,119,16,223,201,212,27,197,64,137,178,244,179,157,47,164,117,22,93,38,107,152,98,219,3},
  {129,156,209,97,216,127,230,10,196,93,131,132,8,72,64,192,105,34,102,132,50,137,243,123,140,203,186,159,122,99,243,5},
  {38,232,149,143,194,178,39,176,69,195,244,137,242,239,152,240,213,223,172,5,211,198,51,57,177,56,2,136,109,83,252,5},
  {241,83,53,187,193,212,174,9,91,104,0,113,237,209,237,22,148,228,44,251,112,52,5,238,132,246,185,178,166,111,115,6},
  {18,224,16,248,231,39,70,45,164,115,44,162,53,252,179,95,2,27,27,175,27,137,178,112,130,229,197,157,121,62,86,7},
  {194,182,111,24,203,182,215,90,236,39,243,118,38,38,194,23,129,253,47,15,169,145,119,89,206,112,13,89,22,153,145,8},
  {147,51,243,23,109,22,116,225,167,164,124,223,197,41,98,238,117,177,164,47,181,37,180,135,79,242,51,29,115,141,98,14},
  {89,46,240,193,138,63,158,190,180,203,64,132,82,50,71,171,123,216,172,8,77,178,79,47,185,39,160,188,151,34,173,14},
  {40,12,236,4,15,64,117,117,178,230,19,71,19,19,53,18,49,14,105,80,148,246,106,11,48,194,189,8,254,67,5,16},
  {226,201,13,23,87,255,179,198,122,236,86,21,27,53,231,55,3,212,2,105,232,68,167,199,168,213,242,58,121,163,88,16},
  {195,82,178,226,156,181,31,63,226,89,157,119,162,33,68,89,2,238,250,158,169,215,75,100,121,252,214,208,192,209,207,16},
  {62,201,173,102,22,52,130,75,143,176,41,3,70,250,182,70,220,18,105,64,77,107,233,159,157,35,169,41,102,215,53,17},
  {147,246,106,224,201,236,94,37,219,83,4,251,201,149,209,68,14,88,2,140,90,210,82,209,250,61,250,245,196,119,203,26},
  {74,200,238,22,199,37,200,128,148,15,80,145,16,3,161,120,84,71,251,230,95,47,35,235,115,144,13,114,36,110,152,28},
  {160,225,228,246,105,56,121,30,159,138,77,105,238,152,39,107,135,216,88,143,45,42,48,113,178,206,104,225,13,18,88,30},
  {102,55,216,186,113,33,217,104,153,8,118,119,100,78,9,161,155,124,8,45,174,155,49,119,195,219,238,82,200,146,135,30},
  {151,17,196,2,87,132,196,237,216,35,128,12,206,236,145,70,248,205,163,114,199,5,70,56,57,215,52,143,116,213,125,32},
  {222,136,183,18,240,91,145,161,198,8,75,171,135,11,12,122,93,247,51,12,29,154,254,236,131,63,162,197,202,127,229,36},
  {71,203,163,127,61,162,83,158,17,109,46,29,186,163,167,133,48,164,87,132,47,234,224,217,22,54,128,190,165,114,54,40},
  {99,192,3,80,181,231,34,241,240,111,140,60,187,151,28,5,108,137,69,184,99,88,162,117,43,41,29,38,60,137,45,41},
  {147,132,37,198,44,214,20,254,65,84,134,32,250,107,231,206,116,145,140,241,95,160,187,244,243,122,162,135,196,58,176,44},
  {182,159,33,218,36,54,82,116,237,96,1,129,26,204,104,165,41,254,232,139,115,90,99,197,130,11,71,157,9,165,9,50},
  {114,233,7,56,117,61,113,42,219,47,222,196,187,179,103,150,132,19,88,35,101,124,46,111,170,44,145,221,251,228,49,51},
  {129,202,111,146,254,13,201,8,16,17,251,195,170,42,79,164,254,44,214,204,94,5,255,146,68,197,70,111,210,104,118,52},
  {14,25,214,142,146,214,128,244,9,127,35,0,152,86,204,178,194,68,100,30,69,101,220,141,133,105,202,131,69,103,144,52},
  {97,91,49,148,88,12,82,159,80,25,159,170,103,125,25,97,50,97,37,247,53,231,178,41,192,229,37,88,105,87,131,53},
  {80,220,254,202,182,2,43,205,234,179,239,143,144,151,179,219,73,11,24,224,170,180,219,236,77,25,42,150,70,252,236,53},
  {15,107,97,219,29,24,83,131,60,191,127,75,0,87,25,142,104,111,192,12,72,223,132,235,116,235,119,37,25,174,94,54},
  {95,235,188,21,7,218,30,39,173,117,137,120,82,227,47,225,213,187,57,139,194,14,221,177,217,128,254,43,252,35,130,55},
  {33,158,20,26,13,160,220,36,224,154,184,232,223,120,118,59,52,76,244,10,223,92,109,197,53,78,56,236,34,166,235,56},
  {82,132,241,36,117,25,9,104,119,241,135,180,34,67,21,3,178,207,125,108,103,19,65,132,188,57,50,69,54,29,245,58},
  {78,140,20,224,52,27,201,36,89,209,174,149,72,194,147,130,41,13,38,56,78,70,49,142,114,20,58,180,74,226,70,61},
  {138,245,193,250,5,5,97,181,241,120,249,83,158,60,19,176,141,136,110,14,187,255,222,14,136,207,86,173,56,42,64,67},
  {6,217,177,199,21,112,114,132,244,3,229,76,43,143,128,239,244,213,43,17,92,87,242,48,39,140,145,94,200,1,133,67},
  {247,225,74,212,231,250,151,164,161,154,159,172,20,204,251,209,176,144,38,181,66,69,13,80,80,173,202,184,221,127,239,70},
  {238,49,5,74,218,128,248,183,44,230,125,170,22,211,254,66,212,58,210,44,24,188,82,184,114,120,217,46,98,1,111,74},
  {146,204,227,148,117,91,30,160,102,89,96,24,211,48,38,147,31,14,146,155,210,151,224,150,193,8,137,112,0,191,54,76},
  {210,202,220,164,246,20,113,100,180,12,151,155,88,88,88,145,232,83,65,207,203,112,134,14,206,120,161,44,178,247,76,79},
  {255,73,130,191,221,80,59,217,184,28,130,67,82,52,14,70,154,222,20,67,247,149,40,129,54,201,105,110,239,159,95,80},
  {139,249,139,242,180,114,253,231,63,42,190,37,169,162,179,81,193,126,98,185,1,76,62,17,143,53,76,176,138,212,238,81},
  {47,163,188,90,33,133,4,135,193,239,35,48,111,132,3,67,53,104,115,61,241,83,172,241,49,30,77,80,53,164,132,86},
  {95,156,149,188,163,80,140,36,177,208,177,85,156,131,239,91,4,68,92,196,88,28,142,134,216,34,78,221,208,159,17,87},
  {111,10,86,115,2,46,170,54,148,52,250,47,50,56,43,7,216,138,19,111,126,8,39,124,220,10,24,40,198,115,160,92},
  {170,223,157,23,236,62,32,108,24,248,79,7,102,172,52,177,222,29,19,7,3,17,152,104,5,178,63,61,253,161,227,95},
  {187,201,42,73,246,165,102,43,122,7,159,244,24,84,225,192,11,129,148,42,208,164,22,129,133,144,163,118,103,85,204,96},
  {142,149,157,28,5,201,173,174,133,174,98,68,117,157,25,87,59,175,211,196,165,241,116,163,212,99,99,133,123,48,111,102},
  {125,212,64,236,173,27,166,51,72,55,177,70,196,58,53,4,38,199,138,229,115,240,42,58,41,57,15,0,28,63,21,108},
  {237,197,109,47,77,179,144,100,157,130,214,51,210,10,226,76,215,202,65,22,131,42,185,219,128,57,32,227,252,255,132,108},
  {125,45,159,13,208,200,222,43,12,33,0,254,227,97,97,32,232,25,53,199,145,219,119,66,145,110,82,242,17,249,163,118},
  {218,124,213,86,252,123,194,102,72,95,152,194,239,155,120,87,147,101,181,39,136,178,213,63,208,88,239,217,88,229,237,119},
  {189,68,80,58,53,159,69,72,96,63,22,164,219,217,177,104,30,162,150,170,40,227,107,49,103,70,18,244,141,182,4,120},
  {141,86,94,148,247,167,7,64,21,249,20,82,26,95,159,145,129,43,67,188,112,24,28,197,142,0,147,152,108,232,246,120},
  {109,105,188,79,1,147,89,93,3,188,177,189,165,192,94,147,244,7,23,147,84,125,168,186,44,88,176,120,191,227,206,121},
  {199,23,106,112,61,77,216,79,186,60,11,118,13,16,103,15,42,32,83,250,44,57,204,198,78,199,253,119,146,172,3,122},
  {16,144,21,200,119,145,159,194,55,138,26,190,154,139,72,46,187,132,134,43,89,106,125,20,10,208,228,120,140,13,108,123},
  {5,97,68,33,155,166,192,59,217,105,121,61,195,81,22,19,33,196,102,252,161,11,78,213,178,193,220,63,254,134,221,124},
  {174,123,183,169,128,93,4,127,95,144,172,90,27,196,103,169,70,105,66,180,68,78,96,240,173,118,98,214,113,217,176,127},
  {224,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {225,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {226,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {227,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {228,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {229,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {230,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {231,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {232,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {233,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {234,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {235,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {237,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {238,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {239,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {240,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {241,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {245,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {248,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {250,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {252,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {253,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {254,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {8,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {11,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {14,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {17,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {18,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {19,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {22,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {23,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {24,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {26,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {27,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {28,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {29,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {30,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {31,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {205,235,122,124,59,65,184,174,22,86,227,250,241,159,196,106,218,9,141,235,156,50,177,253,134,98,5,22,95,73,184,128},
  {160,37,10,26,225,245,150,67,90,82,89,131,201,239,182,175,225,228,27,31,200,10,216,0,158,161,240,56,190,37,98,129},
  {126,193,215,50,203,125,133,127,196,72,203,3,172,168,69,4,124,66,128,137,155,138,79,62,12,209,148,182,155,150,234,132},
  {214,187,57,121,219,7,180,245,79,199,28,235,3,229,24,162,210,72,140,56,149,230,163,235,193,41,191,151,78,2,170,133},
  {19,232,149,143,194,178,39,176,69,195,244,137,242,239,152,240,213,223,172,5,211,198,51,57,177,56,2,136,109,83,252,133},
  {62,21,89,26,242,137,228,142,221,218,26,139,122,9,150,190,211,255,231,171,115,79,10,216,158,142,92,214,120,151,143,136},
  {81,108,252,226,91,123,2,58,201,115,182,53,216,106,226,19,255,189,15,42,27,210,228,103,255,63,80,6,101,105,24,137},
  {110,167,30,114,238,200,91,206,189,246,80,143,240,154,158,29,237,114,89,30,86,41,243,89,252,236,82,177,174,134,206,142},
  {223,178,224,94,39,7,12,55,105,205,4,119,36,114,140,55,63,97,181,199,242,8,236,220,146,170,18,174,209,107,101,143},
  {27,248,18,51,151,208,228,149,143,27,140,249,190,131,118,98,48,85,214,237,106,115,9,75,148,22,7,29,151,218,7,144},
  {28,72,94,40,137,202,51,69,198,88,34,49,16,78,17,30,21,240,75,177,180,86,175,148,58,181,236,40,244,45,62,146},
  {158,186,152,57,234,203,74,33,92,196,150,65,216,222,226,127,191,18,161,191,239,44,29,43,240,246,221,186,114,111,63,151},
  {250,195,130,230,38,225,2,225,206,208,205,149,59,249,120,21,49,202,81,229,137,184,10,137,242,202,97,38,245,188,212,151},
  {54,117,120,76,28,202,80,109,96,30,159,160,217,89,62,54,1,69,113,116,99,75,17,217,85,101,142,201,51,2,101,152},
  {149,177,114,234,252,75,142,235,123,59,172,102,243,44,203,193,27,157,41,154,123,24,157,151,153,191,27,29,59,251,18,154},
  {102,100,56,74,188,101,169,164,35,169,187,217,141,243,3,55,161,77,32,41,110,158,111,3,56,227,131,126,230,180,45,155},
  {67,192,36,40,3,194,139,15,233,18,214,181,68,174,148,224,232,149,109,38,74,153,50,14,188,66,236,205,202,241,226,157},
  {66,173,58,243,101,117,198,170,56,209,211,188,81,55,42,168,124,219,190,185,49,16,182,103,120,142,68,168,211,236,170,161},
  {214,158,46,187,158,227,121,43,148,253,243,11,84,196,151,224,248,8,133,187,199,38,200,165,82,79,87,227,252,32,75,163},
  {183,25,105,23,254,22,118,190,172,248,135,177,162,56,80,228,241,77,155,50,49,40,189,53,222,77,10,195,159,180,36,169},
  {192,74,224,195,102,131,154,194,42,115,141,84,75,168,130,177,203,227,246,119,46,128,66,54,254,220,160,138,57,28,88,169},
  {57,236,93,48,192,237,101,125,11,22,182,131,135,237,215,120,196,89,208,70,90,68,155,140,49,154,250,205,35,42,34,171},
  {34,164,149,255,137,15,71,180,197,252,42,2,169,202,248,32,183,77,134,43,105,218,92,204,154,157,143,56,212,103,51,172},
  {86,30,104,181,170,61,38,192,4,54,101,16,25,25,44,81,66,135,114,62,180,143,156,81,136,143,16,207,57,45,167,173},
  {91,245,72,216,90,199,134,172,165,93,168,104,67,11,211,223,41,7,195,248,1,92,138,40,187,117,230,81,86,91,26,174},
  {151,141,223,224,103,64,134,109,9,122,230,95,229,166,153,94,19,16,90,73,179,141,203,36,65,81,227,10,205,31,125,174},
  {113,11,129,179,36,154,147,133,7,238,233,76,251,81,65,246,52,129,24,169,217,110,172,227,54,251,102,134,122,59,118,176},
  {56,220,154,85,84,3,159,223,248,143,175,199,155,173,14,212,145,152,55,74,49,90,203,199,112,120,39,138,239,31,10,177},
  {135,104,144,71,33,194,33,133,3,24,60,117,215,178,87,97,114,162,126,199,73,119,145,178,104,158,203,9,101,140,123,179},
  {253,164,38,103,254,232,227,99,197,120,216,11,141,6,80,149,82,139,5,14,109,151,171,95,140,120,105,222,29,75,225,179},
  {16,6,32,40,247,87,109,57,38,33,228,150,230,17,242,15,173,193,172,63,190,117,181,169,233,173,88,122,190,157,147,180},
  {220,96,255,75,79,6,125,48,182,249,141,55,159,169,209,228,80,104,66,94,114,158,242,122,235,91,168,124,40,195,178,181},
  {49,227,89,1,60,152,183,120,89,158,57,84,116,231,55,213,6,81,44,123,127,232,229,121,133,50,184,254,130,13,213,183},
  {196,226,59,10,69,172,238,59,12,106,114,210,55,110,197,177,226,149,26,253,22,198,190,45,156,32,60,94,30,193,155,190},
  {46,212,240,212,71,186,251,161,145,114,228,149,106,116,148,58,17,73,40,233,26,110,117,88,47,188,22,112,183,65,205,191},
  {108,115,168,221,29,29,10,110,178,191,144,33,98,69,158,179,107,50,30,217,56,235,99,100,137,115,187,170,108,225,145,192},
  {88,23,254,127,125,50,108,114,175,73,57,176,55,155,159,156,225,178,180,167,133,57,53,105,82,105,36,21,156,137,88,193},
  {13,161,208,73,87,241,51,97,97,233,39,57,195,242,131,151,162,177,26,48,254,248,44,92,233,228,68,233,92,67,161,193},
  {55,80,74,36,97,26,143,182,137,191,36,191,100,77,67,3,107,215,201,77,181,94,138,165,44,14,150,247,29,118,206,193},
  {46,145,77,66,148,249,125,134,154,224,31,41,99,28,127,185,90,83,173,89,42,15,229,221,132,185,57,226,109,127,221,195},
  {174,142,95,26,102,77,10,180,133,149,255,236,89,72,70,235,43,152,127,141,53,255,3,202,240,70,104,183,125,242,114,196},
  {144,124,64,244,223,107,164,210,14,118,122,26,192,45,89,87,111,104,108,123,107,244,184,92,115,236,27,64,129,228,241,199},
  {172,37,106,178,186,134,115,186,94,177,212,105,200,72,126,162,183,252,84,234,71,113,123,115,27,145,22,179,65,52,126,200},
  {242,131,4,146,239,149,68,63,138,77,104,43,251,247,136,71,173,80,18,182,27,74,218,145,119,152,217,220,241,59,214,200},
  {108,209,156,236,156,166,225,245,1,255,55,161,87,186,87,163,186,173,58,130,239,62,58,38,221,97,184,184,72,35,198,201},
  {178,234,83,79,237,151,116,195,96,77,61,52,122,69,7,81,11,193,156,118,222,117,126,243,15,14,208,220,120,72,211,204},
  {204,131,141,157,131,216,59,249,236,247,255,242,5,235,227,226,14,130,40,32,228,224,200,76,162,5,173,143,35,248,18,207},
  {35,5,252,51,253,104,202,78,1,55,89,199,130,119,215,153,228,235,211,185,24,49,168,133,15,191,16,243,69,249,102,209},
  {236,192,151,177,212,242,244,63,100,18,73,46,25,215,142,32,185,134,95,138,231,167,29,123,189,146,175,242,71,212,160,210},
  {244,123,195,157,46,59,3,176,45,171,88,28,63,64,60,158,176,233,111,81,205,205,200,253,20,53,42,25,236,248,6,211},
  {250,190,87,147,129,35,208,208,243,3,97,112,183,206,100,81,148,116,102,25,203,54,44,111,84,135,214,181,143,175,51,212},
  {214,76,88,159,6,174,51,81,248,78,106,177,236,186,192,219,15,190,150,146,53,178,114,58,92,104,251,202,20,7,53,214},
  {244,140,114,244,62,170,41,248,47,225,143,61,151,206,239,48,118,198,17,125,73,82,89,168,186,241,36,23,150,83,109,214},
  {76,156,149,188,163,80,140,36,177,208,177,85,156,131,239,91,4,68,92,196,88,28,142,134,216,34,78,221,208,159,17,215},
  {139,115,46,248,73,231,193,130,253,199,47,18,242,136,178,77,224,246,139,248,167,75,84,143,72,149,87,254,161,252,22,217},
  {93,88,227,37,114,65,2,2,202,140,162,68,186,54,73,105,10,189,55,176,194,12,242,71,98,179,159,127,78,41,232,221},
  {143,142,107,198,237,68,83,189,7,239,167,232,157,187,159,139,177,161,94,141,23,17,47,115,237,6,59,132,12,168,110,223},
  {22,46,135,210,121,177,182,90,213,250,68,254,13,187,98,28,198,232,133,160,89,202,235,65,61,199,204,207,44,54,181,223},
  {35,238,173,80,157,134,161,13,214,74,174,154,61,187,55,219,216,57,244,57,107,141,22,183,40,187,178,130,223,175,253,223},
  {135,41,111,62,134,23,247,181,64,139,85,137,237,245,175,152,106,177,57,133,253,184,141,40,45,4,212,79,52,174,254,223},
  {210,123,212,47,191,41,21,215,166,78,180,73,218,218,5,4,10,253,18,3,94,228,96,173,112,39,183,132,46,217,2,224},
  {232,64,0,7,2,215,145,253,104,71,120,112,176,10,157,243,134,72,12,11,14,242,103,82,237,168,70,157,242,247,10,224},
  {27,151,33,144,144,198,124,112,170,187,16,189,113,255,84,221,81,127,37,17,143,58,64,32,63,1,169,157,31,50,214,234},
  {133,78,53,177,58,165,28,169,160,156,74,247,22,18,110,17,171,95,48,187,254,210,138,117,179,157,48,108,6,186,193,235},
  {25,216,239,98,90,43,174,36,141,165,171,161,236,144,0,188,173,204,147,113,37,224,186,252,199,176,246,185,163,33,96,236},
  {213,243,48,11,157,197,31,38,25,41,241,11,72,102,114,49,205,71,48,159,37,11,204,163,154,127,238,77,137,218,185,236},
  {168,188,69,201,152,212,193,230,142,199,30,182,79,33,156,162,216,59,153,29,99,172,127,139,16,93,159,100,40,94,29,239},
  {149,33,93,182,34,43,131,42,7,185,191,233,56,97,255,161,24,75,9,111,70,63,117,14,41,111,225,237,10,11,132,240},
  {190,57,20,188,220,113,68,236,247,124,46,122,44,228,172,184,241,46,202,132,38,79,248,49,26,159,251,67,32,15,212,245},
  {180,50,28,164,12,27,60,90,16,84,200,202,112,144,207,37,183,210,179,165,89,2,57,211,178,44,72,179,209,50,225,245},
  {170,254,96,130,76,58,108,247,243,157,78,32,139,192,145,181,120,149,66,18,48,255,179,45,248,153,25,250,125,241,185,247},
  {180,23,106,112,61,77,216,79,186,60,11,118,13,16,103,15,42,32,83,250,44,57,204,198,78,199,253,119,146,172,3,250},
  {91,94,189,239,65,25,164,16,97,157,57,15,98,46,197,53,97,137,208,106,56,113,72,246,21,174,146,118,48,34,127,250},
  {168,56,27,159,191,8,6,120,33,137,112,137,74,90,42,223,96,94,197,77,20,87,240,251,40,205,31,126,97,161,79,252},
  {168,232,248,183,162,21,22,250,43,70,55,127,62,205,17,69,125,80,249,69,191,37,72,39,7,122,11,134,30,177,89,253},
  {239,116,12,234,4,82,0,178,159,114,155,225,154,162,111,134,127,181,234,121,180,60,229,249,105,134,250,147,212,174,170,253},
  {19,137,17,15,242,181,77,55,255,2,106,123,184,194,112,245,204,230,90,240,209,188,209,175,61,67,172,143,75,148,1,255},
  {89,63,37,111,54,84,103,221,103,152,61,93,47,225,181,222,152,84,107,66,201,189,152,146,246,137,153,114,219,135,105,255},
  {192,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {193,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {194,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {195,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {196,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {197,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {198,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {199,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {200,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {201,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {202,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {203,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {204,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {205,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {206,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {207,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {208,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {209,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {210,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {211,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {212,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {213,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {214,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {215,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {216,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {217,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {218,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {219,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {220,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {221,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {222,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {223,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {224,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {225,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {226,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {227,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {228,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {229,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {230,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {231,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {232,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {233,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {234,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {235,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {237,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {238,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {239,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {240,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {241,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {245,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {248,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {250,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {252,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {253,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {254,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
} ;

static void test_pow_inv25519_impl(long long impl)
{
  unsigned char *q = test_pow_inv25519_q;
  unsigned char *p = test_pow_inv25519_p;
  unsigned char *q2 = test_pow_inv25519_q2;
  unsigned char *p2 = test_pow_inv25519_p2;
  long long qlen = crypto_pow_BYTES;
  long long plen = crypto_pow_BYTES;

  if (targeti && strcmp(targeti,lib25519_dispatch_pow_inv25519_implementation(impl))) return;
  if (impl >= 0) {
    crypto_pow = lib25519_dispatch_pow_inv25519(impl);
    printf("pow_inv25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_pow_inv25519_implementation(impl),lib25519_dispatch_pow_inv25519_compiler(impl));
  } else {
    crypto_pow = lib25519_pow_inv25519;
    printf("pow_inv25519 selected implementation %s compiler %s\n",lib25519_pow_inv25519_implementation(),lib25519_pow_inv25519_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 512 : 64;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {

      output_prepare(q2,q,qlen);
      input_prepare(p2,p,plen);
      crypto_pow(q,p);
      checksum(q,qlen);
      output_compare(q2,q,qlen,"crypto_pow");
      input_compare(p2,p,plen,"crypto_pow");

      double_canary(q2,q,qlen);
      double_canary(p2,p,plen);
      crypto_pow(q2,p2);
      if (memcmp(q2,q,qlen) != 0) fail("failure: crypto_pow is nondeterministic\n");

      double_canary(q2,q,qlen);
      double_canary(p2,p,plen);
      crypto_pow(p2,p2);
      if (memcmp(p2,q,qlen) != 0) fail("failure: crypto_pow does not handle p=q overlap\n");
      memcpy(p2,p,plen);
    }
    checksum_expected(pow_inv25519_checksums[checksumbig]);
  }
  for (long long precomp = 0;precomp < precomputed_pow_inv25519_NUM;++precomp) {
    output_prepare(q2,q,crypto_pow_BYTES);
    input_prepare(p2,p,crypto_pow_BYTES);
    memcpy(p,precomputed_pow_inv25519_p[precomp],crypto_pow_BYTES);
    memcpy(p2,precomputed_pow_inv25519_p[precomp],crypto_pow_BYTES);
    crypto_pow(q,p);
    if (memcmp(q,precomputed_pow_inv25519_q[precomp],crypto_pow_BYTES)) {
      fail("failure: crypto_pow fails precomputed test vectors\n");
      printf("expected q: ");
      for (long long pos = 0;pos < crypto_pow_BYTES;++pos) printf("%02x",precomputed_pow_inv25519_q[precomp][pos]);
      printf("\n");
      printf("received q: ");
      for (long long pos = 0;pos < crypto_pow_BYTES;++pos) printf("%02x",q[pos]);
      printf("\n");
    }
    output_compare(q2,q,crypto_pow_BYTES,"crypto_pow");
    input_compare(p2,p,crypto_pow_BYTES,"crypto_pow");
  }
}

static void test_pow_inv25519(void)
{
  if (targeto && strcmp(targeto,"pow")) return;
  if (targetp && strcmp(targetp,"inv25519")) return;
  test_pow_inv25519_q = alignedcalloc(crypto_pow_BYTES);
  test_pow_inv25519_p = alignedcalloc(crypto_pow_BYTES);
  test_pow_inv25519_q2 = alignedcalloc(crypto_pow_BYTES);
  test_pow_inv25519_p2 = alignedcalloc(crypto_pow_BYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("pow_inv25519 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_pow_inv25519();++impl)
      forked(test_pow_inv25519_impl,impl);
    ++test_pow_inv25519_q;
    ++test_pow_inv25519_p;
    ++test_pow_inv25519_q2;
    ++test_pow_inv25519_p2;
  }
}
#undef crypto_pow_BYTES


/* ----- nP, derived from supercop/crypto_nP/try.c */
static const char *nP_montgomery25519_checksums[] = {
  "b861d66109b42359e5994ed57ae566827c345b65a9d0671700320b82888397ec",
  "740924011f3448f65299f61b087f74a6eb9651a4203dfbf621d2bec54e149405",
} ;

static void (*crypto_nP)(unsigned char *,const unsigned char *,const unsigned char *);
#define crypto_nP_SCALARBYTES lib25519_nP_montgomery25519_SCALARBYTES
#define crypto_nP_POINTBYTES lib25519_nP_montgomery25519_POINTBYTES

static unsigned char *test_nP_montgomery25519_q;
static unsigned char *test_nP_montgomery25519_n;
static unsigned char *test_nP_montgomery25519_p;
static unsigned char *test_nP_montgomery25519_q2;
static unsigned char *test_nP_montgomery25519_n2;
static unsigned char *test_nP_montgomery25519_p2;

#define precomputed_nP_montgomery25519_NUM 372

static const unsigned char precomputed_nP_montgomery25519_q[precomputed_nP_montgomery25519_NUM][crypto_nP_POINTBYTES] = {
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {129,42,27,226,56,122,209,122,33,165,197,59,204,245,117,180,65,210,211,35,205,129,134,7,75,25,155,208,55,142,9,28},
  {162,93,163,159,20,188,94,13,51,72,116,75,189,214,110,123,124,9,238,31,79,7,237,111,225,81,4,77,40,149,130,82},
  {121,223,164,139,202,179,32,228,109,181,229,181,139,0,181,124,140,230,145,47,130,223,137,147,17,118,27,113,92,100,228,98},
  {239,133,37,52,169,133,200,97,68,65,139,30,226,202,18,162,33,55,136,193,253,245,29,8,169,139,55,159,251,46,166,107},
  {66,245,111,3,98,139,24,189,155,145,236,234,46,217,98,153,28,247,110,223,14,222,218,185,48,86,154,56,134,240,21,93},
  {85,216,96,126,239,24,171,238,158,88,201,26,202,187,130,154,241,114,114,95,95,29,6,98,33,120,172,125,108,180,197,114},
  {247,143,27,107,199,100,117,167,157,168,246,0,110,30,208,91,218,135,95,226,47,159,110,22,230,94,31,219,179,141,240,71},
  {8,230,182,92,48,214,174,119,128,155,246,232,247,243,230,27,18,49,135,212,35,67,70,253,49,55,189,111,188,19,5,108},
  {135,110,236,86,44,66,16,135,102,160,234,251,75,169,40,162,23,252,170,56,119,186,208,82,4,133,157,1,170,36,83,77},
  {177,112,232,197,251,252,99,183,10,138,35,150,194,24,91,208,73,35,104,140,114,246,168,207,34,63,19,124,216,198,168,44},
  {147,18,46,232,128,232,34,246,212,145,27,134,255,227,150,31,236,60,94,72,20,254,53,138,162,159,253,128,210,223,193,23},
  {91,38,221,90,220,250,249,107,234,224,251,4,109,192,201,181,132,216,71,254,152,204,74,202,124,239,186,53,86,16,183,55},
  {141,157,167,22,166,233,3,111,51,116,212,34,62,101,204,129,57,112,28,177,46,211,164,76,149,205,39,197,223,238,176,48},
  {217,115,32,35,66,144,65,8,25,143,191,82,225,61,129,97,90,4,150,71,57,17,1,231,14,199,229,207,221,174,89,13},
  {168,30,239,136,43,9,85,111,60,43,82,60,196,16,54,69,36,18,140,202,217,195,39,26,118,7,90,218,115,19,147,79},
  {186,206,120,110,192,152,199,91,199,20,152,117,205,170,37,52,176,120,41,54,230,114,120,138,135,222,72,41,155,2,120,95},
  {8,100,222,18,220,20,218,99,254,188,214,171,119,88,234,228,50,86,45,103,188,156,0,91,129,17,52,149,192,244,246,108},
  {38,75,14,165,222,53,13,233,72,76,179,164,11,252,190,89,47,54,76,98,36,68,200,205,16,81,242,128,73,117,120,89},
  {45,251,59,61,97,219,78,236,93,144,47,91,181,225,215,145,201,10,128,250,30,245,216,88,147,237,63,156,243,103,228,122},
  {130,247,47,178,248,52,54,192,213,119,42,9,99,242,192,111,160,106,220,145,217,226,58,89,18,9,81,100,252,221,221,46},
  {110,7,125,207,66,148,213,123,27,131,1,51,171,70,41,235,248,209,1,139,29,117,199,115,19,229,10,124,38,119,167,95},
  {121,232,151,106,249,132,75,30,73,205,71,105,61,158,181,131,150,161,64,69,138,140,30,8,40,61,72,109,102,207,107,46},
  {189,197,102,194,52,207,156,183,67,27,246,1,63,74,157,164,178,75,166,128,84,36,11,214,239,86,170,86,145,66,109,117},
  {198,237,17,59,13,46,43,8,99,121,23,190,202,234,240,252,155,193,97,152,217,159,15,251,82,54,246,226,180,134,32,123},
  {171,226,12,187,31,76,168,13,76,46,35,191,62,137,48,2,209,188,191,149,42,154,3,16,133,1,173,136,97,149,120,43},
  {161,223,243,227,234,158,42,211,46,211,205,39,166,23,202,203,26,65,220,32,37,204,145,199,242,178,16,111,221,227,215,116},
  {227,16,183,177,15,162,157,55,149,46,188,120,29,196,13,218,249,145,106,62,56,120,249,64,11,212,158,254,153,222,134,102},
  {123,3,226,18,231,250,54,63,5,20,66,86,154,30,243,71,196,222,234,139,221,232,16,130,78,97,139,198,176,238,220,27},
  {15,216,90,115,25,121,207,41,112,66,176,118,79,176,85,9,162,102,135,200,230,98,135,24,229,37,74,23,87,146,107,75},
  {167,69,92,82,21,33,118,83,239,21,209,140,157,76,171,158,207,12,119,240,246,209,154,135,0,78,7,81,200,101,186,99},
  {8,139,160,142,177,217,68,114,183,108,120,157,4,55,210,197,122,181,154,109,149,1,44,118,166,30,19,188,14,141,44,35},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {81,212,247,2,165,203,138,13,12,188,47,181,141,146,236,57,186,2,196,20,129,165,17,228,157,55,212,222,126,114,58,74},
  {228,228,179,142,119,218,163,179,18,100,15,16,219,88,146,63,140,252,124,190,66,108,232,165,55,163,242,240,143,207,151,79},
  {205,232,203,120,115,196,192,76,184,227,29,204,114,160,206,2,164,87,199,142,87,173,42,249,6,168,123,162,43,50,233,87},
  {116,162,107,174,184,107,105,141,168,164,21,209,119,243,166,176,21,209,28,31,65,131,190,203,128,255,94,131,55,69,185,35},
  {20,96,92,201,136,69,10,113,254,74,165,39,25,79,82,23,55,110,87,2,223,203,70,39,10,151,217,38,130,228,110,35},
  {18,244,59,81,248,218,145,225,18,224,235,185,206,228,171,123,58,219,224,244,155,107,209,155,205,67,188,62,88,237,239,40},
  {143,89,69,222,179,212,88,144,197,252,13,219,244,149,71,33,78,245,157,244,93,18,109,20,156,74,71,239,48,201,0,36},
  {242,17,147,144,73,76,20,33,70,38,176,162,163,20,110,251,153,126,98,180,47,185,101,227,13,31,169,7,65,168,217,33},
  {15,110,14,218,109,97,208,5,119,1,172,107,129,13,15,212,190,36,103,67,142,60,73,93,73,192,62,251,214,1,248,33},
  {56,21,5,209,153,28,67,172,37,109,174,13,123,39,248,41,6,32,97,215,136,83,40,105,144,155,46,38,66,191,92,111},
  {124,34,221,39,200,223,110,235,114,121,23,183,13,139,29,105,164,48,137,253,75,215,206,50,99,236,183,40,157,114,126,8},
  {209,78,224,191,218,3,170,4,33,85,112,53,175,85,212,14,190,108,45,17,159,66,4,171,44,222,255,213,156,13,224,24},
  {65,230,67,147,81,29,193,205,106,166,158,26,136,203,16,178,103,85,161,96,102,107,105,133,105,84,64,40,68,43,195,91},
  {32,163,168,139,99,25,127,116,102,44,81,149,235,117,39,200,88,48,216,100,104,104,82,71,66,252,153,234,179,66,237,60},
  {140,24,30,176,134,8,31,231,89,88,140,197,126,181,244,132,204,147,137,5,26,207,233,155,233,228,209,90,38,40,202,118},
  {33,8,41,21,196,57,27,11,142,108,106,99,167,6,192,237,255,230,224,36,117,7,242,70,226,111,243,213,61,23,15,53},
  {40,105,249,168,34,223,38,206,115,50,194,65,164,57,143,11,92,115,159,142,217,0,132,251,237,153,184,34,195,196,37,112},
  {148,246,73,15,184,154,236,223,224,226,251,31,64,159,206,121,178,247,22,39,70,164,110,173,199,108,204,202,3,77,219,5},
  {227,182,214,47,131,182,21,112,64,150,229,231,76,159,115,101,254,178,24,4,238,128,253,76,19,54,242,153,40,8,96,107},
  {234,96,13,185,171,29,139,232,201,74,241,138,212,122,221,43,20,207,254,56,35,85,151,69,164,53,40,138,8,222,33,20},
  {228,112,130,63,107,81,23,249,186,10,22,86,99,174,198,10,157,41,6,67,30,228,13,31,83,154,42,32,230,159,64,116},
  {255,129,18,169,7,117,207,247,14,22,204,204,150,149,241,109,184,234,176,191,204,226,59,95,60,137,13,206,30,84,86,58},
  {216,58,60,48,207,147,39,42,240,212,174,51,254,97,191,197,184,179,199,46,208,162,235,199,58,84,206,231,135,146,53,3},
  {166,240,57,159,250,12,176,99,21,179,66,164,251,8,22,151,94,185,246,184,60,34,80,126,163,254,145,138,250,217,239,55},
  {58,144,221,68,149,243,211,220,123,4,9,128,144,125,157,1,216,48,133,89,2,18,245,83,40,245,77,176,207,170,188,126},
  {49,255,250,144,86,100,223,59,248,213,193,15,175,5,233,5,93,3,178,104,161,187,55,226,138,69,127,23,37,210,251,30},
  {152,251,21,58,4,206,92,66,175,109,126,19,17,4,136,153,160,151,177,196,251,14,160,205,121,23,97,7,196,38,40,91},
  {97,215,117,83,136,126,243,115,111,73,20,197,165,85,65,129,118,57,81,198,161,170,79,46,219,7,174,251,203,113,79,67},
  {167,219,221,104,83,243,49,44,209,154,240,113,203,222,164,106,101,186,231,195,242,96,115,182,109,1,17,204,66,19,22,106},
  {36,55,236,229,245,79,207,49,203,20,232,169,77,191,14,108,115,63,173,247,108,115,162,142,117,229,217,72,26,175,68,106},
  {146,149,77,134,215,1,44,221,5,86,144,64,62,218,24,92,66,95,109,169,110,156,123,103,200,136,175,181,79,206,77,30},
  {140,226,113,176,170,125,185,72,144,99,240,177,95,243,32,51,68,104,183,169,16,133,225,52,210,200,4,218,182,105,37,104},
  {63,4,23,253,35,192,48,242,97,105,156,93,94,131,136,135,234,220,44,198,162,110,76,234,4,24,99,125,56,100,216,63},
  {237,127,188,222,241,16,254,146,84,195,223,127,96,31,31,85,67,186,200,162,130,186,146,173,211,182,140,232,125,203,222,2},
  {181,222,209,109,182,226,150,229,99,94,129,129,202,251,88,218,173,54,120,32,159,13,194,200,151,39,193,64,72,6,44,24},
  {2,53,245,62,112,133,194,113,152,186,76,56,71,101,59,255,229,204,223,23,8,98,41,236,15,141,174,84,176,31,225,105},
  {176,85,166,39,118,113,247,129,215,210,93,45,236,83,4,34,146,19,90,66,49,85,204,206,232,3,26,141,96,101,112,4},
  {133,56,209,64,169,248,218,8,156,22,108,249,93,19,19,20,10,202,251,221,92,152,21,138,66,150,67,179,210,116,53,91},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {168,88,184,124,168,95,193,139,180,122,21,83,7,167,242,61,142,140,57,104,68,191,153,220,184,6,164,114,78,208,43,3},
  {27,140,184,246,181,174,139,12,170,14,22,32,229,243,80,102,110,113,2,211,170,28,63,207,246,75,190,124,83,154,186,59},
  {100,160,131,243,53,186,251,114,8,52,151,27,30,160,13,72,173,210,109,126,132,158,106,196,4,88,22,212,130,242,254,124},
  {201,155,12,75,225,4,142,124,244,58,18,139,255,150,172,144,168,104,16,32,44,188,45,70,238,199,97,32,247,139,217,62},
  {45,234,189,106,93,41,197,201,25,229,188,27,200,109,20,28,157,91,139,137,162,0,243,82,191,136,58,230,14,123,21,126},
  {96,255,146,161,76,7,96,239,148,166,23,180,53,104,37,186,46,128,89,3,171,67,92,201,226,96,241,85,156,155,106,26},
  {255,4,221,217,210,234,93,23,172,194,86,106,81,143,125,64,202,114,105,83,19,133,166,47,172,39,142,122,237,86,128,70},
  {110,113,117,196,107,121,84,48,49,149,121,164,3,133,75,138,55,101,216,33,156,143,15,96,198,175,237,75,185,14,12,69},
  {88,3,144,195,28,174,124,41,102,180,112,240,10,31,224,247,202,153,22,182,105,85,35,154,190,93,45,156,96,6,86,9},
  {184,104,230,182,129,76,150,111,49,140,51,165,182,24,57,150,50,121,20,24,229,80,67,104,254,120,83,13,14,124,45,86},
  {69,121,86,227,222,239,11,208,33,176,252,85,217,169,174,186,246,248,170,213,13,176,85,81,25,23,34,81,251,61,27,15},
  {119,95,0,82,108,75,113,6,44,136,6,36,243,251,32,7,104,182,129,199,63,114,69,233,251,107,213,42,64,188,144,76},
  {107,174,134,87,216,186,174,9,66,224,28,33,211,23,25,13,139,44,73,125,110,242,96,106,72,144,93,169,50,27,180,6},
  {7,18,149,199,142,15,15,235,24,15,67,212,106,110,177,124,26,214,252,13,89,48,12,66,35,106,43,25,66,132,159,100},
  {103,231,168,16,38,143,127,110,16,71,251,28,231,215,228,222,85,87,158,104,130,141,45,189,3,180,179,10,162,46,78,92},
  {73,23,176,31,225,42,167,200,73,7,156,3,193,59,105,14,42,173,194,190,183,110,83,43,127,58,195,19,51,229,36,37},
  {215,8,201,193,79,168,61,72,101,20,152,251,171,111,150,218,75,51,85,85,148,255,131,141,255,156,17,126,159,6,177,109},
  {84,52,11,125,245,132,150,18,72,100,216,10,97,116,194,219,120,17,230,231,77,86,11,115,18,17,117,83,207,244,176,124},
  {238,109,71,253,157,191,230,60,96,185,101,2,100,103,60,84,144,93,177,132,112,219,142,64,254,164,8,139,87,191,250,31},
  {134,205,13,226,154,77,112,59,108,242,192,203,108,5,89,119,105,54,99,227,158,177,209,151,225,8,176,145,62,72,244,107},
  {132,211,21,191,66,248,248,89,73,111,43,217,220,90,21,178,4,219,49,48,22,113,99,190,107,95,35,114,217,59,197,42},
  {96,202,115,189,116,193,131,183,206,245,166,211,213,163,181,114,114,156,236,147,196,192,27,239,83,67,106,173,100,236,252,47},
  {43,253,2,146,18,249,141,246,239,169,54,244,169,61,16,180,149,39,14,119,135,74,235,68,146,113,195,81,229,38,122,75},
  {167,175,126,31,208,11,138,29,108,204,241,125,175,14,115,81,219,83,39,19,165,200,222,35,120,16,71,153,151,169,50,104},
  {36,8,58,231,6,163,21,209,25,168,61,203,194,202,156,87,143,205,251,110,150,100,47,16,144,135,57,184,168,5,44,51},
  {179,73,180,255,248,60,100,186,199,247,245,45,119,213,164,143,28,85,7,91,72,48,154,97,161,216,224,199,234,9,254,79},
  {155,56,106,143,220,91,199,35,51,40,34,60,68,132,234,215,235,216,224,102,48,147,31,123,65,192,218,123,192,69,204,88},
  {135,115,33,191,144,231,20,161,98,28,77,82,146,186,0,121,148,163,84,123,175,168,159,183,95,66,121,8,30,105,215,104},
  {184,152,148,235,8,16,51,133,188,254,11,79,161,191,124,78,106,124,211,231,108,219,92,138,18,188,205,203,223,38,138,118},
  {98,215,92,179,246,154,125,32,18,145,5,39,203,210,146,30,53,83,227,20,109,186,64,31,179,85,248,148,129,219,43,39},
  {163,33,2,237,179,15,10,83,254,69,45,136,40,192,117,38,0,213,31,240,83,116,174,100,58,144,175,56,180,187,59,2},
  {118,105,251,142,173,206,119,201,47,240,166,60,39,156,155,111,147,129,253,132,244,244,191,40,212,235,110,70,241,225,145,42},
  {218,19,60,95,164,29,193,143,31,60,118,76,181,15,34,25,36,25,107,130,112,1,254,30,88,119,91,143,204,104,68,115},
  {32,9,88,219,42,178,51,219,92,36,92,6,17,122,205,3,143,164,248,172,108,77,113,31,45,213,215,21,152,78,171,51},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {119,220,130,248,172,121,136,58,112,22,32,242,0,215,240,240,73,194,107,33,173,48,122,143,203,15,1,98,169,138,17,96},
  {191,96,234,8,162,191,161,169,220,137,105,236,215,170,48,86,253,59,49,95,28,131,143,42,119,57,37,26,0,6,3,74},
  {79,44,236,112,159,54,150,95,117,110,182,61,177,18,158,99,107,124,142,159,114,211,46,83,130,42,252,119,143,232,229,84},
  {120,118,158,141,125,24,142,120,47,134,69,41,152,97,48,54,233,93,206,63,177,243,10,239,15,75,5,166,48,220,51,42},
  {154,52,76,173,8,179,82,209,123,62,55,159,184,42,203,79,85,136,37,33,206,15,34,229,172,130,233,168,80,218,61,45},
  {73,60,108,146,86,171,58,233,185,238,63,146,73,30,24,34,168,99,126,227,84,5,105,118,225,99,196,231,236,222,30,78},
  {98,24,253,129,142,215,166,88,61,78,76,204,63,148,128,154,172,46,214,22,232,19,159,246,156,88,25,209,232,64,143,103},
  {57,242,82,233,126,54,168,233,155,25,198,102,114,4,58,53,68,128,90,192,59,130,141,40,215,52,107,196,131,41,65,39},
  {123,70,222,176,120,109,101,245,75,98,220,83,5,255,243,76,191,226,15,90,209,192,192,236,110,38,48,104,108,237,145,40},
  {55,148,59,52,217,39,224,148,171,189,93,22,23,51,10,19,211,155,65,128,118,2,100,22,172,253,152,235,235,75,32,67},
  {249,135,87,93,181,36,185,185,29,41,248,205,125,88,68,213,249,15,243,25,75,40,228,78,236,220,85,85,118,111,169,86},
  {106,103,2,243,137,56,146,229,122,239,78,38,157,70,155,109,250,148,25,146,155,131,53,130,252,229,155,60,116,196,27,24},
  {56,169,74,148,128,81,129,189,149,193,170,248,98,241,253,84,138,97,74,51,6,228,232,110,164,73,149,25,108,184,144,118},
  {40,200,204,209,91,183,68,11,249,72,60,139,63,177,138,71,245,134,205,27,114,128,202,70,153,135,44,233,35,233,100,28},
  {41,180,99,22,85,111,172,206,203,210,25,146,24,17,12,58,24,38,145,39,250,10,156,182,230,244,68,97,12,26,173,78},
  {239,222,144,88,98,46,18,204,120,214,116,139,233,19,96,178,82,59,103,84,52,232,24,152,111,232,130,39,175,220,254,34},
  {126,137,129,238,107,156,145,31,56,159,155,66,4,83,86,183,9,220,155,48,244,177,187,3,182,38,36,137,95,249,128,25},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {33,212,245,239,163,87,64,222,199,45,84,224,94,162,129,34,49,21,39,11,84,129,120,16,201,2,126,109,67,19,48,21},
  {136,19,17,112,99,189,63,169,15,116,146,2,181,161,168,201,3,82,207,138,247,146,111,53,253,213,249,195,109,61,62,62},
  {214,96,131,178,122,236,247,212,60,83,213,17,200,164,52,19,131,159,193,252,76,52,94,134,135,233,55,169,243,136,23,107},
  {31,100,25,145,30,62,144,212,64,219,186,236,38,135,240,173,18,18,210,133,193,58,175,187,132,2,62,79,230,138,149,63},
  {146,49,171,76,227,245,47,149,115,6,126,210,49,141,165,156,142,7,128,23,25,254,255,223,153,207,248,237,162,250,198,67},
  {115,167,160,190,36,209,72,210,130,232,16,195,227,119,219,213,42,163,16,207,133,16,126,252,97,244,154,83,165,64,192,26},
  {152,12,190,190,80,17,187,19,71,36,226,40,125,198,215,94,232,63,42,31,14,100,72,70,107,235,184,199,209,44,150,91},
  {145,169,66,194,172,81,198,39,46,210,40,41,254,167,131,100,86,46,217,95,40,211,39,130,222,43,75,12,181,179,175,68},
  {133,21,44,67,64,117,20,116,128,232,253,115,65,157,23,187,236,174,41,220,111,27,126,207,11,138,96,96,151,211,168,49},
  {6,194,203,81,78,250,69,205,80,64,132,142,179,170,110,65,191,233,102,109,191,240,215,13,109,52,60,12,114,5,33,56},
  {245,179,140,16,163,32,44,87,11,245,40,56,66,162,19,245,118,155,111,15,189,36,221,51,162,104,129,154,194,248,202,68},
  {244,234,224,253,20,60,198,188,247,143,28,41,20,220,97,2,77,10,78,118,138,153,225,43,248,21,123,242,192,35,238,1},
  {31,224,135,97,215,173,1,8,159,103,168,115,190,1,26,137,189,20,155,104,150,122,79,28,21,220,74,33,159,177,72,76},
  {215,194,226,30,5,48,102,158,105,8,172,181,127,251,175,126,0,25,101,220,84,10,197,19,105,58,237,49,123,52,190,117},
  {131,178,101,184,126,23,38,56,138,103,253,111,188,251,228,91,254,228,187,90,26,225,156,197,66,28,186,57,44,185,250,23},
  {154,231,161,126,117,93,158,70,201,65,48,137,36,99,132,30,137,109,243,186,100,20,184,154,138,250,91,2,143,46,33,101},
  {231,158,68,235,147,216,171,15,157,107,171,70,180,155,200,39,184,120,149,1,0,152,227,33,46,131,214,172,224,37,32,111},
  {253,205,161,63,111,59,7,121,218,246,210,176,175,45,127,224,204,83,38,105,27,136,210,12,116,200,63,111,243,97,6,112},
  {5,46,30,42,98,63,154,70,66,63,49,231,188,171,247,29,219,150,92,232,106,211,78,164,181,2,112,69,113,43,80,32},
  {118,89,75,102,164,69,59,231,245,160,186,207,73,82,175,87,192,188,225,244,45,156,230,193,151,62,253,224,122,86,181,24},
  {115,208,88,150,38,171,228,174,111,26,37,143,232,112,75,79,110,236,116,118,226,134,16,45,74,163,167,66,94,182,154,7},
  {18,210,84,201,51,34,241,76,154,163,14,204,246,5,232,8,21,91,27,156,153,60,43,242,136,87,232,108,180,171,166,74},
  {197,233,94,100,214,120,245,139,186,201,108,21,34,160,141,46,28,199,221,166,87,96,122,5,69,213,139,52,244,140,249,113},
  {99,251,119,112,231,187,226,19,201,44,152,12,75,37,149,216,238,92,77,90,182,244,215,236,152,248,64,29,159,78,248,62},
  {158,138,9,51,170,145,115,189,112,13,48,3,98,41,174,199,232,105,74,126,173,133,160,77,89,232,220,120,220,157,180,65},
  {65,16,12,71,178,214,47,201,150,50,12,108,176,230,144,60,126,229,4,105,37,131,67,15,164,227,197,10,127,202,91,86},
  {156,161,169,73,10,135,164,15,62,197,29,189,29,233,29,213,248,37,51,240,20,220,225,237,118,6,251,192,160,51,249,13},
  {110,150,14,99,68,203,159,4,163,34,159,247,191,49,163,248,31,16,1,99,125,248,15,121,30,38,206,24,168,24,186,68},
  {8,189,66,216,234,113,36,241,5,26,29,3,178,167,139,138,18,138,176,138,255,185,236,227,217,63,49,202,131,61,120,24},
  {212,158,177,203,244,122,84,74,148,209,29,122,61,231,225,9,252,236,13,189,71,250,166,72,32,254,191,58,116,1,235,118},
  {24,51,215,230,138,23,30,88,143,60,86,238,203,211,116,63,245,76,50,7,178,47,174,153,162,153,90,26,244,64,247,126},
  {71,231,201,137,60,129,65,100,178,201,65,65,97,209,157,227,242,131,60,84,180,233,58,74,146,24,187,55,230,120,180,110},
  {172,70,14,185,30,186,50,208,224,176,120,187,8,3,251,227,44,1,74,205,248,162,76,83,200,48,254,14,251,56,149,66},
  {247,146,255,227,153,101,136,0,146,133,35,205,167,85,69,130,198,226,111,184,244,181,248,124,96,112,53,61,56,88,129,74},
  {206,78,210,8,102,50,143,143,31,56,203,224,32,190,137,2,149,174,115,118,219,32,131,246,43,49,186,161,111,98,144,46},
  {198,154,199,78,110,86,239,200,105,159,84,137,33,219,78,60,63,55,16,37,131,150,75,186,153,157,252,223,107,173,129,105},
  {249,150,54,25,81,32,193,186,113,23,74,213,26,222,126,40,36,69,191,63,53,207,74,7,146,7,223,114,136,152,60,1},
  {250,24,107,107,6,101,57,112,116,170,102,232,87,148,128,233,176,179,128,225,215,239,98,241,165,238,3,177,50,199,204,1},
  {170,208,209,190,88,225,4,146,80,242,232,41,199,19,207,109,186,249,155,73,43,228,161,96,105,32,252,159,150,203,35,3},
  {137,26,192,72,140,228,109,25,100,16,99,83,223,159,111,128,121,49,67,57,47,19,70,152,130,193,111,128,175,222,74,109},
  {230,192,51,223,88,76,165,65,245,172,169,217,135,30,175,238,107,118,156,229,115,148,251,35,247,181,153,141,64,209,111,48},
  {132,85,71,140,81,157,140,27,244,168,251,76,90,21,84,87,47,220,93,183,132,39,90,110,126,195,60,209,172,229,201,62},
  {30,225,230,237,233,238,81,121,182,15,229,62,119,131,26,184,91,255,252,212,184,172,61,70,71,129,95,179,166,196,248,54},
  {226,192,7,12,247,39,31,31,146,232,156,62,81,27,45,7,26,101,90,209,20,194,119,196,21,128,80,155,216,45,136,36},
  {237,55,206,156,153,98,47,4,113,19,141,79,130,147,182,227,67,233,30,11,14,216,24,50,201,173,251,94,125,59,10,23},
  {181,49,110,183,58,255,210,62,187,69,3,62,83,113,156,197,242,100,21,35,207,254,182,177,170,140,233,230,156,93,33,25},
  {155,73,0,92,215,234,237,137,180,16,181,128,83,187,187,194,169,65,21,107,112,232,230,143,118,49,50,87,125,129,170,102},
  {83,174,249,202,179,58,189,144,173,3,137,209,230,75,20,7,24,221,28,81,93,137,8,72,28,93,255,35,19,90,20,123},
  {65,62,154,115,224,88,6,117,184,185,242,211,66,135,27,228,207,140,55,113,135,116,214,21,101,100,88,232,37,123,40,23},
  {21,183,78,80,200,224,244,217,51,149,162,50,234,63,225,34,110,163,6,104,143,7,14,153,90,173,192,153,92,6,198,30},
  {37,4,115,109,112,38,49,17,65,41,208,227,162,7,235,158,167,241,174,152,183,52,193,184,73,164,47,221,169,24,150,56},
  {3,76,186,156,73,32,8,184,243,153,152,187,133,201,4,193,22,190,69,70,182,40,172,229,239,188,2,74,226,188,145,126},
  {107,34,228,2,211,131,135,174,55,26,22,11,123,154,234,171,193,20,22,138,117,172,170,186,145,69,38,36,18,145,190,49},
  {112,155,213,119,42,188,194,117,1,220,30,121,159,134,158,94,15,242,75,232,214,4,87,58,90,222,103,169,64,203,215,34},
  {101,8,45,230,27,51,148,195,75,127,203,167,78,137,135,248,54,28,30,246,64,138,219,86,134,110,189,60,9,77,225,121},
  {135,156,156,36,100,222,65,89,63,7,219,158,162,168,196,27,226,198,185,121,31,130,247,227,124,145,198,121,149,127,164,17},
  {135,15,120,138,71,87,173,231,237,56,243,27,120,254,187,195,198,200,174,75,255,10,148,111,61,165,125,117,227,5,164,102},
  {50,4,225,52,71,88,187,126,109,97,237,180,110,70,84,4,166,163,176,38,217,6,62,76,183,202,153,49,93,93,43,101},
  {90,21,239,238,78,202,71,98,221,74,25,14,52,104,86,203,129,112,143,228,68,7,250,19,186,252,225,53,99,87,39,95},
  {189,43,15,184,94,255,242,174,153,145,193,220,227,67,4,174,147,174,152,18,101,159,166,139,63,163,243,59,213,161,253,1},
  {43,41,186,92,20,48,245,52,112,164,231,7,30,244,98,207,255,80,92,47,122,161,158,99,4,86,10,246,88,141,211,64},
  {164,18,181,163,154,223,13,222,144,40,229,62,74,82,118,81,14,32,25,1,113,163,252,237,51,70,56,223,247,160,78,82},
  {102,229,61,162,248,69,239,179,58,60,8,152,113,12,218,168,141,35,147,32,194,194,98,22,156,1,83,20,8,13,65,122},
  {129,123,252,32,79,232,223,63,208,174,46,91,43,179,128,194,105,151,157,192,205,242,81,44,200,18,14,15,210,144,109,87},
  {159,55,188,83,179,251,171,231,152,89,242,9,193,181,236,12,223,112,236,40,247,85,159,25,85,107,225,18,155,177,61,7},
  {190,145,102,90,116,173,9,96,1,136,94,70,240,65,189,201,171,145,254,247,129,62,21,105,243,190,11,68,201,228,8,12},
  {231,135,174,215,156,67,163,120,76,181,241,178,10,247,172,245,31,239,228,196,174,195,118,22,142,51,13,168,31,199,62,43},
  {73,146,214,103,234,183,216,144,200,165,221,87,194,213,49,122,62,65,255,204,72,182,131,146,75,41,207,45,248,160,66,90},
  {218,151,19,123,28,234,235,200,163,179,10,170,252,43,86,73,85,71,114,100,167,146,135,33,146,246,108,40,7,137,39,115},
  {239,148,237,0,202,18,221,76,204,192,173,223,64,192,155,240,126,69,40,188,148,63,98,179,153,171,127,132,205,146,38,118},
  {114,178,3,172,20,190,19,181,77,121,28,50,216,32,39,195,172,140,113,226,234,6,168,216,230,208,249,107,66,64,233,60},
  {14,53,226,212,72,54,37,122,49,73,1,118,243,185,162,74,34,107,209,235,157,118,141,39,113,86,137,20,138,74,1,98},
  {11,170,79,238,36,206,68,38,246,56,104,176,194,101,73,154,107,241,159,210,25,51,189,254,44,37,137,100,143,9,146,80},
  {178,130,114,238,38,16,52,113,3,101,188,106,231,131,176,203,199,120,61,248,168,88,88,57,200,132,94,21,26,110,252,103},
  {215,155,105,172,214,147,237,205,119,60,252,47,67,96,235,114,110,201,226,162,219,48,127,194,252,165,209,248,235,150,64,23},
  {166,231,42,36,82,248,104,43,82,112,6,217,2,173,23,178,47,58,82,139,81,21,252,83,220,4,25,236,19,233,29,40},
  {0,180,224,147,32,164,89,230,151,104,186,154,99,51,136,73,196,133,201,147,116,52,167,136,145,196,105,175,165,34,28,108},
  {242,84,128,149,205,214,49,57,162,189,47,96,211,217,144,69,45,198,30,208,20,93,90,160,20,186,93,252,217,14,232,42},
  {110,203,38,47,155,200,34,223,176,221,103,84,230,144,95,250,34,28,202,120,170,176,202,218,74,129,95,249,77,229,72,123},
  {236,248,147,230,0,12,122,127,81,208,149,0,97,187,213,177,109,216,52,215,168,107,82,70,144,31,173,149,212,176,141,121},
  {129,54,127,113,23,213,170,23,125,175,158,90,213,181,240,71,94,221,10,124,254,181,32,136,220,100,246,33,142,108,253,122},
  {207,99,98,54,244,37,197,214,156,117,144,114,91,66,109,51,150,66,7,58,125,133,76,166,247,62,34,75,174,82,117,119},
  {121,50,129,4,175,130,113,47,11,231,181,146,108,87,42,154,202,225,14,245,94,177,3,144,235,171,44,206,253,188,50,63},
  {176,182,52,252,134,77,176,235,75,203,19,14,204,192,20,85,158,135,211,17,217,16,231,54,41,73,208,31,27,201,231,67},
  {43,186,50,61,113,54,20,246,129,62,252,230,142,137,188,131,52,135,179,98,232,100,109,104,47,120,116,195,104,53,219,76},
  {42,70,245,57,92,204,31,170,214,134,23,18,203,54,54,166,42,86,52,54,83,242,185,188,38,39,121,150,204,77,186,9},
  {252,153,120,176,103,212,75,243,94,253,126,19,145,198,184,151,142,97,5,167,143,143,166,67,50,196,36,175,109,172,119,2},
  {161,11,162,100,6,245,172,159,215,191,146,107,32,227,42,247,252,245,246,10,78,80,109,130,135,23,147,91,19,93,171,113},
  {85,22,94,43,250,25,107,211,127,176,94,120,179,15,69,112,208,255,149,239,112,175,29,76,194,87,205,12,119,143,224,112},
  {176,88,213,41,206,76,234,184,116,73,157,208,92,216,24,119,57,36,66,147,8,160,21,98,210,253,8,43,48,167,53,48},
  {62,0,123,251,99,62,66,71,243,231,164,120,115,181,126,254,237,36,36,32,140,194,68,35,73,35,222,199,199,107,28,117},
  {70,120,164,208,137,21,121,162,170,232,226,147,29,219,46,113,250,108,94,182,81,170,242,154,139,170,231,136,62,149,152,109},
  {14,237,34,146,132,166,157,178,67,25,254,152,153,185,63,240,88,85,182,87,39,199,1,173,180,1,24,254,13,167,52,122},
  {242,199,150,29,101,160,96,249,71,189,152,244,2,107,249,84,53,111,96,147,59,52,94,253,64,24,115,111,7,201,175,122},
  {1,178,251,232,38,255,44,82,173,188,52,91,62,83,168,38,95,75,72,128,133,212,62,143,110,112,121,149,220,105,4,85},
  {175,8,94,200,26,183,77,83,31,204,57,39,124,104,223,137,28,239,203,131,34,179,127,231,231,26,223,14,36,198,12,102},
  {79,239,164,209,27,133,140,233,228,18,135,107,160,242,47,192,248,192,216,54,217,167,41,134,14,151,241,20,44,45,65,112},
  {46,130,121,139,15,200,12,86,200,15,247,172,80,99,45,114,180,28,227,23,203,233,211,8,246,205,217,147,171,99,219,57},
  {92,164,40,99,40,192,66,62,77,160,155,220,53,40,184,13,30,33,3,251,48,94,67,38,33,104,167,232,228,171,151,84},
  {154,94,142,191,27,252,192,84,123,41,33,19,218,35,128,221,242,122,223,205,179,50,60,118,31,32,16,164,45,185,232,80},
  {162,200,159,138,45,13,115,138,144,161,63,6,91,72,118,169,6,181,210,18,111,174,62,33,31,232,84,171,40,69,200,85},
  {134,117,13,133,76,76,11,104,240,246,53,69,95,54,199,222,80,75,6,41,61,6,42,151,152,238,83,104,84,68,82,56},
  {241,134,87,215,83,178,102,73,233,53,211,88,184,115,74,195,209,8,146,155,112,251,184,47,221,243,207,27,132,10,132,103},
  {226,229,86,250,226,103,61,86,101,102,170,10,63,9,246,69,117,118,210,10,220,237,241,170,242,18,116,58,91,167,107,110},
  {84,254,45,150,154,98,111,211,23,245,171,54,22,154,176,232,226,22,118,224,202,10,81,191,197,247,21,58,221,110,151,50},
  {143,113,135,245,250,124,30,154,148,22,222,2,117,102,37,93,250,232,201,25,74,171,107,245,58,131,36,66,114,107,208,2},
  {1,109,129,242,119,182,55,3,153,159,123,44,185,57,21,206,56,39,169,140,95,249,234,0,69,12,113,108,191,23,1,96},
  {192,244,164,145,98,244,9,246,197,103,1,157,51,121,12,217,237,163,46,47,31,207,34,115,161,241,242,8,216,217,12,64},
  {234,38,16,122,131,105,113,8,184,145,137,86,13,79,72,188,44,16,207,203,179,59,194,14,41,217,244,168,4,6,88,72},
  {47,115,8,251,186,185,171,39,249,40,33,1,18,101,92,219,88,129,3,177,237,246,163,93,167,149,3,184,116,88,241,25},
  {21,84,118,159,59,245,230,151,180,211,234,186,152,140,135,211,131,219,69,203,17,169,193,235,216,127,223,102,98,1,191,26},
  {19,91,35,55,26,16,58,150,14,11,211,85,201,223,90,113,107,149,54,13,225,66,8,38,250,101,202,37,218,226,152,75},
  {173,238,48,49,7,88,140,189,222,52,38,76,94,246,163,179,79,68,199,174,175,35,28,171,166,228,36,118,201,58,109,61},
  {235,14,110,53,191,65,207,130,47,77,134,173,120,240,222,139,29,180,151,72,162,218,70,217,73,77,253,120,147,236,10,61},
  {57,74,239,155,208,129,43,123,158,71,189,175,103,55,72,239,237,109,15,72,56,141,97,161,180,151,109,117,42,250,184,118},
  {52,64,63,32,59,126,186,249,244,99,54,83,22,9,37,16,48,126,99,33,177,126,99,80,233,21,61,226,163,249,55,32},
  {203,110,3,139,59,73,180,116,211,247,111,59,160,56,91,186,33,143,92,198,148,162,24,120,130,109,3,10,231,16,77,30},
  {255,82,248,88,61,49,168,220,29,130,1,110,152,243,156,136,217,219,236,92,185,217,90,81,253,170,11,27,2,238,82,54},
  {60,22,33,140,23,162,113,107,155,140,224,219,58,96,175,108,53,128,62,69,108,113,149,118,205,53,88,43,42,31,142,55},
  {204,187,162,207,214,53,94,210,29,108,25,4,213,123,82,215,173,223,116,44,147,220,85,208,25,20,241,20,100,97,223,41},
  {251,98,204,212,151,230,92,115,12,253,83,80,49,51,202,230,25,35,78,198,170,9,74,228,125,247,101,252,40,101,248,120},
  {5,168,106,188,73,12,34,187,203,246,72,18,25,207,249,218,205,93,206,60,174,15,128,30,117,208,119,125,204,0,69,104},
  {16,176,71,29,5,94,145,54,10,27,149,62,98,110,49,207,187,136,60,70,108,197,164,236,231,202,78,68,116,44,114,12},
  {118,218,149,241,214,33,125,242,130,22,147,72,83,220,163,24,130,172,18,66,72,206,244,12,146,34,20,156,123,174,125,122},
  {146,43,123,255,31,155,213,85,188,25,221,200,248,13,245,68,170,167,163,189,220,88,41,78,249,84,138,234,158,186,176,75},
  {154,252,107,191,196,167,167,136,136,98,93,165,209,41,6,157,130,51,6,68,53,89,39,163,170,39,99,132,237,74,159,57},
  {232,223,29,50,126,217,13,125,105,77,176,189,58,93,172,9,98,165,81,119,248,183,137,75,104,255,23,182,247,162,99,8},
  {116,70,20,190,1,228,58,162,97,230,38,210,46,203,75,121,38,47,150,168,111,194,217,74,139,93,163,63,251,59,118,43},
  {126,55,220,151,180,27,202,117,159,44,8,122,63,39,156,203,142,192,247,237,107,246,70,7,213,252,99,166,210,52,191,97},
  {184,122,157,192,154,231,105,41,94,69,37,230,209,206,80,42,230,92,79,235,72,0,173,193,149,93,81,159,3,222,209,67},
  {180,167,250,223,182,154,131,236,92,247,193,57,146,215,254,245,39,26,86,84,160,44,80,79,10,137,193,48,226,150,244,84},
  {60,192,58,28,87,176,201,167,97,147,222,140,8,28,23,184,137,118,108,190,240,203,59,10,188,50,7,248,244,70,246,85},
  {237,233,209,128,35,74,98,191,219,14,142,247,21,184,48,218,12,66,235,142,38,201,74,115,203,99,197,195,181,145,64,18},
  {130,188,214,217,3,161,54,133,153,23,231,239,204,250,189,175,80,94,207,73,155,208,50,219,223,133,155,189,102,74,86,113},
  {227,14,91,89,149,235,87,47,99,114,144,161,63,49,199,206,166,102,67,148,111,155,190,231,26,97,66,200,152,157,97,106},
  {242,89,152,96,42,56,87,23,28,237,153,236,48,53,216,130,41,246,187,228,42,0,105,229,19,247,196,116,110,188,161,106},
  {14,219,163,120,245,151,25,166,50,224,152,7,225,58,30,88,16,130,127,229,43,47,74,108,254,13,74,201,106,96,143,120},
  {87,236,127,198,169,148,118,83,58,50,75,21,133,21,65,45,14,189,87,81,215,96,59,78,219,142,162,18,59,85,83,99},
  {224,179,222,89,255,48,52,179,140,224,236,20,136,156,186,137,2,157,150,223,155,241,245,156,99,211,82,13,29,52,168,81},
  {64,91,102,139,101,206,222,171,228,8,235,102,170,68,4,28,147,133,171,48,83,187,216,119,107,20,43,2,254,116,218,94},
  {96,152,132,105,44,180,126,28,69,184,225,234,81,248,99,233,148,25,242,161,80,3,51,87,86,219,28,141,82,225,91,18},
  {245,147,197,14,116,81,238,160,86,55,195,142,129,205,120,245,176,155,135,184,139,85,206,241,38,229,120,82,130,101,111,12},
  {9,178,206,181,142,48,124,46,179,156,48,251,55,239,115,5,121,70,93,197,129,97,161,207,155,176,26,201,43,111,39,99},
  {147,241,120,113,224,227,19,27,252,128,102,181,229,207,33,112,228,183,166,22,96,243,43,175,115,109,22,76,240,144,232,7},
  {115,133,173,27,62,250,88,134,96,111,195,204,235,48,166,87,167,250,168,137,187,92,210,236,239,190,20,127,191,201,204,91},
  {26,72,40,11,41,19,139,5,91,221,38,250,195,246,25,132,206,72,32,94,158,183,73,185,132,155,47,231,237,107,22,113},
  {119,243,169,67,101,142,55,101,19,229,2,251,162,150,18,160,112,134,178,193,7,225,98,208,156,230,253,183,65,134,234,30},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {139,147,141,94,93,143,239,121,211,81,7,154,161,209,7,107,178,61,20,223,137,117,220,32,14,123,164,122,20,46,70,31},
  {0,8,245,27,88,69,231,240,113,253,194,181,87,25,252,185,40,105,46,159,147,94,139,142,131,39,53,75,16,206,222,113},
  {4,243,251,109,137,210,96,75,126,55,164,203,108,60,142,184,22,115,198,64,107,67,90,220,241,226,200,57,51,125,193,103},
  {8,178,89,230,199,191,250,150,143,137,166,130,170,245,171,169,133,222,28,78,230,198,190,1,49,175,201,250,128,82,252,47},
  {246,114,119,143,155,39,157,246,34,55,191,127,26,58,19,46,165,222,14,207,65,129,225,39,13,216,33,128,100,154,200,93},
  {204,122,49,114,58,61,115,158,17,228,115,102,218,190,240,197,111,177,250,75,24,247,17,176,143,83,147,166,57,45,102,23},
  {244,146,37,213,178,53,228,12,102,15,156,100,18,101,235,155,107,127,117,214,75,172,94,175,202,9,102,73,140,176,124,5},
  {148,188,32,138,35,155,131,145,247,84,194,147,26,230,196,137,182,79,238,150,206,142,38,214,146,1,127,88,47,63,152,67},
  {209,8,167,238,183,163,11,201,243,185,121,161,29,123,20,175,222,28,200,142,145,47,77,106,203,64,70,86,163,201,45,53},
  {98,163,196,68,252,99,1,247,236,37,88,104,24,167,95,254,22,154,226,52,102,227,151,125,253,130,214,218,119,248,75,58},
  {131,184,40,236,201,25,226,15,254,203,109,30,23,96,252,240,196,182,159,89,115,37,70,250,109,160,42,78,55,155,232,74},
  {128,200,33,171,61,181,149,40,157,81,87,232,179,117,143,223,16,178,66,151,121,165,153,244,21,43,97,130,164,152,211,35},
  {223,75,13,226,76,177,189,93,51,68,131,194,183,178,154,171,211,179,5,128,98,214,70,79,95,15,37,250,248,116,70,127},
  {126,246,89,95,225,129,218,133,97,109,54,10,221,243,180,97,9,73,61,62,229,102,18,182,42,111,35,14,215,178,188,30},
  {218,32,254,212,8,74,75,59,35,10,147,245,81,16,116,191,60,217,50,26,181,201,162,223,252,193,5,7,161,94,137,78},
  {18,53,101,1,161,55,109,55,55,245,40,232,72,51,227,62,45,172,78,1,35,35,96,219,227,194,151,255,96,226,166,98},
  {242,86,244,104,119,137,72,40,118,67,216,30,195,131,6,158,122,176,90,117,93,74,71,235,10,59,122,174,197,2,226,59},
  {104,184,73,21,234,140,146,89,66,128,93,207,104,48,232,88,217,71,111,230,170,67,145,116,165,245,23,47,124,139,236,77},
  {104,184,73,21,234,140,146,89,66,128,93,207,104,48,232,88,217,71,111,230,170,67,145,116,165,245,23,47,124,139,236,77},
  {104,184,73,21,234,140,146,89,66,128,93,207,104,48,232,88,217,71,111,230,170,67,145,116,165,245,23,47,124,139,236,77},
  {104,184,73,21,234,140,146,89,66,128,93,207,104,48,232,88,217,71,111,230,170,67,145,116,165,245,23,47,124,139,236,77},
  {104,184,73,21,234,140,146,89,66,128,93,207,104,48,232,88,217,71,111,230,170,67,145,116,165,245,23,47,124,139,236,77},
  {104,184,73,21,234,140,146,89,66,128,93,207,104,48,232,88,217,71,111,230,170,67,145,116,165,245,23,47,124,139,236,77},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {174,55,84,177,14,127,76,178,164,64,128,169,117,204,11,247,182,187,200,91,189,166,41,125,251,5,34,240,60,43,170,99},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {231,139,75,87,251,198,105,248,16,76,143,20,195,153,152,96,240,141,255,220,107,212,231,155,20,32,248,139,96,147,226,70},
  {209,108,157,87,217,185,241,67,140,38,65,108,77,247,109,169,226,0,168,35,246,109,220,135,77,138,106,158,243,48,87,40},
  {209,108,157,87,217,185,241,67,140,38,65,108,77,247,109,169,226,0,168,35,246,109,220,135,77,138,106,158,243,48,87,40},
  {209,108,157,87,217,185,241,67,140,38,65,108,77,247,109,169,226,0,168,35,246,109,220,135,77,138,106,158,243,48,87,40},
  {209,108,157,87,217,185,241,67,140,38,65,108,77,247,109,169,226,0,168,35,246,109,220,135,77,138,106,158,243,48,87,40},
  {209,108,157,87,217,185,241,67,140,38,65,108,77,247,109,169,226,0,168,35,246,109,220,135,77,138,106,158,243,48,87,40},
  {209,108,157,87,217,185,241,67,140,38,65,108,77,247,109,169,226,0,168,35,246,109,220,135,77,138,106,158,243,48,87,40},
  {65,15,235,90,165,23,55,202,122,21,115,134,135,213,129,92,121,19,17,181,54,134,148,118,57,194,144,32,68,24,130,40},
  {65,15,235,90,165,23,55,202,122,21,115,134,135,213,129,92,121,19,17,181,54,134,148,118,57,194,144,32,68,24,130,40},
  {65,15,235,90,165,23,55,202,122,21,115,134,135,213,129,92,121,19,17,181,54,134,148,118,57,194,144,32,68,24,130,40},
  {65,15,235,90,165,23,55,202,122,21,115,134,135,213,129,92,121,19,17,181,54,134,148,118,57,194,144,32,68,24,130,40},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {109,138,207,247,64,117,107,238,187,28,24,80,125,89,55,137,19,82,117,58,74,123,10,248,234,235,140,156,99,60,63,64},
  {2,4,227,41,22,217,16,242,153,36,17,149,21,122,112,241,172,35,16,124,233,123,240,67,172,231,103,196,30,180,63,123},
  {2,4,227,41,22,217,16,242,153,36,17,149,21,122,112,241,172,35,16,124,233,123,240,67,172,231,103,196,30,180,63,123},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {136,163,126,187,77,21,125,112,229,232,232,195,194,74,226,127,55,149,208,19,158,134,22,89,225,31,185,184,35,162,25,91},
  {4,60,159,148,140,4,91,54,247,97,13,166,37,139,55,207,203,171,12,83,188,81,88,119,110,36,233,70,202,224,137,84},
  {4,60,159,148,140,4,91,54,247,97,13,166,37,139,55,207,203,171,12,83,188,81,88,119,110,36,233,70,202,224,137,84},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
  {208,178,249,55,16,189,207,176,231,55,81,212,146,150,15,170,230,58,26,147,125,3,171,100,93,198,10,185,34,241,199,78},
} ;

static const unsigned char precomputed_nP_montgomery25519_n[precomputed_nP_montgomery25519_NUM][crypto_nP_SCALARBYTES] = {
  {22,165,108,86,5,154,20,29,152,42,38,184,15,197,6,22,9,228,189,48,3,19,228,73,57,251,178,27,117,180,121,83},
  {139,146,167,138,83,58,254,190,251,60,110,156,52,211,217,116,28,175,54,171,72,205,88,250,162,123,91,36,201,104,201,225},
  {40,99,197,149,12,76,46,216,65,163,184,95,88,169,52,102,10,143,162,20,164,177,134,75,36,187,91,135,65,113,164,111},
  {249,136,238,59,164,86,241,133,205,15,12,226,199,113,49,1,247,239,150,87,31,12,54,21,210,187,99,46,64,166,182,166},
  {45,15,68,93,105,161,236,142,132,222,64,65,61,38,44,243,32,161,134,170,84,232,175,213,242,206,236,93,30,166,38,12},
  {66,162,214,17,77,227,115,67,248,216,210,73,216,96,172,93,126,63,38,233,228,89,232,80,113,127,88,112,85,247,241,224},
  {176,167,216,65,10,75,148,78,76,151,92,161,168,237,252,14,170,191,18,24,113,151,192,59,92,204,29,110,25,193,162,76},
  {190,147,100,101,77,151,179,249,20,102,182,218,161,187,38,207,142,126,195,32,205,249,96,90,162,80,63,226,98,120,239,127},
  {147,215,107,156,104,176,12,226,26,128,179,131,210,202,169,59,227,81,62,36,35,82,151,139,126,209,74,57,26,207,219,102},
  {132,175,87,38,2,127,24,123,10,20,230,220,70,101,14,168,26,154,32,108,78,93,106,112,2,147,7,146,149,82,157,35},
  {193,95,146,115,121,195,161,67,226,160,12,126,178,128,86,146,245,113,187,78,36,100,116,199,49,41,122,79,39,152,203,40},
  {236,45,18,11,37,163,223,201,31,238,132,192,1,225,170,91,70,220,130,111,139,208,93,86,211,22,147,29,9,165,75,251},
  {214,148,177,62,22,252,2,219,156,203,249,78,135,68,219,13,25,181,42,250,161,197,21,221,181,140,175,242,30,162,164,254},
  {19,146,48,164,180,234,202,31,98,230,250,183,198,75,6,104,144,119,247,32,46,71,235,138,229,32,112,138,58,243,13,152},
  {40,60,147,152,199,95,6,38,58,248,50,106,237,28,163,224,234,179,168,129,118,218,213,122,233,59,115,10,208,154,53,139},
  {2,222,22,131,160,78,116,188,224,216,251,122,243,253,84,123,180,252,82,233,141,218,42,150,213,250,32,17,174,233,198,144},
  {128,119,130,105,236,245,3,235,114,211,107,56,5,89,175,50,3,46,15,154,32,145,30,229,124,32,44,50,136,214,58,70},
  {76,10,122,92,95,60,108,66,87,202,172,123,231,241,121,153,60,13,209,184,200,16,84,74,62,81,89,106,177,149,201,7},
  {145,52,243,43,154,181,28,84,69,217,127,105,124,26,144,86,194,239,44,38,126,15,220,245,35,24,159,43,124,97,207,182},
  {146,211,172,10,196,116,198,246,17,89,234,39,110,19,252,7,180,188,59,196,241,55,208,72,179,169,75,88,153,141,37,3},
  {212,156,146,156,127,229,232,48,160,29,227,173,107,76,56,36,21,111,10,90,192,138,249,203,182,138,92,200,128,190,180,32},
  {238,205,118,104,224,32,1,124,103,246,230,131,211,47,8,131,118,55,85,122,152,0,26,90,147,81,190,53,208,236,202,243},
  {184,255,235,31,96,215,85,15,110,246,97,134,39,210,12,132,239,67,247,243,236,62,204,71,15,198,95,132,56,102,223,48},
  {176,21,36,69,234,239,12,133,189,95,127,247,243,129,176,233,252,210,8,4,62,179,45,71,132,151,131,142,107,226,149,82},
  {197,131,147,67,59,184,171,62,161,63,145,185,219,190,36,179,208,35,161,239,97,250,147,78,155,122,131,84,222,20,255,57},
  {220,255,225,12,119,108,43,211,239,127,106,206,117,125,148,215,84,165,157,177,185,39,226,94,196,90,235,111,149,251,106,228},
  {1,28,90,119,160,181,243,45,18,11,199,48,127,45,242,31,193,80,42,66,154,199,187,183,101,84,43,245,158,152,45,188},
  {128,211,202,5,133,44,165,24,11,106,41,213,148,126,225,3,196,79,227,38,11,161,243,167,11,73,135,245,143,32,105,241},
  {105,78,213,136,250,153,54,196,78,209,104,98,206,228,99,111,100,117,226,107,188,69,131,53,250,81,161,57,154,75,186,86},
  {103,233,237,136,26,9,199,182,184,122,105,253,11,251,200,159,239,173,81,83,253,65,193,123,246,147,153,39,103,241,131,206},
  {88,208,32,95,60,24,182,214,242,35,2,43,181,126,241,232,103,169,202,71,248,100,223,15,81,170,105,233,181,20,139,143},
  {251,163,38,67,159,49,110,10,31,29,115,131,183,15,141,31,33,127,239,142,94,23,75,0,189,146,169,229,4,13,46,20},
  {99,8,10,186,138,69,61,254,102,60,148,169,218,139,116,20,156,161,166,38,77,146,150,224,159,120,161,205,114,242,117,201},
  {173,229,74,44,190,251,221,217,123,144,158,53,116,65,212,153,140,78,191,166,177,29,200,208,116,177,250,141,151,172,255,18},
  {86,113,216,237,144,237,27,77,43,86,221,218,78,165,92,68,2,121,158,174,144,41,197,225,133,57,198,150,193,205,31,80},
  {253,78,2,197,246,2,80,67,175,244,59,34,195,228,158,234,134,91,114,225,55,146,120,105,181,188,229,205,129,46,176,76},
  {100,157,37,78,181,39,219,183,230,127,78,28,20,217,129,180,250,127,193,234,24,210,28,32,124,140,0,239,47,98,237,54},
  {118,77,87,240,2,224,102,178,26,241,208,238,205,43,49,10,248,202,177,174,221,204,149,132,117,236,242,182,0,197,71,99},
  {65,15,32,20,53,133,12,74,45,16,232,61,190,241,108,142,219,107,165,43,166,77,224,0,143,238,111,108,78,188,83,189},
  {148,190,255,45,219,178,118,248,201,115,191,247,159,237,100,82,206,92,216,227,195,56,214,22,93,175,182,124,241,116,21,197},
  {195,90,245,191,118,127,212,249,167,243,171,178,54,241,180,206,227,159,217,21,10,140,196,140,68,80,151,15,68,176,89,190},
  {187,110,3,156,243,58,23,69,63,43,126,198,25,232,244,117,90,198,194,216,156,226,210,73,183,27,14,37,47,227,115,7},
  {83,201,187,156,49,162,122,98,244,64,232,65,97,99,249,20,112,35,197,48,214,75,215,12,58,113,197,148,113,214,183,52},
  {216,225,212,157,91,85,59,235,154,67,45,114,168,180,42,127,197,112,107,19,206,209,121,58,84,128,9,85,201,218,145,12},
  {213,147,100,238,218,244,221,153,119,214,125,130,158,51,183,19,100,226,158,224,124,214,84,217,214,5,100,138,151,82,47,113},
  {162,66,132,16,45,177,133,232,174,4,192,131,107,32,178,62,173,98,230,51,26,75,146,57,50,234,195,185,78,252,166,122},
  {108,47,222,232,210,191,39,56,170,87,9,43,52,219,179,129,140,92,177,75,218,9,219,168,101,130,70,60,50,158,31,182},
  {46,81,149,149,39,21,251,100,137,131,130,206,145,167,225,73,16,83,211,60,37,120,156,59,54,63,12,239,157,17,84,250},
  {73,75,142,5,29,149,53,179,200,42,206,120,189,172,169,234,155,245,32,16,189,242,246,199,228,216,79,193,21,149,73,11},
  {77,142,218,116,40,190,158,142,14,25,103,22,220,112,238,115,106,208,60,113,132,11,139,211,134,73,184,101,247,197,94,92},
  {30,227,78,104,84,81,163,164,213,29,187,243,170,68,117,193,9,81,235,137,157,206,4,218,77,29,188,210,29,124,49,251},
  {142,225,151,172,17,225,127,68,154,120,49,210,16,84,196,235,13,86,29,77,208,152,52,147,1,64,149,163,99,12,19,231},
  {212,34,94,235,200,193,0,184,101,187,23,99,253,170,8,165,176,93,221,119,178,52,105,186,227,109,125,247,157,129,161,28},
  {66,185,1,82,139,99,252,182,220,250,204,91,12,232,235,41,29,21,190,254,183,189,74,31,4,12,67,189,37,68,172,221},
  {29,139,208,144,158,44,163,1,154,204,136,129,249,124,170,119,165,192,236,183,170,69,227,92,93,168,183,102,236,70,132,35},
  {201,38,160,195,92,79,81,7,153,46,160,11,225,87,6,177,229,123,232,181,40,91,68,106,252,81,130,223,73,157,105,86},
  {197,251,119,136,36,212,2,98,147,53,96,203,69,110,178,248,42,129,249,180,77,223,221,240,247,101,119,199,240,147,174,59},
  {117,58,35,111,63,75,58,219,24,188,41,38,213,60,56,25,107,40,249,28,25,194,92,106,2,201,194,172,55,240,143,88},
  {76,75,16,137,188,138,139,186,248,43,22,143,182,126,58,197,103,73,165,66,85,12,234,199,46,110,80,174,76,163,47,59},
  {53,245,55,25,59,19,52,104,12,190,154,155,254,60,211,10,173,59,255,119,78,185,214,240,80,140,246,173,40,214,178,31},
  {95,123,103,87,5,178,10,135,206,161,95,233,75,102,103,198,205,232,37,55,209,52,254,26,201,114,42,28,28,52,229,250},
  {212,103,173,234,110,130,170,202,238,170,113,83,237,70,174,165,128,137,208,112,166,62,58,119,136,16,92,2,42,246,113,235},
  {37,37,206,228,45,61,199,89,10,17,7,249,133,9,82,105,39,44,59,174,252,120,185,49,25,36,174,51,102,101,30,42},
  {23,50,2,94,118,21,204,94,239,46,9,3,10,238,121,200,247,28,126,236,204,216,61,30,175,166,142,170,88,128,44,182},
  {213,168,128,89,134,56,190,212,254,149,134,166,70,229,99,211,215,36,27,134,65,78,24,151,38,28,67,38,44,67,165,213},
  {179,97,78,242,114,30,10,144,56,149,221,128,190,241,18,143,147,199,4,189,121,234,72,175,18,209,41,0,95,222,34,58},
  {13,74,52,214,90,233,227,224,221,209,189,31,98,134,79,12,20,205,239,229,202,59,161,241,208,66,161,8,230,48,104,231},
  {199,15,246,104,121,68,176,114,148,63,133,234,196,214,43,48,37,221,32,76,238,144,186,125,120,124,75,164,245,223,232,123},
  {242,0,246,18,121,63,165,228,66,126,192,196,235,199,37,199,3,168,187,101,2,146,35,226,198,233,122,105,81,139,29,82},
  {172,34,177,175,150,29,34,180,200,84,157,182,242,243,43,39,147,247,92,183,210,61,20,129,117,211,27,208,5,235,153,176},
  {159,227,167,216,244,16,147,38,173,141,121,182,83,21,236,80,132,102,92,154,166,71,149,127,117,125,4,24,33,63,142,189},
  {175,143,152,210,222,255,216,27,191,206,9,31,49,72,30,207,225,250,90,47,96,110,131,145,55,57,113,118,107,27,83,107},
  {107,186,160,142,195,202,118,52,209,113,10,53,105,99,156,228,1,98,117,197,252,194,43,191,179,77,159,99,106,32,14,129},
  {237,61,58,130,13,85,224,251,35,94,212,78,203,235,201,20,34,129,69,102,214,79,138,98,123,157,67,82,14,64,80,174},
  {182,75,243,148,4,128,244,221,155,58,207,252,40,150,126,7,167,21,179,216,143,122,136,187,36,11,217,148,104,183,162,160},
  {243,198,119,37,239,8,52,57,150,12,13,205,198,171,130,161,99,22,239,72,48,151,65,200,41,169,247,63,119,148,130,170},
  {59,221,34,203,249,204,78,247,89,11,106,78,176,234,184,64,48,248,19,152,166,49,37,109,200,188,112,187,224,180,92,175},
  {196,142,88,86,120,128,248,216,203,143,30,145,212,88,8,226,44,209,237,51,95,0,37,209,222,37,184,14,51,238,143,113},
  {246,50,136,243,85,168,3,233,189,68,221,90,81,189,140,213,160,226,230,85,65,61,248,106,74,191,103,228,116,69,59,59},
  {72,231,186,167,16,158,213,224,231,195,162,8,119,71,123,14,249,87,11,30,84,163,123,174,230,11,109,83,61,178,182,15},
  {196,114,61,3,194,185,195,49,128,208,169,247,44,24,202,182,89,234,161,242,149,34,52,51,148,178,138,2,195,29,56,251},
  {232,254,230,246,132,207,176,119,157,251,89,221,134,63,243,78,83,175,53,239,86,105,93,51,88,4,173,177,32,124,144,193},
  {31,60,154,239,1,99,22,215,237,93,143,96,188,253,160,127,211,130,94,5,225,196,37,124,3,57,3,71,4,241,231,15},
  {232,59,52,197,225,94,168,91,231,73,162,9,196,84,83,9,115,91,228,28,110,247,129,120,199,247,248,114,2,161,167,146},
  {66,171,64,207,81,72,227,218,155,228,231,169,174,118,47,75,92,9,36,200,119,200,153,240,214,245,84,125,0,11,90,137},
  {251,203,92,151,139,124,57,59,173,109,164,85,93,77,202,88,44,76,10,57,85,66,182,43,163,115,122,239,140,82,226,13},
  {37,177,125,17,181,23,36,116,112,172,138,193,110,33,2,198,112,237,96,15,244,166,4,191,129,237,85,244,100,105,65,83},
  {160,194,66,170,188,109,55,145,134,247,234,198,159,74,206,216,28,240,81,111,79,54,182,197,178,15,33,34,44,190,194,210},
  {88,64,1,35,44,133,219,142,181,211,0,49,193,178,129,99,210,8,68,62,157,47,152,209,255,43,234,157,47,157,220,69},
  {198,130,248,4,101,0,28,58,140,67,122,32,79,157,187,169,88,64,24,185,85,96,94,28,162,26,203,99,157,170,13,224},
  {138,118,50,4,97,30,223,67,133,227,178,230,148,132,25,72,174,35,177,140,212,86,244,251,114,81,166,39,66,105,57,204},
  {69,229,130,138,60,156,82,143,20,25,212,170,17,117,206,159,105,196,250,105,245,227,20,236,205,129,92,190,32,22,203,57},
  {143,55,48,87,112,53,12,169,191,131,44,139,63,7,152,58,172,14,127,193,96,112,246,78,28,218,220,172,164,124,33,109},
  {233,7,88,169,245,138,136,155,68,93,31,123,215,120,20,114,152,0,25,64,65,69,245,208,239,35,197,64,79,172,241,2},
  {61,101,52,96,193,219,34,197,56,228,8,41,247,55,20,132,35,123,168,137,255,18,94,150,60,94,124,47,23,172,16,155},
  {202,160,109,22,187,18,114,244,108,11,254,168,103,181,204,123,239,226,125,136,152,139,92,10,115,223,130,70,56,224,173,192},
  {19,22,170,41,57,196,167,197,237,159,108,72,100,236,202,254,165,39,22,31,150,255,39,145,145,109,234,73,190,112,233,226},
  {250,207,235,85,195,176,210,21,49,226,14,28,243,119,217,108,68,70,135,148,114,230,134,229,39,172,0,0,48,80,97,214},
  {171,155,199,168,248,84,62,38,61,251,253,20,229,129,247,173,217,81,240,145,14,20,90,23,134,78,232,212,51,34,250,172},
  {216,66,150,88,66,14,93,184,117,133,100,210,116,16,39,29,158,140,14,79,195,252,81,101,222,179,28,201,66,97,137,95},
  {126,10,81,72,98,92,102,101,31,240,58,127,17,123,117,166,91,170,166,84,188,34,154,208,46,251,14,87,0,105,114,47},
  {10,131,96,200,92,222,33,20,40,227,105,197,27,52,94,103,254,50,55,208,121,2,232,214,99,15,32,20,15,43,222,109},
  {245,128,192,158,84,249,137,57,221,154,202,22,214,133,39,248,114,73,117,202,58,176,43,180,136,84,106,233,196,76,47,60},
  {86,132,7,229,208,131,24,182,237,39,205,5,254,76,56,75,71,195,212,224,168,179,128,128,199,239,9,175,235,190,218,12},
  {218,116,75,114,172,230,16,193,163,154,126,185,187,254,154,213,18,237,33,147,209,222,139,84,80,198,201,142,171,183,9,178},
  {102,244,58,119,155,31,176,155,201,67,186,91,94,48,167,170,255,129,116,151,167,203,173,120,226,156,230,70,180,187,160,129},
  {48,124,248,1,229,78,90,159,144,127,179,25,138,145,171,159,139,209,42,221,103,134,165,156,190,149,2,186,49,191,106,198},
  {127,168,222,138,187,89,94,132,67,191,183,6,49,48,33,163,188,38,86,77,26,121,155,25,71,23,137,248,57,15,9,37},
  {29,96,25,19,112,147,141,120,188,41,78,222,44,150,121,75,69,175,171,106,40,52,87,126,121,16,126,87,23,36,150,140},
  {107,93,211,155,221,173,12,104,216,40,219,59,4,41,87,244,86,208,62,135,204,236,147,28,221,9,196,79,1,84,193,178},
  {134,208,252,118,56,61,27,237,20,246,149,81,237,237,113,234,136,14,30,201,171,146,132,221,168,115,237,104,160,183,135,222},
  {75,114,85,162,109,230,214,121,39,165,181,96,16,182,162,72,61,8,9,52,189,26,92,30,158,109,130,167,215,80,130,122},
  {167,187,243,125,206,236,2,177,69,39,27,152,40,227,228,231,28,216,46,12,238,71,36,195,154,40,15,179,66,180,77,245},
  {244,175,1,128,188,72,149,246,181,16,184,225,132,244,229,128,85,193,26,150,57,165,199,139,193,154,79,158,200,163,180,67},
  {130,235,58,201,21,190,167,80,54,37,71,222,197,100,193,93,169,199,76,141,191,166,181,1,89,8,197,71,71,185,112,226},
  {122,58,111,170,110,97,174,165,185,228,229,99,23,86,80,121,81,137,76,171,44,133,24,21,47,189,44,142,110,203,172,228},
  {138,90,118,69,204,231,142,73,87,186,169,126,23,144,77,150,22,85,129,155,224,213,218,29,206,88,136,173,148,187,212,48},
  {143,5,129,132,16,165,68,26,217,213,183,2,61,160,235,94,97,160,134,98,96,79,204,206,51,31,96,40,42,248,139,74},
  {92,243,79,38,222,93,146,249,0,77,52,32,178,24,251,60,104,5,10,117,211,94,109,59,250,54,153,231,102,60,234,71},
  {0,69,224,111,143,3,121,60,218,111,61,191,244,207,154,238,43,179,95,164,194,49,71,196,224,76,62,140,76,137,174,79},
  {165,25,173,168,70,166,223,122,78,153,39,69,250,162,177,233,203,26,79,231,92,0,158,9,116,19,14,193,175,27,150,117},
  {251,191,130,168,64,230,88,54,22,53,35,177,3,197,33,239,188,60,216,140,88,186,126,206,1,97,131,2,250,177,194,86},
  {50,82,191,214,169,252,155,195,155,212,255,127,87,214,19,237,14,27,139,171,142,23,34,74,60,136,88,220,116,122,30,67},
  {211,246,247,134,90,237,209,205,238,198,243,113,12,34,103,183,150,113,189,173,254,100,88,246,192,98,123,82,70,180,97,180},
  {11,45,66,197,85,116,154,149,65,54,212,30,20,171,189,126,107,236,184,213,194,189,199,3,122,186,156,36,49,221,75,243},
  {252,28,130,205,43,35,198,203,94,180,183,172,82,160,235,193,237,31,103,189,152,231,44,91,251,64,191,27,75,254,228,13},
  {150,106,51,130,110,109,118,34,37,58,101,225,223,130,237,20,148,88,34,55,189,39,159,170,9,166,166,101,82,155,237,246},
  {43,209,137,54,200,206,214,128,133,138,253,245,34,243,159,124,17,121,12,21,147,118,119,165,74,156,74,124,213,9,98,85},
  {121,226,255,72,156,59,180,108,213,127,172,208,139,35,21,97,37,3,146,121,43,122,6,83,172,242,162,166,229,184,224,33},
  {37,245,188,137,193,18,34,79,126,77,72,53,114,255,161,70,96,9,222,4,230,187,65,70,14,30,3,171,235,169,146,107},
  {220,118,210,98,144,140,101,108,216,156,242,52,155,146,195,50,23,251,15,92,0,242,114,115,202,176,242,122,2,187,113,48},
  {253,149,141,117,207,5,64,150,242,46,102,247,223,91,126,103,57,116,158,116,114,97,178,55,90,175,151,93,117,62,82,113},
  {106,22,136,121,130,169,103,161,11,79,65,78,207,152,232,123,241,32,124,101,14,77,170,5,120,175,47,189,82,25,245,59},
  {46,91,106,192,105,223,139,150,210,41,132,178,108,215,36,18,140,88,150,236,62,245,186,175,145,96,199,179,208,55,177,25},
  {197,250,34,65,211,83,13,97,5,122,131,88,133,115,37,233,133,115,96,127,218,231,122,121,42,85,169,187,2,171,159,59},
  {208,150,131,16,28,148,160,13,192,151,253,190,173,231,141,194,53,102,254,194,194,112,120,134,243,84,110,118,115,207,195,120},
  {42,70,117,8,76,143,126,151,191,183,111,41,170,242,171,213,73,232,57,158,199,163,13,0,109,71,169,249,154,34,155,123},
  {106,196,36,245,76,90,224,99,36,155,245,98,7,214,46,253,219,218,171,59,103,128,70,171,122,91,191,15,48,166,153,158},
  {170,215,102,187,235,20,108,177,84,222,40,135,127,131,20,164,228,65,29,220,23,40,59,98,61,249,215,13,202,142,94,189},
  {93,207,41,255,72,7,206,47,11,49,51,239,106,43,171,199,138,46,30,223,144,39,144,75,5,67,212,101,140,159,104,219},
  {44,175,48,213,87,226,33,200,167,239,38,114,103,117,42,90,228,64,131,219,79,3,234,177,48,0,9,125,56,205,68,79},
  {250,114,79,178,120,16,130,147,4,253,171,208,74,42,200,186,236,25,149,137,139,159,200,123,139,203,55,136,171,168,21,138},
  {31,239,111,66,250,165,152,62,61,1,47,9,134,219,166,127,174,8,170,206,103,158,99,99,235,95,103,58,250,213,120,252},
  {207,180,26,162,204,78,145,211,202,192,124,41,122,233,177,209,8,206,161,10,221,180,103,101,250,82,217,23,179,253,193,149},
  {222,88,162,11,91,133,48,186,207,86,217,44,216,141,164,95,29,80,166,16,139,0,83,201,246,29,109,78,238,68,125,157},
  {221,101,130,221,166,124,17,245,238,25,219,91,108,177,232,130,230,79,35,99,5,36,16,61,78,125,124,70,149,231,244,69},
  {114,119,121,98,175,166,37,124,162,211,162,97,85,10,67,79,143,254,46,16,63,219,181,76,202,121,194,114,32,13,109,169},
  {24,51,236,148,11,64,78,154,213,179,159,4,115,141,182,28,102,173,100,144,166,129,16,215,174,117,165,170,180,196,209,236},
  {84,208,129,137,9,251,193,7,81,120,32,66,84,245,156,199,34,180,154,249,92,2,166,86,165,248,77,132,252,142,53,69},
  {85,231,154,122,37,75,13,68,171,35,254,137,72,128,41,109,154,241,119,182,82,180,64,165,188,230,147,88,161,154,82,35},
  {235,152,251,182,2,242,135,168,32,166,19,246,233,172,27,50,121,44,12,173,182,131,17,187,88,155,43,151,159,53,48,239},
  {61,89,240,133,62,242,120,222,117,147,198,216,16,175,16,247,248,60,11,128,6,172,143,37,36,0,216,122,160,69,173,237},
  {231,146,78,235,9,155,237,211,193,96,61,198,102,79,190,77,54,114,95,224,213,45,193,36,145,253,126,52,11,186,208,182},
  {238,56,67,46,144,112,69,253,133,9,217,170,118,242,86,6,52,125,195,122,41,60,158,201,133,200,209,134,204,181,238,205},
  {225,31,186,168,194,238,124,228,39,238,102,175,30,210,63,137,5,69,69,238,72,90,235,71,120,124,68,174,100,168,208,14},
  {129,142,251,243,96,133,162,230,82,12,236,18,159,228,13,83,177,127,202,40,226,42,20,212,223,131,93,100,186,166,252,46},
  {179,223,101,162,71,212,155,246,157,206,93,80,72,143,130,63,213,149,244,37,215,193,98,58,109,142,63,23,177,0,200,36},
  {42,103,110,219,153,124,6,226,248,13,38,209,54,17,138,229,63,108,43,253,67,156,23,218,80,216,42,141,83,202,51,52},
  {79,20,219,25,137,225,165,76,146,77,39,38,68,101,196,110,27,120,16,166,225,132,241,144,51,252,47,72,213,55,167,64},
  {139,212,131,124,25,171,27,175,3,6,47,161,15,105,79,142,37,119,173,48,58,0,91,57,41,8,131,142,248,37,47,75},
  {125,125,27,37,31,143,135,2,194,130,146,95,233,172,2,232,137,39,16,69,209,12,149,214,71,224,189,87,96,39,17,71},
  {142,204,93,4,172,58,216,97,62,202,142,173,63,55,164,188,30,124,53,110,251,57,132,219,196,226,243,51,248,253,196,211},
  {7,221,73,126,146,254,126,111,117,142,17,31,237,8,0,68,160,154,88,55,112,212,116,44,71,109,134,79,44,120,112,37},
  {89,182,12,218,220,230,207,116,32,5,79,25,13,197,126,173,15,180,195,30,11,214,14,6,17,153,44,74,14,173,128,126},
  {217,221,54,187,59,23,198,164,60,160,219,64,248,168,85,207,97,19,155,220,128,246,48,137,142,49,224,90,61,44,219,101},
  {60,198,119,4,157,93,21,50,218,97,77,98,108,198,219,180,152,246,168,214,106,159,198,239,33,173,152,200,50,159,237,195},
  {92,165,88,239,5,221,61,225,13,149,25,23,82,2,96,1,119,0,65,173,221,78,4,206,227,5,131,64,13,203,112,190},
  {178,94,69,233,220,50,222,130,166,39,181,69,40,191,116,203,151,218,24,220,132,222,107,236,239,184,160,139,114,252,191,162},
  {67,255,22,107,137,168,56,244,159,61,114,162,124,161,245,73,222,225,62,157,114,138,222,7,97,23,219,193,201,28,176,56},
  {21,84,254,236,239,46,126,131,224,60,237,210,58,234,219,65,146,237,251,203,220,41,72,103,157,160,236,158,3,232,217,187},
  {17,171,138,180,180,226,243,241,22,94,105,224,150,240,171,28,114,160,47,85,212,91,5,38,164,168,199,192,131,226,94,133},
  {6,39,247,97,135,82,216,158,152,20,244,9,181,230,238,170,16,172,62,95,173,73,50,245,208,31,251,156,87,215,92,47},
  {25,242,127,15,170,249,103,59,20,244,130,221,215,176,77,12,200,251,121,85,238,49,5,133,211,209,74,129,236,119,40,245},
  {72,33,171,71,44,102,239,175,64,120,46,56,198,235,52,96,62,96,164,43,133,198,102,111,54,57,59,182,139,177,48,199},
  {181,28,192,208,105,213,47,2,4,133,81,88,166,125,243,216,66,163,87,202,118,19,60,94,65,189,56,132,89,231,127,14},
  {199,222,5,120,20,88,209,212,97,81,22,238,251,127,179,251,163,181,229,18,135,154,246,23,200,177,133,115,91,247,242,198},
  {91,180,105,35,174,87,20,235,92,71,89,77,152,131,114,106,81,139,4,171,161,35,61,125,41,1,135,240,155,128,35,68},
  {163,155,152,213,111,119,210,187,202,72,84,68,58,114,229,197,92,7,41,68,56,71,147,109,91,123,234,102,149,237,244,28},
  {31,39,168,112,198,233,1,159,99,200,173,70,129,62,5,64,131,128,111,196,62,251,31,79,244,99,219,93,147,157,111,252},
  {198,138,197,237,159,88,193,153,162,71,205,235,65,200,206,228,241,64,162,9,214,117,17,219,108,249,215,220,64,177,203,133},
  {17,170,83,102,67,207,39,21,223,218,81,224,198,140,140,93,161,165,42,92,44,89,180,253,126,55,124,229,101,60,181,201},
  {224,117,38,103,132,218,166,77,3,18,108,50,74,125,235,120,160,25,251,25,99,195,7,219,75,45,19,2,234,159,96,204},
  {160,161,157,146,31,135,215,211,9,246,14,28,195,160,51,136,175,159,70,107,190,56,185,117,213,1,209,161,89,105,112,14},
  {208,146,223,0,10,130,154,248,2,157,61,204,147,5,204,21,114,250,40,205,43,8,80,214,5,101,127,169,30,178,53,137},
  {152,145,24,105,128,88,244,36,82,90,72,193,127,80,177,84,145,182,153,215,126,186,231,74,140,122,23,47,254,224,71,241},
  {236,46,171,186,89,186,197,73,226,9,254,112,75,50,158,49,127,66,222,169,84,255,105,114,38,49,220,154,73,219,213,229},
  {247,239,31,190,101,187,254,229,126,75,92,192,3,105,157,224,172,44,207,92,15,79,2,45,244,181,206,165,77,46,57,251},
  {90,45,40,212,48,116,157,8,31,149,17,162,168,8,80,160,222,248,234,103,130,82,113,71,231,32,77,51,157,127,251,13},
  {130,87,3,11,229,236,213,51,129,246,89,146,134,202,228,170,229,222,134,180,251,27,102,183,8,20,163,90,167,120,229,170},
  {166,19,206,199,245,88,224,186,251,104,76,239,137,187,207,75,4,51,252,191,42,203,27,202,90,39,104,174,176,53,155,147},
  {70,98,133,45,92,153,42,114,193,1,169,74,85,162,189,47,40,192,195,195,190,167,17,8,124,248,198,91,77,77,76,187},
  {53,132,199,81,11,201,113,109,199,84,160,219,185,187,62,46,149,38,234,25,16,44,80,51,20,218,85,206,242,143,193,229},
  {105,95,221,186,249,163,76,30,159,173,15,12,47,116,53,165,247,77,250,228,41,122,20,199,135,56,204,8,32,185,110,204},
  {185,65,102,155,86,42,66,137,198,133,133,100,74,67,77,176,139,40,147,143,72,105,62,87,7,119,145,45,181,120,204,120},
  {225,250,207,49,79,0,242,177,148,78,153,60,96,143,63,27,126,123,35,67,94,64,246,82,4,143,123,46,214,217,4,203},
  {205,134,235,203,124,126,159,19,10,114,85,61,128,28,54,139,148,212,84,221,41,170,135,244,16,248,51,220,233,21,148,142},
  {226,246,94,150,74,153,137,249,205,77,50,216,122,57,174,15,56,200,114,149,108,39,37,64,119,244,18,45,180,144,81,110},
  {7,253,193,58,175,41,205,115,33,166,243,153,184,10,43,46,177,68,128,214,230,75,53,63,110,205,14,138,28,4,170,122},
  {252,94,239,81,117,164,15,227,11,136,47,221,160,0,112,48,75,245,169,49,28,66,103,200,31,165,123,91,102,225,49,181},
  {253,137,9,247,203,124,24,46,218,175,229,53,181,209,246,137,119,4,163,65,141,102,147,90,89,107,249,125,59,39,181,176},
  {26,73,199,77,137,156,65,62,242,248,157,124,0,233,203,85,118,6,69,150,81,226,45,216,106,201,6,12,234,114,1,55},
  {245,75,186,86,169,16,76,178,243,176,11,204,175,254,84,179,94,169,63,28,176,221,142,93,220,117,199,161,227,2,11,151},
  {15,182,29,11,249,249,210,54,202,63,102,206,22,139,241,14,240,225,101,98,97,88,120,55,200,151,109,26,119,136,34,156},
  {24,113,214,137,60,212,159,0,197,103,163,201,179,148,131,59,203,28,179,9,203,249,187,135,0,163,200,149,208,20,23,253},
  {231,182,113,252,88,106,180,122,247,46,118,29,64,46,11,183,82,110,78,0,249,244,125,26,167,156,41,250,113,111,87,27},
  {21,73,182,113,25,32,183,230,191,199,180,51,101,33,170,156,139,80,203,154,58,155,82,74,52,96,158,247,238,207,150,235},
  {143,37,213,51,161,231,224,75,16,56,35,130,76,12,253,214,79,34,55,212,159,106,240,121,43,193,22,182,251,220,206,220},
  {1,212,157,103,232,159,234,223,107,249,58,63,187,122,154,71,215,115,105,125,123,13,216,158,238,87,62,1,147,150,74,91},
  {4,125,207,3,202,0,18,17,153,119,245,2,150,30,216,90,168,14,225,105,57,217,74,74,170,97,59,191,217,207,41,50},
  {217,196,76,61,125,89,107,15,75,92,77,43,108,217,85,154,247,0,143,2,74,98,132,4,233,82,51,186,107,119,248,217},
  {202,5,147,74,179,48,169,40,156,198,55,4,197,182,10,91,43,62,80,143,47,103,78,121,37,164,50,113,36,101,11,118},
  {48,54,85,129,231,252,147,229,224,143,247,165,134,246,63,179,227,248,71,165,201,19,59,229,18,76,197,48,38,161,152,200},
  {15,178,181,41,106,62,93,252,17,170,218,222,225,22,147,136,1,241,195,59,36,141,156,234,43,72,126,126,41,17,193,34},
  {140,20,116,65,40,144,224,107,218,207,103,120,233,230,200,36,235,153,213,2,165,191,132,50,23,197,154,210,54,80,227,225},
  {16,186,3,79,167,172,35,23,153,145,76,173,252,89,100,128,157,108,140,3,111,159,86,179,182,166,18,0,144,105,90,60},
  {138,115,96,4,71,205,43,171,21,152,153,99,190,18,34,162,183,121,231,19,180,164,138,104,80,104,207,183,2,217,179,210},
  {128,83,222,94,225,133,130,100,117,220,66,98,26,70,116,10,171,17,177,191,249,73,198,53,147,139,0,141,89,108,21,132},
  {86,250,18,0,104,153,243,165,197,139,139,226,90,121,113,11,56,40,253,53,174,168,237,82,165,31,245,32,206,93,96,189},
  {1,129,111,159,224,98,133,48,105,2,130,113,58,161,223,238,13,149,116,37,158,4,80,158,137,172,82,159,76,30,90,198},
  {22,25,157,191,182,145,141,114,11,227,34,104,72,192,36,19,221,45,137,85,89,83,163,40,16,141,176,151,27,155,53,98},
  {235,205,101,195,113,156,65,13,74,68,207,37,52,0,64,243,229,176,52,109,93,181,253,95,202,37,46,81,102,160,111,253},
  {30,172,238,224,197,130,196,220,34,173,118,170,81,36,14,91,149,82,226,167,147,33,88,60,66,39,33,177,134,78,205,127},
  {29,4,43,97,249,178,206,176,24,181,101,239,229,11,192,189,75,210,11,146,108,200,142,33,195,168,88,98,47,178,42,86},
  {187,188,55,105,117,127,48,88,219,168,140,162,236,164,142,130,39,125,143,134,233,2,241,216,207,11,210,25,49,121,237,100},
  {91,236,197,142,183,255,180,242,223,169,55,214,19,251,194,108,83,232,55,186,94,32,121,252,117,219,77,53,28,82,178,197},
  {249,121,227,0,220,61,220,130,163,234,166,135,156,84,185,240,222,192,240,251,42,89,37,174,250,67,204,200,124,5,208,74},
  {199,49,248,141,255,31,48,127,152,79,223,151,117,207,201,52,60,57,104,59,61,108,162,64,156,190,128,175,242,55,94,48},
  {93,221,22,55,195,227,161,174,147,211,145,19,79,172,208,253,119,134,208,36,107,5,8,20,150,167,236,214,248,239,5,159},
  {5,156,184,178,39,194,31,255,117,92,162,195,37,165,244,163,1,95,247,189,238,81,110,7,131,121,120,104,241,60,93,66},
  {242,199,99,179,152,3,130,123,89,46,201,162,154,42,255,238,249,39,148,248,119,195,64,7,88,245,92,103,176,127,246,246},
  {241,148,11,238,65,111,83,221,62,163,122,3,214,81,163,124,215,161,175,59,109,179,234,146,143,101,238,191,51,189,207,19},
  {252,183,195,196,181,28,177,17,120,12,14,179,208,173,153,181,207,112,70,87,86,106,193,54,242,158,69,84,146,24,2,249},
  {223,165,235,66,86,199,79,249,146,49,58,149,36,142,248,189,193,104,102,128,215,30,81,109,178,82,166,213,180,252,107,6},
  {160,206,225,52,77,80,51,243,156,109,3,151,189,102,95,154,232,34,76,174,61,53,134,144,181,241,50,76,253,127,95,104},
  {182,169,173,124,154,41,121,130,249,60,17,47,224,46,34,4,109,78,19,61,55,95,38,31,3,241,167,172,114,247,68,94},
  {141,243,62,246,228,126,164,138,215,42,211,203,59,98,231,37,182,36,20,225,114,183,104,237,62,66,102,136,42,216,171,71},
  {76,5,85,225,28,105,236,171,21,214,182,252,200,65,53,141,180,203,157,174,166,136,231,81,155,199,255,157,38,182,191,34},
  {171,140,216,125,246,116,155,154,112,37,101,190,158,206,239,245,50,123,181,160,210,63,64,181,69,108,17,204,227,57,22,67},
  {16,155,44,56,94,103,146,14,89,136,151,106,208,28,115,62,147,211,236,88,68,162,46,191,252,165,129,173,162,71,195,131},
  {76,1,174,177,42,93,230,196,214,123,172,212,69,19,229,229,112,172,129,42,103,14,236,234,193,218,233,22,211,213,16,1},
  {144,32,180,164,202,131,21,99,23,54,90,43,226,186,88,251,65,173,68,225,7,246,245,25,193,0,199,155,74,232,45,118},
  {236,169,210,206,173,249,102,74,90,138,129,27,92,33,164,217,136,98,132,201,254,183,12,196,82,156,127,152,143,155,135,192},
  {242,118,199,205,119,164,108,93,255,103,82,80,71,24,76,10,37,191,147,128,26,14,16,1,201,170,187,82,112,15,231,179},
  {123,159,170,137,125,249,79,117,52,118,158,61,65,244,43,10,156,31,198,161,153,173,129,5,130,173,150,248,49,217,8,227},
  {66,244,159,19,88,253,213,23,32,209,189,251,8,249,71,51,66,19,33,88,169,184,184,83,24,137,123,237,88,112,136,100},
  {105,32,136,160,60,155,175,45,48,161,226,173,0,138,143,225,238,198,72,214,85,185,77,134,223,35,233,234,148,169,7,121},
  {114,237,141,232,195,47,33,59,235,143,14,231,243,190,94,78,53,71,137,45,84,252,75,0,62,232,175,124,142,112,220,222},
  {209,15,156,222,4,222,189,121,131,58,205,56,210,159,113,238,70,35,148,1,65,130,208,50,53,177,126,156,112,17,10,106},
  {105,85,101,187,197,3,87,214,112,224,218,200,110,84,23,24,84,51,176,82,246,133,72,56,74,110,162,183,16,65,156,9},
  {255,30,104,119,178,60,91,121,220,185,35,46,37,16,253,178,135,28,160,129,123,119,167,118,87,3,6,74,16,25,171,167},
  {116,185,13,114,141,41,129,67,255,178,218,214,149,9,141,97,144,215,186,32,22,135,194,93,20,52,11,141,223,165,75,67},
  {250,93,251,149,137,50,222,108,176,129,27,24,187,95,27,248,51,94,232,229,55,90,171,191,160,85,183,86,23,6,224,128},
  {65,99,53,92,228,166,24,237,239,207,164,149,189,109,242,0,113,5,174,121,235,137,62,7,24,44,128,29,111,223,2,8},
  {147,175,202,105,252,90,225,232,240,127,173,129,198,217,39,91,13,202,235,161,2,63,4,105,31,248,88,122,189,14,202,217},
  {130,38,64,153,50,178,132,13,249,105,90,134,211,153,139,250,179,52,6,126,172,131,79,192,196,11,75,32,14,192,52,157},
  {160,100,121,66,86,16,225,47,180,132,98,217,222,58,98,233,242,189,70,72,24,109,180,168,134,12,202,37,38,186,241,243},
  {24,143,235,33,67,131,180,27,71,202,101,41,90,157,230,92,56,143,162,41,110,98,199,236,58,206,79,101,250,17,98,84},
  {227,49,81,31,220,251,248,247,148,58,181,157,82,126,45,220,192,245,253,208,243,173,42,250,105,251,200,201,25,149,246,50},
  {193,235,196,99,199,87,172,17,95,224,191,36,128,62,51,208,76,95,133,10,13,137,195,28,125,191,249,15,61,55,2,94},
  {89,236,77,222,176,64,70,44,184,182,164,93,227,248,251,30,209,123,23,82,78,124,19,21,52,41,227,132,216,223,196,216},
  {142,181,57,7,232,193,70,222,196,113,47,171,219,255,149,10,92,22,47,155,193,100,162,214,251,60,200,133,126,165,61,164},
  {128,176,62,75,113,107,246,234,120,66,247,56,41,217,30,167,80,190,150,146,17,193,11,247,74,235,142,53,223,132,110,26},
  {36,77,85,226,56,38,117,133,11,109,233,69,172,47,97,87,165,160,3,75,141,239,61,167,109,222,27,229,7,145,39,212},
  {124,10,75,235,100,213,226,134,158,190,52,54,191,184,24,122,19,241,103,46,31,244,18,11,112,159,61,204,212,171,148,77},
  {114,77,137,119,43,99,102,76,57,148,166,12,209,145,244,18,51,32,100,72,123,209,37,111,231,73,58,102,253,18,34,162},
  {98,41,196,71,7,230,103,174,63,255,108,213,119,236,171,19,162,180,162,134,224,35,65,176,113,107,250,148,124,143,127,33},
  {42,75,239,217,24,95,98,148,232,45,231,23,58,147,211,241,137,82,226,205,0,137,96,60,250,6,17,71,225,110,153,121},
  {5,32,48,98,160,151,211,105,47,139,190,68,135,114,76,139,230,135,88,1,28,202,107,153,140,154,79,162,154,105,231,217},
  {117,246,27,36,54,185,197,126,128,70,19,206,190,244,213,241,2,214,6,42,73,184,224,42,134,19,67,210,95,131,206,65},
  {127,56,48,91,234,196,208,190,82,145,55,146,222,16,9,128,161,66,197,177,237,132,255,64,248,2,201,114,190,1,253,140},
  {132,172,226,25,189,173,154,215,196,207,251,66,181,178,144,119,92,47,169,146,3,24,13,155,176,60,167,107,65,196,145,120},
  {103,96,224,34,32,32,4,192,29,59,69,42,12,161,242,208,65,31,166,128,221,191,253,253,209,190,200,141,126,51,100,190},
  {58,127,250,228,154,38,80,111,154,118,109,43,124,162,201,216,109,113,253,122,247,187,181,147,40,167,0,129,243,224,127,55},
  {222,142,62,16,49,39,57,171,168,42,148,96,163,45,1,9,98,82,32,250,131,253,106,61,28,238,182,254,185,100,104,183},
  {221,15,180,234,241,143,88,255,62,22,37,253,96,95,93,214,130,78,236,187,114,98,25,123,184,13,24,116,241,244,9,3},
  {139,232,90,130,109,54,160,143,19,93,33,62,239,86,106,141,212,170,67,203,91,227,33,162,207,68,209,31,55,222,213,61},
  {128,219,230,29,173,177,141,63,189,111,11,56,254,236,235,103,133,18,242,200,176,98,251,230,170,142,80,174,220,115,74,136},
  {205,52,231,62,162,59,157,180,143,65,237,10,155,182,225,131,84,77,97,169,54,21,134,141,114,74,99,57,55,14,160,150},
  {120,117,181,0,114,107,75,63,81,45,26,100,177,189,190,75,76,155,79,108,26,93,46,211,26,166,229,184,32,217,220,117},
  {217,184,164,88,62,54,210,17,153,249,234,177,206,234,187,1,173,27,4,169,47,206,63,97,199,12,146,88,111,217,78,65},
  {129,242,248,169,53,254,41,115,23,99,37,158,165,8,170,24,63,109,136,182,235,12,252,1,226,141,235,45,76,29,144,179},
  {204,176,178,17,32,215,206,10,207,12,78,92,119,191,189,237,111,57,21,219,243,225,101,99,132,103,153,131,44,57,99,194},
  {103,125,120,219,142,193,63,99,95,94,120,255,195,163,244,192,141,144,144,242,54,7,248,237,124,71,23,57,198,209,115,177},
  {6,229,131,13,74,195,162,9,2,228,150,154,123,93,113,203,82,225,206,134,33,190,195,16,138,7,140,6,165,142,200,248},
  {69,29,123,95,20,77,200,220,161,127,205,110,115,242,135,240,13,237,24,47,141,27,141,147,168,195,78,250,57,75,33,193},
  {56,76,151,133,96,85,108,66,115,196,191,219,109,219,122,25,202,81,94,248,63,77,42,51,36,139,236,237,20,60,17,48},
  {119,37,3,232,154,73,101,203,242,144,126,179,22,10,45,94,17,184,23,255,211,161,52,136,26,150,171,21,252,192,146,105},
  {185,89,159,240,55,61,227,236,79,54,134,247,113,119,201,223,61,57,176,52,178,107,76,248,195,183,218,70,2,173,201,75},
  {238,4,202,6,200,58,96,24,207,82,16,141,202,169,160,7,165,61,40,5,116,241,187,116,159,80,6,242,182,173,94,59},
  {101,222,223,26,206,156,204,185,76,95,32,117,10,126,86,35,35,87,27,95,11,25,168,243,204,196,231,23,141,222,245,4},
  {228,205,241,152,208,216,43,53,3,195,46,142,103,217,206,84,73,194,178,157,241,64,165,11,152,2,113,151,69,234,12,192},
  {153,8,207,112,220,24,154,208,124,64,209,151,219,203,20,87,3,185,38,62,238,27,69,114,86,102,36,245,54,109,70,217},
  {49,205,141,255,216,41,246,132,104,170,191,91,166,2,115,173,134,218,66,23,152,187,30,137,25,191,57,232,211,39,135,94},
  {173,94,114,153,255,15,146,241,23,12,151,123,87,7,174,47,165,162,164,136,62,36,54,15,9,138,189,230,124,181,208,104},
  {182,183,161,12,243,138,218,91,67,43,210,252,17,182,137,251,37,22,202,234,43,254,44,97,244,244,97,40,199,151,186,161},
  {183,164,111,106,254,186,133,112,217,52,220,7,24,172,132,172,170,68,251,154,140,38,240,10,204,87,79,34,55,162,159,103},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
} ;

static const unsigned char precomputed_nP_montgomery25519_p[precomputed_nP_montgomery25519_NUM][crypto_nP_POINTBYTES] = {
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {8,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {11,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {14,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {17,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {18,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {19,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {22,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {23,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {24,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {26,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {27,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {28,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {29,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {30,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {31,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {111,18,243,62,180,93,101,187,23,73,13,39,192,139,194,106,10,72,242,42,183,238,122,107,144,57,141,20,210,163,142,0},
  {224,235,122,124,59,65,184,174,22,86,227,250,241,159,196,106,218,9,141,235,156,50,177,253,134,98,5,22,95,73,184,0},
  {245,196,70,99,114,97,152,96,91,20,31,160,88,52,151,161,94,92,11,177,23,242,92,137,253,43,147,168,159,120,99,3},
  {38,232,149,143,194,178,39,176,69,195,244,137,242,239,152,240,213,223,172,5,211,198,51,57,177,56,2,136,109,83,252,5},
  {180,140,35,108,225,133,69,227,5,99,158,116,180,144,160,84,162,70,208,203,161,213,101,113,83,32,64,115,124,202,72,8},
  {95,70,199,15,156,222,195,114,220,234,120,29,41,57,178,200,195,111,57,87,200,244,18,3,131,83,38,147,255,157,82,11},
  {100,78,166,101,186,16,33,249,148,243,238,215,228,226,85,199,101,181,163,20,243,90,100,96,195,116,97,35,31,153,67,17},
  {216,45,32,54,238,248,211,160,133,246,29,225,4,154,119,36,23,185,121,58,169,66,254,145,201,141,179,170,190,62,54,18},
  {36,64,6,108,6,62,53,35,126,236,93,246,67,94,0,106,66,15,83,213,254,31,127,68,93,156,238,47,127,221,92,20},
  {148,214,197,89,82,44,90,188,128,24,185,102,86,13,225,157,32,231,77,115,186,232,140,218,184,193,239,49,124,231,169,21},
  {68,129,160,218,218,241,248,204,45,63,176,224,36,15,218,40,118,107,85,223,166,31,199,0,47,181,170,156,116,254,39,22},
  {207,108,156,64,182,29,155,51,168,111,164,225,193,194,145,89,21,77,220,193,35,3,222,175,89,78,123,87,130,50,248,22},
  {137,80,205,184,163,118,77,202,204,124,57,5,93,109,93,13,226,255,107,99,47,121,243,196,104,138,162,229,83,2,98,26},
  {34,240,10,243,6,229,25,22,251,207,81,236,92,210,208,177,233,177,0,228,234,100,198,50,208,115,10,189,11,213,7,32},
  {55,42,50,167,103,43,115,186,153,214,232,93,85,246,127,28,39,87,49,26,34,202,133,77,222,157,109,253,151,211,235,32},
  {35,194,4,145,228,170,52,36,136,247,152,246,231,250,199,161,157,68,105,95,235,110,137,99,98,244,73,208,83,37,109,35},
  {106,173,113,9,210,120,250,178,209,255,187,62,17,65,238,231,37,96,212,220,76,123,244,227,72,68,22,201,239,234,252,35},
  {69,46,183,255,198,153,158,240,122,231,14,3,68,224,88,41,255,150,5,100,114,25,47,223,131,156,7,132,91,115,13,36},
  {148,73,188,146,204,183,204,19,83,160,11,237,141,97,161,237,152,98,49,50,41,86,14,129,143,0,92,46,72,233,49,38},
  {165,184,205,223,247,7,80,140,96,73,134,57,123,193,114,229,182,148,182,23,189,138,186,90,244,253,92,62,29,75,227,38},
  {108,20,31,116,94,141,126,45,163,85,126,44,19,190,238,243,224,86,12,212,102,52,127,109,241,32,139,167,37,52,87,39},
  {26,98,96,48,105,236,129,197,157,34,71,126,66,69,87,108,146,182,197,41,180,14,89,76,92,46,119,230,190,58,177,46},
  {56,103,119,158,120,22,48,230,208,156,54,119,226,189,178,33,160,83,156,57,66,229,19,16,110,194,27,195,36,104,230,50},
  {148,163,155,168,115,30,231,224,48,18,225,45,29,95,43,203,221,69,199,158,140,41,162,221,127,99,184,43,182,49,191,52},
  {33,110,138,129,221,49,212,141,87,34,182,133,210,124,174,124,181,138,116,52,154,117,0,194,143,57,157,171,95,142,131,54},
  {16,14,168,202,146,235,104,175,47,76,126,143,2,68,12,74,128,54,65,154,82,43,146,59,129,187,98,222,21,120,161,57},
  {77,80,233,58,202,29,49,53,159,3,213,231,47,143,129,165,208,30,47,197,42,119,210,132,72,188,68,202,24,143,20,61},
  {247,9,183,167,211,71,178,197,159,211,252,106,164,133,217,230,19,245,215,29,92,240,192,129,163,60,151,149,102,219,251,61},
  {21,85,14,139,190,149,43,205,86,195,25,195,234,193,198,172,181,215,183,28,239,61,27,154,113,244,161,237,94,192,145,62},
  {2,179,55,63,101,158,203,81,211,253,4,244,143,91,239,184,216,246,222,242,51,162,197,100,105,151,85,160,120,245,221,63},
  {144,35,226,102,111,234,137,9,121,153,51,58,8,51,155,153,57,79,246,105,200,26,149,246,46,35,137,143,149,5,19,66},
  {39,183,227,89,201,132,189,69,4,190,233,93,209,47,210,121,112,180,254,123,62,210,123,36,178,222,2,150,99,8,35,66},
  {56,176,43,249,168,49,164,59,122,166,156,67,205,1,29,102,123,29,160,44,247,99,10,241,124,103,109,238,186,72,8,75},
  {178,235,248,67,138,16,63,246,250,175,128,244,254,92,235,203,151,120,251,136,184,163,190,14,217,41,101,253,24,24,44,78},
  {144,156,227,220,61,222,81,95,42,91,74,29,61,104,162,134,169,177,250,227,191,48,4,92,102,178,221,36,47,191,11,80},
  {91,92,141,74,120,143,102,65,177,14,15,243,89,123,207,69,62,223,25,195,152,71,40,96,65,238,58,41,216,213,97,82},
  {43,198,245,39,220,128,136,246,170,207,98,217,133,116,138,163,11,32,198,18,109,169,214,121,50,33,201,242,219,142,111,82},
  {10,177,130,220,69,165,144,38,157,90,143,67,154,190,167,190,50,19,237,121,170,125,249,96,216,189,175,54,84,188,180,82},
  {94,229,114,115,221,228,181,199,119,253,17,139,10,59,149,2,202,196,187,230,76,134,89,44,251,24,230,195,37,229,246,85},
  {12,123,117,152,26,247,24,62,172,166,28,83,83,247,213,200,159,238,221,115,67,36,1,103,17,111,231,9,73,16,197,86},
  {95,156,149,188,163,80,140,36,177,208,177,85,156,131,239,91,4,68,92,196,88,28,142,134,216,34,78,221,208,159,17,87},
  {22,233,173,34,218,187,183,195,246,174,184,130,178,202,229,97,80,232,75,20,49,182,202,180,87,80,28,232,150,21,32,87},
  {26,84,171,174,223,46,251,179,68,112,12,140,25,120,168,230,49,157,85,92,120,86,93,64,218,91,204,5,82,50,64,87},
  {89,147,70,16,191,57,46,101,100,225,112,11,39,133,85,116,187,123,222,118,25,139,220,212,91,148,174,60,170,219,28,88},
  {237,126,162,241,233,132,69,64,152,207,154,191,214,121,178,91,45,8,97,215,143,67,183,19,166,189,181,22,103,153,198,88},
  {166,71,52,250,165,142,124,133,244,167,47,167,159,231,196,183,228,176,163,220,102,133,113,231,10,178,182,106,107,111,142,89},
  {187,24,54,60,85,139,162,25,69,55,173,79,185,91,202,234,176,176,71,225,85,8,207,26,71,123,185,60,233,190,79,90},
  {133,133,21,198,3,134,9,12,210,254,123,114,174,238,77,96,254,88,37,64,5,178,199,141,45,0,39,9,184,239,76,94},
  {67,255,144,201,115,140,26,118,4,229,216,118,29,235,86,170,110,116,92,125,4,165,62,125,93,218,171,166,14,251,223,95},
  {227,121,24,129,106,238,151,39,102,19,51,220,96,154,229,232,131,181,191,15,149,0,235,69,220,12,5,240,198,151,132,96},
  {240,22,72,68,27,247,111,127,36,102,253,162,235,57,103,223,77,206,56,9,214,96,69,138,244,14,98,103,63,107,126,98},
  {216,133,220,191,70,32,82,157,83,231,220,192,43,155,5,1,175,92,52,144,185,209,117,107,251,155,54,254,111,193,98,99},
  {85,180,86,15,22,113,163,114,168,159,20,183,93,182,205,13,113,42,45,139,252,244,3,166,9,43,252,141,81,24,129,103},
  {41,161,241,169,209,53,71,139,150,143,73,155,162,199,45,186,77,244,153,141,226,6,54,243,185,180,107,39,148,117,92,106},
  {128,23,97,182,37,168,50,230,91,51,94,191,211,109,124,125,74,88,82,154,251,171,187,124,56,245,169,188,106,34,145,107},
  {127,15,15,250,255,35,159,42,17,119,215,29,236,176,123,244,254,92,142,79,152,129,191,128,162,120,217,207,55,134,218,108},
  {15,154,28,146,107,252,218,170,25,168,127,42,52,44,9,232,19,129,115,18,153,155,113,175,13,84,224,189,164,184,185,109},
  {102,152,70,122,156,35,87,188,3,59,236,143,255,220,32,60,180,184,194,174,67,78,155,210,158,158,124,57,123,29,226,111},
  {224,127,120,206,90,168,171,155,214,37,59,146,114,174,149,43,10,200,87,58,190,106,83,37,22,129,102,187,150,99,56,112},
  {145,46,193,20,199,31,45,117,163,16,240,251,124,50,2,144,229,182,166,220,69,62,50,87,111,130,156,251,207,229,213,117},
  {132,5,166,191,250,161,222,193,180,248,102,211,248,213,60,251,68,10,193,132,181,254,109,76,71,80,157,162,230,28,133,121},
  {199,23,106,112,61,77,216,79,186,60,11,118,13,16,103,15,42,32,83,250,44,57,204,198,78,199,253,119,146,172,3,122},
  {40,159,165,57,215,189,171,104,153,204,112,113,174,10,40,167,37,157,88,148,126,99,85,136,235,128,68,248,197,12,47,122},
  {224,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {225,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {226,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {227,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {228,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {229,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {230,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {231,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {232,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {233,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {234,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {235,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {237,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {238,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {239,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {240,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {241,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {245,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {248,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {250,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {252,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {253,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {254,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {8,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {11,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {14,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {17,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {18,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {19,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {22,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {23,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {24,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {26,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {27,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {28,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {29,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {30,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {31,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {205,235,122,124,59,65,184,174,22,86,227,250,241,159,196,106,218,9,141,235,156,50,177,253,134,98,5,22,95,73,184,128},
  {40,98,193,190,245,237,7,74,121,123,4,94,59,199,62,176,195,18,156,252,169,18,89,40,237,6,154,127,2,176,94,129},
  {181,0,249,162,129,240,19,130,20,185,201,69,162,235,51,51,160,82,204,246,150,73,125,6,149,42,172,102,0,215,65,132},
  {62,73,24,66,66,142,216,188,26,26,140,60,231,157,121,133,255,143,145,204,30,172,47,178,111,241,184,140,95,102,197,132},
  {19,232,149,143,194,178,39,176,69,195,244,137,242,239,152,240,213,223,172,5,211,198,51,57,177,56,2,136,109,83,252,133},
  {125,55,105,100,127,60,72,138,81,84,228,190,83,158,168,146,141,87,127,175,218,74,35,174,189,170,2,180,179,16,1,137},
  {129,43,98,81,200,60,211,58,130,133,78,109,209,68,63,72,135,148,109,167,130,138,113,36,70,35,125,182,53,181,83,140},
  {112,219,85,141,78,156,71,229,153,137,212,86,246,41,139,137,210,13,246,3,78,206,173,113,225,30,236,100,242,203,91,142},
  {231,77,226,40,229,224,72,3,74,203,150,143,218,164,211,149,236,191,232,231,122,157,146,89,72,59,89,49,46,161,128,143},
  {133,115,68,47,111,244,70,154,6,222,111,195,182,203,89,217,56,164,165,2,53,122,87,204,87,223,122,234,67,192,154,144},
  {223,12,82,97,145,239,48,99,177,83,194,80,181,66,99,23,65,96,19,230,123,192,120,197,142,28,29,234,21,134,160,148},
  {248,209,60,152,240,138,191,75,173,194,6,183,199,84,27,79,175,215,246,27,222,29,199,47,232,61,233,117,32,51,140,150},
  {219,58,144,115,100,151,51,85,47,103,37,189,190,237,157,116,69,199,145,142,58,96,214,20,32,232,39,197,126,193,49,151},
  {95,200,38,56,245,199,134,15,168,128,65,85,197,140,23,11,141,135,124,193,253,118,2,188,6,130,168,239,200,176,14,152},
  {170,138,69,101,51,179,185,50,235,207,22,189,77,155,217,80,172,85,92,4,153,152,118,128,93,130,49,239,154,53,165,152},
  {174,139,27,98,245,150,194,250,72,239,94,30,130,68,86,121,100,115,184,14,46,231,88,52,64,201,183,229,190,44,174,154},
  {43,252,244,51,167,151,107,235,93,92,248,1,29,208,144,19,254,172,17,60,103,54,237,33,7,26,50,245,24,147,154,155},
  {36,129,171,217,20,211,14,8,126,132,169,222,235,63,12,118,50,155,57,73,49,89,249,62,24,217,147,143,203,111,156,155},
  {205,2,202,59,157,250,84,118,151,165,55,104,243,106,40,149,64,241,251,65,42,56,34,16,234,54,211,253,5,37,252,155},
  {59,200,140,46,90,47,67,112,167,35,75,6,224,7,207,227,99,207,251,113,168,1,23,218,226,51,117,255,161,142,153,156},
  {92,215,72,182,154,128,219,211,47,140,246,17,244,233,3,54,167,134,151,153,169,146,83,148,29,65,179,15,94,41,232,161},
  {123,220,115,8,78,56,160,102,17,73,213,119,146,233,53,244,107,123,154,141,135,28,131,87,69,49,87,209,20,249,56,162},
  {86,147,246,133,145,58,250,80,175,26,64,139,193,73,15,156,233,170,63,226,153,120,103,252,148,59,25,147,112,38,215,163},
  {77,15,3,198,139,24,114,103,137,170,49,188,87,52,125,80,8,103,119,222,166,174,134,83,41,191,108,14,195,128,111,169},
  {96,42,30,228,229,15,43,141,229,125,150,149,63,62,255,69,132,153,11,122,8,147,80,31,71,0,114,172,6,236,188,169},
  {94,247,178,221,6,102,158,19,44,117,210,243,87,196,221,157,214,60,167,130,240,210,131,167,12,112,193,95,51,171,253,169},
  {47,105,39,192,222,200,75,96,119,240,6,138,50,238,145,191,125,221,161,35,221,145,84,101,52,190,86,71,200,76,239,170},
  {88,199,215,35,217,92,77,65,25,45,63,102,205,94,235,17,76,74,238,228,59,31,46,64,132,86,206,193,8,10,254,170},
  {236,221,110,85,119,102,142,32,139,41,201,114,77,187,166,133,54,229,25,180,236,76,218,201,124,44,182,42,4,183,49,172},
  {255,194,56,232,174,245,62,71,26,68,191,128,100,50,10,237,213,105,100,167,68,21,27,124,129,62,45,161,97,228,235,172},
  {24,248,205,13,251,68,160,65,173,109,89,254,224,107,51,244,92,207,167,180,90,30,242,9,86,56,9,82,78,63,98,173},
  {5,148,50,200,225,154,104,2,186,157,64,251,100,43,110,250,60,12,112,60,188,21,135,97,51,100,226,99,38,208,159,176},
  {58,124,149,244,219,245,28,139,123,7,200,215,136,196,159,79,45,230,10,167,187,124,197,72,22,196,111,77,4,44,35,178},
  {206,106,196,63,222,184,149,49,241,126,132,63,200,64,98,76,47,56,113,193,227,167,145,250,109,15,29,169,198,4,252,178},
  {202,89,131,157,3,118,249,46,74,101,132,152,106,47,243,202,220,92,140,62,172,55,113,156,173,4,96,104,141,80,79,180},
  {23,11,234,10,227,139,156,80,199,166,30,156,2,164,43,61,58,71,110,74,197,50,42,192,114,28,144,175,233,77,233,181},
  {4,185,44,222,234,105,45,228,84,211,77,226,210,230,92,138,18,64,100,12,109,20,161,29,169,107,73,50,2,113,32,184},
  {167,149,123,83,102,56,8,128,52,35,252,196,135,30,152,179,223,240,232,71,44,238,176,128,107,94,97,60,121,129,219,184},
  {92,16,248,176,17,243,88,13,137,6,58,182,139,221,105,188,108,139,108,94,186,224,118,160,35,97,77,197,56,233,17,190},
  {90,240,227,251,144,184,238,241,55,58,14,223,188,73,5,141,231,144,206,200,253,27,157,213,251,175,49,90,4,21,43,193},
  {70,56,241,199,2,77,116,39,168,14,15,19,243,81,48,228,66,197,86,102,62,178,178,118,177,163,197,69,113,99,107,196},
  {179,13,226,14,207,73,38,8,115,109,3,46,235,11,118,202,164,47,136,100,117,120,90,6,44,242,147,142,67,58,34,197},
  {110,16,92,235,125,30,216,86,90,26,103,163,229,193,149,39,17,86,214,36,50,108,148,100,208,16,40,12,194,4,159,198},
  {95,204,232,169,227,16,187,78,103,76,46,253,39,219,178,130,101,27,212,225,115,213,227,31,126,186,195,148,169,85,245,199},
  {174,0,5,175,234,211,162,13,126,160,91,211,165,51,116,163,210,242,48,25,249,254,242,78,240,81,44,86,166,90,115,206},
  {223,152,153,220,140,131,68,151,208,123,2,153,171,70,47,103,111,91,11,234,230,63,51,223,200,244,236,171,145,50,249,207},
  {155,109,9,21,56,25,184,169,128,198,2,189,9,126,77,58,227,73,99,0,182,182,150,89,26,73,98,86,218,251,57,209},
  {250,180,62,72,167,188,211,161,75,8,67,186,254,249,239,196,135,111,170,129,173,38,239,174,51,106,216,119,163,96,80,209},
  {125,242,29,114,232,48,27,244,31,171,46,41,1,218,112,38,126,119,17,27,127,158,41,97,36,105,51,103,198,133,79,214},
  {76,156,149,188,163,80,140,36,177,208,177,85,156,131,239,91,4,68,92,196,88,28,142,134,216,34,78,221,208,159,17,215},
  {3,30,161,46,86,232,112,81,22,22,166,87,64,190,106,88,166,32,34,174,203,96,240,21,231,168,202,224,136,252,95,216},
  {58,50,13,118,120,27,56,5,140,124,44,234,250,149,21,57,233,36,207,197,165,234,195,118,154,167,56,193,69,102,30,217},
  {33,166,110,206,84,190,224,115,16,16,158,248,30,0,233,114,28,202,152,41,116,80,233,121,144,82,116,76,168,173,30,218},
  {56,0,37,25,36,249,203,33,105,221,23,56,110,251,249,180,154,183,67,64,123,150,36,125,50,218,191,132,212,126,128,222},
  {142,22,38,205,200,222,195,37,214,165,19,37,81,22,87,95,227,152,243,211,179,168,34,169,78,193,55,91,16,56,230,222},
  {250,186,28,252,147,237,14,97,25,165,61,72,56,205,33,143,0,159,175,39,99,66,31,226,24,190,5,150,240,223,238,222},
  {141,193,75,162,4,203,133,174,153,196,55,67,223,46,171,9,180,112,112,89,191,187,159,93,201,66,140,226,7,118,67,226},
  {236,16,165,65,1,173,130,47,201,213,171,154,185,172,79,19,203,117,86,19,191,75,213,174,85,205,184,195,105,19,136,228},
  {87,6,127,250,161,21,246,107,3,203,72,146,112,227,19,204,92,125,202,100,252,110,139,19,133,193,218,63,15,199,167,228},
  {112,52,5,29,28,239,246,218,83,106,50,236,83,212,229,146,245,69,47,61,220,72,230,59,243,251,128,98,206,213,120,229},
  {84,33,33,195,205,95,87,46,215,226,73,47,6,206,1,220,87,223,209,122,91,233,94,210,113,168,229,78,121,87,53,230},
  {38,57,245,107,31,171,57,222,37,89,242,246,200,69,132,204,142,29,85,51,40,241,114,164,100,92,53,88,211,92,202,230},
  {41,26,248,132,62,248,195,130,95,125,203,188,212,31,51,250,131,201,151,226,53,4,200,159,144,59,61,181,50,96,124,232},
  {237,131,60,71,84,163,46,254,98,12,230,87,251,116,196,185,116,253,229,243,247,223,86,243,70,181,200,64,6,180,114,234},
  {159,109,38,90,9,148,16,166,4,12,203,81,214,74,15,69,187,18,59,178,98,237,243,35,8,205,27,12,125,168,206,236},
  {77,177,87,210,156,163,175,191,211,150,62,37,21,150,200,44,125,200,224,117,67,164,169,113,221,215,61,61,88,133,65,237},
  {51,146,91,49,186,142,83,19,20,166,202,118,177,2,123,32,74,77,241,206,200,63,243,219,5,255,246,165,176,60,121,242},
  {37,23,19,91,121,184,178,247,132,107,151,13,254,239,14,213,228,79,139,141,234,186,35,114,206,67,48,145,227,157,176,245},
  {180,23,106,112,61,77,216,79,186,60,11,118,13,16,103,15,42,32,83,250,44,57,204,198,78,199,253,119,146,172,3,250},
  {128,48,20,114,22,220,220,209,152,138,21,42,4,56,162,215,127,41,23,102,98,16,235,175,36,137,194,230,234,79,79,251},
  {235,162,54,148,157,227,126,179,250,16,171,200,149,22,152,172,34,91,213,50,6,121,240,134,54,215,4,38,91,154,39,252},
  {14,156,212,9,229,242,211,90,86,23,60,168,108,114,41,2,84,100,202,204,244,219,54,187,48,194,31,211,37,152,156,253},
  {26,210,64,74,96,212,130,61,169,249,41,43,135,136,190,112,128,56,18,47,169,218,118,240,98,194,168,50,223,178,247,254},
  {192,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {193,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {194,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {195,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {196,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {197,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {198,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {199,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {200,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {201,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {202,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {203,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {204,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {205,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {206,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {207,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {208,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {209,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {210,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {211,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {212,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {213,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {214,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {215,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {216,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {217,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {218,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {219,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {220,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {221,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {222,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {223,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {224,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {225,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {226,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {227,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {228,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {229,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {230,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {231,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {232,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {233,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {234,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {235,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {237,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {238,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {239,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {240,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {241,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {245,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {248,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {250,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {252,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {253,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {254,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
} ;

static void test_nP_montgomery25519_impl(long long impl)
{
  unsigned char *q = test_nP_montgomery25519_q;
  unsigned char *n = test_nP_montgomery25519_n;
  unsigned char *p = test_nP_montgomery25519_p;
  unsigned char *q2 = test_nP_montgomery25519_q2;
  unsigned char *n2 = test_nP_montgomery25519_n2;
  unsigned char *p2 = test_nP_montgomery25519_p2;
  long long qlen = crypto_nP_POINTBYTES;
  long long nlen = crypto_nP_SCALARBYTES;
  long long plen = crypto_nP_POINTBYTES;

  if (targeti && strcmp(targeti,lib25519_dispatch_nP_montgomery25519_implementation(impl))) return;
  if (impl >= 0) {
    crypto_nP = lib25519_dispatch_nP_montgomery25519(impl);
    printf("nP_montgomery25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_nP_montgomery25519_implementation(impl),lib25519_dispatch_nP_montgomery25519_compiler(impl));
  } else {
    crypto_nP = lib25519_nP_montgomery25519;
    printf("nP_montgomery25519 selected implementation %s compiler %s\n",lib25519_nP_montgomery25519_implementation(),lib25519_nP_montgomery25519_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 512 : 64;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {

      output_prepare(q2,q,qlen);
      input_prepare(n2,n,nlen);
      input_prepare(p2,p,plen);
      crypto_nP(q,n,p);
      checksum(q,qlen);
      output_compare(q2,q,qlen,"crypto_nP");
      input_compare(n2,n,nlen,"crypto_nP");
      input_compare(p2,p,plen,"crypto_nP");

      double_canary(q2,q,qlen);
      double_canary(n2,n,nlen);
      double_canary(p2,p,plen);
      crypto_nP(q2,n2,p2);
      if (memcmp(q2,q,qlen) != 0) fail("failure: crypto_nP is nondeterministic\n");

      double_canary(q2,q,qlen);
      double_canary(n2,n,nlen);
      double_canary(p2,p,plen);
      crypto_nP(n2,n2,p);
      if (memcmp(n2,q,qlen) != 0) fail("failure: crypto_nP does not handle n=q overlap\n");
      memcpy(n2,n,nlen);
      crypto_nP(p2,n,p2);
      if (memcmp(p2,q,qlen) != 0) fail("failure: crypto_nP does not handle p=q overlap\n");
      memcpy(p2,p,plen);
    }
    checksum_expected(nP_montgomery25519_checksums[checksumbig]);
  }
  for (long long precomp = 0;precomp < precomputed_nP_montgomery25519_NUM;++precomp) {
    output_prepare(q2,q,crypto_nP_POINTBYTES);
    input_prepare(n2,n,crypto_nP_SCALARBYTES);
    memcpy(n,precomputed_nP_montgomery25519_n[precomp],crypto_nP_SCALARBYTES);
    memcpy(n2,precomputed_nP_montgomery25519_n[precomp],crypto_nP_SCALARBYTES);
    input_prepare(p2,p,crypto_nP_POINTBYTES);
    memcpy(p,precomputed_nP_montgomery25519_p[precomp],crypto_nP_POINTBYTES);
    memcpy(p2,precomputed_nP_montgomery25519_p[precomp],crypto_nP_POINTBYTES);
    crypto_nP(q,n,p);
    if (memcmp(q,precomputed_nP_montgomery25519_q[precomp],crypto_nP_POINTBYTES)) {
      fail("failure: crypto_nP fails precomputed test vectors\n");
      printf("expected q: ");
      for (long long pos = 0;pos < crypto_nP_POINTBYTES;++pos) printf("%02x",precomputed_nP_montgomery25519_q[precomp][pos]);
      printf("\n");
      printf("received q: ");
      for (long long pos = 0;pos < crypto_nP_POINTBYTES;++pos) printf("%02x",q[pos]);
      printf("\n");
    }
    output_compare(q2,q,crypto_nP_POINTBYTES,"crypto_nP");
    input_compare(n2,n,crypto_nP_SCALARBYTES,"crypto_nP");
    input_compare(p2,p,crypto_nP_POINTBYTES,"crypto_nP");
  }
}

static void test_nP_montgomery25519(void)
{
  if (targeto && strcmp(targeto,"nP")) return;
  if (targetp && strcmp(targetp,"montgomery25519")) return;
  test_nP_montgomery25519_q = alignedcalloc(crypto_nP_POINTBYTES);
  test_nP_montgomery25519_n = alignedcalloc(crypto_nP_SCALARBYTES);
  test_nP_montgomery25519_p = alignedcalloc(crypto_nP_POINTBYTES);
  test_nP_montgomery25519_q2 = alignedcalloc(crypto_nP_POINTBYTES);
  test_nP_montgomery25519_n2 = alignedcalloc(crypto_nP_SCALARBYTES);
  test_nP_montgomery25519_p2 = alignedcalloc(crypto_nP_POINTBYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("nP_montgomery25519 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_nP_montgomery25519();++impl)
      forked(test_nP_montgomery25519_impl,impl);
    ++test_nP_montgomery25519_q;
    ++test_nP_montgomery25519_n;
    ++test_nP_montgomery25519_p;
    ++test_nP_montgomery25519_q2;
    ++test_nP_montgomery25519_n2;
    ++test_nP_montgomery25519_p2;
  }
}
#undef crypto_nP_SCALARBYTES
#undef crypto_nP_POINTBYTES


/* ----- nG, derived from supercop/crypto_nG/try.c */
static const char *nG_merged25519_checksums[] = {
  "a4e761839798a07817484e97605bd63215b4938934ed9ce01935bbced48155bc",
  "0a01c09fc8a8c7e8c18f841b2e1b2da9c156868737d194d223b03531cf2db731",
} ;

static void (*crypto_nG)(unsigned char *,const unsigned char *);
#define crypto_nG_SCALARBYTES lib25519_nG_merged25519_SCALARBYTES
#define crypto_nG_POINTBYTES lib25519_nG_merged25519_POINTBYTES

static unsigned char *test_nG_merged25519_q;
static unsigned char *test_nG_merged25519_n;
static unsigned char *test_nG_merged25519_q2;
static unsigned char *test_nG_merged25519_n2;

#define precomputed_nG_merged25519_NUM 204

static const unsigned char precomputed_nG_merged25519_q[precomputed_nG_merged25519_NUM][crypto_nG_POINTBYTES] = {
  {133,218,221,213,182,167,220,17,81,165,169,16,157,1,98,146,173,245,154,229,120,122,142,232,80,50,175,61,48,98,60,116},
  {21,137,40,17,147,173,40,217,252,204,13,39,126,161,52,33,103,220,167,227,173,212,171,240,74,133,108,100,110,38,37,203},
  {162,29,197,12,149,155,174,166,249,128,97,252,70,240,197,153,55,135,92,1,67,192,227,81,165,24,200,228,196,179,252,14},
  {99,233,95,90,78,115,131,115,66,215,223,175,169,79,14,163,151,0,237,8,26,122,160,89,64,3,106,116,214,252,226,227},
  {54,150,206,203,58,110,225,157,183,86,161,217,21,173,52,118,210,103,209,184,244,215,140,213,108,38,253,214,220,68,59,21},
  {229,150,16,70,160,155,23,143,193,204,18,207,12,35,16,109,19,161,33,163,231,148,93,33,192,8,208,33,52,245,73,118},
  {73,105,152,67,21,142,191,92,30,165,214,112,236,97,191,160,115,199,32,54,253,64,12,183,200,153,220,13,247,122,168,120},
  {154,10,210,253,121,116,76,235,128,102,47,92,73,216,196,148,103,90,201,122,11,94,192,85,13,193,122,32,104,107,44,86},
  {242,127,197,101,157,182,135,14,30,144,113,135,145,40,211,42,139,166,32,175,62,68,63,56,125,65,228,23,153,31,9,121},
  {56,11,123,153,186,17,205,235,203,26,210,162,38,100,99,95,184,127,125,207,51,224,32,196,119,210,101,197,184,231,245,122},
  {36,227,145,67,153,167,149,57,231,243,241,68,24,56,226,208,21,225,45,44,253,75,155,98,34,41,113,202,172,245,217,59},
  {200,209,38,54,107,155,162,213,139,66,104,33,240,22,163,55,57,117,215,169,210,36,216,146,41,231,244,195,139,121,254,0},
  {0,247,38,106,217,100,0,75,146,156,162,91,151,191,98,7,50,138,182,191,206,5,104,123,113,39,192,119,91,70,133,44},
  {24,71,171,234,213,78,8,17,237,82,105,158,41,52,68,101,207,165,51,142,135,25,110,107,184,104,200,119,68,173,59,119},
  {217,153,48,91,233,138,209,224,28,163,218,7,29,45,183,133,213,145,116,178,133,212,17,102,130,112,139,93,187,16,48,11},
  {233,141,127,176,161,169,81,186,69,119,8,7,219,207,140,141,179,217,154,20,74,236,120,151,133,9,40,11,248,100,171,61},
  {143,235,242,112,234,244,127,42,130,112,146,216,216,118,207,241,18,54,165,57,153,36,51,254,49,158,71,172,234,86,30,115},
  {119,237,4,68,155,203,157,8,39,68,130,178,241,153,84,205,9,184,30,234,214,64,139,141,221,0,208,205,160,208,206,122},
  {230,126,124,186,28,168,134,3,88,238,75,227,231,171,125,245,132,57,18,181,223,229,2,189,221,242,88,163,213,64,50,104},
  {146,242,159,166,200,201,150,160,60,217,52,208,122,192,206,216,213,254,39,88,137,139,18,78,111,220,114,219,129,119,123,67},
  {90,86,188,252,38,122,3,18,104,33,60,199,75,240,95,203,122,59,34,63,90,181,216,112,120,252,142,38,100,73,219,51},
  {208,111,18,67,63,233,46,43,98,19,222,177,223,200,241,217,152,121,123,245,214,32,222,249,73,169,177,60,12,248,55,158},
  {195,200,243,197,196,207,227,141,88,22,134,33,16,234,171,107,21,159,157,201,96,21,72,205,31,234,131,81,182,213,113,64},
  {71,47,120,45,240,125,236,219,24,118,182,126,40,128,132,87,0,8,136,25,211,29,205,193,196,82,219,215,30,225,165,85},
  {6,120,14,54,131,140,165,197,128,126,193,206,138,103,77,167,127,12,178,66,21,27,149,46,219,110,179,232,1,3,10,96},
  {170,173,18,125,183,4,162,58,4,108,184,211,64,98,80,11,14,189,137,1,113,91,228,142,135,109,168,37,4,185,110,41},
  {239,168,117,46,90,253,251,53,129,85,95,19,196,0,198,236,179,159,6,5,138,155,31,208,30,203,63,42,92,176,246,159},
  {70,90,116,158,218,216,103,205,236,149,206,168,77,49,223,238,123,157,44,221,113,190,218,163,15,167,193,24,64,230,200,32},
  {47,28,67,32,72,218,215,211,41,169,231,221,225,158,209,123,45,150,20,202,129,47,66,198,156,164,232,193,166,228,55,98},
  {133,255,102,184,241,33,53,161,64,165,61,128,174,250,60,25,164,76,242,30,148,21,89,53,205,101,155,189,203,217,87,85},
  {58,208,59,76,188,233,160,118,158,60,166,149,188,13,26,154,116,64,194,48,140,179,59,4,238,206,210,89,93,109,139,32},
  {73,62,203,112,176,134,155,69,135,47,30,173,28,43,178,113,188,188,231,27,172,122,96,99,236,242,253,109,210,254,20,16},
  {12,97,67,237,42,207,179,144,2,227,167,226,3,28,166,216,220,242,8,29,166,64,216,106,83,71,185,169,242,153,49,17},
  {93,96,122,15,112,17,211,233,152,205,188,181,90,160,82,225,19,105,186,83,107,89,227,115,56,108,105,11,79,53,207,63},
  {239,87,246,243,207,185,86,248,217,30,54,240,27,254,167,204,120,202,154,90,202,122,61,59,230,89,22,167,42,11,98,45},
  {80,122,179,178,62,164,3,194,112,28,39,23,252,192,155,80,162,6,223,44,24,14,33,18,104,83,41,135,41,109,17,224},
  {56,182,76,24,217,7,54,70,242,91,148,57,174,212,99,231,177,247,71,183,247,38,3,12,249,130,68,7,39,67,189,143},
  {160,69,77,238,221,238,44,234,198,74,237,123,188,231,251,245,2,84,48,87,44,90,108,105,8,157,246,65,156,58,195,57},
  {115,114,240,204,123,58,219,165,106,184,23,78,196,223,139,214,11,17,77,10,195,244,194,41,207,226,246,221,66,68,222,62},
  {17,27,90,95,72,5,219,79,198,96,140,229,7,19,105,220,31,44,10,189,228,142,143,175,103,37,93,43,100,177,222,150},
  {10,2,138,157,12,244,25,125,236,19,139,240,232,244,16,156,12,161,228,81,57,78,67,210,129,29,113,210,20,106,198,226},
  {240,222,146,238,207,103,248,61,137,145,17,6,76,137,189,125,12,79,213,118,195,80,109,29,74,38,106,28,157,152,206,110},
  {35,213,178,187,124,140,73,101,93,210,200,86,179,158,164,210,215,156,54,180,216,243,29,26,235,227,26,142,109,206,108,112},
  {193,217,128,217,238,154,50,186,70,148,167,93,206,43,227,92,253,237,69,30,135,11,132,104,186,147,79,11,193,235,158,85},
  {139,112,244,194,138,153,217,230,143,225,97,150,194,143,167,165,75,20,121,127,220,4,195,150,232,91,181,219,31,78,26,37},
  {142,40,63,66,176,219,127,245,3,253,178,123,88,91,146,185,21,183,120,70,76,69,217,33,34,177,86,96,126,114,27,3},
  {103,129,227,106,53,41,239,32,158,10,78,37,130,41,79,69,183,208,103,118,89,251,74,175,236,209,163,52,221,35,157,31},
  {249,27,137,39,161,15,133,187,99,132,151,171,146,22,39,79,38,151,254,224,252,144,151,55,224,57,37,209,206,32,240,194},
  {133,185,231,141,84,179,78,90,88,65,9,167,25,154,109,95,66,91,94,230,249,234,239,128,77,134,211,97,31,163,247,38},
  {195,234,12,244,247,159,43,89,154,207,87,129,9,174,86,91,49,14,72,158,133,4,10,63,132,217,83,9,59,118,139,19},
  {133,31,247,111,200,212,191,254,182,231,194,23,126,28,147,124,91,165,243,237,143,121,211,16,47,7,174,139,23,104,56,47},
  {100,164,52,85,247,129,172,141,125,67,177,226,248,185,123,176,80,99,233,160,164,225,197,207,250,18,72,121,10,201,34,14},
  {252,139,231,164,188,7,92,122,16,244,213,166,136,81,235,88,209,136,101,105,120,96,170,54,22,136,203,159,77,117,81,154},
  {143,242,153,38,70,0,130,0,67,98,254,148,131,85,127,154,165,67,196,69,56,101,209,108,105,196,117,70,142,241,152,78},
  {52,224,221,192,111,110,162,155,102,66,47,166,37,90,240,129,107,78,51,255,22,163,195,220,215,169,174,127,144,94,226,201},
  {19,129,58,16,91,112,212,175,101,238,120,14,118,215,72,122,168,243,103,14,5,155,120,167,91,165,171,143,43,209,191,81},
  {23,155,224,66,163,85,106,226,131,22,240,140,147,193,178,204,47,129,185,94,6,212,43,215,172,31,181,7,21,234,243,236},
  {219,197,41,136,68,150,118,164,141,38,74,12,215,111,202,130,136,47,4,28,146,148,224,65,117,210,26,200,213,112,74,14},
  {168,9,245,4,218,176,115,72,11,204,7,190,161,49,137,247,5,27,179,115,230,95,14,158,68,30,227,132,125,153,20,105},
  {19,6,106,157,254,10,36,27,66,74,133,218,136,144,135,125,66,231,33,43,150,176,158,159,4,122,81,126,56,97,66,73},
  {162,141,255,162,40,108,188,125,233,10,145,159,166,172,147,218,231,202,74,84,140,122,134,231,252,137,85,102,193,57,244,28},
  {94,102,107,210,115,6,137,18,14,220,220,154,102,44,152,50,112,163,169,91,102,236,176,18,209,18,66,128,188,112,213,122},
  {40,34,94,183,173,244,46,30,228,227,7,34,147,79,57,140,16,211,131,33,164,143,86,13,59,79,3,126,221,12,28,7},
  {88,211,7,25,201,88,168,130,72,20,154,92,7,71,239,4,46,12,86,203,211,97,229,221,126,224,228,156,234,121,95,234},
  {164,93,249,139,221,192,154,53,53,4,75,103,251,134,100,22,176,210,193,110,41,152,7,181,183,242,116,87,93,105,63,103},
  {78,17,74,143,171,215,95,164,212,137,254,178,80,127,13,45,218,255,252,69,223,52,164,135,35,235,86,167,242,168,236,103},
  {238,71,34,225,188,4,13,26,144,70,157,250,182,111,168,59,200,155,227,247,34,54,218,28,163,8,138,72,214,170,189,205},
  {205,244,204,202,186,74,88,247,237,46,10,63,190,73,201,18,80,21,131,2,27,114,134,253,136,114,45,238,195,219,180,95},
  {207,105,197,117,253,71,103,227,138,245,193,163,168,77,226,89,15,205,65,110,249,119,140,15,0,216,133,172,219,11,12,78},
  {18,244,198,188,213,166,93,14,150,4,96,98,118,116,209,211,231,126,9,244,38,88,106,37,193,144,221,115,240,252,81,111},
  {165,90,179,130,204,218,247,189,238,129,95,131,245,206,6,87,236,171,29,181,84,203,58,135,151,5,105,151,216,202,1,74},
  {133,92,158,60,21,108,200,188,216,157,11,100,96,200,221,117,219,48,228,176,97,62,104,115,183,167,21,42,129,98,38,186},
  {20,237,110,80,59,209,61,56,111,148,161,62,94,73,71,11,136,115,27,106,35,245,9,34,242,155,207,61,54,203,109,48},
  {253,54,15,90,187,233,161,252,143,101,101,38,34,132,111,199,94,15,122,119,75,57,99,36,225,104,27,222,80,204,86,56},
  {28,105,254,10,231,115,112,55,241,180,78,217,105,125,154,196,34,237,67,69,158,58,184,109,75,145,66,138,183,152,79,114},
  {106,64,102,69,75,76,222,173,193,40,48,164,165,224,27,134,45,156,207,131,71,218,255,75,233,205,19,222,200,63,137,110},
  {51,0,154,92,203,162,128,110,60,210,148,225,64,147,104,76,53,214,226,119,105,175,147,176,189,36,196,48,204,125,140,74},
  {173,63,99,222,178,169,165,115,226,108,38,97,159,224,238,158,102,213,137,254,244,132,140,168,205,19,207,181,159,165,194,87},
  {142,19,103,213,195,194,97,250,148,224,75,200,156,48,183,123,204,31,244,92,207,162,85,25,191,192,177,25,22,58,95,124},
  {233,95,239,159,15,181,173,37,243,124,28,216,129,180,0,151,91,112,141,36,220,146,17,124,19,178,132,24,234,162,36,244},
  {235,171,214,149,185,117,9,254,75,198,158,20,118,39,247,14,72,254,101,241,228,92,233,245,163,79,202,89,154,38,132,29},
  {214,216,103,27,218,87,176,96,72,144,132,217,160,62,190,195,11,199,232,164,81,188,169,152,35,134,228,11,185,203,32,73},
  {147,107,197,226,199,221,124,115,1,151,31,97,7,191,23,89,121,172,248,61,81,73,56,254,34,100,135,233,166,53,221,186},
  {243,66,247,66,62,57,139,168,183,216,236,87,204,55,230,94,176,175,28,253,21,22,179,98,36,80,46,193,32,75,176,221},
  {66,98,79,170,54,172,253,63,39,103,7,48,86,182,100,69,160,157,35,220,78,33,201,243,81,165,193,173,74,107,65,111},
  {63,109,129,80,49,251,201,41,111,80,204,4,32,17,72,57,245,212,128,171,163,29,223,218,225,178,156,90,41,231,63,50},
  {165,218,225,122,152,34,207,72,61,226,193,169,80,241,112,239,38,233,231,207,178,54,206,212,133,125,35,105,102,123,35,78},
  {129,14,10,182,22,250,85,228,111,244,59,106,178,39,85,228,194,153,252,106,152,225,13,35,178,228,16,124,177,11,144,124},
  {15,156,131,174,171,245,73,196,50,113,21,84,229,117,131,19,240,26,40,194,169,109,75,243,132,163,33,178,59,115,34,35},
  {167,150,91,222,34,202,202,158,222,85,128,125,225,140,250,177,131,111,252,19,78,156,30,206,120,120,33,191,222,164,107,115},
  {157,28,212,39,141,145,95,162,161,34,193,186,160,255,21,46,4,123,71,62,139,70,253,180,92,58,33,23,147,100,24,51},
  {32,4,56,86,178,184,215,9,111,109,135,216,176,215,186,144,237,223,114,178,34,90,4,207,191,96,29,51,205,35,111,76},
  {225,190,105,124,86,25,110,82,29,88,218,55,207,70,14,99,173,57,30,203,18,85,102,59,145,14,76,243,120,196,138,86},
  {182,234,44,163,47,9,78,52,43,3,241,173,22,50,195,210,227,148,226,23,149,227,156,108,210,163,27,67,1,136,37,39},
  {48,224,174,249,239,168,73,98,61,187,13,145,102,138,147,147,14,188,98,66,30,117,0,77,145,146,36,181,80,202,231,6},
  {103,4,29,212,189,81,41,119,43,101,173,124,11,88,197,0,119,233,81,166,249,141,15,95,117,141,185,72,175,241,87,103},
  {27,156,149,160,184,52,147,29,234,37,202,73,115,189,28,177,197,146,89,15,96,199,5,64,169,222,219,220,171,226,66,248},
  {1,24,59,4,101,181,100,139,204,31,91,210,250,247,205,54,207,76,172,109,87,165,124,71,54,10,195,97,56,12,172,119},
  {148,212,155,105,22,42,217,97,82,59,154,179,74,106,70,86,158,48,249,1,47,163,225,178,78,55,115,221,50,22,43,58},
  {225,34,160,192,183,237,222,62,18,34,101,94,193,177,206,74,24,97,106,197,225,78,237,65,51,252,187,109,92,29,202,129},
  {139,196,209,44,209,44,206,240,225,210,83,122,92,29,82,49,201,107,25,98,252,160,170,19,93,176,110,173,57,73,137,180},
  {200,68,80,223,3,67,208,248,155,67,114,26,214,120,251,203,153,130,112,48,196,193,96,204,125,162,51,125,191,190,48,61},
  {162,0,28,103,136,199,40,69,106,172,107,46,177,130,171,227,6,191,227,196,211,59,112,31,127,243,230,75,168,13,19,92},
  {181,83,207,199,72,216,70,141,103,75,12,215,43,233,19,173,128,44,64,109,76,20,22,167,24,248,77,171,213,137,180,56},
  {71,119,22,251,156,223,174,133,159,200,189,175,198,175,37,249,90,163,106,137,219,152,188,26,107,104,148,212,4,101,234,212},
  {97,191,64,101,220,164,47,33,92,169,162,204,26,9,86,248,55,187,250,75,234,108,64,1,193,167,59,166,224,103,184,207},
  {76,27,91,200,170,63,179,215,123,123,223,230,21,31,9,240,212,176,243,58,224,151,208,131,227,132,237,68,13,224,219,148},
  {80,193,160,250,22,205,105,190,55,82,214,175,175,117,25,132,128,10,117,160,3,237,122,84,219,126,65,17,178,107,51,33},
  {88,132,109,9,176,42,17,99,143,109,179,197,8,51,55,60,45,32,11,228,153,51,203,230,238,12,9,134,16,146,75,126},
  {9,59,236,159,88,156,60,215,65,29,144,206,180,240,221,133,71,146,251,75,252,22,188,79,124,76,59,55,152,30,120,93},
  {191,249,157,232,214,180,112,120,128,229,97,155,200,139,7,30,132,28,110,105,27,93,58,81,241,138,52,156,75,38,170,223},
  {193,163,64,210,46,2,8,101,66,90,202,139,180,114,144,167,116,154,10,162,29,28,70,190,121,65,247,181,59,221,95,10},
  {161,100,179,27,120,46,131,106,156,182,96,12,244,230,127,114,131,43,131,19,163,133,110,90,144,253,114,178,96,133,93,5},
  {128,238,13,127,53,21,194,18,125,63,187,117,97,223,163,170,16,44,103,209,192,127,158,79,25,214,13,249,222,171,186,65},
  {71,86,74,9,116,177,179,72,49,12,187,225,75,169,228,26,94,248,217,152,21,193,203,221,238,197,40,170,162,145,195,184},
  {96,77,52,193,212,255,140,137,183,101,150,16,239,128,116,134,165,76,199,185,49,200,211,176,137,163,66,104,2,14,220,110},
  {251,122,111,246,120,208,239,166,211,47,180,228,135,106,178,161,15,90,163,79,210,199,245,48,125,114,70,136,15,51,123,43},
  {254,56,122,215,227,189,131,240,213,75,243,96,40,48,216,14,61,70,218,222,28,25,28,112,138,222,58,39,139,105,208,188},
  {221,144,122,161,220,168,63,59,177,157,151,203,105,57,134,223,243,13,30,44,194,10,1,192,48,233,52,177,105,254,108,37},
  {78,244,242,41,180,182,171,231,130,180,254,93,212,239,161,66,35,163,55,159,190,81,232,197,205,60,66,192,146,71,126,210},
  {178,209,37,76,92,190,183,6,38,251,155,51,255,151,33,71,104,89,232,137,138,125,206,77,17,154,32,61,166,217,74,76},
  {191,10,24,120,44,179,136,225,116,92,171,204,31,30,102,250,50,117,22,18,136,83,30,123,36,38,207,185,206,28,234,141},
  {58,181,135,41,98,14,200,55,50,69,113,145,130,232,245,132,1,147,73,34,113,192,103,105,0,250,245,167,128,228,122,23},
  {10,72,125,214,110,45,3,118,32,248,245,177,46,193,190,2,48,119,133,76,133,151,170,178,95,229,20,25,94,227,144,32},
  {183,207,141,110,147,213,44,151,129,41,25,148,84,127,165,255,122,216,43,166,238,19,240,81,44,68,20,188,167,16,179,83},
  {63,43,52,114,8,20,123,139,103,128,106,211,137,227,227,93,175,5,45,53,98,253,244,166,11,210,188,150,201,99,173,5},
  {6,174,70,114,4,103,88,251,52,214,179,58,36,214,175,226,215,13,97,187,25,122,13,188,214,181,191,132,225,111,28,210},
  {17,207,26,138,16,164,197,241,107,166,102,45,23,192,169,209,69,89,215,28,123,239,163,166,213,162,143,19,18,4,128,96},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,162},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,230},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,162},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,230},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,162},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,230},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,162},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,230},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,162},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,230},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,162},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,230},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,162},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,230},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {88,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102,102},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,163,248,106,174,70,95,14,86,81,56,100,81,15,57,151,86,31,162,201,232,94,162,29,194,41,35,9,243,205,96,34},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
} ;

static const unsigned char precomputed_nG_merged25519_n[precomputed_nG_merged25519_NUM][crypto_nG_SCALARBYTES] = {
  {14,252,177,48,51,24,150,255,44,250,132,235,147,239,78,66,173,190,143,74,31,10,84,177,152,242,102,165,15,106,180,143},
  {4,98,90,244,250,174,214,153,18,248,206,99,12,173,215,101,111,19,140,120,200,225,137,93,10,154,215,119,183,184,110,67},
  {26,90,61,26,108,30,1,123,69,213,78,134,230,158,104,207,14,110,188,73,111,89,39,23,114,21,175,51,18,34,172,225},
  {40,23,118,110,26,89,64,207,170,236,133,185,238,148,225,2,118,128,234,122,43,160,205,8,71,218,49,136,251,19,2,82},
  {193,46,183,245,21,125,231,201,156,66,212,74,156,49,235,176,118,234,219,138,128,194,100,66,242,63,179,4,64,172,10,185},
  {10,7,143,232,161,214,197,102,31,170,6,0,81,126,210,18,101,96,18,149,227,110,205,192,95,127,78,212,117,99,195,227},
  {97,3,128,29,30,130,136,227,27,41,200,232,149,155,85,88,165,150,242,99,80,35,129,101,113,59,149,211,77,30,75,178},
  {106,169,193,176,237,39,174,67,198,73,118,162,37,236,106,78,45,67,20,98,141,248,56,249,185,128,248,126,74,155,171,214},
  {137,33,93,180,100,175,164,180,175,113,146,77,71,201,157,232,74,69,159,109,245,3,51,44,39,14,155,211,55,34,16,27},
  {129,241,79,112,4,141,121,80,104,172,222,222,108,215,166,191,18,54,187,139,89,29,71,91,98,103,167,160,72,42,77,162},
  {248,184,56,147,163,223,180,173,66,182,20,230,145,143,177,100,65,10,32,55,127,20,137,122,228,85,77,171,245,28,137,214},
  {192,35,128,83,162,109,231,202,199,158,130,36,132,214,107,77,69,130,37,98,42,1,138,9,249,184,64,213,118,109,188,141},
  {97,207,224,138,5,184,83,176,42,139,86,52,44,49,3,197,167,220,73,214,54,86,97,21,176,173,2,51,189,184,163,22},
  {56,98,7,141,181,187,179,200,10,182,204,190,168,129,189,32,44,119,77,78,218,191,112,5,67,87,84,50,133,102,245,18},
  {164,117,126,223,222,210,183,203,165,89,96,90,92,52,180,71,137,243,235,34,42,247,179,157,221,184,215,122,42,77,19,148},
  {37,20,118,112,165,202,61,122,229,92,171,192,202,22,253,197,79,109,40,131,114,128,218,129,178,49,179,254,218,218,37,147},
  {228,139,29,124,221,239,86,110,242,200,229,137,171,165,136,144,62,238,2,156,164,202,112,170,25,170,222,164,60,38,172,251},
  {15,188,181,84,79,123,28,96,218,82,147,215,19,139,191,255,171,27,45,165,24,25,233,142,225,76,57,60,188,116,151,63},
  {68,205,99,39,157,39,232,121,65,138,14,209,106,34,53,93,228,125,167,202,33,116,203,133,224,96,179,158,7,219,76,208},
  {146,70,128,239,133,16,48,130,169,201,83,140,118,113,39,26,174,164,250,46,127,84,154,254,213,186,121,88,183,134,34,53},
  {188,20,82,254,10,0,82,194,233,195,12,205,100,107,3,45,38,220,109,116,141,32,25,94,57,182,26,22,49,115,11,224},
  {247,146,8,90,8,199,41,31,163,112,106,211,122,242,184,170,18,98,204,153,159,229,79,111,7,232,42,88,7,163,87,55},
  {114,73,62,161,254,9,21,14,233,92,252,67,128,17,118,236,147,107,117,46,236,71,147,133,124,155,254,161,88,97,5,200},
  {117,189,86,19,188,223,94,108,110,96,152,163,238,14,215,95,249,3,35,246,180,115,139,215,204,210,174,17,27,225,146,148},
  {105,44,234,39,55,66,111,237,198,40,48,255,113,86,248,146,42,107,85,53,126,173,135,195,83,124,223,150,49,42,214,231},
  {164,33,179,146,61,72,144,115,145,255,13,173,230,213,204,172,46,179,219,180,17,115,213,208,164,190,252,45,2,121,87,67},
  {54,168,211,156,169,144,89,156,75,38,141,222,162,116,177,116,22,176,16,39,129,67,110,18,174,149,183,208,22,80,243,60},
  {62,224,81,23,101,197,155,131,180,224,124,147,140,157,182,200,221,142,62,41,87,49,32,137,192,120,40,138,150,172,172,17},
  {242,196,171,22,237,29,23,48,203,78,97,43,51,177,33,41,173,144,211,164,204,185,200,115,27,200,20,223,153,136,23,152},
  {46,118,218,21,6,58,150,225,219,27,183,159,86,243,238,102,167,82,148,176,235,100,137,93,140,190,69,205,42,59,132,137},
  {107,168,181,208,37,162,103,49,30,13,181,90,116,182,204,161,210,253,141,95,240,205,240,165,69,113,238,29,210,3,249,255},
  {64,78,238,224,233,71,9,219,30,166,238,145,142,135,243,131,103,7,83,232,183,239,99,65,25,171,26,38,93,253,194,162},
  {127,26,200,57,169,2,77,216,149,78,53,110,25,175,50,233,26,43,78,67,243,152,142,159,88,195,163,197,134,251,85,202},
  {234,161,205,240,101,129,58,120,2,203,55,111,131,137,23,245,216,239,61,14,186,84,2,81,92,183,101,93,33,34,103,225},
  {171,190,230,107,212,78,249,130,216,145,106,42,198,203,178,238,76,62,101,159,59,39,51,201,169,89,122,92,124,212,24,250},
  {142,200,252,229,150,10,123,159,206,168,81,231,16,240,92,236,28,144,210,217,0,173,112,127,136,187,15,111,150,138,109,119},
  {210,185,232,239,74,58,15,215,244,15,186,140,243,94,6,166,138,80,213,191,7,4,200,44,138,232,124,15,70,166,80,60},
  {208,70,173,227,58,128,247,7,158,114,44,134,37,236,155,193,175,48,1,24,28,247,142,145,47,197,25,148,3,252,167,179},
  {241,50,50,190,236,125,117,62,114,248,185,248,121,110,167,101,7,19,95,47,189,128,92,51,191,197,203,21,245,254,56,242},
  {8,35,126,152,221,126,219,224,48,87,27,139,201,175,66,173,138,113,5,73,26,181,127,228,40,221,169,219,238,57,93,43},
  {166,222,31,175,196,76,20,78,132,242,56,61,146,97,155,109,209,170,216,48,130,119,160,84,86,85,188,207,77,30,140,126},
  {251,246,135,3,126,6,236,29,185,3,148,209,173,16,2,242,27,110,212,137,5,171,125,180,154,52,232,126,34,220,226,177},
  {122,189,106,75,10,100,25,87,235,251,121,142,37,54,45,38,27,156,146,187,235,146,11,63,196,248,43,228,239,152,176,150},
  {249,68,234,200,175,106,31,122,79,107,167,10,37,124,170,233,66,5,127,233,5,108,234,71,191,189,120,252,248,164,23,146},
  {240,8,108,24,44,3,72,203,70,205,254,161,212,43,190,211,30,58,80,191,231,230,149,33,102,225,39,78,169,87,28,231},
  {95,160,182,6,202,134,90,35,53,96,136,211,134,171,148,211,118,251,48,95,248,21,243,174,83,117,207,18,150,221,29,28},
  {46,95,36,196,148,52,40,77,79,83,152,45,161,99,13,111,188,217,182,203,240,81,177,154,63,31,98,65,11,142,197,24},
  {152,102,154,216,31,251,97,187,236,3,80,121,124,92,13,244,145,112,85,148,115,233,176,177,63,64,130,68,166,15,242,108},
  {47,157,164,241,25,123,61,95,97,4,76,139,183,126,27,22,244,89,189,11,230,254,90,189,241,163,249,126,236,10,78,210},
  {226,212,119,253,81,239,100,23,38,101,72,87,105,45,253,60,103,45,35,156,37,137,251,60,163,35,9,106,16,154,45,119},
  {57,101,29,22,86,253,64,71,187,107,116,30,238,144,148,48,0,227,142,231,144,235,31,81,176,45,10,93,225,205,164,242},
  {193,60,149,62,156,153,43,108,5,77,156,134,113,145,113,111,47,126,75,27,129,110,204,69,79,155,127,123,13,191,43,221},
  {40,252,175,146,145,151,124,167,228,175,139,42,158,118,218,147,194,36,175,191,70,184,100,168,251,241,12,121,124,116,150,53},
  {137,53,41,50,33,208,134,171,193,107,122,236,87,44,133,159,235,80,141,15,54,51,242,62,212,89,171,147,200,10,135,140},
  {52,173,167,245,97,13,31,36,106,202,38,234,75,71,153,19,71,218,63,248,155,14,202,32,206,84,144,180,241,224,180,33},
  {130,236,212,243,13,80,154,170,130,215,24,193,141,101,101,168,215,153,194,150,96,153,173,27,108,9,111,197,60,7,172,217},
  {39,24,96,237,13,251,124,122,234,58,1,166,176,139,228,74,91,191,18,74,25,117,123,119,139,1,176,179,130,133,200,49},
  {135,108,110,216,118,255,247,110,14,250,1,70,15,227,48,188,238,144,45,58,31,18,45,23,220,103,197,59,35,188,183,243},
  {59,60,133,102,147,72,54,52,175,218,110,6,20,238,236,143,221,118,254,225,212,21,86,202,90,58,152,199,48,197,95,182},
  {51,38,100,77,150,15,128,241,49,163,156,43,131,210,175,131,148,239,188,253,239,68,38,174,91,253,231,127,56,77,194,119},
  {43,75,147,223,200,128,246,99,53,144,97,248,156,143,22,49,63,52,25,172,135,115,71,208,209,169,242,131,232,185,9,159},
  {72,100,21,171,53,71,79,134,164,152,237,173,26,184,165,153,158,133,172,216,74,232,190,206,30,247,0,133,14,153,102,60},
  {157,38,1,210,164,205,77,143,52,65,73,133,72,122,248,162,212,199,0,145,19,35,54,117,137,14,135,105,63,81,144,235},
  {253,3,6,26,136,2,66,96,106,120,74,93,73,149,57,185,179,34,114,254,238,89,214,249,89,134,63,107,246,153,54,65},
  {196,127,157,234,203,32,233,90,251,147,116,243,54,34,152,23,113,128,143,41,35,171,220,236,200,29,59,57,81,216,242,46},
  {189,88,70,51,3,225,206,243,44,236,56,56,63,27,10,70,38,118,146,59,244,21,230,106,99,190,212,77,221,42,67,119},
  {96,67,95,208,121,23,46,57,171,108,102,161,216,172,175,73,159,168,123,35,51,75,193,132,253,144,246,222,228,201,23,120},
  {80,100,100,236,81,142,247,64,196,4,194,165,131,253,155,207,59,201,241,174,81,150,188,140,106,66,123,104,218,67,145,225},
  {233,72,18,18,154,96,44,46,69,98,137,231,214,88,100,165,3,75,237,52,80,33,116,228,43,128,227,6,227,217,76,187},
  {43,62,58,85,209,87,135,23,116,39,173,126,145,132,233,35,14,143,64,228,200,245,115,249,221,87,246,141,46,248,100,149},
  {64,158,173,19,54,201,125,205,64,105,92,171,243,239,180,235,148,208,199,169,144,13,196,138,184,75,87,249,102,183,126,43},
  {19,113,230,38,129,48,147,11,124,8,111,128,153,85,193,126,61,154,198,200,239,194,164,107,68,141,42,46,56,69,33,98},
  {24,83,121,203,216,158,47,158,106,131,35,241,108,240,206,28,173,12,170,244,188,196,51,234,225,185,32,12,113,236,151,236},
  {248,159,134,88,110,15,245,200,135,10,19,194,156,195,9,247,186,157,233,66,255,55,66,40,70,31,13,43,11,111,10,230},
  {115,32,32,108,246,168,201,224,61,168,100,219,252,25,55,139,127,219,33,52,24,76,69,66,108,103,119,186,96,134,30,172},
  {231,137,251,196,111,109,126,243,38,184,15,33,198,220,208,33,59,108,93,74,212,123,38,136,118,183,19,173,181,143,188,221},
  {110,142,138,114,250,226,80,42,161,22,196,158,214,195,61,114,158,160,103,242,24,249,73,36,79,200,62,23,67,249,127,233},
  {239,172,33,52,11,26,217,131,181,27,143,107,112,67,36,57,94,240,68,167,75,36,103,79,98,132,210,155,149,60,209,161},
  {6,217,152,202,221,143,3,38,149,101,61,144,40,23,137,233,163,16,12,229,171,154,216,148,14,205,132,185,238,71,249,112},
  {6,135,247,173,18,176,22,216,166,110,161,143,141,200,221,129,42,79,83,173,174,132,231,147,232,175,117,130,2,31,93,14},
  {206,30,35,121,193,167,76,209,105,93,144,225,202,208,155,182,114,69,152,249,191,40,241,150,74,237,103,141,45,134,49,150},
  {145,47,221,170,77,40,173,100,195,78,80,9,96,127,171,225,209,30,146,84,132,80,124,73,136,250,118,185,99,198,43,199},
  {205,249,166,148,75,51,38,174,72,78,240,79,133,244,142,19,70,159,50,81,53,205,71,30,104,21,48,115,35,67,138,109},
  {117,117,96,45,99,75,192,252,46,168,191,224,36,237,231,142,215,231,57,205,200,223,55,199,229,183,115,121,31,33,190,65},
  {193,32,91,37,76,201,20,214,211,232,4,199,153,114,132,79,182,60,248,77,231,125,36,85,96,131,88,21,129,8,12,254},
  {173,240,96,2,195,99,177,226,68,136,188,49,84,64,250,149,78,173,55,99,107,69,7,98,140,191,148,157,137,64,57,253},
  {119,134,61,169,0,75,34,229,11,132,226,236,8,126,158,37,140,35,100,188,235,250,43,92,241,191,144,35,62,158,192,194},
  {92,225,6,97,141,108,210,197,69,239,217,18,245,137,32,30,244,109,38,198,48,32,13,219,142,47,192,78,4,86,66,146},
  {75,67,250,11,172,214,74,39,6,155,217,211,40,248,114,121,85,162,145,159,43,211,53,203,221,189,68,127,205,54,116,114},
  {20,207,210,24,33,88,119,199,31,61,106,27,94,223,248,77,172,83,39,188,217,206,87,250,252,38,206,58,2,113,194,248},
  {77,228,48,160,181,122,36,55,119,132,115,250,186,94,158,181,32,107,7,234,127,48,192,30,17,88,200,152,147,12,19,139},
  {0,85,143,145,217,163,110,101,94,145,23,188,73,70,35,131,56,156,39,40,139,38,11,113,24,155,9,1,103,240,219,86},
  {202,103,195,152,83,118,152,164,128,218,18,106,71,167,100,220,1,129,233,117,55,58,2,173,24,163,2,68,28,27,161,123},
  {162,179,114,79,63,57,44,81,193,33,56,180,102,58,185,3,70,217,121,210,171,203,151,18,81,228,82,143,252,240,97,232},
  {106,112,23,50,29,210,39,79,57,12,122,158,177,133,83,96,21,12,240,106,115,15,239,159,86,17,166,198,166,144,68,131},
  {242,83,31,132,61,78,147,229,73,88,255,37,96,242,142,208,96,30,12,65,15,166,34,212,185,19,37,196,234,33,166,151},
  {6,103,172,137,117,164,200,126,96,32,78,249,8,9,121,182,225,39,158,28,170,212,240,198,33,82,172,221,48,191,250,100},
  {241,165,131,211,163,174,119,205,198,110,254,246,41,123,207,207,233,245,144,31,180,78,131,233,236,243,226,255,93,157,226,206},
  {18,32,225,235,121,109,146,205,229,146,24,56,24,233,22,126,111,33,133,159,249,31,175,203,2,41,57,167,207,53,236,206},
  {228,182,61,134,80,248,152,169,144,228,86,31,210,36,187,230,104,176,123,182,92,161,98,225,228,95,8,126,179,188,115,118},
  {49,236,81,17,2,139,29,237,148,53,47,163,234,48,26,98,132,219,243,15,45,42,104,62,63,171,30,136,14,40,174,109},
  {33,217,207,179,131,181,1,204,249,19,229,30,156,109,111,164,27,182,181,77,81,125,9,245,26,188,57,29,171,9,47,192},
  {236,78,246,110,90,65,156,79,220,93,140,241,38,133,101,190,143,22,91,102,47,73,132,2,22,121,87,44,105,189,228,180},
  {245,221,226,170,131,224,135,73,20,30,235,241,107,173,155,180,184,216,168,105,219,47,44,188,238,195,28,137,94,120,174,91},
  {8,110,37,59,118,72,82,221,109,207,92,19,105,210,153,89,229,172,147,205,252,51,0,133,51,39,92,169,81,25,113,18},
  {51,31,171,63,97,168,57,202,66,43,203,171,187,15,254,191,246,7,131,10,3,57,222,127,141,185,51,175,75,110,249,4},
  {241,246,125,43,103,239,237,44,122,195,143,51,187,182,68,211,122,192,110,133,60,55,181,156,118,151,66,42,142,233,132,34},
  {2,41,17,26,198,119,88,167,221,59,136,122,82,120,126,136,179,22,219,44,233,104,217,7,148,253,9,120,150,203,254,237},
  {125,90,104,48,37,34,117,80,250,44,224,225,239,233,34,113,20,30,116,156,8,71,218,236,40,109,90,225,65,250,159,108},
  {227,109,59,1,58,16,245,87,220,24,30,226,66,198,241,233,56,135,231,193,211,152,219,126,146,145,2,63,45,26,101,128},
  {113,187,8,166,79,70,149,53,130,231,56,237,79,78,172,236,61,72,102,223,207,79,138,186,125,207,95,0,177,15,132,122},
  {93,227,44,20,223,204,200,210,52,230,156,36,182,136,110,83,212,117,127,20,116,192,95,155,246,9,93,2,88,150,91,175},
  {156,159,107,251,13,107,248,203,242,51,231,164,48,88,246,149,122,70,55,125,192,227,47,76,143,7,212,219,249,226,91,20},
  {155,169,171,165,88,143,193,192,229,174,23,49,24,155,232,133,248,158,173,112,162,42,89,62,236,107,89,123,133,124,54,150},
  {26,62,104,111,173,4,224,251,190,230,249,253,27,212,248,115,63,69,100,77,7,29,186,147,98,124,34,222,64,48,49,2},
  {187,42,244,170,51,253,205,98,135,221,225,10,161,197,254,133,41,49,62,82,9,165,246,119,57,111,103,87,234,183,118,119},
  {13,90,122,148,215,139,219,237,185,114,250,70,116,26,1,229,181,21,41,198,156,62,65,116,102,65,232,237,83,231,46,123},
  {173,73,74,2,64,170,144,40,211,161,103,64,184,199,221,15,119,56,55,58,32,155,79,206,222,17,202,243,85,7,249,32},
  {91,105,22,97,41,149,73,204,172,72,190,52,82,126,42,195,82,141,250,205,139,134,76,38,99,191,208,128,76,201,55,231},
  {127,124,129,161,81,20,51,3,237,224,90,78,3,255,236,201,174,25,159,192,35,38,221,205,117,196,123,153,131,97,173,41},
  {235,79,40,53,50,46,91,206,26,218,236,208,101,116,229,44,123,135,18,53,252,116,102,73,71,88,173,160,24,196,88,153},
  {164,254,126,2,194,185,7,81,241,195,17,238,106,157,143,211,114,205,13,51,6,145,23,226,4,20,99,74,71,42,49,39},
  {203,123,250,217,215,118,221,47,96,108,78,133,238,243,192,231,94,7,153,53,191,65,197,139,131,119,1,241,107,153,115,69},
  {127,92,207,211,116,218,76,193,246,23,111,48,131,110,18,170,173,143,73,65,103,209,153,66,51,108,250,130,234,97,143,31},
  {226,156,59,127,152,20,194,27,95,223,50,111,146,27,26,216,73,45,195,208,255,45,214,208,129,172,213,207,4,213,28,172},
  {207,188,210,60,233,151,159,85,160,172,249,57,189,94,226,240,173,18,186,146,117,178,6,183,175,114,227,103,89,124,73,60},
  {164,141,12,199,217,10,241,247,45,93,212,32,193,214,246,251,81,32,178,225,232,253,224,210,26,214,89,22,84,132,155,7},
  {204,196,172,188,163,217,81,173,134,119,178,82,23,75,105,24,147,9,9,245,75,228,228,161,166,124,93,87,233,144,56,163},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
} ;

static void test_nG_merged25519_impl(long long impl)
{
  unsigned char *q = test_nG_merged25519_q;
  unsigned char *n = test_nG_merged25519_n;
  unsigned char *q2 = test_nG_merged25519_q2;
  unsigned char *n2 = test_nG_merged25519_n2;
  long long qlen = crypto_nG_POINTBYTES;
  long long nlen = crypto_nG_SCALARBYTES;

  if (targeti && strcmp(targeti,lib25519_dispatch_nG_merged25519_implementation(impl))) return;
  if (impl >= 0) {
    crypto_nG = lib25519_dispatch_nG_merged25519(impl);
    printf("nG_merged25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_nG_merged25519_implementation(impl),lib25519_dispatch_nG_merged25519_compiler(impl));
  } else {
    crypto_nG = lib25519_nG_merged25519;
    printf("nG_merged25519 selected implementation %s compiler %s\n",lib25519_nG_merged25519_implementation(),lib25519_nG_merged25519_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 512 : 64;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {

      output_prepare(q2,q,qlen);
      input_prepare(n2,n,nlen);
      crypto_nG(q,n);
      checksum(q,qlen);
      output_compare(q2,q,qlen,"crypto_nG");
      input_compare(n2,n,nlen,"crypto_nG");

      double_canary(q2,q,qlen);
      double_canary(n2,n,nlen);
      crypto_nG(q2,n2);
      if (memcmp(q2,q,qlen) != 0) fail("failure: crypto_nG is nondeterministic\n");

      double_canary(q2,q,qlen);
      double_canary(n2,n,nlen);
      crypto_nG(n2,n2);
      if (memcmp(n2,q,qlen) != 0) fail("failure: crypto_nG does not handle n=q overlap\n");
      memcpy(n2,n,nlen);
    }
    checksum_expected(nG_merged25519_checksums[checksumbig]);
  }
  for (long long precomp = 0;precomp < precomputed_nG_merged25519_NUM;++precomp) {
    output_prepare(q2,q,crypto_nG_POINTBYTES);
    input_prepare(n2,n,crypto_nG_SCALARBYTES);
    memcpy(n,precomputed_nG_merged25519_n[precomp],crypto_nG_SCALARBYTES);
    memcpy(n2,precomputed_nG_merged25519_n[precomp],crypto_nG_SCALARBYTES);
    crypto_nG(q,n);
    if (memcmp(q,precomputed_nG_merged25519_q[precomp],crypto_nG_POINTBYTES)) {
      fail("failure: crypto_nG fails precomputed test vectors\n");
      printf("expected q: ");
      for (long long pos = 0;pos < crypto_nG_POINTBYTES;++pos) printf("%02x",precomputed_nG_merged25519_q[precomp][pos]);
      printf("\n");
      printf("received q: ");
      for (long long pos = 0;pos < crypto_nG_POINTBYTES;++pos) printf("%02x",q[pos]);
      printf("\n");
    }
    output_compare(q2,q,crypto_nG_POINTBYTES,"crypto_nG");
    input_compare(n2,n,crypto_nG_SCALARBYTES,"crypto_nG");
  }
}

static void test_nG_merged25519(void)
{
  if (targeto && strcmp(targeto,"nG")) return;
  if (targetp && strcmp(targetp,"merged25519")) return;
  test_nG_merged25519_q = alignedcalloc(crypto_nG_POINTBYTES);
  test_nG_merged25519_n = alignedcalloc(crypto_nG_SCALARBYTES);
  test_nG_merged25519_q2 = alignedcalloc(crypto_nG_POINTBYTES);
  test_nG_merged25519_n2 = alignedcalloc(crypto_nG_SCALARBYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("nG_merged25519 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_nG_merged25519();++impl)
      forked(test_nG_merged25519_impl,impl);
    ++test_nG_merged25519_q;
    ++test_nG_merged25519_n;
    ++test_nG_merged25519_q2;
    ++test_nG_merged25519_n2;
  }
}
#undef crypto_nG_SCALARBYTES
#undef crypto_nG_POINTBYTES

static const char *nG_montgomery25519_checksums[] = {
  "5c8a5d8b32e3d26b33071779ce9191095d7bd4ab3bb6a40b68976e41a98cfc3b",
  "2becc8cd065820fcf82e53a03c5b5235582480fc11d072f2bd15153aebd4e057",
} ;

static void (*crypto_nG)(unsigned char *,const unsigned char *);
#define crypto_nG_SCALARBYTES lib25519_nG_montgomery25519_SCALARBYTES
#define crypto_nG_POINTBYTES lib25519_nG_montgomery25519_POINTBYTES

static unsigned char *test_nG_montgomery25519_q;
static unsigned char *test_nG_montgomery25519_n;
static unsigned char *test_nG_montgomery25519_q2;
static unsigned char *test_nG_montgomery25519_n2;

#define precomputed_nG_montgomery25519_NUM 204

static const unsigned char precomputed_nG_montgomery25519_q[precomputed_nG_montgomery25519_NUM][crypto_nG_POINTBYTES] = {
  {44,101,73,224,192,197,182,193,198,236,100,246,66,1,61,81,166,127,157,110,54,66,251,198,92,55,173,36,249,59,233,126},
  {22,233,216,103,29,176,73,7,154,58,234,156,164,185,58,67,63,98,25,70,163,27,197,131,27,124,136,194,50,127,27,91},
  {11,119,200,123,188,243,40,196,247,76,172,56,18,200,54,108,199,218,66,158,249,60,59,253,46,128,27,35,33,98,140,121},
  {232,150,156,228,44,205,185,197,125,253,134,159,89,140,116,166,52,204,93,160,165,235,252,106,74,24,135,3,220,188,153,18},
  {251,17,37,208,222,241,58,226,178,13,172,0,33,142,148,168,121,36,132,171,245,230,144,7,155,170,104,45,90,96,96,72},
  {42,58,16,107,223,183,177,33,181,110,224,163,48,54,79,177,87,245,128,181,167,50,136,8,250,202,95,210,148,233,40,116},
  {97,239,71,43,224,161,158,90,19,122,241,102,97,255,166,42,96,161,158,9,11,221,175,222,163,228,209,79,49,242,182,119},
  {154,114,106,210,32,53,205,118,189,69,147,82,82,194,188,134,3,234,129,186,111,226,201,222,157,165,205,54,110,139,120,83},
  {255,124,30,66,230,132,210,253,61,154,13,174,253,37,123,239,111,36,77,63,2,122,199,142,34,154,140,2,60,18,193,38},
  {233,203,176,130,48,239,30,31,73,239,234,82,72,96,143,113,37,101,28,17,113,90,196,238,212,68,64,137,165,236,73,32},
  {235,43,183,250,73,96,207,60,226,212,213,174,72,40,62,49,32,238,19,237,100,203,157,20,16,80,17,188,169,199,87,69},
  {201,14,27,106,204,26,136,248,200,36,109,120,34,56,120,195,103,145,198,66,151,130,68,56,51,120,64,116,101,166,148,34},
  {18,17,188,202,61,40,175,192,37,198,66,1,209,60,129,254,194,76,254,181,30,207,172,187,242,254,97,118,71,131,64,121},
  {37,224,252,164,235,13,240,252,189,252,60,188,200,216,143,15,252,125,135,25,180,38,248,255,215,211,197,207,205,250,234,6},
  {109,65,57,126,39,59,194,55,244,212,93,233,113,222,214,19,240,116,36,152,0,118,101,227,49,251,93,111,143,23,201,29},
  {140,101,22,137,6,44,213,100,234,203,208,180,139,115,83,102,43,195,100,183,105,9,246,30,25,84,161,100,50,117,182,83},
  {203,46,169,217,216,136,69,179,134,217,182,110,81,152,110,71,18,222,44,153,11,72,221,54,218,73,175,50,62,227,189,105},
  {128,93,5,190,107,222,19,204,217,12,113,236,162,176,166,25,251,40,252,164,171,43,90,44,6,77,20,164,154,63,158,7},
  {9,218,176,210,206,47,135,144,154,202,245,156,198,134,209,172,52,35,134,21,218,42,99,135,81,134,110,255,138,126,248,34},
  {8,3,202,49,25,245,106,49,116,110,42,158,86,222,129,254,168,26,255,3,247,11,249,170,190,211,250,155,24,150,192,41},
  {228,227,141,214,206,35,141,24,202,113,127,15,243,143,213,65,42,116,208,171,234,179,152,66,10,169,171,57,71,192,159,85},
  {207,123,193,30,219,178,11,72,105,117,4,65,4,162,57,212,54,199,105,198,19,197,31,67,143,39,113,74,202,173,159,91},
  {126,135,38,197,155,193,166,187,158,210,160,0,248,215,35,59,136,120,38,73,173,186,23,18,95,9,248,187,43,13,199,101},
  {90,163,30,112,14,2,97,154,84,67,176,71,190,143,20,7,240,216,14,241,210,1,250,181,129,147,94,225,35,32,40,76},
  {43,176,209,174,33,167,159,219,239,213,211,254,235,136,64,88,102,86,126,197,24,230,200,108,26,105,230,132,70,203,15,60},
  {18,108,13,98,125,12,35,117,78,14,173,144,130,233,220,103,61,48,61,138,173,73,78,230,43,146,200,198,202,68,166,114},
  {36,32,32,180,194,114,27,225,13,172,80,245,211,245,17,217,34,50,62,79,248,217,44,43,7,129,4,138,24,80,143,79},
  {14,21,190,58,193,178,32,136,216,132,1,159,253,136,204,25,198,161,209,166,24,212,123,57,204,106,194,185,61,102,133,114},
  {213,51,228,40,129,146,130,248,131,62,14,178,172,217,20,149,191,212,249,72,1,34,156,61,74,34,226,105,17,84,209,80},
  {129,167,201,196,132,169,163,1,17,105,68,159,113,135,169,108,40,212,23,159,143,238,70,113,27,52,199,197,95,146,85,126},
  {120,103,152,49,82,245,180,210,18,103,251,55,45,145,67,83,21,117,18,175,181,22,48,61,206,21,154,250,12,134,157,23},
  {127,149,123,121,135,208,71,131,79,205,255,68,95,48,168,111,64,113,226,65,209,97,166,86,123,14,198,194,84,249,147,57},
  {137,150,86,87,214,209,137,142,54,23,127,97,186,99,246,136,84,195,89,22,226,112,3,222,181,48,26,113,72,46,18,68},
  {120,108,163,209,215,42,47,22,132,165,178,217,23,235,221,94,11,144,55,102,118,56,235,203,26,31,235,91,2,233,9,57},
  {51,39,77,39,12,64,251,237,75,31,86,190,173,24,121,116,204,235,21,66,28,17,160,215,115,32,87,235,99,0,70,73},
  {156,178,123,100,84,35,125,66,241,126,52,112,26,121,95,96,46,13,32,86,1,209,114,182,78,32,135,74,149,53,56,89},
  {64,79,89,242,159,172,86,45,6,141,239,99,98,223,140,102,111,250,187,174,139,78,208,60,128,208,255,239,40,124,197,100},
  {83,156,177,241,3,80,35,145,181,118,166,248,17,239,23,170,31,152,148,199,160,196,98,107,210,33,239,125,129,173,11,112},
  {243,184,108,115,167,61,66,115,189,103,208,101,151,125,150,73,235,160,215,7,167,34,113,116,91,10,18,21,241,157,169,25},
  {219,184,178,42,76,242,99,76,211,155,123,47,162,5,41,170,253,180,125,151,68,141,133,133,149,250,27,187,31,238,212,31},
  {24,77,226,254,82,120,244,151,250,191,142,87,196,80,117,173,128,35,37,190,63,246,152,209,55,109,125,174,176,123,195,42},
  {111,125,87,59,189,141,150,157,177,154,49,224,192,185,125,47,118,225,94,50,164,225,47,31,26,59,3,110,175,121,255,63},
  {74,161,209,191,143,72,135,114,36,221,77,51,77,61,210,243,82,42,233,127,149,62,173,6,142,107,76,119,122,52,0,24},
  {240,17,65,128,222,188,51,179,44,254,96,233,152,221,248,113,109,126,143,3,165,24,49,239,182,203,195,77,153,21,26,16},
  {70,60,90,37,152,210,72,119,75,248,250,200,153,217,188,179,196,241,189,227,97,221,11,237,158,126,217,113,77,144,83,35},
  {129,118,6,79,108,115,231,206,173,24,242,100,63,20,27,107,141,232,187,210,82,114,175,42,14,76,72,103,186,85,27,46},
  {219,61,28,233,38,51,65,2,96,178,192,54,95,214,207,115,46,121,124,181,251,20,214,205,199,27,84,8,85,19,48,103},
  {165,191,33,172,185,254,208,67,197,9,199,247,156,53,228,176,95,45,236,46,250,120,42,230,68,117,25,236,2,161,233,38},
  {2,194,163,242,105,204,65,204,69,54,166,138,0,160,39,161,68,206,132,81,40,122,143,120,238,196,216,243,243,86,9,3},
  {59,49,73,91,67,32,68,135,249,39,2,143,248,204,34,10,163,154,194,12,173,67,107,115,194,162,64,117,168,63,45,68},
  {118,185,133,0,163,216,37,228,197,73,162,158,208,118,252,196,144,118,90,172,3,170,49,62,40,161,26,79,99,207,48,73},
  {151,140,64,139,222,109,163,24,8,7,171,199,62,167,194,130,220,166,14,55,133,56,215,239,100,79,12,215,59,35,47,5},
  {146,200,111,108,165,8,233,118,47,44,9,200,173,98,87,150,201,29,12,120,52,115,41,9,102,238,154,13,118,165,80,8},
  {219,85,213,7,35,123,174,0,146,57,249,221,49,106,153,177,125,210,202,249,215,106,135,220,50,108,101,221,120,222,209,121},
  {238,52,219,74,35,150,194,118,91,22,192,59,176,21,39,105,71,79,179,208,28,50,48,174,222,202,50,32,17,86,136,74},
  {238,83,44,137,90,53,130,229,180,124,10,146,170,228,211,83,122,247,24,29,38,201,8,67,244,71,153,35,158,158,248,94},
  {92,28,64,43,179,201,212,121,19,176,150,159,19,34,105,211,186,150,29,79,217,30,43,18,91,244,147,30,37,50,145,73},
  {42,123,79,236,5,178,213,172,228,33,53,20,223,155,22,83,178,60,197,76,216,66,253,72,234,255,100,190,187,215,47,111},
  {250,177,147,217,233,160,186,29,119,105,104,242,246,148,130,225,133,120,254,157,80,7,137,38,116,114,236,211,201,59,47,56},
  {129,77,30,15,126,37,8,178,208,227,146,4,183,147,231,48,102,168,28,240,188,210,90,245,217,121,55,56,86,40,22,112},
  {121,66,2,74,6,6,88,169,97,137,221,1,33,78,114,235,25,147,99,206,254,85,152,219,170,41,133,78,143,99,59,127},
  {170,119,56,39,128,28,95,151,68,67,79,245,138,27,171,172,185,60,193,43,24,47,44,22,133,51,192,33,91,235,206,106},
  {8,65,6,229,48,237,228,7,186,78,20,76,185,115,71,216,3,174,43,115,161,223,182,143,26,77,223,136,88,179,28,15},
  {97,202,213,1,93,35,63,31,114,140,174,116,11,159,132,134,43,215,202,227,193,252,196,226,200,189,101,97,25,250,52,110},
  {144,90,151,40,70,230,158,150,37,254,76,138,189,139,90,226,169,139,212,188,116,43,13,72,178,251,51,114,187,178,133,105},
  {10,188,206,205,136,173,60,128,77,197,17,231,223,211,230,220,17,104,1,218,62,134,163,191,85,109,102,124,51,150,122,56},
  {208,24,51,144,1,211,11,51,209,140,140,180,150,71,16,72,255,82,76,63,254,109,43,14,72,89,12,117,77,196,221,112},
  {50,40,208,33,127,78,209,52,163,161,65,165,100,201,185,211,194,83,220,236,171,131,64,9,243,32,91,219,61,241,131,20},
  {255,255,52,23,148,8,10,129,27,228,21,242,65,164,23,127,138,187,31,100,178,128,195,40,228,149,121,155,207,184,181,12},
  {169,10,50,75,199,215,160,218,225,10,166,49,42,98,157,131,106,197,113,14,21,16,69,162,127,31,115,83,26,50,220,44},
  {33,128,135,172,244,183,44,121,229,229,30,190,63,226,46,39,242,3,248,58,154,233,198,50,245,4,78,235,192,222,16,116},
  {144,75,112,204,177,170,103,101,100,245,2,231,54,239,231,165,204,193,16,59,117,199,192,107,107,218,95,48,56,38,110,59},
  {236,116,148,250,53,204,255,155,189,174,49,31,227,4,199,185,208,101,4,144,61,99,214,209,250,189,47,68,87,244,255,119},
  {190,179,239,119,211,163,178,156,228,21,15,252,208,25,41,206,154,89,209,161,71,3,248,190,218,248,185,156,20,160,126,76},
  {151,247,57,97,74,63,212,198,121,181,59,203,196,23,78,158,248,155,40,115,225,235,68,103,24,145,186,35,48,255,78,29},
  {91,88,13,142,88,152,119,94,151,178,38,105,65,203,224,246,198,72,71,98,229,32,31,51,217,79,138,34,198,58,179,70},
  {113,242,158,115,21,159,155,180,109,128,161,142,51,244,151,129,226,247,98,58,15,136,15,211,29,232,125,88,18,156,4,87},
  {59,136,166,195,70,104,62,128,52,248,9,234,7,194,70,131,203,213,54,0,194,151,73,250,173,41,5,68,170,108,31,97},
  {202,17,137,96,105,114,181,212,253,36,207,127,122,175,226,141,188,173,58,232,56,248,91,180,241,125,166,251,235,85,42,112},
  {170,194,130,44,72,120,74,92,93,148,117,38,246,71,92,140,114,84,251,163,98,26,115,180,126,160,230,40,229,122,74,88},
  {37,181,156,223,73,148,255,25,225,127,31,176,26,136,85,198,118,20,104,38,18,215,103,66,160,33,238,135,14,72,65,85},
  {31,131,56,113,246,52,240,82,43,199,215,249,193,40,111,135,237,109,39,35,12,11,42,67,129,73,56,188,52,215,185,88},
  {169,156,99,85,216,243,253,94,165,4,157,157,50,105,112,162,106,23,170,55,67,80,69,166,131,249,157,9,223,46,95,107},
  {70,37,114,73,84,70,104,61,216,163,160,43,161,16,125,139,33,44,118,108,80,0,40,14,175,183,38,130,96,78,152,39},
  {169,247,83,137,121,24,61,102,195,157,195,159,35,145,47,183,46,246,87,52,6,96,168,129,208,74,198,116,5,100,41,21},
  {244,201,218,228,24,195,67,76,89,84,36,108,125,251,247,27,12,181,242,88,132,116,255,121,142,33,45,179,14,63,74,45},
  {235,42,62,230,239,101,137,17,210,137,216,226,83,83,199,183,46,85,55,18,144,218,212,187,11,229,21,108,58,2,154,16},
  {115,196,14,7,223,5,34,5,145,161,211,231,159,143,233,118,19,24,78,249,104,215,51,54,88,168,226,120,163,227,155,38},
  {115,253,197,120,56,153,69,26,253,71,74,213,1,23,182,13,27,7,160,241,108,64,86,196,241,157,106,63,59,102,83,70},
  {166,74,198,146,147,49,106,142,62,179,216,242,183,55,52,108,245,246,139,38,110,64,8,26,208,107,9,188,81,47,146,68},
  {56,254,54,251,73,9,106,208,32,93,47,47,184,0,187,3,250,206,159,119,108,28,207,205,6,107,79,126,117,212,229,104},
  {42,113,184,14,22,84,226,141,134,22,150,26,253,241,96,135,90,28,222,1,178,62,150,181,184,105,88,129,174,162,148,31},
  {99,65,125,140,163,100,187,220,51,93,151,14,223,124,138,182,211,85,220,102,72,111,151,161,197,39,207,150,74,161,195,72},
  {248,127,161,232,101,239,115,61,149,169,29,53,123,150,212,49,199,212,35,26,112,56,50,56,38,192,228,150,254,103,189,37},
  {65,117,211,12,216,225,143,146,33,92,96,43,220,234,33,87,126,58,103,77,161,160,102,185,241,215,210,159,17,71,11,5},
  {221,57,1,167,72,124,145,106,61,123,89,250,26,224,11,219,121,16,207,132,86,104,47,8,29,126,69,51,110,83,43,27},
  {136,160,227,119,24,213,77,220,29,148,198,247,252,173,61,88,71,207,43,69,129,84,18,236,191,3,122,36,144,171,75,20},
  {39,176,114,157,15,77,195,47,98,46,154,106,101,97,186,245,113,93,10,243,11,51,126,171,222,29,36,172,254,185,16,107},
  {4,112,186,26,73,115,35,109,244,30,18,239,197,25,210,206,205,96,194,179,248,102,154,82,203,230,30,234,127,20,47,124},
  {247,1,180,237,7,35,235,55,91,30,219,255,215,219,252,149,109,218,104,100,69,236,67,247,154,161,81,88,69,65,32,118},
  {90,21,64,11,37,11,141,49,46,74,98,219,243,147,68,174,67,63,55,239,169,24,90,91,169,140,138,183,205,46,99,16},
  {82,74,202,211,226,26,97,34,75,225,54,117,19,142,96,216,218,114,149,24,159,148,129,161,240,127,243,22,171,202,34,125},
  {190,106,244,14,253,44,237,59,19,182,46,49,42,4,13,23,206,21,53,123,219,92,219,72,189,254,12,254,222,163,139,112},
  {11,192,78,123,179,201,109,66,134,214,78,204,16,96,195,16,175,13,246,229,141,108,82,77,67,243,128,121,82,195,16,43},
  {59,19,185,127,46,16,161,64,111,171,136,138,68,224,35,109,105,28,245,243,51,65,0,166,142,105,203,98,99,113,14,78},
  {126,41,246,11,112,28,100,230,194,136,26,88,211,173,236,33,44,41,106,174,67,215,91,37,46,49,215,16,209,95,196,8},
  {218,219,136,245,228,2,203,64,153,42,104,191,33,82,145,64,77,113,118,66,59,175,113,230,17,39,108,240,48,135,218,5},
  {215,252,67,88,250,76,235,146,160,67,132,180,99,166,125,255,236,116,0,120,234,205,110,20,121,207,98,60,13,181,143,4},
  {177,225,217,188,205,185,112,112,6,225,160,156,86,13,239,157,124,31,215,163,40,199,131,58,85,193,231,193,206,186,98,77},
  {217,169,80,55,17,90,153,11,148,47,211,56,65,144,161,138,66,5,206,212,207,66,115,159,107,113,126,238,3,82,214,54},
  {106,232,125,67,140,243,175,7,29,77,204,9,36,222,19,112,33,80,108,118,175,181,57,13,53,82,180,200,198,104,114,100},
  {25,167,166,42,250,60,154,221,179,53,153,104,130,229,157,17,231,241,201,107,57,106,224,96,242,13,148,59,97,220,153,42},
  {68,33,236,124,237,226,8,150,133,116,178,140,238,225,215,179,52,244,84,146,197,55,48,62,196,98,118,86,18,69,78,109},
  {33,37,129,47,133,98,239,169,189,200,141,166,43,22,238,225,172,247,225,218,173,149,150,77,179,73,52,25,216,221,41,3},
  {215,39,57,114,185,6,106,47,249,244,241,228,115,218,5,72,202,54,192,107,229,202,220,157,233,113,60,93,190,49,132,127},
  {4,160,84,26,135,171,173,252,81,216,123,198,123,183,81,98,69,205,244,47,30,1,131,217,10,22,97,41,89,219,154,42},
  {134,187,23,244,110,88,203,14,172,152,244,161,178,203,137,72,214,92,117,231,113,66,108,33,14,122,4,43,30,153,41,58},
  {109,131,103,194,88,121,106,20,61,194,0,112,7,219,194,121,205,239,137,238,145,227,176,145,116,169,132,110,222,186,25,13},
  {204,41,112,129,220,87,8,115,81,70,91,107,192,66,116,55,79,98,37,51,156,120,70,3,43,144,95,139,52,48,224,41},
  {134,162,134,212,176,129,0,175,152,87,105,13,210,170,183,174,181,214,22,255,166,217,245,17,157,178,60,67,182,213,243,81},
  {163,57,72,14,105,41,25,105,33,251,68,189,186,30,95,182,210,48,26,251,148,236,40,94,236,6,100,46,116,13,102,73},
  {231,214,186,185,132,85,228,182,28,78,168,61,112,205,229,145,139,97,178,236,91,120,171,153,29,24,190,249,110,159,96,14},
  {18,2,183,113,150,221,15,140,218,124,112,103,112,27,4,89,12,253,76,44,209,86,249,235,77,10,204,164,190,77,237,10},
  {248,99,147,125,98,129,78,142,57,115,131,11,241,197,240,182,57,193,94,193,14,11,136,122,72,116,247,254,53,232,184,73},
  {103,102,33,65,143,34,247,169,101,62,153,147,175,173,107,164,123,7,10,12,57,84,221,14,208,230,35,240,80,156,123,102},
  {131,46,70,237,163,138,5,131,9,132,240,153,214,179,149,218,22,99,31,46,136,7,28,80,213,71,143,234,132,201,142,32},
  {18,236,254,203,182,30,41,246,109,110,144,221,12,121,3,75,88,133,20,228,205,237,237,21,179,34,177,40,34,171,125,37},
  {164,246,166,99,198,192,139,239,233,157,116,118,18,84,11,82,8,253,193,169,18,215,22,128,96,174,105,202,219,183,140,17},
  {47,229,125,163,71,205,98,67,21,40,218,172,95,187,41,7,48,255,246,132,175,196,207,194,237,144,153,95,88,203,59,116},
  {47,229,125,163,71,205,98,67,21,40,218,172,95,187,41,7,48,255,246,132,175,196,207,194,237,144,153,95,88,203,59,116},
  {47,229,125,163,71,205,98,67,21,40,218,172,95,187,41,7,48,255,246,132,175,196,207,194,237,144,153,95,88,203,59,116},
  {47,229,125,163,71,205,98,67,21,40,218,172,95,187,41,7,48,255,246,132,175,196,207,194,237,144,153,95,88,203,59,116},
  {47,229,125,163,71,205,98,67,21,40,218,172,95,187,41,7,48,255,246,132,175,196,207,194,237,144,153,95,88,203,59,116},
  {47,229,125,163,71,205,98,67,21,40,218,172,95,187,41,7,48,255,246,132,175,196,207,194,237,144,153,95,88,203,59,116},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {195,166,48,25,132,99,114,14,28,223,119,38,243,122,231,159,191,27,92,243,17,18,22,201,77,251,14,81,224,73,208,113},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {58,19,199,99,205,16,241,198,49,144,137,227,220,29,92,41,147,79,255,160,161,146,18,67,210,38,76,113,3,191,249,11},
  {129,96,75,13,134,130,203,253,136,159,23,146,238,55,240,99,36,115,117,88,2,108,40,255,68,207,86,174,255,137,20,91},
  {129,96,75,13,134,130,203,253,136,159,23,146,238,55,240,99,36,115,117,88,2,108,40,255,68,207,86,174,255,137,20,91},
  {129,96,75,13,134,130,203,253,136,159,23,146,238,55,240,99,36,115,117,88,2,108,40,255,68,207,86,174,255,137,20,91},
  {129,96,75,13,134,130,203,253,136,159,23,146,238,55,240,99,36,115,117,88,2,108,40,255,68,207,86,174,255,137,20,91},
  {129,96,75,13,134,130,203,253,136,159,23,146,238,55,240,99,36,115,117,88,2,108,40,255,68,207,86,174,255,137,20,91},
  {129,96,75,13,134,130,203,253,136,159,23,146,238,55,240,99,36,115,117,88,2,108,40,255,68,207,86,174,255,137,20,91},
  {41,204,100,115,229,19,170,59,195,143,92,9,18,123,202,155,144,96,108,128,22,254,48,177,149,5,166,70,76,3,74,5},
  {41,204,100,115,229,19,170,59,195,143,92,9,18,123,202,155,144,96,108,128,22,254,48,177,149,5,166,70,76,3,74,5},
  {41,204,100,115,229,19,170,59,195,143,92,9,18,123,202,155,144,96,108,128,22,254,48,177,149,5,166,70,76,3,74,5},
  {41,204,100,115,229,19,170,59,195,143,92,9,18,123,202,155,144,96,108,128,22,254,48,177,149,5,166,70,76,3,74,5},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {239,19,0,85,228,133,238,15,35,42,93,205,223,5,24,254,95,49,91,161,116,208,209,231,125,157,104,224,183,152,206,121},
  {186,100,129,102,128,121,51,14,111,124,248,226,230,194,177,153,116,171,27,164,13,96,68,219,72,119,208,151,145,146,43,25},
  {186,100,129,102,128,121,51,14,111,124,248,226,230,194,177,153,116,171,27,164,13,96,68,219,72,119,208,151,145,146,43,25},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {49,69,121,140,250,11,254,249,40,76,73,64,54,199,254,133,6,8,187,230,229,221,241,248,52,191,158,201,205,76,149,38},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {251,78,104,221,156,70,174,92,92,11,53,30,237,92,63,143,20,113,21,125,104,12,117,217,183,241,115,24,213,66,211,32},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
  {18,60,113,251,175,3,10,192,89,8,28,98,103,78,130,248,100,186,27,194,145,77,83,69,230,171,87,109,26,188,18,28},
} ;

static const unsigned char precomputed_nG_montgomery25519_n[precomputed_nG_montgomery25519_NUM][crypto_nG_SCALARBYTES] = {
  {248,181,1,40,168,159,146,159,7,139,148,246,114,42,35,194,81,211,62,44,126,145,190,89,62,58,107,20,218,93,118,95},
  {14,232,69,146,96,32,217,118,165,148,112,63,5,13,120,17,228,235,88,112,98,223,9,159,154,56,205,253,30,183,254,241},
  {11,163,86,36,137,113,113,229,202,180,1,160,250,8,230,243,132,17,69,51,144,151,87,27,156,223,140,110,112,87,53,140},
  {16,216,149,3,205,241,168,94,207,104,27,160,54,112,73,99,134,123,175,246,124,247,59,182,146,31,82,203,200,18,92,130},
  {148,4,13,240,188,103,159,76,215,255,194,136,69,190,104,224,217,76,164,0,233,176,114,160,88,113,113,9,79,141,177,167},
  {18,57,230,52,26,1,62,159,250,37,100,205,24,38,31,225,20,8,251,54,163,185,28,238,130,157,199,35,237,92,244,183},
  {31,148,53,108,30,105,71,99,143,68,89,101,48,139,33,183,231,45,1,138,84,28,226,175,40,187,63,17,182,147,181,111},
  {88,108,123,69,26,196,138,148,157,192,0,60,139,64,0,18,50,253,240,196,178,158,190,242,213,127,171,27,171,100,126,98},
  {199,203,24,166,64,157,248,207,126,145,226,70,226,46,83,164,212,97,249,87,34,190,68,80,167,63,215,145,26,153,223,72},
  {12,178,98,251,119,168,170,140,182,33,62,143,118,199,236,245,180,189,123,225,65,6,230,228,227,92,55,211,134,132,198,38},
  {202,122,215,32,241,88,182,107,239,199,244,200,35,37,123,181,26,150,59,21,180,73,135,215,184,42,187,112,115,13,235,84},
  {119,199,1,5,187,141,221,57,160,118,141,51,84,183,7,115,31,183,209,1,224,243,85,240,163,250,207,186,125,153,24,121},
  {147,181,139,164,118,11,157,253,138,152,243,158,200,155,204,226,14,234,142,113,42,233,232,152,113,172,69,205,115,162,116,65},
  {151,201,136,193,3,163,232,197,250,69,54,50,146,250,192,80,116,116,69,85,81,243,34,135,215,190,6,34,187,205,146,211},
  {53,140,196,22,171,6,214,196,80,29,248,191,58,99,92,7,43,116,208,20,74,221,103,42,77,122,30,52,191,213,24,111},
  {152,21,165,122,147,183,199,178,223,227,148,248,12,192,148,244,198,101,20,221,20,40,81,179,137,166,133,247,158,97,16,186},
  {199,126,47,157,213,119,132,90,244,150,154,153,154,154,175,20,126,75,236,6,157,105,203,242,103,149,187,238,74,100,29,95},
  {147,41,221,16,120,22,132,154,184,105,117,139,112,209,48,48,102,83,82,213,18,58,173,96,158,249,223,174,110,72,249,153},
  {112,148,171,50,138,202,67,156,140,89,111,230,219,30,184,200,227,20,71,82,155,103,17,59,134,168,24,154,48,55,32,71},
  {199,183,70,67,59,68,228,176,208,10,125,52,255,142,161,107,28,174,244,115,102,51,114,77,64,190,79,234,136,47,175,91},
  {233,159,249,86,219,139,158,70,42,195,176,76,198,41,132,143,44,224,5,108,23,14,93,69,204,227,164,90,104,14,60,40},
  {190,72,88,96,176,73,76,47,132,132,175,86,38,14,206,53,218,244,156,179,108,147,149,29,169,28,214,15,234,26,85,53},
  {61,117,112,156,219,187,229,35,238,210,246,106,48,168,101,69,220,180,114,154,115,7,171,250,27,121,92,134,165,83,99,112},
  {250,220,191,52,104,105,84,62,242,254,8,18,46,88,247,248,39,31,253,69,209,26,127,225,178,41,120,249,238,52,73,53},
  {165,236,60,105,227,60,108,62,233,194,213,235,121,33,24,69,113,87,55,189,250,66,55,250,182,214,129,41,251,136,2,211},
  {183,127,46,22,41,41,11,240,249,213,246,134,2,67,153,129,251,42,136,14,216,249,183,120,53,251,1,106,115,229,245,147},
  {144,246,246,13,14,80,136,47,137,223,221,221,172,249,61,118,37,166,102,74,111,169,38,132,56,10,247,64,253,214,61,241},
  {185,57,126,93,32,245,59,244,228,128,212,243,96,46,242,13,13,75,2,167,111,120,117,105,47,12,230,96,5,40,73,184},
  {109,84,200,146,153,129,235,230,7,79,177,144,214,106,177,23,5,90,63,28,203,159,12,93,87,217,61,244,52,63,225,61},
  {228,162,222,194,144,84,3,9,115,119,212,85,233,217,59,199,57,157,40,142,48,6,250,102,92,195,127,128,27,22,158,71},
  {171,218,223,232,27,15,193,203,132,118,247,251,155,117,124,7,128,103,192,93,64,34,74,143,66,151,138,176,6,252,218,205},
  {73,212,56,83,87,96,45,59,56,150,101,42,101,118,4,94,69,135,31,96,60,74,155,222,137,187,207,211,110,129,54,134},
  {128,86,9,240,10,150,186,54,196,60,206,41,190,111,6,15,104,70,250,151,198,189,157,227,13,100,184,190,70,252,108,106},
  {143,68,191,130,34,54,10,179,78,122,92,103,61,194,242,182,56,117,173,172,239,46,26,254,221,174,44,106,77,163,109,150},
  {6,48,68,18,62,116,229,178,180,39,94,200,111,75,220,203,76,199,207,219,113,118,148,175,32,32,107,146,64,12,228,65},
  {54,5,81,2,149,104,165,112,156,85,53,35,105,152,8,215,27,169,227,200,119,128,41,178,83,124,82,55,80,34,124,147},
  {132,195,224,61,35,5,159,144,25,69,4,4,244,116,232,252,18,91,221,80,99,247,100,169,124,166,202,86,74,253,217,77},
  {80,56,243,242,188,63,207,83,52,5,144,165,76,248,253,168,159,178,148,61,10,233,168,227,9,236,253,57,87,93,157,133},
  {231,188,33,49,132,250,243,148,132,244,118,152,49,163,206,63,32,69,96,52,196,79,0,159,62,60,118,108,183,151,228,232},
  {43,158,30,127,10,160,120,118,158,170,140,134,64,33,132,156,29,44,71,180,174,52,16,184,8,204,93,155,16,209,127,97},
  {150,131,157,218,200,100,94,164,145,203,20,105,222,208,169,27,13,186,35,245,28,144,248,193,82,95,235,172,180,170,118,124},
  {193,246,237,0,40,33,29,56,183,153,174,89,123,144,18,83,96,136,49,82,239,132,102,241,0,53,52,62,17,45,121,18},
  {70,3,65,0,114,24,25,165,155,42,222,112,212,37,3,7,163,20,13,4,9,101,15,124,148,110,72,37,49,216,152,0},
  {33,194,117,109,66,207,181,250,92,38,54,162,70,144,28,242,118,168,64,183,98,239,199,235,154,113,222,57,0,186,194,49},
  {193,201,139,241,210,71,234,92,126,85,186,0,4,145,4,241,77,35,164,79,95,114,230,112,21,41,180,230,215,116,73,184},
  {150,139,28,166,172,205,159,1,23,222,96,89,102,139,25,175,183,101,239,183,206,32,33,192,41,22,223,107,143,73,160,50},
  {251,133,84,194,4,222,184,37,45,25,80,61,85,223,22,91,124,40,235,196,203,193,31,137,38,211,231,154,76,104,73,200},
  {131,86,213,72,251,92,146,144,158,137,245,228,136,104,186,182,226,128,158,192,62,202,69,189,225,43,214,117,51,26,31,51},
  {60,42,201,227,21,150,42,186,133,169,250,72,97,236,91,235,123,183,31,173,64,51,96,236,221,228,0,223,179,62,23,89},
  {128,199,106,32,154,78,38,27,72,148,80,4,95,247,73,203,28,41,88,197,27,184,58,22,198,111,243,15,190,166,149,198},
  {127,101,191,28,148,8,98,199,227,49,249,127,188,215,238,202,94,48,189,113,126,162,24,156,49,220,132,169,16,205,44,51},
  {32,114,157,231,253,81,34,116,65,111,53,89,235,252,196,252,48,13,141,44,21,182,186,106,84,221,61,119,12,206,88,155},
  {25,189,178,233,5,103,25,80,233,209,70,192,154,204,135,200,150,5,16,53,37,51,165,109,147,246,72,162,122,250,68,194},
  {172,208,42,169,82,1,140,1,13,13,126,143,78,134,234,87,52,24,69,233,98,68,63,110,47,160,4,231,94,116,177,114},
  {34,121,0,61,76,153,99,93,123,129,113,110,206,196,186,13,188,221,84,169,8,138,142,207,90,12,143,247,209,84,77,131},
  {42,136,94,105,148,88,105,238,54,10,193,182,191,238,173,106,165,179,167,114,129,129,171,197,72,161,212,183,140,130,184,99},
  {18,205,14,159,90,202,16,229,181,12,52,228,161,16,61,44,107,173,102,151,53,184,32,24,130,29,92,84,67,92,132,180},
  {163,203,116,25,171,243,121,61,195,21,221,204,26,153,102,181,18,126,247,132,97,24,154,66,189,179,44,203,2,156,163,230},
  {125,31,64,167,137,175,45,21,51,211,233,128,110,198,74,117,45,90,176,84,221,252,149,184,186,209,44,76,155,54,22,59},
  {201,219,182,219,90,131,170,113,119,195,100,44,89,247,250,41,204,90,36,184,227,150,213,164,34,114,159,96,7,210,203,249},
  {80,128,135,167,93,150,211,226,214,185,68,212,42,8,155,194,56,246,61,248,130,15,179,233,68,78,0,21,88,229,27,250},
  {156,66,77,180,174,107,233,12,184,58,46,61,64,98,183,92,16,219,71,111,27,51,188,248,186,186,102,180,73,56,13,192},
  {126,105,113,222,138,197,147,2,78,6,164,71,89,108,88,171,66,1,135,129,235,20,23,71,23,196,77,202,214,125,157,175},
  {210,1,39,43,213,4,113,52,209,168,194,36,183,83,208,113,96,24,160,90,15,173,111,2,218,216,204,35,245,82,131,100},
  {26,133,164,74,34,10,213,241,118,228,228,196,85,9,45,2,27,221,132,101,239,21,197,32,148,26,6,122,61,238,33,109},
  {24,77,11,250,21,9,26,2,228,106,203,113,113,8,205,27,52,212,75,253,88,90,98,94,225,3,17,117,52,37,118,8},
  {143,172,134,214,84,140,49,228,174,156,78,179,61,77,91,228,144,247,118,134,122,129,186,68,152,25,181,254,25,239,203,173},
  {219,123,41,215,52,53,170,77,222,109,52,73,25,99,9,198,56,1,84,246,138,208,150,1,142,15,66,240,6,89,57,84},
  {204,171,34,245,66,116,196,142,227,179,177,211,221,118,54,247,140,254,82,214,0,116,156,233,213,182,128,73,40,197,46,135},
  {61,124,30,206,44,119,58,162,88,242,155,6,244,255,208,58,104,217,249,227,171,95,17,220,83,192,221,148,164,122,8,64},
  {11,222,99,51,38,46,245,94,147,100,179,51,147,230,162,113,17,37,165,101,252,247,185,36,144,73,114,123,22,194,214,45},
  {0,109,134,77,20,214,239,230,93,78,150,182,114,48,60,49,182,164,80,57,127,100,123,26,249,117,119,77,10,164,16,174},
  {190,52,77,80,59,196,250,15,154,113,245,213,86,255,155,2,170,204,185,85,136,183,10,194,90,223,247,17,165,194,113,185},
  {115,106,3,157,86,134,12,145,213,31,164,99,232,92,120,54,207,172,141,117,138,12,49,213,173,20,82,175,80,110,119,181},
  {75,105,208,239,69,92,1,44,88,234,160,169,110,27,102,193,137,141,31,119,244,190,62,149,23,253,114,138,166,219,233,12},
  {25,4,206,72,111,144,216,129,151,243,187,122,192,98,90,152,226,21,218,134,198,164,40,4,78,212,53,34,231,49,235,93},
  {130,177,74,203,203,9,147,135,150,55,71,149,91,175,112,194,97,100,29,255,151,202,137,159,32,238,169,17,105,195,248,204},
  {26,139,84,222,73,164,95,200,197,246,159,238,232,197,100,177,248,155,162,245,92,148,69,65,182,152,102,102,144,175,66,166},
  {114,164,45,29,139,80,49,60,106,107,60,128,63,120,253,179,193,56,108,162,227,252,98,109,0,25,202,38,28,138,44,27},
  {140,255,196,225,13,181,102,161,58,158,219,87,108,13,233,160,85,170,107,238,165,112,3,40,204,223,33,176,135,137,131,231},
  {89,164,46,121,146,172,87,140,34,168,132,6,230,49,237,159,96,92,236,101,47,59,64,29,206,188,61,248,197,236,69,167},
  {15,224,104,161,59,86,211,32,209,58,171,165,234,218,86,47,130,95,28,62,155,193,166,66,168,42,126,13,43,240,89,217},
  {91,21,41,161,17,32,132,81,220,123,110,142,13,121,29,181,216,173,254,249,123,99,54,191,199,98,8,201,237,129,121,229},
  {119,175,107,233,214,49,15,33,34,240,165,131,124,172,88,165,151,21,231,192,33,212,143,125,164,32,251,55,215,88,94,173},
  {45,185,247,101,62,178,115,117,40,137,19,3,52,214,247,56,98,107,124,152,151,34,77,254,14,218,87,71,38,70,109,201},
  {212,148,110,55,200,16,191,27,144,170,216,245,122,143,77,52,2,173,111,97,150,226,50,250,255,202,251,217,158,10,214,176},
  {83,109,4,217,251,38,255,79,150,202,155,27,23,235,50,83,71,46,95,204,190,1,133,105,176,96,36,185,100,100,219,160},
  {220,73,255,236,98,106,71,217,120,72,104,95,145,59,95,228,172,211,130,57,153,25,136,142,244,43,208,201,7,59,127,82},
  {101,45,94,226,67,86,209,9,52,17,7,240,188,142,33,206,171,161,24,172,81,96,131,173,95,1,190,230,66,32,127,37},
  {139,29,203,36,46,99,179,5,84,125,7,126,231,205,84,116,96,255,175,78,138,28,127,170,191,60,163,92,81,71,35,141},
  {218,129,65,22,37,94,131,101,35,110,15,119,83,245,240,48,244,186,36,142,53,245,35,180,236,233,183,22,106,235,36,246},
  {134,239,139,12,40,148,217,177,8,124,217,162,184,125,47,135,143,189,125,245,99,46,131,92,130,250,53,108,244,194,45,133},
  {230,3,148,40,241,93,220,142,10,240,184,111,118,170,205,217,252,224,208,159,65,35,156,179,246,174,57,32,65,59,60,236},
  {123,179,56,107,13,69,146,119,8,144,192,54,27,20,55,8,83,79,44,187,149,68,160,184,173,31,244,171,70,41,131,106},
  {134,128,185,138,206,221,239,252,160,115,159,34,206,174,143,34,93,127,60,193,12,198,66,6,163,192,30,205,168,82,170,178},
  {139,0,60,75,217,43,90,39,138,78,41,105,11,51,18,195,181,67,4,2,160,54,244,203,125,170,229,99,68,44,246,50},
  {130,119,54,17,101,169,43,173,142,182,170,201,18,38,139,248,159,4,19,116,61,37,18,196,236,194,38,233,123,175,78,46},
  {203,223,42,236,165,214,194,133,186,90,243,232,20,65,51,85,124,253,59,176,226,143,40,176,27,56,128,97,229,157,203,62},
  {233,83,35,64,153,97,121,198,109,141,67,100,71,202,244,103,185,53,218,170,171,201,115,137,228,128,154,136,5,207,1,237},
  {70,227,81,151,194,81,152,39,109,58,72,118,146,53,147,99,166,243,65,97,127,11,103,218,15,58,248,94,80,141,219,179},
  {176,126,237,81,102,102,132,124,198,161,76,34,206,83,37,206,222,67,114,153,232,1,251,175,144,138,7,199,201,115,7,28},
  {8,139,80,129,28,92,227,252,104,99,17,50,65,187,193,22,212,206,170,208,43,233,87,99,27,150,116,7,208,93,145,84},
  {29,177,188,135,13,64,192,142,201,69,250,152,30,173,54,236,59,99,15,242,142,72,189,22,157,126,27,244,31,52,167,177},
  {106,62,139,211,234,205,169,252,229,42,19,207,121,246,194,120,167,15,115,166,20,223,32,104,216,154,194,95,131,167,36,197},
  {87,95,6,48,16,178,73,234,28,200,209,171,227,63,109,129,57,214,223,106,131,249,240,73,116,200,157,242,240,42,76,162},
  {204,15,228,131,156,185,61,161,152,150,67,113,68,53,243,62,24,216,129,171,30,11,31,102,248,59,127,142,161,57,80,215},
  {55,138,52,244,26,64,233,55,219,89,73,81,174,126,193,63,206,82,174,123,74,198,156,229,224,10,38,108,77,120,157,26},
  {85,137,204,48,130,249,234,35,89,233,48,221,53,243,96,97,211,18,211,45,21,98,234,221,177,119,203,170,210,44,229,164},
  {56,252,57,128,138,57,148,28,83,174,202,88,98,17,163,34,82,26,247,112,162,64,192,167,231,8,189,28,156,29,188,251},
  {112,121,57,107,72,182,104,221,176,75,223,205,42,214,83,163,255,49,31,102,204,116,167,237,242,106,100,110,205,135,219,89},
  {44,125,194,131,141,28,85,153,189,124,131,6,215,148,12,146,101,154,22,73,139,185,118,218,137,216,101,196,127,218,186,94},
  {51,130,62,172,209,36,41,161,29,1,87,181,150,163,126,169,188,192,128,188,163,6,47,152,40,145,201,170,93,219,137,176},
  {117,39,80,87,195,165,184,217,135,17,153,93,117,67,4,42,11,71,238,32,18,163,25,219,224,124,157,18,124,177,88,48},
  {99,161,20,181,199,241,107,128,91,222,223,174,253,195,200,117,10,189,188,0,149,112,34,159,91,220,244,190,164,50,47,91},
  {129,131,164,9,1,35,50,200,135,132,250,64,150,121,41,247,33,174,51,108,194,47,111,153,203,98,177,70,155,227,121,56},
  {74,193,132,93,153,229,221,132,20,175,212,87,254,121,113,97,254,58,26,58,211,44,26,123,29,223,66,16,103,248,71,41},
  {115,254,100,139,60,197,181,208,22,157,181,82,59,72,36,58,212,9,46,184,187,159,233,153,4,128,243,94,6,94,131,242},
  {219,73,229,221,188,105,135,94,153,141,144,222,123,209,184,245,88,225,11,205,31,170,143,55,8,119,4,13,98,20,122,168},
  {196,193,187,93,241,121,11,47,90,122,60,236,194,215,109,163,178,7,61,147,65,26,117,3,164,2,158,146,36,119,32,60},
  {227,222,100,66,28,246,173,249,53,149,213,37,185,54,69,163,16,250,185,224,102,146,4,98,179,194,53,222,8,220,97,123},
  {209,99,147,114,75,237,209,1,229,15,127,18,235,119,0,211,147,124,173,121,62,141,17,8,76,143,7,103,226,214,166,146},
  {103,224,148,94,155,129,179,251,105,250,220,47,46,139,24,183,209,66,113,59,131,78,182,20,165,48,145,219,12,96,248,18},
  {208,4,251,158,62,204,198,242,117,188,234,88,248,175,49,90,172,113,94,200,25,91,3,16,43,140,111,76,171,161,135,4},
  {126,74,143,250,221,41,169,175,238,249,166,84,124,36,25,221,221,106,160,39,213,160,65,1,220,177,253,54,123,209,136,144},
  {143,57,128,178,242,8,119,32,25,176,50,115,223,118,164,211,113,228,56,213,248,18,237,148,101,184,118,9,25,46,43,44},
  {245,122,67,58,179,139,166,104,97,201,74,88,230,100,106,82,94,69,97,82,95,158,166,80,96,136,166,26,117,220,136,213},
  {177,1,54,227,31,2,70,177,148,115,178,232,34,44,225,90,104,138,115,154,32,57,48,20,168,164,0,83,173,234,42,97},
  {127,247,155,26,58,240,193,235,103,31,209,206,83,124,54,240,122,240,31,168,185,104,161,158,250,17,202,252,121,120,91,1},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
} ;

static void test_nG_montgomery25519_impl(long long impl)
{
  unsigned char *q = test_nG_montgomery25519_q;
  unsigned char *n = test_nG_montgomery25519_n;
  unsigned char *q2 = test_nG_montgomery25519_q2;
  unsigned char *n2 = test_nG_montgomery25519_n2;
  long long qlen = crypto_nG_POINTBYTES;
  long long nlen = crypto_nG_SCALARBYTES;

  if (targeti && strcmp(targeti,lib25519_dispatch_nG_montgomery25519_implementation(impl))) return;
  if (impl >= 0) {
    crypto_nG = lib25519_dispatch_nG_montgomery25519(impl);
    printf("nG_montgomery25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_nG_montgomery25519_implementation(impl),lib25519_dispatch_nG_montgomery25519_compiler(impl));
  } else {
    crypto_nG = lib25519_nG_montgomery25519;
    printf("nG_montgomery25519 selected implementation %s compiler %s\n",lib25519_nG_montgomery25519_implementation(),lib25519_nG_montgomery25519_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 512 : 64;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {

      output_prepare(q2,q,qlen);
      input_prepare(n2,n,nlen);
      crypto_nG(q,n);
      checksum(q,qlen);
      output_compare(q2,q,qlen,"crypto_nG");
      input_compare(n2,n,nlen,"crypto_nG");

      double_canary(q2,q,qlen);
      double_canary(n2,n,nlen);
      crypto_nG(q2,n2);
      if (memcmp(q2,q,qlen) != 0) fail("failure: crypto_nG is nondeterministic\n");

      double_canary(q2,q,qlen);
      double_canary(n2,n,nlen);
      crypto_nG(n2,n2);
      if (memcmp(n2,q,qlen) != 0) fail("failure: crypto_nG does not handle n=q overlap\n");
      memcpy(n2,n,nlen);
    }
    checksum_expected(nG_montgomery25519_checksums[checksumbig]);
  }
  for (long long precomp = 0;precomp < precomputed_nG_montgomery25519_NUM;++precomp) {
    output_prepare(q2,q,crypto_nG_POINTBYTES);
    input_prepare(n2,n,crypto_nG_SCALARBYTES);
    memcpy(n,precomputed_nG_montgomery25519_n[precomp],crypto_nG_SCALARBYTES);
    memcpy(n2,precomputed_nG_montgomery25519_n[precomp],crypto_nG_SCALARBYTES);
    crypto_nG(q,n);
    if (memcmp(q,precomputed_nG_montgomery25519_q[precomp],crypto_nG_POINTBYTES)) {
      fail("failure: crypto_nG fails precomputed test vectors\n");
      printf("expected q: ");
      for (long long pos = 0;pos < crypto_nG_POINTBYTES;++pos) printf("%02x",precomputed_nG_montgomery25519_q[precomp][pos]);
      printf("\n");
      printf("received q: ");
      for (long long pos = 0;pos < crypto_nG_POINTBYTES;++pos) printf("%02x",q[pos]);
      printf("\n");
    }
    output_compare(q2,q,crypto_nG_POINTBYTES,"crypto_nG");
    input_compare(n2,n,crypto_nG_SCALARBYTES,"crypto_nG");
  }
}

static void test_nG_montgomery25519(void)
{
  if (targeto && strcmp(targeto,"nG")) return;
  if (targetp && strcmp(targetp,"montgomery25519")) return;
  test_nG_montgomery25519_q = alignedcalloc(crypto_nG_POINTBYTES);
  test_nG_montgomery25519_n = alignedcalloc(crypto_nG_SCALARBYTES);
  test_nG_montgomery25519_q2 = alignedcalloc(crypto_nG_POINTBYTES);
  test_nG_montgomery25519_n2 = alignedcalloc(crypto_nG_SCALARBYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("nG_montgomery25519 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_nG_montgomery25519();++impl)
      forked(test_nG_montgomery25519_impl,impl);
    ++test_nG_montgomery25519_q;
    ++test_nG_montgomery25519_n;
    ++test_nG_montgomery25519_q2;
    ++test_nG_montgomery25519_n2;
  }
}
#undef crypto_nG_SCALARBYTES
#undef crypto_nG_POINTBYTES


/* ----- mGnP, derived from supercop/crypto_mGnP/try.c */
static const char *mGnP_ed25519_checksums[] = {
  "dc80be44fb0d482c5ae430779e76fe612c53fcd9e5847254bf27ab34e90745f4",
  "9e1a3b7015c8fdb12763fd88494f5bfe9e2565ead4d3407d5ecf7ff6ca24c1d0",
} ;

static void (*crypto_mGnP)(unsigned char *,const unsigned char *,const unsigned char *,const unsigned char *);
#define crypto_mGnP_MBYTES lib25519_mGnP_ed25519_MBYTES
#define crypto_mGnP_NBYTES lib25519_mGnP_ed25519_NBYTES
#define crypto_mGnP_PBYTES lib25519_mGnP_ed25519_PBYTES
#define crypto_mGnP_OUTPUTBYTES lib25519_mGnP_ed25519_OUTPUTBYTES

static unsigned char *test_mGnP_ed25519_Q;
static unsigned char *test_mGnP_ed25519_m;
static unsigned char *test_mGnP_ed25519_n;
static unsigned char *test_mGnP_ed25519_P;
static unsigned char *test_mGnP_ed25519_Q2;
static unsigned char *test_mGnP_ed25519_m2;
static unsigned char *test_mGnP_ed25519_n2;
static unsigned char *test_mGnP_ed25519_P2;

#define precomputed_mGnP_ed25519_NUM 453

static const unsigned char precomputed_mGnP_ed25519_Q[precomputed_mGnP_ed25519_NUM][crypto_mGnP_OUTPUTBYTES] = {
  {193,20,31,22,66,224,234,80,225,166,208,110,185,170,33,13,42,170,37,31,162,113,128,119,198,205,250,123,183,207,66,236,0},
  {239,212,209,8,125,39,38,41,94,189,180,94,193,163,130,175,142,228,76,49,248,193,255,202,155,95,140,198,38,12,125,85,0},
  {55,62,88,224,26,74,175,40,62,4,120,230,61,96,198,36,123,205,4,104,84,86,86,127,219,32,148,226,161,23,102,170,0},
  {107,224,24,189,33,29,232,2,162,186,95,183,1,220,227,240,212,2,3,115,73,34,232,59,29,52,104,48,138,100,168,7,0},
  {31,253,125,101,135,126,126,76,190,193,95,50,18,196,29,41,215,190,147,142,86,228,65,24,220,35,222,83,116,163,20,189,0},
  {243,15,115,147,232,219,192,154,21,238,149,1,185,144,3,64,115,243,5,204,208,176,175,41,80,203,104,37,114,91,56,35,0},
  {172,182,182,78,25,22,62,39,198,223,99,189,255,52,241,97,195,181,245,66,20,227,79,115,28,186,88,10,3,240,179,224,0},
  {62,104,105,88,128,13,255,31,4,177,175,220,184,45,120,237,99,88,147,176,117,206,134,181,197,33,182,172,43,24,216,224,0},
  {36,217,83,145,174,217,124,120,161,163,60,166,133,73,218,76,115,10,1,225,148,53,222,179,249,147,201,35,226,121,230,13,0},
  {218,160,114,157,113,239,46,33,164,162,215,160,65,155,241,51,107,60,133,236,25,115,104,102,157,175,184,77,194,185,123,87,0},
  {215,18,28,155,4,191,122,36,53,143,209,22,121,185,137,113,183,218,156,83,150,28,58,43,202,4,0,59,210,69,177,23,0},
  {15,85,186,179,54,59,138,71,36,152,81,185,49,161,155,253,201,12,186,201,124,43,173,68,103,18,84,216,189,3,91,30,0},
  {189,165,144,177,33,155,200,169,68,113,243,73,187,247,153,192,74,26,254,232,14,126,200,148,143,100,214,124,84,85,5,196,0},
  {177,6,89,157,63,159,208,220,103,251,34,240,243,60,133,94,159,15,174,173,92,53,169,217,72,217,127,178,7,200,66,108,0},
  {200,218,238,202,246,181,95,139,167,63,121,155,154,33,125,40,65,81,42,170,163,12,128,110,223,77,176,79,105,118,186,253,0},
  {236,93,106,16,60,173,90,181,193,160,13,14,253,63,252,27,19,171,35,102,98,249,245,250,37,95,13,113,55,125,247,179,0},
  {21,53,251,238,252,219,189,16,63,238,20,219,129,77,8,52,75,60,4,184,215,235,138,142,75,206,183,127,188,3,19,109,0},
  {104,145,19,16,233,184,221,87,79,192,223,49,15,182,132,34,180,180,33,122,1,117,180,158,225,242,80,206,107,78,59,83,0},
  {60,113,37,3,202,241,247,44,54,115,117,67,243,43,170,197,26,34,207,51,98,51,204,54,181,252,211,107,106,231,22,55,0},
  {218,77,138,8,52,211,187,12,118,234,240,239,195,152,92,96,223,76,137,6,253,192,37,144,100,76,186,107,176,55,85,247,0},
  {24,85,179,137,39,191,189,162,202,35,169,187,29,201,116,49,239,233,68,57,75,237,56,237,66,16,223,28,30,143,117,106,0},
  {172,217,171,30,110,128,229,186,14,247,12,249,185,158,249,162,172,37,224,59,12,85,83,159,221,126,198,249,221,176,176,201,0},
  {202,97,44,50,136,124,132,199,192,148,9,31,216,168,18,23,195,214,1,160,248,207,238,145,34,71,174,25,110,9,104,128,0},
  {42,175,159,17,84,17,216,59,200,90,90,140,55,107,154,70,233,62,180,32,59,23,139,192,76,231,83,162,187,130,234,51,0},
  {229,252,98,104,214,93,46,189,70,61,31,182,9,80,169,26,123,205,82,111,118,102,239,192,12,184,220,83,122,171,92,231,0},
  {84,75,38,86,84,117,13,29,151,24,80,21,25,89,25,32,205,134,170,105,36,113,194,148,178,184,173,16,139,168,33,43,0},
  {67,171,250,206,251,11,201,48,202,162,189,211,63,128,226,36,143,62,132,190,101,147,174,76,208,185,81,61,220,19,68,102,0},
  {71,164,27,141,7,85,148,122,157,184,159,42,226,65,201,137,250,129,124,104,39,250,175,186,127,192,172,189,173,240,51,207,0},
  {213,3,201,50,183,32,237,188,33,37,139,137,250,76,159,166,57,190,10,236,250,160,21,49,216,160,143,15,209,235,186,153,0},
  {207,70,74,84,108,201,112,207,197,30,35,32,255,145,252,198,242,10,24,11,0,25,123,89,186,187,120,52,145,109,160,39,1},
  {102,62,185,27,136,36,225,54,217,146,46,69,87,248,199,31,218,148,52,41,250,110,21,12,16,48,208,245,191,46,232,150,0},
  {115,4,8,153,144,34,13,70,176,166,73,136,245,197,230,207,156,233,120,86,135,113,171,109,230,136,173,95,134,17,45,86,0},
  {247,100,145,199,12,157,94,212,196,149,181,92,183,129,63,148,17,225,141,190,150,16,49,111,24,209,126,46,209,157,216,81,0},
  {135,223,68,75,225,224,171,218,194,215,94,44,44,230,188,134,166,236,26,42,244,255,23,106,155,119,58,238,28,146,223,52,1},
  {121,233,37,176,6,139,215,0,20,81,159,70,184,202,89,182,176,171,14,107,188,59,75,128,28,119,29,32,162,222,94,106,0},
  {176,31,35,125,139,63,38,58,105,40,48,98,119,210,112,7,29,105,79,198,94,40,97,1,214,19,28,236,246,58,96,137,0},
  {65,116,102,140,80,101,108,148,65,135,63,241,188,126,246,182,165,206,235,171,202,191,58,105,36,157,55,236,200,3,170,136,0},
  {49,30,57,168,151,93,61,109,78,11,35,40,175,184,60,156,2,4,19,195,251,124,28,137,180,227,154,16,231,36,156,32,0},
  {209,194,45,74,228,198,222,120,85,129,55,68,190,66,75,250,110,49,195,163,27,138,80,152,62,235,61,12,62,210,194,95,0},
  {37,42,141,79,143,206,32,113,33,250,56,118,114,47,255,34,120,45,148,45,39,200,112,55,236,171,106,39,149,138,170,19,0},
  {94,11,111,55,202,45,7,20,114,142,84,135,129,234,54,228,229,87,148,60,156,205,122,126,58,171,5,65,240,38,65,150,0},
  {161,18,114,212,74,182,224,14,205,231,130,28,0,244,89,29,89,230,182,63,15,139,205,166,39,231,62,141,101,106,32,241,0},
  {85,74,198,40,127,105,178,33,172,143,174,12,203,69,54,100,169,70,251,247,94,213,36,48,163,228,6,249,24,26,36,66,0},
  {50,148,146,153,8,194,122,97,124,86,244,62,98,5,174,209,161,216,141,183,236,5,126,189,102,203,102,129,68,92,27,36,0},
  {222,120,50,224,107,165,97,25,148,20,191,53,96,241,49,81,129,181,93,40,67,175,133,247,115,145,121,84,75,223,160,22,0},
  {66,48,108,158,122,44,152,103,229,32,221,2,152,228,177,66,227,222,32,71,175,145,178,185,127,147,228,14,41,157,192,110,0},
  {126,136,17,40,84,115,4,255,31,54,129,120,86,20,190,40,19,144,1,55,84,140,181,64,238,166,69,132,190,8,210,91,0},
  {8,208,145,188,93,39,126,145,138,44,142,5,201,209,18,218,146,22,137,12,24,119,61,219,201,172,42,123,153,63,159,229,0},
  {49,41,251,71,131,78,87,0,35,53,209,64,135,218,91,95,129,70,119,42,123,5,187,50,164,230,137,80,91,88,197,65,0},
  {93,97,70,22,148,27,117,184,152,192,112,222,247,231,147,169,177,158,219,64,38,33,3,126,16,133,167,62,76,118,149,225,0},
  {141,190,90,236,73,144,189,50,207,184,82,102,249,93,67,78,231,203,228,18,251,139,68,150,78,182,8,252,105,29,13,62,1},
  {111,13,65,39,150,200,200,58,253,154,76,160,106,59,207,119,201,160,208,28,180,32,67,90,242,69,105,34,120,70,142,0,0},
  {42,245,168,171,149,209,9,91,24,87,52,230,85,220,126,222,69,163,48,98,130,181,12,218,98,234,7,90,112,192,83,84,0},
  {17,231,125,108,129,151,180,104,115,140,118,188,144,92,45,254,71,110,57,130,124,241,213,81,99,32,170,84,123,93,141,43,0},
  {229,74,103,141,63,20,223,15,26,155,169,69,200,113,14,92,38,237,104,218,218,104,178,25,83,167,233,61,65,154,210,83,0},
  {90,138,186,229,58,42,128,65,199,105,59,121,35,54,61,204,217,100,17,70,6,214,39,14,21,239,40,82,237,202,222,50,0},
  {62,27,45,160,250,34,184,215,198,155,193,87,104,14,35,226,242,60,127,75,153,5,171,26,217,244,133,116,119,59,122,224,0},
  {224,173,93,72,16,1,131,58,212,4,34,59,155,108,195,82,176,62,23,140,160,161,48,199,199,13,65,232,203,20,241,221,0},
  {121,121,155,104,4,104,124,156,77,161,185,126,48,59,140,173,178,0,150,126,64,242,216,1,115,7,145,52,66,77,156,86,0},
  {247,153,138,3,58,73,75,192,37,157,100,207,10,101,23,28,114,69,175,230,212,83,196,230,180,191,226,237,61,243,46,146,0},
  {230,176,124,10,70,122,91,26,159,37,95,221,50,227,72,169,204,99,70,93,221,132,169,222,129,233,176,129,30,203,152,229,0},
  {15,120,33,48,159,194,147,183,215,54,156,119,133,208,196,214,5,129,145,152,127,18,10,144,224,219,225,100,242,108,101,79,0},
  {167,1,29,49,129,245,249,183,143,251,188,150,151,209,115,166,165,118,250,183,26,57,79,21,85,191,65,166,191,201,178,226,0},
  {107,196,164,146,171,4,159,15,144,166,186,207,125,135,194,226,253,160,150,86,164,112,100,13,238,66,203,120,66,83,101,16,0},
  {148,82,13,175,152,167,145,182,128,172,189,135,197,63,174,66,64,205,75,214,54,139,19,139,198,56,15,78,222,158,119,249,0},
  {19,162,43,187,208,101,241,158,117,177,24,253,210,184,160,154,65,106,173,236,158,48,208,201,155,121,139,213,47,180,224,184,0},
  {83,172,167,145,216,247,82,113,240,244,220,85,56,113,45,184,111,149,176,83,223,154,213,203,140,88,116,202,237,145,52,40,0},
  {124,253,128,117,41,108,29,202,23,254,170,169,212,118,185,180,24,221,61,137,66,235,22,4,231,117,103,40,99,39,154,204,0},
  {226,73,19,152,162,95,45,105,186,66,128,221,228,182,93,245,203,202,137,236,71,232,124,86,133,157,97,208,174,116,30,46,0},
  {89,91,180,113,206,71,1,178,50,154,24,136,73,99,156,64,182,251,10,198,168,83,118,93,115,243,72,233,101,7,107,15,0},
  {206,79,49,233,164,205,90,99,13,239,177,207,94,255,194,154,53,19,68,90,101,199,68,154,65,52,161,101,75,123,113,118,0},
  {154,151,24,243,94,113,28,51,166,188,104,255,208,182,208,212,240,164,80,252,97,69,24,105,246,109,51,205,41,200,108,76,0},
  {64,121,48,83,161,68,2,245,86,175,96,187,75,222,98,16,240,73,208,21,162,145,211,100,214,231,219,203,43,223,214,40,0},
  {52,133,105,67,134,132,43,116,176,162,111,127,16,102,221,138,121,248,38,220,141,30,25,139,133,85,49,186,179,60,71,15,0},
  {70,229,176,18,106,251,184,129,253,195,114,250,19,172,48,77,36,29,55,91,130,203,238,115,24,195,58,198,142,248,40,167,0},
  {112,164,82,139,97,62,130,10,27,55,125,252,142,110,128,95,47,2,61,232,184,197,48,6,104,16,155,171,223,192,89,35,0},
  {190,208,247,211,155,59,75,157,179,250,254,252,189,215,31,67,131,3,63,236,249,194,189,213,208,109,108,138,209,69,193,248,1},
  {195,212,248,226,74,37,57,179,228,212,149,3,56,29,169,51,207,220,254,199,57,150,214,28,90,235,253,21,171,157,55,249,1},
  {255,165,59,4,252,157,248,120,243,44,253,180,197,252,185,108,117,207,103,140,135,24,128,142,70,140,155,225,189,22,211,36,0},
  {181,191,220,203,233,160,210,206,1,145,86,245,157,81,209,216,149,236,44,74,147,78,240,173,11,181,37,82,115,76,250,5,0},
  {29,1,186,38,166,25,107,94,126,203,3,182,237,28,134,27,16,62,100,83,118,66,155,243,241,233,189,95,149,230,247,8,0},
  {210,159,87,255,96,17,153,83,14,98,77,176,147,132,97,224,31,145,85,54,10,224,74,103,64,188,22,226,110,163,107,22,0},
  {76,173,220,109,186,95,116,216,62,108,230,81,64,143,201,53,239,88,186,253,201,3,109,8,244,1,169,220,102,71,189,27,0},
  {96,133,226,52,201,79,156,205,252,142,87,98,60,169,195,153,147,125,204,131,160,247,241,74,201,95,36,22,21,79,107,21,0},
  {54,166,52,180,65,207,217,247,52,226,216,255,155,219,159,105,29,230,137,236,98,63,73,251,131,97,220,106,65,87,20,168,0},
  {208,150,155,48,210,67,120,151,4,173,240,229,117,12,18,63,155,67,118,183,250,105,93,194,145,86,138,3,225,124,7,141,0},
  {149,26,45,113,116,70,47,95,51,219,222,12,204,90,132,194,176,93,201,200,185,65,215,195,57,34,228,110,202,47,169,25,0},
  {241,16,21,139,214,66,244,12,211,67,9,249,78,225,166,176,197,33,255,175,81,20,190,3,98,36,164,96,8,82,42,195,0},
  {10,146,230,112,13,84,206,28,164,135,218,74,29,232,232,2,109,110,199,116,213,135,33,100,37,73,119,232,73,241,147,212,0},
  {108,70,18,238,34,153,50,10,20,64,101,210,204,188,154,173,222,179,182,223,153,116,154,157,128,200,164,200,36,106,141,54,0},
  {245,171,111,80,134,249,172,93,28,33,1,185,246,12,244,174,120,46,65,202,57,96,255,211,85,76,201,80,218,16,240,69,0},
  {242,85,168,64,59,31,74,217,247,26,172,63,140,87,91,217,142,210,135,30,245,129,254,32,163,57,109,218,179,210,181,153,0},
  {118,173,18,240,139,31,58,166,73,214,236,231,203,65,140,213,148,241,218,146,249,73,69,26,34,16,222,28,172,17,27,228,0},
  {201,203,49,247,94,189,26,226,146,64,224,110,217,106,165,142,108,225,107,89,114,234,161,25,244,151,106,205,7,190,246,52,0},
  {255,19,90,217,255,0,128,224,20,1,148,40,83,205,84,84,165,62,37,174,124,85,38,53,40,252,196,125,97,149,110,171,0},
  {211,180,113,95,215,200,92,227,150,31,167,198,135,86,173,162,80,27,232,101,211,160,248,110,159,133,107,211,168,58,106,151,0},
  {103,197,24,44,169,145,191,232,180,145,83,223,37,90,49,228,95,244,186,197,248,216,89,17,31,148,158,193,22,104,51,47,0},
  {116,206,157,129,17,137,64,12,112,19,56,150,93,252,205,252,49,86,59,188,154,131,254,222,0,121,150,28,165,255,247,137,0},
  {120,103,31,230,182,239,69,236,65,16,9,114,175,98,139,133,70,51,150,24,202,31,137,78,106,125,160,250,107,220,7,150,0},
  {20,34,70,138,6,117,245,37,186,76,131,83,174,95,215,192,24,253,109,124,102,52,251,210,167,98,46,54,121,111,62,63,0},
  {55,88,87,251,10,197,49,177,238,221,225,75,124,110,119,127,168,72,234,68,102,17,32,167,101,52,50,174,6,161,4,125,0},
  {7,62,151,237,172,86,187,58,29,196,83,25,157,21,213,201,228,225,228,158,187,235,59,119,233,220,142,248,96,19,79,65,0},
  {107,43,89,86,247,102,229,131,249,152,25,245,66,71,250,216,38,159,91,238,127,109,121,59,102,44,253,208,87,116,141,0,0},
  {8,17,192,63,50,158,168,102,129,177,21,70,78,37,56,138,173,12,163,34,76,192,35,118,10,70,17,163,254,193,176,205,0},
  {200,9,16,33,63,225,14,34,183,215,65,22,65,3,89,181,80,108,26,189,159,229,64,48,180,69,121,133,123,149,199,34,0},
  {125,132,107,105,210,208,156,202,80,157,75,72,33,40,179,247,181,210,161,224,233,119,166,69,19,110,19,208,31,102,50,171,0},
  {5,204,97,195,120,117,198,174,183,127,170,92,124,133,192,69,27,67,54,19,70,138,109,209,94,200,113,215,86,185,245,228,0},
  {255,24,95,153,139,29,46,0,231,218,135,243,163,160,101,196,133,223,78,170,96,117,27,56,180,88,152,216,149,114,207,202,0},
  {54,27,212,85,95,160,199,90,116,104,191,57,45,222,41,22,219,99,245,78,12,56,14,255,120,13,0,164,3,106,209,162,0},
  {189,230,15,155,214,237,120,15,10,176,36,34,170,194,156,204,227,221,126,68,79,6,196,101,43,32,153,35,120,182,190,189,0},
  {20,68,46,35,213,144,84,147,26,72,49,234,68,118,158,193,190,126,115,230,76,226,28,234,240,18,177,207,39,74,242,177,0},
  {231,152,158,248,99,125,191,99,51,241,205,255,198,147,204,11,12,6,50,123,146,75,15,144,32,199,62,23,143,193,187,67,0},
  {112,182,218,230,100,12,144,223,152,5,48,247,181,187,16,2,204,27,69,12,242,149,142,220,93,19,184,9,224,114,165,200,0},
  {32,108,149,172,24,17,19,69,247,138,153,110,171,166,206,10,22,39,218,22,221,31,7,102,185,31,78,170,181,109,151,133,0},
  {176,103,204,52,144,92,167,238,89,64,214,119,85,215,38,88,233,146,72,8,52,198,251,149,92,88,200,88,43,233,191,141,0},
  {158,9,173,207,212,76,100,39,39,219,190,56,199,130,209,91,34,3,73,180,50,202,129,131,138,97,82,224,209,20,83,134,0},
  {140,118,145,18,189,236,122,204,159,143,57,252,100,127,144,220,198,52,207,154,7,68,82,239,173,38,39,248,215,63,245,65,0},
  {134,80,198,26,66,229,227,41,64,199,143,187,214,39,86,56,189,72,68,79,15,227,135,136,209,208,9,83,113,91,41,188,0},
  {78,134,217,103,144,107,124,56,91,147,41,235,238,163,23,169,129,90,233,229,226,160,226,79,227,84,172,155,126,28,81,89,0},
  {191,187,120,44,116,84,59,130,25,178,28,141,111,36,175,99,101,231,32,62,158,108,202,68,140,28,225,37,148,36,106,246,0},
  {104,165,206,237,111,169,182,158,80,219,96,26,198,139,230,201,74,238,211,251,12,2,12,86,157,243,215,146,188,27,20,212,0},
  {62,99,61,185,77,186,42,211,64,182,27,209,75,169,210,207,112,138,220,172,38,202,145,28,42,49,201,238,99,138,176,66,0},
  {255,128,95,36,155,42,108,241,68,206,24,93,254,14,251,124,65,209,252,42,155,254,194,221,187,227,180,196,102,15,192,200,0},
  {71,124,174,113,113,200,75,165,71,238,138,181,147,104,175,96,79,165,53,47,91,89,224,94,91,30,4,255,207,155,107,131,0},
  {192,199,166,23,122,9,41,147,126,255,96,11,123,70,188,210,117,13,21,115,75,145,7,149,34,24,87,236,81,212,105,123,0},
  {95,57,218,46,33,122,19,132,187,143,62,23,141,219,208,93,173,20,245,7,145,198,127,96,189,169,123,183,109,174,142,24,0},
  {24,220,23,138,41,174,187,235,127,134,105,202,13,154,221,103,5,175,154,27,87,88,230,85,253,27,9,118,123,97,123,139,0},
  {6,127,154,241,105,197,73,207,78,140,216,10,209,0,1,111,198,43,91,22,202,85,111,148,153,235,217,179,224,18,139,175,0},
  {114,185,212,101,17,176,14,73,225,55,117,235,124,78,245,62,227,93,182,113,87,123,236,48,21,16,86,42,56,29,43,230,0},
  {68,53,138,202,55,37,154,26,169,237,166,137,47,227,201,78,52,83,158,203,239,50,51,26,229,77,29,187,41,153,116,172,0},
  {199,163,182,225,35,57,239,34,221,184,129,24,114,207,139,77,157,40,230,186,60,247,92,230,171,160,97,212,32,105,204,6,0},
  {28,48,13,170,57,220,199,222,31,116,43,138,243,164,133,78,30,134,195,235,16,115,49,235,223,176,216,229,227,159,80,162,0},
  {77,231,65,175,171,54,11,105,191,213,248,116,225,246,165,83,134,23,165,231,13,129,111,80,127,44,64,244,235,34,216,56,0},
  {116,219,203,77,47,170,20,249,30,225,2,105,203,150,213,232,245,243,199,78,218,56,198,32,163,96,187,178,187,111,233,59,0},
  {12,149,25,189,53,59,73,57,118,143,16,122,232,76,20,173,233,105,35,44,198,170,87,24,44,121,34,191,243,120,39,192,0},
  {183,133,90,205,224,187,120,46,193,197,200,18,55,182,2,19,163,111,83,210,101,6,34,219,91,242,86,82,69,240,95,183,0},
  {3,193,235,82,80,166,219,203,130,123,255,154,163,102,180,121,172,4,45,161,154,164,146,203,130,92,197,6,229,183,104,232,0},
  {17,131,117,98,52,166,242,131,110,136,225,67,241,27,174,176,210,247,252,96,94,40,234,112,134,11,22,19,82,54,123,235,0},
  {89,48,120,174,189,168,67,136,147,144,24,102,140,217,245,111,175,180,8,120,97,18,106,164,203,202,109,5,149,151,154,185,0},
  {95,26,214,180,169,50,120,33,235,33,135,9,24,42,105,156,222,214,42,58,71,126,68,38,96,160,234,53,96,81,218,235,0},
  {8,241,97,132,151,205,153,98,62,32,240,186,229,71,248,54,98,52,11,46,138,223,98,9,203,105,170,64,169,144,14,248,0},
  {71,254,121,172,26,226,43,20,59,222,179,97,129,67,69,100,170,14,39,29,53,70,38,149,106,111,216,22,214,129,19,50,0},
  {58,22,218,58,46,250,154,56,84,211,162,130,30,66,231,90,37,37,113,141,77,65,231,184,112,84,152,40,240,199,220,109,0},
  {229,29,91,95,230,172,180,94,111,43,153,238,158,66,21,110,153,171,251,243,51,236,238,162,136,169,65,99,39,105,26,244,0},
  {42,154,181,199,6,43,244,102,32,254,229,151,112,171,65,44,224,58,97,19,227,236,166,230,56,183,27,177,78,54,12,45,0},
  {53,183,37,164,130,100,28,213,40,148,71,208,23,49,203,73,144,205,63,92,192,89,221,71,160,71,243,113,228,166,23,7,0},
  {196,187,46,107,131,239,107,108,70,83,225,30,69,139,115,178,7,79,61,53,254,226,168,224,141,101,195,124,82,57,180,144,0},
  {164,45,65,239,131,91,209,132,251,220,19,236,251,255,42,141,199,39,55,21,196,206,212,1,130,81,215,163,29,31,191,189,0},
  {155,185,177,145,56,110,246,62,7,227,190,186,160,36,126,241,82,87,168,123,203,131,246,166,246,105,239,67,219,213,42,121,0},
  {222,79,171,162,9,243,141,33,197,62,62,217,119,110,176,50,100,89,159,237,228,99,30,46,174,37,238,201,47,97,144,77,0},
  {61,229,205,97,254,155,235,98,129,166,180,47,176,52,24,219,139,33,118,198,152,129,196,250,218,71,4,169,38,204,169,252,0},
  {84,60,226,93,55,50,198,219,139,144,57,250,189,155,243,228,138,127,217,137,51,34,175,116,163,114,71,27,4,113,8,150,0},
  {172,20,123,108,148,102,186,89,194,53,169,189,51,152,114,124,204,59,186,9,134,107,123,150,49,52,132,185,203,123,136,23,0},
  {252,174,110,147,150,114,141,212,103,252,204,100,120,178,89,146,225,64,218,205,154,196,157,163,8,235,7,220,65,159,111,126,0},
  {115,191,92,79,27,34,234,71,173,17,26,226,138,2,173,24,175,181,255,35,149,21,84,244,225,92,63,192,227,107,31,75,0},
  {66,212,59,97,232,251,10,32,205,47,158,20,164,123,62,218,241,179,190,9,243,126,211,215,240,207,35,66,78,220,188,109,0},
  {169,208,153,159,119,139,209,205,27,148,13,159,44,107,42,208,84,245,51,200,225,13,27,30,196,224,150,202,28,25,198,131,0},
  {82,218,112,210,39,143,249,187,111,26,168,224,208,105,170,24,138,122,89,31,166,180,243,151,33,242,80,115,4,74,252,72,0},
  {100,51,23,48,217,96,93,253,69,253,170,112,170,189,149,41,28,121,238,73,197,28,81,12,225,43,97,174,222,62,35,79,0},
  {204,76,252,214,59,238,164,106,236,158,77,84,78,111,232,107,229,15,22,205,75,111,200,152,217,33,248,157,82,108,214,29,0},
  {162,7,104,250,204,252,190,118,11,79,0,230,238,0,135,193,107,216,152,99,250,141,250,108,75,176,156,244,180,114,225,171,0},
  {113,142,3,188,224,225,64,16,42,75,164,117,63,185,157,176,168,16,26,62,172,46,186,197,113,146,221,227,160,222,40,3,0},
  {145,204,129,76,111,90,122,151,18,62,83,41,50,222,93,73,33,103,240,182,178,62,145,226,135,111,76,85,164,30,207,121,0},
  {253,32,158,246,185,200,100,75,108,192,60,202,95,140,115,57,96,43,23,116,184,233,40,41,57,74,163,161,78,130,250,47,0},
  {143,137,99,37,83,232,154,92,209,108,156,13,53,131,249,129,3,131,162,58,46,162,43,60,54,138,180,147,66,74,168,159,0},
  {17,50,68,66,138,207,255,176,109,34,252,145,38,125,70,58,228,133,74,228,78,31,243,33,120,161,61,117,114,49,229,112,0},
  {180,42,203,20,96,93,224,34,43,104,178,173,192,206,15,51,30,225,185,223,31,164,25,79,78,186,18,242,210,84,185,88,1},
  {170,248,231,87,169,14,107,153,250,229,207,222,36,155,164,10,88,78,83,89,106,65,173,153,200,142,163,8,96,10,80,158,0},
  {103,160,252,150,179,149,236,129,119,38,138,37,37,61,76,152,141,116,252,242,118,196,190,168,252,45,3,117,99,98,180,11,0},
  {197,180,25,226,65,174,162,231,187,190,128,89,124,246,161,158,26,201,118,220,187,143,118,194,50,141,24,167,56,49,122,77,0},
  {28,192,229,46,163,226,32,247,83,76,190,145,182,169,223,116,30,109,154,149,79,230,170,41,192,71,73,33,172,225,239,79,0},
  {19,96,218,16,8,153,11,135,6,183,147,120,154,80,131,229,181,210,188,110,137,145,179,196,34,39,193,71,253,183,166,95,0},
  {235,100,123,252,248,96,118,43,67,163,70,32,242,185,195,186,42,16,54,80,42,69,218,6,61,27,173,116,54,146,193,34,0},
  {164,204,77,160,107,164,249,130,209,67,217,113,228,212,19,207,242,104,198,209,177,230,207,233,16,14,169,46,87,3,152,31,0},
  {195,161,158,212,169,55,66,22,247,245,255,251,146,34,248,154,58,167,3,112,124,165,182,183,175,199,225,5,196,12,88,118,0},
  {29,133,75,26,14,7,216,176,177,131,46,232,245,191,123,162,132,179,215,23,121,95,90,50,80,87,7,200,243,156,66,217,0},
  {242,1,23,67,30,255,234,221,144,234,121,206,183,149,3,95,203,9,78,198,131,117,137,10,161,154,90,24,136,210,244,198,0},
  {140,127,232,127,42,139,245,211,115,49,137,247,0,14,121,94,246,77,162,229,60,112,210,155,106,1,132,89,228,25,169,62,0},
  {152,145,26,162,12,22,19,131,144,45,225,129,138,147,212,143,183,147,150,225,139,229,99,49,51,204,204,97,23,187,209,176,0},
  {1,198,186,155,42,13,193,149,240,112,73,41,220,134,236,255,26,218,110,116,134,123,141,115,44,224,219,46,115,212,245,15,0},
  {225,41,205,108,217,212,116,206,149,104,183,105,139,86,70,92,199,5,97,113,145,73,158,167,223,176,161,213,200,134,233,155,0},
  {224,87,175,131,73,104,125,235,12,179,232,8,65,24,22,185,74,178,78,123,189,161,115,179,122,19,210,58,105,52,34,184,0},
  {178,52,25,199,90,86,164,55,188,96,66,234,37,235,245,156,179,116,31,108,118,11,114,193,69,161,163,240,131,70,136,41,0},
  {85,46,68,184,32,39,85,215,14,37,191,242,205,247,233,219,40,252,20,2,124,192,230,205,143,154,50,107,179,132,174,56,0},
  {9,243,61,41,221,46,28,109,249,126,189,32,63,44,91,250,250,148,190,216,153,35,38,255,98,58,31,14,16,207,218,103,0},
  {119,41,16,238,183,228,113,21,134,7,19,136,104,62,44,43,195,214,140,138,66,115,132,164,66,130,164,51,9,221,14,112,0},
  {57,83,133,85,54,122,202,157,26,71,227,204,227,96,115,93,121,66,204,30,215,212,168,81,140,220,209,99,94,205,66,188,0},
  {85,131,34,39,133,3,77,93,255,199,226,43,87,0,11,50,186,31,208,244,85,75,227,216,11,153,16,168,54,170,62,156,0},
  {108,187,90,43,9,35,178,142,161,240,55,101,92,85,180,233,170,4,58,82,212,95,36,102,168,64,179,218,64,185,249,136,0},
  {176,147,15,138,8,9,139,54,188,247,58,123,226,190,104,64,29,197,33,9,7,87,21,20,96,196,204,99,39,238,94,156,0},
  {20,242,117,252,25,179,45,243,102,34,226,222,49,157,239,97,35,44,139,16,7,90,75,80,49,51,157,128,31,106,210,185,0},
  {192,52,89,109,71,234,255,195,139,173,95,153,38,119,131,232,225,73,89,113,228,62,41,166,9,21,53,83,211,226,171,120,1},
  {82,108,228,253,226,128,19,98,172,63,94,238,167,19,62,163,143,33,126,13,127,153,70,102,66,163,232,249,109,196,106,142,0},
  {65,18,167,198,136,217,142,219,197,159,173,15,141,124,121,224,244,194,232,205,129,65,213,26,17,197,194,215,24,230,183,159,0},
  {180,153,173,48,95,67,84,30,93,52,3,136,190,168,74,195,211,232,146,202,169,24,76,61,238,37,31,234,223,213,0,89,0},
  {202,165,120,117,22,86,74,86,42,94,98,255,206,37,2,35,223,183,125,67,52,172,84,145,249,32,83,201,229,223,96,58,0},
  {252,149,85,217,212,247,232,213,144,197,71,191,222,89,29,96,101,121,15,12,140,126,159,130,216,149,56,127,205,179,255,42,0},
  {253,124,193,98,4,18,41,163,123,41,18,189,233,126,188,187,68,144,108,131,84,61,128,51,89,113,59,241,74,77,95,34,0},
  {188,29,254,226,86,215,157,94,245,37,243,238,40,143,25,94,21,253,188,156,53,228,75,108,131,54,57,155,48,211,213,129,0},
  {209,34,147,218,189,70,178,72,40,129,67,159,66,200,19,63,72,79,152,135,124,35,56,168,164,223,207,22,252,93,111,64,0},
  {93,76,193,194,54,30,117,98,14,53,17,169,170,27,204,4,157,228,41,224,171,146,185,161,70,199,87,15,161,183,240,201,0},
  {61,103,197,180,235,223,40,225,115,79,9,17,37,200,186,138,126,90,206,172,135,235,5,78,243,216,63,167,186,125,13,227,0},
  {178,50,149,29,111,242,184,196,63,52,27,241,10,33,85,253,53,225,66,169,232,210,116,220,191,158,106,223,113,13,239,23,0},
  {227,250,49,27,215,124,198,231,252,72,129,179,12,77,154,82,38,161,85,154,254,197,77,116,127,154,184,117,48,43,32,60,0},
  {162,168,183,171,141,199,160,230,69,138,6,31,77,19,221,73,161,80,198,208,166,13,140,104,184,208,145,20,174,145,219,6,0},
  {130,227,156,88,100,152,79,46,54,66,60,198,45,98,139,132,108,44,137,25,30,167,75,44,44,112,224,15,7,12,108,208,0},
  {223,99,10,15,80,12,136,175,186,78,202,203,77,55,128,186,46,141,153,129,90,46,50,209,92,83,37,17,97,71,174,136,0},
  {7,226,229,121,219,1,69,163,157,234,153,155,116,164,105,100,204,236,238,68,221,194,115,28,120,91,129,20,12,129,194,204,0},
  {146,185,22,158,22,156,200,222,72,243,18,73,154,74,6,38,187,161,199,209,251,39,188,210,166,238,217,23,208,29,115,93,0},
  {220,20,252,235,233,155,5,174,46,223,217,163,86,123,197,228,116,137,70,218,144,179,140,238,49,184,97,96,222,242,172,41,0},
  {87,180,129,206,116,98,229,165,211,57,20,123,16,172,247,177,38,34,77,129,141,13,104,4,71,78,221,151,64,9,227,207,0},
  {138,83,62,171,164,228,152,62,149,33,204,90,73,129,157,187,205,251,174,188,1,90,163,25,164,67,106,197,228,67,103,131,0},
  {82,112,12,96,103,197,179,28,20,128,39,136,107,88,132,43,184,43,41,231,251,31,57,16,215,194,190,240,131,101,34,18,0},
  {228,191,234,206,207,194,2,143,218,110,155,104,57,184,134,79,67,238,190,112,250,40,246,2,130,207,114,57,33,127,128,171,0},
  {161,90,36,53,141,110,156,20,122,51,220,124,82,170,27,230,90,141,81,51,162,61,186,105,188,105,136,219,9,177,190,179,0},
  {3,94,55,1,159,2,144,103,7,54,238,41,106,185,84,26,82,215,247,0,224,206,147,124,2,12,94,9,220,41,21,113,0},
  {199,3,164,43,140,80,61,67,252,2,205,9,88,166,121,232,198,78,113,232,151,210,14,135,34,187,162,208,75,178,124,54,0},
  {144,167,68,89,193,115,3,57,150,193,215,228,142,146,187,195,182,13,35,165,206,236,185,148,20,101,65,138,166,145,41,67,0},
  {35,35,205,103,235,11,200,107,209,181,106,171,31,196,94,21,223,44,28,147,190,168,239,128,144,166,158,53,21,239,235,184,0},
  {210,175,83,242,204,216,185,214,0,117,34,62,223,145,219,10,120,30,212,212,149,103,66,7,172,52,103,245,174,61,248,82,0},
  {25,244,154,144,157,199,133,159,60,157,193,240,22,84,206,193,141,203,216,178,252,141,52,226,8,6,210,29,113,16,148,225,0},
  {23,191,194,22,148,166,227,27,30,97,0,222,234,91,50,29,121,36,234,116,245,144,249,101,95,120,5,35,198,48,3,15,0},
  {41,83,21,146,88,185,198,30,244,178,108,215,138,32,1,207,175,165,94,241,7,236,67,24,124,136,86,143,226,183,83,203,0},
  {251,74,33,48,141,14,131,64,221,144,162,40,157,123,205,79,188,203,169,171,127,135,65,62,70,57,68,207,173,140,150,124,0},
  {175,33,162,150,85,64,145,43,28,221,190,87,193,114,108,146,36,17,119,159,90,246,246,53,193,236,185,28,173,102,235,161,0},
  {59,36,194,168,201,220,196,70,67,10,222,249,45,198,10,44,106,250,35,112,163,206,167,154,189,93,76,44,66,228,100,161,1},
  {114,9,49,40,212,23,179,75,28,102,210,160,169,223,176,133,248,104,224,175,49,117,118,125,157,123,64,213,142,31,130,12,0},
  {42,171,2,7,246,166,55,178,44,119,67,67,62,24,56,206,211,215,209,199,5,51,88,121,55,84,165,31,100,239,175,84,0},
  {10,185,192,100,193,13,22,66,210,15,216,17,10,70,195,98,40,7,58,200,188,174,82,233,74,33,211,12,91,242,52,126,0},
  {156,2,163,240,192,93,137,199,248,250,21,15,159,122,156,90,25,108,170,247,81,138,7,189,83,66,214,106,240,54,66,232,0},
  {236,18,162,230,26,28,181,47,92,243,181,213,140,35,191,229,20,250,118,64,147,146,108,21,29,13,151,245,187,145,214,73,0},
  {190,68,204,69,229,214,239,30,153,193,35,157,226,239,225,120,25,174,74,29,238,136,100,124,106,17,82,226,73,187,211,20,0},
  {190,35,37,39,77,143,53,98,17,237,222,54,77,242,166,57,115,92,236,44,62,174,237,140,239,65,248,104,41,65,30,111,0},
  {232,234,160,8,92,243,54,161,222,76,228,225,185,75,37,106,124,0,79,153,141,102,6,216,92,150,89,140,43,118,196,64,0},
  {197,121,33,225,12,194,117,115,248,227,67,153,25,61,94,204,79,105,91,104,224,16,84,203,177,138,189,233,131,254,10,98,0},
  {208,77,232,76,135,123,240,130,253,231,215,253,203,127,129,57,238,35,69,224,221,210,15,154,221,69,170,14,189,225,193,70,0},
  {71,165,71,49,221,111,50,150,144,109,166,113,59,147,170,5,96,2,14,67,19,6,230,43,156,217,111,37,7,21,9,42,0},
  {62,210,131,16,27,120,66,42,7,217,158,191,234,49,8,252,214,229,103,30,142,26,165,222,165,248,29,3,152,44,160,231,0},
  {155,44,143,156,178,35,187,10,204,170,176,150,171,225,64,69,227,179,174,96,238,148,144,58,55,48,94,69,238,147,29,74,0},
  {164,200,251,80,141,175,165,46,168,58,119,86,99,254,16,153,98,6,35,244,121,39,59,73,239,253,210,184,79,71,53,158,0},
  {24,108,121,29,123,18,89,201,0,22,250,171,43,248,34,210,128,201,131,183,216,30,158,106,204,43,145,12,36,222,113,78,0},
  {132,157,198,91,20,211,87,216,89,127,89,41,118,214,90,146,63,11,117,208,85,87,142,53,171,87,129,97,66,122,132,222,0},
  {80,85,196,250,69,253,94,223,32,90,241,226,45,250,73,99,141,188,4,112,61,32,75,42,156,75,140,2,246,159,66,25,0},
  {79,245,189,181,161,97,113,252,200,68,132,187,222,179,160,96,166,154,242,93,125,45,222,205,189,185,51,126,111,128,252,91,0},
  {225,241,204,216,255,203,85,53,148,16,104,88,19,180,44,163,88,64,122,170,181,223,80,170,111,21,224,11,156,76,6,181,0},
  {198,247,142,219,73,90,65,222,13,74,42,233,166,233,50,193,57,255,128,222,34,192,93,92,71,213,58,112,52,152,118,64,0},
  {66,204,116,163,189,198,214,73,223,140,42,159,2,82,125,14,78,4,176,152,193,86,58,229,176,33,212,84,47,93,221,103,0},
  {112,12,82,45,212,169,254,58,217,11,206,128,137,26,186,245,188,198,101,145,76,62,42,170,160,75,242,45,245,192,31,11,0},
  {16,97,169,139,87,198,116,75,23,241,99,82,210,232,198,16,136,14,156,247,110,104,8,30,113,250,112,93,145,44,155,36,1},
  {82,235,197,221,58,177,215,157,160,234,29,145,15,243,176,121,23,123,138,235,49,161,65,147,194,57,120,231,192,34,212,68,0},
  {63,58,168,61,54,143,174,251,141,222,44,163,212,255,42,27,173,48,233,60,1,33,174,138,112,38,5,8,242,170,27,130,0},
  {128,182,122,216,4,148,92,90,129,15,121,127,190,243,189,221,252,246,92,37,158,73,228,104,74,159,201,89,55,110,120,16,0},
  {58,156,110,9,80,101,157,77,247,119,74,79,172,89,124,84,44,158,234,83,137,44,158,74,29,240,253,241,176,203,241,145,0},
  {215,138,169,142,137,200,172,15,131,147,146,165,8,215,97,182,80,240,186,243,172,2,78,236,187,192,20,182,199,20,123,92,0},
  {20,203,245,30,250,36,252,204,38,57,201,16,83,133,239,160,45,188,186,205,129,245,35,9,194,178,149,81,66,148,242,205,0},
  {13,88,203,40,233,19,57,252,246,231,216,85,148,0,35,17,209,216,176,56,182,7,24,221,84,118,200,186,135,102,193,67,0},
  {30,164,131,247,247,116,113,65,227,89,53,151,189,111,118,64,152,104,161,222,191,243,211,115,28,70,80,35,196,65,57,135,0},
  {181,240,55,244,182,164,166,157,89,89,224,13,231,191,223,132,125,225,80,109,168,42,231,163,96,226,93,231,204,228,128,135,0},
  {53,232,219,238,243,176,146,221,153,68,43,106,117,194,171,138,125,48,71,72,99,69,30,94,235,154,148,136,36,99,104,179,0},
  {235,96,246,82,173,40,188,73,224,49,96,139,37,47,220,209,94,213,139,58,253,178,92,43,76,149,252,114,140,55,90,166,0},
  {94,102,18,22,192,243,176,241,136,198,106,204,0,221,136,33,211,181,112,41,14,159,144,225,48,160,171,63,108,131,79,77,0},
  {58,198,253,34,117,240,205,250,186,31,242,221,186,254,182,188,93,213,61,5,78,6,211,226,20,55,76,163,127,223,196,107,0},
  {163,111,57,221,35,61,216,135,132,215,209,146,150,248,156,247,115,24,23,73,135,76,204,16,72,169,204,68,219,26,6,2,0},
  {183,66,185,25,239,245,38,189,205,145,87,151,18,213,63,161,228,156,38,71,243,84,20,100,89,79,80,3,191,12,19,211,0},
  {165,219,225,130,241,216,20,249,198,112,215,14,248,238,170,91,39,90,22,212,201,131,179,180,170,69,0,42,221,92,41,169,0},
  {55,125,223,79,223,218,60,45,178,50,199,200,81,45,119,47,167,168,153,237,46,42,30,149,37,40,26,2,153,137,177,34,0},
  {204,33,182,22,8,190,85,19,220,2,226,71,40,81,44,37,125,44,206,33,223,144,166,187,16,226,44,156,218,108,156,123,0},
  {166,122,8,143,42,117,138,63,36,139,187,206,236,142,227,185,3,61,136,203,15,51,50,231,95,228,132,248,151,153,63,31,0},
  {78,242,158,211,160,131,27,233,103,49,9,112,16,229,48,104,77,40,209,207,239,30,45,27,222,105,206,190,136,219,42,23,0},
  {237,229,192,81,87,9,130,26,161,245,219,222,206,58,23,3,243,174,188,121,160,141,71,157,10,195,226,30,34,75,201,10,0},
  {176,97,9,153,67,111,171,114,173,68,120,211,72,254,38,96,85,87,171,114,78,251,20,75,159,198,75,140,95,17,118,212,0},
  {136,249,48,195,129,239,39,161,236,232,216,19,100,140,145,176,180,236,182,89,203,6,234,216,163,51,6,61,98,108,131,118,0},
  {67,87,57,116,116,249,148,221,183,132,186,23,217,11,73,43,223,179,222,93,191,98,211,97,182,117,125,89,254,244,248,13,1},
  {211,139,150,109,86,70,72,151,207,75,55,46,162,8,210,112,204,206,242,170,250,148,64,34,201,28,13,226,90,190,10,249,0},
  {136,26,115,219,117,102,216,214,244,64,153,135,212,60,252,222,13,39,35,251,129,234,95,134,218,118,219,0,233,5,92,106,0},
  {134,254,137,2,121,41,135,55,92,56,69,83,138,47,193,90,191,212,92,218,214,171,237,23,226,188,60,231,223,53,231,116,0},
  {54,96,25,43,27,132,7,162,55,220,219,111,178,72,204,56,43,183,191,220,148,105,38,34,224,119,207,37,58,141,100,68,0},
  {53,231,56,245,72,21,108,139,108,44,215,172,190,110,98,122,250,215,63,116,161,183,16,229,87,25,6,27,81,142,11,129,0},
  {3,203,78,167,128,154,162,210,153,240,140,63,244,149,81,158,109,30,237,245,254,163,231,130,4,66,58,238,116,179,115,246,0},
  {60,10,175,169,0,186,208,193,137,123,201,33,242,223,46,162,79,125,186,79,39,45,52,82,187,224,47,244,157,4,241,195,0},
  {60,79,47,167,252,76,163,21,137,191,70,129,147,50,123,113,28,210,21,215,212,101,239,129,40,173,131,208,251,65,222,232,0},
  {89,241,213,216,184,40,215,87,146,40,142,10,226,213,5,158,47,20,108,165,101,58,13,171,100,73,192,41,97,197,128,103,0},
  {29,238,164,66,136,242,54,17,255,198,96,229,117,62,102,155,10,114,128,61,191,193,214,127,18,15,166,70,9,219,253,113,0},
  {53,189,247,61,203,45,14,87,114,118,252,12,250,197,145,211,11,93,150,183,253,46,254,19,58,244,122,25,130,56,244,8,0},
  {17,2,251,49,204,212,35,49,191,249,162,49,28,115,151,75,167,96,167,189,35,54,94,58,53,156,106,205,79,187,53,155,0},
  {244,56,191,72,237,248,0,47,107,22,170,34,188,62,206,59,180,170,169,95,88,227,4,144,110,188,110,94,112,193,12,46,0},
  {104,158,120,157,33,210,119,66,141,15,254,232,22,15,224,24,159,178,181,254,14,110,84,12,230,253,111,118,224,129,192,162,0},
  {212,137,249,139,67,192,231,21,2,180,67,155,210,2,252,226,120,49,189,33,62,12,178,41,3,193,209,115,171,44,22,34,0},
  {142,198,90,148,77,241,97,195,74,13,4,181,21,94,158,83,132,89,8,19,35,237,160,240,47,170,138,38,183,18,214,146,0},
  {172,15,32,44,236,230,214,84,177,194,24,202,68,196,118,57,44,52,216,28,225,33,46,83,144,248,137,53,235,249,146,41,0},
  {219,205,196,118,109,104,75,78,200,3,75,44,254,230,51,80,31,14,173,152,215,202,5,103,99,226,91,52,95,151,213,202,0},
  {237,35,252,175,254,29,232,172,91,234,16,211,50,233,181,191,113,32,237,206,87,200,254,247,233,218,5,41,11,42,87,138,0},
  {39,161,48,124,76,218,214,65,32,126,189,231,202,109,66,122,200,93,34,124,69,75,190,129,121,172,22,192,95,222,179,248,0},
  {217,12,215,20,215,223,157,220,51,105,90,33,120,40,65,127,70,244,99,78,221,4,185,221,238,250,27,208,59,2,139,154,0},
  {55,52,233,241,230,34,83,143,171,246,229,9,87,25,223,156,228,254,160,36,210,216,64,162,53,172,93,148,121,146,43,192,0},
  {196,167,30,14,10,153,90,181,217,188,78,128,150,244,152,92,114,29,57,166,244,129,224,156,117,7,73,57,154,122,217,142,0},
  {44,17,5,243,14,116,35,189,106,235,187,90,163,130,102,84,98,235,183,48,0,84,13,228,202,82,64,192,217,235,254,90,1},
  {125,148,91,223,153,201,60,155,110,18,122,250,175,202,132,24,226,253,137,178,79,19,144,39,151,169,122,70,73,230,198,45,1},
  {240,20,151,82,156,223,39,19,131,65,88,136,255,184,176,42,217,70,51,130,83,9,143,194,4,251,103,102,133,26,76,141,1},
  {251,98,125,147,146,54,37,27,230,2,80,106,139,155,246,208,145,58,147,247,87,236,24,21,28,132,22,46,141,107,58,10,1},
  {61,135,30,168,179,157,221,230,0,41,246,221,162,152,124,17,88,96,116,40,49,196,102,85,232,121,243,62,99,247,148,35,1},
  {145,111,200,31,44,143,139,105,250,170,39,69,148,156,22,190,107,142,166,152,64,39,3,231,52,23,18,16,166,118,118,90,0},
  {76,84,78,210,37,75,244,108,245,154,178,180,166,203,31,107,138,113,1,119,47,52,27,142,229,70,9,187,72,89,56,49,0},
  {142,251,32,76,1,36,120,255,116,38,168,25,161,157,2,54,134,36,101,100,141,149,203,253,247,10,125,34,4,214,93,113,0},
  {110,165,144,122,238,139,107,215,61,195,98,203,213,87,64,232,106,254,181,231,46,132,11,206,198,143,136,25,82,250,221,22,0},
  {19,156,134,235,126,190,11,5,225,2,41,163,245,207,23,200,30,239,38,59,102,173,216,196,87,111,242,196,0,229,246,200,0},
  {22,108,142,74,86,189,140,68,10,130,141,192,12,59,33,152,75,121,252,151,180,180,161,223,37,212,235,147,111,147,249,225,0},
  {196,211,118,205,0,71,125,122,97,137,50,247,211,201,200,219,169,248,95,173,13,54,133,74,104,206,4,250,168,104,136,52,0},
  {181,139,201,60,58,135,163,190,31,55,133,151,2,209,69,11,194,5,148,128,39,93,249,171,158,96,2,242,81,124,237,9,0},
  {247,0,169,141,199,163,136,79,74,162,2,44,212,2,155,131,201,39,62,164,225,37,213,85,208,61,182,57,165,151,29,164,0},
  {169,7,33,75,22,162,82,106,14,167,37,103,235,188,161,139,34,203,3,185,81,154,141,226,21,27,173,219,68,188,37,30,0},
  {92,125,44,181,81,185,182,110,204,208,209,48,103,193,55,133,163,10,159,4,232,223,162,161,208,111,10,247,24,99,8,82,0},
  {180,88,129,167,183,83,153,252,186,230,255,211,246,183,169,165,232,236,91,82,111,143,165,1,6,208,42,47,252,106,161,85,0},
  {72,84,58,168,164,171,219,193,241,254,96,126,74,59,32,48,180,110,112,246,44,69,219,200,13,96,193,79,24,194,159,201,0},
  {132,185,122,66,237,0,216,167,4,89,27,80,40,65,10,75,59,167,140,169,194,81,98,122,98,159,118,191,129,10,243,119,0},
  {145,211,114,94,154,103,103,227,189,122,227,169,149,200,57,78,76,96,9,254,168,139,187,208,178,122,8,38,13,202,28,41,0},
  {233,166,245,105,109,67,122,68,225,255,35,81,216,44,29,126,26,102,76,149,179,8,88,111,42,246,214,170,25,250,204,142,0},
  {18,197,157,39,130,197,224,164,81,107,101,200,42,6,106,204,211,179,199,105,224,205,231,111,248,79,175,89,177,71,46,4,0},
  {189,178,166,64,103,104,249,173,2,56,220,111,64,32,248,162,185,138,126,94,134,158,133,174,143,229,190,14,230,178,227,150,0},
  {58,38,9,251,215,67,35,222,155,181,169,68,188,180,71,180,157,139,126,59,133,227,78,27,81,135,64,130,6,222,245,253,0},
  {146,183,70,152,99,224,17,8,151,98,16,27,215,90,17,53,186,221,46,130,61,215,24,160,204,59,35,149,180,156,35,207,0},
  {124,77,118,44,133,202,246,122,137,139,47,53,109,93,12,183,103,186,110,43,17,229,102,209,19,206,43,81,31,60,165,238,0},
  {215,122,85,84,245,17,180,81,187,47,7,30,188,105,26,91,195,239,251,54,222,249,217,61,132,147,144,94,178,81,240,219,0},
  {235,46,88,203,236,205,14,150,63,172,40,3,151,227,158,207,182,126,205,31,179,42,173,176,134,108,219,122,21,61,240,88,0},
  {235,113,68,219,223,158,1,63,248,118,188,21,16,43,208,0,229,104,181,243,223,246,205,81,53,201,140,46,64,76,139,232,0},
  {212,50,80,203,50,18,244,86,255,178,208,183,37,170,139,20,110,4,0,101,231,84,20,190,233,106,95,175,140,196,4,11,0},
  {90,203,157,13,157,160,72,116,32,145,125,71,221,24,236,191,135,4,102,82,46,177,109,120,240,239,54,152,75,6,185,132,0},
  {81,247,93,195,186,245,109,224,12,40,242,198,14,144,77,23,185,214,32,215,146,236,121,255,194,146,164,152,209,1,76,248,0},
  {188,27,57,198,143,249,2,41,176,80,179,22,135,206,140,247,37,89,124,162,195,166,220,40,84,54,46,61,203,235,156,152,0},
  {119,0,167,175,73,140,3,11,189,120,177,231,6,5,150,153,111,165,232,220,87,46,10,172,3,112,172,181,218,97,104,65,0},
  {133,47,94,200,127,16,103,193,238,205,61,223,132,224,34,200,143,144,114,41,54,188,198,44,84,174,9,156,211,150,43,213,0},
  {7,205,105,24,239,71,151,233,241,27,244,26,174,71,108,235,207,206,8,224,146,131,111,19,142,9,198,9,177,41,113,28,0},
  {251,33,84,63,77,9,185,195,3,214,146,249,246,58,87,253,252,13,110,47,98,97,97,91,200,81,154,17,166,10,110,0,0},
  {227,53,173,36,84,118,95,94,214,97,173,182,239,34,45,172,54,1,36,139,70,75,29,234,117,239,120,125,211,75,228,80,0},
  {228,28,36,245,125,243,215,211,114,120,139,164,184,196,106,3,245,114,74,135,17,197,91,169,165,69,142,49,203,193,120,78,0},
  {191,9,216,18,117,238,71,215,33,49,87,89,154,177,68,244,190,80,26,9,102,71,34,146,86,88,71,193,189,70,137,236,0},
  {110,139,248,71,35,223,29,67,237,208,34,79,41,118,188,185,86,214,171,248,244,170,106,63,62,22,28,30,124,182,162,201,0},
  {219,64,213,52,124,37,226,29,229,50,162,200,46,170,119,26,95,189,176,116,114,89,249,90,102,40,214,218,51,209,9,6,0},
  {224,236,49,166,102,5,231,70,198,51,43,93,87,131,246,97,172,21,0,20,175,44,158,93,205,90,228,77,17,55,55,106,0},
  {44,4,170,208,102,190,245,244,75,66,189,63,118,196,203,20,18,66,242,226,213,192,2,70,157,36,226,32,122,207,170,180,0},
  {224,94,201,150,232,112,183,146,255,138,134,238,99,88,158,182,61,177,227,14,17,91,158,174,48,176,173,5,205,239,230,170,0},
  {213,91,138,174,165,239,154,58,220,159,225,44,36,155,243,133,233,254,115,155,116,217,156,168,151,64,1,186,141,135,154,150,0},
  {143,84,85,128,217,159,207,201,36,86,110,46,7,233,187,133,135,215,47,222,53,66,202,120,15,144,32,102,69,94,137,124,0},
  {42,95,236,32,166,111,161,65,195,233,122,226,79,173,78,182,174,201,22,137,86,159,15,168,140,220,231,33,130,110,236,154,0},
  {107,87,183,201,49,107,199,199,120,7,144,175,111,118,154,130,87,250,252,78,151,145,242,65,175,147,72,83,179,171,205,96,0},
  {161,13,241,12,129,212,31,40,34,220,87,101,255,166,72,62,179,116,133,235,254,165,112,146,19,252,121,167,196,154,105,111,0},
  {167,124,86,39,65,240,122,226,141,151,207,80,9,245,53,222,21,98,85,75,198,174,215,78,157,252,30,254,60,94,248,176,0},
  {151,68,13,33,225,23,167,125,73,96,183,15,14,216,217,95,88,58,211,122,40,119,232,228,61,42,119,13,148,61,167,198,0},
  {29,64,140,55,144,149,55,224,198,63,92,7,69,0,136,45,24,82,104,57,30,55,41,199,235,124,194,82,223,72,100,8,0},
  {60,130,153,73,59,48,191,202,61,200,238,142,199,112,33,88,165,116,158,3,69,72,76,186,160,50,231,109,135,145,83,218,0},
  {144,78,239,149,207,245,233,27,9,216,167,214,66,192,209,118,136,88,74,211,70,66,246,4,68,236,183,74,245,108,31,4,0},
  {113,134,181,67,145,81,155,62,247,142,132,150,217,31,154,56,50,145,22,135,5,110,55,214,116,148,183,50,119,158,31,222,0},
  {184,206,207,102,188,77,48,17,171,189,214,230,91,159,166,170,12,59,148,222,7,211,64,77,189,48,209,121,221,92,124,194,0},
  {2,22,252,88,25,172,72,12,102,252,174,2,150,229,83,32,193,245,184,31,197,69,112,148,56,125,251,193,35,192,191,204,0},
  {104,13,88,37,195,21,231,139,128,98,10,254,229,228,227,179,97,39,143,162,161,231,172,42,183,52,101,79,202,160,89,121,0},
  {69,151,42,251,202,228,228,125,8,58,250,246,177,174,98,45,124,79,242,90,217,154,158,221,94,53,40,74,188,172,239,152,0},
  {250,85,60,241,217,74,206,98,165,230,29,217,181,135,195,211,87,224,78,136,112,130,25,241,55,181,127,93,217,26,192,144,0},
  {24,80,228,12,222,181,124,77,167,65,183,88,53,93,144,100,31,172,179,152,175,236,84,148,228,245,66,85,24,219,169,242,0},
  {85,84,92,43,87,234,27,209,249,104,158,59,224,86,8,14,19,207,155,13,44,228,243,199,246,130,81,97,196,198,0,246,0},
  {220,58,109,151,43,113,175,178,120,58,144,99,30,177,249,150,108,223,248,5,249,213,201,227,33,135,216,126,21,162,181,247,0},
  {59,113,209,87,0,114,69,108,186,170,64,212,204,154,151,181,37,66,159,146,19,170,98,120,8,92,115,47,44,196,188,193,0},
  {56,222,22,7,140,75,23,172,124,105,178,2,253,153,81,25,221,109,33,161,188,96,237,198,252,52,207,193,129,206,151,178,0},
  {245,90,130,145,190,19,210,241,183,105,167,186,18,80,140,252,82,200,120,107,39,121,218,146,127,28,68,251,140,239,80,160,0},
  {230,64,205,67,11,110,247,200,27,87,233,239,74,186,255,214,252,39,63,152,177,149,118,58,151,48,198,17,68,26,108,30,0},
  {202,151,34,53,207,196,154,190,166,130,14,133,165,215,149,7,57,61,102,204,141,162,227,119,127,43,251,98,211,215,101,171,0},
  {93,31,135,74,199,245,79,232,9,198,24,117,107,129,92,233,221,106,52,219,152,205,77,52,40,146,180,3,119,117,193,132,0},
  {155,121,125,129,139,85,185,112,47,147,237,162,29,203,7,225,108,226,253,61,13,173,157,234,164,50,234,95,198,101,27,60,0},
  {69,238,166,105,199,142,16,249,181,164,72,236,90,157,207,57,120,88,150,30,205,35,172,221,202,122,111,32,76,68,26,77,0},
  {120,55,129,150,171,221,57,132,17,173,121,224,193,44,123,202,215,106,38,191,111,213,183,125,109,67,178,68,77,133,11,97,0},
  {134,125,245,139,125,0,197,206,69,129,139,74,83,32,220,218,176,138,121,153,229,115,77,163,236,65,51,193,191,167,205,250,0},
  {13,78,25,148,182,155,109,162,69,119,72,230,195,6,76,68,199,38,131,89,138,16,81,166,1,190,159,212,11,208,35,3,0},
  {179,140,107,211,6,6,235,171,118,81,191,228,162,81,12,19,14,133,38,182,201,17,68,5,90,81,84,149,232,2,224,22,0},
  {132,106,115,155,226,23,148,224,146,74,252,171,21,76,245,69,99,243,72,223,170,179,206,180,108,210,114,237,230,178,96,131,0},
  {67,194,96,229,135,188,65,167,43,191,227,113,111,232,3,224,217,59,30,130,195,72,124,79,204,253,189,68,254,60,231,93,0},
  {92,111,73,233,73,142,111,101,20,178,65,247,174,103,94,136,141,209,1,232,53,8,247,243,123,78,71,198,23,217,209,43,0},
  {43,113,162,143,27,176,99,38,101,141,114,160,94,218,15,22,16,175,80,180,140,105,47,249,199,82,201,105,226,104,164,240,0},
  {89,56,92,193,152,78,214,137,95,200,200,33,221,170,214,136,145,104,210,101,250,7,27,0,74,156,189,244,189,165,206,52,0},
  {193,95,6,55,181,117,11,254,167,23,186,229,63,192,201,145,239,223,92,72,203,138,233,165,185,6,197,220,133,119,165,232,0},
  {197,131,130,136,241,231,68,243,47,93,142,114,56,221,219,231,30,210,244,146,93,98,56,59,238,66,127,99,99,128,239,236,0},
  {191,87,227,71,138,15,154,235,175,232,113,10,130,211,21,203,51,40,117,128,206,35,97,21,72,178,73,109,235,67,96,72,0},
  {131,212,171,36,16,149,185,0,135,90,49,184,201,172,245,156,114,161,90,135,23,202,193,34,25,46,217,168,224,5,141,82,0},
  {193,115,85,106,111,221,254,165,113,201,16,163,171,169,24,61,116,253,85,11,138,48,75,164,15,177,6,92,151,184,123,127,0},
  {67,24,60,142,158,194,215,0,144,100,179,117,195,226,171,165,7,133,229,136,23,86,248,75,222,107,126,192,51,148,124,186,0},
  {51,150,201,232,148,35,83,167,251,25,29,68,153,121,44,214,95,203,157,162,234,132,10,73,187,173,10,92,101,166,73,205,0},
  {57,17,126,156,117,24,26,132,180,69,128,29,179,228,8,245,110,149,136,120,180,75,206,54,154,127,247,168,229,50,10,27,1},
  {55,57,174,109,248,87,105,123,243,188,211,254,196,140,25,4,197,171,189,165,126,219,156,255,202,165,29,58,208,102,150,214,0},
  {160,185,162,19,77,208,102,189,88,75,193,241,35,243,137,103,36,188,141,236,144,84,131,108,130,33,11,121,26,159,219,36,1},
  {85,41,235,155,11,83,177,80,121,28,91,26,118,229,182,177,177,139,168,186,5,183,219,190,8,121,70,68,169,144,77,159,0},
  {173,230,69,55,143,97,61,182,36,10,43,59,148,50,33,144,235,77,53,16,158,188,1,123,151,90,43,163,59,255,193,11,0},
  {53,194,119,188,166,34,194,231,71,106,82,213,108,90,46,120,25,165,184,237,200,94,81,133,230,101,164,178,65,47,254,161,0},
  {11,35,121,102,35,193,18,24,192,127,100,212,6,183,52,109,209,123,6,11,75,95,242,26,88,48,22,58,73,18,241,88,0},
  {205,77,173,183,97,161,72,21,169,221,203,90,86,251,218,197,182,47,210,209,137,226,64,24,73,111,98,218,163,213,109,27,0},
  {84,212,56,170,67,12,212,250,250,176,38,213,171,69,178,145,143,198,27,180,156,184,43,249,135,105,218,127,168,183,254,212,0},
  {227,63,27,164,16,117,79,205,65,124,139,168,19,142,173,199,143,221,1,245,44,69,25,144,36,175,165,251,111,46,65,160,0},
  {217,26,152,92,14,142,43,99,247,14,146,140,100,248,104,61,134,101,216,218,124,143,61,163,8,94,71,136,74,122,165,78,0},
  {106,251,118,70,209,84,59,48,9,6,168,6,24,138,11,139,13,126,6,60,51,146,168,117,219,97,194,60,40,249,65,105,0},
  {252,89,218,3,30,102,86,183,132,87,18,197,4,215,77,56,202,238,31,155,212,158,19,58,138,17,139,181,94,221,9,199,0},
  {77,83,12,101,216,9,174,206,222,250,193,106,192,225,13,108,226,97,226,252,126,62,213,20,19,161,170,21,120,163,98,229,0},
  {92,139,4,122,179,64,4,41,101,143,233,68,52,183,242,114,196,153,24,121,248,105,33,102,218,164,91,242,246,98,60,183,0},
  {102,62,150,163,178,29,70,247,183,36,38,181,101,186,253,189,48,57,46,66,237,185,191,221,114,46,22,85,223,199,107,43,0},
  {220,66,142,134,197,190,19,60,223,57,208,34,173,111,6,236,243,76,211,22,156,210,232,246,56,253,173,191,250,245,188,242,0},
  {189,194,44,42,253,202,220,45,41,176,161,180,52,88,24,135,24,126,149,230,51,247,130,154,197,49,202,166,109,58,106,104,0},
  {143,49,184,94,164,189,147,102,207,142,147,2,195,121,83,109,38,216,80,240,72,184,189,62,88,91,52,144,144,230,28,18,0},
  {179,121,199,184,79,219,38,17,246,241,114,109,218,57,208,65,80,152,68,246,18,91,239,146,77,42,119,222,57,213,35,90,0},
  {192,55,197,97,88,67,73,231,202,36,46,22,238,185,214,23,1,212,234,34,114,85,197,191,38,60,6,144,217,226,94,34,0},
  {81,153,105,29,112,168,184,97,206,162,64,218,240,172,240,222,161,127,102,195,79,192,144,140,59,109,210,15,178,174,135,249,0},
  {212,34,0,27,170,123,138,62,30,39,77,89,83,252,58,29,246,66,12,73,164,166,37,13,131,1,57,252,159,145,109,140,0},
  {109,96,57,191,128,166,207,198,146,5,102,168,39,145,142,122,44,49,234,239,249,236,208,71,25,20,229,66,125,125,117,121,1},
  {139,49,174,184,131,138,111,1,112,29,219,133,72,43,115,199,26,253,88,66,156,118,111,50,233,167,195,112,186,132,203,128,0},
  {122,39,67,45,33,119,129,55,107,45,121,229,233,133,94,149,63,31,248,100,204,61,93,66,142,181,226,162,104,35,136,27,0},
  {66,238,205,20,186,85,219,78,26,148,41,85,205,167,9,66,199,0,183,154,231,65,193,252,46,118,41,15,103,6,136,188,0},
  {1,178,191,249,169,12,71,147,193,128,126,87,14,254,127,179,240,245,55,24,132,179,153,50,153,121,121,252,69,82,69,17,0},
  {31,48,154,117,119,23,111,150,139,102,229,169,127,51,95,197,180,84,3,245,91,6,43,194,90,209,50,37,211,85,179,19,1},
  {193,252,195,46,223,4,1,99,12,32,167,226,128,233,215,229,159,202,105,64,28,66,151,62,151,199,252,43,48,136,9,116,0},
  {244,124,164,37,184,116,50,48,106,21,224,238,117,43,73,138,174,60,7,183,227,54,132,2,29,151,73,153,60,28,236,222,0},
  {206,72,241,216,93,103,32,179,157,46,147,196,231,173,253,135,162,249,237,190,184,204,32,255,228,102,232,96,142,26,33,151,0},
  {73,57,183,92,248,126,190,127,79,46,210,216,111,55,200,14,133,63,197,38,64,45,70,33,206,208,20,46,98,34,230,206,0},
  {192,67,243,29,87,20,183,153,155,144,13,217,8,250,132,164,221,167,144,25,56,179,189,218,38,172,92,212,224,46,132,6,0},
  {64,137,248,177,99,62,163,121,95,37,168,249,209,243,229,210,175,50,8,59,156,246,24,202,165,63,139,7,176,163,39,74,0},
  {89,32,224,78,213,190,81,214,33,217,136,201,8,249,222,30,36,93,248,113,102,9,54,167,120,214,225,233,134,218,234,38,0},
  {109,23,152,137,75,50,45,172,208,149,96,184,20,135,24,127,253,38,251,18,9,199,149,105,113,223,220,5,217,150,201,219,0},
  {181,73,241,121,77,117,161,186,111,87,90,229,8,83,98,255,247,211,195,58,182,230,99,55,83,155,231,64,247,193,59,140,0},
  {66,4,105,102,148,236,87,97,148,215,173,135,174,138,13,68,133,51,136,133,186,182,231,141,107,139,235,119,82,245,105,170,0},
  {48,226,10,54,55,100,32,69,129,149,205,194,3,125,121,155,74,131,197,230,108,161,157,137,194,100,167,36,229,33,165,193,1},
  {231,8,246,164,143,148,161,253,65,71,108,202,141,60,199,147,139,242,33,186,72,20,60,155,159,158,83,99,5,22,178,51,0},
  {10,16,175,143,245,180,168,201,222,130,16,192,166,131,197,131,186,40,101,209,165,211,149,141,70,90,69,35,25,11,0,71,0},
  {49,14,135,194,64,118,131,71,147,86,240,36,0,84,236,181,248,160,78,34,92,15,251,220,52,205,35,173,32,26,254,221,0},
  {151,81,241,90,254,97,188,51,22,99,35,0,98,252,185,81,238,169,87,218,14,233,144,180,188,167,179,95,68,10,239,80,0},
  {199,12,217,16,33,40,150,157,102,254,145,11,225,147,203,111,108,241,153,203,75,126,212,241,7,87,184,133,130,43,207,206,0},
  {57,2,35,2,249,117,84,252,10,193,101,253,18,253,200,152,220,184,158,208,139,93,243,130,242,126,155,10,38,199,17,95,0},
  {52,192,15,192,93,12,221,74,117,58,250,69,235,168,67,105,167,2,18,0,86,57,40,64,153,37,128,2,245,153,241,246,0},
  {20,108,223,255,216,229,185,3,199,87,125,205,22,142,1,178,191,104,41,27,180,29,223,142,177,10,98,131,88,46,171,0,1},
  {220,142,170,70,232,30,214,33,169,132,215,116,251,58,20,181,93,168,134,141,27,143,153,254,37,169,41,206,134,165,202,135,0},
  {32,164,96,155,169,210,102,177,169,25,253,176,2,87,94,20,37,151,42,209,0,179,204,144,79,229,124,109,207,223,220,210,0},
  {173,217,144,241,67,183,157,124,112,42,195,91,35,203,123,26,101,7,88,81,97,239,4,144,128,10,124,83,19,189,88,180,0},
  {59,124,118,135,29,160,108,37,109,53,62,118,190,110,247,210,124,89,43,111,156,56,255,186,1,112,8,220,149,239,157,124,0},
  {137,86,52,37,120,11,30,74,8,75,233,212,226,242,151,94,222,128,160,61,168,174,186,31,115,152,24,153,223,149,27,216,1},
  {14,167,103,118,72,150,128,186,28,35,218,23,117,191,99,222,202,154,121,19,165,7,119,154,147,6,55,13,48,21,149,250,0},
  {119,151,223,244,98,123,1,159,97,84,22,210,81,2,174,157,19,142,200,52,82,205,202,164,122,240,209,243,39,79,186,124,0},
  {89,92,5,28,30,66,60,187,55,73,29,166,205,118,199,24,151,109,229,2,106,212,73,7,142,164,196,236,47,57,146,177,0},
  {20,201,161,43,60,94,246,136,130,93,58,117,247,8,239,119,154,255,96,228,236,148,172,125,62,154,27,195,186,255,36,170,0},
  {127,41,102,29,76,221,143,28,75,49,249,247,240,118,25,61,145,119,89,238,174,152,226,189,182,202,240,53,11,179,26,199,0},
  {135,249,172,252,232,177,128,137,229,250,248,226,60,73,76,188,207,60,220,221,124,108,27,4,162,186,132,202,182,200,190,54,0},
  {29,24,157,199,123,1,108,145,211,213,86,40,150,133,201,220,108,164,179,230,22,209,10,170,111,30,147,180,76,156,13,182,0},
  {229,237,148,21,59,195,201,18,37,134,153,193,144,197,124,157,71,129,106,76,130,106,247,30,87,247,201,44,101,236,57,243,0},
  {215,218,67,211,52,195,64,168,41,141,69,138,2,3,230,204,180,13,103,90,139,30,145,90,21,240,62,186,4,1,151,118,1},
  {249,122,140,8,101,24,105,169,158,64,151,121,147,201,152,242,112,137,24,226,240,1,247,125,153,44,235,179,132,2,244,175,0},
  {122,177,205,145,4,155,33,123,126,121,162,79,192,162,130,212,168,2,246,36,34,171,13,172,134,144,254,104,202,165,88,235,0},
  {59,86,63,78,51,23,240,169,120,46,127,14,117,221,154,163,237,5,177,128,37,86,92,179,218,84,83,102,233,138,40,218,0},
  {73,135,161,184,160,248,51,147,132,56,183,127,61,223,130,165,198,229,84,155,139,37,194,49,238,63,169,167,86,158,12,47,0},
  {44,52,124,109,175,6,234,48,243,213,212,218,71,240,227,54,37,208,220,13,20,192,205,3,123,140,45,252,79,177,225,68,0},
  {216,122,150,35,191,132,40,252,28,184,201,186,85,92,93,152,202,244,117,42,206,16,31,75,199,72,100,157,145,204,9,77,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1},
} ;

static const unsigned char precomputed_mGnP_ed25519_m[precomputed_mGnP_ed25519_NUM][crypto_mGnP_MBYTES] = {
  {224,135,14,17,150,165,131,191,160,179,224,178,234,183,171,20,151,112,40,238,215,61,53,84,208,28,30,103,95,140,84,209},
  {78,64,12,16,211,36,206,251,165,39,152,226,219,206,5,210,37,186,123,49,239,249,65,170,191,244,78,72,66,168,159,60},
  {65,178,251,62,244,59,49,41,66,147,53,227,142,111,248,204,4,194,178,1,198,173,145,105,221,2,69,247,223,200,147,39},
  {96,154,68,253,117,168,110,149,206,46,4,114,0,15,189,95,68,173,57,32,121,250,25,129,13,74,155,200,176,155,84,147},
  {135,164,96,150,65,255,150,117,20,160,209,216,115,150,240,103,114,178,158,183,81,223,81,71,186,99,201,241,63,107,80,181},
  {40,198,180,56,7,46,177,116,244,230,243,39,186,199,131,56,93,231,171,247,190,216,228,148,76,101,139,44,166,158,197,63},
  {185,129,137,49,182,163,45,164,52,45,158,110,241,87,158,78,47,131,88,69,5,27,141,209,80,53,111,130,41,153,19,249},
  {222,81,100,34,85,252,179,241,247,165,190,25,214,31,237,181,34,80,174,90,14,211,251,22,37,139,233,203,91,152,186,195},
  {31,220,101,111,190,159,95,88,202,97,113,215,243,48,175,182,70,32,191,205,217,168,95,154,144,77,134,194,154,250,81,155},
  {194,225,139,187,217,246,76,169,82,71,14,242,243,66,223,18,107,162,8,79,205,43,135,179,184,253,172,176,78,229,15,62},
  {34,218,17,71,209,47,96,134,121,56,232,65,155,248,19,205,226,134,69,225,156,9,101,153,255,139,4,30,155,90,116,58},
  {22,153,156,234,207,199,232,81,16,19,228,112,122,41,92,192,16,162,133,98,37,244,69,112,26,172,30,5,37,165,204,230},
  {36,66,86,84,86,197,40,125,132,245,33,52,48,239,178,198,91,139,125,206,183,122,2,164,113,158,66,45,153,160,112,103},
  {24,132,96,205,123,135,225,41,44,33,190,36,39,71,205,22,140,220,198,241,211,206,242,157,27,150,13,37,0,176,208,62},
  {44,188,140,157,177,222,198,151,186,133,190,196,200,213,82,156,26,166,170,132,16,223,221,220,136,134,131,241,243,150,13,145},
  {105,212,6,187,154,227,199,215,80,21,209,32,148,252,203,4,192,151,119,104,250,202,161,192,157,206,130,103,205,92,139,45},
  {156,17,157,240,57,162,195,58,179,230,62,129,75,129,255,9,135,33,13,190,41,64,156,38,253,96,126,84,190,220,91,137},
  {95,111,137,168,162,7,8,4,43,143,176,35,153,139,8,1,245,26,140,172,113,166,180,251,39,253,81,250,21,118,118,126},
  {90,184,253,66,136,55,247,4,239,11,97,152,17,76,22,16,251,98,6,240,97,78,87,187,98,67,131,181,5,226,41,195},
  {170,235,102,12,200,238,165,23,13,235,59,201,106,179,213,215,174,1,241,163,149,102,208,87,87,19,208,64,185,199,62,181},
  {9,194,170,132,65,64,96,19,188,39,234,139,61,160,10,194,167,249,122,23,54,22,83,2,183,23,124,59,200,94,44,96},
  {104,156,133,242,202,230,100,133,22,28,163,155,114,236,3,202,131,1,120,186,192,240,136,224,97,66,121,104,20,173,107,33},
  {240,40,179,155,15,30,160,15,110,229,206,44,247,187,190,51,231,225,215,92,119,232,90,39,154,246,108,225,11,76,156,11},
  {59,189,252,71,108,218,131,67,53,196,149,41,201,185,96,21,80,84,204,138,251,123,41,46,233,66,113,44,154,57,252,220},
  {81,53,84,178,203,206,244,186,175,55,117,173,89,238,189,253,22,197,35,30,246,116,139,247,48,79,125,60,92,148,235,202},
  {205,123,216,129,99,250,189,75,237,203,41,24,82,134,106,23,12,188,23,53,47,192,184,89,108,187,185,44,219,154,156,42},
  {159,201,96,249,184,41,137,82,196,84,139,138,16,92,187,74,129,234,39,56,79,232,94,116,175,28,57,86,69,106,171,103},
  {6,115,24,244,136,17,223,94,14,127,120,116,74,16,130,217,246,22,90,66,152,3,78,42,128,109,17,165,217,41,114,113},
  {213,169,196,107,214,153,170,23,104,45,78,143,73,113,67,82,255,216,217,103,124,108,171,209,146,176,15,64,182,153,235,90},
  {248,241,234,149,182,116,91,240,203,29,127,212,156,65,59,135,54,233,190,26,191,113,109,106,183,54,148,34,201,198,232,11},
  {17,227,78,205,14,234,160,142,238,92,207,103,109,40,48,141,24,155,115,88,85,46,9,238,170,187,94,231,197,40,82,90},
  {243,11,222,138,209,83,183,37,113,136,111,115,213,8,99,118,163,178,52,32,173,35,236,141,131,35,72,167,114,225,103,160},
  {23,23,23,249,255,10,120,189,135,172,128,58,82,119,223,133,221,221,238,94,194,236,247,186,53,150,141,127,171,215,144,232},
  {166,202,6,51,186,210,4,174,40,143,237,94,98,245,25,39,135,185,139,203,125,129,247,79,41,2,157,237,93,102,165,12},
  {146,163,235,3,55,145,127,30,39,42,69,29,68,253,30,68,0,252,241,178,241,138,49,140,14,70,48,103,40,7,68,84},
  {210,115,123,137,154,23,15,78,52,47,71,159,22,50,129,84,23,66,250,68,205,164,157,114,71,74,7,78,240,105,39,95},
  {217,80,89,164,141,226,21,142,135,217,95,139,92,193,100,112,55,7,110,43,70,184,20,116,5,34,62,238,13,44,149,250},
  {68,87,194,243,34,95,180,188,4,205,131,73,224,90,220,36,174,245,235,76,55,195,156,232,148,35,133,223,73,183,122,140},
  {241,55,251,132,82,181,45,42,245,135,135,208,208,48,1,161,182,60,98,163,19,153,215,113,190,244,61,216,158,21,37,144},
  {64,132,11,216,163,104,20,120,97,148,243,4,197,113,248,13,105,128,92,102,90,86,162,39,96,163,75,133,150,0,162,27},
  {143,184,247,252,72,182,232,65,2,72,224,250,7,139,16,127,146,198,233,2,137,25,174,121,46,248,250,254,130,86,79,93},
  {72,58,202,217,106,65,115,47,56,165,160,168,183,3,60,167,223,32,103,255,44,14,92,146,102,250,108,94,58,6,27,10},
  {206,239,7,45,75,56,204,143,232,81,132,76,20,191,142,90,49,100,171,207,80,205,152,180,137,25,1,132,25,33,4,98},
  {223,216,45,133,29,28,215,158,234,156,213,203,239,190,61,129,3,61,212,175,176,234,137,228,210,219,218,104,101,132,214,246},
  {250,67,5,139,43,25,135,247,84,211,166,230,180,243,82,215,206,96,50,193,31,96,166,142,208,203,147,166,254,249,44,45},
  {46,207,118,166,175,165,16,232,250,232,93,251,204,224,6,208,100,36,135,247,17,193,204,110,126,205,113,131,190,51,103,214},
  {170,107,5,217,141,132,107,17,134,193,106,217,239,7,58,230,49,178,43,35,159,234,25,147,178,121,33,190,158,14,218,120},
  {107,78,39,225,76,227,118,123,0,129,248,249,234,57,152,24,81,250,97,25,113,97,107,172,96,9,73,106,150,136,57,164},
  {3,60,64,15,31,214,116,195,218,43,231,226,105,110,21,134,13,234,154,44,231,179,120,94,129,160,28,163,111,217,2,140},
  {64,40,108,149,22,25,59,138,149,195,70,113,58,105,68,121,39,188,163,188,106,65,154,198,157,222,180,173,75,94,76,21},
  {122,221,70,145,164,237,217,51,55,195,210,30,170,18,195,122,6,60,190,21,22,202,218,140,114,175,126,238,77,162,149,2},
  {152,220,75,52,124,232,213,35,4,214,177,235,213,216,159,240,218,18,61,19,82,69,154,230,47,232,108,229,115,202,67,109},
  {107,68,95,175,251,249,69,249,156,187,187,33,165,98,72,250,38,189,236,179,56,2,209,131,160,110,99,41,255,184,127,55},
  {244,76,29,67,84,149,0,177,39,7,38,58,223,25,63,7,24,14,52,41,218,252,69,216,245,25,118,227,42,228,143,76},
  {187,212,9,183,225,165,169,197,130,23,30,202,74,74,247,229,108,218,26,168,40,192,123,146,169,10,84,28,13,20,67,75},
  {9,106,247,3,83,87,111,18,167,253,145,203,37,197,185,197,63,49,99,158,211,90,119,134,30,211,230,117,84,82,16,13},
  {36,156,85,102,129,162,247,33,30,168,166,69,59,172,115,168,87,199,144,147,25,145,145,66,30,247,226,24,236,210,19,232},
  {162,8,33,200,237,112,120,68,185,134,234,24,136,19,241,200,239,8,170,157,33,133,24,179,43,198,40,151,209,152,43,127},
  {239,112,136,141,239,134,110,247,137,41,45,95,13,123,64,184,132,209,0,188,149,124,58,209,41,147,104,188,67,188,231,108},
  {16,117,126,29,176,36,4,104,39,137,174,35,224,159,30,217,88,230,203,37,59,174,76,203,202,232,170,86,146,179,85,120},
  {168,252,129,57,167,239,52,226,8,19,35,166,207,237,31,255,30,67,105,228,102,140,10,161,161,65,129,177,45,52,207,215},
  {226,102,135,157,218,1,165,137,166,39,235,168,202,200,223,13,239,184,253,140,248,77,118,116,192,222,193,171,187,173,146,145},
  {25,220,47,212,47,127,98,191,89,110,122,214,72,141,108,205,69,68,229,103,10,76,14,86,44,57,114,6,111,187,215,114},
  {86,59,0,2,72,23,188,253,134,220,248,36,208,73,205,107,166,226,25,105,133,206,181,124,204,164,57,77,188,109,207,118},
  {243,184,217,100,254,29,178,240,239,135,70,162,239,138,146,6,138,29,148,72,184,160,38,114,148,228,18,128,153,11,224,174},
  {131,168,29,187,246,140,99,94,109,254,15,212,72,65,93,22,101,1,49,47,37,211,77,145,134,207,71,76,253,174,193,181},
  {69,53,136,190,232,87,98,174,206,191,165,216,171,225,227,134,100,47,142,248,62,46,203,65,5,199,81,60,79,53,59,241},
  {181,197,14,3,82,56,91,173,34,65,93,208,45,196,167,35,106,53,92,85,2,69,144,149,93,6,53,246,2,232,207,42},
  {51,88,59,2,201,75,73,134,79,197,103,135,71,43,170,214,193,5,43,187,93,2,38,44,119,204,54,181,222,212,7,84},
  {47,170,80,5,220,151,138,196,1,111,19,189,219,114,204,255,134,95,57,60,177,219,76,239,129,186,115,216,207,221,177,97},
  {168,204,42,11,125,101,255,202,37,162,68,168,66,20,42,231,15,235,235,69,216,65,137,29,243,17,78,106,125,55,130,206},
  {232,62,105,117,172,202,149,71,156,226,84,158,85,1,3,99,52,53,170,156,54,133,250,249,82,238,123,66,210,200,148,227},
  {213,21,80,94,117,75,174,79,26,21,25,31,34,159,199,155,50,245,59,194,11,133,223,27,124,188,163,165,42,66,149,141},
  {125,164,137,67,133,155,45,50,163,76,109,132,112,54,60,115,59,20,89,156,73,106,13,179,48,32,251,143,176,64,66,221},
  {106,51,20,116,2,106,118,224,97,162,241,129,66,163,0,251,71,95,239,139,109,241,58,219,210,235,154,10,19,19,141,123},
  {76,46,148,134,27,146,178,213,128,114,239,156,159,128,150,41,144,70,48,114,70,67,51,110,71,229,144,230,74,200,166,215},
  {205,131,253,72,145,232,38,205,159,8,207,144,84,249,21,115,31,124,139,206,142,38,128,2,147,116,78,29,37,160,129,11},
  {96,151,31,137,24,112,150,195,145,38,26,220,52,42,60,74,60,61,115,209,131,131,11,217,189,194,65,222,80,117,95,10},
  {245,201,0,64,196,72,44,155,190,144,4,1,69,209,190,214,34,145,212,174,132,231,129,10,247,141,156,121,18,42,31,159},
  {219,20,222,204,237,101,236,30,241,105,252,86,94,187,66,70,99,27,64,64,93,121,251,110,188,136,215,46,80,124,221,237},
  {226,43,170,250,230,36,5,208,250,219,166,16,189,100,48,198,218,233,52,81,24,60,154,82,141,11,187,41,38,155,59,253},
  {13,166,217,76,35,11,140,70,25,175,208,170,163,100,113,144,234,212,165,224,185,131,99,236,54,101,218,224,236,200,206,116},
  {97,27,44,210,100,246,162,191,36,182,233,119,62,224,118,46,208,57,111,67,86,29,30,2,244,150,80,249,36,221,172,176},
  {172,148,59,148,242,32,5,108,246,237,156,51,71,55,106,173,223,35,246,194,152,131,24,170,9,249,97,247,188,131,126,194},
  {226,1,9,4,145,114,92,183,14,187,130,148,34,53,4,22,2,15,85,11,195,110,60,39,137,55,109,0,174,43,139,243},
  {51,101,159,49,0,222,246,249,145,81,79,191,217,180,122,137,157,223,123,85,141,209,24,74,145,147,124,120,76,94,15,29},
  {174,84,241,21,156,12,46,24,245,84,16,220,182,182,72,88,23,90,159,65,196,187,75,150,35,18,188,54,81,30,45,122},
  {31,142,135,193,1,215,238,172,244,14,164,230,69,5,133,47,94,175,138,108,184,157,66,8,230,7,51,212,47,35,8,242},
  {142,237,160,93,232,126,192,35,11,164,19,150,213,194,81,41,176,182,223,40,50,8,239,15,145,243,39,254,159,27,81,67},
  {213,41,111,56,14,157,182,65,98,218,184,232,244,236,124,43,245,69,210,154,29,107,247,30,59,177,116,16,210,176,213,99},
  {120,189,78,19,31,201,127,145,42,44,95,214,209,131,134,42,146,165,137,201,14,3,54,93,229,178,234,38,225,184,168,178},
  {91,244,98,112,241,189,229,178,62,171,51,94,144,103,129,164,115,59,204,8,207,121,185,66,99,59,183,82,29,183,203,91},
  {138,3,228,82,184,177,246,115,161,210,89,153,80,152,89,7,216,143,182,131,237,72,201,179,69,41,160,95,111,198,148,242},
  {251,69,208,40,183,154,92,138,213,240,233,57,155,57,134,55,171,172,70,233,22,23,12,11,6,228,64,154,139,59,217,77},
  {36,30,18,116,136,206,195,223,24,156,63,44,105,195,239,5,109,24,17,95,193,236,220,50,199,103,162,62,118,6,143,227},
  {113,13,201,84,66,144,100,125,30,146,212,189,210,38,19,118,187,32,170,213,209,232,157,106,249,47,167,90,228,215,135,240},
  {252,28,157,173,147,227,165,169,62,81,24,242,78,114,91,135,215,151,45,7,12,41,201,232,174,84,116,243,75,57,216,131},
  {77,119,217,151,138,214,35,150,192,254,164,19,76,179,37,162,125,188,249,9,252,150,98,209,144,163,36,40,58,244,92,66},
  {38,6,220,211,85,41,183,164,57,2,213,162,187,73,18,247,191,91,241,192,118,216,51,196,83,195,0,117,148,146,160,228},
  {164,93,230,155,71,186,139,79,185,170,33,158,49,249,72,90,134,123,155,133,208,45,253,132,221,197,89,127,49,113,152,85},
  {114,139,40,87,137,10,94,45,68,210,159,149,9,152,81,115,240,212,32,71,249,129,125,242,129,109,74,133,220,29,205,174},
  {90,90,250,202,123,157,139,60,81,30,109,201,231,145,80,120,119,138,56,218,20,81,240,182,171,33,147,64,111,27,225,240},
  {46,115,7,34,147,20,153,141,216,185,30,60,9,251,166,16,119,83,248,198,102,219,9,121,88,162,7,172,191,223,66,110},
  {113,98,137,30,106,13,22,133,131,248,96,106,126,130,13,1,123,66,105,10,144,146,14,199,143,253,64,177,18,207,119,200},
  {113,138,250,102,7,28,175,209,255,148,203,157,106,6,99,249,32,200,190,201,194,1,216,45,236,97,122,13,136,242,46,235},
  {184,205,249,85,69,156,209,39,132,167,194,51,124,132,36,136,16,11,193,58,175,41,58,190,23,12,100,38,198,250,140,100},
  {205,55,168,59,148,135,133,56,182,243,202,2,24,39,123,29,222,246,255,155,251,54,182,239,119,2,180,173,55,125,195,19},
  {215,117,246,213,246,237,2,157,18,144,98,97,197,169,76,70,240,219,206,187,13,17,162,159,236,193,63,39,37,206,100,61},
  {218,1,181,54,252,151,52,152,98,63,246,185,37,238,12,143,156,96,244,238,21,34,170,19,207,32,98,52,33,222,38,213},
  {70,223,44,90,4,250,128,197,248,164,182,24,249,255,220,227,0,244,98,187,128,88,7,191,115,110,174,40,40,29,9,111},
  {62,53,166,132,145,59,229,236,211,144,10,30,189,189,52,128,0,227,17,175,9,172,123,126,249,165,101,153,77,93,39,71},
  {38,212,103,81,24,150,223,127,173,168,132,82,161,40,166,201,84,19,243,68,128,25,98,97,180,77,89,203,132,236,127,31},
  {47,7,194,66,215,22,125,132,254,8,136,209,110,119,183,15,50,221,82,247,231,156,69,97,239,254,138,143,148,180,26,255},
  {145,15,103,67,163,205,50,247,78,68,63,114,67,47,13,26,78,135,133,39,213,85,42,125,173,45,134,206,239,19,83,103},
  {138,111,241,12,122,61,26,127,140,24,94,206,131,172,201,79,10,248,203,208,75,181,29,119,23,244,8,132,247,127,227,128},
  {221,205,93,150,194,205,101,59,79,122,150,89,32,0,6,186,149,69,231,223,72,89,241,207,58,37,233,14,189,244,235,131},
  {130,233,7,163,52,220,24,227,159,123,50,0,133,180,114,210,41,176,151,60,92,29,204,46,187,101,234,31,71,47,47,129},
  {110,52,33,49,248,96,63,242,162,221,98,187,212,225,224,140,14,96,122,134,110,44,32,218,2,162,251,77,216,177,216,120},
  {243,181,103,66,169,7,112,247,209,251,8,96,169,84,233,184,215,59,27,80,183,237,207,100,2,38,112,71,40,118,115,101},
  {121,34,242,188,194,159,16,197,45,147,252,179,183,6,144,95,100,159,12,220,225,156,58,249,0,7,253,54,76,222,222,41},
  {201,252,102,219,33,221,121,59,44,112,155,31,220,167,88,179,32,40,180,64,241,132,96,111,28,171,50,163,236,94,108,140},
  {229,180,104,21,93,201,20,238,29,107,169,121,148,43,155,189,232,171,189,88,95,254,43,211,196,55,176,119,252,215,100,69},
  {79,218,103,24,20,143,160,145,65,56,59,164,143,216,45,120,41,81,242,255,69,193,159,48,33,32,238,107,233,173,176,7},
  {38,72,166,226,178,225,201,209,210,249,249,232,157,67,191,138,36,168,1,8,90,6,181,141,229,132,181,82,220,84,18,255},
  {28,59,168,15,251,0,231,52,217,208,36,127,117,28,86,155,82,181,137,175,22,232,25,56,78,56,242,254,207,108,167,15},
  {248,37,158,253,175,78,16,243,160,22,175,192,5,12,129,56,64,204,7,7,113,188,177,175,250,252,13,243,226,10,231,139},
  {197,58,34,193,150,71,33,215,232,25,162,67,103,152,59,70,160,158,46,89,5,223,149,169,91,155,137,172,124,244,105,198},
  {3,7,72,202,244,59,15,254,212,94,21,34,136,185,243,88,11,168,83,36,244,45,245,3,101,36,133,174,54,114,138,56},
  {72,123,50,27,246,69,107,134,96,47,239,50,125,29,99,167,45,240,138,156,226,51,95,101,31,48,217,44,161,52,211,164},
  {93,254,143,54,95,125,60,214,254,8,21,3,63,72,214,18,81,55,127,38,185,226,77,7,168,97,53,40,91,122,18,12},
  {213,154,141,90,176,37,247,205,253,210,188,33,45,151,186,153,45,100,97,160,23,48,56,46,147,55,73,60,97,64,248,19},
  {140,168,60,159,14,114,223,4,225,222,119,83,228,211,81,118,216,25,221,194,34,170,69,18,79,65,227,3,39,182,147,186},
  {68,226,24,51,172,200,67,147,24,9,208,89,94,123,224,22,210,51,7,176,192,54,162,195,206,114,132,106,51,62,174,4},
  {51,4,150,24,156,115,10,217,237,112,56,74,8,43,181,247,41,68,144,184,103,137,86,16,72,24,122,103,59,199,114,229},
  {17,243,81,216,195,86,121,33,21,227,34,79,11,222,119,175,210,215,229,234,133,79,156,93,225,219,232,75,231,166,112,243},
  {146,241,160,254,50,216,130,196,35,63,60,4,197,133,153,15,77,15,252,120,52,190,145,170,227,189,18,37,46,156,173,128},
  {161,238,243,109,189,85,176,198,244,242,19,141,186,32,20,135,254,117,53,198,153,13,84,56,54,88,11,78,73,250,181,246},
  {52,171,180,223,196,42,90,89,7,121,70,171,201,106,54,179,77,138,200,154,181,202,246,145,185,98,229,43,190,25,230,150},
  {78,117,172,73,218,81,228,218,222,33,38,62,66,119,154,4,233,228,19,153,196,193,23,65,192,105,188,231,123,2,106,205},
  {231,13,250,132,75,39,138,183,123,251,102,111,37,112,23,151,89,10,178,195,103,11,118,84,6,239,107,246,197,211,104,175},
  {155,102,23,199,192,166,244,154,235,65,17,146,86,51,183,205,199,47,206,60,87,184,13,94,29,53,190,119,14,161,172,163},
  {89,96,23,38,74,48,18,170,81,175,188,105,128,163,77,192,248,18,213,205,19,44,148,62,66,142,150,239,44,229,27,91},
  {195,102,104,54,168,254,140,128,242,147,202,110,177,159,31,204,164,25,182,159,143,52,206,20,4,230,125,198,20,111,88,204},
  {254,89,36,214,241,246,56,119,140,28,69,130,105,224,22,171,24,18,62,30,59,0,85,204,165,236,37,0,110,71,156,47},
  {22,249,40,206,42,144,158,71,234,159,248,207,225,0,32,235,223,228,231,8,97,71,51,59,157,216,49,207,30,47,24,122},
  {124,87,184,135,160,92,168,83,146,230,153,2,159,192,228,81,201,209,183,137,124,160,214,58,7,136,75,50,39,115,70,132},
  {111,199,12,204,196,123,37,14,141,105,183,246,232,99,231,247,200,141,135,196,145,113,185,153,230,139,11,175,212,133,134,132},
  {156,117,133,41,95,71,126,238,232,146,193,214,128,163,162,225,240,211,142,64,146,17,105,240,211,185,37,28,155,196,137,118},
  {107,2,211,135,126,94,215,112,219,124,11,113,77,147,87,239,43,167,84,168,79,216,224,175,96,229,178,109,37,172,41,137},
  {223,200,236,126,192,41,104,138,209,216,187,230,190,26,163,137,253,40,199,41,185,77,158,35,208,221,62,74,24,99,43,150},
  {188,29,3,170,231,52,8,38,178,138,154,156,21,147,38,159,168,182,66,70,53,110,28,193,22,87,26,82,129,216,192,211},
  {160,208,75,19,73,39,30,188,136,179,50,130,85,78,98,246,222,0,94,198,6,20,4,40,7,211,44,255,179,129,179,179},
  {112,181,180,93,129,108,32,122,171,196,84,89,167,78,233,221,80,77,79,94,218,112,74,80,130,150,216,101,167,229,52,186},
  {8,112,197,13,236,156,115,226,147,155,184,9,110,102,229,64,221,17,118,189,98,164,17,82,124,72,106,232,214,11,103,235},
  {246,11,85,52,221,169,40,242,154,70,94,159,119,26,112,181,8,0,13,172,227,227,199,205,150,202,222,5,11,162,24,30},
  {47,113,147,43,53,91,216,12,102,100,175,21,204,204,113,83,228,89,229,134,110,19,224,241,47,94,111,180,117,97,27,193},
  {96,38,80,131,232,234,123,13,191,167,174,112,114,248,11,106,123,4,175,164,0,1,145,32,243,241,78,170,18,196,177,36},
  {230,10,252,165,184,146,174,57,12,236,103,52,227,205,225,2,80,7,83,107,234,88,46,14,251,220,150,233,225,196,76,6},
  {19,244,183,24,81,51,61,208,84,223,232,30,99,126,66,217,212,254,247,99,221,65,88,113,188,178,111,182,118,89,223,145},
  {160,235,191,217,224,240,150,77,15,182,207,97,168,98,10,82,167,169,152,173,113,127,200,107,156,238,18,81,101,170,126,223},
  {165,51,142,202,198,239,102,147,241,93,202,74,205,152,54,118,55,38,253,190,39,96,189,133,126,66,72,109,247,105,244,44},
  {240,162,102,147,127,59,128,59,244,44,158,25,221,102,174,197,45,157,139,209,127,211,130,113,81,246,7,156,84,192,22,133},
  {28,46,130,193,202,163,47,174,225,30,11,208,198,7,207,50,99,49,38,60,200,71,20,221,119,36,166,13,93,208,16,181},
  {21,252,132,23,198,46,209,43,118,183,229,167,179,36,202,45,178,179,234,85,114,210,20,180,57,242,35,30,91,186,217,53},
  {217,154,50,174,119,220,81,93,56,206,191,121,115,228,21,155,153,111,214,121,250,146,186,201,44,99,80,199,67,135,109,200},
  {108,85,197,23,0,146,163,41,131,216,153,90,138,83,121,145,122,214,87,0,238,111,131,11,166,213,98,201,108,28,213,50},
  {242,239,41,230,99,54,191,16,145,212,236,93,191,231,213,119,116,87,22,55,58,135,154,243,22,221,145,203,82,43,83,5},
  {178,130,247,226,166,48,237,238,3,164,133,169,180,88,16,193,118,236,39,248,119,184,95,42,223,124,228,113,30,237,235,98},
  {189,54,209,143,47,160,4,194,38,248,182,90,132,251,104,159,19,242,31,195,191,197,242,255,145,69,128,25,237,163,98,210},
  {111,54,10,14,78,211,115,205,175,156,64,100,98,214,247,122,229,100,175,149,85,129,119,135,165,86,223,219,43,155,43,35},
  {202,151,192,65,202,80,25,118,217,109,51,204,194,2,236,165,102,25,64,237,1,242,131,171,81,133,111,153,125,108,43,67},
  {49,142,164,146,62,226,62,144,123,156,207,117,100,23,102,80,206,33,212,113,89,248,243,32,157,3,20,119,145,24,85,159},
  {195,232,151,76,162,145,99,161,33,145,188,135,199,139,239,188,47,126,204,229,198,121,73,152,243,9,164,30,21,132,75,130},
  {37,94,141,104,131,132,69,129,106,250,215,87,168,145,0,2,17,239,75,243,63,23,161,229,110,53,6,133,39,4,83,163},
  {73,44,13,40,152,232,103,106,176,96,32,125,170,206,121,184,103,214,45,230,55,190,103,160,114,77,86,209,101,127,244,206},
  {153,46,76,236,244,71,99,131,120,2,139,85,67,152,71,110,218,197,59,36,230,190,102,104,199,166,239,111,20,243,38,226},
  {124,243,110,210,132,174,49,249,236,4,181,60,143,152,10,22,240,83,230,136,193,10,112,60,124,145,249,70,248,205,246,113},
  {234,98,3,217,93,185,21,30,164,65,164,167,143,22,5,167,209,0,198,178,15,236,190,150,149,144,158,93,132,187,108,200},
  {130,56,54,245,188,111,198,213,126,118,218,165,113,68,120,37,230,59,183,192,200,214,107,34,124,155,60,179,79,223,151,216},
  {156,20,27,90,74,232,173,207,13,170,101,80,142,193,249,255,73,20,38,37,73,19,95,43,232,94,54,243,219,68,28,73},
  {152,163,124,136,53,192,127,80,131,23,138,235,108,202,119,198,241,159,219,254,4,81,193,81,66,123,128,151,128,48,3,36},
  {211,96,75,255,137,21,14,100,24,14,33,56,129,81,78,43,88,109,233,15,148,120,100,141,221,183,155,226,16,31,181,132},
  {141,45,246,246,188,182,105,33,128,157,122,29,61,146,77,154,125,189,77,73,81,115,223,253,185,168,243,135,242,240,239,184},
  {158,146,191,162,204,181,50,56,48,226,130,120,250,218,86,233,91,137,2,185,61,143,39,50,147,107,191,240,83,214,22,207},
  {59,251,84,103,98,154,125,192,56,201,233,20,46,178,154,234,126,33,140,10,186,222,170,92,237,0,240,68,157,106,204,178},
  {199,156,190,42,58,161,239,118,15,62,111,87,135,186,83,208,3,228,46,31,122,89,119,123,144,177,212,173,174,114,100,27},
  {79,238,157,90,203,90,165,174,241,38,214,142,130,134,216,20,47,138,16,195,22,178,206,125,129,222,150,231,194,231,138,32},
  {193,9,36,103,143,12,208,9,127,231,115,229,220,192,102,1,204,36,255,216,71,206,160,151,96,141,33,76,119,112,167,199},
  {220,238,240,224,77,239,227,208,119,21,64,171,42,248,111,64,105,107,246,14,46,131,174,71,253,108,83,150,158,65,55,186},
  {58,240,72,28,153,158,220,55,75,191,212,68,176,59,15,242,205,57,166,34,162,99,199,96,99,32,138,155,184,191,86,3},
  {124,6,37,20,96,197,6,85,182,201,142,200,147,132,130,101,254,163,19,178,64,197,44,166,22,162,117,108,117,95,155,49},
  {141,86,122,119,60,122,213,130,193,20,57,29,232,125,102,222,86,71,9,47,30,205,228,174,244,210,235,190,160,87,211,6},
  {228,173,160,207,15,99,90,53,230,205,56,82,46,27,135,50,85,223,225,33,235,210,236,200,64,181,250,51,22,199,254,134},
  {180,184,62,178,121,127,7,25,26,3,139,41,241,80,230,188,37,13,226,78,176,244,106,164,27,189,65,234,78,228,93,106},
  {198,70,188,140,74,171,33,84,87,219,111,244,215,205,34,59,113,14,120,80,198,100,30,15,25,226,70,113,220,235,84,134},
  {212,237,195,194,92,132,197,140,100,225,231,151,5,248,169,228,226,135,188,247,4,135,67,43,61,221,64,77,255,8,118,85},
  {178,169,70,238,131,11,221,12,29,237,145,89,232,116,200,36,117,129,85,147,100,139,146,119,77,213,128,156,202,72,29,235},
  {164,208,126,148,99,158,148,96,166,241,57,130,179,39,75,7,76,32,1,171,136,59,181,42,136,7,14,106,166,158,197,42},
  {18,195,222,170,175,239,59,22,4,186,46,204,165,223,74,37,15,241,173,241,239,154,13,197,12,176,110,32,101,142,129,245},
  {43,59,249,123,246,175,123,89,65,162,161,96,155,25,255,16,134,170,210,155,231,15,243,200,57,247,116,192,97,125,253,221},
  {202,72,127,124,38,170,0,105,181,101,184,154,28,101,200,201,166,164,159,154,215,40,114,152,97,169,183,127,53,113,250,138},
  {8,227,84,252,204,238,151,122,104,169,44,212,117,51,82,215,199,139,131,149,172,233,189,90,4,4,113,109,50,57,43,209},
  {67,253,248,98,89,200,80,121,28,193,177,14,38,129,3,7,60,235,253,0,175,120,57,228,29,229,233,129,31,148,81,241},
  {16,101,119,175,245,176,204,87,4,9,55,228,150,126,151,38,169,104,89,186,32,137,54,35,81,117,253,17,85,227,94,151},
  {133,241,102,186,12,199,81,252,133,136,150,140,61,151,106,53,6,167,174,145,224,109,159,22,112,252,227,82,152,31,223,58},
  {92,237,199,133,245,117,54,115,211,97,22,14,239,203,90,78,137,77,209,184,7,164,181,231,101,204,254,108,185,9,161,57},
  {63,180,53,184,238,9,40,253,14,178,214,69,60,215,136,36,40,193,172,123,7,100,78,142,65,87,16,224,129,187,219,184},
  {138,18,134,249,243,115,255,231,69,235,212,107,231,147,203,186,225,11,7,67,241,119,47,123,101,169,147,82,121,57,148,240},
  {213,243,50,107,60,244,9,158,7,115,0,111,230,81,225,52,104,52,52,238,9,8,69,105,149,116,164,41,29,185,137,226},
  {255,163,130,223,111,130,158,202,61,207,126,239,83,125,118,42,168,207,92,167,47,201,47,170,129,190,65,236,220,232,197,170},
  {23,233,44,178,192,138,73,162,245,196,85,166,115,141,81,169,35,85,119,147,187,30,175,175,40,157,50,206,7,191,226,83},
  {226,55,127,198,136,90,204,69,180,89,226,77,183,200,95,195,170,140,56,51,251,18,103,97,230,20,97,255,108,242,37,43},
  {60,184,1,127,200,237,122,0,75,120,156,32,103,183,126,127,110,87,232,66,143,31,42,240,25,35,151,123,162,236,175,33},
  {252,201,68,1,94,213,176,96,2,5,141,152,116,97,229,211,48,91,19,106,57,152,28,37,95,100,130,157,229,50,118,70},
  {231,103,184,78,230,219,176,248,252,214,4,7,7,239,191,143,194,135,129,145,126,111,51,60,106,158,31,202,165,20,18,34},
  {11,246,230,84,240,242,160,187,249,208,38,128,61,151,57,85,111,124,235,186,87,72,100,114,45,9,151,119,75,247,171,180},
  {123,199,124,5,85,143,214,64,217,18,43,45,39,204,217,159,39,33,185,196,241,171,236,125,5,200,88,8,181,152,19,169},
  {86,12,0,74,231,236,29,181,68,80,19,137,173,200,54,76,103,209,118,93,159,195,175,8,143,147,18,55,200,169,181,54},
  {6,247,211,213,81,14,215,159,197,217,252,9,154,173,148,202,237,181,85,184,174,224,147,172,252,91,177,23,13,234,53,143},
  {32,196,186,12,241,6,125,112,184,252,58,3,243,4,108,132,27,11,79,253,187,157,30,89,207,110,210,33,90,120,30,75},
  {39,42,223,58,176,69,139,186,25,114,20,183,79,51,25,171,2,105,216,149,80,167,230,200,136,24,132,97,62,16,78,234},
  {222,49,39,101,35,154,168,68,8,225,127,249,6,101,78,4,80,95,130,20,2,99,92,153,167,132,117,241,233,40,139,29},
  {246,160,115,111,198,158,68,163,34,223,48,251,101,74,106,194,183,87,72,172,184,172,81,60,154,18,43,87,243,126,73,229},
  {204,119,228,209,134,144,69,104,82,98,196,54,140,244,56,111,248,203,155,115,246,117,66,247,45,220,137,74,153,164,144,139},
  {227,61,146,44,172,31,134,175,146,53,17,9,125,16,118,214,228,146,117,65,219,185,235,194,182,37,197,33,198,43,18,153},
  {99,161,87,12,20,66,189,241,16,138,226,166,105,111,71,39,253,156,198,223,3,208,49,126,63,86,51,21,153,90,166,3},
  {213,64,225,91,125,166,169,4,65,235,235,140,70,125,121,115,191,49,102,71,47,65,250,176,10,180,167,42,250,56,27,108},
  {179,170,4,90,121,97,71,167,146,73,173,136,244,210,168,186,220,147,232,58,80,93,177,158,126,22,125,38,94,94,158,242},
  {214,24,55,9,125,6,44,252,139,242,66,214,108,97,249,86,198,34,156,157,135,140,93,18,185,75,70,233,41,186,56,251},
  {126,108,4,175,196,149,87,41,92,161,48,162,107,127,253,179,38,16,101,26,161,179,212,185,156,157,51,219,249,64,218,20},
  {40,243,177,97,188,241,248,61,5,108,47,235,75,145,155,217,156,130,27,131,172,88,72,55,28,249,68,69,56,62,118,238},
  {3,163,97,215,119,68,20,21,129,249,34,94,134,211,106,245,154,180,36,99,138,5,93,83,179,231,30,60,45,187,159,208},
  {228,238,53,22,156,231,245,246,48,215,205,22,222,15,161,187,214,191,149,51,96,105,137,17,146,235,94,118,198,201,185,192},
  {181,92,130,12,107,166,241,91,64,73,252,9,166,113,49,27,189,205,145,18,118,188,2,170,249,41,224,232,84,215,87,195},
  {44,164,82,238,42,239,82,245,14,133,17,109,18,202,163,118,251,231,230,215,28,83,32,103,178,44,254,232,175,63,248,249},
  {139,122,221,214,126,124,228,76,144,138,1,132,158,140,119,178,27,38,40,141,119,220,240,248,16,154,13,150,209,39,47,98},
  {172,103,65,171,227,157,83,82,218,213,148,215,246,22,151,218,205,67,54,92,30,212,77,199,62,164,100,32,23,211,138,230},
  {97,171,134,102,99,61,106,216,114,100,117,206,206,171,81,16,241,114,38,89,92,115,141,66,218,15,30,224,90,249,106,91},
  {138,103,203,238,58,207,32,253,113,254,133,92,65,11,230,73,120,229,13,85,221,53,245,247,238,169,66,213,202,234,231,147},
  {218,76,66,170,147,200,170,75,155,167,10,189,170,22,196,241,106,132,209,227,203,150,31,48,178,164,201,92,81,104,106,167},
  {243,143,194,112,28,67,41,87,79,10,168,107,26,139,151,78,8,189,78,124,194,145,154,253,244,136,170,29,25,64,53,192},
  {174,18,122,54,137,79,164,228,135,9,84,83,151,25,211,186,141,111,71,235,102,253,20,165,184,13,78,40,164,177,50,231},
  {30,17,120,242,0,80,77,49,66,107,5,112,128,154,240,198,176,213,128,140,56,203,186,135,28,0,71,111,105,18,237,80},
  {29,187,15,26,40,78,231,104,185,76,15,244,127,88,247,237,242,244,38,132,203,0,133,252,9,118,49,187,235,23,191,253},
  {124,24,168,235,35,244,248,20,181,139,58,235,120,77,155,34,172,199,90,85,50,139,80,200,252,210,57,192,114,15,82,109},
  {184,173,65,165,231,90,124,51,54,198,158,186,67,210,230,142,102,80,73,246,77,253,26,213,180,129,220,199,200,228,251,149},
  {16,252,233,198,84,113,59,241,120,91,52,99,140,159,193,65,155,74,156,25,234,87,108,152,137,47,5,147,102,156,95,73},
  {175,203,195,215,186,181,148,98,57,226,154,204,141,235,79,124,53,47,53,177,65,45,1,181,238,48,18,68,167,27,91,207},
  {11,56,76,200,39,233,184,108,16,95,39,1,57,0,40,252,19,59,66,111,175,151,176,139,226,84,251,126,98,194,178,5},
  {252,149,250,226,164,51,189,101,173,248,254,3,87,210,36,92,239,206,135,75,169,112,79,17,50,45,78,210,141,42,243,185},
  {36,24,246,0,84,54,230,80,131,5,222,167,36,217,241,205,175,145,15,50,150,96,88,16,86,226,122,75,40,22,22,136},
  {241,182,198,252,143,228,121,7,130,249,24,56,134,195,183,225,168,127,1,173,65,155,154,136,43,165,105,154,254,176,132,208},
  {153,218,128,83,221,40,239,133,220,187,215,210,136,151,3,51,222,219,225,100,207,89,54,211,116,0,100,123,154,46,66,102},
  {24,152,129,206,15,18,77,228,176,74,181,191,156,18,76,181,246,18,78,36,89,226,252,247,204,90,79,122,159,255,177,143},
  {145,222,2,114,39,87,28,15,121,166,195,84,159,44,198,181,141,139,39,189,164,210,106,16,158,207,193,98,89,68,48,152},
  {43,27,215,159,148,76,159,27,121,254,242,220,133,91,183,126,82,176,131,64,30,51,52,218,44,223,4,37,120,32,101,84},
  {168,111,137,210,250,183,89,128,123,86,82,128,94,40,140,35,245,24,247,137,161,147,69,223,177,97,248,183,65,153,234,53},
  {140,134,234,252,248,114,253,216,18,237,27,115,63,240,65,42,121,139,206,24,45,47,3,43,92,156,6,254,168,31,219,172},
  {221,229,215,242,40,207,209,209,204,234,50,253,80,68,121,90,14,153,108,255,41,7,228,99,81,75,131,146,191,76,158,122},
  {122,151,17,172,100,37,239,70,0,69,96,171,213,9,80,15,60,174,145,99,253,210,79,202,86,70,153,151,83,62,232,223},
  {55,169,184,243,37,158,140,197,152,61,37,149,246,232,177,126,135,238,8,6,119,194,84,33,232,236,167,245,200,118,128,75},
  {86,163,214,52,251,144,171,195,41,30,200,28,193,28,78,255,174,100,115,190,128,246,65,10,221,12,246,156,86,194,202,243},
  {225,161,48,213,170,114,180,124,103,36,171,175,33,166,187,184,14,1,99,127,177,99,4,42,48,28,230,109,116,68,30,163},
  {165,52,39,93,78,59,6,153,160,81,240,187,231,139,4,129,75,228,32,162,167,85,25,223,49,132,8,116,7,24,192,102},
  {46,148,250,208,189,152,75,183,204,6,16,128,254,109,52,228,5,81,136,243,3,122,122,221,7,94,117,161,224,170,133,29},
  {95,111,47,84,205,185,39,87,160,182,233,31,237,137,13,112,138,12,184,191,86,244,144,225,35,228,147,188,173,213,23,203},
  {175,227,218,66,9,208,36,171,143,181,29,176,110,193,223,115,204,115,239,203,52,231,131,159,183,83,63,94,44,215,133,36},
  {181,235,77,84,62,214,225,51,188,235,72,170,33,62,154,79,102,112,52,173,82,136,13,219,93,96,72,119,13,237,224,79},
  {113,149,145,243,232,230,95,252,71,104,200,245,125,159,212,207,97,47,159,131,155,173,74,53,107,236,232,82,73,27,199,63},
  {207,19,204,239,132,44,140,132,200,231,165,25,90,62,214,117,141,189,42,41,237,77,225,118,83,172,11,215,202,201,229,23},
  {142,30,103,204,72,0,222,221,2,191,37,61,7,65,157,144,160,75,105,100,158,172,57,116,130,20,91,136,129,72,245,250},
  {213,115,0,117,26,114,244,5,197,99,241,112,41,252,14,213,139,22,213,72,38,163,179,233,73,15,130,255,13,42,116,132},
  {120,13,69,168,234,93,233,214,97,126,196,35,107,205,65,251,207,128,150,60,126,204,62,185,21,145,89,224,56,192,56,2},
  {11,23,92,200,69,154,13,208,17,59,124,150,82,160,225,29,61,169,102,154,209,1,111,218,49,167,100,118,158,81,105,203},
  {86,159,255,54,92,50,218,208,163,225,98,177,16,96,85,8,249,78,38,10,36,79,189,127,36,90,204,8,55,8,14,155},
  {96,171,211,51,104,100,182,78,125,47,84,9,74,42,206,87,228,217,189,118,24,185,133,21,28,51,34,126,250,159,146,132},
  {101,230,20,254,146,255,150,59,64,252,80,203,38,102,122,53,167,219,107,255,216,150,33,9,85,64,144,183,41,153,158,162},
  {129,204,0,103,37,49,76,190,57,124,179,139,40,81,188,168,211,122,127,18,107,190,117,76,197,153,253,239,111,102,46,69},
  {172,100,182,63,39,106,10,197,225,72,198,150,53,185,80,185,247,173,14,43,46,29,237,229,109,210,219,66,94,159,41,26},
  {74,103,13,128,72,204,163,212,17,51,33,236,204,86,242,181,10,216,139,69,199,205,255,234,168,178,48,131,161,54,146,94},
  {64,161,115,156,111,54,131,33,226,199,189,243,58,166,42,26,253,26,0,241,127,211,145,229,157,199,156,143,220,220,166,179},
  {78,162,175,206,131,204,167,156,2,153,94,154,37,27,43,2,176,113,63,111,154,37,198,104,138,215,32,33,82,39,242,203},
  {116,99,254,150,49,123,185,158,242,173,16,216,185,49,48,183,178,191,115,136,201,48,191,234,163,230,146,209,117,15,175,77},
  {58,228,62,86,9,65,13,42,210,44,64,77,33,171,36,202,93,38,102,253,210,77,233,110,219,30,72,193,225,173,48,168},
  {212,189,103,25,15,170,171,46,17,50,135,224,107,111,67,136,179,190,113,87,203,146,150,131,160,157,243,188,47,120,114,205},
  {239,151,238,247,79,152,138,220,169,255,214,47,220,62,231,79,64,2,216,126,132,198,245,57,249,248,32,140,233,232,32,184},
  {3,198,63,166,195,50,19,182,180,126,247,51,49,209,2,73,230,252,71,105,236,54,107,2,3,212,80,52,107,5,160,38},
  {151,204,245,111,240,77,190,230,82,25,203,14,168,240,196,54,68,37,6,12,118,223,17,2,211,196,100,160,41,7,101,198},
  {83,14,13,106,241,31,216,224,6,182,171,64,52,39,40,109,170,44,121,74,151,68,62,20,64,32,248,30,118,56,35,171},
  {243,188,31,185,17,240,54,212,163,67,198,61,222,2,85,54,166,202,127,228,53,12,19,122,27,29,7,249,10,239,74,236},
  {234,109,51,8,136,219,239,115,219,249,36,254,195,18,13,70,207,128,185,164,30,115,89,195,208,236,142,179,88,33,15,105},
  {23,120,210,21,148,60,20,226,88,46,209,237,37,207,43,116,233,163,91,72,126,74,215,234,147,242,235,62,60,221,121,139},
  {146,23,158,50,64,109,29,147,48,98,56,178,131,27,155,87,213,203,41,119,183,94,210,243,120,135,204,49,165,252,31,35},
  {83,142,151,19,250,223,214,60,236,144,84,103,70,105,63,79,88,197,91,77,185,55,133,57,163,89,118,102,44,46,170,139},
  {202,92,186,239,64,49,62,145,125,247,207,31,29,46,157,64,157,13,253,250,12,19,246,187,141,251,220,47,41,179,255,134},
  {197,80,193,251,165,120,253,208,180,2,67,53,203,150,206,128,20,252,176,106,75,228,244,8,77,227,1,56,87,246,37,55},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112},
  {102,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {103,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {104,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {105,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {106,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {83,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {84,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {85,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {86,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {87,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144},
  {64,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {65,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {66,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {67,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {68,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160},
  {45,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {46,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {47,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {48,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {49,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176},
  {26,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {27,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {28,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {29,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {30,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192},
  {7,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {8,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {9,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {10,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {11,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208},
  {244,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {245,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {246,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {247,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {248,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224},
  {225,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {226,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {227,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {228,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {229,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240},
  {50,247,98,124,67,204,221,248,209,165,188,179,170,154,165,66,87,155,135,247,105,66,110,206,78,244,28,55,223,142,122,245},
  {199,80,145,57,100,121,211,94,66,204,192,98,233,188,192,147,187,160,198,149,184,22,79,48,25,112,233,137,39,40,109,151},
  {38,51,73,253,25,129,134,224,136,101,17,183,140,209,92,230,117,177,115,225,215,64,109,239,74,79,80,16,159,118,78,70},
  {207,225,129,22,78,143,67,62,115,106,79,68,74,23,203,163,26,45,12,144,7,126,220,2,5,215,161,85,179,220,64,118},
  {143,143,26,149,89,167,83,31,99,155,20,230,251,58,16,10,159,151,255,168,239,127,216,141,63,151,167,91,107,105,190,204},
  {203,173,5,225,170,220,72,5,48,247,162,199,91,222,56,69,42,137,62,212,222,2,247,45,118,159,45,118,98,169,80,177},
  {50,38,100,17,163,75,209,175,7,113,155,30,230,56,114,123,89,174,82,136,109,163,208,116,196,111,94,236,102,234,200,254},
  {219,122,149,107,99,68,92,215,255,10,135,64,237,29,223,151,162,91,214,55,75,98,105,237,97,156,190,21,169,95,219,246},
  {133,229,77,66,2,162,48,32,154,115,112,182,77,118,116,76,35,213,17,59,252,53,65,188,20,131,128,155,16,161,69,50},
  {236,86,14,162,74,11,78,194,38,123,210,23,142,221,110,70,17,186,188,130,186,154,32,218,122,184,85,232,161,136,237,47},
  {221,85,115,26,126,40,239,29,93,138,5,86,167,130,212,18,238,116,116,139,91,12,114,106,169,97,190,73,234,174,129,8},
  {80,109,109,181,208,203,231,56,75,158,218,172,27,188,6,112,209,117,149,238,2,230,232,134,117,252,240,39,54,183,81,20},
  {3,81,105,1,61,71,43,50,136,142,81,144,185,21,171,124,165,159,134,240,227,71,130,32,250,115,117,52,149,166,185,1},
  {178,140,16,174,161,83,105,65,112,89,123,35,208,213,110,220,183,176,228,171,146,1,48,158,21,194,161,241,209,134,45,150},
  {22,192,122,10,159,17,132,226,79,39,32,139,39,19,142,152,201,222,228,74,183,247,32,94,12,208,5,234,237,22,214,223},
  {136,255,210,49,149,125,54,245,134,128,132,253,24,76,223,136,3,51,240,76,161,205,38,166,162,119,166,20,0,66,217,140},
  {7,150,213,243,14,198,34,76,161,85,199,153,43,159,241,223,125,255,149,229,129,138,199,232,139,228,212,75,87,165,63,231},
  {247,159,60,71,236,187,184,178,255,54,212,66,217,120,243,191,178,43,173,246,173,67,135,15,173,163,25,170,189,134,18,206},
  {44,180,35,154,7,80,216,223,203,245,123,183,155,73,217,51,136,62,220,160,204,254,181,236,113,150,172,66,50,123,198,129},
  {31,43,66,200,145,35,9,60,94,69,34,40,241,197,187,52,193,1,144,59,208,137,119,37,193,93,242,70,98,212,136,148},
  {140,57,86,79,204,177,197,92,68,95,243,160,104,39,40,204,190,202,47,186,185,167,122,221,183,213,83,59,52,224,127,161},
  {99,96,119,144,25,24,92,67,231,199,167,135,25,149,232,112,52,208,120,6,179,143,31,227,130,93,5,25,60,131,150,62},
  {151,58,22,113,38,153,67,59,3,178,208,145,145,212,14,206,79,32,244,156,231,173,26,150,208,144,167,51,14,235,0,136},
  {37,77,235,222,0,49,80,159,230,217,196,55,135,17,181,41,21,141,192,57,197,143,229,13,126,189,110,126,72,33,109,156},
  {105,129,189,232,233,65,53,212,88,101,152,62,50,244,218,143,3,184,151,35,126,241,195,108,171,194,187,152,176,108,235,178},
  {59,30,114,237,166,31,87,58,92,157,233,84,117,233,177,202,70,65,57,195,30,166,182,149,153,222,117,70,200,26,123,16},
  {128,104,203,124,234,142,36,185,56,244,121,47,130,59,86,182,216,246,9,230,25,55,88,129,51,134,144,69,83,207,213,54},
  {140,167,94,78,152,191,235,206,55,116,156,116,118,29,56,164,155,119,192,47,185,8,49,207,232,134,51,21,0,122,37,41},
  {99,93,96,152,17,198,47,118,62,236,136,5,96,202,97,95,150,55,161,108,241,73,106,45,147,71,224,227,192,152,132,59},
  {50,166,234,225,34,229,255,15,178,22,84,61,132,70,42,102,87,63,242,238,211,141,0,11,76,110,61,26,143,249,90,146},
  {32,144,214,166,58,254,157,90,160,187,105,254,248,223,140,97,77,59,42,155,14,128,232,92,79,232,63,19,120,123,106,218},
  {6,61,72,1,135,22,241,176,105,221,28,249,12,18,2,104,147,67,252,86,198,147,138,188,129,83,30,213,219,62,254,132},
  {192,249,60,81,169,130,20,17,72,107,212,6,121,135,232,73,184,255,9,121,216,243,10,56,16,31,6,58,22,156,50,137},
  {94,227,199,98,192,50,96,173,60,19,245,111,89,25,251,29,18,46,10,109,142,191,78,7,167,238,177,8,59,236,127,5},
  {255,132,240,211,207,145,87,205,47,232,98,219,80,114,27,146,140,251,184,248,119,17,140,31,139,118,188,127,219,205,235,157},
  {210,208,151,164,197,204,95,119,156,254,92,185,39,36,166,20,146,226,170,92,208,2,36,21,154,201,118,244,54,8,139,67},
  {233,9,29,35,13,184,111,142,181,11,8,67,172,39,110,183,146,236,102,139,246,201,130,162,213,136,175,163,80,232,165,149},
  {173,199,98,81,82,60,221,55,213,55,38,63,153,168,53,102,152,228,20,193,103,58,107,112,126,92,50,143,249,118,68,216},
  {39,60,76,87,192,84,166,117,217,108,218,19,97,46,177,222,89,179,54,158,115,21,48,132,36,136,97,218,98,60,78,1},
  {81,222,52,66,249,232,218,206,70,254,174,91,91,100,24,114,248,103,20,56,101,255,9,230,81,128,64,65,234,153,141,19},
  {99,132,115,213,182,7,130,71,29,90,224,218,150,39,153,192,138,31,157,32,177,211,108,168,101,99,124,239,217,25,92,203},
  {15,122,180,2,9,44,176,195,159,204,115,41,45,249,187,140,5,56,176,31,163,185,121,195,112,76,251,9,221,170,244,80},
  {136,98,123,97,76,16,179,239,13,107,189,181,195,213,60,84,118,130,156,8,217,64,253,127,91,44,26,196,21,89,108,45},
  {28,22,165,161,239,141,73,106,233,25,70,192,88,244,42,98,169,49,165,32,210,238,219,34,200,105,88,95,54,158,58,140},
  {215,192,143,58,239,126,237,20,206,72,197,232,182,25,0,45,52,86,193,159,30,191,251,167,232,208,46,9,89,101,231,212},
  {129,162,95,148,199,73,194,97,79,203,249,61,232,181,40,131,140,222,20,135,8,212,187,62,168,161,11,136,114,136,122,140},
  {145,234,29,155,101,52,70,142,75,2,170,91,5,134,84,229,6,95,225,60,96,223,224,118,63,233,241,116,211,40,75,168},
  {113,36,146,59,141,19,162,38,46,52,237,194,93,253,133,10,194,173,153,155,48,23,51,4,132,226,51,144,232,38,165,60},
  {6,40,82,117,144,41,251,52,60,131,12,54,5,140,177,190,117,184,35,40,199,103,135,39,106,254,91,189,60,248,98,228},
  {28,169,114,16,51,167,220,35,197,162,191,32,72,101,2,103,75,193,5,210,18,204,227,130,169,93,179,227,96,170,253,6},
  {100,47,31,235,31,23,113,235,193,136,124,242,213,243,53,20,117,154,99,37,198,84,90,73,121,48,87,38,160,5,7,252},
  {170,70,206,205,152,109,165,61,210,11,41,86,14,2,83,88,108,128,184,103,0,11,231,38,161,109,112,158,211,124,176,113},
  {230,16,12,191,217,110,63,224,23,76,95,88,44,220,85,148,205,195,226,4,45,223,179,51,118,243,195,114,144,10,141,162},
  {176,164,164,245,120,197,156,178,86,202,84,187,34,7,143,222,14,143,0,126,205,24,76,107,132,216,65,139,41,215,167,228},
  {8,167,78,91,212,5,58,231,156,236,156,222,129,97,87,137,123,99,106,70,149,27,223,149,240,121,81,100,13,206,28,242},
  {101,181,126,15,202,29,46,224,217,159,46,57,148,234,216,83,66,239,207,210,103,117,139,244,229,106,133,232,160,138,24,121},
  {100,66,147,234,214,28,28,240,14,101,245,90,142,103,58,181,228,202,131,240,78,140,116,47,150,234,49,127,72,103,80,224},
  {2,14,111,222,146,80,86,116,169,243,114,174,43,169,214,21,112,86,120,144,232,236,118,144,218,37,56,146,239,106,140,2},
  {147,149,67,253,234,99,236,222,19,235,90,6,170,177,147,72,114,156,132,186,209,148,65,198,254,196,180,169,82,168,134,121},
  {213,133,197,14,160,79,236,192,222,250,208,58,99,212,112,182,112,189,148,125,115,113,102,92,82,234,136,172,19,53,121,244},
  {89,220,44,248,163,151,31,162,75,79,110,60,200,78,72,147,113,147,211,61,112,105,234,102,250,173,150,180,175,92,106,237},
  {33,226,15,227,96,114,57,238,60,151,18,31,134,54,122,41,51,29,210,109,220,87,44,30,159,173,196,45,2,134,215,97},
  {97,102,149,173,172,66,165,6,96,35,166,63,184,140,9,147,126,34,183,200,13,196,15,162,166,169,164,23,115,131,185,11},
  {10,90,241,102,235,255,64,98,219,112,34,114,108,126,107,227,183,53,238,192,39,1,193,200,158,42,89,165,226,225,33,194},
  {237,73,159,211,126,107,188,227,76,134,90,77,105,34,227,200,119,76,224,107,123,28,253,10,1,109,151,99,48,185,5,253},
  {180,94,48,143,229,8,26,255,31,189,107,202,103,218,104,248,145,121,235,152,107,20,39,95,122,36,254,110,42,183,236,203},
  {233,197,40,79,78,143,213,1,1,110,128,177,128,194,188,171,22,249,50,109,38,92,161,78,80,177,149,169,211,130,146,47},
  {204,214,183,19,199,253,75,211,29,221,48,28,194,116,216,83,217,241,10,151,36,149,115,167,188,48,11,38,99,176,161,52},
  {27,132,165,245,134,148,241,185,242,229,18,16,135,90,99,75,136,215,240,224,43,204,224,57,160,217,207,126,249,77,209,249},
  {67,62,31,1,158,158,28,200,121,17,134,7,244,93,62,241,176,126,12,88,58,171,206,119,24,67,209,169,232,106,203,39},
  {247,125,106,221,240,122,104,215,76,78,240,21,167,68,180,80,118,45,209,46,175,43,172,86,121,232,19,150,71,16,225,109},
  {169,234,215,22,220,141,118,156,255,73,18,109,61,34,162,66,227,208,251,226,174,163,39,177,86,25,140,98,103,130,153,12},
  {177,110,244,29,255,64,176,62,10,94,6,171,191,248,16,172,46,49,93,91,238,106,67,76,85,155,46,10,40,194,168,216},
  {128,94,94,142,196,61,23,216,182,30,160,1,79,145,66,177,240,142,3,135,163,17,10,236,11,142,19,22,239,52,226,107},
  {112,126,34,147,36,143,34,215,193,236,52,52,17,107,81,234,174,194,32,77,177,17,184,87,237,195,134,234,142,5,138,39},
  {69,234,246,175,219,94,153,99,185,47,95,141,8,47,61,121,17,247,59,130,80,193,174,142,104,71,86,204,76,30,11,218},
  {78,36,103,246,70,203,240,103,8,189,190,34,70,115,108,103,200,160,107,193,157,135,164,97,208,118,177,36,21,166,249,252},
  {87,248,19,224,227,4,250,150,47,56,3,75,250,167,205,124,78,36,168,39,143,153,206,6,64,49,168,1,32,37,240,158},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
} ;

static const unsigned char precomputed_mGnP_ed25519_n[precomputed_mGnP_ed25519_NUM][crypto_mGnP_NBYTES] = {
  {10,143,232,22,159,195,90,177,170,55,106,207,146,159,0,90,100,75,5,116,95,64,95,139,118,51,233,68,49,9,56,116,173,6,210,199,68,82,3,67,188,180,196,29,248,93,109,239,95,170,254,173,250,15,113,183,142,24,153,174,149,19,53,209},
  {147,166,155,108,84,238,173,228,212,61,200,159,5,20,138,201,108,43,242,173,100,71,240,92,219,123,154,10,24,229,36,173,252,162,4,55,146,220,234,252,72,199,58,95,44,234,48,84,67,100,240,82,17,226,194,127,186,209,79,255,50,135,240,210},
  {135,20,231,42,210,124,195,244,59,203,227,211,33,7,39,164,238,148,46,28,52,235,215,193,76,127,220,55,140,54,67,220,22,228,26,93,13,73,3,92,245,148,86,97,242,13,10,188,167,143,128,61,94,190,187,222,134,83,107,67,2,18,139,129},
  {93,23,40,225,110,159,207,34,135,141,181,194,198,68,38,219,47,212,6,165,191,255,1,237,217,130,20,80,230,148,245,159,125,85,148,114,170,197,206,208,77,111,33,137,46,51,197,213,243,60,209,171,162,116,249,228,65,73,159,247,19,55,15,135},
  {41,17,182,121,130,2,128,100,186,37,92,212,243,161,29,169,240,87,29,215,17,83,82,46,14,65,118,45,46,238,62,63,214,169,56,252,83,151,1,86,240,77,253,94,159,30,47,229,36,111,103,143,146,81,98,197,144,247,230,66,9,129,144,192},
  {144,66,204,78,248,67,198,192,211,26,54,67,65,7,97,103,231,132,200,149,160,233,139,56,33,97,196,143,112,216,26,21,6,217,42,39,247,244,199,183,11,24,190,187,12,160,153,89,205,247,218,239,225,11,28,153,254,134,144,184,87,217,185,134},
  {141,174,82,207,98,133,57,165,52,92,254,252,28,236,233,248,40,11,35,100,222,189,129,137,38,98,167,13,1,28,127,189,76,91,239,98,181,20,191,58,164,44,188,230,127,178,135,107,218,32,240,145,87,73,87,57,78,105,138,111,219,16,224,213},
  {71,153,68,94,14,197,196,216,63,57,251,150,148,206,84,33,0,164,206,176,4,171,98,35,27,44,245,1,231,38,217,156,93,238,240,189,216,12,19,43,251,206,170,29,6,176,124,203,238,85,36,95,172,122,49,215,14,83,91,12,189,154,252,184},
  {90,6,74,119,127,156,235,106,1,29,234,104,229,15,167,252,87,147,213,55,222,44,109,86,64,246,182,208,55,101,230,230,182,152,129,113,41,240,23,16,243,89,164,203,74,253,19,7,176,238,180,109,222,250,236,128,140,240,238,165,138,37,199,35},
  {15,175,136,25,173,216,247,254,165,79,208,96,254,123,37,204,130,108,204,9,129,148,77,223,95,178,154,217,216,125,76,35,209,243,90,173,83,238,65,9,208,67,134,145,250,132,113,253,8,146,106,225,79,112,52,4,220,90,243,9,73,50,109,89},
  {235,176,174,149,75,199,68,1,201,234,123,129,6,9,227,35,35,126,129,27,142,189,44,26,84,69,68,111,60,148,159,174,214,231,53,132,141,15,99,88,127,218,158,8,24,7,232,0,252,97,65,87,79,214,187,222,189,249,68,221,37,20,102,101},
  {150,118,227,102,87,244,124,54,111,91,155,132,44,128,231,182,122,177,10,208,143,169,242,102,108,238,212,252,38,246,120,170,18,174,198,21,83,5,111,189,210,27,234,40,154,135,198,169,2,109,123,110,78,145,218,255,153,60,155,160,232,60,225,51},
  {33,88,211,34,64,90,213,27,84,221,9,15,164,245,4,190,208,93,209,167,109,177,162,57,122,24,213,252,246,133,4,93,115,121,215,144,192,207,191,139,250,134,27,206,38,245,246,248,242,209,147,92,22,208,167,140,239,100,181,72,124,253,189,86},
  {23,42,9,168,149,67,225,68,221,92,5,101,135,205,220,233,113,229,93,28,11,14,240,8,223,244,78,233,61,201,117,195,203,239,136,244,145,213,179,193,69,248,238,30,49,168,84,223,122,219,155,116,77,182,16,144,221,148,18,214,220,87,145,76},
  {172,37,236,32,17,202,10,98,43,251,179,124,92,172,152,16,151,116,16,0,95,217,219,130,228,74,231,241,234,71,130,228,213,107,205,33,28,54,95,81,39,43,202,247,49,81,34,157,110,184,9,52,255,196,125,211,33,77,87,17,72,142,79,252},
  {150,233,194,54,155,39,49,154,32,100,23,54,135,243,66,176,95,170,253,9,95,198,10,76,179,116,65,226,19,206,61,75,77,112,16,76,136,128,23,122,221,125,73,98,246,231,76,233,174,115,158,61,171,210,129,114,15,96,52,53,26,148,183,9},
  {1,143,5,254,141,156,221,179,170,95,175,24,168,198,50,141,40,141,32,92,12,134,72,244,206,168,15,127,61,195,218,47,91,195,156,203,55,154,54,162,249,75,142,211,86,147,206,68,140,105,78,111,152,248,124,160,206,127,231,186,177,199,207,112},
  {254,164,41,191,208,165,189,79,230,205,190,82,12,194,86,16,71,124,103,109,196,160,233,80,201,247,78,6,128,160,247,26,142,186,213,16,245,185,19,114,215,8,4,117,242,89,19,10,213,30,196,23,229,242,94,14,6,170,152,46,82,221,251,67},
  {61,92,137,6,34,150,107,7,148,8,26,254,197,74,65,243,54,162,117,165,219,103,164,127,89,179,183,219,27,53,147,25,10,38,203,255,194,23,25,140,203,74,249,111,52,86,86,190,135,255,169,111,205,74,26,59,164,156,81,61,76,226,91,26},
  {209,181,234,189,149,207,147,30,22,222,107,103,214,26,164,77,88,58,47,239,253,86,139,110,77,111,173,118,110,20,112,202,175,156,148,52,75,142,201,81,76,20,186,168,209,123,131,251,1,86,6,53,198,185,99,153,86,26,54,75,63,85,72,191},
  {88,135,198,15,244,0,249,143,217,131,124,180,16,33,34,49,215,207,124,219,214,53,141,227,151,126,198,246,133,28,50,250,56,141,31,212,164,209,181,184,95,14,174,138,21,246,106,156,163,37,102,129,23,47,37,80,22,69,80,122,157,251,86,30},
  {48,217,101,122,57,119,101,209,130,108,197,155,238,104,206,51,22,218,238,190,53,39,187,244,231,124,68,104,139,195,67,219,243,216,33,240,183,234,182,105,60,149,175,107,20,122,140,73,158,248,112,201,31,228,106,106,228,97,199,226,96,78,147,27},
  {140,120,134,114,131,143,42,140,252,3,175,149,95,183,156,36,188,7,6,139,251,109,180,137,230,203,209,217,190,232,138,140,122,179,225,11,72,8,203,76,204,66,198,132,76,251,205,81,146,23,190,232,214,33,239,122,129,52,213,53,203,53,74,240},
  {110,202,34,214,68,131,135,9,183,1,213,132,43,143,152,213,63,182,240,165,137,185,240,166,52,161,34,224,212,192,141,194,182,83,90,119,43,4,31,98,1,121,111,19,17,20,177,60,56,18,69,77,123,43,49,45,76,180,177,95,216,79,67,90},
  {94,114,246,10,223,63,165,241,29,40,236,6,152,3,235,220,215,250,44,5,138,4,33,165,167,220,232,66,25,216,153,16,59,188,45,87,94,254,250,80,19,138,250,120,81,159,185,6,32,29,128,122,153,242,245,231,223,89,3,95,129,73,70,70},
  {21,171,10,249,206,3,219,18,98,31,194,170,128,250,202,80,18,38,221,51,13,162,100,127,78,124,28,71,207,187,93,131,127,101,159,116,164,65,165,172,222,79,77,78,234,50,153,126,172,118,197,231,236,136,11,27,244,132,85,222,43,37,106,25},
  {39,101,117,76,150,173,138,245,65,161,98,236,22,91,73,63,24,156,252,184,36,227,177,5,0,134,13,114,93,124,212,208,242,208,255,202,204,131,139,163,109,193,136,28,175,58,163,110,245,94,181,67,96,137,56,91,182,114,159,225,33,245,253,243},
  {90,124,101,194,183,238,181,83,64,18,104,46,48,40,106,115,138,244,44,41,232,106,71,225,252,7,127,157,160,222,17,112,79,219,131,178,82,214,173,64,191,121,255,99,95,119,142,17,32,59,7,172,192,95,146,156,51,88,191,85,45,192,102,155},
  {17,150,128,217,168,128,47,247,30,95,29,206,129,58,69,125,59,22,61,167,37,208,16,255,70,217,223,193,113,21,174,72,13,164,239,78,193,219,76,177,155,114,220,240,67,15,115,249,191,223,10,87,138,253,161,245,51,60,138,158,179,188,214,150},
  {125,122,129,99,100,83,139,36,25,5,105,72,13,43,91,126,141,101,191,77,185,233,220,215,15,251,101,193,196,190,118,214,105,175,85,105,171,190,207,109,138,54,234,178,190,119,76,2,157,88,43,114,50,8,190,211,201,169,66,81,198,224,176,131},
  {56,216,82,248,119,166,33,241,123,179,233,82,89,98,112,99,121,211,21,156,115,10,143,113,133,77,137,117,39,24,218,156,167,24,137,58,26,142,62,129,127,13,218,216,36,47,77,174,140,91,197,132,71,81,173,55,201,207,137,208,50,198,75,121},
  {19,56,52,249,87,122,96,42,253,25,106,245,140,45,164,36,6,251,174,59,47,126,231,88,212,250,165,87,194,73,86,129,150,193,246,5,82,135,157,255,209,102,224,159,194,65,124,21,138,10,19,58,26,170,28,214,150,125,86,31,192,166,162,13},
  {92,59,126,154,0,251,5,55,215,202,252,87,40,88,129,21,81,254,46,239,97,29,231,56,141,26,174,74,148,24,113,141,157,196,170,47,140,135,187,68,109,4,86,159,174,69,152,160,121,208,36,87,255,32,191,43,135,246,72,21,35,188,66,83},
  {1,238,24,201,178,207,86,169,146,84,111,162,109,24,110,131,5,51,117,137,71,24,150,115,227,241,102,182,109,239,124,8,155,236,6,124,168,25,192,21,241,104,61,120,160,244,51,112,79,53,44,151,241,171,145,102,42,112,109,19,171,140,66,73},
  {205,98,205,214,3,231,180,57,35,75,138,12,167,197,146,43,74,187,145,167,196,234,222,216,9,96,136,127,126,162,73,114,132,236,77,215,31,104,197,78,128,1,95,21,155,213,119,117,238,102,187,73,233,69,134,98,47,110,112,52,40,80,54,124},
  {188,116,138,97,71,249,140,175,239,220,21,154,43,131,216,23,68,28,80,241,44,193,208,135,158,76,75,116,236,247,72,234,233,6,33,114,50,179,202,234,54,60,172,89,38,215,130,118,233,82,126,130,147,248,179,246,120,89,69,159,142,239,147,188},
  {103,139,183,239,241,21,120,151,88,78,62,118,66,185,196,63,15,140,180,214,205,94,37,121,106,27,55,166,158,130,222,208,248,83,62,222,189,177,112,171,4,1,165,137,237,217,215,61,45,100,215,78,190,109,90,230,80,71,182,203,28,14,106,173},
  {185,248,163,215,145,159,175,165,82,103,249,107,23,189,102,135,148,108,75,211,131,15,33,198,91,0,102,42,8,46,246,9,130,149,254,204,10,91,157,255,143,72,237,181,199,65,213,48,112,73,22,236,56,142,224,202,11,180,123,1,211,158,161,65},
  {47,15,230,227,13,90,96,35,140,105,106,70,116,197,143,167,9,109,203,19,223,155,165,59,119,49,6,25,122,191,220,21,128,229,59,104,230,254,193,95,107,237,5,135,181,216,215,55,254,26,31,153,2,2,35,248,120,37,234,5,215,137,132,218},
  {246,34,116,39,99,140,255,153,195,41,170,28,71,22,190,16,63,79,145,179,149,163,220,222,55,84,177,126,4,138,30,63,94,16,130,152,114,48,9,134,115,152,62,85,36,232,246,108,111,81,120,195,130,9,107,112,76,251,247,19,21,65,160,218},
  {209,72,178,230,20,215,145,110,143,247,56,70,201,212,128,5,137,31,25,120,120,76,214,153,88,203,124,109,149,206,147,98,244,58,178,38,238,97,143,50,202,191,57,38,177,9,79,109,36,124,171,154,250,50,98,101,215,94,165,209,123,204,195,213},
  {18,225,198,22,94,21,180,89,23,104,204,234,30,204,255,118,162,114,187,206,255,78,144,100,33,53,155,146,124,149,68,115,19,229,197,26,2,175,238,213,184,123,127,69,175,149,174,250,253,189,14,139,177,144,174,228,241,189,181,41,119,91,53,111},
  {46,26,240,66,138,195,250,142,71,88,160,222,73,200,98,51,151,29,73,75,66,13,192,84,52,205,9,119,16,180,144,5,40,251,117,212,110,187,37,34,86,224,120,225,19,249,109,184,167,248,115,141,185,93,196,29,2,24,0,223,159,61,192,54},
  {8,236,140,201,126,170,97,174,65,143,160,31,105,145,254,249,239,247,198,171,216,199,2,207,52,26,79,27,97,84,46,236,230,49,43,196,156,35,236,8,37,189,236,121,130,80,228,7,67,9,20,186,194,160,230,86,33,234,210,123,243,203,193,181},
  {87,107,190,196,97,156,216,66,194,137,211,85,203,147,39,218,148,52,16,253,196,201,151,86,74,19,196,88,132,135,71,131,49,3,180,92,110,225,11,102,167,84,59,180,63,114,16,25,68,51,133,28,120,91,22,134,93,202,231,219,252,59,111,102},
  {143,110,124,194,173,221,193,10,195,164,161,136,165,156,121,153,4,39,5,125,191,68,45,160,222,56,238,189,99,26,138,178,105,230,188,191,155,117,215,155,219,221,251,237,144,47,11,9,91,56,75,80,24,135,203,36,187,16,67,25,18,16,38,83},
  {232,105,232,199,154,126,83,35,27,82,168,211,14,64,56,254,143,144,82,123,145,124,110,113,79,185,55,104,247,157,241,149,119,66,70,55,120,105,214,47,182,208,1,82,126,135,171,219,207,179,179,40,210,139,206,160,229,99,5,144,153,142,187,126},
  {17,231,205,49,59,70,180,67,189,31,128,177,166,167,178,3,178,161,239,40,62,173,156,115,88,151,228,188,47,26,32,100,17,89,238,249,24,217,233,123,194,129,208,87,73,243,39,117,231,235,219,8,246,162,4,202,197,159,167,167,62,31,9,233},
  {182,109,139,182,15,163,98,14,196,185,37,149,212,38,55,81,47,254,2,12,24,147,85,196,47,45,238,213,68,94,219,14,2,122,53,145,21,235,190,196,52,142,208,86,249,228,67,177,47,231,192,248,46,123,227,218,151,233,55,197,238,73,171,149},
  {45,180,112,92,129,43,151,225,144,14,34,15,232,218,73,62,35,95,69,132,122,90,168,137,151,55,31,53,58,166,175,20,236,201,98,242,253,41,104,75,220,195,163,148,149,138,255,68,58,57,251,211,194,124,91,217,137,227,243,152,93,169,4,143},
  {6,33,23,10,26,80,237,36,18,228,168,55,20,224,12,251,52,93,102,175,46,38,130,66,85,168,195,126,219,95,31,250,41,143,145,41,242,7,30,14,166,249,252,202,138,243,40,41,103,89,133,9,7,154,65,32,36,241,21,247,149,99,48,78},
  {12,122,212,72,89,7,115,202,14,103,9,206,168,142,171,226,178,194,102,85,242,217,133,247,215,163,57,66,134,162,128,55,150,78,152,125,249,175,116,36,94,222,227,140,151,98,214,105,16,27,174,6,39,131,229,238,79,22,164,132,116,169,228,53},
  {167,144,75,53,222,44,55,245,157,236,178,243,115,16,42,19,67,135,216,102,10,46,170,2,38,209,56,120,133,195,226,33,16,74,229,77,87,20,8,1,251,98,145,193,27,214,178,137,143,198,152,51,195,208,131,149,108,136,24,232,185,205,104,134},
  {138,28,146,113,188,179,69,142,119,229,56,30,161,55,58,62,7,147,2,57,106,26,90,239,87,66,162,158,50,129,220,81,146,120,64,148,251,253,133,152,224,222,52,228,200,15,62,120,55,81,4,255,145,176,130,32,118,0,181,42,174,147,43,226},
  {149,235,116,39,243,177,18,254,191,231,41,248,7,175,92,105,250,7,15,162,9,226,135,151,137,179,198,112,3,98,225,69,94,166,251,233,249,153,32,145,244,187,33,182,59,177,133,50,20,20,131,162,244,55,242,214,70,180,32,9,134,182,98,212},
  {217,158,60,158,190,171,81,71,128,188,253,169,75,30,174,243,43,217,32,202,50,146,106,104,92,27,8,110,74,223,196,249,2,57,96,6,99,80,156,142,151,76,182,59,128,208,167,85,190,161,246,201,209,196,144,238,201,241,109,119,41,46,136,40},
  {73,104,197,39,157,212,237,220,182,159,36,63,41,92,133,198,51,225,162,143,170,97,109,149,68,186,217,112,83,45,60,44,152,0,47,108,196,102,109,215,38,169,48,49,8,132,135,30,226,95,25,45,183,84,47,95,149,107,145,155,58,99,225,60},
  {46,132,157,149,28,171,1,116,2,246,39,18,3,88,224,201,137,119,1,73,118,171,176,238,134,199,69,246,22,73,194,9,52,65,218,0,182,109,11,40,186,18,14,166,14,168,135,89,12,194,21,233,76,255,102,185,18,248,125,134,5,26,193,111},
  {198,18,97,51,91,148,134,249,207,239,215,238,12,100,25,107,94,139,141,5,163,2,75,59,6,194,250,215,206,41,200,142,163,33,108,161,111,101,201,234,145,158,254,43,221,162,229,27,227,150,69,229,129,100,197,63,22,57,190,227,111,218,33,225},
  {80,163,246,209,93,216,64,181,217,120,113,111,219,245,18,167,158,39,50,10,226,149,110,234,110,122,8,31,81,193,135,57,218,23,88,247,26,21,94,121,123,76,209,198,23,50,96,187,96,154,149,108,130,184,154,77,218,29,178,189,197,211,176,204},
  {207,134,58,218,92,203,35,189,218,41,119,12,225,183,6,187,23,133,153,88,224,160,195,214,98,59,215,16,201,229,196,217,61,77,0,251,133,36,95,82,48,42,48,57,65,3,166,127,150,134,28,248,67,226,39,65,77,68,114,224,104,213,212,94},
  {207,8,48,72,168,20,225,25,8,111,84,209,102,109,22,218,223,62,148,208,60,173,26,232,114,35,200,74,167,192,14,134,196,39,248,114,169,92,3,252,80,9,158,239,247,105,117,160,101,205,18,77,188,39,134,131,167,56,184,162,177,153,114,197},
  {109,71,57,149,94,78,221,191,81,0,100,200,45,146,158,166,111,88,103,255,189,142,59,201,173,20,95,74,90,254,216,236,78,221,19,59,143,67,110,33,179,234,32,228,148,38,213,110,27,29,234,231,246,136,84,178,219,74,209,188,189,176,224,101},
  {115,141,238,54,85,98,1,129,151,40,32,27,183,86,149,152,101,203,23,213,14,111,11,100,185,192,97,249,221,42,249,79,32,254,53,43,140,216,196,85,118,26,230,67,134,214,148,24,206,238,182,98,2,90,254,4,254,40,151,117,160,80,236,82},
  {18,129,251,148,77,204,30,131,168,167,96,115,177,137,22,219,100,8,163,226,182,95,183,98,181,172,184,70,251,45,179,3,217,225,52,50,225,179,184,64,156,18,62,149,186,56,50,10,239,193,105,183,241,32,232,35,30,93,244,229,115,48,46,227},
  {124,18,76,202,170,211,50,76,30,5,28,181,68,26,116,212,246,166,117,23,112,233,70,209,49,152,132,73,66,249,220,92,96,51,160,53,216,47,125,104,101,26,242,194,75,194,186,36,40,131,180,128,219,102,250,214,75,13,222,41,246,193,226,24},
  {171,28,213,215,22,73,53,11,140,211,214,91,126,37,79,220,83,168,110,246,39,47,186,10,41,233,95,172,153,167,100,117,29,243,103,230,179,72,161,255,83,38,96,164,88,248,59,100,169,189,173,108,196,150,252,140,210,67,188,101,46,203,199,169},
  {191,159,171,96,193,210,197,160,187,253,180,224,115,132,3,149,241,226,218,117,165,255,78,238,8,47,155,159,153,239,154,250,152,121,79,178,74,24,209,187,174,123,168,207,173,40,224,173,105,81,11,106,153,183,53,17,135,221,246,185,137,124,118,185},
  {156,173,28,172,87,15,241,153,52,29,164,76,244,93,32,104,16,190,9,234,71,238,190,62,97,241,221,48,131,50,38,102,125,56,79,194,147,69,151,241,248,151,81,210,215,69,67,174,216,20,168,196,153,95,23,54,246,177,62,156,211,97,227,208},
  {39,84,203,200,33,78,183,157,46,66,214,77,164,219,139,211,134,101,46,254,6,120,47,37,125,220,250,149,26,183,50,167,33,117,49,198,184,6,86,46,111,34,77,97,77,84,190,47,42,160,189,56,82,128,236,77,70,150,113,184,39,24,191,143},
  {47,170,80,43,120,140,181,105,110,231,224,90,247,148,168,19,226,45,151,94,93,17,150,1,29,133,88,104,190,220,142,192,70,26,220,242,35,216,89,233,166,213,42,183,151,188,7,169,118,20,216,56,155,119,62,224,130,131,49,252,47,189,126,119},
  {36,235,101,95,31,236,177,136,189,133,164,188,53,192,178,103,124,168,159,19,96,80,49,222,52,119,170,189,73,57,219,72,226,167,225,15,253,196,110,252,65,171,116,57,164,14,232,57,40,2,153,228,42,14,206,119,181,23,49,253,131,8,33,182},
  {5,108,97,54,96,230,153,71,106,3,218,125,149,5,159,154,188,196,147,252,223,103,80,72,97,34,170,120,214,169,223,121,88,115,195,128,71,143,28,196,152,172,55,216,147,195,255,226,165,41,228,153,106,97,147,239,247,194,126,221,97,186,201,88},
  {173,81,213,70,32,139,78,30,36,177,96,197,184,233,228,128,44,216,189,182,26,215,81,164,17,36,48,68,173,6,44,48,207,136,1,93,25,9,79,36,73,209,228,55,76,108,199,65,33,138,112,94,78,144,184,158,25,110,136,139,255,238,174,203},
  {25,92,130,161,236,117,229,84,64,240,225,48,70,103,223,159,230,71,15,105,105,41,180,175,44,115,42,184,102,59,184,173,253,148,214,131,34,144,14,204,119,126,146,245,16,224,108,83,95,213,24,2,166,206,169,47,250,179,57,19,71,196,67,68},
  {205,81,237,151,104,62,56,197,209,59,65,52,112,97,217,204,169,47,111,146,67,238,180,49,158,239,158,236,107,97,6,180,144,119,192,185,131,10,38,173,206,202,243,229,243,136,186,48,7,27,192,18,81,51,166,80,161,29,66,9,2,236,235,42},
  {27,250,68,106,69,135,8,17,92,199,244,83,21,216,247,255,86,57,236,57,224,220,208,165,121,215,67,40,95,51,225,220,202,83,249,38,68,60,47,197,186,102,92,212,229,254,220,180,37,38,171,126,2,188,211,212,13,40,161,91,133,187,103,105},
  {68,100,155,89,237,22,117,100,171,79,170,41,63,92,46,112,139,137,142,58,0,252,236,128,197,81,87,255,243,19,14,239,221,172,114,234,5,101,79,179,30,234,65,160,52,161,130,228,98,4,17,21,122,29,225,182,203,80,106,195,219,72,57,143},
  {5,18,113,52,85,89,174,185,54,255,162,123,102,134,49,39,238,106,194,35,48,102,63,116,156,76,125,146,5,61,164,198,226,170,114,23,222,230,16,236,102,139,45,131,159,4,92,99,153,140,130,53,217,58,252,173,245,129,161,208,226,246,241,250},
  {9,12,65,92,141,211,51,205,191,85,91,214,242,128,113,167,208,193,195,194,76,94,16,80,148,201,194,60,160,50,148,138,190,45,25,232,16,186,242,210,75,143,170,134,144,125,136,61,220,192,4,229,131,231,82,65,56,28,240,99,178,130,216,108},
  {253,161,124,49,71,104,59,14,197,58,198,3,2,180,47,226,99,120,236,127,124,70,207,143,67,95,114,76,127,32,28,44,104,87,126,75,17,171,252,168,0,253,235,98,144,101,42,59,225,71,173,225,1,109,188,181,249,238,159,238,24,163,114,165},
  {14,83,99,227,44,181,29,117,72,145,85,163,141,92,245,239,109,203,235,168,5,73,104,218,22,191,168,158,156,24,209,55,214,194,242,49,85,209,110,94,40,245,48,38,243,15,29,227,208,171,133,13,238,76,135,185,87,221,62,219,240,247,160,54},
  {172,153,23,5,217,225,127,61,47,82,197,188,253,153,164,97,162,236,180,19,239,25,226,201,149,189,143,114,47,107,105,219,185,247,176,74,193,250,225,174,112,4,127,255,190,119,184,209,125,215,115,20,98,65,254,71,229,81,27,147,227,74,20,18},
  {242,202,119,59,130,110,137,56,89,31,116,119,219,183,56,32,180,116,34,189,6,115,215,155,96,238,116,199,113,122,93,131,94,173,130,210,198,43,214,250,162,161,197,203,112,254,189,184,9,208,250,210,253,124,82,98,153,70,131,95,24,53,181,97},
  {31,169,161,54,171,247,212,8,187,222,193,175,219,74,22,216,213,218,142,161,8,231,122,76,221,89,106,46,32,156,185,120,46,0,115,96,217,143,199,87,215,149,160,44,209,69,96,114,95,76,77,110,153,90,109,223,146,230,246,121,111,243,45,253},
  {178,133,155,88,53,191,29,127,62,85,109,89,223,233,197,21,105,108,224,145,93,218,129,60,82,123,205,224,228,190,206,214,252,92,251,143,224,54,68,121,104,120,164,222,173,116,18,40,120,170,43,199,187,155,61,115,175,28,156,175,194,231,133,90},
  {44,165,20,253,169,77,142,157,138,235,8,94,82,83,62,210,235,215,108,179,141,177,3,124,163,205,152,244,131,193,92,18,76,60,107,104,50,27,144,43,67,213,13,113,142,125,231,12,17,23,105,136,224,231,134,72,79,100,244,252,152,175,224,52},
  {254,197,187,189,154,165,185,64,65,167,17,132,24,37,18,119,134,132,121,123,222,30,202,74,154,91,234,221,35,20,242,254,171,167,145,65,128,175,176,37,130,67,254,175,74,102,254,42,202,141,143,171,83,189,17,152,74,105,97,217,218,49,210,231},
  {134,108,81,3,119,194,231,152,100,83,182,196,215,22,208,169,104,64,123,46,179,85,122,29,155,186,142,125,248,89,0,22,66,223,190,131,176,180,65,24,193,247,10,41,165,189,25,254,132,59,62,29,147,133,45,167,82,20,151,49,6,17,227,129},
  {87,155,98,90,210,82,97,43,159,197,38,198,21,82,107,135,235,221,17,97,36,230,11,233,101,8,25,119,191,149,49,12,25,105,236,105,178,6,201,247,242,140,70,143,220,127,91,119,254,64,244,26,109,166,136,95,87,154,7,78,223,244,173,19},
  {14,37,56,41,228,116,185,166,177,208,177,165,162,226,107,3,239,160,144,3,38,230,115,97,148,42,230,176,187,143,23,230,100,129,106,199,208,185,75,48,58,38,69,150,168,87,4,184,148,90,121,45,9,219,76,35,44,218,157,164,196,190,196,207},
  {131,248,154,140,112,176,222,235,17,13,162,238,64,211,140,42,85,113,189,251,49,107,0,90,215,201,120,243,70,243,59,211,238,100,220,142,116,151,196,254,222,197,228,6,253,202,78,29,40,33,238,117,198,49,200,154,52,62,91,51,255,239,155,205},
  {147,73,122,2,95,222,165,110,175,206,207,47,253,44,217,108,30,169,73,186,41,203,82,161,102,26,65,133,38,34,137,83,200,91,97,68,208,113,240,95,29,242,101,128,144,28,60,216,135,56,140,21,170,202,169,236,237,76,141,90,191,136,39,191},
  {220,175,73,29,205,206,167,28,179,132,245,123,143,223,3,136,251,241,221,41,186,168,59,237,171,29,129,9,175,123,236,177,19,228,89,50,222,96,116,152,27,196,158,9,27,56,102,96,223,93,76,190,146,218,222,63,87,87,152,56,193,112,226,251},
  {246,224,71,2,195,14,113,180,222,135,88,91,238,96,197,2,190,130,168,100,78,29,71,97,6,145,58,134,227,215,72,130,105,47,88,65,239,8,115,104,251,185,63,143,112,49,43,249,184,75,115,36,202,55,39,168,225,111,238,206,31,209,144,33},
  {74,98,159,204,117,5,76,155,155,147,199,171,41,188,89,171,183,232,72,54,53,41,3,55,54,85,165,131,82,186,96,49,75,134,166,9,107,132,76,106,175,42,185,149,18,108,199,67,164,185,242,128,248,153,98,74,89,220,179,54,149,231,149,219},
  {4,145,9,42,171,194,229,233,52,88,190,228,190,222,68,135,168,48,69,210,70,143,23,36,154,38,179,236,10,15,223,239,152,92,179,6,5,245,84,85,193,76,111,196,186,250,156,161,160,188,47,212,209,18,14,250,45,252,43,70,20,159,165,12},
  {143,220,233,112,70,65,20,178,104,91,248,35,197,179,99,206,155,6,112,142,226,37,148,83,137,228,122,249,59,82,102,193,193,49,237,227,63,215,117,69,146,114,83,65,143,21,5,52,100,70,166,94,254,162,124,57,31,146,107,250,220,46,48,210},
  {74,141,73,135,168,17,244,91,161,53,237,232,131,49,58,140,175,5,83,120,7,252,173,85,105,72,6,191,42,232,59,223,158,23,3,230,174,75,253,35,123,213,53,144,58,1,85,66,226,96,140,65,43,249,98,144,82,30,231,90,191,93,71,30},
  {129,85,99,153,177,59,85,81,226,56,208,148,88,67,123,80,15,94,23,3,82,63,140,147,82,140,206,46,70,173,245,128,19,50,242,204,219,11,213,204,74,211,115,68,181,248,47,19,63,219,165,115,83,60,229,179,107,92,15,90,251,1,214,89},
  {158,112,251,126,223,141,246,49,88,113,153,160,233,47,203,41,142,154,140,113,26,112,157,145,128,179,240,229,240,246,24,169,229,227,250,38,11,33,107,89,147,0,47,169,188,36,30,43,24,69,162,171,228,133,233,125,250,180,166,62,128,194,126,96},
  {138,42,16,151,1,40,58,88,213,38,245,200,208,98,206,130,79,146,26,34,1,217,50,82,233,167,231,28,45,11,99,115,179,247,196,134,32,240,171,128,40,153,248,49,10,16,27,192,169,65,191,183,199,98,106,207,36,102,45,217,190,37,36,238},
  {21,150,103,39,221,112,136,105,246,247,204,108,186,236,220,63,170,42,46,125,212,155,47,207,238,206,60,94,15,184,197,4,79,109,101,66,111,109,79,21,205,192,230,222,197,161,222,176,189,230,149,121,46,176,213,209,174,154,6,201,67,57,103,99},
  {130,175,197,155,124,136,216,18,174,47,139,133,153,146,17,228,25,161,188,150,173,246,13,160,152,73,117,229,116,238,111,88,76,218,152,72,58,172,238,80,4,124,50,251,104,73,51,147,167,117,86,38,138,62,215,71,148,116,10,120,92,150,236,95},
  {114,133,254,32,147,11,3,1,50,26,49,60,5,138,113,0,94,146,31,25,104,44,31,253,61,242,172,110,149,26,239,55,84,238,117,164,133,112,55,77,211,31,70,229,1,78,70,186,8,248,35,34,95,36,74,206,160,83,135,240,170,57,142,221},
  {80,174,98,51,244,50,158,117,14,244,195,248,114,15,97,217,20,109,92,78,245,138,66,236,51,109,9,60,72,218,47,162,107,112,33,41,15,76,203,16,255,101,110,247,107,175,190,159,130,142,233,105,116,175,194,181,75,224,199,79,124,21,35,178},
  {77,57,194,150,238,157,124,237,1,254,44,245,240,8,208,95,191,202,22,155,240,53,183,46,243,194,74,102,35,94,156,26,77,135,20,216,132,81,57,37,224,145,20,53,103,183,51,56,219,169,13,160,242,188,27,208,243,66,111,244,46,63,194,138},
  {234,155,226,137,69,255,33,70,196,5,120,205,168,123,71,192,21,153,96,54,8,71,93,159,31,251,163,140,119,23,32,116,99,123,163,25,42,210,206,163,164,244,220,228,233,192,209,55,44,116,103,121,167,166,35,85,82,242,197,79,214,8,149,131},
  {21,125,219,61,204,220,120,132,102,255,45,210,98,218,200,234,68,166,172,116,127,222,173,41,33,165,248,254,239,193,117,164,145,194,24,200,207,90,38,14,143,254,93,8,60,7,122,182,224,203,254,183,179,21,5,242,253,131,189,37,40,77,180,194},
  {93,174,39,41,49,250,206,29,88,137,212,1,37,79,193,89,47,189,207,132,63,172,81,199,203,192,60,47,90,213,51,136,207,114,110,249,208,160,70,37,51,200,204,218,40,245,32,69,111,229,3,92,234,37,127,112,229,167,195,196,77,222,120,27},
  {97,20,118,126,153,175,217,148,123,242,202,27,73,230,131,223,215,148,122,144,9,196,65,98,45,131,181,22,187,200,158,249,123,32,157,44,197,138,235,192,62,137,10,122,137,40,252,238,57,141,210,166,44,141,27,8,25,200,142,200,112,118,183,164},
  {242,164,31,199,237,100,217,1,180,22,30,65,15,163,172,107,142,202,228,15,59,216,147,90,216,38,199,228,65,137,104,66,145,2,169,83,166,228,88,146,118,203,38,112,36,56,224,109,164,160,159,54,235,86,53,192,29,85,11,27,192,136,152,172},
  {147,70,227,102,142,17,16,254,6,103,253,142,77,105,251,249,252,87,148,142,236,180,205,102,77,173,224,198,125,238,27,47,34,99,15,62,237,11,36,123,179,186,185,188,41,197,16,235,129,53,16,221,152,8,252,42,85,65,183,43,215,39,245,129},
  {171,152,80,207,160,59,203,100,143,138,131,218,171,184,52,92,235,180,238,198,30,215,176,135,143,176,180,157,187,154,179,157,113,226,109,79,58,96,237,67,218,123,180,139,7,13,164,175,81,20,38,57,77,48,250,131,167,123,11,161,178,173,182,182},
  {219,65,110,116,62,16,178,220,181,77,80,11,0,139,206,28,193,144,188,153,254,195,132,180,232,2,54,78,228,115,99,91,115,1,109,0,184,222,68,251,13,152,98,53,210,74,171,186,124,64,129,142,50,237,19,242,132,35,232,71,114,132,233,63},
  {75,41,73,9,199,94,27,224,112,69,95,166,54,86,63,37,188,7,185,72,104,235,111,79,198,208,207,184,48,216,26,174,147,87,226,227,88,254,247,0,222,196,0,3,192,109,158,72,51,219,126,63,133,208,119,202,175,105,62,162,65,37,42,130},
  {50,126,23,109,68,133,155,156,217,22,172,193,197,218,203,124,110,224,27,158,253,165,85,246,215,147,103,85,206,79,154,253,116,179,4,146,59,30,241,81,13,6,31,108,84,52,151,235,99,84,250,201,56,152,5,36,103,118,89,114,180,22,134,78},
  {142,126,137,185,25,177,75,128,148,113,60,167,96,8,18,210,7,108,237,156,73,11,211,136,38,138,216,62,251,46,223,117,21,13,150,60,125,198,29,45,27,191,67,75,148,105,101,144,203,102,50,158,167,211,52,186,24,113,11,46,142,97,115,192},
  {34,63,4,211,143,33,185,22,179,151,163,78,97,139,95,91,38,71,136,133,152,116,65,153,136,182,217,62,211,45,45,252,25,229,171,245,105,176,166,96,246,131,195,153,211,188,184,67,34,30,34,14,171,89,146,23,248,146,231,119,22,84,236,84},
  {156,15,101,160,27,154,0,29,232,217,28,97,41,113,136,34,64,85,55,204,33,92,254,236,180,158,223,98,20,161,74,68,12,1,191,176,209,16,117,165,246,52,187,70,182,232,216,200,132,246,36,228,158,157,170,27,84,171,154,99,53,111,131,53},
  {249,71,115,69,194,249,255,217,170,125,182,152,125,249,110,88,229,164,199,87,120,65,103,92,13,239,101,199,159,96,89,199,161,43,154,79,109,157,147,233,59,85,200,235,61,191,35,83,206,83,55,100,165,112,186,73,73,75,30,155,252,228,192,1},
  {132,20,96,140,203,126,73,214,76,213,255,168,133,59,96,217,133,58,186,38,145,128,95,206,208,245,244,69,141,110,230,24,62,152,34,241,218,160,178,75,23,39,241,187,57,125,139,186,17,82,209,56,117,69,226,214,165,129,158,84,83,177,140,154},
  {126,120,194,48,184,7,192,50,190,200,30,215,218,34,0,122,189,219,42,155,53,200,224,178,38,180,52,244,81,124,23,140,38,212,37,214,71,234,75,111,239,174,50,200,248,170,208,80,81,77,61,13,198,213,2,156,78,182,123,51,179,195,102,252},
  {37,70,218,208,202,82,62,107,232,177,221,100,3,187,8,203,52,104,20,140,243,136,123,2,61,224,209,127,0,28,208,95,193,78,187,35,178,68,2,156,252,191,244,161,60,38,254,144,137,172,125,237,218,113,86,192,159,50,108,186,238,238,69,37},
  {62,144,245,240,117,132,213,140,77,73,23,236,201,57,99,168,240,204,80,148,141,41,76,154,192,216,173,247,41,218,181,72,41,200,40,243,180,255,30,228,93,51,177,174,225,244,121,84,217,112,98,40,50,171,80,244,81,87,239,121,22,145,27,25},
  {219,15,136,27,168,13,9,69,74,111,181,89,54,13,194,227,207,84,119,193,156,232,65,5,141,188,93,217,33,167,111,178,94,67,128,105,183,27,31,177,159,164,108,48,2,206,135,126,141,219,71,22,169,115,83,66,149,51,229,20,174,14,142,16},
  {92,87,209,72,136,101,148,139,72,243,226,126,204,107,10,214,69,0,212,237,8,218,240,10,253,251,170,43,164,132,95,206,233,106,113,169,135,32,165,203,185,135,75,3,200,198,246,57,193,251,161,134,116,0,200,173,162,197,64,80,226,64,123,116},
  {154,226,188,29,207,58,204,67,145,231,224,17,151,250,114,162,123,26,164,26,65,123,220,195,44,141,164,96,195,118,235,198,2,196,237,0,26,10,86,68,69,126,113,56,70,80,189,220,168,160,90,212,34,208,171,3,84,176,189,89,151,69,125,243},
  {11,14,17,45,30,238,85,17,20,107,199,8,34,71,135,138,191,23,213,148,8,95,218,250,40,112,59,222,222,73,247,2,120,124,83,103,25,77,98,165,139,15,217,217,35,234,11,248,95,23,222,39,179,33,208,102,43,77,194,146,161,160,21,20},
  {200,215,62,177,19,93,74,25,59,141,5,38,241,150,196,120,6,77,139,250,160,117,2,206,193,252,73,96,185,58,207,49,139,149,148,59,230,62,172,184,158,178,172,89,111,211,170,38,150,127,124,253,71,41,169,5,138,244,59,247,153,144,50,40},
  {153,140,159,212,25,196,204,88,54,129,130,91,134,137,42,225,128,76,19,119,225,29,255,139,62,47,34,74,144,252,100,114,197,138,83,172,189,181,135,99,174,155,187,74,2,245,80,242,82,170,135,76,151,227,244,198,125,234,115,199,146,228,179,43},
  {243,206,191,204,135,200,231,168,195,105,221,106,16,144,125,2,40,255,145,167,203,231,87,98,234,90,120,209,218,233,247,94,132,130,71,83,59,229,88,30,137,188,191,46,227,176,171,4,242,120,56,174,250,102,90,154,194,143,176,183,59,66,234,173},
  {36,169,122,77,218,37,200,41,226,195,191,84,67,165,228,227,170,182,200,178,203,105,26,66,36,253,31,13,75,236,197,209,88,4,213,186,125,202,128,63,69,59,237,226,35,19,82,182,17,100,9,129,226,40,198,134,161,45,211,34,94,152,211,51},
  {196,115,91,214,46,252,212,148,31,147,239,114,151,170,2,159,7,185,136,190,197,144,151,12,73,162,160,201,61,54,1,133,71,190,4,31,188,238,252,59,22,42,38,137,217,28,62,128,18,128,192,197,254,185,8,172,192,144,155,192,169,1,214,81},
  {185,12,54,180,225,175,58,55,101,237,77,148,67,47,135,55,31,83,209,139,166,204,23,241,53,172,177,83,33,52,67,142,229,13,126,195,254,81,235,153,225,173,50,164,59,65,189,207,158,88,94,52,60,141,6,157,230,31,108,203,254,118,70,176},
  {246,252,167,11,132,100,239,88,196,58,112,53,254,54,67,8,139,37,207,110,168,206,239,203,98,36,40,36,118,205,124,198,60,99,131,250,134,111,69,56,152,0,101,83,95,111,123,210,234,208,164,232,218,93,155,114,89,197,56,11,147,211,188,208},
  {251,213,16,229,248,198,61,233,97,47,192,26,243,146,204,117,74,82,111,195,63,56,111,146,0,238,185,116,46,46,14,151,40,52,88,179,70,240,165,41,135,182,209,11,5,29,61,180,3,110,163,85,133,187,51,81,239,221,88,104,88,59,201,97},
  {11,124,191,250,55,47,167,194,24,93,13,15,87,98,181,59,123,225,1,12,31,154,167,244,254,134,119,175,79,146,82,214,35,74,251,94,78,155,85,181,144,232,104,58,219,12,86,101,229,212,163,3,217,251,69,47,15,116,73,18,100,201,57,5},
  {183,16,219,162,152,244,143,43,223,189,102,1,213,94,213,253,55,28,46,129,32,47,118,224,84,71,251,52,110,130,214,224,25,206,45,253,66,185,19,27,91,146,218,211,0,113,230,198,170,16,210,69,220,65,144,230,168,200,3,81,124,153,135,9},
  {36,83,95,248,54,143,3,115,74,74,171,42,20,9,200,127,193,223,224,181,150,192,94,87,251,10,125,222,79,185,34,103,113,20,117,217,211,132,0,51,111,147,115,58,252,248,238,187,249,161,36,220,139,70,33,231,247,227,220,153,77,231,147,132},
  {76,202,187,29,91,51,169,133,58,65,199,57,145,14,151,26,195,181,199,84,51,129,160,90,34,233,76,220,234,69,215,150,43,128,25,152,141,24,192,164,11,179,181,95,171,76,123,4,237,242,129,178,86,28,150,19,39,123,20,196,110,200,170,91},
  {88,179,237,23,194,217,190,229,11,50,236,30,39,37,36,251,101,246,242,165,174,255,68,9,27,222,15,129,165,2,214,211,14,159,31,255,6,218,238,38,0,198,92,133,63,216,32,19,123,104,130,250,220,248,210,242,234,222,9,113,50,170,64,137},
  {100,189,217,77,137,55,180,156,207,118,196,189,180,22,7,173,65,143,22,133,140,120,182,163,60,93,46,28,26,222,51,216,233,67,183,233,169,150,129,178,159,188,60,193,232,53,200,207,133,53,105,158,249,149,93,220,94,85,165,167,222,98,32,239},
  {117,185,93,209,248,186,91,228,93,128,96,166,196,170,182,97,73,65,25,166,11,30,33,153,228,54,194,52,209,219,46,130,144,245,189,159,86,72,47,43,160,219,151,51,91,107,101,98,10,108,10,136,100,103,57,241,10,139,55,111,171,139,4,54},
  {54,220,72,155,95,58,203,179,110,46,188,123,239,115,28,112,45,95,176,186,253,170,95,63,38,6,118,246,228,19,46,186,41,178,242,169,60,73,75,111,88,127,215,64,208,115,255,239,125,172,32,36,121,122,8,86,133,236,128,61,56,97,174,173},
  {227,247,86,247,12,45,120,98,208,224,17,233,8,243,31,90,181,172,32,96,200,3,22,222,68,220,187,211,173,24,215,67,8,152,226,191,53,85,191,150,116,49,221,62,161,252,212,135,194,92,214,232,16,120,29,20,97,44,12,98,186,194,132,75},
  {153,43,111,197,124,94,28,132,69,196,96,222,13,238,114,63,94,180,20,222,146,195,163,124,166,127,118,157,186,240,168,57,31,6,129,60,18,115,72,138,105,73,229,47,142,200,224,185,10,43,142,137,130,205,86,169,2,89,192,48,199,75,136,243},
  {160,249,35,44,202,191,26,249,137,131,149,41,154,66,186,69,184,114,147,164,80,100,182,34,28,92,143,150,130,9,197,181,237,234,60,246,16,151,163,109,95,111,166,155,39,80,54,174,144,56,250,63,91,48,45,251,66,113,54,127,43,108,62,20},
  {251,216,249,198,18,41,64,152,167,16,62,209,201,188,206,161,225,230,21,45,192,221,78,7,105,63,162,51,188,193,31,116,99,131,142,64,232,73,107,159,164,128,122,28,221,171,202,132,40,186,17,151,155,28,22,122,75,210,123,100,22,177,164,48},
  {248,141,181,24,87,11,64,14,25,177,140,68,183,105,43,137,19,176,152,211,224,104,221,177,66,21,241,69,206,105,57,67,127,114,255,36,97,176,160,157,215,33,59,72,24,134,132,53,205,129,87,45,53,48,74,90,54,5,55,65,173,171,32,119},
  {221,154,88,128,7,144,112,61,149,0,181,220,82,202,13,95,246,246,73,183,36,19,56,232,1,218,237,33,108,155,112,176,104,133,243,216,107,172,57,94,207,88,46,236,252,61,153,85,76,194,212,16,169,152,50,247,158,70,75,106,168,187,33,77},
  {99,2,3,195,220,173,199,182,30,20,153,205,161,117,213,51,222,198,195,5,134,30,28,78,127,159,55,20,75,106,68,233,47,77,60,142,189,127,56,12,53,243,96,130,42,9,3,177,204,126,164,145,47,41,228,26,57,233,105,240,201,90,13,250},
  {229,116,196,134,23,234,184,3,140,145,214,199,29,238,117,212,229,115,218,78,183,248,226,148,154,198,24,134,100,239,52,84,2,249,206,65,238,148,85,28,151,116,191,202,201,20,91,31,228,91,244,142,52,247,84,17,47,177,143,212,205,255,68,41},
  {118,30,204,118,134,252,66,184,96,30,223,212,8,9,115,186,80,100,235,145,149,140,143,83,206,3,121,16,112,134,160,244,8,127,199,103,224,180,196,237,105,208,140,46,222,137,121,181,60,8,104,88,132,61,75,134,159,27,33,216,202,163,7,162},
  {165,235,193,25,52,195,98,12,65,45,138,167,91,222,49,136,151,38,131,83,11,168,124,121,5,11,11,65,119,218,248,136,235,50,174,156,3,159,240,139,124,172,18,133,52,119,156,9,154,158,169,105,241,202,54,245,56,176,19,37,67,80,235,221},
  {32,21,151,73,16,125,194,46,0,7,28,209,109,182,0,117,2,81,211,236,228,58,113,197,235,80,40,96,238,104,184,71,45,241,65,22,40,157,192,9,102,178,226,205,61,169,240,16,29,119,251,177,66,176,15,8,226,10,138,149,234,207,94,164},
  {204,13,37,89,196,32,191,222,115,219,238,109,253,20,122,93,213,116,186,131,104,97,203,45,60,41,61,174,120,114,25,158,131,139,132,41,110,52,76,48,107,2,223,241,89,41,51,117,187,138,71,65,61,63,164,69,193,148,50,56,118,203,240,105},
  {182,187,177,54,239,215,245,223,85,18,152,157,1,43,114,84,142,38,201,11,147,197,148,169,239,33,250,29,146,7,57,20,103,141,252,9,89,60,113,97,107,94,94,15,254,91,81,183,212,24,2,210,43,230,148,12,202,201,82,201,122,108,171,132},
  {66,77,237,65,197,138,200,205,254,235,9,180,85,34,232,209,201,78,136,208,212,122,218,222,100,227,97,166,56,127,173,161,233,228,9,232,223,141,214,216,104,175,173,9,79,114,135,99,49,10,230,158,184,141,133,85,111,36,174,34,195,205,31,140},
  {132,59,218,205,170,175,214,177,88,249,91,100,66,97,121,19,41,50,71,197,33,63,112,192,56,18,1,251,132,74,131,232,204,203,47,126,73,34,251,137,5,223,107,210,80,245,33,187,193,10,23,51,229,129,243,64,193,228,134,209,119,71,148,239},
  {29,67,8,153,36,13,74,59,216,43,248,161,200,33,90,5,107,45,147,235,213,219,218,53,112,216,99,18,177,221,129,51,212,203,161,123,84,215,163,151,111,127,66,110,227,142,50,229,161,109,172,140,232,5,127,199,79,10,77,103,237,156,158,0},
  {173,162,28,26,11,157,125,204,43,26,126,60,56,145,15,33,9,30,90,42,60,1,229,23,135,183,89,9,100,76,200,4,226,136,220,15,73,232,112,66,72,87,13,102,220,38,237,197,224,156,237,50,133,74,226,45,91,7,237,210,114,251,232,43},
  {12,115,70,29,125,246,44,220,151,73,32,10,62,32,79,103,54,121,100,190,64,38,136,97,158,30,120,212,167,112,20,85,7,87,211,152,63,103,244,211,147,136,159,205,30,116,121,170,116,129,95,223,23,77,203,46,144,82,61,179,39,248,69,137},
  {154,85,85,105,211,163,45,61,71,233,173,140,215,243,163,106,48,3,60,7,141,5,155,230,215,206,168,91,232,5,1,4,16,242,16,96,144,54,227,46,118,202,31,24,159,192,131,32,139,72,170,229,167,187,253,41,251,50,21,160,122,214,65,227},
  {42,33,41,87,29,24,162,76,73,2,83,172,51,169,55,101,234,186,137,163,174,23,251,67,57,14,145,70,176,51,70,170,222,45,198,126,186,112,26,29,191,135,162,247,234,31,58,210,204,189,132,156,58,155,48,54,155,13,137,109,22,199,106,66},
  {247,136,49,153,189,83,10,217,251,127,245,109,26,46,11,176,63,224,12,35,161,182,36,173,112,59,33,132,180,102,119,163,25,113,100,151,44,113,101,75,110,104,82,54,233,254,100,171,185,251,84,252,5,132,147,104,205,158,36,219,74,123,145,21},
  {19,36,190,25,114,215,98,134,222,168,25,114,83,145,96,235,91,71,182,120,241,89,162,208,150,190,203,217,88,202,180,60,21,252,82,236,49,84,143,147,86,36,162,52,136,59,194,178,216,99,248,32,168,123,81,51,229,44,94,15,17,129,111,2},
  {116,151,250,118,177,206,21,141,3,105,144,214,63,102,14,250,229,198,110,138,120,9,190,187,197,149,38,78,138,152,184,15,21,120,190,85,155,232,133,183,191,109,66,92,134,201,199,39,68,11,113,76,25,85,187,59,100,82,244,186,61,70,144,110},
  {107,2,249,219,140,92,102,202,46,194,113,169,205,140,151,0,209,19,233,139,214,42,249,55,22,171,155,148,72,189,99,250,232,12,114,79,92,165,232,7,59,32,209,101,4,95,160,79,204,54,182,1,5,119,214,108,128,156,232,69,237,146,12,241},
  {158,99,162,55,241,2,75,213,95,62,244,209,51,229,83,7,87,93,67,144,125,206,162,81,74,155,157,114,224,50,198,207,47,206,22,122,186,138,88,128,142,138,26,152,148,2,29,171,83,108,184,56,186,120,205,105,149,39,151,208,160,162,107,69},
  {193,177,241,237,126,49,37,33,237,48,114,223,39,11,29,227,182,131,221,141,139,162,29,186,31,1,56,216,134,59,201,119,89,137,216,108,165,135,237,174,154,14,127,250,131,172,164,111,63,23,1,237,66,82,9,105,39,134,115,103,252,137,180,199},
  {184,114,80,10,35,136,114,124,58,192,72,100,242,16,121,218,140,92,253,16,139,36,211,74,35,27,46,34,204,100,101,10,240,91,50,91,125,103,155,71,145,171,122,14,77,183,41,48,36,33,211,241,186,57,47,2,92,103,20,248,85,57,84,32},
  {244,10,229,211,252,107,78,240,231,180,218,76,80,54,53,12,254,201,210,161,240,221,201,95,5,191,179,76,76,102,164,56,60,4,216,238,157,230,239,102,13,104,214,92,81,175,211,98,209,28,33,110,106,166,248,210,249,115,96,152,113,253,95,15},
  {24,99,151,252,42,59,152,60,26,237,225,70,225,126,142,145,101,68,179,222,117,225,113,152,43,116,205,191,223,47,41,214,49,91,83,200,100,197,232,119,236,180,159,211,192,221,68,235,102,194,200,109,165,240,86,140,7,228,81,216,116,249,248,223},
  {14,97,227,130,153,248,7,102,216,47,118,132,11,91,226,43,105,244,164,113,226,92,23,216,176,127,239,202,40,83,43,186,59,105,148,114,175,162,130,17,12,214,78,214,218,41,118,67,130,158,184,166,100,184,15,93,64,25,232,111,158,154,46,92},
  {195,107,215,90,182,12,98,121,242,164,61,56,35,249,163,115,1,235,1,250,255,111,96,109,166,141,112,133,56,15,238,62,168,127,101,253,104,26,155,232,151,46,169,114,148,72,5,82,229,0,30,150,224,89,201,14,108,118,222,146,208,152,40,220},
  {200,96,110,104,129,251,200,226,242,179,128,100,82,32,223,4,107,208,161,4,113,249,149,130,149,18,24,168,115,129,68,27,213,126,16,187,250,249,131,233,84,103,240,249,86,241,144,205,148,60,62,188,46,132,44,198,250,16,9,159,104,109,75,162},
  {193,159,235,1,222,80,8,184,5,128,208,161,28,100,77,189,87,16,99,63,44,221,28,134,242,14,35,66,119,212,231,148,120,47,210,71,145,13,200,242,192,126,33,238,169,233,0,192,161,49,75,229,219,100,220,90,63,206,61,218,44,98,27,199},
  {209,9,193,4,25,223,169,185,117,168,255,229,240,209,237,2,249,190,116,134,32,200,105,153,79,166,218,127,124,218,99,115,72,28,9,188,72,205,118,184,123,252,66,172,186,35,8,81,160,253,38,25,29,233,232,243,95,147,90,19,89,221,152,162},
  {81,146,157,56,150,177,73,108,252,13,230,157,197,135,33,88,149,49,228,195,42,160,0,227,82,226,130,203,245,167,182,42,178,190,116,255,209,53,11,196,244,171,88,211,35,48,159,6,170,195,142,43,69,0,233,227,2,144,69,10,67,43,98,208},
  {36,56,24,86,54,151,71,174,155,73,66,71,234,233,161,52,93,48,34,80,17,82,131,16,46,248,249,172,84,4,82,78,12,54,192,44,187,186,251,148,42,94,163,126,165,24,194,201,2,214,12,43,169,71,240,129,112,72,167,240,158,172,39,168},
  {99,178,22,182,239,234,125,35,159,66,171,186,209,123,118,224,11,69,254,146,107,161,56,136,71,75,17,219,44,126,113,84,220,7,99,191,3,101,201,193,193,6,212,177,205,203,254,216,210,35,105,209,45,49,207,62,95,44,107,113,194,79,20,225},
  {100,34,38,128,128,148,254,65,214,132,178,41,238,240,90,10,17,143,198,197,127,206,215,12,123,206,208,30,14,246,99,19,139,203,193,68,22,254,140,202,15,216,91,204,191,170,12,14,46,97,122,125,22,76,58,193,107,144,150,183,171,79,182,223},
  {123,72,158,227,26,114,128,145,114,68,47,169,122,114,45,115,87,210,162,128,220,156,134,207,8,103,188,98,239,180,254,130,196,161,55,120,29,135,185,193,225,234,124,3,228,184,67,6,45,223,97,196,103,184,255,55,226,251,195,5,13,174,71,84},
  {148,247,18,61,196,62,30,231,88,78,138,216,177,42,241,63,36,190,179,75,129,243,6,37,45,250,164,109,113,2,80,168,4,229,102,60,163,187,60,236,98,148,209,31,111,132,149,132,98,55,213,26,196,250,93,204,246,208,5,130,119,150,237,197},
  {148,90,102,253,3,231,219,57,125,134,201,70,150,175,163,102,241,146,106,178,238,63,181,137,67,40,129,51,253,221,199,233,19,221,19,131,184,129,26,59,38,254,200,9,188,237,193,190,42,174,242,77,47,208,79,121,97,115,65,38,182,138,226,43},
  {55,25,138,113,191,122,65,36,56,90,159,163,98,107,207,121,24,48,142,145,223,185,233,87,56,78,188,238,2,109,63,154,77,199,116,1,13,114,68,196,150,185,202,51,89,115,148,219,37,146,244,48,186,76,173,85,236,119,131,63,54,231,136,137},
  {229,131,107,164,150,112,180,120,32,75,164,171,35,123,110,80,117,178,73,46,51,96,30,168,211,198,50,132,26,115,135,129,184,89,121,248,151,151,143,14,144,214,173,222,21,107,49,172,33,65,126,226,9,74,180,71,119,42,205,75,241,44,96,53},
  {208,89,187,110,197,252,151,175,29,94,45,218,31,104,26,128,77,159,195,43,218,180,225,217,4,135,104,20,211,127,82,188,104,211,47,248,34,95,33,236,210,135,114,82,191,137,29,21,78,89,193,8,24,232,65,184,191,182,226,187,89,74,156,159},
  {74,60,135,0,80,192,192,50,162,223,119,229,210,135,96,124,210,3,163,224,85,110,133,142,78,5,175,47,21,205,161,68,36,95,3,69,167,236,25,15,11,228,96,68,49,127,100,173,171,51,158,53,158,147,195,4,52,71,36,22,15,102,245,229},
  {33,141,131,2,245,139,100,221,114,7,48,126,162,9,43,8,71,145,203,42,25,229,22,188,130,248,90,97,237,96,173,36,137,106,174,68,47,188,161,60,31,103,235,172,182,173,252,187,25,195,118,79,227,33,72,154,241,207,78,167,157,255,124,135},
  {132,255,233,216,109,244,132,116,3,114,150,129,0,115,45,125,4,109,51,48,74,217,61,218,127,65,93,242,100,153,184,159,53,247,158,48,94,155,222,31,105,222,39,214,93,215,30,152,16,254,147,6,235,220,167,252,207,32,234,45,78,214,218,142},
  {41,105,47,26,224,127,209,84,173,217,44,126,201,17,27,227,176,212,218,21,15,1,13,217,204,97,236,50,10,5,177,251,44,77,121,228,54,24,5,52,139,254,233,72,223,171,151,57,160,31,0,240,114,85,15,99,96,84,71,155,129,221,172,196},
  {186,230,120,240,153,228,255,40,111,24,10,201,119,19,195,4,179,96,15,159,140,227,27,158,80,83,68,142,234,17,66,132,144,91,82,156,114,74,80,204,203,129,128,178,174,140,208,126,115,70,26,102,110,215,68,163,112,71,200,95,124,2,35,53},
  {64,3,111,151,32,171,146,43,9,222,61,190,16,220,45,182,70,21,254,219,108,238,98,62,171,197,65,106,177,204,196,23,112,8,217,134,114,100,196,255,212,160,129,98,179,56,57,156,170,105,54,213,211,13,64,249,193,204,111,154,142,31,107,147},
  {117,92,12,66,198,142,243,156,235,73,160,92,74,29,71,247,87,106,69,13,137,237,191,5,116,40,185,67,158,43,97,171,231,143,212,235,34,189,110,14,103,111,181,110,17,100,179,122,236,88,140,8,195,41,86,46,44,64,79,136,177,240,171,232},
  {181,106,32,120,51,1,41,131,131,28,250,13,43,95,134,224,107,41,190,248,24,24,221,9,51,40,26,78,216,99,97,157,26,141,178,222,219,206,48,87,48,171,23,21,170,228,1,123,135,197,36,180,89,58,255,39,197,250,92,153,87,159,174,42},
  {147,83,86,182,164,153,7,102,179,135,151,29,181,229,35,237,37,135,214,186,200,214,38,170,161,48,177,151,214,138,4,77,16,61,196,109,124,37,168,148,196,135,171,183,248,148,207,235,178,249,42,49,201,67,131,251,59,229,250,114,138,218,51,74},
  {163,46,165,194,103,59,155,215,196,39,160,145,73,73,65,54,105,217,191,223,4,177,130,60,33,179,143,227,129,115,82,50,80,114,232,55,114,250,173,196,191,60,103,35,46,37,80,139,184,157,140,102,87,47,109,103,222,36,190,207,103,170,14,62},
  {125,133,43,156,71,142,210,18,147,227,231,103,155,97,182,54,14,217,87,44,22,205,195,103,84,111,163,115,38,179,110,123,65,71,196,248,12,128,175,250,188,211,214,219,50,44,227,25,38,114,33,4,184,192,234,43,200,185,191,59,81,205,194,186},
  {182,217,112,132,144,80,215,11,17,135,111,56,195,9,127,152,242,219,220,8,79,66,69,138,23,144,142,198,170,4,182,204,168,107,213,58,255,102,213,98,79,232,244,234,202,116,55,119,2,109,229,136,94,84,145,68,134,90,219,162,100,161,241,58},
  {77,185,124,132,110,182,163,171,9,84,195,224,241,33,166,104,200,224,124,220,2,247,253,229,90,195,241,39,166,111,133,137,242,191,77,90,113,187,235,206,143,55,212,30,21,239,1,14,72,135,145,26,117,125,110,156,230,234,121,79,149,226,239,171},
  {154,127,88,106,16,99,113,32,174,106,175,6,245,4,4,99,2,17,112,93,69,171,61,104,72,12,204,134,230,100,40,91,202,185,10,220,74,101,181,17,160,86,229,113,147,187,82,37,83,116,10,89,31,251,13,153,120,113,4,255,70,172,141,164},
  {172,255,143,238,161,97,77,184,85,133,183,232,13,188,173,62,199,86,201,254,104,100,0,190,36,73,234,185,109,127,219,108,168,41,215,154,74,226,247,220,204,143,29,5,239,189,107,112,131,31,122,60,201,207,233,90,185,11,94,169,219,143,109,75},
  {221,8,203,31,191,214,102,172,167,210,182,29,237,249,238,113,17,25,179,31,88,185,11,73,156,126,136,35,12,33,78,53,13,6,245,177,142,47,101,115,218,78,116,95,24,179,205,230,151,235,122,13,141,172,222,80,246,16,83,63,67,121,14,254},
  {229,194,241,187,240,85,31,241,8,27,107,112,42,149,42,151,219,112,226,18,75,29,170,89,62,10,193,103,162,22,62,77,174,176,188,253,200,208,45,252,80,228,248,154,93,110,32,136,109,206,120,81,121,123,182,54,234,227,20,185,8,33,94,231},
  {217,14,58,211,26,112,231,63,85,116,31,223,128,20,132,12,6,254,94,119,98,235,63,237,17,243,208,6,12,84,71,2,205,184,206,58,57,54,51,191,45,116,206,86,216,46,230,36,16,85,148,158,218,117,188,30,107,51,44,97,46,191,11,217},
  {80,152,218,3,109,117,203,246,7,252,99,27,24,236,82,18,96,197,225,175,239,50,105,124,160,51,41,226,16,152,79,208,149,67,40,177,146,49,68,201,224,235,131,213,120,75,13,254,155,42,183,146,87,23,28,234,118,245,203,112,150,115,29,60},
  {185,77,77,223,206,157,10,7,169,243,7,4,37,167,233,252,178,151,25,104,174,148,140,23,136,13,4,72,206,131,226,44,165,215,100,196,134,13,153,146,165,167,171,8,128,76,42,49,119,133,245,90,223,84,49,97,220,27,142,208,157,23,251,194},
  {219,140,78,5,84,69,165,203,45,58,107,145,62,194,32,73,189,116,185,110,20,199,99,31,58,66,197,229,75,200,197,133,51,219,54,78,128,93,6,137,77,14,100,66,151,134,27,213,90,198,96,39,242,22,42,105,60,94,117,244,79,199,39,161},
  {28,91,39,252,22,160,118,163,11,72,96,252,116,64,205,238,72,240,240,22,11,153,192,231,2,239,53,153,22,220,62,204,240,160,29,153,159,38,228,198,68,93,76,198,24,101,236,248,78,188,254,32,91,220,229,199,244,102,218,119,129,235,57,137},
  {8,215,69,175,190,91,61,154,122,188,236,45,218,42,35,188,55,59,206,194,92,29,27,241,147,4,140,121,93,96,72,23,67,229,224,219,230,60,46,25,83,233,159,20,124,204,145,179,136,202,51,245,64,167,192,22,154,38,117,172,168,176,210,57},
  {202,107,222,180,137,7,69,200,180,223,7,81,238,34,136,201,102,255,182,134,7,211,86,125,129,28,211,148,95,129,130,150,71,209,35,245,22,222,127,230,30,247,203,16,172,109,220,77,159,21,82,130,183,252,154,90,82,173,0,56,177,146,219,119},
  {18,225,215,0,251,197,98,192,148,176,44,68,37,77,147,79,188,61,22,146,140,204,205,95,240,3,105,182,83,84,13,206,185,123,112,245,253,240,35,197,155,190,163,197,206,193,131,44,95,34,255,100,122,58,49,171,31,125,147,252,246,218,178,147},
  {201,21,201,17,52,212,38,10,80,238,68,212,119,216,232,160,210,104,42,253,106,146,9,88,32,160,158,31,20,188,219,182,234,248,46,122,182,168,15,131,53,235,93,37,45,153,220,116,100,158,219,241,173,119,107,56,88,119,138,220,121,57,125,79},
  {63,227,207,135,56,61,10,92,82,205,66,150,195,234,161,81,162,85,218,182,11,108,144,35,109,57,149,115,58,207,172,254,138,212,231,103,182,143,6,242,123,120,24,229,87,76,167,155,164,139,187,71,195,9,5,101,228,204,87,22,154,16,54,177},
  {62,130,84,46,185,146,127,149,43,141,48,244,117,214,138,57,135,203,144,143,94,209,36,20,74,86,231,91,173,66,241,61,210,244,108,105,88,64,122,99,134,100,4,95,145,136,161,125,33,27,26,97,161,64,197,221,179,89,70,47,8,240,87,9},
  {131,82,74,109,156,216,73,104,21,80,164,21,153,27,202,143,174,42,126,131,9,141,217,137,171,0,152,112,129,50,131,16,245,163,226,243,52,54,254,196,82,132,209,45,219,75,21,180,6,90,82,50,215,82,182,139,148,29,205,6,145,73,225,7},
  {31,189,85,46,120,235,185,72,86,83,2,215,60,148,244,67,107,25,76,57,202,254,11,161,74,42,255,169,144,160,54,4,211,1,215,88,162,156,178,237,119,118,189,215,55,225,90,162,87,208,220,144,204,80,149,117,241,15,10,156,237,9,189,95},
  {22,233,36,77,152,106,18,224,247,125,69,231,202,47,119,217,150,135,56,107,123,4,174,152,140,30,253,77,231,132,105,60,153,33,183,79,194,42,38,161,60,132,35,58,5,86,33,209,26,6,56,94,250,147,122,235,254,107,219,253,149,199,223,171},
  {217,205,183,237,25,111,74,88,56,26,133,189,95,232,97,113,181,7,143,135,12,24,91,202,201,20,3,151,164,169,240,244,16,241,203,83,68,68,233,172,42,155,91,17,209,163,186,226,120,93,0,78,127,48,119,65,91,35,158,246,42,25,166,253},
  {70,8,121,86,214,95,183,13,153,166,125,182,114,171,19,193,183,197,254,10,250,30,11,136,24,87,161,150,41,229,128,235,0,81,246,60,155,8,159,20,167,67,169,116,5,159,168,8,95,78,159,24,213,188,233,219,154,230,86,221,148,186,250,8},
  {80,52,175,207,226,66,13,172,73,22,94,170,255,85,169,173,30,24,71,217,175,44,235,120,86,118,114,244,226,83,20,57,66,54,73,222,143,185,53,148,239,249,254,134,56,192,199,213,240,58,192,222,160,150,136,108,212,5,196,23,97,125,214,137},
  {34,28,32,188,19,79,105,28,217,120,58,105,155,5,240,84,212,112,53,96,181,196,75,226,194,255,39,52,228,133,93,15,248,125,74,175,65,110,142,200,240,85,203,228,180,175,14,241,111,51,170,182,38,79,199,122,159,197,215,128,222,203,173,33},
  {196,30,2,236,127,231,171,72,102,24,135,192,165,170,12,166,204,36,198,108,63,144,18,82,69,6,138,133,44,235,18,208,22,44,99,173,218,140,152,55,58,37,152,44,209,123,127,131,251,112,55,207,124,155,176,92,143,119,26,216,46,126,46,54},
  {168,126,130,120,92,49,253,62,221,48,72,21,60,167,54,196,57,7,79,167,208,175,156,24,174,178,119,212,168,242,112,57,7,235,20,34,19,227,213,81,48,203,64,68,240,212,189,49,219,226,149,140,165,28,65,199,164,251,223,206,28,32,92,143},
  {32,237,46,139,216,17,186,119,137,188,111,30,163,114,226,156,254,153,239,143,41,207,153,238,164,36,2,181,143,158,85,221,35,16,234,159,168,85,19,209,22,179,139,212,109,44,109,147,212,24,24,50,12,75,225,83,204,169,96,65,136,48,218,118},
  {40,218,149,81,94,146,168,65,8,150,28,186,227,251,74,223,107,69,146,38,73,140,219,28,123,123,1,99,159,13,238,223,76,207,86,54,169,68,1,60,248,46,83,162,220,123,16,128,128,64,135,132,25,101,247,241,63,134,234,24,53,35,136,28},
  {100,70,73,202,65,63,221,64,142,253,30,183,0,213,125,11,230,49,141,197,253,168,167,64,45,16,171,246,235,202,89,176,109,91,178,1,197,94,3,225,36,7,224,202,116,120,225,226,192,121,201,200,144,232,117,207,27,11,52,172,181,18,151,85},
  {110,247,15,160,173,143,29,98,33,158,214,211,230,168,51,179,145,82,220,48,245,5,198,43,47,201,75,89,228,209,17,151,241,122,221,103,29,248,111,175,79,217,162,187,15,167,147,226,223,86,177,86,7,173,206,35,128,178,93,205,187,4,85,40},
  {35,193,250,135,44,212,74,90,178,24,47,120,195,50,130,58,229,154,2,218,140,49,213,66,67,234,54,142,204,49,56,108,25,51,207,130,229,86,34,57,207,89,195,93,220,53,16,181,100,230,242,57,198,36,184,105,66,205,149,116,122,15,86,176},
  {252,25,214,248,57,106,26,57,18,120,222,88,72,128,132,203,29,69,199,252,48,93,125,1,226,150,251,33,50,255,234,35,74,147,172,144,12,110,118,181,157,163,182,254,217,106,13,224,164,143,82,231,240,9,150,120,29,31,145,112,206,59,232,74},
  {57,174,130,202,195,172,162,247,173,6,57,143,182,197,132,91,223,68,7,202,151,153,250,41,88,173,42,138,54,202,153,211,63,246,223,21,179,154,98,63,80,108,19,237,3,65,249,81,165,42,229,63,186,211,221,213,144,94,164,101,52,81,198,248},
  {57,43,110,221,229,5,165,60,106,201,77,88,176,136,76,239,133,43,87,58,151,113,79,102,226,110,140,151,150,137,202,175,67,74,235,68,53,104,66,75,105,252,96,247,81,210,1,223,164,52,140,13,225,162,49,161,104,73,126,44,39,55,180,184},
  {145,240,119,47,131,242,187,0,64,208,92,64,100,202,11,239,173,98,30,150,208,137,115,168,117,73,229,69,36,178,252,67,140,197,81,52,223,122,155,215,201,71,77,239,177,66,142,27,119,38,50,164,62,63,185,32,8,26,235,132,52,242,176,6},
  {98,91,204,191,253,254,221,229,8,163,12,51,234,1,55,251,20,245,103,148,119,39,58,170,107,90,148,161,130,109,101,197,48,109,231,228,52,215,129,98,42,74,224,200,154,108,220,84,68,176,41,64,10,16,235,190,31,20,187,81,85,17,147,160},
  {174,103,43,172,245,59,42,159,77,22,126,7,135,239,60,159,232,253,153,233,13,246,80,195,227,202,10,117,182,87,70,27,61,71,242,34,107,44,27,115,107,136,18,51,180,126,116,247,143,226,97,250,165,6,100,229,35,26,5,74,201,89,179,163},
  {45,209,204,223,89,219,38,253,214,42,25,169,12,132,117,18,33,16,208,207,25,208,254,178,24,28,124,111,56,208,174,181,233,231,1,70,30,147,131,105,128,87,105,109,183,14,13,107,192,61,124,183,193,113,88,188,165,49,165,32,143,108,62,228},
  {231,156,170,159,6,15,151,219,172,83,236,248,154,9,17,84,215,157,163,110,158,125,184,79,76,45,179,87,35,181,76,200,153,224,184,50,241,178,66,117,138,203,119,219,135,57,121,49,232,67,145,171,225,71,166,239,238,42,107,197,154,90,173,236},
  {50,247,253,97,73,246,90,67,42,143,253,50,38,9,230,53,186,232,66,247,94,143,45,168,207,37,4,228,119,244,115,12,238,165,59,212,215,125,9,184,64,144,19,122,151,36,172,160,129,76,53,223,121,31,78,234,176,100,182,26,134,211,242,154},
  {30,101,15,157,189,172,165,131,240,34,215,194,23,151,0,228,250,214,203,172,245,110,23,66,130,171,80,80,17,46,113,145,157,148,51,135,124,34,189,88,10,197,201,138,33,47,32,165,44,91,6,63,68,103,40,69,104,82,53,109,142,9,50,142},
  {208,55,1,142,177,235,117,146,32,1,150,1,42,245,168,187,122,149,231,1,100,59,219,132,208,25,115,168,77,66,168,135,122,123,100,86,182,153,24,208,130,205,229,210,171,213,103,99,116,89,14,237,182,89,68,34,94,119,157,158,167,227,146,7},
  {174,19,225,239,99,235,171,67,252,203,111,89,70,191,78,160,67,207,89,45,97,65,157,181,192,71,30,17,157,88,250,157,72,233,119,250,17,155,185,245,6,29,195,231,179,75,99,105,120,180,238,22,108,23,209,64,68,191,196,202,222,102,113,91},
  {223,49,126,77,20,195,177,169,138,58,93,60,156,173,246,233,210,113,14,106,67,65,140,135,8,90,113,120,71,78,34,183,245,238,77,36,212,207,135,141,84,181,82,66,214,76,198,248,174,127,112,28,75,105,80,150,56,158,105,196,41,92,26,38},
  {16,229,152,52,91,209,227,123,46,97,82,249,78,206,57,87,41,214,101,205,226,127,54,24,235,212,228,60,244,83,88,234,159,252,118,223,17,234,9,2,161,13,8,169,17,46,229,242,145,175,90,30,26,125,209,30,65,127,25,89,58,121,55,174},
  {21,231,105,231,44,71,76,74,203,41,174,117,239,113,119,38,152,16,98,124,223,129,88,212,30,25,45,96,14,107,224,80,22,170,235,84,18,247,111,83,218,61,155,108,29,234,51,234,241,183,145,134,199,148,53,82,192,172,153,114,30,6,148,76},
  {148,2,167,106,31,108,121,135,161,188,115,241,153,172,42,97,175,5,48,2,14,186,111,167,215,20,4,168,179,193,8,68,189,12,52,139,205,215,253,164,41,234,118,169,183,62,120,117,2,54,189,83,88,230,43,250,157,170,172,75,128,19,225,134},
  {23,235,7,102,40,138,93,190,228,0,160,65,16,41,89,206,240,6,32,182,151,228,184,3,84,222,243,46,137,238,116,31,178,33,32,23,167,25,46,128,40,149,188,218,205,142,131,142,127,55,55,162,174,155,179,198,195,188,173,179,130,111,91,229},
  {189,128,246,208,219,162,86,50,58,105,113,118,28,126,174,187,81,99,143,98,250,31,26,77,200,23,196,8,39,110,79,171,168,79,93,108,199,246,186,213,81,156,243,35,231,15,180,80,234,113,190,248,218,112,200,196,196,235,231,213,113,91,107,131},
  {216,6,153,30,119,43,161,143,94,58,213,122,144,21,151,19,4,44,53,14,85,183,83,79,109,61,243,213,28,172,137,215,193,2,233,184,170,220,117,4,244,190,250,31,51,111,87,177,114,42,214,113,118,233,43,85,244,240,114,168,109,85,31,168},
  {31,105,14,223,25,63,94,242,23,113,182,211,84,176,99,225,13,233,177,228,149,57,187,47,37,92,13,148,39,223,160,127,20,45,128,210,91,79,144,17,28,151,37,78,221,193,78,89,148,190,222,32,209,187,210,216,32,246,200,72,245,135,82,242},
  {118,153,217,101,129,47,177,126,188,230,81,218,164,92,44,14,195,41,25,86,148,100,240,82,110,248,52,153,142,137,134,189,80,81,135,189,238,82,172,91,114,183,37,205,3,172,97,248,202,83,44,46,103,87,110,40,40,62,75,176,254,225,89,222},
  {64,236,220,69,109,44,49,144,204,200,200,199,235,251,164,226,249,127,149,49,40,154,206,80,155,116,18,46,71,32,57,154,226,140,228,12,9,103,96,21,72,31,131,227,186,72,107,236,12,246,153,155,158,172,74,5,38,140,238,227,139,90,212,9},
  {32,4,111,207,85,246,33,102,204,70,209,221,97,120,208,86,232,185,48,34,195,39,26,175,103,31,143,20,91,122,41,11,59,184,191,206,96,246,86,73,149,30,49,83,131,132,188,124,156,207,216,43,246,117,200,16,201,88,144,134,177,185,46,158},
  {82,46,166,167,200,0,30,35,158,165,198,238,150,104,23,94,99,48,179,164,77,240,111,212,123,234,130,242,49,104,7,149,165,248,79,102,204,252,84,245,116,24,132,193,65,242,46,166,245,250,85,16,13,163,69,69,251,148,127,16,246,87,138,52},
  {245,232,216,44,174,188,125,199,173,57,11,111,151,62,201,81,205,223,144,222,94,222,15,93,232,144,125,0,32,56,219,222,112,216,220,250,160,134,102,53,154,189,36,219,124,117,185,36,207,101,14,94,98,220,255,38,81,148,34,132,248,85,125,128},
  {67,47,45,242,30,144,209,112,1,187,205,86,147,211,120,233,198,113,232,115,148,89,195,176,225,118,223,224,217,35,55,249,115,108,134,237,66,90,149,171,118,11,148,112,125,169,59,79,147,187,64,51,52,64,127,224,184,185,122,155,228,40,164,223},
  {200,211,214,156,145,122,173,119,182,126,153,4,40,74,164,53,177,7,66,35,92,168,88,106,159,134,228,151,134,71,153,182,186,88,138,179,93,199,8,98,119,7,106,211,37,203,207,86,199,19,79,0,88,106,105,129,108,14,79,238,248,5,80,80},
  {56,173,245,95,27,126,171,66,235,98,248,209,43,65,104,112,157,175,85,51,13,148,253,199,70,215,129,7,246,107,181,56,165,165,5,159,249,74,29,113,186,89,236,255,9,117,207,104,154,223,187,153,107,46,218,219,195,120,35,22,254,247,134,99},
  {189,181,92,6,239,201,182,204,32,105,23,226,16,116,125,76,98,61,8,66,187,236,63,102,158,143,109,160,178,182,103,160,135,179,2,130,66,182,17,162,203,77,124,210,89,56,131,202,235,234,109,179,143,176,35,243,54,29,24,139,201,18,237,88},
  {47,109,230,83,88,70,102,154,204,198,0,84,63,148,162,108,9,89,191,157,97,181,14,200,117,186,233,213,157,65,227,78,110,11,177,52,29,2,75,110,138,46,225,145,75,133,47,209,242,117,255,255,252,188,228,133,4,222,178,164,86,36,245,198},
  {129,248,69,248,183,248,157,253,233,226,209,179,189,228,232,209,217,64,45,47,74,13,85,106,173,132,33,236,212,215,45,180,167,215,181,179,217,185,19,193,170,13,141,244,133,23,21,71,241,249,66,148,36,121,145,218,247,151,181,219,79,156,23,40},
  {44,123,136,223,244,234,123,211,24,156,210,126,139,9,183,115,176,190,238,35,83,96,2,121,91,161,77,178,178,236,30,91,27,15,60,157,45,80,101,11,50,156,209,165,20,52,105,81,240,42,76,124,123,199,97,168,4,71,117,249,227,34,132,159},
  {129,40,162,240,217,149,138,189,149,115,86,75,224,32,173,199,101,252,194,198,87,87,192,145,176,119,125,31,227,189,168,101,193,177,49,70,121,74,159,99,243,142,201,51,76,184,70,17,223,4,145,136,19,173,255,108,83,42,255,32,107,138,8,193},
  {134,188,248,120,169,48,214,35,17,119,219,34,27,50,165,117,122,172,5,49,174,46,233,69,206,193,80,111,42,254,209,15,253,226,36,248,212,21,209,87,114,201,240,89,141,236,15,144,59,96,26,225,24,206,218,208,240,135,96,36,5,227,22,233},
  {101,27,214,108,4,187,138,220,26,139,245,139,219,119,132,37,220,220,230,161,190,141,105,50,213,227,128,165,194,65,215,211,228,159,51,70,194,236,131,100,21,149,22,88,162,103,48,114,59,10,214,67,180,167,35,240,33,119,236,127,244,78,159,28},
  {35,38,75,24,211,130,48,144,88,83,131,116,15,172,178,196,43,69,5,35,132,4,130,173,229,212,169,129,156,237,44,244,190,167,232,241,227,6,43,105,9,110,95,116,27,124,217,76,48,217,210,197,191,27,64,187,38,54,172,60,56,69,126,178},
  {49,61,210,7,14,203,122,127,119,177,137,151,91,75,127,60,157,14,156,209,185,246,113,25,18,163,172,236,163,181,21,243,72,100,164,133,98,118,95,161,6,86,171,68,240,92,116,195,251,51,83,65,121,131,87,235,244,187,72,241,145,166,51,246},
  {19,162,2,69,139,248,156,7,129,189,185,25,222,6,87,162,160,117,154,50,1,194,226,5,191,216,13,43,75,10,32,250,93,255,36,162,156,254,184,204,142,93,212,87,170,194,27,223,99,127,115,99,61,90,18,68,46,105,65,65,49,23,73,158},
  {124,217,192,174,189,193,52,54,60,134,142,225,134,255,36,169,138,206,7,203,6,164,252,59,111,166,254,82,222,195,79,137,65,38,187,207,126,224,170,85,204,159,88,119,101,187,127,230,64,183,229,223,244,6,56,221,81,129,236,151,242,144,103,42},
  {33,129,143,2,249,251,225,11,174,252,165,33,192,176,15,171,36,73,213,173,22,80,10,168,119,84,118,124,22,77,97,228,136,42,21,21,100,104,11,72,51,145,146,177,90,187,220,126,127,8,246,191,61,131,154,76,117,206,62,59,119,27,95,13},
  {169,191,221,149,202,54,122,116,198,40,77,146,201,86,54,122,88,193,253,112,198,116,199,49,150,1,252,63,165,195,227,187,45,84,216,250,101,72,136,138,38,108,205,124,27,96,31,125,224,229,163,216,175,27,147,83,229,46,182,69,82,135,111,118},
  {75,158,212,141,137,190,224,221,12,153,133,233,245,166,234,98,184,40,75,43,52,19,235,80,127,84,180,190,77,141,112,153,213,157,66,234,105,110,240,0,95,7,229,90,19,8,20,7,202,247,235,10,219,190,149,14,36,243,171,213,117,190,50,78},
  {189,66,146,82,5,114,244,103,117,13,48,75,158,218,203,94,17,174,97,122,100,121,211,83,177,65,68,21,96,191,213,17,92,166,156,131,214,155,70,220,221,119,166,204,66,156,3,159,86,136,182,31,74,119,173,88,124,163,152,125,145,116,38,150},
  {72,63,98,38,139,127,159,104,215,84,142,36,127,95,2,1,118,40,65,37,20,124,68,211,100,163,188,222,107,215,155,201,113,95,171,130,39,56,193,5,172,21,74,105,106,223,0,109,68,233,89,189,177,207,63,97,41,221,79,6,227,244,245,33},
  {207,225,224,157,112,58,16,87,103,236,241,29,43,244,87,185,87,34,228,55,86,60,121,242,251,74,10,133,243,121,125,247,29,14,201,51,124,3,134,118,99,12,81,122,2,54,231,200,255,97,45,187,154,248,44,146,86,176,21,148,14,49,167,87},
  {21,216,177,199,208,3,237,231,3,126,123,31,99,44,123,141,15,177,53,254,16,86,44,118,192,132,202,171,68,124,145,6,7,49,39,28,58,142,158,91,233,47,105,13,56,134,225,82,160,110,240,239,246,91,50,123,16,165,224,27,71,18,105,113},
  {154,175,84,83,42,230,253,145,200,52,105,36,90,27,157,80,11,58,52,223,50,75,229,112,132,115,226,5,186,48,195,17,28,246,224,192,217,111,163,77,223,122,173,2,202,57,173,150,11,31,255,251,107,67,10,81,230,38,254,236,109,240,52,140},
  {49,244,52,80,227,54,170,35,197,1,233,41,136,85,162,107,220,66,197,94,125,69,2,81,177,20,213,234,132,176,40,128,151,224,57,205,16,5,10,207,116,187,222,170,85,112,172,154,133,219,197,27,18,209,241,126,165,22,53,86,59,124,253,127},
  {120,124,213,155,64,247,233,28,34,0,236,124,71,120,181,162,221,17,21,147,26,63,95,130,214,251,206,250,248,120,84,179,61,60,114,197,47,61,83,196,68,201,36,139,96,23,46,140,229,175,118,185,14,248,81,137,176,34,223,109,52,39,1,45},
  {247,60,8,85,13,168,100,104,208,78,233,253,181,241,126,39,88,192,183,247,20,206,92,27,181,225,173,197,207,60,225,188,36,121,246,132,242,171,68,137,74,188,232,3,236,199,223,129,9,66,239,179,130,214,81,80,189,11,159,14,19,71,28,102},
  {173,2,26,76,99,98,143,145,20,160,224,76,67,34,238,93,156,132,46,85,86,209,216,131,24,158,17,234,107,146,7,160,169,127,23,125,94,190,194,15,233,243,211,240,4,54,255,178,1,111,60,124,224,198,62,215,246,246,91,75,186,3,40,116},
  {181,230,0,6,52,90,138,97,30,36,17,66,143,124,185,40,91,122,144,32,83,130,70,247,95,125,210,63,85,144,93,94,41,15,84,112,190,32,201,11,90,33,38,20,83,12,42,209,15,176,27,15,0,145,236,140,175,55,140,83,26,240,222,185},
  {23,155,225,234,242,251,40,70,86,15,1,144,6,248,79,239,77,228,153,113,105,42,141,9,96,172,148,172,92,61,106,11,213,155,162,158,236,26,34,223,128,39,195,130,9,212,51,184,81,187,111,21,180,168,128,34,5,254,234,143,66,19,156,58},
  {152,111,109,47,40,127,35,30,131,10,52,91,29,219,13,158,44,102,85,89,178,216,212,69,226,230,182,178,89,90,37,251,12,144,12,50,94,154,197,173,236,183,3,48,58,140,78,109,186,195,125,197,189,135,125,243,40,41,83,15,186,55,218,196},
  {116,171,220,41,71,93,211,173,236,54,245,165,118,169,19,29,41,82,28,18,128,109,222,48,77,216,177,123,213,176,177,65,146,230,85,115,58,95,180,5,63,133,226,125,224,219,206,220,171,170,255,23,109,87,168,13,224,101,141,40,89,202,135,74},
  {31,165,231,132,226,216,117,89,158,21,111,163,122,68,229,19,45,121,69,116,187,238,153,214,62,43,16,19,115,177,176,19,109,197,89,42,165,73,6,240,205,204,152,137,190,197,99,78,196,197,32,75,149,147,115,11,10,84,182,116,71,95,149,163},
  {93,4,129,147,199,144,87,228,143,218,222,104,250,231,147,88,60,200,82,14,65,57,232,160,77,209,36,86,218,164,12,90,106,52,80,4,223,251,245,186,110,24,150,26,87,147,44,184,247,73,247,109,199,39,50,128,21,33,91,90,170,194,152,66},
  {55,117,164,78,252,232,15,132,117,2,12,106,207,128,62,101,231,116,105,24,45,44,187,245,111,65,230,78,133,235,207,77,74,208,107,149,117,162,114,54,152,173,135,248,26,38,72,8,80,239,38,168,105,181,81,120,134,230,113,79,249,128,187,30},
  {116,134,97,169,66,106,96,56,232,85,195,216,46,214,27,156,136,4,84,62,4,92,15,166,254,33,128,38,117,119,110,21,18,76,177,97,85,144,159,87,38,134,252,5,188,128,142,104,239,134,110,182,208,125,24,161,70,27,246,154,92,76,1,129},
  {108,101,49,157,164,169,12,88,137,28,178,88,234,220,23,144,52,193,94,120,185,171,221,105,253,232,67,110,141,101,171,170,49,127,140,120,133,63,166,154,140,24,210,116,69,58,67,254,148,203,99,196,212,189,213,249,255,208,115,247,53,133,170,58},
  {160,156,230,140,208,124,253,30,145,232,143,76,168,28,99,207,180,221,55,93,175,71,194,175,181,130,147,124,25,54,157,128,9,95,0,186,170,126,120,135,211,212,41,52,230,87,39,93,34,241,79,192,187,66,74,133,197,215,235,4,14,176,91,52},
  {14,7,215,232,122,60,209,240,119,149,95,185,71,34,29,103,114,150,160,49,179,190,49,146,64,76,161,144,197,6,53,160,185,25,221,46,8,47,209,70,83,158,224,34,248,201,200,253,121,97,61,40,214,7,79,232,173,222,198,33,236,99,59,239},
  {243,85,150,0,71,155,114,72,65,230,62,199,161,253,217,37,97,127,92,176,105,24,232,82,69,209,234,38,254,74,98,156,222,241,47,69,60,223,111,170,134,154,100,71,99,211,237,46,128,239,243,149,242,167,7,2,5,255,247,136,182,182,131,113},
  {167,136,179,133,24,29,48,11,41,59,249,73,213,30,93,160,198,94,54,164,174,130,96,199,241,221,205,155,218,153,252,168,209,123,112,66,176,104,202,91,184,70,194,61,170,119,255,96,211,186,189,115,247,0,185,13,26,82,181,39,129,144,89,18},
  {239,149,210,115,125,206,136,50,210,170,17,201,206,70,125,86,2,60,17,164,151,184,47,18,49,108,254,19,81,128,139,255,51,187,18,139,210,31,98,114,203,11,54,165,159,42,68,141,204,103,71,8,201,179,120,95,109,254,163,166,17,196,171,120},
  {44,183,123,252,220,112,230,38,97,156,179,226,109,106,97,129,166,186,7,23,235,103,36,114,135,184,23,23,44,83,151,85,14,121,52,122,90,27,35,145,228,60,66,110,250,48,218,205,205,150,23,231,204,87,47,165,27,245,247,27,66,128,204,155},
  {53,37,62,47,148,57,194,227,7,15,47,52,109,201,230,184,173,64,249,234,71,93,229,54,108,63,151,3,27,64,198,33,106,113,13,8,8,43,200,120,11,107,79,170,182,164,193,202,78,78,250,25,223,245,138,245,22,101,96,13,114,245,53,118},
  {135,171,221,169,164,163,154,89,163,251,160,219,4,113,192,175,150,219,84,116,107,217,153,25,119,16,66,148,40,240,103,128,58,255,214,156,217,207,194,19,162,57,230,0,49,223,217,108,15,113,135,168,14,73,124,228,119,129,90,145,22,149,242,207},
  {201,72,150,7,241,51,16,117,198,44,36,204,86,54,20,14,44,142,95,204,148,46,198,107,36,127,87,94,42,171,60,125,52,175,3,110,167,9,121,239,90,229,246,79,207,133,18,130,200,33,53,154,230,226,17,158,218,242,155,145,123,102,166,146},
  {59,24,87,186,197,111,43,119,184,236,82,245,124,70,20,49,192,252,241,98,91,156,234,107,72,253,127,55,40,67,175,195,37,152,90,120,94,66,14,11,123,117,89,38,211,91,65,21,68,15,46,252,87,83,207,196,146,19,107,175,175,208,205,206},
  {16,17,176,101,163,50,211,155,10,105,124,200,74,228,159,69,67,205,130,26,37,2,175,114,146,40,115,44,204,73,192,207,201,238,193,153,117,42,136,52,129,118,6,57,225,155,166,110,181,105,204,169,163,141,14,12,148,187,180,23,212,247,248,204},
  {71,70,112,189,61,249,226,103,119,81,55,193,235,102,173,41,178,203,101,252,118,255,186,122,134,233,126,46,145,199,231,87,185,31,80,252,226,127,40,172,117,114,250,165,20,69,67,59,11,71,107,46,146,142,87,103,136,42,62,211,52,136,231,235},
  {92,186,91,229,142,102,241,162,229,102,202,23,227,216,28,169,73,184,253,26,58,96,11,90,101,81,198,214,229,126,102,149,11,55,229,57,31,95,193,137,175,238,106,210,144,143,245,30,223,172,58,171,122,209,147,35,202,220,24,128,106,131,157,134},
  {183,165,249,6,25,14,24,81,209,126,139,87,46,69,203,192,186,25,74,52,101,37,40,51,26,209,20,205,30,185,249,80,225,243,143,106,108,99,171,79,224,165,146,164,245,0,204,243,36,137,151,219,170,228,223,70,250,153,247,73,74,255,104,40},
  {34,6,139,221,152,8,143,132,167,173,123,204,96,50,167,139,147,135,145,236,2,174,125,120,40,130,202,43,173,206,189,64,30,61,20,61,241,232,152,100,37,94,180,74,156,30,83,33,217,176,186,194,47,125,63,210,168,64,235,231,11,246,8,218},
  {136,12,119,186,31,200,77,54,218,46,120,55,79,161,192,25,125,186,34,204,187,161,120,167,15,65,164,224,206,147,188,71,37,173,225,208,174,60,250,69,152,179,137,145,153,0,137,160,56,121,243,230,196,3,57,12,170,230,157,77,68,125,20,175},
  {214,74,66,22,9,188,113,131,5,55,192,208,227,24,249,60,243,115,101,254,133,98,210,15,170,210,26,118,187,244,90,254,183,250,23,178,175,130,35,137,254,166,230,143,201,32,31,155,128,87,200,209,164,121,57,152,3,16,124,157,28,186,91,60},
  {232,217,149,40,251,197,92,97,37,169,175,76,159,222,112,10,18,215,229,39,72,57,116,51,43,126,180,160,69,50,17,160,207,229,63,19,210,78,242,209,186,68,53,48,189,21,60,180,74,128,104,110,121,156,98,192,107,70,21,66,243,13,31,6},
  {91,49,238,2,98,0,236,119,135,218,202,177,9,120,151,217,226,34,16,71,112,168,11,133,240,199,74,14,177,115,211,174,34,50,138,238,103,36,244,136,42,159,151,110,74,81,166,19,143,255,186,49,164,97,249,163,136,61,63,163,156,93,169,62},
  {31,128,135,220,89,29,198,45,153,253,66,171,202,139,22,121,10,232,41,249,252,230,112,128,77,67,235,245,215,46,52,103,28,211,57,124,197,131,93,47,183,76,55,197,155,61,14,251,66,152,33,231,178,232,39,196,106,55,240,156,170,171,170,67},
  {244,136,58,161,45,199,39,133,70,147,33,116,222,252,8,92,26,162,184,199,166,51,227,99,110,145,114,185,122,88,159,78,169,51,149,82,160,203,19,46,3,189,141,224,168,211,99,51,18,97,236,161,57,30,192,36,129,218,152,165,192,186,192,84},
  {221,43,217,181,107,94,231,245,81,139,114,166,95,34,125,236,224,179,241,125,15,65,35,130,26,217,204,138,23,41,75,237,71,148,60,153,116,3,124,61,254,20,142,24,221,105,240,82,164,49,181,240,114,71,99,178,195,64,219,16,189,234,187,49},
  {23,217,10,132,79,32,111,154,130,199,232,12,95,93,251,179,177,164,233,94,145,148,225,94,203,205,252,225,45,252,220,80,98,122,174,146,76,75,217,244,179,70,206,214,65,104,112,71,156,191,241,45,7,251,3,168,188,27,36,215,94,169,1,52},
  {83,73,96,96,81,137,174,175,201,89,8,14,51,142,180,168,191,201,176,253,175,128,63,184,16,100,227,192,140,158,227,59,125,167,178,250,119,204,134,233,182,204,34,10,27,226,170,79,125,254,17,214,246,121,102,218,139,251,39,32,222,199,179,219},
  {130,111,176,230,202,71,25,157,70,66,57,144,48,137,68,121,163,246,229,247,125,71,150,154,119,87,84,191,232,198,150,0,59,245,34,204,103,233,3,85,250,148,100,90,126,247,44,177,109,195,115,55,125,138,242,45,105,92,36,107,190,22,127,198},
  {5,27,94,67,156,137,28,208,42,231,22,27,30,97,150,47,77,79,99,108,37,21,30,234,20,64,41,157,239,48,180,65,182,255,182,45,35,21,35,248,7,115,206,250,189,122,238,67,51,182,90,118,87,54,178,97,196,195,121,235,81,119,101,0},
  {160,8,74,241,171,79,118,180,232,122,192,144,70,116,213,11,168,172,12,64,78,8,63,248,10,237,13,228,34,13,119,178,62,169,102,38,255,186,13,80,197,16,5,45,43,33,8,203,34,74,99,71,194,111,252,188,11,10,142,102,167,119,250,103},
  {213,229,158,73,58,153,203,229,84,106,17,74,193,223,162,36,177,61,223,213,9,87,62,139,101,247,128,53,74,21,142,44,184,7,69,165,235,111,177,93,58,198,15,124,104,140,99,134,188,114,248,219,191,186,229,213,102,203,197,255,13,220,120,225},
  {106,134,48,45,197,11,74,217,81,184,157,238,192,211,93,144,87,193,146,175,14,220,93,86,151,179,133,111,186,75,250,136,199,167,12,0,103,40,207,12,1,11,125,67,13,35,128,148,221,111,11,184,35,38,147,200,157,219,82,76,175,107,36,232},
  {169,152,51,152,200,212,26,135,97,97,231,240,96,74,226,98,106,199,51,182,182,3,8,23,247,207,184,102,232,49,5,226,53,201,116,45,48,77,11,110,102,4,77,232,25,227,143,182,31,21,104,51,215,230,191,101,37,126,167,1,237,182,222,28},
  {33,97,218,5,186,84,79,7,214,55,68,39,107,179,140,49,72,147,97,84,30,98,156,209,205,148,113,111,84,163,235,47,251,20,69,176,112,153,6,58,26,131,54,33,81,69,42,76,25,208,228,71,87,169,25,167,252,51,112,43,103,50,221,18},
  {9,171,48,246,207,159,173,245,48,120,164,54,103,205,201,106,65,178,70,16,127,225,10,154,159,92,211,57,6,82,3,170,118,25,236,146,138,3,125,77,19,164,211,87,203,124,70,34,146,105,195,171,178,167,157,230,249,12,244,63,129,15,200,44},
  {155,209,109,240,170,31,134,225,217,73,124,14,156,209,214,222,81,153,135,153,217,52,68,87,74,223,44,207,92,245,32,242,236,117,3,63,133,13,181,62,63,78,192,158,192,89,253,62,197,110,235,126,71,57,121,255,122,174,73,13,13,241,207,173},
  {239,116,126,127,85,130,165,209,168,210,28,253,119,136,89,76,220,242,152,217,34,208,122,206,172,32,173,214,13,114,128,85,27,112,178,175,119,215,15,194,39,248,192,86,42,107,67,88,208,212,118,27,92,186,220,185,255,245,148,30,62,249,207,161},
  {35,238,83,97,210,209,186,236,62,39,173,168,237,32,31,43,157,190,46,124,96,142,224,46,137,115,193,149,144,239,181,42,125,8,149,251,7,120,30,31,218,51,43,111,171,171,236,78,82,145,147,131,125,122,245,45,132,23,231,171,114,34,171,120},
  {123,245,145,41,166,71,131,252,165,84,241,5,1,20,116,95,57,247,71,88,87,62,16,201,67,46,253,19,98,218,135,128,10,219,52,102,197,78,102,147,182,14,243,200,193,23,110,24,215,168,100,37,114,206,249,117,80,251,185,147,148,147,110,148},
  {111,163,82,17,146,181,41,3,243,151,31,229,198,92,141,153,133,215,17,130,100,242,12,218,248,42,90,31,112,185,49,173,186,141,104,161,115,218,222,168,48,169,238,88,44,228,207,43,30,105,67,210,208,142,192,190,64,177,136,72,111,124,174,112},
  {189,2,145,5,203,165,188,69,95,163,13,167,3,60,65,144,184,206,159,13,31,171,252,156,16,194,103,198,79,244,255,142,250,187,6,26,54,43,41,144,183,182,185,103,34,227,181,211,157,49,192,163,184,133,72,217,255,216,132,13,170,37,102,125},
  {51,255,134,18,23,255,242,126,243,48,113,68,8,16,239,230,209,173,121,125,108,192,170,118,51,231,243,186,175,3,84,154,141,40,25,124,198,82,188,47,164,171,116,142,147,226,199,20,170,126,164,166,173,74,61,18,69,245,61,232,106,121,146,143},
  {155,73,5,159,11,242,211,154,225,185,149,87,66,68,156,148,109,170,73,167,40,162,104,254,63,194,14,116,239,250,169,189,184,211,146,158,10,52,98,98,137,60,62,42,9,243,101,218,102,218,201,27,156,204,218,179,127,56,9,55,28,52,238,213},
  {142,239,232,5,88,221,175,162,123,109,50,203,71,155,38,95,248,253,216,214,91,55,240,68,183,208,194,137,217,3,231,123,162,224,122,109,0,106,255,124,30,11,107,141,217,208,144,53,250,230,93,98,166,212,251,177,125,73,170,161,13,57,146,165},
  {155,164,0,58,39,26,147,254,133,93,92,48,130,207,60,6,233,204,157,168,148,35,227,210,205,46,20,132,89,229,15,7,66,199,72,197,141,217,26,113,89,64,17,220,254,131,60,149,101,138,88,250,37,180,175,99,122,122,9,192,99,25,46,63},
  {40,60,234,41,90,192,188,24,197,154,115,232,49,194,227,145,15,136,148,240,223,253,82,44,232,45,190,231,244,78,202,90,181,185,212,201,120,216,211,129,194,15,156,164,134,166,214,58,114,234,42,62,129,201,213,107,214,12,150,202,2,254,210,45},
  {13,252,126,55,152,99,2,217,145,26,173,220,250,69,153,51,131,184,23,208,68,125,146,181,212,83,225,172,127,156,34,34,27,159,94,110,157,20,25,232,124,17,95,3,246,75,47,124,64,106,49,25,64,12,230,236,100,223,163,86,206,238,97,108},
  {177,144,22,245,236,153,147,168,226,29,183,70,74,246,156,70,126,8,177,69,51,35,134,72,134,85,248,109,24,89,162,204,117,12,171,169,94,67,128,1,122,140,199,97,203,56,90,67,241,144,55,176,77,113,154,118,242,9,99,237,182,148,195,18},
  {249,78,160,157,57,88,101,3,43,82,91,111,125,75,197,87,124,23,126,211,78,15,142,108,65,176,92,91,61,20,189,98,106,153,188,169,164,29,107,185,189,37,213,38,199,152,203,48,122,107,79,47,29,65,249,57,65,39,89,239,80,104,12,203},
  {226,75,177,82,13,243,166,236,91,202,112,253,204,8,177,78,251,176,58,141,204,240,65,246,151,63,181,143,50,178,204,198,57,13,81,24,213,122,63,86,12,16,29,181,239,56,206,253,234,157,185,61,62,71,216,44,150,114,253,249,54,199,60,203},
  {47,205,253,217,113,222,229,39,102,159,189,155,85,40,213,158,91,121,204,59,197,173,36,245,142,79,69,133,232,23,152,94,69,177,59,126,233,150,25,65,34,156,14,119,160,89,34,201,218,249,146,236,78,83,170,157,119,44,155,246,234,71,50,230},
  {53,173,77,198,54,205,192,100,237,19,100,213,125,142,248,202,226,225,63,239,211,154,163,62,253,19,106,185,151,53,52,115,203,233,161,113,220,177,44,34,170,119,116,105,87,73,254,71,109,255,196,206,110,178,212,198,161,154,1,70,176,223,101,191},
  {82,246,106,134,160,89,219,146,142,18,74,49,130,60,181,60,207,126,242,60,176,139,135,53,65,110,12,97,128,255,11,75,159,9,137,129,117,115,123,196,21,179,99,174,233,50,184,28,130,213,143,128,182,15,58,220,10,113,187,227,194,166,135,88},
  {49,221,118,84,30,0,75,45,3,171,131,185,182,74,226,185,70,12,84,167,134,141,199,39,99,199,8,52,232,77,2,9,240,78,112,238,107,183,98,190,39,74,210,234,171,147,164,25,27,160,164,218,27,242,72,8,150,110,51,79,163,198,40,98},
  {247,168,137,173,170,10,227,83,74,20,183,235,68,13,64,166,7,181,253,160,56,43,134,244,43,87,121,10,233,208,240,178,122,247,103,172,144,76,21,97,20,93,5,196,188,143,229,204,157,63,4,77,77,168,171,141,218,102,124,170,30,166,95,8},
  {51,98,239,173,208,3,63,174,158,136,208,217,227,36,70,101,176,204,39,70,243,82,93,108,115,134,21,239,98,137,214,160,203,228,22,213,231,207,19,231,28,1,59,22,183,17,216,169,114,17,147,213,217,121,190,72,3,14,124,69,60,206,171,160},
  {111,7,226,59,200,106,221,190,36,201,2,146,219,147,218,137,32,91,187,72,6,219,229,115,97,107,173,64,142,123,122,28,72,31,205,226,40,250,40,247,33,64,84,180,36,69,46,97,0,35,23,194,158,5,144,203,45,168,162,166,236,41,70,207},
  {129,61,72,222,141,150,85,26,170,207,228,33,137,105,16,211,115,34,58,71,73,171,169,216,125,252,129,40,15,31,21,56,18,106,176,33,49,216,109,26,212,73,79,97,81,86,43,30,174,64,80,179,0,188,155,79,138,11,153,13,110,213,237,175},
  {209,17,79,33,55,134,253,134,147,83,199,14,59,11,168,14,45,16,196,36,119,135,77,8,152,218,98,55,254,81,228,13,142,231,35,236,110,25,108,160,153,128,164,186,155,251,29,14,107,1,253,202,46,61,94,246,170,134,222,25,65,95,110,101},
  {101,157,162,172,19,142,30,114,118,182,40,14,162,159,49,44,237,36,92,186,190,254,57,15,83,64,23,218,155,211,181,90,66,190,54,97,241,235,22,55,108,186,211,197,4,120,126,121,90,89,204,123,204,240,211,103,213,92,67,62,105,84,173,146},
  {238,251,68,84,101,205,149,174,128,1,40,159,141,164,14,133,156,32,18,136,178,28,0,146,244,168,55,110,43,153,61,215,119,142,206,230,28,183,69,202,197,101,212,43,22,142,147,224,136,152,12,199,23,44,56,115,1,125,240,114,162,199,184,95},
  {242,150,2,165,84,34,81,8,107,135,249,24,79,154,79,242,142,110,145,130,59,36,231,78,120,121,247,237,195,230,16,196,224,217,107,40,16,36,125,111,132,223,231,226,34,125,55,68,99,110,239,95,86,175,195,254,245,196,111,140,180,70,254,130},
  {192,231,21,241,26,139,85,34,175,61,240,104,52,214,36,95,53,53,232,102,99,41,63,229,28,252,196,85,59,124,125,95,173,80,173,154,6,108,7,182,232,234,69,67,84,41,216,71,133,181,134,11,247,212,15,241,136,48,230,39,217,106,173,224},
  {164,8,208,190,45,234,120,146,85,80,227,72,167,241,209,116,240,80,133,225,76,59,241,30,148,151,24,15,175,16,118,111,96,92,126,22,249,175,208,198,196,211,195,18,224,150,140,90,67,109,80,204,136,198,215,63,244,252,74,255,236,216,224,60},
  {143,68,129,48,186,217,28,124,178,53,148,75,164,38,16,58,21,177,97,27,228,98,104,59,49,159,154,179,23,145,191,190,9,36,90,228,173,33,59,159,119,198,226,45,207,50,27,103,113,137,240,67,50,59,251,147,91,134,239,122,166,37,6,136},
  {67,93,192,31,207,9,240,174,18,59,30,165,76,64,27,40,112,70,105,99,119,62,225,154,99,73,179,3,33,249,21,14,128,197,37,106,10,254,154,218,74,131,249,114,165,34,233,63,117,174,15,152,99,36,7,51,167,130,163,73,67,146,171,151},
  {192,3,32,202,44,212,135,27,253,102,83,247,102,120,34,9,97,46,96,214,13,46,146,219,117,109,182,22,239,16,44,217,92,157,26,91,29,83,156,90,75,141,187,14,145,91,234,237,9,156,69,145,68,148,151,19,235,236,106,207,245,145,180,2},
  {226,165,78,202,22,237,69,208,177,172,157,176,242,23,192,161,196,251,94,197,97,117,202,166,52,98,47,79,216,39,200,18,60,69,180,214,173,120,179,84,141,113,248,106,39,93,176,49,167,77,71,67,59,63,143,241,135,14,89,72,107,213,59,220},
  {98,9,78,12,22,50,70,109,92,255,65,81,205,129,89,30,144,247,160,216,103,106,119,11,167,55,101,207,183,94,94,241,20,138,73,41,70,20,70,42,95,186,118,151,177,240,148,227,203,3,216,217,152,182,91,61,192,210,188,176,8,24,101,18},
  {198,200,36,153,137,8,120,18,160,122,245,42,251,253,233,221,37,193,116,162,95,6,178,147,131,14,177,225,198,95,43,155,150,3,107,158,159,46,183,60,242,103,124,114,225,255,183,3,123,39,200,121,183,249,209,124,12,131,123,51,228,148,113,111},
  {122,4,92,214,25,28,246,55,44,13,231,113,24,72,149,92,51,68,240,203,18,0,70,173,78,32,167,134,92,104,199,99,82,80,25,88,90,89,60,254,168,86,220,244,249,64,247,100,86,186,152,178,81,255,159,104,33,37,222,251,64,229,139,118},
  {47,49,114,144,183,90,3,119,112,4,154,218,207,102,224,224,86,245,150,229,96,192,55,236,83,40,157,15,160,43,91,98,196,18,31,124,241,123,162,0,217,21,216,227,37,98,181,138,188,215,227,71,188,45,92,189,125,39,165,196,195,29,56,134},
  {30,241,146,172,103,176,178,250,28,84,107,190,117,45,113,248,38,195,142,117,188,59,90,202,12,50,59,150,8,127,46,10,0,98,116,164,225,118,172,253,66,13,56,184,43,136,53,215,42,201,123,82,137,196,84,207,167,173,115,7,46,234,24,169},
  {183,36,145,194,227,6,73,124,85,195,22,159,211,254,202,239,31,42,106,158,131,168,241,112,0,55,53,252,134,211,45,163,92,154,38,220,231,25,189,227,198,135,116,24,89,99,191,214,75,10,239,12,6,239,249,130,236,20,105,4,171,9,171,180},
  {252,105,66,142,48,138,13,125,229,34,10,52,33,36,178,168,209,198,250,99,118,213,133,59,112,228,72,58,83,34,125,99,248,182,162,8,173,237,39,252,41,107,173,253,175,1,243,205,54,4,227,49,243,158,54,75,166,50,71,242,32,128,43,56},
  {53,147,135,177,56,1,28,187,128,77,129,190,18,233,246,140,50,211,214,50,177,167,193,3,188,27,3,22,64,173,196,50,62,90,20,17,140,22,89,249,172,52,186,93,83,192,156,55,240,229,143,59,81,187,250,154,176,113,8,121,57,16,209,229},
  {115,157,44,194,143,197,141,255,115,244,100,98,189,188,176,3,245,227,18,1,123,25,153,126,152,12,80,95,178,190,166,105,130,18,233,201,16,143,63,81,204,37,60,1,223,64,181,69,58,142,3,230,100,21,114,9,108,69,172,65,19,116,43,141},
  {178,68,121,209,110,66,188,205,118,78,254,215,133,224,197,187,172,27,152,154,45,18,223,187,86,114,32,234,234,79,13,15,168,176,248,171,122,69,31,47,39,32,118,114,73,112,126,164,115,136,34,36,217,160,141,8,62,125,156,237,246,185,76,56},
  {240,255,27,241,38,250,76,228,158,7,147,221,248,113,166,106,246,42,149,165,56,70,159,192,228,40,109,4,11,192,66,0,150,39,18,29,185,189,242,126,208,78,24,182,9,196,97,88,132,231,62,86,159,121,87,203,23,245,191,3,38,74,1,191},
  {153,135,153,14,56,81,112,25,105,144,201,54,149,227,71,108,165,69,42,37,221,40,119,233,21,121,69,29,72,8,90,241,61,133,7,196,188,243,146,217,27,5,169,165,39,252,4,61,30,248,195,193,222,228,125,190,52,136,10,142,25,142,107,237},
  {48,5,107,58,208,171,249,81,154,62,223,87,244,189,219,53,205,20,169,78,214,244,241,95,188,168,10,113,165,206,250,170,52,0,214,38,175,25,185,172,86,29,221,50,106,91,251,113,252,3,71,130,17,171,229,225,224,240,177,246,240,196,81,225},
  {33,141,171,29,83,96,74,60,127,75,23,102,142,180,122,239,151,68,217,225,60,67,113,22,34,165,46,215,59,107,90,51,96,57,235,150,252,246,20,58,157,199,253,208,84,82,21,160,135,79,71,226,160,154,21,141,207,48,154,98,227,104,42,239},
  {36,132,131,161,6,44,29,171,56,19,123,216,248,2,24,156,199,90,205,235,85,197,174,120,168,157,201,226,109,140,163,26,133,171,111,70,3,234,111,13,180,161,107,87,111,166,135,177,141,217,148,51,254,186,133,191,73,32,123,177,71,102,109,44},
  {131,11,253,0,145,245,180,242,40,201,7,4,79,35,114,208,205,57,67,154,232,69,11,36,44,6,180,171,58,76,160,253,103,171,128,120,83,178,220,182,8,122,246,240,138,94,209,37,147,44,6,106,189,209,179,242,138,241,254,39,86,126,213,74},
  {60,100,129,116,104,189,53,213,124,61,79,238,202,162,13,175,145,170,76,90,153,56,142,208,59,242,142,187,246,148,66,1,189,127,207,75,215,223,206,162,82,192,70,135,92,14,242,205,88,96,36,128,158,230,202,9,97,7,174,69,91,170,130,86},
  {210,201,20,91,78,203,250,194,97,138,185,65,190,198,76,249,191,142,203,115,244,13,29,213,165,128,55,102,107,173,175,44,62,162,1,28,138,130,144,175,12,200,166,112,31,102,255,223,32,107,88,58,114,85,249,15,63,218,244,184,91,170,160,81},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {235,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {236,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {237,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {238,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {239,211,245,92,26,99,18,88,214,156,247,162,222,249,222,20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {216,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {217,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {218,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {219,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {220,167,235,185,52,198,36,176,172,57,239,69,189,243,189,41,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,32,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {197,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {198,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {199,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {200,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {201,123,225,22,79,41,55,8,131,214,230,232,155,237,156,62,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,48,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {178,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {179,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {180,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {181,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {182,79,215,115,105,140,73,96,89,115,222,139,122,231,123,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {159,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {160,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {161,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {162,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {163,35,205,208,131,239,91,184,47,16,214,46,89,225,90,104,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {140,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {141,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {142,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {143,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {144,247,194,45,158,82,110,16,6,173,205,209,55,219,57,125,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,96,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {121,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {122,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {123,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {124,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {125,203,184,138,184,181,128,104,220,73,197,116,22,213,24,146,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,112,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {102,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {103,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {104,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {105,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {106,159,174,231,210,24,147,192,178,230,188,23,245,206,247,166,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {83,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {84,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {85,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {86,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {87,115,164,68,237,123,165,24,137,131,180,186,211,200,214,187,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,144,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {64,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {65,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {66,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {67,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {68,71,154,161,7,223,183,112,95,32,172,93,178,194,181,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,160,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {45,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {46,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {47,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {48,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {49,27,144,254,33,66,202,200,53,189,163,0,145,188,148,229,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,176,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {26,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {27,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {28,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {29,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {30,239,133,91,60,165,220,32,12,90,155,163,111,182,115,250,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,192,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {7,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {8,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {11,195,123,184,86,8,239,120,226,246,146,70,78,176,82,15,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,208,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {244,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {245,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {246,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {247,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {248,150,113,21,113,107,1,209,184,147,138,233,44,170,49,36,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,224,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {225,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {226,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {227,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {228,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {229,106,103,114,139,206,19,41,143,48,130,140,11,164,16,57,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,240,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
} ;

static const unsigned char precomputed_mGnP_ed25519_P[precomputed_mGnP_ed25519_NUM][crypto_mGnP_PBYTES] = {
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {8,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {11,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {14,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {17,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {18,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {19,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {22,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {23,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {24,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {26,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {27,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {28,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {29,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {30,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {31,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {133,160,158,72,186,1,91,246,75,247,188,75,16,198,93,128,164,113,55,55,220,92,49,210,31,166,134,94,97,102,5,0},
  {126,9,112,146,235,66,90,188,88,90,224,136,228,242,98,43,140,84,144,235,92,61,104,8,151,8,170,19,74,101,26,0},
  {224,235,122,124,59,65,184,174,22,86,227,250,241,159,196,106,218,9,141,235,156,50,177,253,134,98,5,22,95,73,184,0},
  {78,153,126,111,141,111,252,5,83,158,51,138,15,154,98,2,177,156,85,136,224,97,247,104,153,151,213,108,44,80,209,2},
  {38,232,149,143,194,178,39,176,69,195,244,137,242,239,152,240,213,223,172,5,211,198,51,57,177,56,2,136,109,83,252,5},
  {216,108,32,196,179,96,94,68,67,2,255,92,49,186,224,159,180,48,192,50,147,246,46,195,68,43,166,22,8,46,100,7},
  {57,219,28,111,68,228,27,79,57,195,231,19,30,185,73,83,121,159,249,151,252,104,182,48,163,51,251,217,35,68,59,15},
  {236,63,241,36,166,68,105,38,186,89,59,208,14,4,117,182,224,197,36,94,214,86,73,144,201,200,124,44,114,135,210,18},
  {131,196,183,157,143,181,189,1,175,102,45,100,112,127,96,148,29,133,197,254,108,223,64,228,78,218,154,119,172,195,122,20},
  {210,186,161,55,8,56,251,114,246,167,32,187,114,36,90,87,225,135,97,195,100,30,70,140,91,62,25,23,39,4,151,20},
  {36,145,108,107,202,212,204,240,212,7,88,47,196,61,65,201,118,189,125,61,219,34,22,53,62,0,98,1,168,122,87,26},
  {122,28,92,77,125,237,195,230,62,33,8,41,109,183,82,24,187,193,165,119,212,177,27,181,161,51,180,157,140,73,39,28},
  {40,88,214,103,129,21,75,153,249,2,125,196,106,194,160,240,125,135,250,100,100,169,68,112,20,135,90,176,29,90,59,28},
  {84,98,59,221,196,58,205,222,193,212,5,15,125,194,15,151,226,134,6,28,104,129,10,154,232,189,228,19,219,62,250,28},
  {190,29,39,95,90,208,83,102,202,66,104,169,222,136,104,211,11,169,221,5,219,246,37,64,231,183,79,157,51,91,215,31},
  {121,143,86,97,2,235,73,184,161,214,123,238,227,125,254,83,105,132,81,226,170,103,169,131,3,239,243,66,130,162,245,31},
  {73,5,247,160,82,142,177,165,227,112,105,120,156,113,65,250,51,182,120,221,241,74,127,117,108,20,238,116,190,67,140,33},
  {195,197,239,216,169,17,254,69,106,234,207,109,125,91,219,86,176,172,130,149,239,188,156,126,148,254,169,106,106,151,164,33},
  {199,107,6,14,106,204,173,171,172,197,195,135,208,242,65,105,184,65,3,53,248,19,42,24,130,222,59,132,6,169,41,35},
  {65,151,24,239,64,74,239,193,174,29,224,191,212,221,134,157,51,195,248,49,107,37,99,200,22,128,125,206,250,81,111,35},
  {177,247,0,177,3,171,57,137,155,38,125,194,25,247,245,140,155,229,0,42,239,191,97,152,75,172,202,145,101,17,214,38},
  {223,216,44,45,181,10,83,110,84,6,126,6,109,2,206,245,142,111,78,106,228,46,208,250,97,79,18,205,28,200,77,39},
  {87,139,29,60,28,115,112,14,34,56,78,44,30,181,221,106,70,70,179,41,26,78,163,211,121,60,160,133,169,94,159,40},
  {214,29,37,12,25,50,23,234,30,104,146,137,133,26,147,2,192,156,213,212,0,107,225,220,7,156,137,47,109,121,65,41},
  {71,115,11,40,71,121,222,157,200,253,37,88,149,201,189,228,121,103,16,216,209,224,15,179,45,249,173,154,254,86,129,44},
  {196,228,176,109,8,132,104,21,146,247,171,74,172,171,187,80,178,223,34,193,250,46,71,23,171,214,1,6,110,17,210,44},
  {180,197,177,2,16,210,146,58,7,75,243,35,76,11,100,180,162,152,182,121,119,109,163,254,48,36,81,97,128,102,147,45},
  {186,70,250,82,228,40,43,83,55,31,133,225,239,143,160,226,23,225,58,252,207,202,116,81,178,20,150,89,14,164,65,48},
  {2,164,39,56,19,10,255,33,76,226,219,91,174,27,185,47,165,171,243,223,243,221,75,112,196,245,103,21,248,247,187,48},
  {142,244,68,150,222,150,211,254,205,215,68,178,88,52,152,26,145,14,101,206,18,159,199,146,23,90,210,210,151,118,130,49},
  {18,235,145,30,130,228,169,214,15,240,202,250,202,187,97,240,114,203,81,230,53,178,128,21,204,150,37,113,166,164,179,50},
  {167,210,150,114,229,201,82,81,135,160,75,243,98,1,199,26,226,39,106,107,128,163,31,11,162,59,146,125,231,88,224,50},
  {68,2,91,26,217,254,139,221,39,73,184,210,146,164,44,78,184,255,112,208,54,162,56,50,153,18,172,159,67,102,122,53},
  {233,12,172,112,192,147,166,147,39,59,151,90,81,96,95,114,142,243,50,253,200,25,171,89,32,91,82,78,239,132,13,55},
  {225,110,19,97,81,243,92,255,202,187,153,206,129,100,141,62,48,57,186,217,143,9,134,162,123,112,179,197,253,176,56,55},
  {15,38,156,170,214,169,239,180,234,147,218,70,196,3,211,71,186,238,99,62,246,134,37,6,124,5,211,28,220,130,237,55},
  {217,49,191,136,255,92,143,239,180,251,18,245,222,109,34,183,41,127,73,5,254,225,153,68,176,43,208,43,10,220,95,57},
  {63,122,67,230,107,240,216,213,10,9,110,222,86,94,177,229,180,235,177,107,114,124,213,49,118,214,35,68,243,102,62,61},
  {205,131,228,181,108,197,210,92,220,69,217,99,14,236,103,11,254,178,143,176,74,17,223,84,177,112,113,45,60,168,63,64},
  {121,78,107,39,56,46,163,237,221,7,132,223,157,121,249,154,234,72,89,68,193,136,205,145,28,153,97,230,101,230,232,64},
  {45,143,42,61,151,228,197,167,52,156,175,209,21,74,32,108,56,62,136,106,141,11,89,252,59,187,76,76,65,62,245,64},
  {145,208,46,32,13,252,228,53,6,139,4,78,173,242,164,13,67,207,6,60,77,204,46,193,43,88,175,225,13,16,72,65},
  {182,18,95,224,137,137,7,82,75,21,206,108,134,232,116,54,130,23,101,9,173,169,33,221,66,72,237,193,236,175,21,68},
  {90,151,55,119,99,172,162,93,193,113,7,114,197,167,118,93,81,212,200,116,11,244,13,131,119,104,212,16,145,200,237,69},
  {52,237,196,85,30,241,2,205,246,153,11,186,199,15,81,101,179,37,191,166,164,177,249,254,245,193,141,16,27,12,7,71},
  {100,38,244,188,137,70,78,230,162,25,214,6,142,134,54,150,147,230,43,93,36,46,210,188,247,179,32,157,207,140,12,71},
  {103,147,223,167,47,28,51,206,134,23,84,253,66,253,110,157,218,220,83,83,161,249,85,144,43,247,67,184,116,36,66,74},
  {128,225,254,112,11,214,173,199,112,250,76,43,159,253,110,117,105,22,167,152,221,165,163,11,235,207,217,79,112,132,194,74},
  {36,104,163,116,124,109,81,96,141,238,89,230,234,59,182,21,202,187,196,154,32,78,218,111,43,79,206,50,58,105,205,74},
  {208,242,110,90,72,140,78,17,11,232,22,29,211,15,115,42,91,210,250,111,137,68,115,53,37,13,223,153,64,187,214,79},
  {103,146,179,33,108,99,169,165,255,105,248,67,103,183,62,212,155,30,0,215,242,229,230,102,119,48,128,159,187,25,225,79},
  {103,70,152,180,184,0,206,31,156,181,59,146,75,251,253,210,202,235,151,165,221,1,0,48,246,42,167,193,126,17,72,80},
  {29,249,54,71,143,91,3,152,41,66,231,43,195,76,148,132,237,52,126,153,4,43,248,35,68,10,40,98,122,17,1,82},
  {221,28,66,103,33,110,75,113,187,134,110,164,119,237,103,137,146,195,72,82,183,130,227,165,61,111,248,242,216,34,43,83},
  {8,95,214,196,190,247,191,203,167,220,91,68,19,123,144,2,144,245,250,107,3,96,231,241,243,86,193,203,135,228,31,85},
  {95,156,149,188,163,80,140,36,177,208,177,85,156,131,239,91,4,68,92,196,88,28,142,134,216,34,78,221,208,159,17,87},
  {29,182,217,36,219,194,226,80,102,180,84,3,108,168,134,207,11,172,66,128,227,162,187,36,19,1,127,120,57,109,178,87},
  {87,173,12,84,130,4,224,15,255,165,110,22,255,93,102,224,201,159,218,22,129,106,106,100,37,117,25,252,193,223,118,88},
  {50,218,214,158,75,153,232,238,246,90,195,93,68,97,229,252,238,232,229,49,117,154,108,77,181,165,23,48,119,20,9,94},
  {33,158,135,215,170,85,89,178,199,10,178,241,198,171,131,3,255,209,83,204,190,12,196,245,219,115,188,16,200,207,179,98},
  {54,129,203,31,182,210,103,149,201,84,239,1,4,173,95,59,164,114,126,8,140,177,48,18,5,0,77,93,8,226,19,103},
  {213,105,89,54,151,2,93,204,228,177,220,135,151,91,75,114,81,144,186,151,76,179,156,47,245,18,103,71,82,115,76,107},
  {210,76,135,163,211,65,145,20,82,211,212,126,203,80,22,158,160,58,151,101,216,149,170,224,51,231,7,192,105,11,100,107},
  {249,39,38,36,232,230,206,127,84,51,49,159,243,150,75,230,222,245,87,204,175,97,107,179,146,240,206,238,89,157,153,108},
  {60,165,126,144,74,35,167,121,71,123,75,252,34,112,32,138,190,164,37,251,92,56,21,215,208,217,97,205,165,45,185,108},
  {78,154,192,104,2,2,81,141,207,192,101,46,36,183,197,24,154,29,122,177,62,241,220,250,177,110,114,197,242,199,41,112},
  {215,112,3,150,147,250,141,106,5,182,204,42,223,25,43,127,93,204,144,57,219,152,192,236,42,0,172,170,34,163,162,112},
  {28,108,193,36,56,188,232,222,209,69,152,221,56,98,101,247,22,253,219,116,84,194,12,86,14,193,79,74,18,213,192,112},
  {113,168,202,225,174,241,105,63,76,244,7,153,70,8,140,238,63,59,110,110,8,78,147,119,241,98,69,40,35,48,237,113},
  {63,24,82,104,230,229,60,65,45,199,150,19,78,9,16,25,94,21,229,57,134,215,65,160,213,176,158,236,37,61,42,114},
  {136,141,166,125,172,46,107,241,28,17,176,185,50,174,195,232,68,142,198,30,162,192,132,24,159,163,8,179,67,215,113,116},
  {15,212,111,12,27,25,213,247,143,250,231,170,223,124,55,203,221,72,232,246,227,201,223,174,76,9,234,163,228,146,211,116},
  {189,129,176,12,94,42,9,222,24,87,94,100,15,11,204,157,195,143,17,33,3,187,44,41,61,147,60,52,244,226,78,118},
  {7,15,29,158,249,45,243,86,149,240,67,227,182,187,29,165,226,107,4,189,241,76,230,17,194,202,244,253,216,151,180,121},
  {199,23,106,112,61,77,216,79,186,60,11,118,13,16,103,15,42,32,83,250,44,57,204,198,78,199,253,119,146,172,3,122},
  {109,157,174,111,128,130,186,160,252,179,9,224,135,223,178,38,180,195,3,142,101,113,183,205,46,227,81,238,31,115,191,126},
  {224,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {225,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {226,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {227,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {228,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {229,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {230,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {231,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {232,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {233,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {234,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {235,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {237,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {238,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {239,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {240,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {241,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {245,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {248,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {250,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {252,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {253,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {254,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,127},
  {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {8,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {9,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {11,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {14,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {17,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {18,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {19,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {20,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {22,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {23,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {24,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {26,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {27,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {28,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {29,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {30,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {31,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,128},
  {15,202,72,65,134,59,164,90,150,30,251,86,17,92,90,253,247,183,247,41,145,35,193,117,17,136,192,98,245,81,100,128},
  {205,235,122,124,59,65,184,174,22,86,227,250,241,159,196,106,218,9,141,235,156,50,177,253,134,98,5,22,95,73,184,128},
  {134,45,86,6,135,126,106,53,88,197,149,226,163,37,202,21,175,21,54,79,124,201,81,251,29,168,112,67,85,215,214,133},
  {19,232,149,143,194,178,39,176,69,195,244,137,242,239,152,240,213,223,172,5,211,198,51,57,177,56,2,136,109,83,252,133},
  {169,110,86,37,132,219,71,97,245,248,1,116,176,11,9,146,76,167,15,128,12,4,150,39,207,94,13,180,150,216,139,134},
  {27,94,32,221,185,253,205,81,16,106,160,71,66,193,172,50,128,123,150,24,2,205,204,28,35,232,42,182,24,139,247,134},
  {42,253,187,252,134,95,243,186,16,181,187,15,255,28,206,0,110,241,40,76,23,47,234,133,198,158,33,31,179,145,23,135},
  {41,109,180,132,253,90,130,59,68,248,72,68,176,181,63,184,171,140,214,99,65,91,214,4,160,17,215,177,76,55,164,136},
  {89,176,33,129,176,6,44,214,190,23,39,124,63,9,188,121,108,239,97,239,34,189,0,153,47,119,0,160,146,111,87,139},
  {11,46,34,224,225,54,132,37,50,225,50,181,111,149,20,28,55,237,157,88,182,35,146,71,255,20,62,22,131,72,123,143},
  {175,8,2,207,88,111,14,41,6,241,193,250,227,210,71,27,173,58,211,40,158,235,38,41,77,191,164,39,149,115,145,143},
  {158,63,163,2,209,61,243,101,255,215,205,118,14,128,144,81,227,196,193,155,244,96,139,47,49,229,71,152,32,158,96,144},
  {168,222,62,78,165,5,144,178,39,48,36,112,244,186,178,129,204,219,85,212,107,36,99,166,133,109,125,246,2,134,254,145},
  {161,164,99,189,12,125,22,108,64,69,81,70,238,147,169,129,39,21,125,46,182,24,184,237,48,87,82,210,76,63,69,148},
  {170,162,42,35,41,84,42,97,251,71,156,129,94,24,92,143,122,46,126,93,23,192,112,188,136,198,130,139,213,202,223,148},
  {168,98,236,26,122,57,204,10,102,3,57,100,14,248,246,86,160,155,93,84,30,194,127,129,10,213,18,36,220,243,182,150},
  {62,72,157,122,56,223,50,242,126,141,3,69,108,245,176,206,48,10,150,154,125,43,80,183,144,157,16,85,100,144,208,153},
  {59,3,54,152,124,140,187,237,103,170,221,75,236,125,70,49,81,201,224,116,6,46,3,154,33,29,120,202,38,178,124,155},
  {197,131,97,219,129,218,22,217,9,209,249,57,140,173,163,106,113,34,131,143,175,83,178,28,7,22,221,212,49,225,175,159},
  {162,32,69,38,74,213,90,104,72,3,18,57,109,49,55,243,64,172,152,80,15,123,12,201,235,19,76,9,208,93,0,166},
  {116,224,121,163,115,49,221,231,34,23,75,55,170,41,248,219,64,30,172,12,14,12,22,73,85,92,239,84,238,225,137,167},
  {187,129,32,77,78,44,152,80,61,210,244,23,155,18,159,123,75,11,180,226,234,33,106,228,99,10,71,36,196,124,186,168},
  {33,135,173,176,8,63,68,129,123,108,150,228,3,1,207,219,61,134,80,166,211,205,62,43,251,80,230,122,84,229,174,170},
  {192,162,3,203,98,171,180,1,119,178,173,156,44,204,12,246,1,20,24,143,6,228,102,249,59,211,209,107,144,228,91,171},
  {214,214,220,104,206,133,2,160,149,204,55,98,82,188,149,15,156,46,142,39,255,210,231,206,119,101,85,249,61,71,105,171},
  {203,39,31,0,55,163,112,132,197,100,203,59,48,241,40,44,149,91,114,205,194,202,152,164,241,115,85,14,215,8,63,179},
  {50,174,127,41,66,118,225,92,168,54,126,229,107,137,0,209,170,175,233,44,66,199,138,179,129,180,92,2,191,35,1,180},
  {133,87,197,21,1,127,223,75,81,227,105,50,176,135,246,243,135,36,218,220,216,101,27,75,222,163,174,73,160,3,113,181},
  {169,250,39,98,67,83,7,88,248,188,130,125,154,194,114,112,26,221,139,207,155,50,237,29,19,134,110,13,212,8,68,182},
  {231,97,91,239,176,88,194,117,73,15,124,25,71,53,206,155,65,82,141,200,198,213,236,155,199,186,144,197,94,36,72,185},
  {82,164,31,126,105,247,223,71,7,119,74,46,203,243,129,215,184,44,218,12,105,81,114,135,162,240,192,229,122,11,238,186},
  {68,107,89,151,178,8,165,126,190,185,3,102,128,110,169,106,207,22,232,197,118,195,230,143,170,73,134,183,173,15,40,188},
  {185,152,154,170,236,246,115,238,74,219,239,56,100,83,37,163,50,33,189,75,224,75,190,203,72,137,112,104,220,239,104,193},
  {59,214,195,53,132,229,207,187,49,198,64,174,162,142,160,26,20,185,105,4,177,178,190,21,28,215,211,7,3,121,39,198},
  {165,1,85,204,92,195,160,51,144,94,25,60,59,220,190,189,53,83,239,30,122,38,249,5,158,236,141,178,225,14,167,203},
  {88,81,251,2,207,106,39,128,151,6,28,147,198,137,136,250,160,252,133,169,145,168,218,72,34,12,204,148,17,234,154,208},
  {238,225,222,190,117,211,58,162,19,55,118,168,14,229,79,71,244,82,123,68,109,97,137,130,38,14,102,255,34,118,243,213},
  {116,42,148,68,23,204,156,70,174,248,50,174,62,189,211,94,76,60,4,17,71,103,132,33,105,19,78,144,215,223,83,214},
  {76,156,149,188,163,80,140,36,177,208,177,85,156,131,239,91,4,68,92,196,88,28,142,134,216,34,78,221,208,159,17,215},
  {139,202,181,227,103,99,103,9,81,107,188,27,164,116,79,159,216,139,8,179,208,79,37,87,110,212,142,176,25,99,210,216},
  {81,92,255,221,73,228,142,1,185,168,93,221,226,119,29,193,48,71,4,77,84,27,95,120,52,51,217,35,88,133,26,217},
  {182,156,86,236,201,64,95,215,117,54,216,131,185,176,83,160,222,174,131,178,241,17,60,108,57,200,140,182,155,46,54,217},
  {9,188,121,142,101,162,29,184,100,78,137,159,37,111,57,77,113,60,16,102,124,242,102,127,172,96,14,134,139,214,140,217},
  {68,49,61,40,119,1,237,10,222,249,20,18,173,202,76,112,101,244,115,26,3,110,95,3,175,58,158,206,228,224,69,221},
  {110,231,191,231,185,198,135,152,226,160,157,20,180,236,4,95,104,20,220,153,68,77,224,29,125,241,106,176,102,3,108,222},
  {175,196,110,247,30,28,103,111,47,204,216,96,129,4,55,151,114,121,86,203,7,249,208,248,173,147,50,247,229,196,88,225},
  {25,77,245,209,222,132,188,180,142,57,37,51,8,156,72,189,163,170,90,15,141,174,226,219,255,181,243,35,175,126,96,225},
  {170,98,56,164,72,174,110,232,61,139,156,208,254,101,181,175,151,86,215,57,110,193,214,63,2,135,171,215,210,20,245,225},
  {31,197,166,74,131,146,196,77,31,22,233,128,227,240,166,28,119,63,127,132,247,126,126,16,126,5,253,115,3,239,206,226},
  {69,106,177,228,109,125,112,168,162,247,211,94,144,52,79,60,108,23,19,11,146,34,37,84,243,205,47,160,105,187,209,226},
  {28,171,209,216,175,254,199,10,151,151,156,191,38,110,76,81,57,13,152,95,69,12,16,242,120,100,70,152,131,35,11,227},
  {81,189,38,89,68,73,35,175,82,161,183,179,3,172,11,122,32,165,68,13,82,166,61,232,24,57,169,104,75,75,179,230},
  {182,145,235,22,179,180,64,250,107,114,83,50,154,212,240,27,181,88,189,14,124,230,164,117,193,3,21,221,168,253,50,231},
  {176,79,79,215,176,10,153,157,240,165,119,165,163,42,33,167,29,129,169,168,188,121,206,135,66,47,164,94,59,253,205,235},
  {99,157,130,192,144,139,60,72,218,187,135,117,63,67,39,238,21,141,213,107,239,19,161,86,49,187,57,169,167,46,50,237},
  {180,23,106,112,61,77,216,79,186,60,11,118,13,16,103,15,42,32,83,250,44,57,204,198,78,199,253,119,146,172,3,250},
  {246,62,201,197,215,170,104,159,210,158,46,56,180,16,88,31,190,213,165,127,255,80,20,248,96,9,223,108,60,26,11,250},
  {239,19,109,220,255,224,30,59,97,20,253,177,54,40,172,36,207,251,146,132,101,137,195,105,75,155,185,156,11,234,91,250},
  {29,21,207,61,161,253,57,233,159,131,233,236,31,111,26,48,33,50,187,28,71,206,175,59,55,143,249,117,160,119,245,250},
  {150,161,67,40,164,154,114,122,147,65,34,174,195,86,255,45,139,28,69,231,240,129,82,161,24,231,190,9,96,57,113,251},
  {192,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {193,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {194,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {195,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {196,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {197,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {198,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {199,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {200,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {201,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {202,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {203,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {204,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {205,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {206,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {207,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {208,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {209,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {210,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {211,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {212,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {213,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {214,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {215,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {216,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {217,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {218,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {219,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {220,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {221,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {222,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {223,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {224,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {225,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {226,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {227,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {228,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {229,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {230,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {231,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {232,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {233,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {234,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {235,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {236,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {237,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {238,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {239,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {240,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {241,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {242,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {243,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {244,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {245,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {246,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {247,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {248,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {249,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {250,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {251,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {252,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {253,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {254,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
  {10,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0},
} ;

static void test_mGnP_ed25519_impl(long long impl)
{
  unsigned char *Q = test_mGnP_ed25519_Q;
  unsigned char *m = test_mGnP_ed25519_m;
  unsigned char *n = test_mGnP_ed25519_n;
  unsigned char *P = test_mGnP_ed25519_P;
  unsigned char *Q2 = test_mGnP_ed25519_Q2;
  unsigned char *m2 = test_mGnP_ed25519_m2;
  unsigned char *n2 = test_mGnP_ed25519_n2;
  unsigned char *P2 = test_mGnP_ed25519_P2;
  long long Qlen = crypto_mGnP_OUTPUTBYTES;
  long long mlen = crypto_mGnP_MBYTES;
  long long nlen = crypto_mGnP_NBYTES;
  long long Plen = crypto_mGnP_PBYTES;

  if (targeti && strcmp(targeti,lib25519_dispatch_mGnP_ed25519_implementation(impl))) return;
  if (impl >= 0) {
    crypto_mGnP = lib25519_dispatch_mGnP_ed25519(impl);
    printf("mGnP_ed25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_mGnP_ed25519_implementation(impl),lib25519_dispatch_mGnP_ed25519_compiler(impl));
  } else {
    crypto_mGnP = lib25519_mGnP_ed25519;
    printf("mGnP_ed25519 selected implementation %s compiler %s\n",lib25519_mGnP_ed25519_implementation(),lib25519_mGnP_ed25519_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 1024 : 128;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {

      output_prepare(Q2,Q,Qlen);
      input_prepare(m2,m,mlen);
      input_prepare(n2,n,nlen);
      input_prepare(P2,P,Plen);
      crypto_mGnP(Q,m,n,P);
      checksum(Q,Qlen);
      output_compare(Q2,Q,Qlen,"crypto_mGnP");
      input_compare(m2,m,mlen,"crypto_mGnP");
      input_compare(n2,n,nlen,"crypto_mGnP");
      input_compare(P2,P,Plen,"crypto_mGnP");

      double_canary(Q2,Q,Qlen);
      double_canary(m2,m,mlen);
      double_canary(n2,n,nlen);
      double_canary(P2,P,Plen);
      crypto_mGnP(Q2,m2,n2,P2);
      if (memcmp(Q2,Q,Qlen) != 0) fail("failure: crypto_mGnP is nondeterministic\n");

      double_canary(Q2,Q,Qlen);
      double_canary(m2,m,mlen);
      double_canary(n2,n,nlen);
      double_canary(P2,P,Plen);
      crypto_mGnP(m2,m2,n,P);
      if (memcmp(m2,Q,Qlen) != 0) fail("failure: crypto_mGnP does not handle m=Q overlap\n");
      memcpy(m2,m,mlen);
      crypto_mGnP(n2,m,n2,P);
      if (memcmp(n2,Q,Qlen) != 0) fail("failure: crypto_mGnP does not handle n=Q overlap\n");
      memcpy(n2,n,nlen);
      crypto_mGnP(P2,m,n,P2);
      if (memcmp(P2,Q,Qlen) != 0) fail("failure: crypto_mGnP does not handle P=Q overlap\n");
      memcpy(P2,P,Plen);
    }
    checksum_expected(mGnP_ed25519_checksums[checksumbig]);
  }
  for (long long precomp = 0;precomp < precomputed_mGnP_ed25519_NUM;++precomp) {
    output_prepare(Q2,Q,crypto_mGnP_OUTPUTBYTES);
    input_prepare(m2,m,crypto_mGnP_MBYTES);
    memcpy(m,precomputed_mGnP_ed25519_m[precomp],crypto_mGnP_MBYTES);
    memcpy(m2,precomputed_mGnP_ed25519_m[precomp],crypto_mGnP_MBYTES);
    input_prepare(n2,n,crypto_mGnP_NBYTES);
    memcpy(n,precomputed_mGnP_ed25519_n[precomp],crypto_mGnP_NBYTES);
    memcpy(n2,precomputed_mGnP_ed25519_n[precomp],crypto_mGnP_NBYTES);
    input_prepare(P2,P,crypto_mGnP_PBYTES);
    memcpy(P,precomputed_mGnP_ed25519_P[precomp],crypto_mGnP_PBYTES);
    memcpy(P2,precomputed_mGnP_ed25519_P[precomp],crypto_mGnP_PBYTES);
    crypto_mGnP(Q,m,n,P);
    if (memcmp(Q,precomputed_mGnP_ed25519_Q[precomp],crypto_mGnP_OUTPUTBYTES)) {
      fail("failure: crypto_mGnP fails precomputed test vectors\n");
      printf("expected Q: ");
      for (long long pos = 0;pos < crypto_mGnP_OUTPUTBYTES;++pos) printf("%02x",precomputed_mGnP_ed25519_Q[precomp][pos]);
      printf("\n");
      printf("received Q: ");
      for (long long pos = 0;pos < crypto_mGnP_OUTPUTBYTES;++pos) printf("%02x",Q[pos]);
      printf("\n");
    }
    output_compare(Q2,Q,crypto_mGnP_OUTPUTBYTES,"crypto_mGnP");
    input_compare(m2,m,crypto_mGnP_MBYTES,"crypto_mGnP");
    input_compare(n2,n,crypto_mGnP_NBYTES,"crypto_mGnP");
    input_compare(P2,P,crypto_mGnP_PBYTES,"crypto_mGnP");
  }
}

static void test_mGnP_ed25519(void)
{
  if (targeto && strcmp(targeto,"mGnP")) return;
  if (targetp && strcmp(targetp,"ed25519")) return;
  test_mGnP_ed25519_Q = alignedcalloc(crypto_mGnP_OUTPUTBYTES);
  test_mGnP_ed25519_m = alignedcalloc(crypto_mGnP_MBYTES);
  test_mGnP_ed25519_n = alignedcalloc(crypto_mGnP_NBYTES);
  test_mGnP_ed25519_P = alignedcalloc(crypto_mGnP_PBYTES);
  test_mGnP_ed25519_Q2 = alignedcalloc(crypto_mGnP_OUTPUTBYTES);
  test_mGnP_ed25519_m2 = alignedcalloc(crypto_mGnP_MBYTES);
  test_mGnP_ed25519_n2 = alignedcalloc(crypto_mGnP_NBYTES);
  test_mGnP_ed25519_P2 = alignedcalloc(crypto_mGnP_PBYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("mGnP_ed25519 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_mGnP_ed25519();++impl)
      forked(test_mGnP_ed25519_impl,impl);
    ++test_mGnP_ed25519_Q;
    ++test_mGnP_ed25519_m;
    ++test_mGnP_ed25519_n;
    ++test_mGnP_ed25519_P;
    ++test_mGnP_ed25519_Q2;
    ++test_mGnP_ed25519_m2;
    ++test_mGnP_ed25519_n2;
    ++test_mGnP_ed25519_P2;
  }
}
#undef crypto_mGnP_MBYTES
#undef crypto_mGnP_NBYTES
#undef crypto_mGnP_PBYTES
#undef crypto_mGnP_OUTPUTBYTES


/* ----- dh, derived from supercop/crypto_dh/try.c */
static const char *dh_x25519_checksums[] = {
  "2c8a73ec86d5d4c4bc838f49cfd78c87b60b534ae6fff59ce3bea0c32cdc1450",
  "b09016b3a1371786b46a183085133338159e623c5eb9cbc5eaa4f8b62d6c5aea",
} ;

static void (*crypto_dh_keypair)(unsigned char *,unsigned char *);
static void (*crypto_dh)(unsigned char *,const unsigned char *,const unsigned char *);
#define crypto_dh_SECRETKEYBYTES lib25519_dh_x25519_SECRETKEYBYTES
#define crypto_dh_PUBLICKEYBYTES lib25519_dh_x25519_PUBLICKEYBYTES
#define crypto_dh_BYTES lib25519_dh_x25519_BYTES

static unsigned char *test_dh_x25519_a;
static unsigned char *test_dh_x25519_b;
static unsigned char *test_dh_x25519_c;
static unsigned char *test_dh_x25519_d;
static unsigned char *test_dh_x25519_e;
static unsigned char *test_dh_x25519_f;
static unsigned char *test_dh_x25519_a2;
static unsigned char *test_dh_x25519_b2;
static unsigned char *test_dh_x25519_c2;
static unsigned char *test_dh_x25519_d2;
static unsigned char *test_dh_x25519_e2;
static unsigned char *test_dh_x25519_f2;

static void test_dh_x25519_impl(long long impl)
{
  unsigned char *a = test_dh_x25519_a;
  unsigned char *b = test_dh_x25519_b;
  unsigned char *c = test_dh_x25519_c;
  unsigned char *d = test_dh_x25519_d;
  unsigned char *e = test_dh_x25519_e;
  unsigned char *f = test_dh_x25519_f;
  unsigned char *a2 = test_dh_x25519_a2;
  unsigned char *b2 = test_dh_x25519_b2;
  unsigned char *c2 = test_dh_x25519_c2;
  unsigned char *d2 = test_dh_x25519_d2;
  unsigned char *e2 = test_dh_x25519_e2;
  unsigned char *f2 = test_dh_x25519_f2;
  long long alen = crypto_dh_SECRETKEYBYTES;
  long long blen = crypto_dh_SECRETKEYBYTES;
  long long clen = crypto_dh_PUBLICKEYBYTES;
  long long dlen = crypto_dh_PUBLICKEYBYTES;
  long long elen = crypto_dh_BYTES;
  long long flen = crypto_dh_BYTES;

  if (targeti && strcmp(targeti,lib25519_dispatch_dh_x25519_implementation(impl))) return;
  if (impl >= 0) {
    crypto_dh_keypair = lib25519_dispatch_dh_x25519_keypair(impl);
    crypto_dh = lib25519_dispatch_dh_x25519(impl);
    printf("dh_x25519 %lld implementation %s compiler %s\n",impl,lib25519_dispatch_dh_x25519_implementation(impl),lib25519_dispatch_dh_x25519_compiler(impl));
  } else {
    crypto_dh_keypair = lib25519_dh_x25519_keypair;
    crypto_dh = lib25519_dh_x25519;
    printf("dh_x25519 selected implementation %s compiler %s\n",lib25519_dh_x25519_implementation(),lib25519_dh_x25519_compiler());
  }
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 512 : 64;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {

      output_prepare(c2,c,clen);
      output_prepare(a2,a,alen);
      crypto_dh_keypair(c,a);
      checksum(c,clen);
      checksum(a,alen);
      output_compare(c2,c,clen,"crypto_dh_keypair");
      output_compare(a2,a,alen,"crypto_dh_keypair");
      output_prepare(d2,d,dlen);
      output_prepare(b2,b,blen);
      crypto_dh_keypair(d,b);
      checksum(d,dlen);
      checksum(b,blen);
      output_compare(d2,d,dlen,"crypto_dh_keypair");
      output_compare(b2,b,blen,"crypto_dh_keypair");
      output_prepare(e2,e,elen);
      memcpy(d2,d,dlen);
      double_canary(d2,d,dlen);
      memcpy(a2,a,alen);
      double_canary(a2,a,alen);
      crypto_dh(e,d,a);
      checksum(e,elen);
      output_compare(e2,e,elen,"crypto_dh");
      input_compare(d2,d,dlen,"crypto_dh");
      input_compare(a2,a,alen,"crypto_dh");

      double_canary(e2,e,elen);
      double_canary(d2,d,dlen);
      double_canary(a2,a,alen);
      crypto_dh(e2,d2,a2);
      if (memcmp(e2,e,elen) != 0) fail("failure: crypto_dh is nondeterministic\n");

      double_canary(e2,e,elen);
      double_canary(d2,d,dlen);
      double_canary(a2,a,alen);
      crypto_dh(d2,d2,a);
      if (memcmp(d2,e,elen) != 0) fail("failure: crypto_dh does not handle d=e overlap\n");
      memcpy(d2,d,dlen);
      crypto_dh(a2,d,a2);
      if (memcmp(a2,e,elen) != 0) fail("failure: crypto_dh does not handle a=e overlap\n");
      memcpy(a2,a,alen);
      output_prepare(f2,f,flen);
      memcpy(c2,c,clen);
      double_canary(c2,c,clen);
      memcpy(b2,b,blen);
      double_canary(b2,b,blen);
      crypto_dh(f,c,b);
      checksum(f,flen);
      output_compare(f2,f,flen,"crypto_dh");
      input_compare(c2,c,clen,"crypto_dh");
      input_compare(b2,b,blen,"crypto_dh");

      double_canary(f2,f,flen);
      double_canary(c2,c,clen);
      double_canary(b2,b,blen);
      crypto_dh(f2,c2,b2);
      if (memcmp(f2,f,flen) != 0) fail("failure: crypto_dh is nondeterministic\n");

      double_canary(f2,f,flen);
      double_canary(c2,c,clen);
      double_canary(b2,b,blen);
      crypto_dh(c2,c2,b);
      if (memcmp(c2,f,flen) != 0) fail("failure: crypto_dh does not handle c=f overlap\n");
      memcpy(c2,c,clen);
      crypto_dh(b2,c,b2);
      if (memcmp(b2,f,flen) != 0) fail("failure: crypto_dh does not handle b=f overlap\n");
      memcpy(b2,b,blen);

      if (memcmp(f,e,elen) != 0) fail("failure: crypto_dh not associative\n");
    }
    checksum_expected(dh_x25519_checksums[checksumbig]);
  }
}

static void test_dh_x25519(void)
{
  if (targeto && strcmp(targeto,"dh")) return;
  if (targetp && strcmp(targetp,"x25519")) return;
  test_dh_x25519_a = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_b = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_c = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_d = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_e = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_f = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_a2 = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_b2 = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_c2 = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_d2 = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_e2 = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);
  test_dh_x25519_f2 = alignedcalloc(crypto_dh_BYTES+crypto_dh_PUBLICKEYBYTES+crypto_dh_SECRETKEYBYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("dh_x25519 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_dh_x25519();++impl)
      forked(test_dh_x25519_impl,impl);
    ++test_dh_x25519_a;
    ++test_dh_x25519_b;
    ++test_dh_x25519_c;
    ++test_dh_x25519_d;
    ++test_dh_x25519_e;
    ++test_dh_x25519_f;
    ++test_dh_x25519_a2;
    ++test_dh_x25519_b2;
    ++test_dh_x25519_c2;
    ++test_dh_x25519_d2;
    ++test_dh_x25519_e2;
    ++test_dh_x25519_f2;
  }
}
#undef crypto_dh_SECRETKEYBYTES
#undef crypto_dh_PUBLICKEYBYTES
#undef crypto_dh_BYTES


/* ----- sign, derived from supercop/crypto_sign/try.c */
static const char *sign_ed25519_checksums[] = {
  "ce11fd7c1eac4dd0bc5eec49b26ad1e91aef696fae50ce377dbd806dc394da01",
  "2ed857f17c917a8185e6c296303a11772ae45683a5e7cb5b095489bad65fffde",
} ;

static void (*crypto_sign_keypair)(unsigned char *,unsigned char *);
static void (*crypto_sign)(unsigned char *,long long *,const unsigned char *,long long,const unsigned char *);
static int (*crypto_sign_open)(unsigned char *,long long *,const unsigned char *,long long,const unsigned char *);
#define crypto_sign_SECRETKEYBYTES lib25519_sign_ed25519_SECRETKEYBYTES
#define crypto_sign_PUBLICKEYBYTES lib25519_sign_ed25519_PUBLICKEYBYTES
#define crypto_sign_BYTES lib25519_sign_ed25519_BYTES

static unsigned char *test_sign_ed25519_p;
static unsigned char *test_sign_ed25519_s;
static unsigned char *test_sign_ed25519_m;
static unsigned char *test_sign_ed25519_c;
static unsigned char *test_sign_ed25519_t;
static unsigned char *test_sign_ed25519_p2;
static unsigned char *test_sign_ed25519_s2;
static unsigned char *test_sign_ed25519_m2;
static unsigned char *test_sign_ed25519_c2;
static unsigned char *test_sign_ed25519_t2;

static void test_sign_ed25519_impl(long long impl)
{
  unsigned char *p = test_sign_ed25519_p;
  unsigned char *s = test_sign_ed25519_s;
  unsigned char *m = test_sign_ed25519_m;
  unsigned char *c = test_sign_ed25519_c;
  unsigned char *t = test_sign_ed25519_t;
  unsigned char *p2 = test_sign_ed25519_p2;
  unsigned char *s2 = test_sign_ed25519_s2;
  unsigned char *m2 = test_sign_ed25519_m2;
  unsigned char *c2 = test_sign_ed25519_c2;
  unsigned char *t2 = test_sign_ed25519_t2;
  long long plen = crypto_sign_PUBLICKEYBYTES;
  long long slen = crypto_sign_SECRETKEYBYTES;
  long long mlen;
  long long clen;
  long long tlen;

  if (targeti && strcmp(targeti,lib25519_dispatch_sign_ed25519_implementation(impl))) return;
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
  for (long long checksumbig = 0;checksumbig < 2;++checksumbig) {
    long long loops = checksumbig ? 64 : 8;
    long long maxtest = checksumbig ? 4096 : 128;

    checksum_clear();

    for (long long loop = 0;loop < loops;++loop) {
      int result;
      mlen = myrandom() % (maxtest + 1);

      output_prepare(p2,p,plen);
      output_prepare(s2,s,slen);
      crypto_sign_keypair(p,s);
      checksum(p,plen);
      checksum(s,slen);
      output_compare(p2,p,plen,"crypto_sign_keypair");
      output_compare(s2,s,slen,"crypto_sign_keypair");
      clen = mlen + crypto_sign_BYTES;
      output_prepare(c2,c,clen);
      input_prepare(m2,m,mlen);
      memcpy(s2,s,slen);
      double_canary(s2,s,slen);
      crypto_sign(c,&clen,m,mlen,s);
      if (clen < mlen) fail("failure: crypto_sign returns smaller output than input\n");
      if (clen > mlen + crypto_sign_BYTES) fail("failure: crypto_sign returns more than crypto_sign_BYTES extra bytes\n");
      checksum(c,clen);
      output_compare(c2,c,clen,"crypto_sign");
      input_compare(m2,m,mlen,"crypto_sign");
      input_compare(s2,s,slen,"crypto_sign");
      tlen = clen;
      output_prepare(t2,t,tlen);
      memcpy(c2,c,clen);
      double_canary(c2,c,clen);
      memcpy(p2,p,plen);
      double_canary(p2,p,plen);
      result = crypto_sign_open(t,&tlen,c,clen,p);
      if (result != 0) fail("failure: crypto_sign_open returns nonzero\n");
      if (tlen != mlen) fail("failure: crypto_sign_open does not match mlen\n");
      if (memcmp(t,m,mlen) != 0) fail("failure: crypto_sign_open does not match m\n");
      checksum(t,tlen);
      output_compare(t2,t,clen,"crypto_sign_open");
      input_compare(c2,c,clen,"crypto_sign_open");
      input_compare(p2,p,plen,"crypto_sign_open");

      double_canary(t2,t,tlen);
      double_canary(c2,c,clen);
      double_canary(p2,p,plen);
      result = crypto_sign_open(t2,&tlen,c2,clen,p2);
      if (result != 0) fail("failure: crypto_sign_open returns nonzero\n");
      if (memcmp(t2,t,tlen) != 0) fail("failure: crypto_sign_open is nondeterministic\n");

      double_canary(t2,t,tlen);
      double_canary(c2,c,clen);
      double_canary(p2,p,plen);
      result = crypto_sign_open(c2,&tlen,c2,clen,p);
      if (result != 0) fail("failure: crypto_sign_open with c=t overlap returns nonzero\n");
      if (memcmp(c2,t,tlen) != 0) fail("failure: crypto_sign_open does not handle c=t overlap\n");
      memcpy(c2,c,clen);
      result = crypto_sign_open(p2,&tlen,c,clen,p2);
      if (result != 0) fail("failure: crypto_sign_open with p=t overlap returns nonzero\n");
      if (memcmp(p2,t,tlen) != 0) fail("failure: crypto_sign_open does not handle p=t overlap\n");
      memcpy(p2,p,plen);

      c[myrandom() % clen] += 1 + (myrandom() % 255);
      if (crypto_sign_open(t,&tlen,c,clen,p) == 0)
        if ((tlen != mlen) || (memcmp(t,m,mlen) != 0))
          fail("failure: crypto_sign_open allows trivial forgeries\n");
      c[myrandom() % clen] += 1 + (myrandom() % 255);
      if (crypto_sign_open(t,&tlen,c,clen,p) == 0)
        if ((tlen != mlen) || (memcmp(t,m,mlen) != 0))
          fail("failure: crypto_sign_open allows trivial forgeries\n");
      c[myrandom() % clen] += 1 + (myrandom() % 255);
      if (crypto_sign_open(t,&tlen,c,clen,p) == 0)
        if ((tlen != mlen) || (memcmp(t,m,mlen) != 0))
          fail("failure: crypto_sign_open allows trivial forgeries\n");
    }
    checksum_expected(sign_ed25519_checksums[checksumbig]);
  }
}

static void test_sign_ed25519(void)
{
  if (targeto && strcmp(targeto,"sign")) return;
  if (targetp && strcmp(targetp,"ed25519")) return;
  test_sign_ed25519_p = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_s = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_m = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_c = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_t = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_p2 = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_s2 = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_m2 = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_c2 = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);
  test_sign_ed25519_t2 = alignedcalloc(4096+crypto_sign_BYTES+crypto_sign_PUBLICKEYBYTES+crypto_sign_SECRETKEYBYTES);

  for (long long offset = 0;offset < 2;++offset) {
    printf("sign_ed25519 offset %lld\n",offset);
    for (long long impl = -1;impl < lib25519_numimpl_sign_ed25519();++impl)
      forked(test_sign_ed25519_impl,impl);
    ++test_sign_ed25519_p;
    ++test_sign_ed25519_s;
    ++test_sign_ed25519_m;
    ++test_sign_ed25519_c;
    ++test_sign_ed25519_t;
    ++test_sign_ed25519_p2;
    ++test_sign_ed25519_s2;
    ++test_sign_ed25519_m2;
    ++test_sign_ed25519_c2;
    ++test_sign_ed25519_t2;
  }
}
#undef crypto_sign_SECRETKEYBYTES
#undef crypto_sign_PUBLICKEYBYTES
#undef crypto_sign_BYTES

/* ----- top level */

#include "print_cpuid.inc"

int main(int argc,char **argv)
{
  setvbuf(stdout,0,_IOLBF,0);
  kernelrandombytes_setup();
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

  test_verify();
  test_hashblocks_sha512();
  test_hash_sha512();
  test_pow_inv25519();
  test_nP_montgomery25519();
  test_nG_merged25519();
  test_nG_montgomery25519();
  test_mGnP_ed25519();
  test_dh_x25519();
  test_sign_ed25519();

  if (!ok) {
    printf("some tests failed\n");
    return 100;
  }
  printf("all tests succeeded\n");
  return 0;
}
