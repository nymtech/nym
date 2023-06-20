            X9 = mem256[&w + 72]
            4x X9right1 = X9 unsigned>> 1
      r4Sigma1 = r4>>>14
    r7 += wc0123[0]
    ch7 = r6
    ch7 ^= r5

            4x X9left63 = X9 << 63
      r418 = r4>>>18
    ch7 &= r4
  maj6 = r1
  maj6 ^= r0

            X9sigma0 = X9right1 ^ X9left63
      r441 = r4>>>41
      r4Sigma1 ^= r418
    ch7 ^= r6

            4x X9right8 = X9 unsigned>> 8
      r4Sigma1 ^= r441
      r0Sigma0 = r0>>>28
    r7 += ch7

            X9sigma0 = X9sigma0 ^ X9right8
      r034 = r0>>>34
      r7 += r4Sigma1
  maj7 = r2
  maj7 &= maj6

            W6 = mem128[&w + 48],0
            2x,0 W6right19 = W6 unsigned>> 19
      r0Sigma0 ^= r034
      r039 = r0>>>39
  r3 += r7

            4x X9left56 = X9 << 56
      r0Sigma0 ^= r039
    r6 += wc0123[1]
  r0andr1 = r1
  r0andr1 &= r0

            2x,0 W6left45 = W6 << 45
      r7 += r0Sigma0
  maj7 ^= r0andr1
    ch6 = r5
    ch6 ^= r4

            X9sigma0 = X9sigma0 ^ X9left56
            2x,0 W6right61 = W6 unsigned>> 61
      r3Sigma1 = r3>>>14
  r7 += maj7

            4x X9right7 = X9 unsigned>> 7
            1x,0 W6sigma1 = W6right19 ^ W6left45
      r318 = r3>>>18
    ch6 &= r3

            X9sigma0 = X9sigma0 ^ X9right7
      r3Sigma1 ^= r318
      r341 = r3>>>41
  maj6 &= r7

            1x,0 W6sigma1 ^= W6right61
            4x X8 = X8 + mem256[&w + 8]
      r3Sigma1 ^= r341
  maj6 ^= r0andr1

            2x,0 W6left3 = W6 << 3
      r7Sigma0 = r7>>>28
    ch6 ^= r5
      r6 += r3Sigma1

            4x X8 = X8 + X9sigma0
      r734 = r7>>>34
    r5 += wc0123[2]
    r6 += ch6

            1x,0 W6sigma1 ^= W6left3
      r7Sigma0 ^= r734
      r739 = r7>>>39
  r2 += r6

            2x,0 W6right6 = W6 unsigned>> 6
      r7Sigma0 ^= r739
  r6 += maj6
    ch5 = r4
    ch5 ^= r3

            1x,0 W6sigma1 ^= W6right6
      r6 += r7Sigma0
      r2Sigma1 = r2>>>14
    ch5 &= r2

            4x X8 = W6sigma1 + X8
      r218 = r2>>>18
      r241 = r2>>>41
    ch5 ^= r4

            2x,0 W8right19 = X8 unsigned>> 19
      r2Sigma1 ^= r218
      r6Sigma0 = r6>>>28
    r5 += ch5

            2x,0 W8left45 = X8 << 45
      r2Sigma1 ^= r241
      r634 = r6>>>34
  maj4 = r7
  maj4 ^= r6

            2x,0 W8right61 = X8 unsigned>> 61
            1x,0 W8sigma1 = W8right19 ^ W8left45
      r6Sigma0 ^= r634
      r639 = r6>>>39

            2x,0 W8left3 = X8 << 3
            1x,0 W8sigma1 ^= W8right61
      r6Sigma0 ^= r639
      r5 += r2Sigma1

            2x,0 W8right6 = X8 unsigned>> 6
            1x,0 W8sigma1 ^= W8left3
  r1 += r5
  r6andr7 = r7
  r6andr7 &= r6

            1x,0 W8sigma1 ^= W8right6
      r1Sigma1 = r1>>>14
      r5 += r6Sigma0
  maj5 = r0
  maj5 &= maj4

            W8sigma1 = W8sigma1[1],W8sigma1[0]
  maj5 ^= r6andr7
    ch4 = r3
    ch4 ^= r2

  r5 += maj5
    ch4 &= r1
      r118 = r1>>>18

  maj4 &= r5
    ch4 ^= r3
    r4 += wc0123[3]
      r1Sigma1 ^= r118

      r141 = r1>>>41
            4x X8 = X8 + W8sigma1
            mem256[&w + 64] = X8
    r4 += ch4
  maj4 ^= r6andr7

      r5Sigma0 = r5>>>28
            4x D8 = X8 + mem256[constants + 64]
            wc891011 = D8
      r534 = r5>>>34
      r1Sigma1 ^= r141

      r4 += r1Sigma1
      r5Sigma0 ^= r534
      r539 = r5>>>39

  r0 += r4
  r4 += maj4
      r5Sigma0 ^= r539

      r4 += r5Sigma0
