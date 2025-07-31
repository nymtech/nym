### NAME

cpucycles-info - report information about CPU cycle counters

### SYNOPSIS

    cpucycles-info

### DESCRIPTION

`cpucycles-info`
prints human-readable information
about the cycle counters considered by `cpucycles()`.

The format is subject to change
but currently includes
a `cpucycles version` line,
`cpucycles tracesetup` lines
showing which cycle counters are considered
and how precise they seem to be
(with smaller `precision` values being better,
except that `precision 0` means a cycle counter that does not seem to work),
a `cpucycles persecond` line about the selected cycle counter,
a `cpucycles implementation` line about the selected cycle counter,
and
`cpucycles observed persecond` lines about the selected cycle counter.

### SEE ALSO

**cpucycles**(3)
