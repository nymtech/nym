Here is how libcpucycles decides which cycle counter to use. The
underlying principles are as follows:

* Failure is not allowed. Using a low-resolution timer such as
  `gettimeofday()` to estimate cycle counts is not desirable but is better
  than providing no information.

* A counter that does well on some CPUs and OSes can do badly on others.
  The counter selection in libcpucycles is based not just on rules set
  at compile time but also on measurements of how well the counters
  perform when the program first calls `cpucycles()`.

* A critical application of cycle counting is collecting cycle counts
  for multiple options to see which option is faster. It is the caller's
  responsibility to compute medians of cycle counts for many runs of
  whatever is being benchmarked: medians filter out occasional
  cycle-count jumps caused by migration to another core (if the
  benchmark is not pinned to a single core) or interrupts from other OS
  activity. libcpucycles does not reject an otherwise attractive counter
  merely because of occasional jumps.

* Cycle-counting overhead is not desirable, but does not directly affect
  comparisons of multiple options measured using the same cycle counter,
  so it is less important than consistent major errors such as treating
  2^32 + x cycles as x cycles. (Performance experts seeing a function
  that takes billions of cycles usually focus on smaller subroutines,
  but libcpucycles should not break larger measurements.) This is why
  libcpucycles does not provide direct access to 32-bit cycle counters:
  it provides wrappers that combine the counters with gettimeofday() to
  produce 64 bits, even though this incurs some extra overhead.

* The noise introduced by typical off-core clocks, such as multiplying a
  24MHz clock by 86 to estimate cycles on a 2.064GHz CPU core, comes in
  small part from low resolution but much more from changes in CPU
  frequency: e.g., a 10000-cycle computation might be measured as 20000
  cycles when the CPU enters a power-saving mode. When libcpucycles has
  access to what is believed to be an on-core cycle counter, it uses
  that even when its measurements show some noise. (Choosing an on-core
  cycle counter does not magically eliminate the change in the relative
  speed of the CPU and DRAM; the usual advice to warm up the CPU and set
  constant frequencies if possible still applies.)

When `cpucycles()` is first called, libcpucycles tries running each
cycle counter that has been compiled into the library. For example, for
64-bit ARM CPUs, libcpucycles will try `arm64-pmc`, `arm64-vct`,
`default-gettimeofday`, `default-mach`, `default-monotonic`, and
`default-perfevent`, minus any of those that failed to compile.

Cycle counters that fail at run time with SIGILL (or SIGFPE or SIGBUS or
SIGSEGV) are eliminated from the list. For example, `arm64-pmc` will
fail with SIGILL if the kernel does not allow user access to
`PMCCNTR_EL0`. Beware that libcpucycles does not catch SIGILL after its
initial tests: if the kernel initially allows user access to
`PMCCNTR_EL0` but later turns it off then `arm64-pmc` will crash.

Independently of these counters, libcpucycles uses various OS mechanisms
to obtain an _estimate_ of the CPU frequency. This estimate is also
available to the caller as `cpucycles_persecond()`.

The methods that libcpucycles uses to ask the OS for an estimated CPU
frequency fail on some OS-CPU combinations, in which case libcpucycles
falls back to a `cpucyclespersecond` environment variable, or, if that
variable does not exist, an estimate of 2399987654 cycles per second.
(This estimate is in a realistic range of CPU speeds, and is close to
multiples of 24MHz, 25MHz, and 19.2MHz, which are common crystal
frequencies.) The sysadmin can create `/etc/cpucyclespersecond` to
override all of the OS mechanisms.

For counters that do not ask for scaling, the estimated CPU frequency is
shown in `cpucycles-info` as a double-check on the counter results. For
counters that ask for scaling, libcpucycles uses the estimated CPU
frequency to compute the scaling, so this is not a double-check. If a
counter asks for scaling and the estimated CPU frequency does not seem
close to a multiple of the counter frequency (possibly with a small
power-of-2 denominator) then libcpucycles will throw the counter away,
except in the case of fixed-resolution OS counters such as
`gettimeofday` and `CLOCK_MONOTONIC`.

libcpucycles computes a precision estimate for each counter (times any
applicable scaling) as follows. Call the counter 1000 times. Check that
the counter has never decreased, and has increased at least once. (A
counter where the decrease/increase checks fail is retried 10 times, so
10000 calls overall, and removed if it fails all 10 times.) The
precision estimate is then the smallest nonzero difference between
adjacent counter results, plus a penalty explained below.

The penalty is 100 cycles for off-core counters (including RDTSC) and
`default-perfevent`, and 200 cycles for fixed-resolution OS counters.
For example, an on-core CPU cycle counter will be selected even if it
actually has, e.g., a resolution of 8 cycles and 50 cycles of overhead.

Finally, libcpucycles selects the counter where the precision estimate
is the smallest number of cycles. Note that an inaccurate estimate of
CPU frequency can influence the choice between a scaled counter and an
unscaled counter.

libcpucycles does _not_ carry out its counter selection (typically tens
of milliseconds, sometimes even more) as a static initializer; callers
are presumed to not want to incur the cost of initialization unless and
until they are actually using `cpucycles()`. A multithreaded caller thus
has to place locks around any possibly-first call to `cpucycles()`, or
create its own static initializer (an `__attribute__((constructor))`
function) with an initial `cpucycles()` call so that all subsequent
`cpucycles()` calls are thread-safe.
