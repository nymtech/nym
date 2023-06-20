#!/usr/bin/env python3

import sys

i = 0

for doubleround in range(8):
  i0 = (i+0)&7
  i1 = (i+1)&7
  i2 = (i+2)&7
  i3 = (i+3)&7
  i4 = (i+4)&7
  i5 = (i+5)&7
  i6 = (i+6)&7
  i7 = (i+7)&7

  round = 2*doubleround
  round4 = round&~3
  loadarray = 'wc%d%d%d%d' % (round4,round4+1,round4+2,round4+3)
  i0load = '%s[%d]' % (loadarray,(round+0)&3)
  i1load = '%s[%d]' % (loadarray,(round+1)&3)

  i -= 2

  if len(sys.argv) > 1:
    if sys.argv[1] != str(doubleround):
      continue


  print('    r%d += %s' % (i7,i0load))
  print('      r%dSigma1 = r%d>>>14' % (i4,i4))
  print('    ch%d = r%d' % (i7,i6))
  print('      r%d18 = r%d>>>18' % (i4,i4))
  print('    ch%d ^= r%d' % (i7,i5))

  print('      r%d41 = r%d>>>41' % (i4,i4))
  print('      r%dSigma1 ^= r%d18' % (i4,i4))
  print('    ch%d &= r%d' % (i7,i4))
  print('      r%dSigma0 = r%d>>>28' % (i0,i0))

  print('      r%dSigma1 ^= r%d41' % (i4,i4))
  print('    ch%d ^= r%d' % (i7,i6))
  print('      r%d34 = r%d>>>34' % (i0,i0))
  print('  maj%d = r%d' % (i6,i1))
  print('  maj%d ^= r%d' % (i6,i0))

  print('      r%dSigma0 ^= r%d34' % (i0,i0))
  print('    r%d += ch%d' % (i7,i7))
  print('  r%dandr%d = r%d' % (i0,i1,i1))
  print('      r%d39 = r%d>>>39' % (i0,i0))
  print('  r%dandr%d &= r%d' % (i0,i1,i0))

  print('      r%dSigma0 ^= r%d39' % (i0,i0))
  print('      r%d += r%dSigma1' % (i7,i4))
  print('  maj%d = r%d' % (i7,i2))
  print('            r%d += %s' % (i6,i1load))
  print('  maj%d &= maj%d' % (i7,i6))

  print('  r%d += r%d' % (i3,i7))
  print('      r%d += r%dSigma0' % (i7,i0))
  print('            ch%d = r%d' % (i6,i5))
  print('  maj%d ^= r%dandr%d' % (i7,i0,i1))
  print('            ch%d ^= r%d' % (i6,i4))

  print('          r%dSigma1 = r%d>>>14' % (i3,i3))
  print('  r%d += maj%d' % (i7,i7))
  print('            ch%d &= r%d' % (i6,i3))
  print('              r%d18 = r%d>>>18' % (i3,i3))

  print('              r%dSigma1 ^= r%d18' % (i3,i3))
  print('          maj%d &= r%d' % (i6,i7))
  print('            ch%d ^= r%d' % (i6,i5))
  print('              r%d41 = r%d>>>41' % (i3,i3))

  print('              r%dSigma1 ^= r%d41' % (i3,i3))
  print('              r%dSigma0 = r%d>>>28' % (i7,i7))
  print('          maj%d ^= r%dandr%d' % (i6,i0,i1))
  print('            r%d += ch%d' % (i6,i6))

  print('              r%d += r%dSigma1' % (i6,i3))
  print('              r%d34 = r%d>>>34' % (i7,i7))

  print('              r%dSigma0 ^= r%d34' % (i7,i7))
  print('          r%d += r%d' % (i2,i6))
  print('          r%d += maj%d' % (i6,i6))
  print('              r%d39 = r%d>>>39' % (i7,i7))

  print('              r%dSigma0 ^= r%d39' % (i7,i7))

  print('              r%d += r%dSigma0' % (i6,i7))
