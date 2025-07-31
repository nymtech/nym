To download and unpack the latest version of libcpucycles:

    wget -m https://cpucycles.cr.yp.to/libcpucycles-latest-version.txt
    version=$(cat cpucycles.cr.yp.to/libcpucycles-latest-version.txt)
    wget -m https://cpucycles.cr.yp.to/libcpucycles-$version.tar.gz
    tar -xzf cpucycles.cr.yp.to/libcpucycles-$version.tar.gz
    cd libcpucycles-$version

Then [install](install.html).

### Archives and changelog (reverse chronological)

[`libcpucycles-20240318.tar.gz`](libcpucycles-20240318.tar.gz) [browse](libcpucycles-20240318.html)

Port to MacOS X:
handle missing `-lrt`, and handle differences in shared-library naming.

Include `cpucycles-info` man page.

[`libcpucycles-20240114.tar.gz`](libcpucycles-20240114.tar.gz) [browse](libcpucycles-20240114.html)

Add `arm32-1176` counter.

Allow slop 0.2 rather than 0.1 for `FINDMULTIPLIER`.

Improve platform detection.

Port to FreeBSD.

Use blue boldface during compilation for "skipping option that did not compile".

`doc/install.md`: headings; note manual pages.

Add `doc/license.md`.

Update HTML style for better tt visibility and copy-paste.

[`libcpucycles-20230115.tar.gz`](libcpucycles-20230115.tar.gz) [browse](libcpucycles-20230115.html)

Update actual `cpucycles_version` behavior to match documentation.

[`libcpucycles-20230110.tar.gz`](libcpucycles-20230110.tar.gz) [browse](libcpucycles-20230110.html)

`doc/api.md`: Document `cpucycles_version()`.

Add `s390x-stckf` counter.

`cpucycles/default-perfevent.c`: Read into `int64_t` instead of `long long`.
Add comment explaining issues with `PERF_FORMAT_TOTAL_TIME_RUNNING`.

`configure`: Improve `uname` handling.

`doc/api.md`: Update description of default frequency.

[`libcpucycles-20230105.tar.gz`](libcpucycles-20230105.tar.gz) [browse](libcpucycles-20230105.html)
