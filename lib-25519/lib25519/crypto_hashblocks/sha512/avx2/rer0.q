            X1 = mem256[&w + 8]
            4x X1right1 = X1 unsigned>> 1
      r4Sigma1 = r4>>>14
    r7 += wc891011[0]
    ch7 = r6
    ch7 ^= r5

            4x X1left63 = X1 << 63
      r418 = r4>>>18
    ch7 &= r4
  maj6 = r1
  maj6 ^= r0

            X1sigma0 = X1right1 ^ X1left63
      r441 = r4>>>41
      r4Sigma1 ^= r418
    ch7 ^= r6

            4x X1right8 = X1 unsigned>> 8
      r4Sigma1 ^= r441
      r0Sigma0 = r0>>>28
    r7 += ch7

            X1sigma0 = X1sigma0 ^ X1right8
      r034 = r0>>>34
      r7 += r4Sigma1
  maj7 = r2
  maj7 &= maj6

            W14 = mem128[&w + 112],0
            2x,0 W14right19 = W14 unsigned>> 19
      r0Sigma0 ^= r034
      r039 = r0>>>39
  r3 += r7

            4x X1left56 = X1 << 56
      r0Sigma0 ^= r039
    r6 += wc891011[1]
  r0andr1 = r1
  r0andr1 &= r0

            2x,0 W14left45 = W14 << 45
      r7 += r0Sigma0
  maj7 ^= r0andr1
    ch6 = r5
    ch6 ^= r4

            X1sigma0 = X1sigma0 ^ X1left56
            2x,0 W14right61 = W14 unsigned>> 61
      r3Sigma1 = r3>>>14
  r7 += maj7

            4x X1right7 = X1 unsigned>> 7
            1x,0 W14sigma1 = W14right19 ^ W14left45
      r318 = r3>>>18
    ch6 &= r3

            X1sigma0 = X1sigma0 ^ X1right7
      r3Sigma1 ^= r318
      r341 = r3>>>41
  maj6 &= r7

            1x,0 W14sigma1 ^= W14right61
            4x X0 = X0 + mem256[&w + 72]
      r3Sigma1 ^= r341
  maj6 ^= r0andr1

            2x,0 W14left3 = W14 << 3
      r7Sigma0 = r7>>>28
    ch6 ^= r5
      r6 += r3Sigma1

            4x X0 = X0 + X1sigma0
      r734 = r7>>>34
    r5 += wc891011[2]
    r6 += ch6

            1x,0 W14sigma1 ^= W14left3
      r7Sigma0 ^= r734
      r739 = r7>>>39
  r2 += r6

            2x,0 W14right6 = W14 unsigned>> 6
      r7Sigma0 ^= r739
  r6 += maj6
    ch5 = r4
    ch5 ^= r3

            1x,0 W14sigma1 ^= W14right6
      r6 += r7Sigma0
      r2Sigma1 = r2>>>14
    ch5 &= r2

            4x X0 = W14sigma1 + X0
      r218 = r2>>>18
      r241 = r2>>>41
    ch5 ^= r4

            2x,0 W0right19 = X0 unsigned>> 19
      r2Sigma1 ^= r218
      r6Sigma0 = r6>>>28
    r5 += ch5

            2x,0 W0left45 = X0 << 45
      r2Sigma1 ^= r241
      r634 = r6>>>34
  maj4 = r7
  maj4 ^= r6

            2x,0 W0right61 = X0 unsigned>> 61
            1x,0 W0sigma1 = W0right19 ^ W0left45
      r6Sigma0 ^= r634
      r639 = r6>>>39

            2x,0 W0left3 = X0 << 3
            1x,0 W0sigma1 ^= W0right61
      r6Sigma0 ^= r639
      r5 += r2Sigma1

            2x,0 W0right6 = X0 unsigned>> 6
            1x,0 W0sigma1 ^= W0left3
  r1 += r5
  r6andr7 = r7
  r6andr7 &= r6

            1x,0 W0sigma1 ^= W0right6
      r1Sigma1 = r1>>>14
      r5 += r6Sigma0
  maj5 = r0
  maj5 &= maj4

            W0sigma1 = W0sigma1[1],W0sigma1[0]
  maj5 ^= r6andr7
    ch4 = r3
    ch4 ^= r2

  r5 += maj5
    ch4 &= r1
      r118 = r1>>>18

  maj4 &= r5
    ch4 ^= r3
    r4 += wc891011[3]
      r1Sigma1 ^= r118

      r141 = r1>>>41
            4x X0 = X0 + W0sigma1
            mem256[&w + 128] = X0
            mem256[&w + 0] = X0
    r4 += ch4
  maj4 ^= r6andr7

      r5Sigma0 = r5>>>28
            4x D0 = X0 + mem256[constants + 0]
            wc0123 = D0
      r534 = r5>>>34
      r1Sigma1 ^= r141

      r4 += r1Sigma1
      r5Sigma0 ^= r534
      r539 = r5>>>39

  r0 += r4
  r4 += maj4
      r5Sigma0 ^= r539

      r4 += r5Sigma0
