            X13 = mem256[&w + 104]
            4x X13right1 = X13 unsigned>> 1
      r0Sigma1 = r0>>>14
    r3 += wc4567[0]
    ch3 = r2
    ch3 ^= r1

            4x X13left63 = X13 << 63
      r018 = r0>>>18
    ch3 &= r0
  maj2 = r5
  maj2 ^= r4

            X13sigma0 = X13right1 ^ X13left63
      r041 = r0>>>41
      r0Sigma1 ^= r018
    ch3 ^= r2

            4x X13right8 = X13 unsigned>> 8
      r0Sigma1 ^= r041
      r4Sigma0 = r4>>>28
    r3 += ch3

            X13sigma0 = X13sigma0 ^ X13right8
      r434 = r4>>>34
      r3 += r0Sigma1
  maj3 = r6
  maj3 &= maj2

            W10 = mem128[&w + 80],0
            2x,0 W10right19 = W10 unsigned>> 19
      r4Sigma0 ^= r434
      r439 = r4>>>39
  r7 += r3

            4x X13left56 = X13 << 56
      r4Sigma0 ^= r439
    r2 += wc4567[1]
  r4andr5 = r5
  r4andr5 &= r4

            2x,0 W10left45 = W10 << 45
      r3 += r4Sigma0
  maj3 ^= r4andr5
    ch2 = r1
    ch2 ^= r0

            X13sigma0 = X13sigma0 ^ X13left56
            2x,0 W10right61 = W10 unsigned>> 61
      r7Sigma1 = r7>>>14
  r3 += maj3

            4x X13right7 = X13 unsigned>> 7
            1x,0 W10sigma1 = W10right19 ^ W10left45
      r718 = r7>>>18
    ch2 &= r7

            X13sigma0 = X13sigma0 ^ X13right7
      r7Sigma1 ^= r718
      r741 = r7>>>41
  maj2 &= r3

            1x,0 W10sigma1 ^= W10right61
            4x X12 = X12 + mem256[&w + 40]
      r7Sigma1 ^= r741
  maj2 ^= r4andr5

            2x,0 W10left3 = W10 << 3
      r3Sigma0 = r3>>>28
    ch2 ^= r1
      r2 += r7Sigma1

            4x X12 = X12 + X13sigma0
      r334 = r3>>>34
    r1 += wc4567[2]
    r2 += ch2

            1x,0 W10sigma1 ^= W10left3
      r3Sigma0 ^= r334
      r339 = r3>>>39
  r6 += r2

            2x,0 W10right6 = W10 unsigned>> 6
      r3Sigma0 ^= r339
  r2 += maj2
    ch1 = r0
    ch1 ^= r7

            1x,0 W10sigma1 ^= W10right6
      r2 += r3Sigma0
      r6Sigma1 = r6>>>14
    ch1 &= r6

            4x X12 = W10sigma1 + X12
      r618 = r6>>>18
      r641 = r6>>>41
    ch1 ^= r0

            2x,0 W12right19 = X12 unsigned>> 19
      r6Sigma1 ^= r618
      r2Sigma0 = r2>>>28
    r1 += ch1

            2x,0 W12left45 = X12 << 45
      r6Sigma1 ^= r641
      r234 = r2>>>34
  maj0 = r3
  maj0 ^= r2

            2x,0 W12right61 = X12 unsigned>> 61
            1x,0 W12sigma1 = W12right19 ^ W12left45
      r2Sigma0 ^= r234
      r239 = r2>>>39

            2x,0 W12left3 = X12 << 3
            1x,0 W12sigma1 ^= W12right61
      r2Sigma0 ^= r239
      r1 += r6Sigma1

            2x,0 W12right6 = X12 unsigned>> 6
            1x,0 W12sigma1 ^= W12left3
  r5 += r1
  r2andr3 = r3
  r2andr3 &= r2

            1x,0 W12sigma1 ^= W12right6
      r5Sigma1 = r5>>>14
      r1 += r2Sigma0
  maj1 = r4
  maj1 &= maj0

            W12sigma1 = W12sigma1[1],W12sigma1[0]
  maj1 ^= r2andr3
    ch0 = r7
    ch0 ^= r6

  r1 += maj1
    ch0 &= r5
      r518 = r5>>>18

  maj0 &= r1
    ch0 ^= r7
    r0 += wc4567[3]
      r5Sigma1 ^= r518

      r541 = r5>>>41
            4x X12 = X12 + W12sigma1
            mem256[&w + 96] = X12
    r0 += ch0
  maj0 ^= r2andr3

      r1Sigma0 = r1>>>28
            4x D12 = X12 + mem256[constants + 96]
            wc12131415 = D12
      r134 = r1>>>34
      r5Sigma1 ^= r541

      r0 += r5Sigma1
      r1Sigma0 ^= r134
      r139 = r1>>>39

  r4 += r0
  r0 += maj0
      r1Sigma0 ^= r139

      r0 += r1Sigma0
