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

`arm32-1176`: Requires a 32-bit ARM1176 platform. Uses
`mrc p15, 0, %0, c15, c12, 1` to read the cycle counter. Requires user
access to the cycle counter, which is not enabled by default but can be
enabled under Linux via
[a kernel module](https://bench.cr.yp.to/cpucycles/n810.html).
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
machines named `cfarm*` are from the
[GCC Compile Farm](https://gcc.gnu.org/wiki/CompileFarm).

A `median` line saying, e.g., `47 +47+28+0+2-5+0+2-5...` means that the
differences between adjacent cycle counts were 47+47, 47+28, 47+0, 47+2,
47−5, 47+0, 47+2, 47−5, etc., with median difference 47. The first few
differences are typically larger because of cache effects.

`berry0`,
Broadcom BCM2835:
```
cpucycles version 20240114
cpucycles tracesetup 0 arm32-cortex precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 1 arm32-1176 precision 22 scaling 1.000000 only32 1
cpucycles tracesetup 2 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 1199 scaling 1.000000 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 1200 scaling 1000.000000 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 1000000000
cpucycles implementation arm32-1176
cpucycles median 720 +942+124+1+0+2+0+0+0+0+0+0+0+0+0+0+0+0+0+0+1+2+0+0+2+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+222+300+1+0+0+2+0+0+0+0+0+0+0+0+0+0+0+0+0
cpucycles observed persecond 798307692...2045181819 with 1024 loops 12 microseconds
cpucycles observed persecond 915478260...1260523810 with 2048 loops 22 microseconds
cpucycles observed persecond 947809523...1106100000 with 4096 loops 41 microseconds
cpucycles observed persecond 966353658...1129037500 with 8192 loops 81 microseconds
cpucycles observed persecond 988490566...1030019109 with 16384 loops 158 microseconds
cpucycles observed persecond 995169327...1002034063 with 32768 loops 2379 microseconds
cpucycles observed persecond 996871019...1012568691 with 65536 loops 627 microseconds
cpucycles observed persecond 997832134...1004212170 with 131072 loops 1250 microseconds
cpucycles observed persecond 997740918...1000887780 with 262144 loops 5009 microseconds
cpucycles observed persecond 998528349...1001961164 with 524288 loops 5537 microseconds
cpucycles observed persecond 999202882...1001166794 with 1048576 loops 10547 microseconds
```

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

`cfarm14`,
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

`cfarm23`,
Cavium Octeon II V0.1,
Debian 8.11,
Linux kernel 4.1.4:
```
cpucycles version 20240114
cpucycles tracesetup 0 mips64-cc precision 24 scaling 1.000000 only32 1
cpucycles tracesetup 1 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 46649 scaling 2.399988 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 45799 scaling 2399.987654 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2399987654
cpucycles implementation mips64-cc
cpucycles median 2206 +581+5+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+0+83791+581+18+18+18+18+18+18
cpucycles observed persecond 634500000...1843500000 with 1024 loops 9 microseconds
cpucycles observed persecond 746142857...1361500000 with 2048 loops 13 microseconds
cpucycles observed persecond 846318181...1222000000 with 4096 loops 21 microseconds
cpucycles observed persecond 897717948...1105432433 with 8192 loops 38 microseconds
cpucycles observed persecond 954521126...1065971015 with 16384 loops 70 microseconds
cpucycles observed persecond 979395454...1018958716 with 32768 loops 219 microseconds
cpucycles observed persecond 986875354...1011415955 with 65536 loops 352 microseconds
cpucycles observed persecond 994412144...1005722798 with 131072 loops 773 microseconds
cpucycles observed persecond 997076363...1003483613 with 262144 loops 1374 microseconds
cpucycles observed persecond 959310151...1001940950 with 524288 loops 2846 microseconds
cpucycles observed persecond 993951907...1000833365 with 1048576 loops 5426 microseconds
```

`cfarm26`,
Intel Core i5-4570 in 32-bit mode under KVM,
Debian 12.4,
Linux kernel 6.1.0-17-686-pae:
```
cpucycles version 20240114
cpucycles tracesetup 0 x86-tsc precision 118 scaling 1.000000 only32 0
cpucycles tracesetup 1 x86-tscasm precision 118 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-perfevent precision 627 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 2335 scaling 3.192606 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 3392 scaling 3192.606000 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3192606000
cpucycles implementation x86-tsc
cpucycles median 18 +34+0+0+13+0+0+13+0+0+13+0+0+13+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+0
cpucycles observed persecond 1950500000...6176000000 with 8192 loops 3 microseconds
cpucycles observed persecond 2591000000...5117000000 with 16384 loops 5 microseconds
cpucycles observed persecond 2824090909...4013333334 with 32768 loops 10 microseconds
cpucycles observed persecond 2993757575...3362258065 with 65536 loops 32 microseconds
cpucycles observed persecond 3093644067...3286807018 with 131072 loops 58 microseconds
cpucycles observed persecond 3126202531...3270727273 with 262144 loops 78 microseconds
cpucycles observed persecond 3144248407...3216322581 with 524288 loops 156 microseconds
cpucycles observed persecond 3172426332...3209545742 with 1048576 loops 318 microseconds
```

`cfarm27`,
Intel Core i5-4570 in 32-bit mode under KVM,
Alpine 3.19.0,
Linux kernel 6.6.7-0-lts:
```
cpucycles version 20240114
cpucycles tracesetup 0 x86-tsc precision 118 scaling 1.000000 only32 0
cpucycles tracesetup 1 x86-tscasm precision 118 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-perfevent precision 631 scaling 1.000000 only32 0
cpucycles tracesetup 3 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 1084 scaling 3.192606 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 3392 scaling 3192.606000 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3192606000
cpucycles implementation x86-tsc
cpucycles median 18 +113+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+0+13+0+13+0+0+13+0+0+13+0+0+13+0+0+13
cpucycles observed persecond 2404000000...4642666667 with 8192 loops 4 microseconds
cpucycles observed persecond 2617333333...4441250000 with 16384 loops 5 microseconds
cpucycles observed persecond 3001312500...3606857143 with 32768 loops 15 microseconds
cpucycles observed persecond 3096870967...3394000000 with 65536 loops 30 microseconds
cpucycles observed persecond 3123943661...3244913044 with 131072 loops 70 microseconds
cpucycles observed persecond 3173264150...3225305733 with 262144 loops 158 microseconds
cpucycles observed persecond 3170094339...3210561905 with 524288 loops 211 microseconds
cpucycles observed persecond 3178732087...3205529781 with 1048576 loops 320 microseconds
```

`cfarm29`,
IBM POWER9,
Debian 12.4,
Linux kernel 6.1.0-17-powerpc64le:
```
cpucycles version 20240114
cpucycles tracesetup 0 ppc64-mftb precision 218 scaling 7.421875 only32 0
cpucycles tracesetup 1 default-perfevent precision 292 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 355 scaling 3.800000 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 4000 scaling 3800.000000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3800000000
cpucycles implementation ppc64-mftb
cpucycles median 207 +52+31+1-14+0+1-14-6+8-7-6+8-7+1-7+1+8-21-7+1+16-22+1+8-21-7+16-7-21+1+8-21+0+1+1-14+8-6-14+0+9-7+1+1-14-14+8-7-21+1+8-7+1-7+9-22+8-6+1-14+8-7-6
cpucycles observed persecond 3267500000...6865000000 with 4096 loops 3 microseconds
cpucycles observed persecond 3246125000...4445666667 with 8192 loops 7 microseconds
cpucycles observed persecond 3435333333...4016307693 with 16384 loops 14 microseconds
cpucycles observed persecond 3674892857...3984115385 with 32768 loops 27 microseconds
cpucycles observed persecond 3734963636...3888641510 with 65536 loops 54 microseconds
cpucycles observed persecond 3768266055...3845158879 with 131072 loops 108 microseconds
cpucycles observed persecond 3783654377...3822125582 with 262144 loops 216 microseconds
cpucycles observed persecond 3791669745...3810830627 with 524288 loops 432 microseconds
cpucycles observed persecond 3795847398...3805719583 with 1048576 loops 864 microseconds
```

`cfarm45`,
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

`cfarm91`,
StarFive JH7100,
Linux trixie/sid,
Linux kernel 5.18.11-starfive:
```
cpucycles version 20240114
cpucycles tracesetup 0 riscv64-rdcycle precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 1 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 1351 scaling 2.399988 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 2599 scaling 2399.987654 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2399987654
cpucycles implementation default-monotonic
cpucycles median 1536 -384+0+384+0-384+0-384+0+0-384+384-1-384+0+0-381+0-384+384+0+0-384+0-384+0+0+384+0+0+0+0+0-384+0+384+0-384+0-384+0+0-384+384+0-384+0+0-384+0+0+0+0+0-384+0-382+0+0+384-384+0-384+0
cpucycles observed persecond 1590857142...4147200000 with 1024 loops 6 microseconds
cpucycles observed persecond 1954909090...3157333334 with 2048 loops 10 microseconds
cpucycles observed persecond 2142421052...2755882353 with 4096 loops 18 microseconds
cpucycles observed persecond 2293085714...2606606061 with 8192 loops 34 microseconds
cpucycles observed persecond 2337970588...2496090910 with 16384 loops 67 microseconds
cpucycles observed persecond 2358522388...2443712122 with 32768 loops 133 microseconds
cpucycles observed persecond 2382335849...2423813689 with 65536 loops 264 microseconds
cpucycles observed persecond 2385986013...2405815790 with 131072 loops 571 microseconds
cpucycles observed persecond 2395157522...2405531915 with 262144 loops 1129 microseconds
cpucycles observed persecond 2397798685...2402770560 with 524288 loops 2433 microseconds
cpucycles observed persecond 2398637218...2401114855 with 1048576 loops 4572 microseconds
```

`cfarm92`,
SiFive Freedom U740,
Ubuntu 22.04.3,
Linux kernel 5.19.0-1021-generic:
```
cpucycles version 20240114
cpucycles tracesetup 0 riscv64-rdcycle precision 8 scaling 1.000000 only32 0
cpucycles tracesetup 1 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 2599 scaling 2.399988 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 2599 scaling 2399.987654 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2399987654
cpucycles implementation riscv64-rdcycle
cpucycles median 8 +168+20+2+2+0+0+0+0+570+0+0+0+0+0+0+0+144+0+0+0+0+0+0+0+160+0+0+0+0+0+0+0+160+0+0+0+0+0+0+0+154+0+0+0+0+0+0+0+154+0+0+0+0+0+0+0+152+0+0+0+0+0+0
cpucycles observed persecond 571500000...2198000000 with 1024 loops 3 microseconds
cpucycles observed persecond 833600000...2094000000 with 2048 loops 4 microseconds
cpucycles observed persecond 921888888...1445142858 with 4096 loops 8 microseconds
cpucycles observed persecond 1029625000...1320642858 with 8192 loops 15 microseconds
cpucycles observed persecond 1137034482...1284481482 with 16384 loops 28 microseconds
cpucycles observed persecond 1155701754...1227454546 with 32768 loops 56 microseconds
cpucycles observed persecond 1177464285...1217163637 with 65536 loops 111 microseconds
cpucycles observed persecond 1188018099...1207858448 with 131072 loops 220 microseconds
cpucycles observed persecond 1189925170...1200519363 with 262144 loops 440 microseconds
cpucycles observed persecond 1193962457...1199117446 with 524288 loops 878 microseconds
cpucycles observed persecond 1194051324...1196780111 with 1048576 loops 1811 microseconds
```

`cfarm103`,
Apple M1 (Icestorm-M1 + Firestorm-M1),
Debian trixie/sid,
Linux kernel 6.5.0-asahi-00780-g62806c2c6f29:
```
cpucycles version 20240114
cpucycles tracesetup 0 arm64-pmc precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 1 arm64-vct precision 186 scaling 86.000000 only32 0
cpucycles tracesetup 2 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 285 scaling 2.064000 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 2264 scaling 2064.000000 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2064000000
cpucycles implementation arm64-vct
cpucycles median 0 +86+0+0+0+86+0+0+0+0+0+0+0+0+0+0+0+0+86+0+0+0+0+0+0+0+0+0+0+0+86+0+0+0+0+0+0+0+0+0+0+0+0+86+0+0+0+0+0+0+0+0+0+0+0+86+0+0+0+0+0+0+0+0
cpucycles observed persecond 1440500000...3010000000 with 4096 loops 3 microseconds
cpucycles observed persecond 1621714285...2339200000 with 8192 loops 6 microseconds
cpucycles observed persecond 1884833333...2296200000 with 16384 loops 11 microseconds
cpucycles observed persecond 1963043478...2166380953 with 32768 loops 22 microseconds
cpucycles observed persecond 2004755555...2106000000 with 65536 loops 44 microseconds
cpucycles observed persecond 2051295454...2103000000 with 131072 loops 87 microseconds
cpucycles observed persecond 2054549450...2080722223 with 262144 loops 181 microseconds
cpucycles observed persecond 2056159544...2068681949 with 524288 loops 350 microseconds
cpucycles observed persecond 2061174285...2067573066 with 1048576 loops 699 microseconds
```

`cfarm104`,
Apple M1 (Icestorm-M1 + Firestorm-M1),
MacOSX 12.6 21.6.0:
```
cpucycles version 20240318
cpucycles tracesetup 0 arm64-pmc precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 1 arm64-vct precision 200 scaling 100.000000 only32 0
cpucycles tracesetup 2 default-perfevent precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-mach precision 200 scaling 100.000000 only32 0
cpucycles tracesetup 4 default-monotonic precision 2599 scaling 2.399988 only32 0
cpucycles tracesetup 5 default-gettimeofday precision 2599 scaling 2399.987654 only32 0
cpucycles tracesetup 6 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2399987654
cpucycles implementation arm64-vct
cpucycles median 0 +8700+0+0+0+0+0+0+0+0+0+100+0+0+0+0+0+0+0+0+0+0+0+0+0+100+0+0+0+0+0+0+0+0+0+0+0+0+0+0+100+0+0+0+0+0+0+0+0+0+0+0+0+0+100+0+0+0+0+0+0+0+0+0
cpucycles observed persecond 1450000000...3000000000 with 4096 loops 3 microseconds
cpucycles observed persecond 1916666666...2900000000 with 8192 loops 5 microseconds
cpucycles observed persecond 2310000000...2887500000 with 16384 loops 9 microseconds
cpucycles observed persecond 2290000000...2550000000 with 32768 loops 19 microseconds
cpucycles observed persecond 2351282051...2478378379 with 65536 loops 38 microseconds
cpucycles observed persecond 2374025974...2438666667 with 131072 loops 76 microseconds
cpucycles observed persecond 2373076923...2403896104 with 262144 loops 155 microseconds
cpucycles observed persecond 2386774193...2402272728 with 524288 loops 309 microseconds
cpucycles observed persecond 2395454545...2403257329 with 1048576 loops 615 microseconds
```

`cfarm110` (`gcc1-power7`),
IBM POWER7,
CentOS 7.9 AltArch,
Linux kernel 3.10.0-862.14.4.el7.ppc64:
```
cpucycles version 20240114
cpucycles tracesetup 0 ppc64-mftb precision 212 scaling 7.000000 only32 0
cpucycles tracesetup 1 default-perfevent precision 236 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 346 scaling 3.550000 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 3750 scaling 3550.000000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3550000000
cpucycles implementation ppc64-mftb
cpucycles median 168 -49+56-21+21-14+14-28+28-28+28-7+0-14+21-28+28-28+28-42+28-35+28-35+35-35+21-21+56-49+42-49+21-21+0+0-21+21-49+28-35+7-7-14+14-42+42-7+7+0+0-7+7-21+21-28+28-35+35-42+28-35+28-35
cpucycles observed persecond 3136000000...6569500000 with 4096 loops 3 microseconds
cpucycles observed persecond 3108000000...4233833334 with 8192 loops 7 microseconds
cpucycles observed persecond 3322666666...3878538462 with 16384 loops 14 microseconds
cpucycles observed persecond 3423000000...3698592593 with 32768 loops 28 microseconds
cpucycles observed persecond 3480842105...3616327273 with 65536 loops 56 microseconds
cpucycles observed persecond 3571702702...3641862386 with 131072 loops 110 microseconds
cpucycles observed persecond 3571387387...3605986364 with 262144 loops 221 microseconds
cpucycles observed persecond 3570914414...3588307693 with 524288 loops 443 microseconds
cpucycles observed persecond 3578817155...3587452489 with 1048576 loops 885 microseconds
```

`cfarm112` (`gcc2-power8`),
IBM POWER8E,
CentOS 7.9 AltArch,
Linux kernel 3.10.0-1127.13.1.el7.ppc64le:
```
cpucycles version 20240114
cpucycles tracesetup 0 ppc64-mftb precision 194 scaling 7.250000 only32 0
cpucycles tracesetup 1 default-perfevent precision 308 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 414 scaling 3.690000 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 3890 scaling 3690.000000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 3690000000
cpucycles implementation ppc64-mftb
cpucycles median 123 +1871+7+0+1+0+7-7+1+7+0+1-7+0+0+8+0+0+0+1+7-7+8+0+0+0+8+0+0+1+7+0+1+7+0+1+0+7-7+8+0+0+1+7-7+8+7-7+8-7-7+0+7+1+0+0+8-7+0+0+8+0+0+0
cpucycles observed persecond 2903666666...4451500000 with 4096 loops 5 microseconds
cpucycles observed persecond 3475700000...4630875000 with 8192 loops 9 microseconds
cpucycles observed persecond 3640684210...4205882353 with 16384 loops 18 microseconds
cpucycles observed persecond 3545051282...3800189190 with 32768 loops 38 microseconds
cpucycles observed persecond 3683973333...3816780822 with 65536 loops 74 microseconds
cpucycles observed persecond 3682366666...3747662163 with 131072 loops 149 microseconds
cpucycles observed persecond 3706476510...3739236487 with 262144 loops 297 microseconds
cpucycles observed persecond 3706573825...3722984849 with 524288 loops 595 microseconds
cpucycles observed persecond 3709504617...3717714046 with 1048576 loops 1190 microseconds
```

`cfarm120`,
IBM POWER10,
AlmaLinux 9.3,
Linux kernel 5.14.0-284.11.1.el9_2.ppc64le:
```
cpucycles version 20240114
cpucycles tracesetup 0 ppc64-mftb precision 123 scaling 5.750000 only32 0
cpucycles tracesetup 1 default-perfevent precision 203 scaling 1.000000 only32 0
cpucycles tracesetup 2 default-mach precision 0 scaling 0.000000 only32 0
cpucycles tracesetup 3 default-monotonic precision 226 scaling 2.950000 only32 0
cpucycles tracesetup 4 default-gettimeofday precision 3150 scaling 2950.000000 only32 0
cpucycles tracesetup 5 default-zero precision 0 scaling 0.000000 only32 0
cpucycles persecond 2950000000
cpucycles implementation ppc64-mftb
cpucycles median 69 +6-40+6+5-40+0-40+11+0-40+6-46+6+5-46+12-40+0+5-40+6-46+11+6-46+6+6-41+0-40+12-46+5+6-46+6+11-46+0-40+0+12-41+0-40+6-46+6+11-40+0+6-41+0-40+12+0-41+6-46+6-40+5
cpucycles observed persecond 2103666666...3215500000 with 8192 loops 5 microseconds
cpucycles observed persecond 2827666666...3662714286 with 16384 loops 8 microseconds
cpucycles observed persecond 2821000000...3185125000 with 32768 loops 17 microseconds
cpucycles observed persecond 2818305555...2989823530 with 65536 loops 35 microseconds
cpucycles observed persecond 2897014285...2985852942 with 131072 loops 69 microseconds
cpucycles observed persecond 2920582733...2964649636 with 262144 loops 138 microseconds
cpucycles observed persecond 2930339350...2952341819 with 524288 loops 276 microseconds
cpucycles observed persecond 2941188405...2952218182 with 1048576 loops 551 microseconds
```

`cfarm202`,
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
