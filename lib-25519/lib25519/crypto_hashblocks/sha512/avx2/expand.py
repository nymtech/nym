#!/usr/bin/env python3

import sys

for i in (0,4,8,12):
  if len(sys.argv) > 1:
    if sys.argv[1] != str(i):
      continue

  i0 = (i+0)&15
  i1 = (i+1)&15
  i9 = (i+9)&15
  i14 = (i+14)&15

  print('            X%d = mem256[&w + %d]' % (i1,8*i1))
  print('            W%d = mem128[&w + %d],0' % (i14,8*i14))
  print('')
  print('            4x X%dright1 = X%d unsigned>> 1' % (i1,i1))
  print('            4x X%dleft63 = X%d << 63' % (i1,i1))
  print('            X%dsigma0 = X%dright1 ^ X%dleft63' % (i1,i1,i1))
  print('            4x X%dright8 = X%d unsigned>> 8' % (i1,i1))
  print('            X%dsigma0 = X%dsigma0 ^ X%dright8' % (i1,i1,i1))
  print('                2x,0 W%dright19 = W%d unsigned>> 19' % (i14,i14))
  print('            4x X%dleft56 = X%d << 56' % (i1,i1))
  print('                2x,0 W%dleft45 = W%d << 45' % (i14,i14))
  print('            X%dsigma0 = X%dsigma0 ^ X%dleft56' % (i1,i1,i1))
  print('                1x,0 W%dsigma1 = W%dright19 ^ W%dleft45' % (i14,i14,i14))
  print('            4x X%dright7 = X%d unsigned>> 7' % (i1,i1))
  print('                2x,0 W%dright61 = W%d unsigned>> 61' % (i14,i14))
  print('            X%dsigma0 = X%dsigma0 ^ X%dright7' % (i1,i1,i1))
  print('                1x,0 W%dsigma1 ^= W%dright61' % (i14,i14))
  print('            4x X%d = X%d + X%dsigma0' % (i0,i0,i1))
  print('                2x,0 W%dleft3 = W%d << 3' % (i14,i14))
  print('            4x X%d = X%d + mem256[&w + %d]' % (i0,i0,8*i9))
  print('                1x,0 W%dsigma1 ^= W%dleft3' % (i14,i14))
  print('                2x,0 W%dright6 = W%d unsigned>> 6' % (i14,i14))
  print('                1x,0 W%dsigma1 ^= W%dright6' % (i14,i14))
  print('            4x X%d = W%dsigma1 + X%d' % (i0,i14,i0))
  print('')
  print('            2x,0 W%dright19 = X%d unsigned>> 19' % (i0,i0))
  print('            2x,0 W%dleft45 = X%d << 45' % (i0,i0))
  print('            1x,0 W%dsigma1 = W%dright19 ^ W%dleft45' % (i0,i0,i0))
  print('            2x,0 W%dright61 = X%d unsigned>> 61' % (i0,i0))
  print('            1x,0 W%dsigma1 ^= W%dright61' % (i0,i0))
  print('            2x,0 W%dleft3 = X%d << 3' % (i0,i0))
  print('            1x,0 W%dsigma1 ^= W%dleft3' % (i0,i0))
  print('            2x,0 W%dright6 = X%d unsigned>> 6' % (i0,i0))
  print('            1x,0 W%dsigma1 ^= W%dright6' % (i0,i0))
  print('            W%dsigma1 = W%dsigma1[1],W%dsigma1[0]' % (i0,i0,i0))
  print('')
  print('            4x X%d = X%d + W%dsigma1' % (i0,i0,i0))
  if i == 0:
    print('            mem256[&w + 128] = X%d' % (i0))
  print('            mem256[&w + %d] = X%d' % (8*i0,i0))
  print('            4x D%d = X%d + mem256[constants + %d]' % (i0,i0,8*i0))
  print('            wc%d%d%d%d = D%d' % (i,i+1,i+2,i+3,i0))
  print('')
