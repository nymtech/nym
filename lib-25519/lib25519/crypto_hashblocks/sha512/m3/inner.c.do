#!/bin/sh

cat inner.top
( cat inner.desc
  cat inner.q | sed 's/#.*//' | sed 's/:/#/g'
) \
| qhasm-ops \
| qhasm-regs \
| qhasm-as \
| grep -v '^# qhasm: livefloat80' \
| grep . \
| sed 's_^#_\/\/_'
