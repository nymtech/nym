spill64 r0_spill
spill64 r1_spill
spill64 r2_spill
spill64 r3_spill
spill64 r4_spill
spill64 r5_spill
spill64 r6_spill
spill64 r7_spill
spill64 w0_spill
spill64 w1_spill
spill64 w2_spill
spill64 w3_spill
spill64 w4_spill
spill64 w5_spill
spill64 w6_spill
spill64 w7_spill
stack64 w8_stack
stack64 state0
stack64 state1
stack64 state2
stack64 state3
stack64 state4
stack64 state5
stack64 state6
stack64 state7
int64 r0
int64 r1
int64 r2
int64 r3
int64 r4
int64 r5
int64 r6
int64 r7
int64 w0
int64 w1
int64 w2
int64 w3
int64 w4
int64 w5
int64 w6
int64 w7
stack64 w0_next
stack64 w1_next
stack64 w2_next
stack64 w3_next
stack64 w4_next
stack64 w5_next
stack64 w6_next
stack64 w7_next
int64 w8
int64 w9
int64 w10
int64 w11
int64 w12
int64 w13
int64 w14
int64 w15
int32 i
ptr statebytes
ptr in
int32 inlen
ptr constants
stackptr in_stack
stackptr statebytes_stack
stackptr constants_stack
stack32 inlen_stack
stack32 i_stack

pushenter inner

statebytes_stack = input_0
in_stack = input_1

inlen = input_2 - 128
inlen_stack = inlen

constants_stack = input_3

r0 = flip mem64[input_0]
r1 = flip mem64[input_0+8]
r2 = flip mem64[input_0+16]
r3 = flip mem64[input_0+24]

r0 = reverse flip r0
r1 = reverse flip r1
r2 = reverse flip r2
r3 = reverse flip r3

state0 = r0
state1 = r1
state2 = r2
state3 = r3

r0_spill = r0
r1_spill = r1
r2_spill = r2
r3_spill = r3

r4 = flip mem64[input_0+32]
r5 = flip mem64[input_0+40]
r6 = flip mem64[input_0+48]
r7 = flip mem64[input_0+56]

r4 = reverse flip r4
r5 = reverse flip r5
r6 = reverse flip r6
r7 = reverse flip r7

state4 = r4
state5 = r5
state6 = r6
state7 = r7

r4_spill = r4
r5_spill = r5
r6_spill = r6
r7_spill = r7

mainloop:

  in = in_stack

  w0 = flip mem64[in]; in += 8
  w1 = flip mem64[in]; in += 8
  w2 = flip mem64[in]; in += 8
  w3 = flip mem64[in]; in += 8

  w0 = reverse flip w0
  w1 = reverse flip w1
  w2 = reverse flip w2
  w3 = reverse flip w3

  w0_spill = w0
  w1_spill = w1
  w2_spill = w2
  w3_spill = w3

  w4 = flip mem64[in]; in += 8
  w5 = flip mem64[in]; in += 8
  w6 = flip mem64[in]; in += 8
  w7 = flip mem64[in]; in += 8

  w4 = reverse flip w4
  w5 = reverse flip w5
  w6 = reverse flip w6
  w7 = reverse flip w7

  w4_spill = w4
  w5_spill = w5
  w6_spill = w6
  w7_spill = w7

  w8 = flip mem64[in]; in += 8
  w9 = flip mem64[in]; in += 8
  w10 = flip mem64[in]; in += 8
  w11 = flip mem64[in]; in += 8

  w8 = reverse flip w8
  w9 = reverse flip w9
  w10 = reverse flip w10
  w11 = reverse flip w11

  w0_next = w8
  w1_next = w9
  w2_next = w10
  w3_next = w11

  w12 = flip mem64[in]; in += 8
  w13 = flip mem64[in]; in += 8
  w14 = flip mem64[in]; in += 8
  w15 = flip mem64[in]; in += 8

  w12 = reverse flip w12
  w13 = reverse flip w13
  w14 = reverse flip w14
  w15 = reverse flip w15

  w4_next = w12
  w5_next = w13
  w6_next = w14
  w7_next = w15

  in_stack = in

  i = 80 simple
  i_stack = i

  innerloop:

    assign 0 to r0_spill
    assign 1 to r1_spill
    assign 2 to r2_spill
    assign 3 to r3_spill
    assign 4 to r4_spill
    assign 5 to r5_spill
    assign 6 to r6_spill
    assign 7 to r7_spill

    constants = constants_stack

      r3 = r3_spill
      r4 = r4_spill
      r5 = r5_spill
      r6 = r6_spill
      r7 = r7_spill

      w0 = w0_spill
    Sigma1_setup
    r7 += w0 + mem64[constants] + Sigma1(r4) + Ch(r4,r5,r6); constants += 8
    r3 += r7
      r3_spill = r3
      r0 = r0_spill
      r1 = r1_spill
      r2 = r2_spill
    Sigma0_setup
    r7 += Sigma0(r0) + Maj(r0,r1,r2)
      r7_spill = r7

      r4 = r4_spill
      r5 = r5_spill
      r6 = r6_spill
      w1 = w1_spill
    Sigma1_setup
    r6 += w1 + mem64[constants] + Sigma1(r3) + Ch(r3,r4,r5); constants += 8
    r2 += r6
      r2_spill = r2
      r7 = r7_spill
      r0 = r0_spill
      r1 = r1_spill
    Sigma0_setup
    r6 += Sigma0(r7) + Maj(r7,r0,r1)
      r6_spill = r6

    assign 0 to r0_spill
    assign 1 to r1_spill
    assign 2 to r2_spill
    assign 3 to r3_spill
    assign 4 to r4_spill
    assign 5 to r5_spill
    assign 6 to r6_spill
    assign 7 to r7_spill

      r3 = r3_spill
      r4 = r4_spill
      r5 = r5_spill
      w2 = w2_spill
    Sigma1_setup
    r5 += w2 + mem64[constants] + Sigma1(r2) + Ch(r2,r3,r4); constants += 8
    r1 += r5
      r1_spill = r1
      r6 = r6_spill
      r7 = r7_spill
      r0 = r0_spill
    Sigma0_setup
    r5 += Sigma0(r6) + Maj(r6,r7,r0)
      r5_spill = r5

      r2 = r2_spill
      r3 = r3_spill
      r4 = r4_spill
      w3 = w3_spill
    Sigma1_setup
    r4 += w3 + mem64[constants] + Sigma1(r1) + Ch(r1,r2,r3); constants += 8
    r0 += r4
      r0_spill = r0
      r5 = r5_spill
      r6 = r6_spill
      r7 = r7_spill
    Sigma0_setup
    r4 += Sigma0(r5) + Maj(r5,r6,r7)
      r4_spill = r4

    assign 0 to r0_spill
    assign 1 to r1_spill
    assign 2 to r2_spill
    assign 3 to r3_spill
    assign 4 to r4_spill
    assign 5 to r5_spill
    assign 6 to r6_spill
    assign 7 to r7_spill

      r1 = r1_spill
      r2 = r2_spill
      r3 = r3_spill
      w4 = w4_spill
    Sigma1_setup
    r3 += w4 + mem64[constants] + Sigma1(r0) + Ch(r0,r1,r2); constants += 8
    r7 += r3
      r7_spill = r7
      r4 = r4_spill
      r5 = r5_spill
      r6 = r6_spill
    Sigma0_setup
    r3 += Sigma0(r4) + Maj(r4,r5,r6)
      r3_spill = r3

      r0 = r0_spill
      r1 = r1_spill
      r2 = r2_spill
      w5 = w5_spill
    Sigma1_setup
    r2 += w5 + mem64[constants] + Sigma1(r7) + Ch(r7,r0,r1); constants += 8
    r6 += r2
      r6_spill = r6
      r3 = r3_spill
      r4 = r4_spill
      r5 = r5_spill
    Sigma0_setup
    r2 += Sigma0(r3) + Maj(r3,r4,r5)
      r2_spill = r2

    assign 0 to r0_spill
    assign 1 to r1_spill
    assign 2 to r2_spill
    assign 3 to r3_spill
    assign 4 to r4_spill
    assign 5 to r5_spill
    assign 6 to r6_spill
    assign 7 to r7_spill

      r7 = r7_spill
      r0 = r0_spill
      r1 = r1_spill
      w6 = w6_spill
    Sigma1_setup
    r1 += w6 + mem64[constants] + Sigma1(r6) + Ch(r6,r7,r0); constants += 8
    r5 += r1
      r5_spill = r5
      r2 = r2_spill
      r3 = r3_spill
      r4 = r4_spill
    Sigma0_setup
    r1 += Sigma0(r2) + Maj(r2,r3,r4)
      r1_spill = r1

      r6 = r6_spill
      r7 = r7_spill
      r0 = r0_spill
      w7 = w7_spill
    Sigma1_setup
    r0 += w7 + mem64[constants] + Sigma1(r5) + Ch(r5,r6,r7); constants += 8
    r4 += r0
      r4_spill = r4
      r1 = r1_spill
      r2 = r2_spill
      r3 = r3_spill
    Sigma0_setup
    r0 += Sigma0(r1) + Maj(r1,r2,r3)
      r0_spill = r0

    constants_stack = constants

    assign 8 to w0_spill
    assign 9 to w1_spill
    assign 10 to w2_spill
    assign 11 to w3_spill
    assign 12 to w4_spill
    assign 13 to w5_spill
    assign 14 to w6_spill
    assign 15 to w7_spill

    i = i_stack
                         =? i -= 8
    goto endinnerloop if =
    i_stack = i

                    =? i - 8
    goto nearend if =

      sigma1_setup
      sigma0_setup

      w8 = w0_spill
      w9 = w1_spill
      w6 = w6_next
      w1 = w1_next

      w8  += sigma1(w6)
      w8  += sigma0(w9)

      w8  += w1
        w1_spill = w1

      w7 = w7_next
        w8_stack = w8

      w9  += sigma1(w7)
      w10 = w2_spill
      w9  += sigma0(w10)

      w2 = w2_next
      w9  += w2
        w2_spill = w2
        w1_next = w9

      w10 += sigma1(w8)
      w11 = w3_spill
      w10 += sigma0(w11)

      w3 = w3_next
      w10 += w3
        w3_spill = w3
        w2_next = w10

      w11 += sigma1(w9)
      w12 = w4_spill
      w11 += sigma0(w12)

      w4 = w4_next
      w11 += w4
        w4_spill = w4
        w3_next = w11

      w12 += sigma1(w10)
      w13 = w5_spill
      w12 += sigma0(w13)

      w5 = w5_next
      w12 += w5
        w5_spill = w5
        w4_next = w12

      w13 += sigma1(w11)

      w14 = w6_spill
        w6_spill = w6

      w13 += sigma0(w14)
      w13 += w6
        w5_next = w13

      w14 += sigma1(w12)

      w15 = w7_spill
        w7_spill = w7

      w14 += sigma0(w15)
      w14 += w7
        w6_next = w14

      w15 += sigma1(w13)

      w0 = w0_next
        w8 = w8_stack
        w0_next = w8

      w15 += sigma0(w0)
      w15 += w8
        w7_next = w15

        w0_spill = w0

    goto innerloop

    nearend:

      w0 = w0_next
      w1 = w1_next
      w2 = w2_next
      w3 = w3_next

      w0_spill = w0
      w1_spill = w1
      w2_spill = w2
      w3_spill = w3

      w4 = w4_next
      w5 = w5_next
      w6 = w6_next
      w7 = w7_next

      w4_spill = w4
      w5_spill = w5
      w6_spill = w6
      w7_spill = w7

    goto innerloop
  endinnerloop:

  constants = constants_stack
  constants -= 640
  constants_stack = constants

  r0 = r0_spill
  r1 = r1_spill
  r2 = r2_spill
  r3 = r3_spill

  r0 += state0
  r1 += state1
  r2 += state2
  r3 += state3

  state0 = r0
  state1 = r1
  state2 = r2
  state3 = r3

  r0_spill = r0
  r1_spill = r1
  r2_spill = r2
  r3_spill = r3

  r4 = r4_spill
  r5 = r5_spill
  r6 = r6_spill
  r7 = r7_spill

  r4 += state4
  r5 += state5
  r6 += state6
  r7 += state7

  state4 = r4
  state5 = r5
  state6 = r6
  state7 = r7

  r4_spill = r4
  r5_spill = r5
  r6_spill = r6
  r7_spill = r7

  inlen = inlen_stack

                   unsigned>=? inlen -= 128
  inlen_stack = inlen
  goto mainloop if unsigned>=
endmainloop:

statebytes = statebytes_stack

r0 = state0
r1 = state1
r2 = state2
r3 = state3

r0 = reverse flip r0
r1 = reverse flip r1
r2 = reverse flip r2
r3 = reverse flip r3

mem64[statebytes] = flip r0
mem64[statebytes+8] = flip r1
mem64[statebytes+16] = flip r2
mem64[statebytes+24] = flip r3

r4 = state4
r5 = state5
r6 = state6
r7 = state7

r4 = reverse flip r4
r5 = reverse flip r5
r6 = reverse flip r6
r7 = reverse flip r7

mem64[statebytes+32] = flip r4
mem64[statebytes+40] = flip r5
mem64[statebytes+48] = flip r6
mem64[statebytes+56] = flip r7

inlen += 128
popreturn inlen
