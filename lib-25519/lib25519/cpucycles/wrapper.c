#include <stdio.h>
#include <stdlib.h>
#include "cpucycles.h"

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

  return 0;
}

static long long persecond = 0;
const char *implementation = "none";

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

static double cpucycles_scaled_scaling = 0;
static long long (*cpucycles_scaled_from)(void) = 0;

static long long cpucycles_scaled(void)
{
  return cpucycles_scaled_from()*cpucycles_scaled_scaling;
}

#include "options.inc"

#define CALLS 1000

long long cpucycles_init(void)
{
  long long precision[NUMOPTIONS];
  long long scaling[NUMOPTIONS];
  long long bestprecision;
  long long bestopt;

  persecond = osfreq();

  for (long long opt = 0;opt < NUMOPTIONS;++opt) {
    long long freq = options[opt].ticks_setup();

    // freq > 0: freq ticks per second
    // freq == 0: do not use
    // freq == -1: cycle counter (e.g., rdpmc)
    // freq == -2: probably cycle counter (e.g., rdtsc)
    // freq == -3: tick counter every N cycles for some unknown N

    precision[opt] = 0;

    if (freq > 0) { // means: freq ticks per second
      scaling[opt] = persecond*1.0/freq;
    } else if (freq == -1) { // means: cycle counter; e.g., rdpmc
      scaling[opt] = 1.0;
    } else if (freq == -2) { // means: probably cycle counter; e.g., rdtsc
      scaling[opt] = 1.0;
    } else {
      continue;
    }

    for (long long tries = 0;tries < 10;++tries) {
      long long t[CALLS+1];
      long long ok = 1;

      if (scaling[opt] == 1.0) {
        for (long long i = 0;i <= CALLS;++i)
          t[i] = options[opt].ticks();
      } else {
        double scalingopt = scaling[opt];
        for (long long i = 0;i <= CALLS;++i)
          t[i] = options[opt].ticks()*scalingopt;
      }
      for (long long i = 0;i < CALLS;++i)
        if (t[i] > t[i+1])
          ok = 0;
      if (t[0] == t[CALLS])
        ok = 0;

      if (ok) {
        long long smallestdiff = 0;
        for (long long i = 0;i < CALLS;++i) {
          long long diff = t[i+1]-t[i];
          if (diff <= 0) continue;
          if (smallestdiff == 0 || diff < smallestdiff)
            smallestdiff = diff;
        }
        precision[opt] = smallestdiff;
        if (freq != -1)
          precision[opt] += 100;
        break;
      }

      // otherwise keep trying
      // since !ok can be caused by overflow
      // or by core swap
    }
  }

  bestopt = DEFAULTOPTION;
  bestprecision = 0;
  for (long long opt = 0;opt < NUMOPTIONS;++opt)
    if (precision[opt] > 0)
      if (!bestprecision || precision[opt] < bestprecision) {
        bestopt = opt;
        bestprecision = precision[opt];
      }

  implementation = options[bestopt].implementation;
  
  if (scaling[bestopt] == 1.0) {
    cpucycles = options[bestopt].ticks;
  } else {
    cpucycles_scaled_scaling = scaling[bestopt];
    cpucycles_scaled_from = options[bestopt].ticks;
    cpucycles = cpucycles_scaled;
  }

  return cpucycles();
}
