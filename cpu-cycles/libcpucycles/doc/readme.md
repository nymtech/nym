libcpucycles is a microlibrary for counting CPU cycles.
Cycle counts are not as detailed as
[Falk diagrams](https://gamozolabs.github.io/metrology/2019/08/19/sushi_roll.html)
but are the most precise timers available to typical software; they are
central tools used in understanding and improving software performance.

The libcpucycles [API](api.html) is simple: include `<cpucycles.h>`, call
`cpucycles()` to receive a `long long` whenever desired, and link with
`-lcpucycles`.

[Internally](counters.html), libcpucycles understands machine-level
cycle counters for amd64 (both PMC and TSC), arm32, arm64 (both PMC and
VCT), mips64, ppc32, ppc64, riscv32, riscv64, s390x, sparc64, and x86.
libcpucycles also understands four OS-level mechanisms, which give
varying levels of accuracy: `mach_absolute_time`, `perf_event`,
`CLOCK_MONOTONIC`, and, as a fallback, microsecond-resolution
`gettimeofday`.

When the program first calls `cpucycles()`, libcpucycles automatically
benchmarks the available mechanisms and [selects](selection.html) the
mechanism that does the best job. Subsequent `cpucycles()` calls are
thread-safe and very fast. An accompanying `cpucycles-info` program
prints a summary of cycle-counter accuracy.

For comparison, there is a simple-sounding `__rdtsc()` API provided by
compilers, but this works only on Intel/AMD CPUs and is generally noisier
than PMC. There is a `__builtin_readcyclecounter()` that works on more
CPUs, but this works only with `clang` and has the same noise problems.
Both of these mechanisms put the burden on the caller to figure out what
can be done on other CPUs. Various packages include their own more
portable abstraction layers for counting cycles (see, e.g., FFTW's
[`cycle.h`](https://github.com/FFTW/fftw3/blob/master/kernel/cycle.h),
used to automatically select from among multiple implementations
provided by FFTW), but this creates per-package effort to keep up with
the latest cycle counters. The goal of libcpucycles is to provide
state-of-the-art cycle counting centrally for all packages to use.
