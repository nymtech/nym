Currently libcpucycles supports the following cycle counters. Some
cycle counters are actually other forms of counters that libcpucycles
scales to imitate a cycle counter. There is
[separate documentation](selection.html)
for how libcpucycles makes a choice of cycle counter. See also
[security considerations](security.html) regarding enabling or disabling
counters and regarding Turbo Boost.

`amd64-pmc`: Requires a 64-bit Intel/AMD platform. Requires the Linux
perf_event interface. Accesses a cycle counter through RDPMC. Requires
`/proc/sys/kernel/perf_event_paranoid` to be at most 2 for user-level
RDPMC access. This counter runs at the clock frequency of the CPU core.

`amd64-tsc`, `amd64-tscasm`: Requires a 64-bit Intel/AMD platform.
Requires RDTSC to be enabled, which it is by default. Uses RDTSC to
access the CPU's time-stamp counter. On current CPUs, this is an
off-core clock rather than a cycle counter, but it is typically a very
fast off-core clock, making it adequate for seeing cycle counts if
overclocking and underclocking are disabled. The difference between
`tsc` and `tscasm` is that `tsc` uses the compiler's `__rdtsc()` while
`tscasm` uses inline assembly.

`arm32-cortex`: Requires a 32-bit ARMv7-A platform. Uses
`mrc p15, 0, %0, c9, c13, 0` to read the cycle counter. Requires user
access to the cycle counter, which is not enabled by default but can be
enabled under Linux via
[a kernel module](https://github.com/thoughtpolice/enable_arm_pmu).
This counter is natively 32 bits, but libcpucycles watches how the
counter and `gettimeofday` increase to compute a 64-bit extension of the
counter.

`arm64-pmc`: Requires a 64-bit ARMv8-A platform. Uses
`mrs %0, PMCCNTR_EL0` to read the cycle counter. Requires user access
to the cycle counter, which is not enabled by default but can be enabled
under Linux via
[a kernel module](https://github.com/rdolbeau/enable_arm_pmu).

`arm64-vct`: Requires a 64-bit ARMv8-A platform. Uses
`mrs %0, CNTVCT_EL0` to read a "virtual count" timer. This is an
off-core clock, typically running at 24MHz. Results are scaled by
libcpucycles.

`mips64-cc`: Requires a 64-bit MIPS platform. (Maybe the same code would
also work as `mips32-cc`, but this has not been tested yet.) Uses RDHWR
to read the hardware cycle counter (hardware register 2 times a constant
scale factor in hardware register 3). This counter is natively 32 bits,
but libcpucycles watches how the counter and `gettimeofday` increase to
compute a 64-bit extension of the counter.

`ppc32-mftb`: Requires a 32-bit PowerPC platform. Uses `mftb` and
`mftbu` to read the "time base". This is an off-core clock, typically
running at 24MHz.

`ppc64-mftb`: Requires a 64-bit PowerPC platform. Uses `mftb` and
`mftbu` to read the "time base". This is an off-core clock, typically
running at 24MHz.

`riscv32-rdcycle`: Requires a 32-bit RISC-V platform. Uses `rdcycle`
and `rdcycleh` to read a cycle counter.

`riscv64-rdcycle`: Requires a 64-bit RISC-V platform. Uses `rdcycle`
to read a cycle counter.

`s390x-stckf`: Requires a 64-bit z/Architecture platform. Uses `stckf`
to read the TOD clock, which is documented to run at 4096MHz. On the
z15, this looks like a doubling of an off-core 2048MHz clock. Results
are scaled by libcpucycles.

`sparc64-rdtick`: Requires a 64-bit SPARC platform. Uses `rd %tick`
to read a cycle counter.

`x86-tsc`, `x86-tscasm`: Same as `amd64-tsc` and `amd64-tscasm`, but
for 32-bit Intel/AMD platforms instead of 64-bit Intel/AMD platforms.

`default-gettimeofday`: Reasonably portable. Resolution is limited to 1
microsecond. Results are scaled by libcpucycles.

`default-mach`: Requires an OS with `mach_absolute_time()`. Typically
runs at 24MHz. Results are scaled by libcpucycles.

`default-monotonic`: Requires `CLOCK_MONOTONIC`. Reasonably portable,
although might fail on older systems where `default-gettimeofday` works.
Resolution is limited to 1 nanosecond. Can be almost as good as a cycle
counter, or orders of magnitude worse, depending on the OS and CPU.
Results are scaled by libcpucycles.

`default-perfevent`: Requires the Linux `perf_event` interface, and a
CPU where `perf_event` supports `PERF_COUNT_HW_CPU_CYCLES`. Similar
variations in quality to `default-monotonic`, without the 1-nanosecond
limitation.

`default-zero`: The horrifying last resort if nothing else works.

## Examples

These are examples of `cpucycles-info` output on various machines. The
machines named `gcc*` are from the
[GCC Compile Farm](https://gcc.gnu.org/wiki/CompileFarm).

A `median` line saying, e.g., `47 +47+28+0+2-5+0+2-5...` means that the
differences between adjacent cycle counts were 47+47, 47+28, 47+0, 47+2,
47−5, 47+0, 47+2, 47−5, etc., with median difference 47. The first few
differences are typically larger because of cache effects.

`pi3aplus`,
Broadcom BCM2837B0:
```
cpucycles version 20230105
cpucycles tracesetup 0 arm64-pmc precision 9 scaling 1.000000 only32 0
cpucycles tracesetup 1 arm64-vct precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 2 default-perfevent precision 189 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 272 scaling 1.400000 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 1600 scaling 1400.000000 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 1400000000
cpucycles implementation arm64-pmc
cpucycles median 10 +10+8+3+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0
cpucycles observed persecond 1032000000...4224666667 with 1024 loops 4 microseconds
cpucycles observed persecond 1286000000...1756000000 with 2048 loops 7 microseconds
cpucycles observed persecond 1368266666...1598000000 with 4096 loops 14 microseconds
cpucycles observed persecond 1366700000...1473428572 with 8192 loops 29 microseconds
cpucycles observed persecond 1366100000...1417534483 with 16384 loops 59 microseconds
cpucycles observed persecond 1332739837...1357132232 with 32768 loops 122 microseconds
cpucycles observed persecond 1354483471...1366945834 with 65536 loops 241 microseconds
cpucycles observed persecond 1385684989...1392195330 with 131072 loops 472 microseconds
cpucycles observed persecond 1347223021...1350328528 with 262144 loops 972 microseconds
cpucycles observed persecond 1375460125...1377069853 with 524288 loops 1905 microseconds
cpucycles observed persecond 1376527697...1377335961 with 1048576 loops 3808 microseconds
```

`bblack`,
TI Sitara XAM3359AZCZ100:
```
cpucycles version 20230105
cpucycles tracesetup 0 arm32-cortex precision 8 scaling 1.000000 only32 1
cpucycles tracesetup 1 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 1283 scaling 1.000000 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 1200 scaling 1000.000000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 1000000000
cpucycles implementation arm32-cortex
cpucycles median 1260 +1506+62+31+7+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+13+7+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0
cpucycles observed persecond 622181818...2101888889 with 1024 loops 10 microseconds
cpucycles observed persecond 806133333...1492615385 with 2048 loops 14 microseconds
cpucycles observed persecond 879880000...1232565218 with 4096 loops 24 microseconds
cpucycles observed persecond 939577777...1130581396 with 8192 loops 44 microseconds
cpucycles observed persecond 956954022...1050047059 with 16384 loops 86 microseconds
cpucycles observed persecond 982878542...1020685715 with 32768 loops 246 microseconds
cpucycles observed persecond 988105105...1012217523 with 65536 loops 332 microseconds
cpucycles observed persecond 993752077...1007159723 with 131072 loops 721 microseconds
cpucycles observed persecond 995364296...1004009448 with 262144 loops 1377 microseconds
cpucycles observed persecond 998216306...1001821536 with 524288 loops 2685 microseconds
cpucycles observed persecond 998991848...1000914196 with 1048576 loops 5397 microseconds
```

`hiphop`,
Intel Xeon E3-1220 v3:
```
cpucycles version 20230105
cpucycles tracesetup 0 amd64-pmc precision 40 scaling 1.000000 only32 0
cpucycles tracesetup 1 amd64-tsc precision 124 scaling 1.000000 only32 0
cpucycles tracesetup 2 amd64-tscasm precision 124 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-perfevent precision 160 scaling 1.000000 only32 0
cpucycles tracesetup 4 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 5 default-monotonic precision 272 scaling 3.100000 only32 0
cpucycles tracesetup 6 default-gettimeofday precision 3300 scaling 3100.000000 only32 0
cpucycles tracesetup 7 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3100000000
cpucycles implementation amd64-pmc
cpucycles median 44 +38+23+23+23-4+0-4+0-4+0-4+0+10-4-2+1-4+1-4+1+17+1-4+1-4+1-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4+0-4
cpucycles observed persecond 2066500000...4235000000 with 8192 loops 3 microseconds
cpucycles observed persecond 2760833333...4200250000 with 16384 loops 5 microseconds
cpucycles observed persecond 2743416666...3313100000 with 32768 loops 11 microseconds
cpucycles observed persecond 2986227272...3295000000 with 65536 loops 21 microseconds
cpucycles observed persecond 3052069767...3206073171 with 131072 loops 42 microseconds
cpucycles observed persecond 3050395348...3125523810 with 262144 loops 85 microseconds
cpucycles observed persecond 3085123529...3123059524 with 524288 loops 169 microseconds
cpucycles observed persecond 3084561764...3103434912 with 1048576 loops 339 microseconds
```

`nucnuc`,
Intel Pentium N3700:
```
cpucycles version 20230105
cpucycles tracesetup 0 amd64-pmc precision 26 scaling 1.000000 only32 0
cpucycles tracesetup 1 amd64-tsc precision 120 scaling 1.000000 only32 0
cpucycles tracesetup 2 amd64-tscasm precision 120 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-perfevent precision 427 scaling 1.000000 only32 0
cpucycles tracesetup 4 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 5 default-monotonic precision 320 scaling 1.600000 only32 0
cpucycles tracesetup 6 default-gettimeofday precision 1800 scaling 1600.000000 only32 0
cpucycles tracesetup 7 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 1600000000
cpucycles implementation amd64-pmc
cpucycles median 66 +12+12+14+14-1-1+0-1+0-1+0-1+0+1-1+0-1+0-1+0-2+0-1+0-1+0-1+0-2+0-1+0-1+0-1+0-2+0-1+0-1+1-1+0-2-1-1+0-1+0-1+0-2+0-1+2+0-1+0-1+0+0-1
cpucycles observed persecond 1060500000...2325000000 with 2048 loops 3 microseconds
cpucycles observed persecond 1387166666...2208250000 with 4096 loops 5 microseconds
cpucycles observed persecond 1376083333...1705500000 with 8192 loops 11 microseconds
cpucycles observed persecond 1495727272...1671800000 with 16384 loops 21 microseconds
cpucycles observed persecond 1563428571...1655100000 with 32768 loops 41 microseconds
cpucycles observed persecond 1580807228...1626234568 with 65536 loops 82 microseconds
cpucycles observed persecond 1589539393...1612619632 with 131072 loops 164 microseconds
cpucycles observed persecond 1598841463...1610230062 with 262144 loops 327 microseconds
cpucycles observed persecond 1564336810...1569988042 with 524288 loops 670 microseconds
cpucycles observed persecond 1599759725...1602608098 with 1048576 loops 1310 microseconds
```

`saber214`,
AMD FX-8350:
```
cpucycles version 20230105
cpucycles tracesetup 0 amd64-pmc precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 1 amd64-tsc precision 167 scaling 1.000000 only32 0
cpucycles tracesetup 2 amd64-tscasm precision 168 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 5 default-monotonic precision 376 scaling 4.013452 only32 0
cpucycles tracesetup 6 default-gettimeofday precision 4213 scaling 4013.452000 only32 0
cpucycles tracesetup 7 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 4013452000
cpucycles implementation amd64-tsc
cpucycles median 77 +87-2+21+7+4+1+0+2-2-7-4+0+1+4-2+3+1-2-2+5-6+2+2+2+2+1-1-1+0-4+0-1-1-1-2+3-1-1+2-2+0+0+2+0+0+2-2-2+1-1-2+2-5+2+0+2+0+1+0+3-2-1-1
cpucycles observed persecond 2767500000...5759000000 with 4096 loops 3 microseconds
cpucycles observed persecond 3426000000...4893800000 with 8192 loops 6 microseconds
cpucycles observed persecond 3724076923...4446363637 with 16384 loops 12 microseconds
cpucycles observed persecond 3977833333...4363318182 with 32768 loops 23 microseconds
cpucycles observed persecond 3984854166...4168739131 with 65536 loops 47 microseconds
cpucycles observed persecond 3981709923...4048193799 with 131072 loops 130 microseconds
cpucycles observed persecond 3982716417...4026914573 with 262144 loops 200 microseconds
cpucycles observed persecond 4001637602...4025136987 with 524288 loops 366 microseconds
cpucycles observed persecond 4007411111...4018600248 with 1048576 loops 809 microseconds
```

`gcc14`,
Intel Xeon E5-2620 v3,
Debian testing (bookworm),
Linux kernel 6.0.0-6-amd64:
```
cpucycles version 20230105
cpucycles tracesetup 0 amd64-pmc precision 41 scaling 1.000000 only32 0
cpucycles tracesetup 1 amd64-tsc precision 148 scaling 1.000000 only32 0
cpucycles tracesetup 2 amd64-tscasm precision 148 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-perfevent precision 159 scaling 1.000000 only32 0
cpucycles tracesetup 4 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 5 default-monotonic precision 289 scaling 3.200000 only32 0
cpucycles tracesetup 6 default-gettimeofday precision 3400 scaling 3200.000000 only32 0
cpucycles tracesetup 7 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3200000000
cpucycles implementation amd64-pmc
cpucycles median 47 +47+28+0+2-5+0+2-5+16+2-5+0+2-5+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0+1-4+0
cpucycles observed persecond 1653800000...2819333334 with 8192 loops 4 microseconds
cpucycles observed persecond 1832111111...2389285715 with 16384 loops 8 microseconds
cpucycles observed persecond 1936058823...2207200000 with 32768 loops 16 microseconds
cpucycles observed persecond 2052843750...2196200000 with 65536 loops 31 microseconds
cpucycles observed persecond 2050750000...2120048388 with 131072 loops 63 microseconds
cpucycles observed persecond 2081896825...2117048388 with 262144 loops 125 microseconds
cpucycles observed persecond 2089478087...2107044177 with 524288 loops 250 microseconds
cpucycles observed persecond 2093343313...2102124249 with 1048576 loops 500 microseconds
```

`gcc23`,
Cavium Octeon II V0.1,
Debian 8.11,
Linux kernel 4.1.4:
```
cpucycles version 20230105
cpucycles tracesetup 0 mips64-cc precision 24 scaling 1.000000 only32 1
cpucycles tracesetup 1 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 46702 scaling 2.399988 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 45799 scaling 2399.987654 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2399987654
cpucycles implementation mips64-cc
cpucycles median 2177 +828+17+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0
cpucycles observed persecond 641900000...1845125000 with 1024 loops 9 microseconds
cpucycles observed persecond 745357142...1352083334 with 2048 loops 13 microseconds
cpucycles observed persecond 809826086...1162333334 with 4096 loops 22 microseconds
cpucycles observed persecond 897717948...1104405406 with 8192 loops 38 microseconds
cpucycles observed persecond 957467532...1059986667 with 16384 loops 76 microseconds
cpucycles observed persecond 973102189...1029777778 with 32768 loops 136 microseconds
cpucycles observed persecond 986518656...1015830828 with 65536 loops 267 microseconds
cpucycles observed persecond 993452830...1008166667 with 131072 loops 529 microseconds
cpucycles observed persecond 996036966...1003403609 with 262144 loops 1054 microseconds
cpucycles observed persecond 984706378...1001682630 with 524288 loops 2131 microseconds
cpucycles observed persecond 992585292...1001178580 with 1048576 loops 4296 microseconds
```

`gcc45`,
AMD Athlon II X4 640,
Debian 8.11,
Linux kernel 3.16.0-11-686-pae:
```
cpucycles version 20230105
cpucycles tracesetup 0 x86-tsc precision 199 scaling 1.000000 only32 0
cpucycles tracesetup 1 x86-tscasm precision 199 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-perfevent precision 170 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 941 scaling 3.000000 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 3200 scaling 3000.000000 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3000000000
cpucycles implementation default-perfevent
cpucycles median 72 +12+0+0+0+0+0+0+0+5+0+0+0+0+0+0+0+2+0+0+0+0+0+0+0+1+0+0+0+0+0+0+0+2+0+0+0+0+0+0+0+1+0+0+0+0+0+0+0+2+0+0+0+0+0+0+0+1+0+0+0+0+0+0
cpucycles observed persecond 541500000...1812000000 with 1024 loops 3 microseconds
cpucycles observed persecond 712333333...1212250000 with 2048 loops 5 microseconds
cpucycles observed persecond 1193285714...1733600000 with 4096 loops 6 microseconds
cpucycles observed persecond 1689176470...1804562500 with 8192 loops 33 microseconds
cpucycles observed persecond 1713074626...1770600000 with 16384 loops 66 microseconds
cpucycles observed persecond 1765107692...1795140625 with 32768 loops 129 microseconds
cpucycles observed persecond 1785369649...1800603922 with 65536 loops 256 microseconds
cpucycles observed persecond 1781377862...1796288462 with 131072 loops 261 microseconds
cpucycles observed persecond 1772647398...1778247827 with 262144 loops 691 microseconds
cpucycles observed persecond 1789670493...1794149598 with 524288 loops 870 microseconds
cpucycles observed persecond 1860276211...1861561332 with 1048576 loops 3156 microseconds
```

`gcc92`,
SiFive Freedom U740,
Ubuntu 22.04,
Linux kernel 5.15.0-1014-generic:
```
cpucycles version 20230105
cpucycles tracesetup 0 riscv64-rdcycle precision 8 scaling 1.000000 only32 0
cpucycles tracesetup 1 default-perfevent precision 3024 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 2599 scaling 2.399988 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 2599 scaling 2399.987654 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2399987654
cpucycles implementation riscv64-rdcycle
cpucycles median 8 +33+27+1+1+1+1+0+0+0+22+0+0+0+0+0+0+0+628+0+0+0+7+0+0+0+145+0+0+0+0+0+0+0+22+0+0+0+0+0+0+0+158+0+0+0+0+0+0+0+22+0+0+0+0+0+0+0+22+0+0+0+0+0
cpucycles observed persecond 530250000...1978000000 with 1024 loops 3 microseconds
cpucycles observed persecond 831000000...1915666667 with 2048 loops 4 microseconds
cpucycles observed persecond 1055750000...1689500000 with 4096 loops 7 microseconds
cpucycles observed persecond 1045562500...1305428572 with 8192 loops 15 microseconds
cpucycles observed persecond 1102700000...1236357143 with 16384 loops 29 microseconds
cpucycles observed persecond 1176053571...1247444445 with 32768 loops 55 microseconds
cpucycles observed persecond 1173321428...1209127273 with 65536 loops 111 microseconds
cpucycles observed persecond 1187805429...1205210046 with 131072 loops 220 microseconds
cpucycles observed persecond 1192415909...1201157535 with 262144 loops 439 microseconds
cpucycles observed persecond 1194694760...1199247717 with 524288 loops 877 microseconds
cpucycles observed persecond 1194656004...1197023034 with 1048576 loops 1781 microseconds
```

`gcc103`,
Apple M1 (Icestorm-M1 + Firestorm-M1),
Debian unstable (bookworm),
Linux kernel 6.0.0-rc5-asahi-00001-gc62bd3fe430f:
```
cpucycles version 20230105
cpucycles tracesetup 0 arm64-pmc precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 1 arm64-vct precision 186 scaling 86.000000 only32 0
cpucycles tracesetup 2 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 285 scaling 2.064000 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 2264 scaling 2064.000000 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2064000000
cpucycles implementation arm64-vct
cpucycles median 0 +0+86+0+0+0+0+0+0+0+0+0+0+0+0+86+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+86+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+86+0+0+0+0+0+0+0+0
cpucycles observed persecond 1784500000...3655000000 with 8192 loops 3 microseconds
cpucycles observed persecond 1773750000...2393666667 with 16384 loops 7 microseconds
cpucycles observed persecond 1897733333...2222769231 with 32768 loops 14 microseconds
cpucycles observed persecond 1951310344...2114962963 with 65536 loops 28 microseconds
cpucycles observed persecond 2024071428...2107000000 with 131072 loops 55 microseconds
cpucycles observed persecond 2041531531...2082935780 with 262144 loops 110 microseconds
cpucycles observed persecond 2051158371...2071461188 with 524288 loops 220 microseconds
cpucycles observed persecond 2058539682...2068309795 with 1048576 loops 440 microseconds
```

`gcc112` (`gcc2-power8`),
IBM POWER8E,
CentOS 7.9 AltArch,
Linux kernel 3.10.0-1127.13.1.el7.ppc64le:
```
cpucycles version 20230105
cpucycles tracesetup 0 ppc64-mftb precision 251 scaling 7.207031 only32 0
cpucycles tracesetup 1 default-perfevent precision 295 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 536 scaling 3.690000 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 3890 scaling 3690.000000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3690000000
cpucycles implementation ppc64-mftb
cpucycles median 195 +2969-8+14+0-8+7-8-7+7+6-7-1+0-1+0+7+7-15+7-1-7+6+0+0-8+0+6+0-8+7+0+7-8-8-7-1+7-8+7+0-8+0+14-8-7+6+0-8+7+7-15+0-1+0-1+14+0-15+14+0-1+7+0
cpucycles observed persecond 2603750000...5510000000 with 2048 loops 3 microseconds
cpucycles observed persecond 3430500000...6052250000 with 4096 loops 5 microseconds
cpucycles observed persecond 3411333333...4457500000 with 8192 loops 11 microseconds
cpucycles observed persecond 3548695652...4060333334 with 16384 loops 22 microseconds
cpucycles observed persecond 3624977777...3876534884 with 32768 loops 44 microseconds
cpucycles observed persecond 3621855555...3745363637 with 65536 loops 89 microseconds
cpucycles observed persecond 3660157303...3722227273 with 131072 loops 177 microseconds
cpucycles observed persecond 3680471751...3711622160 with 262144 loops 353 microseconds
cpucycles observed persecond 3685321074...3700886525 with 524288 loops 706 microseconds
cpucycles observed persecond 3687745930...3695537208 with 1048576 loops 1412 microseconds
```

`gcc202`,
UltraSparc T5,
Debian unstable (bookworm),
Linux kernel 5.19.0-2-sparc64-smp:
```
cpucycles version 20230105
cpucycles tracesetup 0 sparc64-rdtick precision 65 scaling 1.000000 only32 0
cpucycles tracesetup 1 default-perfevent precision 386 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 442 scaling 3.599910 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 3799 scaling 3599.910000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3599910000
cpucycles implementation sparc64-rdtick
cpucycles median 73 +24+0+24+24+24+24+24+24+0+1+24+0+1+24+0+1+24+0+0+1+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+1+0+0+0+0+0+0+0+0+0+0+0+0+0
cpucycles observed persecond 2751500000...4258250000 with 4096 loops 5 microseconds
cpucycles observed persecond 3289200000...4206875000 with 8192 loops 9 microseconds
cpucycles observed persecond 3454789473...3900823530 with 16384 loops 18 microseconds
cpucycles observed persecond 3452026315...3659888889 with 32768 loops 37 microseconds
cpucycles observed persecond 3543770270...3650916667 with 65536 loops 73 microseconds
cpucycles observed persecond 3567299319...3620662069 with 131072 loops 146 microseconds
cpucycles observed persecond 3591373287...3618220690 with 262144 loops 291 microseconds
cpucycles observed persecond 3597353344...3610774527 with 524288 loops 582 microseconds
cpucycles observed persecond 3595899403...3603058071 with 1048576 loops 1172 microseconds
```

IBM z15:
```
cpucycles version 20230106
cpucycles tracesetup 0 s390x-stckf precision 250 scaling 1.269531 only32 0
cpucycles tracesetup 1 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 272 scaling 5.200000 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 5400 scaling 5200.000000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 5200000000
cpucycles implementation s390x-stckf
cpucycles median 48 +87+8+0-2+0+0+38-2+0+1-3+1+28+0+3-3+1+0+28+0-2+3+0-2+36+0+0+0+1+0+28+0-2+0+3-2+35+1+0-2+0+3+28+0-2+0+0-2+3+25+3+0-2+0+1+35+1+0+0-2+0+28+0
cpucycles observed persecond 4948941176...5627733334 with 8192 loops 16 microseconds
cpucycles observed persecond 4104125000...5515666667 with 16384 loops 7 microseconds
cpucycles observed persecond 5047076923...5987818182 with 32768 loops 12 microseconds
cpucycles observed persecond 5044846153...5475708334 with 65536 loops 25 microseconds
cpucycles observed persecond 5141313725...5357428572 with 131072 loops 50 microseconds
cpucycles observed persecond 5150892156...5257250000 with 262144 loops 101 microseconds
cpucycles observed persecond 5183421568...5236549505 with 524288 loops 203 microseconds
cpucycles observed persecond 5190282555...5216582717 with 1048576 loops 406 microseconds
```
