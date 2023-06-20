#!/bin/sh

cpp \
| qhasm-amd64avx \
| sed 's/\<inner\>/CRYPTO_SHARED_NAMESPACE(inner)/' \
| sed 's/\<_inner\>/_CRYPTO_SHARED_NAMESPACE(inner)/' \
| sed 's/^\.p2align 5/.p2align 7/' \
| awk '{
  found = 0
  if (!found && $0 == "and $31,%r11") {
    found = 1
    $0 = "and $511,%r11"
  }
  print
}'
