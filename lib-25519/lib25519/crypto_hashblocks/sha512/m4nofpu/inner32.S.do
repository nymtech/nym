#!/bin/sh

qhasm-arm-m4 < inner32.q \
| grep -v fpu \
| ./copy-collector \
| ./align \
| sed 's/\<inner\>/CRYPTO_SHARED_NAMESPACE(inner)/'
