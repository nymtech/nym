// version 20240318
// public domain
// djb
// includes some pieces adapted from supercop

// 20240318 djb: loosen 0.1 to 0.2 for FINDMULTIPLIER
// 20230115 djb: cpucycles_version()
// 20230106 djb: support "cpu MHz static" (ibm z15)

#include <time.h>
#include <sys/time.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <inttypes.h>
#include <signal.h>
#include <setjmp.h>
#include "cpucycles.h"
#include "cpucycles_internal.h"

static int tracesetup = 0;

void cpucycles_tracesetup(void)
{
  tracesetup = 1;
}

static jmp_buf crash_jmp;

static void crash(int s)
{
  siglongjmp(crash_jmp,1);
}

int cpucycles_works(long long (*ticks)(void))
{
  volatile int result = 0;
  struct sigaction old_sigill;
  struct sigaction old_sigfpe;
  struct sigaction old_sigbus;
  struct sigaction old_sigsegv;
  struct sigaction crash_action;

  memset(&crash_action,0,sizeof crash_action);
  crash_action.sa_handler = crash;

  sigaction(SIGILL,0,&old_sigill);
  sigaction(SIGFPE,0,&old_sigfpe);
  sigaction(SIGBUS,0,&old_sigbus);
  sigaction(SIGSEGV,0,&old_sigsegv);

  if (!sigsetjmp(crash_jmp,1)) {
    sigaction(SIGILL,&crash_action,0);
    sigaction(SIGFPE,&crash_action,0);
    sigaction(SIGBUS,&crash_action,0);
    sigaction(SIGSEGV,&crash_action,0);
    ticks();
    result = 1;
  }

  sigaction(SIGILL,&old_sigill,0);
  sigaction(SIGFPE,&old_sigfpe,0);
  sigaction(SIGBUS,&old_sigbus,0);
  sigaction(SIGSEGV,&old_sigsegv,0);

  return result;
}

static double osfreq(void)
{
  FILE *f;
  char *x;
  double result;
  int s;

  f = fopen("/etc/cpucyclespersecond", "r");
  if (f) {
    s = fscanf(f,"%lf",&result);
    fclose(f);
    if (s > 0) return result;
  }

  f = fopen("/sys/devices/system/cpu/cpu0/cpufreq/scaling_setspeed", "r");
  if (f) {
    s = fscanf(f,"%lf",&result);
    fclose(f);
    if (s > 0) return 1000.0 * result;
  }

  f = fopen("/sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq", "r");
  if (f) {
    s = fscanf(f,"%lf",&result);
    fclose(f);
    if (s > 0) return 1000.0 * result;
  }

  f = fopen("/sys/devices/system/cpu/cpu0/clock_tick", "r");
  if (f) {
    s = fscanf(f,"%lf",&result);
    fclose(f);
    if (s > 0) return result;
  }

  f = fopen("/proc/cpuinfo","r");
  if (f) {
    for (;;) {
      s = fscanf(f,"cpu MHz : %lf",&result);
      if (s > 0) break;
      if (s == 0) s = fscanf(f,"%*[^\n]\n");
      if (s < 0) { result = 0; break; }
    }
    fclose(f);
    if (result) return 1000000.0 * result;
  }

  f = fopen("/proc/cpuinfo","r");
  if (f) {
    for (;;) {
      s = fscanf(f,"clock : %lf",&result);
      if (s > 0) break;
      if (s == 0) s = fscanf(f,"%*[^\n]\n");
      if (s < 0) { result = 0; break; }
    }
    fclose(f);
    if (result) return 1000000.0 * result;
  }

  f = fopen("/proc/cpuinfo","r");
  if (f) {
    for (;;) {
      s = fscanf(f,"cpu MHz static : %lf",&result);
      if (s > 0) break;
      if (s == 0) s = fscanf(f,"%*[^\n]\n");
      if (s < 0) { result = 0; break; }
    }
    fclose(f);
    if (result) return 1000000.0 * result;
  }

  f = popen("sysctl hw.cpufrequency 2>/dev/null","r");
  if (f) {
    s = fscanf(f,"hw.cpufrequency: %lf",&result);
    pclose(f);
    if (s > 0) if (result > 0) return result;
  }

  f = popen("/usr/sbin/lsattr -E -l proc0 -a frequency 2>/dev/null","r");
  if (f) {
    s = fscanf(f,"frequency %lf",&result);
    pclose(f);
    if (s > 0) return result;
  }

  f = popen("/usr/sbin/psrinfo -v 2>/dev/null","r");
  if (f) {
    for (;;) {
      s = fscanf(f," The %*s processor operates at %lf MHz",&result);
      if (s > 0) break;
      if (s == 0) s = fscanf(f,"%*[^\n]\n");
      if (s < 0) { result = 0; break; }
    }
    pclose(f);
    if (result) return 1000000.0 * result;
  }

  x = getenv("cpucyclespersecond");
  if (x) {
    s = sscanf(x,"%lf",&result);
    if (s > 0) return result;
  }

  return 2399987654.0;
}

static long long persecond = 0;
static const char *implementation = "none";

long long (*cpucycles)(void) = cpucycles_init;

const char *cpucycles_implementation(void)
{
  cpucycles();
  return implementation;
}

long long cpucycles_persecond(void)
{
  cpucycles();
  return persecond;
}

const char *cpucycles_version(void)
{
  return "20240318";
}

// ----- cycle counter scaled from ticks

static double cpucycles_scaled_scaling = 0;
static long long cpucycles_scaled_offset = 0;
static long long (*cpucycles_scaled_from)(void) = 0;

static long long cpucycles_scaled(void)
{
  return (cpucycles_scaled_from()-cpucycles_scaled_offset)*cpucycles_scaled_scaling;
}

// ----- cycle counter extended from 32-bit ticks

static long long (*cpucycles_extend32_from)(void) = 0;

static uint32_t cpucycles_extend32_prev_ticks;
static long long cpucycles_extend32_prev_us;
static long long cpucycles_extend32_prev_cycles;

static void cpucycles_extend32_setup(void)
{
  long long (*ticks)(void) = cpucycles_extend32_from;
  cpucycles_extend32_prev_ticks = ticks();
  cpucycles_extend32_prev_us = cpucycles_microseconds();
  cpucycles_extend32_prev_cycles = 0;
}

static long long cpucycles_extend32(void)
{
  long long (*ticks)(void) = cpucycles_extend32_from;

  uint32_t new_ticks = ticks();
  unsigned long long delta_ticks = new_ticks-cpucycles_extend32_prev_ticks;
  long long new_us = cpucycles_microseconds();
  long long delta_us = new_us-cpucycles_extend32_prev_us;

  // assume that number of cycles cannot increase by 2^32 in 2ms

  if (delta_us < 1000)
    return cpucycles_extend32_prev_cycles+delta_ticks;

  cpucycles_extend32_prev_ticks = new_ticks;
  cpucycles_extend32_prev_us = new_us;

  if (delta_us >= 2000) {
    long long target = (delta_us*0.000001)*persecond;
    while (delta_ticks+2147483648ULL < target)
      delta_ticks += 4294967296ULL;
  }

  return cpucycles_extend32_prev_cycles += delta_ticks;
}

// ----- estimating cycles per tick

long long cpucycles_microseconds(void)
{
  struct timeval t;
  long long result;
  gettimeofday(&t,(struct timezone *) 0);
  result = t.tv_sec;
  result *= 1000000;
  result += t.tv_usec;
  return result;
}

static double estimate_cyclespertick(long long (*ticks)(void))
{
  long long t0,t1,us0,us1;

  t0 = ticks();
  us0 = cpucycles_microseconds();
  do {
    t1 = ticks();
    us1 = cpucycles_microseconds();
  } while (us1-us0 < 10000 || t1-t0 < 1000);
  if (t1 <= t0) return 0;
  t1 -= t0;
  us1 -= us0;
  return (persecond * 0.000001 * (double) us1) / (double) t1;
}

// ----- selecting an option

#include "options.inc"

#define CALLS 1000
#define ESTIMATES 3

long long cpucycles_init(void)
{
  long long precision[NUMOPTIONS];
  double scaling[NUMOPTIONS];
  int only32[NUMOPTIONS];
  long long bestprecision;
  long long bestopt;
  long long opt;

  persecond = osfreq();

  for (opt = 0;opt < NUMOPTIONS;++opt) {
    long long freq = options[opt].ticks_setup();
    long long tries;

    precision[opt] = 0;
    scaling[opt] = 0;
    only32[opt] = 0;

    if (freq > 0) {
      scaling[opt] = persecond*1.0/freq;
    } else if (freq == cpucycles_CYCLECOUNTER) {
      scaling[opt] = 1.0;
    } else if (freq == cpucycles_EXTEND32) {
      only32[opt] = 1;
      scaling[opt] = 1.0;
    } else if (freq == cpucycles_MAYBECYCLECOUNTER) {
      scaling[opt] = 1.0;
    } else if (freq == cpucycles_FINDMULTIPLIER) {
      int ok = 0;
      double denom;
      long long loop;

      for (denom = 1;denom <= 1024;denom += denom) {
        double est[ESTIMATES];
        for (loop = 0;loop < ESTIMATES;++loop)
          est[loop] = denom*estimate_cyclespertick(options[opt].ticks);
        scaling[opt] = (double) (long long) est[0];
        if (scaling[opt] < est[0]-0.5) scaling[opt] += 1;
        if (scaling[opt] > est[0]+0.5) scaling[opt] -= 1;
        ok = 1;
        for (loop = 0;loop < ESTIMATES;++loop) {
          if (est[loop]-scaling[opt] > 0.2) ok = 0;
          if (scaling[opt]-est[loop] > 0.2) ok = 0;
        }
        if (ok) {
          scaling[opt] /= denom;
          break;
        }
        scaling[opt] = 0;
      }
      if (!ok) continue;
    } else {
      continue;
    }

    for (tries = 0;tries < 10;++tries) {
      long long t[CALLS+1];
      long long ok = 1;
      long long i;

      if (scaling[opt] == 1.0) {
        for (i = 0;i <= CALLS;++i)
          t[i] = options[opt].ticks();
      } else {
        double scalingopt = scaling[opt];
        long long offset = options[opt].ticks();
        for (i = 0;i <= CALLS;++i)
          t[i] = (options[opt].ticks()-offset)*scalingopt;
      }
      for (i = 0;i < CALLS;++i)
        if (t[i] > t[i+1])
          ok = 0;
      if (t[0] == t[CALLS])
        ok = 0;

      if (ok) {
        long long smallestdiff = 0;
        for (i = 0;i < CALLS;++i) {
          long long diff = t[i+1]-t[i];
          if (diff <= 0) continue;
          if (smallestdiff == 0 || diff < smallestdiff)
            smallestdiff = diff;
        }
        precision[opt] = smallestdiff;

        // tilt selection towards more robust counters
        if (freq != cpucycles_CYCLECOUNTER && freq != cpucycles_EXTEND32)
          precision[opt] += 100;
        if (freq > 0)
          precision[opt] += 100;

        break;
      }

      // otherwise keep trying
      // since !ok can be caused by overflow
      // or by core swap
    }
  }

  if (tracesetup) {
    for (opt = 0;opt < NUMOPTIONS;++opt)
      printf("cpucycles tracesetup %lld %s precision %lld scaling %lf only32 %d\n"
        ,opt,options[opt].implementation,precision[opt],scaling[opt],only32[opt]);
  }

  bestopt = DEFAULTOPTION;
  bestprecision = 0;
  for (opt = 0;opt < NUMOPTIONS;++opt)
    if (precision[opt] > 0)
      if (!bestprecision || precision[opt] < bestprecision) {
        bestopt = opt;
        bestprecision = precision[opt];
      }

  implementation = options[bestopt].implementation;
  
  if (scaling[bestopt] == 1.0) {
    if (only32[bestopt]) {
      cpucycles_extend32_from = options[bestopt].ticks;
      cpucycles_extend32_setup();
      cpucycles = cpucycles_extend32;
    } else {
      cpucycles = options[bestopt].ticks;
    }
  } else {
    cpucycles_scaled_scaling = scaling[bestopt];
    cpucycles_scaled_from = options[bestopt].ticks;
    cpucycles_scaled_offset = cpucycles_scaled_from();
    cpucycles = cpucycles_scaled;
  }

  return cpucycles();
}
