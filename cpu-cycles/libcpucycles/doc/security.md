Many security systems have been shown to be breakable by "timing
attacks". These attacks extract secrets by analyzing timings of the
legitimate user's operations on secret data. See the June 2022 survey
page [https://timing.attacks.cr.yp.to](https://timing.attacks.cr.yp.to)
for an overview and further references.

Sometimes these attacks are used as motivation to disable the attacker's
access to various timing mechanisms. For example, Firefox rounds its
`performance.now` timer to 1-millisecond resolution
["to mitigate potential security threats"](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now).

As another example, reducing `/proc/sys/kernel/perf_event_paranoid`
under Linux to 2 (from 3 or higher), so that libcpucycles has access to
the best available Intel/AMD cycle counter (RDPMC), also means making
this cycle counter and other performance-monitoring counters available
to any attacker-controlled software running on the computer. Perhaps
this helps timing attacks, not to mention the possibility of opening up
other vulnerabilities via the complicated `perf_event` interface.

As yet another example, ARM CPUs disable user access to the main CPU
cycle counter by default. Installing a kernel module to enable user
access to the cycle counter could help attacks.

Given the availability of simple mechanisms to disable RDPMC etc., it is
easy to recommend using those mechanisms. To avoid creating unnecessary
tension between those recommendations and the use of libcpucycles,
applications that use libcpucycles should be structured so that
high-resolution timers are used only on controlled development and
benchmarking machines, not on general end-user machines.

This structure might seem incompatible with using cycle counts to
automatically select the best of multiple options, as in FFTW. However,
new infrastructure introduced in [lib25519](https://lib25519.cr.yp.to)
automatically selects options on end-user machines based on cycle counts
that were _collected on benchmarking machines_.

The above text should not be understood as endorsing the idea that
disabling timers is an _effective_ defense against timing attacks.
Certainly disabling high-resolution timers is not sufficient for
security: there are many ways for attackers to amplify timing signals
and to statistically filter out noise from low-resolution timers.
Disabling _every_ standard timing mechanism on the machine does not stop
the attacker from accessing a remote timer or a counter maintained by
the attacker's software. Perhaps disabling timers sometimes makes the
difference between a feasible attack and an infeasible attack, but
evaluating this is extremely difficult.

Meanwhile there is an auditable methodology available to stop timing
attacks: constant-time programming, which systematically cuts off data
flow from secrets to timings.

For example, secrets affect a CPU's power consumption, and Turbo Boost
creates data flow from power consumption to timings, as illustrated by
the [Hertzbleed attack](https://www.hertzbleed.com) extracting secret
keys from the SIKE cryptosystem (before SIKE was broken in other ways),
and an [independent attack](https://arxiv.org/abs/2206.07012)
extracting secret AES keys. Consequently, the constant-time methodology
does not allow Turbo Boost.

This is why [https://timing.attacks.cr.yp.to](https://timing.attacks.cr.yp.to)
recommends turning off Turbo Boost "right now", and explains the
mechanisms available to do this. One non-security reason that it was
already normal (although not universal) for manufacturers to provide
these mechanisms to end users is that Turbo Boost has a reputation for
causing premature hardware failures. Turbo Boost also provides very
little speed benefit for modern multithreaded vectorized applications.

Another reaction to timing attacks is to apply "masking" techniques.
These techniques _seem_ to make it more difficult for attackers to
extract secrets from power consumption and other side channels. However,
as [https://timing.attacks.cr.yp.to](https://timing.attacks.cr.yp.to)
explains, it is "practically impossible for an auditor to obtain any
real assurance that these techniques are secure". See the December 2022
paper
["Breaking a fifth-order masked implementation of CRYSTALS-Kyber by copy-paste"](https://eprint.iacr.org/2022/1713)
for a newer example of a security failure in a masked implementation.
