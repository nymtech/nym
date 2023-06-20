#!/bin/sh

cat inner32.top
( cat inner32.desc
  cat inner.q | sed 's/#.*//' | sed 's/:/#/g'
) \
| qhasm-ops \
| qhasm-regs \
| qhasm-as \
| grep -v '^# qhasm: livefloat80' \
| grep . \
| python3 -c '
import sys
import re
for line in sys.stdin:
  while True:
    i = line.find("ADD4(")
    if i < 0: break
    j = line[i+5:].find(")")
    if j < 0: break
    n = int(line[i+5:i+5+j])
    line = line[:i]+str(n+4)+line[i+6+j:]
  sys.stdout.write(line)
'
