This file explains the internal structure of lib25519, and explains how
to add new instruction sets and new implementations.

## Primitives

The directories `crypto_*/*` inside lib25519 define the following
primitives (see also `autogen-test` for Python versions of the
mathematical primitives):

* `crypto_verify/32`: `crypto_verify_32(s,t)` returns 0 when the 32-byte
  arrays `s` and `t` are equal, otherwise `-1`. This function takes
  constant time.

* `crypto_hashblocks/sha512`: `crypto_hashblocks_sha512(h,x,xlen)`
  updates an intermediate SHA-512 hash `h` using all of the full
  128-byte blocks at the beginning of the `xlen`-byte array `x`, and
  returns the number of bytes left over, namely `xlen` mod 128. This
  function takes time that depends on `xlen` but not on the contents of
  `h` or `x`.

* `crypto_hash/sha512`: `crypto_hash_sha512(h,x,xlen)` computes the
  SHA-512 hash `h` of the `xlen`-byte array `x`. This function takes
  time that depends on `xlen` but not on the contents of `x`.

* `crypto_pow/inv25519`: `crypto_pow_inv25519(y,x)` computes the
  2^255^−21 power `y` of an integer `x` modulo 2^255^−19. This is the
  same as the inverse of `x` modulo 2^255^−19 if `x` is not divisible by
  2^255^−19. The integers `x` and `y` are represented as a 32-byte array
  in little-endian form. This function takes constant time.

  This function guarantees that the output `y` is frozen modulo
  2^255^−19, i.e., completely reduced to the range 0,1,...,2^255^−20. The
  caller is expected to freeze `x` before calling this function. The
  function accepts `x` in the range {0,1,...,2^256^−1} while ignoring the
  top bit (the coefficient of 2^255^ in binary): i.e., the function
  reduces `x` modulo 2^255^ and then modulo 2^255^−19.

* `crypto_nP/montgomery25519`: `crypto_nP_montgomery25519(nP,n,P)`
  computes the X25519 function: in short, if a Curve25519 point has
  x-coordinate `P` then the `n`th multiple of the point has x-coordinate
  `nP`. The inputs and outputs are represented as 32-byte arrays in
  little-endian form. This function takes constant time.

  X25519 is defined for `n` in the range 2^254^ + 8{0,1,2,3,...,2^251^−1}.
  `crypto_nP_montgomery25519` allows `n` in the wider range
  {0,1,...,2^256^−1}, and in all cases computes `m`th multiples where `m`
  is defined as follows: make a copy of `n`, clear the top bit, set the
  next bit, and clear the bottom three bits.

  X25519 guarantees that the output `nP` is frozen. It does not require
  the input to be frozen; also, it allows the input to be on the twist,
  and to have small order.

  `crypto_nP_montgomery25519` clears the top bit of `P` before applying
  the X25519 function. Callers that want the X25519 function on `P` with
  the top bit set have to reduce modulo 2^255^−19 for themselves.

* `crypto_nG/merged25519`: `crypto_nG_merged25519(nG,n)` reads an
  integer `n` in the range {0,1,...,2^256^−1} and outputs a frozen
  integer `nG` modulo 2^255^−19, possibly with the top bit set (i.e.,
  adding 2^255^) as described below. Both `n` and `nG` are represented
  as 32-byte arrays in little-endian form. This function takes constant
  time.

  If the top bit of `n` is clear then `nG` is the Edwards y-coordinate
  of the `n`th multiple of G, and the top bit is set exactly when the
  Edwards x-coordinate is odd. Otherwise `nG` is the Montgomery
  x-coordinate of the (`n`−2^255^)th multiple of G, and the top bit is
  clear. Here G is the standard Curve25519 base point, which has
  Montgomery x-coordinate 9, Edwards y-coordinate 4/5, and even Edwards
  x-coordinate.

* `crypto_nG/montgomery25519`: `crypto_nG_montgomery25519(nG,n)` is
  the same as `crypto_nP_montgomery(nG,n,G)` where `G` is the array
  {9,0,0,...,0}. This function takes constant time.

  The point of `crypto_nG` is to save time (using a small table
  precomputed from `G`) compared to the more general `crypto_nP`. This
  has the disadvantage of being more complicated, which is particularly
  important given that lib25519 has not yet been verified, and in any
  case increases code size noticeably for X25519. There is a `ref`
  implementation of `crypto_nG` that simply calls `crypto_nP`, and
  setting sticky bits on the other implementation directories
  (`chmod +t crypto_nG/montgomery25519/*; chmod -t crypto_nG/montgomery25519/ref`)
  will force lib25519 to use `ref`.

* `crypto_mGnP/ed25519`: `crypto_mGnP_ed25519(mGnP,m,n,P)` computes
  `(m mod L)G−(n mod L)P` in Edwards coordinates, where `L` is the prime
  number 2^252^+27742317777372353535851937790883648493 and `G` is the
  same standard base point. This function takes time that depends on the
  inputs.

  The input `m` is an integer in the range {0,1,...,2^256^−1}
  represented as a 32-byte array in little-endian form. Any `m` outside
  the range {0,1,...,L−1} triggers a failure, which is reported as
  described below.

  The input `n` is an integer in the range {0,1,...,2^512^−1}
  represented as a 64-byte array in little-endian form.

  The input point `P` is represented as a 32-byte array as follows: the
  (frozen) Edwards y-coordinate of `P` in {0,1,...,2^255^−20} is stored
  in little-endian form, and then the top bit is set exactly when the
  (frozen) Edwards x-coordinate of `P` is odd. An input 32-byte array
  that does not have this form is instead interpreted as the point `P`
  with Edwards coordinates (...8,26), and triggers a failure, reported
  as described below.

  The output is a 33-byte array. The first 32 bytes are the output point
  `(m mod L)G−(n mod L)P`, represented the same way as `P`. The last
  byte is 1 on success and 0 on failure.

* `crypto_dh/x25519`: `crypto_dh_x25519_keypair(pk,sk)` generates a
  32-byte X25519 public key `pk` and the corresponding 32-byte secret
  key `sk`. This function is the composition of `randombytes` to
  generate `sk` and `crypto_nG_montgomery25519` to generate `pk`.

  `crypto_dh_x25519(k,pk,sk)` generates a 32-byte shared secret `k`
  given a public key `pk` and a secret key `sk`. This function is the
  same as `crypto_nP_montgomery25519`.

* `crypto_sign/ed25519`: `crypto_sign_ed25519_keypair(pk,sk)` generates
  a 32-byte Ed25519 public key `pk` and the corresponding 64-byte secret
  key `sk`. This function takes constant time.

  `crypto_sign_ed25519(sm,&smlen,m,mlen,sk)` generates an `smlen`-byte
  signed message `sm` given an `mlen`-byte message `m` and a secret key
  `sk`. The caller is required to allocate `mlen+64` bytes for `sm`. The
  function always sets `smlen` to `mlen+64`. This function takes time
  that depends on `mlen` but not on the other inputs.

  `crypto_sign_ed25519_open(m,&mlen,sm,smlen,pk)` generates an
  `mlen`-byte message `m` given an `smlen`-byte signed message `sm` and
  a public key `pk`, and returns 0. However, if `sm` is invalid, this
  function returns `-1`, sets `mlen` to `-1`, and clears `m`. The caller is
  required to allocate `smlen` (not just `smlen-64`) bytes for `m`, for
  example using the same array for `sm` and `m`. This function takes time
  that depends on its inputs.

lib25519 includes a command-line utility `lib25519-test` that runs some
tests for each of these primitives, and another utility `lib25519-speed`
that measures cycle counts for each of these primitives.

The stable lib25519 API functions are built from the above primitives:

* `lib25519_dh_keypair` is `crypto_dh_x25519_keypair`.
* `lib25519_dh` is `crypto_dh_x25519`.
* `lib25519_sign_keypair` is `crypto_sign_ed25519_keypair`.
* `lib25519_sign` is `crypto_sign_ed25519`.
* `lib25519_sign_open` is `crypto_sign_ed25519_open`.

Some changes are anticipated in the list of primitives, but these API
functions will remain stable.

As in SUPERCOP and NaCl, message lengths intentionally use `long long`,
not `size_t`. In lib25519, message lengths are signed.

## Implementations

A single primitive can, and usually does, have multiple implementations.
Each implementation is in its own subdirectory. The implementations are
required to have exactly the same input-output behavior, and to some
extent this is tested, although it is not yet formally verified.

Different implementations typically offer different tradeoffs between
portability, simplicity, and efficiency. For example,
`crypto_nP/montgomery25519/ref10` is portable;
`crypto_nP/montgomery25519/amd64-maax` is faster and less portable.

Each unportable implementation has an `architectures` file. Each line in
this file identifies a CPU instruction set (and ABI) where the
implementation works. For example,
`crypto_nP/montgomery25519/amd64-maax/architectures` has one line
`amd64 bmi2 adx`, meaning that the implementation works on CPUs that
have the Intel/AMD 64-bit instruction set with the BMI2 and ADX
instruction-set extensions. The top-level `compilers` directory shows
(among other things) the allowed instruction-set names such as `bmi2`.

At run time, lib25519 checks the CPU where it is running, and selects
an implementation where `architectures` is compatible with that CPU.
Each primitive makes its own selection once per program startup, using
the compiler's `ifunc` mechanism. This type of run-time selection means,
for example, that an `amd64` CPU without AVX2 can share binaries with an
`amd64` CPU with AVX2. However, correctness requires instruction sets to
be preserved by migration across cores via the OS kernel, VM migration,
etc.

The compiler has a `target` mechanism that makes an `ifunc` selection
based on CPU architectures. Instead of using the `target` mechanism,
lib25519 uses a more sophisticated mechanism that also accounts for
benchmarks collected in advance of compilation.

## Compilers

lib25519 tries different C compilers for each implementation. For
example, `compilers/default` lists the following compilers:

        gcc -Wall -fPIC -fwrapv -O2
        clang -Wall -fPIC -fwrapv -Qunused-arguments -O2

Sometimes `gcc` produces better code, and sometimes `clang` produces
better code.

As another example, `compilers/amd64+avx` lists the following compilers:

        gcc -Wall -fPIC -fwrapv -O2 -mmmx -msse -msse2 -msse3 -mssse3 -msse4.1 -msse4.2 -mavx -mtune=sandybridge
        clang -Wall -fPIC -fwrapv -Qunused-arguments -O2 -mmmx -msse -msse2 -msse3 -mssse3 -msse4.1 -msse4.2 -mavx -mtune=sandybridge

The `-mavx` option tells these compilers that they are free to use the
AVX instruction-set extension.

Code compiled using the compilers in `compilers/amd64+avx` will be
considered at run time by the lib25519 selection mechanism if the
`supports()` function in `compilers/amd64+avx.c` returns nonzero. This
function checks whether the run-time CPU supports AVX (and SSE and so on,
and OSXSAVE with XMM/YMM being saved;
[https://gcc.gnu.org/bugzilla/show_bug.cgi?id=85100](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=85100)
says that all versions of gcc until 2018 handled this incorrectly in
`target`). Similar comments apply to other `compilers/*` files.

If some compilers fail (for example, clang is not installed, or the
compiler version is too old to support the compiler options used in
lib25519), the lib25519 compilation process will try its best to produce
a working library using the remaining compilers, even if this means
lower performance.

## Trimming

By default, to reduce size of the compiled library, the lib25519
compilation process trims the library down to the implementations that
are selected by lib25519's selection mechanism (across all CPUs; the
library remains portable, not tied to the compilation CPU).

This trimming is handled at link time rather than compile time to
increase the chance that, even if some implementations are broken by
compiler "upgrades", the library will continue to build successfully.

To avoid this trimming, pass the `--notrim` option to `./configure`.
All implementations that compile are then included in the library,
tested by `lib2519-test`, and measured by `lib25519-speed`. You'll want
to avoid trimming if you're adding new instruction sets or new
implementations (see below), so that you can run tests and benchmarks of
code that isn't selected yet.

## How to recompile after changes

If you make changes in the lib25519 source directory, you have to run
`./configure` again to repopulate the build directory. Simply running
`make` again doesn't suffice.

By default, `./configure` cleans the build directory first, so `make`
will recompile everything. This can be on the scale of seconds if you
have enough cores, but maybe you're developing on a slower machine. Two
options are currently available to accelerate the edit-compile cycle:

   * There is an experimental `--noclean` option to `./configure` that,
     for some simple types of changes, can produce a successful build
     without cleaning.

   * You can disable the implementations you're not using by setting
     sticky bits on the source directories for those implementations:
     e.g., `chmod +t crypto_nG/*/*avx2*`.

Make sure to reenable all implementations and do a full clean build if
you're collecting data to add to the source `benchmarks` directory.

## How to add new instruction sets

Adding another file `compilers/amd64+foo`, along with a `supports()`
implementation in `compilers/amd64+foo.c`, will support a new
instruction set. Do not assume that the new `foo` instruction set
implies support for older instruction sets (the idea of "levels" of
instruction sets); instead make sure to include the older instruction
sets in `+` tags, as illustrated by
`compilers/amd64+avx+bmi2+avx2+adx+avx512f+avx512vl+avx512ifma`.

In the compiler options, always make sure to include `-fPIC` to support
shared libraries, and `-fwrapv` to switch to a slightly less dangerous
version of C.

The `foo` tags don't have to be instruction sets. For example, if a CPU
has the same instruction set but wants different optimizations because
of differences in instruction timings, you can make a tag for those
optimizations, using, e.g., CPU IDs or benchmarks in the corresponding
`supports()` function to decide whether to enable those optimizations.
Benchmarks tend to be more future-proof than a list of CPU IDs, but the
time taken for benchmarks at program startup has to be weighed against
the subsequent speedup from the resulting optimizations.

To see how well lib25519 performs with the new compilers, run
`lib25519-speed` on the target machine and look for the `foo` lines in
the output. If the new performance is better than the performance shown
on the `selected` lines:

   * Copy the `lib25519-speed` output into a file on the `benchmarks`
     directory, typically named after the hostname of the target
     machine.

   * Run `./prioritize` in the top-level directory to create `priority`
     files. These files tell lib25519 which implementations to select
     for any given architecture.

   * Reconfigure (again with `--notrim`), recompile, rerun
     `lib25519-test`, and rerun `lib25519-speed` to check that the
     `default` lines now use the `foo` compiler.

If the `foo` implementation is outperformed by other implementations,
then these steps don't help except for documenting this fact. The same
implementation might turn out to be useful for subsequent `foo` CPUs.

## How to add new implementations

Taking full advantage of the `foo` instruction set usually requires
writing new implementations. Sometimes there are also ideas for taking
better advantage of existing instruction sets.

Structurally, adding a new implementation of a primitive is a simple
matter of adding a new subdirectory with the code for that
implementation. Most of the work is optimizing the use of `foo`
intrinsics in `.c` files or `foo` instructions in `.S` files. Make sure
to include an `architectures` file saying, e.g., `amd64 avx2 foo`.

Names of implementation directories can use letters, digits, dashes, and
underscores. Do not use two implementation names that are the same when
dashes and underscores are removed.

All `.c` and `.S` files in the implementation directory are compiled and
linked. There is no need to edit a separate list of these files. You can
also use `.h` files via the C preprocessor.

If an implementation is actually more restrictive than indicated in
`architectures` then the resulting compiled library will fail on some
machines (although perhaps that implementation will not be used by
default). Putting unnecessary restrictions into `architectures` will not
create such failures, but can unnecessarily limit performance.

Some, but not all, mistakes in `architectures` will produce warnings
from the `checkinsns` script that runs automatically when lib25519 is
compiled. Running the `lib25519-test` program tries all implementations,
but only on the CPU where `lib25519-test` is being run, and `lib25519-test`
does not guarantee code coverage: for example, other message lengths
being signed could involve other code paths.

`amd64` implies little-endian, and implies architectural support for
unaligned loads and stores. Beware, however, that the Intel/AMD
vectorized `load`/`store` intrinsics (and the underlying `movdqa`
instruction) require alignment; if in doubt, use `loadu`/`storeu` (and
`movdqu`). The `lib25519-test` program checks unaligned inputs and
outputs, but can miss issues with unaligned stack variables.

To test your implementation, compile everything, check for compiler
warnings and errors, run `lib25519-test` (or just `lib25519-test nG` to
test a `crypto_nG` implementation), and check for a line saying `all
tests succeeded`. To use AddressSanitizer (for catching, at run time,
buffer overflows in C code), add `-fsanitize=address` to the `gcc` and
`clang` lines in `compilers/*`.

To see the performance of your implementation, run `lib25519-speed`.
If the new performance is better than the performance shown on the
`default` lines, follow the same steps as for a new instruction set:
copy the `lib25519-speed` output into a file on the `benchmarks`
directory; run `./prioritize` in the top-level directory to create
`priority` files; reconfigure (again with `--notrim`); recompile; rerun
`lib25519-test`; rerun `lib25519-speed`; check that the `default` lines
now use the new implementation.

## How to handle namespacing

As in SUPERCOP and NaCl, to call `crypto_hash_sha512()`, you have to
include `crypto_hash_sha512.h`; but to write an implementation of
`crypto_hash_sha512()`, you have to instead include `crypto_hash.h` and
define `crypto_hash`. Similar comments apply to other primitives.

The function name that's actually linked might end up as, e.g.,
`lib25519_hash_sha512_blocksplusavx_C2_hash` where `blocksplusavx`
indicates the implementation and `C2` indicates the compiler. Don't try
to build this name into your implementation.

If you have another global symbol `x` (for example, a non-`static`
function in a `.c` file, or a non-`static` variable outside functions in
a `.c` file), you have to replace it with `CRYPTO_NAMESPACE(x)`, for
example with `#define x CRYPTO_NAMESPACE(x)`.

For global symbols in `.S` files and `shared-*.c` files, use
`CRYPTO_SHARED_NAMESPACE` instead of `CRYPTO_NAMESPACE`. For `.S` files
that define both `x` and `_x` to handle platforms where `x` in C is `_x`
in assembly, use `CRYPTO_SHARED_NAMESPACE(x)` and
`_CRYPTO_SHARED_NAMESPACE(x)`; `CRYPTO_SHARED_NAMESPACE(_x)` is not
sufficient.

lib25519 includes a mechanism to recognize files that are copied across
implementations (possibly of different primitives) and to unify those
into a file compiled only once, reducing the overall size of the
compiled library and possibly improving cache utilization. To request
this mechanism, include a line `// linker define x` for any global
symbol `x` defined in the file, and a line `// linker use x` for any
global symbol `x` used in the file from the same implementation (not
`crypto_*` subroutines that you're calling, `randombytes`, etc.). This
mechanism tries very hard, perhaps too hard, to avoid improperly
unifying files: for example, even a slight difference in a `.h` file
included by a file defining a used symbol will disable the mechanism.

Typical namespacing mistakes will produce either linker failures or
warnings from the `checknamespace` script that runs automatically when
lib25519 is compiled.
