Prerequisites: `python3`; `gcc` and/or `clang`. Currently tested only
under Linux, but porting to other systems shouldn't be difficult.

### For sysadmins

To install in `/usr/local/{include,lib,bin,man}`:

    ./configure && make -j8 install

### For developers with an unprivileged account

Typically you'll already have

    export LD_LIBRARY_PATH="$HOME/lib"
    export LIBRARY_PATH="$HOME/lib"
    export CPATH="$HOME/include"
    export PATH="$HOME/bin:$PATH"

in `$HOME/.profile`. To install in `$HOME/{include,lib,bin,man}`:

    ./configure --prefix=$HOME && make -j8 install

### For distributors creating a package

Run

    ./configure --prefix=/usr && make -j8

and then follow your usual packaging procedures for the
`build/0/package` files:

    build/0/package/man/man3/cpucycles.3
    build/0/package/include/cpucycles.h
    build/0/package/lib/libcpucycles*
    build/0/package/bin/cpucycles-info

There are some old systems where libcpucycles requires `-lrt` for
`clock_gettime`; currently `libcpucycles.so` doesn't link to `-lrt`,
so it's up to the caller to link to `-lrt`.

### More options

You can run

    ./configure --host=amd64

to override `./configure`'s guess of the architecture that it should
compile for. The architecture controls which cycle counters to try
compiling: e.g., `amd64` tries compiling `cpucycles/amd64*` and
`cpucycles/default*`.

Inside the `build` directory, `0` is symlinked to `amd64` for
`--host=amd64`. Running `make clean` removes `build/amd64`. Re-running
`./configure` automatically starts with `make clean`.

A subsequent `./configure --host=arm64` will create `build/arm64` and
symlink `0 -> arm64`, without touching an existing `build/amd64`.
However, cross-compilers aren't yet selected automatically.

Compilers tried are listed in `compilers/default`. Each compiler
includes `-fPIC` to create a shared library, `-fvisibility=hidden` to
hide non-public symbols in the library, and `-fwrapv` to switch to a
slightly less dangerous version of C. The first compiler that seems to
work is used to compile everything.
