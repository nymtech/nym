stack64 r11_stack
stack64 r12_stack
stack64 r13_stack
stack64 r14_stack
stack64 r15_stack
stack64 rbx_stack
stack64 rbp_stack

int64 statebytes
stack64 statebytes_stack
int64 in
stack64 in_stack
int64 inlen
stack64 inlen_stack
int64 constants
stack64 constants_stack
int64 r0
int64 r1
int64 r2
int64 r3
int64 r4
int64 r5
int64 r6
int64 r7
int64 i

stack256 state0123
stack256 state4567
reg256 X0
reg256 X4
reg256 X8
reg256 X12
reg256 X1
reg256 X5
reg256 X9
reg256 X13
reg256 bigendian64
reg256 D0
reg256 D4
reg256 D8
reg256 D12
reg256 W0
reg256 W2
reg256 W4
reg256 W6
reg256 W8
reg256 W10
reg256 W12
reg256 W14

stack1280 w
stack256 wc0123
stack256 wc4567
stack256 wc891011
stack256 wc12131415

int64 r0andr1
int64 r2andr3
int64 r4andr5
int64 r6andr7
int64 ch0
int64 ch1
int64 ch2
int64 ch3
int64 ch4
int64 ch5
int64 ch6
int64 ch7
int64 maj0
int64 maj1
int64 maj2
int64 maj3
int64 maj4
int64 maj5
int64 maj6
int64 maj7
int64 r0Sigma0
int64 r1Sigma0
int64 r2Sigma0
int64 r3Sigma0
int64 r4Sigma0
int64 r5Sigma0
int64 r6Sigma0
int64 r7Sigma0
int64 r0Sigma1
int64 r1Sigma1
int64 r2Sigma1
int64 r3Sigma1
int64 r4Sigma1
int64 r5Sigma1
int64 r6Sigma1
int64 r7Sigma1
int64 r018
int64 r118
int64 r218
int64 r318
int64 r418
int64 r518
int64 r618
int64 r718
int64 r041
int64 r141
int64 r241
int64 r341
int64 r441
int64 r541
int64 r641
int64 r741
int64 r034
int64 r134
int64 r234
int64 r334
int64 r434
int64 r534
int64 r634
int64 r734
int64 r039
int64 r139
int64 r239
int64 r339
int64 r439
int64 r539
int64 r639
int64 r739

reg256 X1right1
reg256 X1left63
reg256 X1right8
reg256 X1left56
reg256 X1right7
reg256 X1sigma0
reg256 X5right1
reg256 X5left63
reg256 X5right8
reg256 X5left56
reg256 X5right7
reg256 X5sigma0
reg256 X9right1
reg256 X9left63
reg256 X9right8
reg256 X9left56
reg256 X9right7
reg256 X9sigma0
reg256 X13right1
reg256 X13left63
reg256 X13right8
reg256 X13left56
reg256 X13right7
reg256 X13sigma0

reg256 W0right19
reg256 W0right61
reg256 W0right6
reg256 W0left45
reg256 W0left3
reg256 W0sigma1
reg256 W2right19
reg256 W2right61
reg256 W2right6
reg256 W2left45
reg256 W2left3
reg256 W2sigma1
reg256 W4right19
reg256 W4right61
reg256 W4right6
reg256 W4left45
reg256 W4left3
reg256 W4sigma1
reg256 W6right19
reg256 W6right61
reg256 W6right6
reg256 W6left45
reg256 W6left3
reg256 W6sigma1
reg256 W8right19
reg256 W8right61
reg256 W8right6
reg256 W8left45
reg256 W8left3
reg256 W8sigma1
reg256 W10right19
reg256 W10right61
reg256 W10right6
reg256 W10left45
reg256 W10left3
reg256 W10sigma1
reg256 W12right19
reg256 W12right61
reg256 W12right6
reg256 W12left45
reg256 W12left3
reg256 W12sigma1
reg256 W14right19
reg256 W14right61
reg256 W14right6
reg256 W14left45
reg256 W14left3
reg256 W14sigma1


enter inner

constants = input_3

bigendian64 = mem256[input_3+640]

X0 = mem256[input_0+0]
statebytes = input_0
X4 = mem256[input_0+32]

2x 16x X0 = X0[bigendian64]
2x 16x X4 = X4[bigendian64]

state0123 = X0

r11_stack = caller_r11
state4567 = X4
r13_stack = caller_r13
r12_stack = caller_r12
r14_stack = caller_r14
rbx_stack = caller_rbx
r15_stack = caller_r15
rbp_stack = caller_rbp

inlen_stack = input_2
in = input_1
statebytes_stack = statebytes

r0 = state0123[0]
r2 = state0123[2]
constants_stack = constants
r1 = state0123[1]
r3 = state0123[3]
r5 = state4567[1]
r4 = state4567[0]
r6 = state4567[2]
r7 = state4567[3]

new w

// interesting pads: 4,5,11,14,17,20,26
nop9
nop9
nop2

outerloop:

          X0 = mem256[in + 0]
          2x 16x X0 = X0[bigendian64]
    ch7 = r6
      r4Sigma1 = r4>>>14
    ch7 ^= r5

          4x D0 = X0 + mem256[constants + 0]
      r418 = r4>>>18
      r4Sigma1 ^= r418
    ch7 &= r4
      r441 = r4>>>41

      r4Sigma1 ^= r441
      r0Sigma0 = r0>>>28
    ch7 ^= r6
      r034 = r0>>>34

      r039 = r0>>>39
inplace state4567[3] = r7
      r0Sigma0 ^= r034
    r7 += ch7
  maj6 = r1
  maj6 ^= r0

      r0Sigma0 ^= r039
  r0andr1 = r1
  r0andr1 &= r0
      r7 += r4Sigma1

  maj7 = r2
          wc0123 = D0
    r7 += wc0123[0]
  maj7 &= maj6
inplace state0123[3] = r3

  r3 += r7
      r7 += r0Sigma0
  maj7 ^= r0andr1
            ch6 = r5

          r3Sigma1 = r3>>>14
            ch6 ^= r4
          X4 = mem256[in + 32]
          2x 16x X4 = X4[bigendian64]
  r7 += maj7
              r318 = r3>>>18

          4x D4 = X4 + mem256[constants + 32]
            ch6 &= r3
              r3Sigma1 ^= r318
          maj6 &= r7
inplace state4567[2] = r6

              r341 = r3>>>41
            ch6 ^= r5
          maj6 ^= r0andr1
              r7Sigma0 = r7>>>28
inplace state4567[1] = r5

            r6 += ch6
              r3Sigma1 ^= r341
              r734 = r7>>>34
inplace state0123[2] = r2
    r5 += wc0123[2]

              r739 = r7>>>39
              r7Sigma0 ^= r734
    ch5 = r4
    ch5 ^= r3
            r6 += wc0123[1]

              r6 += r3Sigma1
          mem256[&w + 0] = X0 # can skip &w+128 this time
              r7Sigma0 ^= r739

          r2 += r6
          r6 += maj6
inplace state4567[0] = r4

      r2Sigma1 = r2>>>14
              r6 += r7Sigma0
    ch5 &= r2
      r218 = r2>>>18
          mem256[&w + 32] = X4

      r2Sigma1 ^= r218
    ch5 ^= r4
      r241 = r2>>>41
  maj4 = r7
  maj4 ^= r6

      r6Sigma0 = r6>>>28
          wc4567 = D4
      r2Sigma1 ^= r241
    r5 += ch5
      r634 = r6>>>34
          in_stack = in

      r6Sigma0 ^= r634
      r639 = r6>>>39
  r6andr7 = r7
  r6andr7 &= r6

      r6Sigma0 ^= r639
  maj5 = r0
inplace state0123[1] = r1
      r5 += r2Sigma1
            r4 += wc0123[3]
  maj5 &= maj4

  r1 += r5
      r5 += r6Sigma0
  maj5 ^= r6andr7
            ch4 = r3

          r1Sigma1 = r1>>>14
            ch4 ^= r2
  r5 += maj5
            ch4 &= r1
              r118 = r1>>>18
inplace state0123[0] = r0

              r1Sigma1 ^= r118
          maj4 &= r5
            ch4 ^= r3
              r141 = r1>>>41
          X8 = mem256[in + 64]

              r1Sigma1 ^= r141
              r5Sigma0 = r5>>>28
          maj4 ^= r6andr7
            r4 += ch4

              r534 = r5>>>34
              r4 += r1Sigma1
              r5Sigma0 ^= r534
    r3 += wc4567[0]

          X12 = mem256[in + 96]
          r0 += r4
              r539 = r5>>>39
          r4 += maj4
              r5Sigma0 ^= r539

      r0Sigma1 = r0>>>14
              r4 += r5Sigma0
    ch3 = r2
      r018 = r0>>>18
    ch3 ^= r1

          2x 16x X8 = X8[bigendian64]
      r0Sigma1 ^= r018
    ch3 &= r0
      r041 = r0>>>41

      r4Sigma0 = r4>>>28
          4x D8 = X8 + mem256[constants + 64]
      r0Sigma1 ^= r041
    ch3 ^= r2
          mem256[&w + 64] = X8

    r3 += ch3
      r434 = r4>>>34
      r439 = r4>>>39
  maj2 = r5
  maj2 ^= r4
          wc891011 = D8

      r4Sigma0 ^= r434
      r3 += r0Sigma1
  r4andr5 = r5
  r4andr5 &= r4
            r2 += wc4567[1]

      r4Sigma0 ^= r439
  maj3 = r6
  maj3 &= maj2
  r7 += r3
      r3 += r4Sigma0

          2x 16x X12 = X12[bigendian64]
            ch2 = r1
  maj3 ^= r4andr5
            ch2 ^= r0
  r3 += maj3
          r7Sigma1 = r7>>>14

          4x D12 = X12 + mem256[constants + 96]
            ch2 &= r7
              r718 = r7>>>18

              r7Sigma1 ^= r718
          maj2 &= r3
            ch2 ^= r1
              r741 = r7>>>41
          mem256[&w + 96] = X12

              r7Sigma1 ^= r741
          maj2 ^= r4andr5
              r3Sigma0 = r3>>>28
          wc12131415 = D12
            r2 += ch2

              r2 += r7Sigma1
    r1 += wc4567[2]
    ch1 = r0
              r334 = r3>>>34
    ch1 ^= r7

              r3Sigma0 ^= r334
          r6 += r2
              r339 = r3>>>39
          r2 += maj2

              r3Sigma0 ^= r339
      r6Sigma1 = r6>>>14
      r618 = r6>>>18

      r641 = r6>>>41
    ch1 &= r6
              r2 += r3Sigma0
      r6Sigma1 ^= r618
      r2Sigma0 = r2>>>28

      r6Sigma1 ^= r641
    ch1 ^= r0
      r234 = r2>>>34
  maj0 = r3
  maj0 ^= r2

      r2Sigma0 ^= r234
    r1 += ch1
      r239 = r2>>>39
  r2andr3 = r3
  r2andr3 &= r2

      r2Sigma0 ^= r239
      r1 += r6Sigma1
  maj1 = r4
  maj1 &= maj0
            r0 += wc4567[3]
  r5 += r1
      r1 += r2Sigma0
            ch0 = r7
  maj1 ^= r2andr3
            ch0 ^= r6
          r5Sigma1 = r5>>>14
  r1 += maj1
            ch0 &= r5
              r518 = r5>>>18
              r5Sigma1 ^= r518
          maj0 &= r1
            ch0 ^= r7
              r541 = r5>>>41
              r5Sigma1 ^= r541
          maj0 ^= r2andr3
              r1Sigma0 = r1>>>28
            r0 += ch0
              r0 += r5Sigma1
              r134 = r1>>>34
              r1Sigma0 ^= r134
          r4 += r0
          r0 += maj0
              r139 = r1>>>39
              r1Sigma0 ^= r139
              r0 += r1Sigma0

  i = 4


  innerloop:

            X1 = mem256[&w + 8]
            4x X1right1 = X1 unsigned>> 1
      r4Sigma1 = r4>>>14
    r7 += wc891011[0]
            4x X1left63 = X1 << 63
    ch7 = r6
    ch7 ^= r5

      r418 = r4>>>18
    ch7 &= r4
  maj6 = r1
  maj6 ^= r0
            W14 = mem128[&w + 112],0

      r441 = r4>>>41
      r4Sigma1 ^= r418
    ch7 ^= r6
            X1sigma0 = X1right1 ^ X1left63

            4x X1right8 = X1 unsigned>> 8
      r4Sigma1 ^= r441
    r7 += ch7
      r0Sigma0 = r0>>>28

      r034 = r0>>>34
            X1sigma0 = X1sigma0 ^ X1right8
      r7 += r4Sigma1
  maj7 = r2
  maj7 &= maj6

      r0Sigma0 ^= r034
            2x,0 W14right19 = W14 unsigned>> 19
      r039 = r0>>>39

            4x X1left56 = X1 << 56
  r3 += r7
      r0Sigma0 ^= r039
    r6 += wc891011[1]
  r0andr1 = r1
  r0andr1 &= r0

            2x,0 W14left45 = W14 << 45
      r7 += r0Sigma0
  maj7 ^= r0andr1
    ch6 = r5
    ch6 ^= r4

            2x,0 W14right61 = W14 unsigned>> 61
      r3Sigma1 = r3>>>14
            X1sigma0 = X1sigma0 ^ X1left56
  r7 += maj7

      r318 = r3>>>18
            4x X1right7 = X1 unsigned>> 7
            1x,0 W14sigma1 = W14right19 ^ W14left45
    ch6 &= r3

      r3Sigma1 ^= r318
      r341 = r3>>>41
  maj6 &= r7
            X1sigma0 = X1sigma0 ^ X1right7

            1x,0 W14sigma1 ^= W14right61
            4x X0 = X0 + X1sigma0
      r3Sigma1 ^= r341
  maj6 ^= r0andr1

            2x,0 W14left3 = W14 << 3
      r7Sigma0 = r7>>>28
    ch6 ^= r5
      r6 += r3Sigma1

            4x X0 = X0 + mem256[&w + 72]
      r734 = r7>>>34
    r5 += wc891011[2]
    r6 += ch6

      r7Sigma0 ^= r734
      r739 = r7>>>39
  r2 += r6
            1x,0 W14sigma1 ^= W14left3

            2x,0 W14right6 = W14 unsigned>> 6
      r7Sigma0 ^= r739
  r6 += maj6
    ch5 = r4
    ch5 ^= r3

      r6 += r7Sigma0
      r2Sigma1 = r2>>>14
            1x,0 W14sigma1 ^= W14right6
    ch5 &= r2

      r218 = r2>>>18
      r241 = r2>>>41
            4x X0 = W14sigma1 + X0
    ch5 ^= r4

      r2Sigma1 ^= r218
            2x,0 W0right19 = X0 unsigned>> 19
      r6Sigma0 = r6>>>28
    r5 += ch5

            2x,0 W0left45 = X0 << 45
      r634 = r6>>>34
      r2Sigma1 ^= r241
  maj4 = r7

                            X5 = mem256[&w + 40]
  maj4 ^= r6
            2x,0 W0right61 = X0 unsigned>> 61
            1x,0 W0sigma1 = W0right19 ^ W0left45
      r6Sigma0 ^= r634
            1x,0 W0sigma1 ^= W0right61
      r639 = r6>>>39

            2x,0 W0left3 = X0 << 3
      r6Sigma0 ^= r639
      r5 += r2Sigma1

            2x,0 W0right6 = X0 unsigned>> 6
            1x,0 W0sigma1 ^= W0left3
  r1 += r5
  r6andr7 = r7
  r6andr7 &= r6

      r1Sigma1 = r1>>>14
      r5 += r6Sigma0
            1x,0 W0sigma1 ^= W0right6
  maj5 = r0
  maj5 &= maj4

            W0sigma1 = W0sigma1[2,3,0,1]
  maj5 ^= r6andr7
    ch4 = r3

                            4x X5right1 = X5 unsigned>> 1
    ch4 ^= r2
  r5 += maj5
      r118 = r1>>>18
    ch4 &= r1

  maj4 &= r5
    ch4 ^= r3
    r4 += wc891011[3]
      r1Sigma1 ^= r118

      r141 = r1>>>41
            4x X0 = X0 + W0sigma1
    r4 += ch4
  maj4 ^= r6andr7

      r5Sigma0 = r5>>>28
            4x D0 = X0 + mem256[constants + 128]
      r1Sigma1 ^= r141
      r534 = r5>>>34
            mem256[&w + 128] = X0

      r4 += r1Sigma1
      r5Sigma0 ^= r534
      r539 = r5>>>39
                    r3 += wc12131415[0]
            mem256[&w + 0] = X0

  r0 += r4
  r4 += maj4
      r5Sigma0 ^= r539
    constants += 128
            wc0123 = D0


                            W2 = mem128[&w + 16],0
      r4 += r5Sigma0
                      r0Sigma1 = r0>>>14
                    ch3 = r2
                    ch3 ^= r1
                
                            4x X5left63 = X5 << 63
                      r018 = r0>>>18
                  maj2 = r5
                    ch3 &= r0
                  maj2 ^= r4
                
                      r041 = r0>>>41
                            X5sigma0 = X5right1 ^ X5left63
                      r0Sigma1 ^= r018
                    ch3 ^= r2
                
                            4x X5right8 = X5 unsigned>> 8
                      r4Sigma0 = r4>>>28
                      r0Sigma1 ^= r041
                    r3 += ch3
                
                      r434 = r4>>>34
                            X5sigma0 = X5sigma0 ^ X5right8
                      r3 += r0Sigma1
                  maj3 = r6
                  maj3 &= maj2
                
                      r4Sigma0 ^= r434
                      r439 = r4>>>39
                            2x,0 W2right19 = W2 unsigned>> 19
                
                            4x X5left56 = X5 << 56
                  r7 += r3
                      r4Sigma0 ^= r439
                    r2 += wc12131415[1]
                  r4andr5 = r5
                  r4andr5 &= r4
                
                            2x,0 W2left45 = W2 << 45
                      r3 += r4Sigma0
                  maj3 ^= r4andr5
                    ch2 = r1
                    ch2 ^= r0
                
                            2x,0 W2right61 = W2 unsigned>> 61
                      r7Sigma1 = r7>>>14
                            X5sigma0 = X5sigma0 ^ X5left56
                  r3 += maj3
                
                            4x X5right7 = X5 unsigned>> 7
                      r718 = r7>>>18
                            1x,0 W2sigma1 = W2right19 ^ W2left45
                    ch2 &= r7
                
                      r7Sigma1 ^= r718
                            X5sigma0 = X5sigma0 ^ X5right7
                      r741 = r7>>>41
                  maj2 &= r3
                
                            1x,0 W2sigma1 ^= W2right61
                            4x X4 = X4 + X5sigma0
                      r7Sigma1 ^= r741
                  maj2 ^= r4andr5
                
                            2x,0 W2left3 = W2 << 3
                      r3Sigma0 = r3>>>28
                    ch2 ^= r1
                      r2 += r7Sigma1
                
                      r334 = r3>>>34
                            4x X4 = X4 + mem256[&w + 104]
                    r1 += wc12131415[2]
                    r2 += ch2
                
                      r3Sigma0 ^= r334
                      r339 = r3>>>39
                  r6 += r2
                            1x,0 W2sigma1 ^= W2left3
                
                            2x,0 W2right6 = W2 unsigned>> 6
                      r3Sigma0 ^= r339
                  r2 += maj2
                    ch1 = r0
                    ch1 ^= r7
                
                      r2 += r3Sigma0
                      r6Sigma1 = r6>>>14
                            1x,0 W2sigma1 ^= W2right6
                    ch1 &= r6
                
                      r618 = r6>>>18
                      r641 = r6>>>41
                            4x X4 = W2sigma1 + X4
                    ch1 ^= r0
                
                      r6Sigma1 ^= r618
                            2x,0 W4right19 = X4 unsigned>> 19
                    r1 += ch1
                      r2Sigma0 = r2>>>28
                
                            2x,0 W4left45 = X4 << 45
                      r6Sigma1 ^= r641
                      r234 = r2>>>34
                  maj0 = r3
                  maj0 ^= r2
                
                            2x,0 W4right61 = X4 unsigned>> 61
            X9 = mem256[&w + 72]
                      r2Sigma0 ^= r234
                            1x,0 W4sigma1 = W4right19 ^ W4left45
                      r239 = r2>>>39
                
                            2x,0 W4left3 = X4 << 3
                            1x,0 W4sigma1 ^= W4right61
                            2x,0 W4right6 = X4 unsigned>> 6
                      r2Sigma0 ^= r239
                      r1 += r6Sigma1
                
                            1x,0 W4sigma1 ^= W4left3
                  r5 += r1
                  r2andr3 = r3
                
                      r5Sigma1 = r5>>>14
                  r2andr3 &= r2
                            1x,0 W4sigma1 ^= W4right6
                      r1 += r2Sigma0
                  maj1 = r4
                  maj1 &= maj0
                
                            W4sigma1 = W4sigma1[2,3,0,1]
                  maj1 ^= r2andr3
                    ch0 = r7
                
            4x X9right1 = X9 unsigned>> 1
                    ch0 ^= r6
                  r1 += maj1
                    ch0 &= r5
                      r518 = r5>>>18
                
                  maj0 &= r1
                    r0 += wc12131415[3]
                    ch0 ^= r7
                      r5Sigma1 ^= r518
                
                            4x X4 = X4 + W4sigma1
                      r541 = r5>>>41
                            mem256[&w + 32] = X4
                    r0 += ch0
                  maj0 ^= r2andr3
                
                      r1Sigma0 = r1>>>28
                            4x D4 = X4 + mem256[constants + 32]
                            wc4567 = D4
                      r5Sigma1 ^= r541
                      r134 = r1>>>34
                
                      r0 += r5Sigma1
                      r1Sigma0 ^= r134
    r7 += wc0123[0]
                      r139 = r1>>>39
                
                  r4 += r0
                  r0 += maj0
                      r1Sigma0 ^= r139
                
      r4Sigma1 = r4>>>14
            W6 = mem128[&w + 48],0
                      r0 += r1Sigma0
    ch7 = r6
    ch7 ^= r5

      r418 = r4>>>18
            4x X9left63 = X9 << 63
    ch7 &= r4
  maj6 = r1
  maj6 ^= r0

      r441 = r4>>>41
            X9sigma0 = X9right1 ^ X9left63
      r4Sigma1 ^= r418

            4x X9right8 = X9 unsigned>> 8
    ch7 ^= r6
      r4Sigma1 ^= r441
      r0Sigma0 = r0>>>28
    r7 += ch7

            X9sigma0 = X9sigma0 ^ X9right8
      r7 += r4Sigma1
      r034 = r0>>>34
  maj7 = r2
  maj7 &= maj6

      r0Sigma0 ^= r034
            2x,0 W6right19 = W6 unsigned>> 19
      r039 = r0>>>39
  r3 += r7

      r0Sigma0 ^= r039
            4x X9left56 = X9 << 56
    r6 += wc0123[1]
  r0andr1 = r1
  r0andr1 &= r0

      r7 += r0Sigma0
    ch6 = r5
            2x,0 W6left45 = W6 << 45
  maj7 ^= r0andr1
    ch6 ^= r4

            2x,0 W6right61 = W6 unsigned>> 61
      r3Sigma1 = r3>>>14
  r7 += maj7
            X9sigma0 = X9sigma0 ^ X9left56

            4x X9right7 = X9 unsigned>> 7
      r318 = r3>>>18
            1x,0 W6sigma1 = W6right19 ^ W6left45
    ch6 &= r3

      r3Sigma1 ^= r318
      r341 = r3>>>41
            X9sigma0 = X9sigma0 ^ X9right7
  maj6 &= r7

            1x,0 W6sigma1 ^= W6right61
            4x X8 = X8 + X9sigma0
      r3Sigma1 ^= r341
  maj6 ^= r0andr1

            2x,0 W6left3 = W6 << 3
      r7Sigma0 = r7>>>28
    ch6 ^= r5
      r6 += r3Sigma1

            4x X8 = X8 + mem256[&w + 8]
      r734 = r7>>>34
    r5 += wc0123[2]
    r6 += ch6

      r7Sigma0 ^= r734
            1x,0 W6sigma1 ^= W6left3
      r739 = r7>>>39
  r2 += r6

      r7Sigma0 ^= r739
    ch5 = r4
            2x,0 W6right6 = W6 unsigned>> 6
  r6 += maj6
    ch5 ^= r3

      r6 += r7Sigma0
      r2Sigma1 = r2>>>14
            1x,0 W6sigma1 ^= W6right6
    ch5 &= r2

      r218 = r2>>>18
      r241 = r2>>>41
            4x X8 = W6sigma1 + X8

            2x,0 W8right19 = X8 unsigned>> 19
    ch5 ^= r4
      r2Sigma1 ^= r218
      r6Sigma0 = r6>>>28
    r5 += ch5

            2x,0 W8left45 = X8 << 45
      r634 = r6>>>34
      r2Sigma1 ^= r241
  maj4 = r7

            2x,0 W8right61 = X8 unsigned>> 61
  maj4 ^= r6
      r6Sigma0 ^= r634
            1x,0 W8sigma1 = W8right19 ^ W8left45
      r639 = r6>>>39

            1x,0 W8sigma1 ^= W8right61
            2x,0 W8left3 = X8 << 3
      r6Sigma0 ^= r639
      r5 += r2Sigma1
  r1 += r5

            2x,0 W8right6 = X8 unsigned>> 6
            1x,0 W8sigma1 ^= W8left3
  r6andr7 = r7
  r6andr7 &= r6

      r1Sigma1 = r1>>>14
            1x,0 W8sigma1 ^= W8right6
      r5 += r6Sigma0
  maj5 = r0
  maj5 &= maj4

            W8sigma1 = W8sigma1[2,3,0,1]
  maj5 ^= r6andr7
    ch4 = r3
    ch4 ^= r2

                            X13 = mem256[&w + 104]
                            4x X13right1 = X13 unsigned>> 1
  r5 += maj5
      r118 = r1>>>18
    ch4 &= r1

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
                    r3 += wc4567[0]

  r0 += r4
  r4 += maj4
                            W10 = mem128[&w + 80],0
      r5Sigma0 ^= r539

                      r0Sigma1 = r0>>>14
      r4 += r5Sigma0
                    ch3 = r2
                    ch3 ^= r1
                    ch3 &= r0
                
                            4x X13left63 = X13 << 63
                      r018 = r0>>>18
                  maj2 = r5
                  maj2 ^= r4
                
                            X13sigma0 = X13right1 ^ X13left63
                      r041 = r0>>>41
                      r0Sigma1 ^= r018
                    ch3 ^= r2
                
                            4x X13right8 = X13 unsigned>> 8
                      r4Sigma0 = r4>>>28
                      r0Sigma1 ^= r041
                    r3 += ch3
                
                      r434 = r4>>>34
                            X13sigma0 = X13sigma0 ^ X13right8
                      r3 += r0Sigma1
                  maj3 = r6
                  maj3 &= maj2
                
                            2x,0 W10right19 = W10 unsigned>> 19
                      r4Sigma0 ^= r434
                  r7 += r3
                      r439 = r4>>>39
                
                            4x X13left56 = X13 << 56
                      r4Sigma0 ^= r439
                    r2 += wc4567[1]
                  r4andr5 = r5
                
                            2x,0 W10left45 = W10 << 45
                  r4andr5 &= r4
                      r3 += r4Sigma0
                  maj3 ^= r4andr5
                    ch2 = r1
                
                            2x,0 W10right61 = W10 unsigned>> 61
                    ch2 ^= r0
                            X13sigma0 = X13sigma0 ^ X13left56
                      r7Sigma1 = r7>>>14
                
                            4x X13right7 = X13 unsigned>> 7
                  r3 += maj3
                      r718 = r7>>>18
                            1x,0 W10sigma1 = W10right19 ^ W10left45
                    ch2 &= r7
                
                      r7Sigma1 ^= r718
                      r741 = r7>>>41
                            X13sigma0 = X13sigma0 ^ X13right7
                  maj2 &= r3
                
                            1x,0 W10sigma1 ^= W10right61
                            4x X12 = X12 + X13sigma0
                      r7Sigma1 ^= r741
                  maj2 ^= r4andr5
                
                            2x,0 W10left3 = W10 << 3
                      r3Sigma0 = r3>>>28
                    ch2 ^= r1
                      r2 += r7Sigma1
                
                      r334 = r3>>>34
                            4x X12 = X12 + mem256[&w + 40]
                    r1 += wc4567[2]
                    r2 += ch2
                
                            1x,0 W10sigma1 ^= W10left3
                      r3Sigma0 ^= r334
                      r339 = r3>>>39
                
                            2x,0 W10right6 = W10 unsigned>> 6
                  r6 += r2
                      r3Sigma0 ^= r339
                    ch1 = r0
                  r2 += maj2
                    ch1 ^= r7
                
                      r2 += r3Sigma0
                      r6Sigma1 = r6>>>14
                            1x,0 W10sigma1 ^= W10right6
                    ch1 &= r6
                
                      r618 = r6>>>18
                    ch1 ^= r0
                            4x X12 = W10sigma1 + X12
                      r641 = r6>>>41
                
                            2x,0 W12right19 = X12 unsigned>> 19
                      r6Sigma1 ^= r618
                    r1 += ch1
                      r2Sigma0 = r2>>>28
                
                            2x,0 W12left45 = X12 << 45
                      r6Sigma1 ^= r641
                      r234 = r2>>>34
                  maj0 = r3
                  maj0 ^= r2
                
                      r2Sigma0 ^= r234
                            2x,0 W12right61 = X12 unsigned>> 61
                      r239 = r2>>>39
                            1x,0 W12sigma1 = W12right19 ^ W12left45
                
                            2x,0 W12left3 = X12 << 3
                            1x,0 W12sigma1 ^= W12right61
                      r2Sigma0 ^= r239
                      r1 += r6Sigma1
                
                            2x,0 W12right6 = X12 unsigned>> 6
                  r5 += r1
                            1x,0 W12sigma1 ^= W12left3
                  r2andr3 = r3
                  r2andr3 &= r2
                
                      r5Sigma1 = r5>>>14
                      r1 += r2Sigma0
                            1x,0 W12sigma1 ^= W12right6
                  maj1 = r4
                  maj1 &= maj0
                
                            W12sigma1 = W12sigma1[2,3,0,1]
                  maj1 ^= r2andr3
                    ch0 = r7
                    ch0 ^= r6
                
                  r1 += maj1
                    ch0 &= r5
                      r518 = r5>>>18
                
                  maj0 &= r1
                    r0 += wc4567[3]
                    ch0 ^= r7
                      r5Sigma1 ^= r518
                
                      r541 = r5>>>41
                            4x X12 = X12 + W12sigma1
                    r0 += ch0
                            mem256[&w + 96] = X12
                  maj0 ^= r2andr3
                
                      r1Sigma0 = r1>>>28
                            4x D12 = X12 + mem256[constants + 96]
                            wc12131415 = D12
                      r5Sigma1 ^= r541
                      r134 = r1>>>34
                
                      r0 += r5Sigma1
                      r1Sigma0 ^= r134
                      r139 = r1>>>39
                
                  r4 += r0
                  r0 += maj0
                      r1Sigma0 ^= r139
                
                      r0 += r1Sigma0




                       =? i -= 1
    goto innerloop if !=

#include "round89.q"

  in = in_stack

#include "round1011.q"

  inlen = inlen_stack
  in += 128

#include "round1213.q"

  inlen -= 128

#include "round1415.q"

  inlen_stack = inlen

  r7 += state4567[3]
  r3 += state0123[3]

  r6 += state4567[2]
  r2 += state0123[2]

  r5 += state4567[1]
  r1 += state0123[1]

  r4 += state4567[0]

  constants = constants_stack # or: constants -= 512
  r0 += state0123[0]
  
                     unsigned<? inlen - 128
  goto outerloop if !unsigned<


inplace state4567[2] = r6
inplace state4567[3] = r7
inplace state0123[2] = r2
inplace state4567[0] = r4
inplace state0123[3] = r3
inplace state4567[1] = r5
inplace state0123[0] = r0
inplace state0123[1] = r1


statebytes = statebytes_stack

X0 = state0123
X4 = state4567

2x 16x X0 = X0[bigendian64]
2x 16x X4 = X4[bigendian64]

mem256[statebytes+0] = X0
mem256[statebytes+32] = X4

vzeroupper

caller_r11 = r11_stack
caller_r12 = r12_stack
caller_r14 = r14_stack
caller_r13 = r13_stack
caller_r15 = r15_stack
caller_rbx = rbx_stack
caller_rbp = rbp_stack

return inlen
