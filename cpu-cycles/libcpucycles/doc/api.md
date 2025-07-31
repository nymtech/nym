### NAME

cpucycles - count CPU cycles

### SYNOPSIS

    #include <cpucycles.h>

    long long count = cpucycles();
    long long persecond = cpucycles_persecond();
    const char *implementation = cpucycles_implementation();
    const char *version = cpucycles_version();

Link with `-lcpucycles`. Old systems may also need `-lrt`.

### DESCRIPTION

`cpucycles()` returns an estimate for the number of CPU cycles that have
occurred since an unspecified time in the past (perhaps system boot,
perhaps program startup).

Accessing true cycle counters can be difficult on some CPUs and
operating systems. `cpucycles()` does its best to produce accurate
results, but selects a low-precision counter if the only other option is
failure.

`cpucycles_persecond()` returns an estimate for the number of CPU cycles
per second. This estimate comes from `/etc/cpucyclespersecond` if that
file exists, otherwise from various OS mechanisms, otherwise from the
`cpucyclespersecond` environment variable if that is set, otherwise
2399987654.

`cpucycles_implementation()` returns the name of the counter in use:
e.g., `"amd64-pmc"`.

`cpucycles_version()` returns the `libcpucycles` version number as a
string: e.g., `"20240318"`. Results of `cpucycles_implementation()`
should be interpreted relative to `cpucycles_version()`.

`cpucycles` is actually a function pointer. The first call to
`cpucycles()` or `cpucycles_persecond()` or `cpucycles_implementation()`
selects one of the available counters and updates the `cpucycles`
pointer accordingly. Subsequent calls to `cpucycles()` are thread-safe.

### SEE ALSO

**gettimeofday**(2), **clock_gettime**(2)
