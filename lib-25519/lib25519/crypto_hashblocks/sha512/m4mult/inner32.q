int32 two13
int32 two23
int32 two24
int32 two25
int32 lotmp
int32 lotmp2
int32 hitmp
int32 hitmp2
int32 lou0
int32 lou1
int32 lou2
int32 lou3
int32 lou4
int32 lou5
int32 hiu0
int32 hiu1
int32 hiu2
int32 hiu3
int32 hiu4
int32 hiu5
float32 hid0
float32 hid1
float32 hid2
float32 hid3
float32 hid4
float32 hid5
float32 hid6
float32 hid7
float32 hid8
float32 hid9
float32 hid10
float32 hid11
float32 hid12
float32 hid13
float32 hid14
float32 hid15
float32 lod0
float32 lod1
float32 lod2
float32 lod3
float32 lod4
float32 lod5
float32 lod6
float32 lod7
float32 lod8
float32 lod9
float32 lod10
float32 lod11
float32 lod12
float32 lod13
float32 lod14
float32 lod15
stack32 him0
stack32 him1
stack32 him2
stack32 him3
stack32 him4
stack32 him5
stack32 him6
stack32 him7
stack32 him8
stack32 him9
stack32 him10
stack32 him11
stack32 him12
stack32 him13
stack32 him14
stack32 him15
stack32 lom0
stack32 lom1
stack32 lom2
stack32 lom3
stack32 lom4
stack32 lom5
stack32 lom6
stack32 lom7
stack32 lom8
stack32 lom9
stack32 lom10
stack32 lom11
stack32 lom12
stack32 lom13
stack32 lom14
stack32 lom15
stack32 o0
stack32 o1
stack32 o2
stack32 o3
stack32 o4
# qhasm: int32 input_0
# qhasm: int32 input_1
# qhasm: int32 input_2
# qhasm: int32 input_3
# qhasm: spill64 r0_spill
# qhasm: spill64 r1_spill
# qhasm: spill64 r2_spill
# qhasm: spill64 r3_spill
# qhasm: spill64 r4_spill
# qhasm: spill64 r5_spill
# qhasm: spill64 r6_spill
# qhasm: spill64 r7_spill
# qhasm: spill64 w0_spill
# qhasm: spill64 w1_spill
# qhasm: spill64 w2_spill
# qhasm: spill64 w3_spill
# qhasm: spill64 w4_spill
# qhasm: spill64 w5_spill
# qhasm: spill64 w6_spill
# qhasm: spill64 w7_spill
# qhasm: stack64 w8_stack
# qhasm: stack64 state0
# qhasm: stack64 state1
# qhasm: stack64 state2
# qhasm: stack64 state3
# qhasm: stack64 state4
# qhasm: stack64 state5
# qhasm: stack64 state6
# qhasm: stack64 state7
# qhasm: int64 r0
# qhasm: int64 r1
# qhasm: int64 r2
# qhasm: int64 r3
# qhasm: int64 r4
# qhasm: int64 r5
# qhasm: int64 r6
# qhasm: int64 r7
# qhasm: int64 w0
# qhasm: int64 w1
# qhasm: int64 w2
# qhasm: int64 w3
# qhasm: int64 w4
# qhasm: int64 w5
# qhasm: int64 w6
# qhasm: int64 w7
# qhasm: stack64 w0_next
# qhasm: stack64 w1_next
# qhasm: stack64 w2_next
# qhasm: stack64 w3_next
# qhasm: stack64 w4_next
# qhasm: stack64 w5_next
# qhasm: stack64 w6_next
# qhasm: stack64 w7_next
# qhasm: int64 w8
# qhasm: int64 w9
# qhasm: int64 w10
# qhasm: int64 w11
# qhasm: int64 w12
# qhasm: int64 w13
# qhasm: int64 w14
# qhasm: int64 w15
# qhasm: int32 i
# qhasm: ptr statebytes
# qhasm: ptr in
# qhasm: int32 inlen
# qhasm: ptr constants
# qhasm: stackptr in_stack
# qhasm: stackptr statebytes_stack
# qhasm: stackptr constants_stack
# qhasm: stack32 inlen_stack
# qhasm: stack32 i_stack
# qhasm: pushenter inner
pushenter inner
# qhasm: statebytes_stack = input_0
# asm 1: >statebytes_stack=stack32#1 = <input_0=int32#1
# asm 2: >statebytes_stack=o0 = <input_0=input_0
o0 = input_0
# qhasm: in_stack = input_1
# asm 1: >in_stack=stack32#2 = <input_1=int32#2
# asm 2: >in_stack=o1 = <input_1=input_1
o1 = input_1
# qhasm: inlen = input_2 - 128
# asm 1: >inlen=int32#2 = <input_2=int32#3 - 128
# asm 2: >inlen=input_1 = <input_2=input_2 - 128
input_1 = input_2 - 128
# qhasm: inlen_stack = inlen
# asm 1: >inlen_stack=stack32#3 = <inlen=int32#2
# asm 2: >inlen_stack=o2 = <inlen=input_1
o2 = input_1
# qhasm: constants_stack = input_3
# asm 1: >constants_stack=stack32#4 = <input_3=int32#4
# asm 2: >constants_stack=o3 = <input_3=input_3
o3 = input_3
# qhasm: r0 = flip mem64[input_0]
# asm 1: hi>r0=int64#1 = mem32[<input_0=int32#1]
# asm 2: hi>r0=u0 = mem32[<input_0=input_0]
hiu0 = mem32[input_0]
# asm 1: lo>r0=int64#1 = mem32[<input_0=int32#1+4]
# asm 2: lo>r0=u0 = mem32[<input_0=input_0+4]
lou0 = mem32[input_0+4]
# qhasm: r1 = flip mem64[input_0+8]
# asm 1: hi>r1=int64#2 = mem32[<input_0=int32#1+8]
# asm 2: hi>r1=u1 = mem32[<input_0=input_0+8]
hiu1 = mem32[input_0+8]
# asm 1: lo>r1=int64#2 = mem32[<input_0=int32#1+12]
# asm 2: lo>r1=u1 = mem32[<input_0=input_0+12]
lou1 = mem32[input_0+12]
# qhasm: r2 = flip mem64[input_0+16]
# asm 1: hi>r2=int64#3 = mem32[<input_0=int32#1+16]
# asm 2: hi>r2=u2 = mem32[<input_0=input_0+16]
hiu2 = mem32[input_0+16]
# asm 1: lo>r2=int64#3 = mem32[<input_0=int32#1+20]
# asm 2: lo>r2=u2 = mem32[<input_0=input_0+20]
lou2 = mem32[input_0+20]
# qhasm: r3 = flip mem64[input_0+24]
# asm 1: hi>r3=int64#4 = mem32[<input_0=int32#1+24]
# asm 2: hi>r3=u3 = mem32[<input_0=input_0+24]
hiu3 = mem32[input_0+24]
# asm 1: lo>r3=int64#4 = mem32[<input_0=int32#1+28]
# asm 2: lo>r3=u3 = mem32[<input_0=input_0+28]
lou3 = mem32[input_0+28]
# qhasm: r0 = reverse flip r0
# asm 1: lo>r0=int64#1 = lo<r0=int64#1[3]lo<r0=int64#1[2]lo<r0=int64#1[1]lo<r0=int64#1[0]
# asm 2: lo>r0=u0 = lo<r0=u0[3]lo<r0=u0[2]lo<r0=u0[1]lo<r0=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>r0=int64#1 = hi<r0=int64#1[3]hi<r0=int64#1[2]hi<r0=int64#1[1]hi<r0=int64#1[0]
# asm 2: hi>r0=u0 = hi<r0=u0[3]hi<r0=u0[2]hi<r0=u0[1]hi<r0=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm: r1 = reverse flip r1
# asm 1: lo>r1=int64#2 = lo<r1=int64#2[3]lo<r1=int64#2[2]lo<r1=int64#2[1]lo<r1=int64#2[0]
# asm 2: lo>r1=u1 = lo<r1=u1[3]lo<r1=u1[2]lo<r1=u1[1]lo<r1=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>r1=int64#2 = hi<r1=int64#2[3]hi<r1=int64#2[2]hi<r1=int64#2[1]hi<r1=int64#2[0]
# asm 2: hi>r1=u1 = hi<r1=u1[3]hi<r1=u1[2]hi<r1=u1[1]hi<r1=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm: r2 = reverse flip r2
# asm 1: lo>r2=int64#3 = lo<r2=int64#3[3]lo<r2=int64#3[2]lo<r2=int64#3[1]lo<r2=int64#3[0]
# asm 2: lo>r2=u2 = lo<r2=u2[3]lo<r2=u2[2]lo<r2=u2[1]lo<r2=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>r2=int64#3 = hi<r2=int64#3[3]hi<r2=int64#3[2]hi<r2=int64#3[1]hi<r2=int64#3[0]
# asm 2: hi>r2=u2 = hi<r2=u2[3]hi<r2=u2[2]hi<r2=u2[1]hi<r2=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm: r3 = reverse flip r3
# asm 1: lo>r3=int64#4 = lo<r3=int64#4[3]lo<r3=int64#4[2]lo<r3=int64#4[1]lo<r3=int64#4[0]
# asm 2: lo>r3=u3 = lo<r3=u3[3]lo<r3=u3[2]lo<r3=u3[1]lo<r3=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>r3=int64#4 = hi<r3=int64#4[3]hi<r3=int64#4[2]hi<r3=int64#4[1]hi<r3=int64#4[0]
# asm 2: hi>r3=u3 = hi<r3=u3[3]hi<r3=u3[2]hi<r3=u3[1]hi<r3=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm: state0 = r0
# asm 1: lo>state0=stack64#1 = lo<r0=int64#1
# asm 2: lo>state0=m0 = lo<r0=u0
lom0 = lou0
# asm 1: hi>state0=stack64#1 = hi<r0=int64#1
# asm 2: hi>state0=m0 = hi<r0=u0
him0 = hiu0
# qhasm: state1 = r1
# asm 1: lo>state1=stack64#2 = lo<r1=int64#2
# asm 2: lo>state1=m1 = lo<r1=u1
lom1 = lou1
# asm 1: hi>state1=stack64#2 = hi<r1=int64#2
# asm 2: hi>state1=m1 = hi<r1=u1
him1 = hiu1
# qhasm: state2 = r2
# asm 1: lo>state2=stack64#3 = lo<r2=int64#3
# asm 2: lo>state2=m2 = lo<r2=u2
lom2 = lou2
# asm 1: hi>state2=stack64#3 = hi<r2=int64#3
# asm 2: hi>state2=m2 = hi<r2=u2
him2 = hiu2
# qhasm: state3 = r3
# asm 1: lo>state3=stack64#4 = lo<r3=int64#4
# asm 2: lo>state3=m3 = lo<r3=u3
lom3 = lou3
# asm 1: hi>state3=stack64#4 = hi<r3=int64#4
# asm 2: hi>state3=m3 = hi<r3=u3
him3 = hiu3
# qhasm: r0_spill = r0
# asm 1: lo>r0_spill=spill64#1 = lo<r0=int64#1
# asm 2: lo>r0_spill=d0 = lo<r0=u0
lod0 = lou0
# asm 1: hi>r0_spill=spill64#1 = hi<r0=int64#1
# asm 2: hi>r0_spill=d0 = hi<r0=u0
hid0 = hiu0
# qhasm: r1_spill = r1
# asm 1: lo>r1_spill=spill64#2 = lo<r1=int64#2
# asm 2: lo>r1_spill=d1 = lo<r1=u1
lod1 = lou1
# asm 1: hi>r1_spill=spill64#2 = hi<r1=int64#2
# asm 2: hi>r1_spill=d1 = hi<r1=u1
hid1 = hiu1
# qhasm: r2_spill = r2
# asm 1: lo>r2_spill=spill64#3 = lo<r2=int64#3
# asm 2: lo>r2_spill=d2 = lo<r2=u2
lod2 = lou2
# asm 1: hi>r2_spill=spill64#3 = hi<r2=int64#3
# asm 2: hi>r2_spill=d2 = hi<r2=u2
hid2 = hiu2
# qhasm: r3_spill = r3
# asm 1: lo>r3_spill=spill64#4 = lo<r3=int64#4
# asm 2: lo>r3_spill=d3 = lo<r3=u3
lod3 = lou3
# asm 1: hi>r3_spill=spill64#4 = hi<r3=int64#4
# asm 2: hi>r3_spill=d3 = hi<r3=u3
hid3 = hiu3
# qhasm: r4 = flip mem64[input_0+32]
# asm 1: hi>r4=int64#1 = mem32[<input_0=int32#1+32]
# asm 2: hi>r4=u0 = mem32[<input_0=input_0+32]
hiu0 = mem32[input_0+32]
# asm 1: lo>r4=int64#1 = mem32[<input_0=int32#1+36]
# asm 2: lo>r4=u0 = mem32[<input_0=input_0+36]
lou0 = mem32[input_0+36]
# qhasm: r5 = flip mem64[input_0+40]
# asm 1: hi>r5=int64#2 = mem32[<input_0=int32#1+40]
# asm 2: hi>r5=u1 = mem32[<input_0=input_0+40]
hiu1 = mem32[input_0+40]
# asm 1: lo>r5=int64#2 = mem32[<input_0=int32#1+44]
# asm 2: lo>r5=u1 = mem32[<input_0=input_0+44]
lou1 = mem32[input_0+44]
# qhasm: r6 = flip mem64[input_0+48]
# asm 1: hi>r6=int64#3 = mem32[<input_0=int32#1+48]
# asm 2: hi>r6=u2 = mem32[<input_0=input_0+48]
hiu2 = mem32[input_0+48]
# asm 1: lo>r6=int64#3 = mem32[<input_0=int32#1+52]
# asm 2: lo>r6=u2 = mem32[<input_0=input_0+52]
lou2 = mem32[input_0+52]
# qhasm: r7 = flip mem64[input_0+56]
# asm 1: hi>r7=int64#4 = mem32[<input_0=int32#1+56]
# asm 2: hi>r7=u3 = mem32[<input_0=input_0+56]
hiu3 = mem32[input_0+56]
# asm 1: lo>r7=int64#4 = mem32[<input_0=int32#1+60]
# asm 2: lo>r7=u3 = mem32[<input_0=input_0+60]
lou3 = mem32[input_0+60]
# qhasm: r4 = reverse flip r4
# asm 1: lo>r4=int64#1 = lo<r4=int64#1[3]lo<r4=int64#1[2]lo<r4=int64#1[1]lo<r4=int64#1[0]
# asm 2: lo>r4=u0 = lo<r4=u0[3]lo<r4=u0[2]lo<r4=u0[1]lo<r4=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>r4=int64#1 = hi<r4=int64#1[3]hi<r4=int64#1[2]hi<r4=int64#1[1]hi<r4=int64#1[0]
# asm 2: hi>r4=u0 = hi<r4=u0[3]hi<r4=u0[2]hi<r4=u0[1]hi<r4=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm: r5 = reverse flip r5
# asm 1: lo>r5=int64#2 = lo<r5=int64#2[3]lo<r5=int64#2[2]lo<r5=int64#2[1]lo<r5=int64#2[0]
# asm 2: lo>r5=u1 = lo<r5=u1[3]lo<r5=u1[2]lo<r5=u1[1]lo<r5=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>r5=int64#2 = hi<r5=int64#2[3]hi<r5=int64#2[2]hi<r5=int64#2[1]hi<r5=int64#2[0]
# asm 2: hi>r5=u1 = hi<r5=u1[3]hi<r5=u1[2]hi<r5=u1[1]hi<r5=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm: r6 = reverse flip r6
# asm 1: lo>r6=int64#3 = lo<r6=int64#3[3]lo<r6=int64#3[2]lo<r6=int64#3[1]lo<r6=int64#3[0]
# asm 2: lo>r6=u2 = lo<r6=u2[3]lo<r6=u2[2]lo<r6=u2[1]lo<r6=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>r6=int64#3 = hi<r6=int64#3[3]hi<r6=int64#3[2]hi<r6=int64#3[1]hi<r6=int64#3[0]
# asm 2: hi>r6=u2 = hi<r6=u2[3]hi<r6=u2[2]hi<r6=u2[1]hi<r6=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm: r7 = reverse flip r7
# asm 1: lo>r7=int64#4 = lo<r7=int64#4[3]lo<r7=int64#4[2]lo<r7=int64#4[1]lo<r7=int64#4[0]
# asm 2: lo>r7=u3 = lo<r7=u3[3]lo<r7=u3[2]lo<r7=u3[1]lo<r7=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>r7=int64#4 = hi<r7=int64#4[3]hi<r7=int64#4[2]hi<r7=int64#4[1]hi<r7=int64#4[0]
# asm 2: hi>r7=u3 = hi<r7=u3[3]hi<r7=u3[2]hi<r7=u3[1]hi<r7=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm: state4 = r4
# asm 1: lo>state4=stack64#5 = lo<r4=int64#1
# asm 2: lo>state4=m4 = lo<r4=u0
lom4 = lou0
# asm 1: hi>state4=stack64#5 = hi<r4=int64#1
# asm 2: hi>state4=m4 = hi<r4=u0
him4 = hiu0
# qhasm: state5 = r5
# asm 1: lo>state5=stack64#6 = lo<r5=int64#2
# asm 2: lo>state5=m5 = lo<r5=u1
lom5 = lou1
# asm 1: hi>state5=stack64#6 = hi<r5=int64#2
# asm 2: hi>state5=m5 = hi<r5=u1
him5 = hiu1
# qhasm: state6 = r6
# asm 1: lo>state6=stack64#7 = lo<r6=int64#3
# asm 2: lo>state6=m6 = lo<r6=u2
lom6 = lou2
# asm 1: hi>state6=stack64#7 = hi<r6=int64#3
# asm 2: hi>state6=m6 = hi<r6=u2
him6 = hiu2
# qhasm: state7 = r7
# asm 1: lo>state7=stack64#8 = lo<r7=int64#4
# asm 2: lo>state7=m7 = lo<r7=u3
lom7 = lou3
# asm 1: hi>state7=stack64#8 = hi<r7=int64#4
# asm 2: hi>state7=m7 = hi<r7=u3
him7 = hiu3
# qhasm: r4_spill = r4
# asm 1: lo>r4_spill=spill64#5 = lo<r4=int64#1
# asm 2: lo>r4_spill=d4 = lo<r4=u0
lod4 = lou0
# asm 1: hi>r4_spill=spill64#5 = hi<r4=int64#1
# asm 2: hi>r4_spill=d4 = hi<r4=u0
hid4 = hiu0
# qhasm: r5_spill = r5
# asm 1: lo>r5_spill=spill64#6 = lo<r5=int64#2
# asm 2: lo>r5_spill=d5 = lo<r5=u1
lod5 = lou1
# asm 1: hi>r5_spill=spill64#6 = hi<r5=int64#2
# asm 2: hi>r5_spill=d5 = hi<r5=u1
hid5 = hiu1
# qhasm: r6_spill = r6
# asm 1: lo>r6_spill=spill64#7 = lo<r6=int64#3
# asm 2: lo>r6_spill=d6 = lo<r6=u2
lod6 = lou2
# asm 1: hi>r6_spill=spill64#7 = hi<r6=int64#3
# asm 2: hi>r6_spill=d6 = hi<r6=u2
hid6 = hiu2
# qhasm: r7_spill = r7
# asm 1: lo>r7_spill=spill64#8 = lo<r7=int64#4
# asm 2: lo>r7_spill=d7 = lo<r7=u3
lod7 = lou3
# asm 1: hi>r7_spill=spill64#8 = hi<r7=int64#4
# asm 2: hi>r7_spill=d7 = hi<r7=u3
hid7 = hiu3
# qhasm: mainloop:
mainloop:
# qhasm:   in = in_stack
# asm 1: >in=int32#1 = <in_stack=stack32#2
# asm 2: >in=input_0 = <in_stack=o1
input_0 = o1
# qhasm:   w0 = flip mem64[in]; in += 8
# asm 1: hi>w0=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w0=u0 = mem32[<in=input_0]; <in=input_0 += 4
hiu0 = mem32[input_0]; input_0 += 4
# asm 1: lo>w0=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w0=u0 = mem32[<in=input_0]; <in=input_0 += 4
lou0 = mem32[input_0]; input_0 += 4
# qhasm:   w1 = flip mem64[in]; in += 8
# asm 1: hi>w1=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w1=u1 = mem32[<in=input_0]; <in=input_0 += 4
hiu1 = mem32[input_0]; input_0 += 4
# asm 1: lo>w1=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w1=u1 = mem32[<in=input_0]; <in=input_0 += 4
lou1 = mem32[input_0]; input_0 += 4
# qhasm:   w2 = flip mem64[in]; in += 8
# asm 1: hi>w2=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w2=u2 = mem32[<in=input_0]; <in=input_0 += 4
hiu2 = mem32[input_0]; input_0 += 4
# asm 1: lo>w2=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w2=u2 = mem32[<in=input_0]; <in=input_0 += 4
lou2 = mem32[input_0]; input_0 += 4
# qhasm:   w3 = flip mem64[in]; in += 8
# asm 1: hi>w3=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w3=u3 = mem32[<in=input_0]; <in=input_0 += 4
hiu3 = mem32[input_0]; input_0 += 4
# asm 1: lo>w3=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w3=u3 = mem32[<in=input_0]; <in=input_0 += 4
lou3 = mem32[input_0]; input_0 += 4
# qhasm:   w0 = reverse flip w0
# asm 1: lo>w0=int64#1 = lo<w0=int64#1[3]lo<w0=int64#1[2]lo<w0=int64#1[1]lo<w0=int64#1[0]
# asm 2: lo>w0=u0 = lo<w0=u0[3]lo<w0=u0[2]lo<w0=u0[1]lo<w0=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>w0=int64#1 = hi<w0=int64#1[3]hi<w0=int64#1[2]hi<w0=int64#1[1]hi<w0=int64#1[0]
# asm 2: hi>w0=u0 = hi<w0=u0[3]hi<w0=u0[2]hi<w0=u0[1]hi<w0=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm:   w1 = reverse flip w1
# asm 1: lo>w1=int64#2 = lo<w1=int64#2[3]lo<w1=int64#2[2]lo<w1=int64#2[1]lo<w1=int64#2[0]
# asm 2: lo>w1=u1 = lo<w1=u1[3]lo<w1=u1[2]lo<w1=u1[1]lo<w1=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>w1=int64#2 = hi<w1=int64#2[3]hi<w1=int64#2[2]hi<w1=int64#2[1]hi<w1=int64#2[0]
# asm 2: hi>w1=u1 = hi<w1=u1[3]hi<w1=u1[2]hi<w1=u1[1]hi<w1=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm:   w2 = reverse flip w2
# asm 1: lo>w2=int64#3 = lo<w2=int64#3[3]lo<w2=int64#3[2]lo<w2=int64#3[1]lo<w2=int64#3[0]
# asm 2: lo>w2=u2 = lo<w2=u2[3]lo<w2=u2[2]lo<w2=u2[1]lo<w2=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>w2=int64#3 = hi<w2=int64#3[3]hi<w2=int64#3[2]hi<w2=int64#3[1]hi<w2=int64#3[0]
# asm 2: hi>w2=u2 = hi<w2=u2[3]hi<w2=u2[2]hi<w2=u2[1]hi<w2=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm:   w3 = reverse flip w3
# asm 1: lo>w3=int64#4 = lo<w3=int64#4[3]lo<w3=int64#4[2]lo<w3=int64#4[1]lo<w3=int64#4[0]
# asm 2: lo>w3=u3 = lo<w3=u3[3]lo<w3=u3[2]lo<w3=u3[1]lo<w3=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>w3=int64#4 = hi<w3=int64#4[3]hi<w3=int64#4[2]hi<w3=int64#4[1]hi<w3=int64#4[0]
# asm 2: hi>w3=u3 = hi<w3=u3[3]hi<w3=u3[2]hi<w3=u3[1]hi<w3=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm:   w0_spill = w0
# asm 1: lo>w0_spill=spill64#9 = lo<w0=int64#1
# asm 2: lo>w0_spill=d8 = lo<w0=u0
lod8 = lou0
# asm 1: hi>w0_spill=spill64#9 = hi<w0=int64#1
# asm 2: hi>w0_spill=d8 = hi<w0=u0
hid8 = hiu0
# qhasm:   w1_spill = w1
# asm 1: lo>w1_spill=spill64#10 = lo<w1=int64#2
# asm 2: lo>w1_spill=d9 = lo<w1=u1
lod9 = lou1
# asm 1: hi>w1_spill=spill64#10 = hi<w1=int64#2
# asm 2: hi>w1_spill=d9 = hi<w1=u1
hid9 = hiu1
# qhasm:   w2_spill = w2
# asm 1: lo>w2_spill=spill64#11 = lo<w2=int64#3
# asm 2: lo>w2_spill=d10 = lo<w2=u2
lod10 = lou2
# asm 1: hi>w2_spill=spill64#11 = hi<w2=int64#3
# asm 2: hi>w2_spill=d10 = hi<w2=u2
hid10 = hiu2
# qhasm:   w3_spill = w3
# asm 1: lo>w3_spill=spill64#12 = lo<w3=int64#4
# asm 2: lo>w3_spill=d11 = lo<w3=u3
lod11 = lou3
# asm 1: hi>w3_spill=spill64#12 = hi<w3=int64#4
# asm 2: hi>w3_spill=d11 = hi<w3=u3
hid11 = hiu3
# qhasm:   w4 = flip mem64[in]; in += 8
# asm 1: hi>w4=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w4=u0 = mem32[<in=input_0]; <in=input_0 += 4
hiu0 = mem32[input_0]; input_0 += 4
# asm 1: lo>w4=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w4=u0 = mem32[<in=input_0]; <in=input_0 += 4
lou0 = mem32[input_0]; input_0 += 4
# qhasm:   w5 = flip mem64[in]; in += 8
# asm 1: hi>w5=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w5=u1 = mem32[<in=input_0]; <in=input_0 += 4
hiu1 = mem32[input_0]; input_0 += 4
# asm 1: lo>w5=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w5=u1 = mem32[<in=input_0]; <in=input_0 += 4
lou1 = mem32[input_0]; input_0 += 4
# qhasm:   w6 = flip mem64[in]; in += 8
# asm 1: hi>w6=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w6=u2 = mem32[<in=input_0]; <in=input_0 += 4
hiu2 = mem32[input_0]; input_0 += 4
# asm 1: lo>w6=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w6=u2 = mem32[<in=input_0]; <in=input_0 += 4
lou2 = mem32[input_0]; input_0 += 4
# qhasm:   w7 = flip mem64[in]; in += 8
# asm 1: hi>w7=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w7=u3 = mem32[<in=input_0]; <in=input_0 += 4
hiu3 = mem32[input_0]; input_0 += 4
# asm 1: lo>w7=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w7=u3 = mem32[<in=input_0]; <in=input_0 += 4
lou3 = mem32[input_0]; input_0 += 4
# qhasm:   w4 = reverse flip w4
# asm 1: lo>w4=int64#1 = lo<w4=int64#1[3]lo<w4=int64#1[2]lo<w4=int64#1[1]lo<w4=int64#1[0]
# asm 2: lo>w4=u0 = lo<w4=u0[3]lo<w4=u0[2]lo<w4=u0[1]lo<w4=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>w4=int64#1 = hi<w4=int64#1[3]hi<w4=int64#1[2]hi<w4=int64#1[1]hi<w4=int64#1[0]
# asm 2: hi>w4=u0 = hi<w4=u0[3]hi<w4=u0[2]hi<w4=u0[1]hi<w4=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm:   w5 = reverse flip w5
# asm 1: lo>w5=int64#2 = lo<w5=int64#2[3]lo<w5=int64#2[2]lo<w5=int64#2[1]lo<w5=int64#2[0]
# asm 2: lo>w5=u1 = lo<w5=u1[3]lo<w5=u1[2]lo<w5=u1[1]lo<w5=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>w5=int64#2 = hi<w5=int64#2[3]hi<w5=int64#2[2]hi<w5=int64#2[1]hi<w5=int64#2[0]
# asm 2: hi>w5=u1 = hi<w5=u1[3]hi<w5=u1[2]hi<w5=u1[1]hi<w5=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm:   w6 = reverse flip w6
# asm 1: lo>w6=int64#3 = lo<w6=int64#3[3]lo<w6=int64#3[2]lo<w6=int64#3[1]lo<w6=int64#3[0]
# asm 2: lo>w6=u2 = lo<w6=u2[3]lo<w6=u2[2]lo<w6=u2[1]lo<w6=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>w6=int64#3 = hi<w6=int64#3[3]hi<w6=int64#3[2]hi<w6=int64#3[1]hi<w6=int64#3[0]
# asm 2: hi>w6=u2 = hi<w6=u2[3]hi<w6=u2[2]hi<w6=u2[1]hi<w6=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm:   w7 = reverse flip w7
# asm 1: lo>w7=int64#4 = lo<w7=int64#4[3]lo<w7=int64#4[2]lo<w7=int64#4[1]lo<w7=int64#4[0]
# asm 2: lo>w7=u3 = lo<w7=u3[3]lo<w7=u3[2]lo<w7=u3[1]lo<w7=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>w7=int64#4 = hi<w7=int64#4[3]hi<w7=int64#4[2]hi<w7=int64#4[1]hi<w7=int64#4[0]
# asm 2: hi>w7=u3 = hi<w7=u3[3]hi<w7=u3[2]hi<w7=u3[1]hi<w7=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm:   w4_spill = w4
# asm 1: lo>w4_spill=spill64#13 = lo<w4=int64#1
# asm 2: lo>w4_spill=d12 = lo<w4=u0
lod12 = lou0
# asm 1: hi>w4_spill=spill64#13 = hi<w4=int64#1
# asm 2: hi>w4_spill=d12 = hi<w4=u0
hid12 = hiu0
# qhasm:   w5_spill = w5
# asm 1: lo>w5_spill=spill64#14 = lo<w5=int64#2
# asm 2: lo>w5_spill=d13 = lo<w5=u1
lod13 = lou1
# asm 1: hi>w5_spill=spill64#14 = hi<w5=int64#2
# asm 2: hi>w5_spill=d13 = hi<w5=u1
hid13 = hiu1
# qhasm:   w6_spill = w6
# asm 1: lo>w6_spill=spill64#15 = lo<w6=int64#3
# asm 2: lo>w6_spill=d14 = lo<w6=u2
lod14 = lou2
# asm 1: hi>w6_spill=spill64#15 = hi<w6=int64#3
# asm 2: hi>w6_spill=d14 = hi<w6=u2
hid14 = hiu2
# qhasm:   w7_spill = w7
# asm 1: lo>w7_spill=spill64#16 = lo<w7=int64#4
# asm 2: lo>w7_spill=d15 = lo<w7=u3
lod15 = lou3
# asm 1: hi>w7_spill=spill64#16 = hi<w7=int64#4
# asm 2: hi>w7_spill=d15 = hi<w7=u3
hid15 = hiu3
# qhasm:   w8 = flip mem64[in]; in += 8
# asm 1: hi>w8=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w8=u0 = mem32[<in=input_0]; <in=input_0 += 4
hiu0 = mem32[input_0]; input_0 += 4
# asm 1: lo>w8=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w8=u0 = mem32[<in=input_0]; <in=input_0 += 4
lou0 = mem32[input_0]; input_0 += 4
# qhasm:   w9 = flip mem64[in]; in += 8
# asm 1: hi>w9=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w9=u1 = mem32[<in=input_0]; <in=input_0 += 4
hiu1 = mem32[input_0]; input_0 += 4
# asm 1: lo>w9=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w9=u1 = mem32[<in=input_0]; <in=input_0 += 4
lou1 = mem32[input_0]; input_0 += 4
# qhasm:   w10 = flip mem64[in]; in += 8
# asm 1: hi>w10=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w10=u2 = mem32[<in=input_0]; <in=input_0 += 4
hiu2 = mem32[input_0]; input_0 += 4
# asm 1: lo>w10=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w10=u2 = mem32[<in=input_0]; <in=input_0 += 4
lou2 = mem32[input_0]; input_0 += 4
# qhasm:   w11 = flip mem64[in]; in += 8
# asm 1: hi>w11=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w11=u3 = mem32[<in=input_0]; <in=input_0 += 4
hiu3 = mem32[input_0]; input_0 += 4
# asm 1: lo>w11=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w11=u3 = mem32[<in=input_0]; <in=input_0 += 4
lou3 = mem32[input_0]; input_0 += 4
# qhasm:   w8 = reverse flip w8
# asm 1: lo>w8=int64#1 = lo<w8=int64#1[3]lo<w8=int64#1[2]lo<w8=int64#1[1]lo<w8=int64#1[0]
# asm 2: lo>w8=u0 = lo<w8=u0[3]lo<w8=u0[2]lo<w8=u0[1]lo<w8=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>w8=int64#1 = hi<w8=int64#1[3]hi<w8=int64#1[2]hi<w8=int64#1[1]hi<w8=int64#1[0]
# asm 2: hi>w8=u0 = hi<w8=u0[3]hi<w8=u0[2]hi<w8=u0[1]hi<w8=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm:   w9 = reverse flip w9
# asm 1: lo>w9=int64#2 = lo<w9=int64#2[3]lo<w9=int64#2[2]lo<w9=int64#2[1]lo<w9=int64#2[0]
# asm 2: lo>w9=u1 = lo<w9=u1[3]lo<w9=u1[2]lo<w9=u1[1]lo<w9=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>w9=int64#2 = hi<w9=int64#2[3]hi<w9=int64#2[2]hi<w9=int64#2[1]hi<w9=int64#2[0]
# asm 2: hi>w9=u1 = hi<w9=u1[3]hi<w9=u1[2]hi<w9=u1[1]hi<w9=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm:   w10 = reverse flip w10
# asm 1: lo>w10=int64#3 = lo<w10=int64#3[3]lo<w10=int64#3[2]lo<w10=int64#3[1]lo<w10=int64#3[0]
# asm 2: lo>w10=u2 = lo<w10=u2[3]lo<w10=u2[2]lo<w10=u2[1]lo<w10=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>w10=int64#3 = hi<w10=int64#3[3]hi<w10=int64#3[2]hi<w10=int64#3[1]hi<w10=int64#3[0]
# asm 2: hi>w10=u2 = hi<w10=u2[3]hi<w10=u2[2]hi<w10=u2[1]hi<w10=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm:   w11 = reverse flip w11
# asm 1: lo>w11=int64#4 = lo<w11=int64#4[3]lo<w11=int64#4[2]lo<w11=int64#4[1]lo<w11=int64#4[0]
# asm 2: lo>w11=u3 = lo<w11=u3[3]lo<w11=u3[2]lo<w11=u3[1]lo<w11=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>w11=int64#4 = hi<w11=int64#4[3]hi<w11=int64#4[2]hi<w11=int64#4[1]hi<w11=int64#4[0]
# asm 2: hi>w11=u3 = hi<w11=u3[3]hi<w11=u3[2]hi<w11=u3[1]hi<w11=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm:   w0_next = w8
# asm 1: lo>w0_next=stack64#9 = lo<w8=int64#1
# asm 2: lo>w0_next=m8 = lo<w8=u0
lom8 = lou0
# asm 1: hi>w0_next=stack64#9 = hi<w8=int64#1
# asm 2: hi>w0_next=m8 = hi<w8=u0
him8 = hiu0
# qhasm:   w1_next = w9
# asm 1: lo>w1_next=stack64#10 = lo<w9=int64#2
# asm 2: lo>w1_next=m9 = lo<w9=u1
lom9 = lou1
# asm 1: hi>w1_next=stack64#10 = hi<w9=int64#2
# asm 2: hi>w1_next=m9 = hi<w9=u1
him9 = hiu1
# qhasm:   w2_next = w10
# asm 1: lo>w2_next=stack64#11 = lo<w10=int64#3
# asm 2: lo>w2_next=m10 = lo<w10=u2
lom10 = lou2
# asm 1: hi>w2_next=stack64#11 = hi<w10=int64#3
# asm 2: hi>w2_next=m10 = hi<w10=u2
him10 = hiu2
# qhasm:   w3_next = w11
# asm 1: lo>w3_next=stack64#12 = lo<w11=int64#4
# asm 2: lo>w3_next=m11 = lo<w11=u3
lom11 = lou3
# asm 1: hi>w3_next=stack64#12 = hi<w11=int64#4
# asm 2: hi>w3_next=m11 = hi<w11=u3
him11 = hiu3
# qhasm:   w12 = flip mem64[in]; in += 8
# asm 1: hi>w12=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w12=u0 = mem32[<in=input_0]; <in=input_0 += 4
hiu0 = mem32[input_0]; input_0 += 4
# asm 1: lo>w12=int64#1 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w12=u0 = mem32[<in=input_0]; <in=input_0 += 4
lou0 = mem32[input_0]; input_0 += 4
# qhasm:   w13 = flip mem64[in]; in += 8
# asm 1: hi>w13=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w13=u1 = mem32[<in=input_0]; <in=input_0 += 4
hiu1 = mem32[input_0]; input_0 += 4
# asm 1: lo>w13=int64#2 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w13=u1 = mem32[<in=input_0]; <in=input_0 += 4
lou1 = mem32[input_0]; input_0 += 4
# qhasm:   w14 = flip mem64[in]; in += 8
# asm 1: hi>w14=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w14=u2 = mem32[<in=input_0]; <in=input_0 += 4
hiu2 = mem32[input_0]; input_0 += 4
# asm 1: lo>w14=int64#3 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w14=u2 = mem32[<in=input_0]; <in=input_0 += 4
lou2 = mem32[input_0]; input_0 += 4
# qhasm:   w15 = flip mem64[in]; in += 8
# asm 1: hi>w15=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: hi>w15=u3 = mem32[<in=input_0]; <in=input_0 += 4
hiu3 = mem32[input_0]; input_0 += 4
# asm 1: lo>w15=int64#4 = mem32[<in=int32#1]; <in=int32#1 += 4
# asm 2: lo>w15=u3 = mem32[<in=input_0]; <in=input_0 += 4
lou3 = mem32[input_0]; input_0 += 4
# qhasm:   w12 = reverse flip w12
# asm 1: lo>w12=int64#1 = lo<w12=int64#1[3]lo<w12=int64#1[2]lo<w12=int64#1[1]lo<w12=int64#1[0]
# asm 2: lo>w12=u0 = lo<w12=u0[3]lo<w12=u0[2]lo<w12=u0[1]lo<w12=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>w12=int64#1 = hi<w12=int64#1[3]hi<w12=int64#1[2]hi<w12=int64#1[1]hi<w12=int64#1[0]
# asm 2: hi>w12=u0 = hi<w12=u0[3]hi<w12=u0[2]hi<w12=u0[1]hi<w12=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm:   w13 = reverse flip w13
# asm 1: lo>w13=int64#2 = lo<w13=int64#2[3]lo<w13=int64#2[2]lo<w13=int64#2[1]lo<w13=int64#2[0]
# asm 2: lo>w13=u1 = lo<w13=u1[3]lo<w13=u1[2]lo<w13=u1[1]lo<w13=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>w13=int64#2 = hi<w13=int64#2[3]hi<w13=int64#2[2]hi<w13=int64#2[1]hi<w13=int64#2[0]
# asm 2: hi>w13=u1 = hi<w13=u1[3]hi<w13=u1[2]hi<w13=u1[1]hi<w13=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm:   w14 = reverse flip w14
# asm 1: lo>w14=int64#3 = lo<w14=int64#3[3]lo<w14=int64#3[2]lo<w14=int64#3[1]lo<w14=int64#3[0]
# asm 2: lo>w14=u2 = lo<w14=u2[3]lo<w14=u2[2]lo<w14=u2[1]lo<w14=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>w14=int64#3 = hi<w14=int64#3[3]hi<w14=int64#3[2]hi<w14=int64#3[1]hi<w14=int64#3[0]
# asm 2: hi>w14=u2 = hi<w14=u2[3]hi<w14=u2[2]hi<w14=u2[1]hi<w14=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm:   w15 = reverse flip w15
# asm 1: lo>w15=int64#4 = lo<w15=int64#4[3]lo<w15=int64#4[2]lo<w15=int64#4[1]lo<w15=int64#4[0]
# asm 2: lo>w15=u3 = lo<w15=u3[3]lo<w15=u3[2]lo<w15=u3[1]lo<w15=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>w15=int64#4 = hi<w15=int64#4[3]hi<w15=int64#4[2]hi<w15=int64#4[1]hi<w15=int64#4[0]
# asm 2: hi>w15=u3 = hi<w15=u3[3]hi<w15=u3[2]hi<w15=u3[1]hi<w15=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm:   w4_next = w12
# asm 1: lo>w4_next=stack64#13 = lo<w12=int64#1
# asm 2: lo>w4_next=m12 = lo<w12=u0
lom12 = lou0
# asm 1: hi>w4_next=stack64#13 = hi<w12=int64#1
# asm 2: hi>w4_next=m12 = hi<w12=u0
him12 = hiu0
# qhasm:   w5_next = w13
# asm 1: lo>w5_next=stack64#14 = lo<w13=int64#2
# asm 2: lo>w5_next=m13 = lo<w13=u1
lom13 = lou1
# asm 1: hi>w5_next=stack64#14 = hi<w13=int64#2
# asm 2: hi>w5_next=m13 = hi<w13=u1
him13 = hiu1
# qhasm:   w6_next = w14
# asm 1: lo>w6_next=stack64#15 = lo<w14=int64#3
# asm 2: lo>w6_next=m14 = lo<w14=u2
lom14 = lou2
# asm 1: hi>w6_next=stack64#15 = hi<w14=int64#3
# asm 2: hi>w6_next=m14 = hi<w14=u2
him14 = hiu2
# qhasm:   w7_next = w15
# asm 1: lo>w7_next=stack64#16 = lo<w15=int64#4
# asm 2: lo>w7_next=m15 = lo<w15=u3
lom15 = lou3
# asm 1: hi>w7_next=stack64#16 = hi<w15=int64#4
# asm 2: hi>w7_next=m15 = hi<w15=u3
him15 = hiu3
# qhasm:   in_stack = in
# asm 1: >in_stack=stack32#2 = <in=int32#1
# asm 2: >in_stack=o1 = <in=input_0
o1 = input_0
# qhasm:   i = 80 simple
# asm 1: >i=int32#1 = 80 simple
# asm 2: >i=input_0 = 80 simple
input_0 = 80 simple
# qhasm:   i_stack = i
# asm 1: >i_stack=stack32#5 = <i=int32#1
# asm 2: >i_stack=o4 = <i=input_0
o4 = input_0
# qhasm:   innerloop:
innerloop:
# qhasm:     assign 0 to r0_spill
# asm 1: assign 0 to lo<r0_spill=spill64#1
# asm 2: assign 0 to lo<r0_spill=d0
assign 0 to lod0
# asm 1: assign 1 to hi<r0_spill=spill64#1
# asm 2: assign 1 to hi<r0_spill=d0
assign 1 to hid0
# qhasm:     assign 1 to r1_spill
# asm 1: assign 2 to lo<r1_spill=spill64#2
# asm 2: assign 2 to lo<r1_spill=d1
assign 2 to lod1
# asm 1: assign 3 to hi<r1_spill=spill64#2
# asm 2: assign 3 to hi<r1_spill=d1
assign 3 to hid1
# qhasm:     assign 2 to r2_spill
# asm 1: assign 4 to lo<r2_spill=spill64#3
# asm 2: assign 4 to lo<r2_spill=d2
assign 4 to lod2
# asm 1: assign 5 to hi<r2_spill=spill64#3
# asm 2: assign 5 to hi<r2_spill=d2
assign 5 to hid2
# qhasm:     assign 3 to r3_spill
# asm 1: assign 6 to lo<r3_spill=spill64#4
# asm 2: assign 6 to lo<r3_spill=d3
assign 6 to lod3
# asm 1: assign 7 to hi<r3_spill=spill64#4
# asm 2: assign 7 to hi<r3_spill=d3
assign 7 to hid3
# qhasm:     assign 4 to r4_spill
# asm 1: assign 8 to lo<r4_spill=spill64#5
# asm 2: assign 8 to lo<r4_spill=d4
assign 8 to lod4
# asm 1: assign 9 to hi<r4_spill=spill64#5
# asm 2: assign 9 to hi<r4_spill=d4
assign 9 to hid4
# qhasm:     assign 5 to r5_spill
# asm 1: assign 10 to lo<r5_spill=spill64#6
# asm 2: assign 10 to lo<r5_spill=d5
assign 10 to lod5
# asm 1: assign 11 to hi<r5_spill=spill64#6
# asm 2: assign 11 to hi<r5_spill=d5
assign 11 to hid5
# qhasm:     assign 6 to r6_spill
# asm 1: assign 12 to lo<r6_spill=spill64#7
# asm 2: assign 12 to lo<r6_spill=d6
assign 12 to lod6
# asm 1: assign 13 to hi<r6_spill=spill64#7
# asm 2: assign 13 to hi<r6_spill=d6
assign 13 to hid6
# qhasm:     assign 7 to r7_spill
# asm 1: assign 14 to lo<r7_spill=spill64#8
# asm 2: assign 14 to lo<r7_spill=d7
assign 14 to lod7
# asm 1: assign 15 to hi<r7_spill=spill64#8
# asm 2: assign 15 to hi<r7_spill=d7
assign 15 to hid7
# qhasm:     constants = constants_stack
# asm 1: >constants=int32#1 = <constants_stack=stack32#4
# asm 2: >constants=input_0 = <constants_stack=o3
input_0 = o3
# qhasm:       r3 = r3_spill
# asm 1: lo>r3=int64#1 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u0 = lo<r3_spill=d3
lou0 = lod3
# asm 1: hi>r3=int64#1 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u0 = hi<r3_spill=d3
hiu0 = hid3
# qhasm:       r4 = r4_spill
# asm 1: lo>r4=int64#2 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u1 = lo<r4_spill=d4
lou1 = lod4
# asm 1: hi>r4=int64#2 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u1 = hi<r4_spill=d4
hiu1 = hid4
# qhasm:       r5 = r5_spill
# asm 1: lo>r5=int64#3 = lo<r5_spill=spill64#6
# asm 2: lo>r5=u2 = lo<r5_spill=d5
lou2 = lod5
# asm 1: hi>r5=int64#3 = hi<r5_spill=spill64#6
# asm 2: hi>r5=u2 = hi<r5_spill=d5
hiu2 = hid5
# qhasm:       r6 = r6_spill
# asm 1: lo>r6=int64#4 = lo<r6_spill=spill64#7
# asm 2: lo>r6=u3 = lo<r6_spill=d6
lou3 = lod6
# asm 1: hi>r6=int64#4 = hi<r6_spill=spill64#7
# asm 2: hi>r6=u3 = hi<r6_spill=d6
hiu3 = hid6
# qhasm:       r7 = r7_spill
# asm 1: lo>r7=int64#5 = lo<r7_spill=spill64#8
# asm 2: lo>r7=u4 = lo<r7_spill=d7
lou4 = lod7
# asm 1: hi>r7=int64#5 = hi<r7_spill=spill64#8
# asm 2: hi>r7=u4 = hi<r7_spill=d7
hiu4 = hid7
# qhasm:       w0 = w0_spill
# asm 1: lo>w0=int64#6 = lo<w0_spill=spill64#9
# asm 2: lo>w0=u5 = lo<w0_spill=d8
lou5 = lod8
# asm 1: hi>w0=int64#6 = hi<w0_spill=spill64#9
# asm 2: hi>w0=u5 = hi<w0_spill=d8
hiu5 = hid8
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r7 += w0 + mem64[constants] + Sigma1(r4) + Ch(r4,r5,r6); constants += 8
# asm 1: carry?  lo<r7=int64#5 += lo<w0=int64#6
# asm 2: carry?  lo<r7=u4 += lo<w0=u5
carry?  lou4 += lou5
# asm 1: hi<r7=int64#5 += hi<w0=int64#6 + carry
# asm 2: hi<r7=u4 += hi<w0=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r7=int64#5 += lotmp
# asm 2: carry? lo<r7=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r7=int64#5 += hitmp + carry
# asm 2: hi<r7=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r4=int64#2 * two23
# asm 2: hitmp lotmp = lo<r4=u1 * two23
hitmp lotmp = lou1 * two23
# asm 1: lotmp hitmp += hi<r4=int64#2 * two23
# asm 2: lotmp hitmp += hi<r4=u1 * two23
lotmp hitmp += hiu1 * two23
# asm 1: lotmp ^= (lo<r4=int64#2 unsigned>> 18)
# asm 2: lotmp ^= (lo<r4=u1 unsigned>> 18)
lotmp ^= (lou1 unsigned>> 18)
# asm 1: lotmp ^= (hi<r4=int64#2 << 14)
# asm 2: lotmp ^= (hi<r4=u1 << 14)
lotmp ^= (hiu1 << 14)
# asm 1: lotmp ^= (lo<r4=int64#2 unsigned>> 14)
# asm 2: lotmp ^= (lo<r4=u1 unsigned>> 14)
lotmp ^= (lou1 unsigned>> 14)
# asm 1: lotmp ^= (hi<r4=int64#2 << 18)
# asm 2: lotmp ^= (hi<r4=u1 << 18)
lotmp ^= (hiu1 << 18)
# asm 1: hitmp ^= (hi<r4=int64#2 unsigned>> 18)
# asm 2: hitmp ^= (hi<r4=u1 unsigned>> 18)
hitmp ^= (hiu1 unsigned>> 18)
# asm 1: hitmp ^= (lo<r4=int64#2 << 14)
# asm 2: hitmp ^= (lo<r4=u1 << 14)
hitmp ^= (lou1 << 14)
# asm 1: hitmp ^= (hi<r4=int64#2 unsigned>> 14)
# asm 2: hitmp ^= (hi<r4=u1 unsigned>> 14)
hitmp ^= (hiu1 unsigned>> 14)
# asm 1: hitmp ^= (lo<r4=int64#2 << 18)
# asm 2: hitmp ^= (lo<r4=u1 << 18)
hitmp ^= (lou1 << 18)
# asm 1: carry? lo<r7=int64#5 += lotmp
# asm 2: carry? lo<r7=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r7=int64#5 += hitmp + carry
# asm 2: hi<r7=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r4=int64#2 & lo<r5=int64#3
# asm 2: lotmp = lo<r4=u1 & lo<r5=u2
lotmp = lou1 & lou2
# asm 1: lotmp2 = lo<r6=int64#4 & ~lo<r4=int64#2
# asm 2: lotmp2 = lo<r6=u3 & ~lo<r4=u1
lotmp2 = lou3 & ~lou1
lotmp ^= lotmp2
# asm 1: carry? lo<r7=int64#5 += lotmp
# asm 2: carry? lo<r7=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r4=int64#2 & hi<r5=int64#3
# asm 2: hitmp = hi<r4=u1 & hi<r5=u2
hitmp = hiu1 & hiu2
# asm 1: hitmp2 = hi<r6=int64#4 & ~hi<r4=int64#2
# asm 2: hitmp2 = hi<r6=u3 & ~hi<r4=u1
hitmp2 = hiu3 & ~hiu1
hitmp ^= hitmp2
# asm 1: hi<r7=int64#5 += hitmp + carry
# asm 2: hi<r7=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r3 += r7
# asm 1: carry? lo<r3=int64#1 += lo<r7=int64#5
# asm 2: carry? lo<r3=u0 += lo<r7=u4
carry? lou0 += lou4
# asm 1: hi<r3=int64#1 += hi<r7=int64#5 + carry
# asm 2: hi<r3=u0 += hi<r7=u4 + carry
hiu0 += hiu4 + carry
# qhasm:       r3_spill = r3
# asm 1: lo>r3_spill=spill64#4 = lo<r3=int64#1
# asm 2: lo>r3_spill=d3 = lo<r3=u0
lod3 = lou0
# asm 1: hi>r3_spill=spill64#4 = hi<r3=int64#1
# asm 2: hi>r3_spill=d3 = hi<r3=u0
hid3 = hiu0
# qhasm:       r0 = r0_spill
# asm 1: lo>r0=int64#2 = lo<r0_spill=spill64#1
# asm 2: lo>r0=u1 = lo<r0_spill=d0
lou1 = lod0
# asm 1: hi>r0=int64#2 = hi<r0_spill=spill64#1
# asm 2: hi>r0=u1 = hi<r0_spill=d0
hiu1 = hid0
# qhasm:       r1 = r1_spill
# asm 1: lo>r1=int64#3 = lo<r1_spill=spill64#2
# asm 2: lo>r1=u2 = lo<r1_spill=d1
lou2 = lod1
# asm 1: hi>r1=int64#3 = hi<r1_spill=spill64#2
# asm 2: hi>r1=u2 = hi<r1_spill=d1
hiu2 = hid1
# qhasm:       r2 = r2_spill
# asm 1: lo>r2=int64#4 = lo<r2_spill=spill64#3
# asm 2: lo>r2=u3 = lo<r2_spill=d2
lou3 = lod2
# asm 1: hi>r2=int64#4 = hi<r2_spill=spill64#3
# asm 2: hi>r2=u3 = hi<r2_spill=d2
hiu3 = hid2
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r7 += Sigma0(r0) + Maj(r0,r1,r2)
# asm 1: hitmp lotmp = lo<r0=int64#2 * two25
# asm 2: hitmp lotmp = lo<r0=u1 * two25
hitmp lotmp = lou1 * two25
# asm 1: lotmp hitmp += hi<r0=int64#2 * two25
# asm 2: lotmp hitmp += hi<r0=u1 * two25
lotmp hitmp += hiu1 * two25
# asm 1: lotmp ^= (hi<r0=int64#2 unsigned>> 2)
# asm 2: lotmp ^= (hi<r0=u1 unsigned>> 2)
lotmp ^= (hiu1 unsigned>> 2)
# asm 1: lotmp ^= (lo<r0=int64#2 << 30)
# asm 2: lotmp ^= (lo<r0=u1 << 30)
lotmp ^= (lou1 << 30)
# asm 1: lotmp ^= (lo<r0=int64#2 unsigned>> 28)
# asm 2: lotmp ^= (lo<r0=u1 unsigned>> 28)
lotmp ^= (lou1 unsigned>> 28)
# asm 1: lotmp ^= (hi<r0=int64#2 << 4)
# asm 2: lotmp ^= (hi<r0=u1 << 4)
lotmp ^= (hiu1 << 4)
# asm 1: hitmp ^= (lo<r0=int64#2 unsigned>> 2)
# asm 2: hitmp ^= (lo<r0=u1 unsigned>> 2)
hitmp ^= (lou1 unsigned>> 2)
# asm 1: hitmp ^= (hi<r0=int64#2 << 30)
# asm 2: hitmp ^= (hi<r0=u1 << 30)
hitmp ^= (hiu1 << 30)
# asm 1: hitmp ^= (hi<r0=int64#2 unsigned>> 28)
# asm 2: hitmp ^= (hi<r0=u1 unsigned>> 28)
hitmp ^= (hiu1 unsigned>> 28)
# asm 1: hitmp ^= (lo<r0=int64#2 << 4)
# asm 2: hitmp ^= (lo<r0=u1 << 4)
hitmp ^= (lou1 << 4)
# asm 1: carry? lo<r7=int64#5 += lotmp
# asm 2: carry? lo<r7=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r7=int64#5 += hitmp + carry
# asm 2: hi<r7=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r1=int64#3 ^ lo<r2=int64#4
# asm 2: lotmp = lo<r1=u2 ^ lo<r2=u3
lotmp = lou2 ^ lou3
# asm 1: lotmp &= lo<r0=int64#2
# asm 2: lotmp &= lo<r0=u1
lotmp &= lou1
# asm 1: lotmp2 = lo<r1=int64#3 & lo<r2=int64#4
# asm 2: lotmp2 = lo<r1=u2 & lo<r2=u3
lotmp2 = lou2 & lou3
lotmp ^= lotmp2
# asm 1: carry? lo<r7=int64#5 += lotmp
# asm 2: carry? lo<r7=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r1=int64#3 ^ hi<r2=int64#4
# asm 2: hitmp = hi<r1=u2 ^ hi<r2=u3
hitmp = hiu2 ^ hiu3
# asm 1: hitmp &= hi<r0=int64#2
# asm 2: hitmp &= hi<r0=u1
hitmp &= hiu1
# asm 1: hitmp2 = hi<r1=int64#3 & hi<r2=int64#4
# asm 2: hitmp2 = hi<r1=u2 & hi<r2=u3
hitmp2 = hiu2 & hiu3
hitmp ^= hitmp2
# asm 1: hi<r7=int64#5 += hitmp + carry
# asm 2: hi<r7=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r7_spill = r7
# asm 1: lo>r7_spill=spill64#8 = lo<r7=int64#5
# asm 2: lo>r7_spill=d7 = lo<r7=u4
lod7 = lou4
# asm 1: hi>r7_spill=spill64#8 = hi<r7=int64#5
# asm 2: hi>r7_spill=d7 = hi<r7=u4
hid7 = hiu4
# qhasm:       r4 = r4_spill
# asm 1: lo>r4=int64#2 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u1 = lo<r4_spill=d4
lou1 = lod4
# asm 1: hi>r4=int64#2 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u1 = hi<r4_spill=d4
hiu1 = hid4
# qhasm:       r5 = r5_spill
# asm 1: lo>r5=int64#3 = lo<r5_spill=spill64#6
# asm 2: lo>r5=u2 = lo<r5_spill=d5
lou2 = lod5
# asm 1: hi>r5=int64#3 = hi<r5_spill=spill64#6
# asm 2: hi>r5=u2 = hi<r5_spill=d5
hiu2 = hid5
# qhasm:       r6 = r6_spill
# asm 1: lo>r6=int64#5 = lo<r6_spill=spill64#7
# asm 2: lo>r6=u4 = lo<r6_spill=d6
lou4 = lod6
# asm 1: hi>r6=int64#5 = hi<r6_spill=spill64#7
# asm 2: hi>r6=u4 = hi<r6_spill=d6
hiu4 = hid6
# qhasm:       w1 = w1_spill
# asm 1: lo>w1=int64#6 = lo<w1_spill=spill64#10
# asm 2: lo>w1=u5 = lo<w1_spill=d9
lou5 = lod9
# asm 1: hi>w1=int64#6 = hi<w1_spill=spill64#10
# asm 2: hi>w1=u5 = hi<w1_spill=d9
hiu5 = hid9
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r6 += w1 + mem64[constants] + Sigma1(r3) + Ch(r3,r4,r5); constants += 8
# asm 1: carry?  lo<r6=int64#5 += lo<w1=int64#6
# asm 2: carry?  lo<r6=u4 += lo<w1=u5
carry?  lou4 += lou5
# asm 1: hi<r6=int64#5 += hi<w1=int64#6 + carry
# asm 2: hi<r6=u4 += hi<w1=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r6=int64#5 += lotmp
# asm 2: carry? lo<r6=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r6=int64#5 += hitmp + carry
# asm 2: hi<r6=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r3=int64#1 * two23
# asm 2: hitmp lotmp = lo<r3=u0 * two23
hitmp lotmp = lou0 * two23
# asm 1: lotmp hitmp += hi<r3=int64#1 * two23
# asm 2: lotmp hitmp += hi<r3=u0 * two23
lotmp hitmp += hiu0 * two23
# asm 1: lotmp ^= (lo<r3=int64#1 unsigned>> 18)
# asm 2: lotmp ^= (lo<r3=u0 unsigned>> 18)
lotmp ^= (lou0 unsigned>> 18)
# asm 1: lotmp ^= (hi<r3=int64#1 << 14)
# asm 2: lotmp ^= (hi<r3=u0 << 14)
lotmp ^= (hiu0 << 14)
# asm 1: lotmp ^= (lo<r3=int64#1 unsigned>> 14)
# asm 2: lotmp ^= (lo<r3=u0 unsigned>> 14)
lotmp ^= (lou0 unsigned>> 14)
# asm 1: lotmp ^= (hi<r3=int64#1 << 18)
# asm 2: lotmp ^= (hi<r3=u0 << 18)
lotmp ^= (hiu0 << 18)
# asm 1: hitmp ^= (hi<r3=int64#1 unsigned>> 18)
# asm 2: hitmp ^= (hi<r3=u0 unsigned>> 18)
hitmp ^= (hiu0 unsigned>> 18)
# asm 1: hitmp ^= (lo<r3=int64#1 << 14)
# asm 2: hitmp ^= (lo<r3=u0 << 14)
hitmp ^= (lou0 << 14)
# asm 1: hitmp ^= (hi<r3=int64#1 unsigned>> 14)
# asm 2: hitmp ^= (hi<r3=u0 unsigned>> 14)
hitmp ^= (hiu0 unsigned>> 14)
# asm 1: hitmp ^= (lo<r3=int64#1 << 18)
# asm 2: hitmp ^= (lo<r3=u0 << 18)
hitmp ^= (lou0 << 18)
# asm 1: carry? lo<r6=int64#5 += lotmp
# asm 2: carry? lo<r6=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r6=int64#5 += hitmp + carry
# asm 2: hi<r6=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r3=int64#1 & lo<r4=int64#2
# asm 2: lotmp = lo<r3=u0 & lo<r4=u1
lotmp = lou0 & lou1
# asm 1: lotmp2 = lo<r5=int64#3 & ~lo<r3=int64#1
# asm 2: lotmp2 = lo<r5=u2 & ~lo<r3=u0
lotmp2 = lou2 & ~lou0
lotmp ^= lotmp2
# asm 1: carry? lo<r6=int64#5 += lotmp
# asm 2: carry? lo<r6=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r3=int64#1 & hi<r4=int64#2
# asm 2: hitmp = hi<r3=u0 & hi<r4=u1
hitmp = hiu0 & hiu1
# asm 1: hitmp2 = hi<r5=int64#3 & ~hi<r3=int64#1
# asm 2: hitmp2 = hi<r5=u2 & ~hi<r3=u0
hitmp2 = hiu2 & ~hiu0
hitmp ^= hitmp2
# asm 1: hi<r6=int64#5 += hitmp + carry
# asm 2: hi<r6=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r2 += r6
# asm 1: carry? lo<r2=int64#4 += lo<r6=int64#5
# asm 2: carry? lo<r2=u3 += lo<r6=u4
carry? lou3 += lou4
# asm 1: hi<r2=int64#4 += hi<r6=int64#5 + carry
# asm 2: hi<r2=u3 += hi<r6=u4 + carry
hiu3 += hiu4 + carry
# qhasm:       r2_spill = r2
# asm 1: lo>r2_spill=spill64#3 = lo<r2=int64#4
# asm 2: lo>r2_spill=d2 = lo<r2=u3
lod2 = lou3
# asm 1: hi>r2_spill=spill64#3 = hi<r2=int64#4
# asm 2: hi>r2_spill=d2 = hi<r2=u3
hid2 = hiu3
# qhasm:       r7 = r7_spill
# asm 1: lo>r7=int64#1 = lo<r7_spill=spill64#8
# asm 2: lo>r7=u0 = lo<r7_spill=d7
lou0 = lod7
# asm 1: hi>r7=int64#1 = hi<r7_spill=spill64#8
# asm 2: hi>r7=u0 = hi<r7_spill=d7
hiu0 = hid7
# qhasm:       r0 = r0_spill
# asm 1: lo>r0=int64#2 = lo<r0_spill=spill64#1
# asm 2: lo>r0=u1 = lo<r0_spill=d0
lou1 = lod0
# asm 1: hi>r0=int64#2 = hi<r0_spill=spill64#1
# asm 2: hi>r0=u1 = hi<r0_spill=d0
hiu1 = hid0
# qhasm:       r1 = r1_spill
# asm 1: lo>r1=int64#3 = lo<r1_spill=spill64#2
# asm 2: lo>r1=u2 = lo<r1_spill=d1
lou2 = lod1
# asm 1: hi>r1=int64#3 = hi<r1_spill=spill64#2
# asm 2: hi>r1=u2 = hi<r1_spill=d1
hiu2 = hid1
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r6 += Sigma0(r7) + Maj(r7,r0,r1)
# asm 1: hitmp lotmp = lo<r7=int64#1 * two25
# asm 2: hitmp lotmp = lo<r7=u0 * two25
hitmp lotmp = lou0 * two25
# asm 1: lotmp hitmp += hi<r7=int64#1 * two25
# asm 2: lotmp hitmp += hi<r7=u0 * two25
lotmp hitmp += hiu0 * two25
# asm 1: lotmp ^= (hi<r7=int64#1 unsigned>> 2)
# asm 2: lotmp ^= (hi<r7=u0 unsigned>> 2)
lotmp ^= (hiu0 unsigned>> 2)
# asm 1: lotmp ^= (lo<r7=int64#1 << 30)
# asm 2: lotmp ^= (lo<r7=u0 << 30)
lotmp ^= (lou0 << 30)
# asm 1: lotmp ^= (lo<r7=int64#1 unsigned>> 28)
# asm 2: lotmp ^= (lo<r7=u0 unsigned>> 28)
lotmp ^= (lou0 unsigned>> 28)
# asm 1: lotmp ^= (hi<r7=int64#1 << 4)
# asm 2: lotmp ^= (hi<r7=u0 << 4)
lotmp ^= (hiu0 << 4)
# asm 1: hitmp ^= (lo<r7=int64#1 unsigned>> 2)
# asm 2: hitmp ^= (lo<r7=u0 unsigned>> 2)
hitmp ^= (lou0 unsigned>> 2)
# asm 1: hitmp ^= (hi<r7=int64#1 << 30)
# asm 2: hitmp ^= (hi<r7=u0 << 30)
hitmp ^= (hiu0 << 30)
# asm 1: hitmp ^= (hi<r7=int64#1 unsigned>> 28)
# asm 2: hitmp ^= (hi<r7=u0 unsigned>> 28)
hitmp ^= (hiu0 unsigned>> 28)
# asm 1: hitmp ^= (lo<r7=int64#1 << 4)
# asm 2: hitmp ^= (lo<r7=u0 << 4)
hitmp ^= (lou0 << 4)
# asm 1: carry? lo<r6=int64#5 += lotmp
# asm 2: carry? lo<r6=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r6=int64#5 += hitmp + carry
# asm 2: hi<r6=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r0=int64#2 ^ lo<r1=int64#3
# asm 2: lotmp = lo<r0=u1 ^ lo<r1=u2
lotmp = lou1 ^ lou2
# asm 1: lotmp &= lo<r7=int64#1
# asm 2: lotmp &= lo<r7=u0
lotmp &= lou0
# asm 1: lotmp2 = lo<r0=int64#2 & lo<r1=int64#3
# asm 2: lotmp2 = lo<r0=u1 & lo<r1=u2
lotmp2 = lou1 & lou2
lotmp ^= lotmp2
# asm 1: carry? lo<r6=int64#5 += lotmp
# asm 2: carry? lo<r6=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r0=int64#2 ^ hi<r1=int64#3
# asm 2: hitmp = hi<r0=u1 ^ hi<r1=u2
hitmp = hiu1 ^ hiu2
# asm 1: hitmp &= hi<r7=int64#1
# asm 2: hitmp &= hi<r7=u0
hitmp &= hiu0
# asm 1: hitmp2 = hi<r0=int64#2 & hi<r1=int64#3
# asm 2: hitmp2 = hi<r0=u1 & hi<r1=u2
hitmp2 = hiu1 & hiu2
hitmp ^= hitmp2
# asm 1: hi<r6=int64#5 += hitmp + carry
# asm 2: hi<r6=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r6_spill = r6
# asm 1: lo>r6_spill=spill64#7 = lo<r6=int64#5
# asm 2: lo>r6_spill=d6 = lo<r6=u4
lod6 = lou4
# asm 1: hi>r6_spill=spill64#7 = hi<r6=int64#5
# asm 2: hi>r6_spill=d6 = hi<r6=u4
hid6 = hiu4
# qhasm:     assign 0 to r0_spill
# asm 1: assign 0 to lo<r0_spill=spill64#1
# asm 2: assign 0 to lo<r0_spill=d0
assign 0 to lod0
# asm 1: assign 1 to hi<r0_spill=spill64#1
# asm 2: assign 1 to hi<r0_spill=d0
assign 1 to hid0
# qhasm:     assign 1 to r1_spill
# asm 1: assign 2 to lo<r1_spill=spill64#2
# asm 2: assign 2 to lo<r1_spill=d1
assign 2 to lod1
# asm 1: assign 3 to hi<r1_spill=spill64#2
# asm 2: assign 3 to hi<r1_spill=d1
assign 3 to hid1
# qhasm:     assign 2 to r2_spill
# asm 1: assign 4 to lo<r2_spill=spill64#3
# asm 2: assign 4 to lo<r2_spill=d2
assign 4 to lod2
# asm 1: assign 5 to hi<r2_spill=spill64#3
# asm 2: assign 5 to hi<r2_spill=d2
assign 5 to hid2
# qhasm:     assign 3 to r3_spill
# asm 1: assign 6 to lo<r3_spill=spill64#4
# asm 2: assign 6 to lo<r3_spill=d3
assign 6 to lod3
# asm 1: assign 7 to hi<r3_spill=spill64#4
# asm 2: assign 7 to hi<r3_spill=d3
assign 7 to hid3
# qhasm:     assign 4 to r4_spill
# asm 1: assign 8 to lo<r4_spill=spill64#5
# asm 2: assign 8 to lo<r4_spill=d4
assign 8 to lod4
# asm 1: assign 9 to hi<r4_spill=spill64#5
# asm 2: assign 9 to hi<r4_spill=d4
assign 9 to hid4
# qhasm:     assign 5 to r5_spill
# asm 1: assign 10 to lo<r5_spill=spill64#6
# asm 2: assign 10 to lo<r5_spill=d5
assign 10 to lod5
# asm 1: assign 11 to hi<r5_spill=spill64#6
# asm 2: assign 11 to hi<r5_spill=d5
assign 11 to hid5
# qhasm:     assign 6 to r6_spill
# asm 1: assign 12 to lo<r6_spill=spill64#7
# asm 2: assign 12 to lo<r6_spill=d6
assign 12 to lod6
# asm 1: assign 13 to hi<r6_spill=spill64#7
# asm 2: assign 13 to hi<r6_spill=d6
assign 13 to hid6
# qhasm:     assign 7 to r7_spill
# asm 1: assign 14 to lo<r7_spill=spill64#8
# asm 2: assign 14 to lo<r7_spill=d7
assign 14 to lod7
# asm 1: assign 15 to hi<r7_spill=spill64#8
# asm 2: assign 15 to hi<r7_spill=d7
assign 15 to hid7
# qhasm:       r3 = r3_spill
# asm 1: lo>r3=int64#1 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u0 = lo<r3_spill=d3
lou0 = lod3
# asm 1: hi>r3=int64#1 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u0 = hi<r3_spill=d3
hiu0 = hid3
# qhasm:       r4 = r4_spill
# asm 1: lo>r4=int64#2 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u1 = lo<r4_spill=d4
lou1 = lod4
# asm 1: hi>r4=int64#2 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u1 = hi<r4_spill=d4
hiu1 = hid4
# qhasm:       r5 = r5_spill
# asm 1: lo>r5=int64#5 = lo<r5_spill=spill64#6
# asm 2: lo>r5=u4 = lo<r5_spill=d5
lou4 = lod5
# asm 1: hi>r5=int64#5 = hi<r5_spill=spill64#6
# asm 2: hi>r5=u4 = hi<r5_spill=d5
hiu4 = hid5
# qhasm:       w2 = w2_spill
# asm 1: lo>w2=int64#6 = lo<w2_spill=spill64#11
# asm 2: lo>w2=u5 = lo<w2_spill=d10
lou5 = lod10
# asm 1: hi>w2=int64#6 = hi<w2_spill=spill64#11
# asm 2: hi>w2=u5 = hi<w2_spill=d10
hiu5 = hid10
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r5 += w2 + mem64[constants] + Sigma1(r2) + Ch(r2,r3,r4); constants += 8
# asm 1: carry?  lo<r5=int64#5 += lo<w2=int64#6
# asm 2: carry?  lo<r5=u4 += lo<w2=u5
carry?  lou4 += lou5
# asm 1: hi<r5=int64#5 += hi<w2=int64#6 + carry
# asm 2: hi<r5=u4 += hi<w2=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r5=int64#5 += lotmp
# asm 2: carry? lo<r5=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r5=int64#5 += hitmp + carry
# asm 2: hi<r5=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r2=int64#4 * two23
# asm 2: hitmp lotmp = lo<r2=u3 * two23
hitmp lotmp = lou3 * two23
# asm 1: lotmp hitmp += hi<r2=int64#4 * two23
# asm 2: lotmp hitmp += hi<r2=u3 * two23
lotmp hitmp += hiu3 * two23
# asm 1: lotmp ^= (lo<r2=int64#4 unsigned>> 18)
# asm 2: lotmp ^= (lo<r2=u3 unsigned>> 18)
lotmp ^= (lou3 unsigned>> 18)
# asm 1: lotmp ^= (hi<r2=int64#4 << 14)
# asm 2: lotmp ^= (hi<r2=u3 << 14)
lotmp ^= (hiu3 << 14)
# asm 1: lotmp ^= (lo<r2=int64#4 unsigned>> 14)
# asm 2: lotmp ^= (lo<r2=u3 unsigned>> 14)
lotmp ^= (lou3 unsigned>> 14)
# asm 1: lotmp ^= (hi<r2=int64#4 << 18)
# asm 2: lotmp ^= (hi<r2=u3 << 18)
lotmp ^= (hiu3 << 18)
# asm 1: hitmp ^= (hi<r2=int64#4 unsigned>> 18)
# asm 2: hitmp ^= (hi<r2=u3 unsigned>> 18)
hitmp ^= (hiu3 unsigned>> 18)
# asm 1: hitmp ^= (lo<r2=int64#4 << 14)
# asm 2: hitmp ^= (lo<r2=u3 << 14)
hitmp ^= (lou3 << 14)
# asm 1: hitmp ^= (hi<r2=int64#4 unsigned>> 14)
# asm 2: hitmp ^= (hi<r2=u3 unsigned>> 14)
hitmp ^= (hiu3 unsigned>> 14)
# asm 1: hitmp ^= (lo<r2=int64#4 << 18)
# asm 2: hitmp ^= (lo<r2=u3 << 18)
hitmp ^= (lou3 << 18)
# asm 1: carry? lo<r5=int64#5 += lotmp
# asm 2: carry? lo<r5=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r5=int64#5 += hitmp + carry
# asm 2: hi<r5=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r2=int64#4 & lo<r3=int64#1
# asm 2: lotmp = lo<r2=u3 & lo<r3=u0
lotmp = lou3 & lou0
# asm 1: lotmp2 = lo<r4=int64#2 & ~lo<r2=int64#4
# asm 2: lotmp2 = lo<r4=u1 & ~lo<r2=u3
lotmp2 = lou1 & ~lou3
lotmp ^= lotmp2
# asm 1: carry? lo<r5=int64#5 += lotmp
# asm 2: carry? lo<r5=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r2=int64#4 & hi<r3=int64#1
# asm 2: hitmp = hi<r2=u3 & hi<r3=u0
hitmp = hiu3 & hiu0
# asm 1: hitmp2 = hi<r4=int64#2 & ~hi<r2=int64#4
# asm 2: hitmp2 = hi<r4=u1 & ~hi<r2=u3
hitmp2 = hiu1 & ~hiu3
hitmp ^= hitmp2
# asm 1: hi<r5=int64#5 += hitmp + carry
# asm 2: hi<r5=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r1 += r5
# asm 1: carry? lo<r1=int64#3 += lo<r5=int64#5
# asm 2: carry? lo<r1=u2 += lo<r5=u4
carry? lou2 += lou4
# asm 1: hi<r1=int64#3 += hi<r5=int64#5 + carry
# asm 2: hi<r1=u2 += hi<r5=u4 + carry
hiu2 += hiu4 + carry
# qhasm:       r1_spill = r1
# asm 1: lo>r1_spill=spill64#2 = lo<r1=int64#3
# asm 2: lo>r1_spill=d1 = lo<r1=u2
lod1 = lou2
# asm 1: hi>r1_spill=spill64#2 = hi<r1=int64#3
# asm 2: hi>r1_spill=d1 = hi<r1=u2
hid1 = hiu2
# qhasm:       r6 = r6_spill
# asm 1: lo>r6=int64#1 = lo<r6_spill=spill64#7
# asm 2: lo>r6=u0 = lo<r6_spill=d6
lou0 = lod6
# asm 1: hi>r6=int64#1 = hi<r6_spill=spill64#7
# asm 2: hi>r6=u0 = hi<r6_spill=d6
hiu0 = hid6
# qhasm:       r7 = r7_spill
# asm 1: lo>r7=int64#2 = lo<r7_spill=spill64#8
# asm 2: lo>r7=u1 = lo<r7_spill=d7
lou1 = lod7
# asm 1: hi>r7=int64#2 = hi<r7_spill=spill64#8
# asm 2: hi>r7=u1 = hi<r7_spill=d7
hiu1 = hid7
# qhasm:       r0 = r0_spill
# asm 1: lo>r0=int64#4 = lo<r0_spill=spill64#1
# asm 2: lo>r0=u3 = lo<r0_spill=d0
lou3 = lod0
# asm 1: hi>r0=int64#4 = hi<r0_spill=spill64#1
# asm 2: hi>r0=u3 = hi<r0_spill=d0
hiu3 = hid0
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r5 += Sigma0(r6) + Maj(r6,r7,r0)
# asm 1: hitmp lotmp = lo<r6=int64#1 * two25
# asm 2: hitmp lotmp = lo<r6=u0 * two25
hitmp lotmp = lou0 * two25
# asm 1: lotmp hitmp += hi<r6=int64#1 * two25
# asm 2: lotmp hitmp += hi<r6=u0 * two25
lotmp hitmp += hiu0 * two25
# asm 1: lotmp ^= (hi<r6=int64#1 unsigned>> 2)
# asm 2: lotmp ^= (hi<r6=u0 unsigned>> 2)
lotmp ^= (hiu0 unsigned>> 2)
# asm 1: lotmp ^= (lo<r6=int64#1 << 30)
# asm 2: lotmp ^= (lo<r6=u0 << 30)
lotmp ^= (lou0 << 30)
# asm 1: lotmp ^= (lo<r6=int64#1 unsigned>> 28)
# asm 2: lotmp ^= (lo<r6=u0 unsigned>> 28)
lotmp ^= (lou0 unsigned>> 28)
# asm 1: lotmp ^= (hi<r6=int64#1 << 4)
# asm 2: lotmp ^= (hi<r6=u0 << 4)
lotmp ^= (hiu0 << 4)
# asm 1: hitmp ^= (lo<r6=int64#1 unsigned>> 2)
# asm 2: hitmp ^= (lo<r6=u0 unsigned>> 2)
hitmp ^= (lou0 unsigned>> 2)
# asm 1: hitmp ^= (hi<r6=int64#1 << 30)
# asm 2: hitmp ^= (hi<r6=u0 << 30)
hitmp ^= (hiu0 << 30)
# asm 1: hitmp ^= (hi<r6=int64#1 unsigned>> 28)
# asm 2: hitmp ^= (hi<r6=u0 unsigned>> 28)
hitmp ^= (hiu0 unsigned>> 28)
# asm 1: hitmp ^= (lo<r6=int64#1 << 4)
# asm 2: hitmp ^= (lo<r6=u0 << 4)
hitmp ^= (lou0 << 4)
# asm 1: carry? lo<r5=int64#5 += lotmp
# asm 2: carry? lo<r5=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r5=int64#5 += hitmp + carry
# asm 2: hi<r5=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r7=int64#2 ^ lo<r0=int64#4
# asm 2: lotmp = lo<r7=u1 ^ lo<r0=u3
lotmp = lou1 ^ lou3
# asm 1: lotmp &= lo<r6=int64#1
# asm 2: lotmp &= lo<r6=u0
lotmp &= lou0
# asm 1: lotmp2 = lo<r7=int64#2 & lo<r0=int64#4
# asm 2: lotmp2 = lo<r7=u1 & lo<r0=u3
lotmp2 = lou1 & lou3
lotmp ^= lotmp2
# asm 1: carry? lo<r5=int64#5 += lotmp
# asm 2: carry? lo<r5=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r7=int64#2 ^ hi<r0=int64#4
# asm 2: hitmp = hi<r7=u1 ^ hi<r0=u3
hitmp = hiu1 ^ hiu3
# asm 1: hitmp &= hi<r6=int64#1
# asm 2: hitmp &= hi<r6=u0
hitmp &= hiu0
# asm 1: hitmp2 = hi<r7=int64#2 & hi<r0=int64#4
# asm 2: hitmp2 = hi<r7=u1 & hi<r0=u3
hitmp2 = hiu1 & hiu3
hitmp ^= hitmp2
# asm 1: hi<r5=int64#5 += hitmp + carry
# asm 2: hi<r5=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r5_spill = r5
# asm 1: lo>r5_spill=spill64#6 = lo<r5=int64#5
# asm 2: lo>r5_spill=d5 = lo<r5=u4
lod5 = lou4
# asm 1: hi>r5_spill=spill64#6 = hi<r5=int64#5
# asm 2: hi>r5_spill=d5 = hi<r5=u4
hid5 = hiu4
# qhasm:       r2 = r2_spill
# asm 1: lo>r2=int64#1 = lo<r2_spill=spill64#3
# asm 2: lo>r2=u0 = lo<r2_spill=d2
lou0 = lod2
# asm 1: hi>r2=int64#1 = hi<r2_spill=spill64#3
# asm 2: hi>r2=u0 = hi<r2_spill=d2
hiu0 = hid2
# qhasm:       r3 = r3_spill
# asm 1: lo>r3=int64#2 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u1 = lo<r3_spill=d3
lou1 = lod3
# asm 1: hi>r3=int64#2 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u1 = hi<r3_spill=d3
hiu1 = hid3
# qhasm:       r4 = r4_spill
# asm 1: lo>r4=int64#5 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u4 = lo<r4_spill=d4
lou4 = lod4
# asm 1: hi>r4=int64#5 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u4 = hi<r4_spill=d4
hiu4 = hid4
# qhasm:       w3 = w3_spill
# asm 1: lo>w3=int64#6 = lo<w3_spill=spill64#12
# asm 2: lo>w3=u5 = lo<w3_spill=d11
lou5 = lod11
# asm 1: hi>w3=int64#6 = hi<w3_spill=spill64#12
# asm 2: hi>w3=u5 = hi<w3_spill=d11
hiu5 = hid11
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r4 += w3 + mem64[constants] + Sigma1(r1) + Ch(r1,r2,r3); constants += 8
# asm 1: carry?  lo<r4=int64#5 += lo<w3=int64#6
# asm 2: carry?  lo<r4=u4 += lo<w3=u5
carry?  lou4 += lou5
# asm 1: hi<r4=int64#5 += hi<w3=int64#6 + carry
# asm 2: hi<r4=u4 += hi<w3=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r4=int64#5 += lotmp
# asm 2: carry? lo<r4=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r4=int64#5 += hitmp + carry
# asm 2: hi<r4=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r1=int64#3 * two23
# asm 2: hitmp lotmp = lo<r1=u2 * two23
hitmp lotmp = lou2 * two23
# asm 1: lotmp hitmp += hi<r1=int64#3 * two23
# asm 2: lotmp hitmp += hi<r1=u2 * two23
lotmp hitmp += hiu2 * two23
# asm 1: lotmp ^= (lo<r1=int64#3 unsigned>> 18)
# asm 2: lotmp ^= (lo<r1=u2 unsigned>> 18)
lotmp ^= (lou2 unsigned>> 18)
# asm 1: lotmp ^= (hi<r1=int64#3 << 14)
# asm 2: lotmp ^= (hi<r1=u2 << 14)
lotmp ^= (hiu2 << 14)
# asm 1: lotmp ^= (lo<r1=int64#3 unsigned>> 14)
# asm 2: lotmp ^= (lo<r1=u2 unsigned>> 14)
lotmp ^= (lou2 unsigned>> 14)
# asm 1: lotmp ^= (hi<r1=int64#3 << 18)
# asm 2: lotmp ^= (hi<r1=u2 << 18)
lotmp ^= (hiu2 << 18)
# asm 1: hitmp ^= (hi<r1=int64#3 unsigned>> 18)
# asm 2: hitmp ^= (hi<r1=u2 unsigned>> 18)
hitmp ^= (hiu2 unsigned>> 18)
# asm 1: hitmp ^= (lo<r1=int64#3 << 14)
# asm 2: hitmp ^= (lo<r1=u2 << 14)
hitmp ^= (lou2 << 14)
# asm 1: hitmp ^= (hi<r1=int64#3 unsigned>> 14)
# asm 2: hitmp ^= (hi<r1=u2 unsigned>> 14)
hitmp ^= (hiu2 unsigned>> 14)
# asm 1: hitmp ^= (lo<r1=int64#3 << 18)
# asm 2: hitmp ^= (lo<r1=u2 << 18)
hitmp ^= (lou2 << 18)
# asm 1: carry? lo<r4=int64#5 += lotmp
# asm 2: carry? lo<r4=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r4=int64#5 += hitmp + carry
# asm 2: hi<r4=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r1=int64#3 & lo<r2=int64#1
# asm 2: lotmp = lo<r1=u2 & lo<r2=u0
lotmp = lou2 & lou0
# asm 1: lotmp2 = lo<r3=int64#2 & ~lo<r1=int64#3
# asm 2: lotmp2 = lo<r3=u1 & ~lo<r1=u2
lotmp2 = lou1 & ~lou2
lotmp ^= lotmp2
# asm 1: carry? lo<r4=int64#5 += lotmp
# asm 2: carry? lo<r4=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r1=int64#3 & hi<r2=int64#1
# asm 2: hitmp = hi<r1=u2 & hi<r2=u0
hitmp = hiu2 & hiu0
# asm 1: hitmp2 = hi<r3=int64#2 & ~hi<r1=int64#3
# asm 2: hitmp2 = hi<r3=u1 & ~hi<r1=u2
hitmp2 = hiu1 & ~hiu2
hitmp ^= hitmp2
# asm 1: hi<r4=int64#5 += hitmp + carry
# asm 2: hi<r4=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r0 += r4
# asm 1: carry? lo<r0=int64#4 += lo<r4=int64#5
# asm 2: carry? lo<r0=u3 += lo<r4=u4
carry? lou3 += lou4
# asm 1: hi<r0=int64#4 += hi<r4=int64#5 + carry
# asm 2: hi<r0=u3 += hi<r4=u4 + carry
hiu3 += hiu4 + carry
# qhasm:       r0_spill = r0
# asm 1: lo>r0_spill=spill64#1 = lo<r0=int64#4
# asm 2: lo>r0_spill=d0 = lo<r0=u3
lod0 = lou3
# asm 1: hi>r0_spill=spill64#1 = hi<r0=int64#4
# asm 2: hi>r0_spill=d0 = hi<r0=u3
hid0 = hiu3
# qhasm:       r5 = r5_spill
# asm 1: lo>r5=int64#1 = lo<r5_spill=spill64#6
# asm 2: lo>r5=u0 = lo<r5_spill=d5
lou0 = lod5
# asm 1: hi>r5=int64#1 = hi<r5_spill=spill64#6
# asm 2: hi>r5=u0 = hi<r5_spill=d5
hiu0 = hid5
# qhasm:       r6 = r6_spill
# asm 1: lo>r6=int64#2 = lo<r6_spill=spill64#7
# asm 2: lo>r6=u1 = lo<r6_spill=d6
lou1 = lod6
# asm 1: hi>r6=int64#2 = hi<r6_spill=spill64#7
# asm 2: hi>r6=u1 = hi<r6_spill=d6
hiu1 = hid6
# qhasm:       r7 = r7_spill
# asm 1: lo>r7=int64#3 = lo<r7_spill=spill64#8
# asm 2: lo>r7=u2 = lo<r7_spill=d7
lou2 = lod7
# asm 1: hi>r7=int64#3 = hi<r7_spill=spill64#8
# asm 2: hi>r7=u2 = hi<r7_spill=d7
hiu2 = hid7
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r4 += Sigma0(r5) + Maj(r5,r6,r7)
# asm 1: hitmp lotmp = lo<r5=int64#1 * two25
# asm 2: hitmp lotmp = lo<r5=u0 * two25
hitmp lotmp = lou0 * two25
# asm 1: lotmp hitmp += hi<r5=int64#1 * two25
# asm 2: lotmp hitmp += hi<r5=u0 * two25
lotmp hitmp += hiu0 * two25
# asm 1: lotmp ^= (hi<r5=int64#1 unsigned>> 2)
# asm 2: lotmp ^= (hi<r5=u0 unsigned>> 2)
lotmp ^= (hiu0 unsigned>> 2)
# asm 1: lotmp ^= (lo<r5=int64#1 << 30)
# asm 2: lotmp ^= (lo<r5=u0 << 30)
lotmp ^= (lou0 << 30)
# asm 1: lotmp ^= (lo<r5=int64#1 unsigned>> 28)
# asm 2: lotmp ^= (lo<r5=u0 unsigned>> 28)
lotmp ^= (lou0 unsigned>> 28)
# asm 1: lotmp ^= (hi<r5=int64#1 << 4)
# asm 2: lotmp ^= (hi<r5=u0 << 4)
lotmp ^= (hiu0 << 4)
# asm 1: hitmp ^= (lo<r5=int64#1 unsigned>> 2)
# asm 2: hitmp ^= (lo<r5=u0 unsigned>> 2)
hitmp ^= (lou0 unsigned>> 2)
# asm 1: hitmp ^= (hi<r5=int64#1 << 30)
# asm 2: hitmp ^= (hi<r5=u0 << 30)
hitmp ^= (hiu0 << 30)
# asm 1: hitmp ^= (hi<r5=int64#1 unsigned>> 28)
# asm 2: hitmp ^= (hi<r5=u0 unsigned>> 28)
hitmp ^= (hiu0 unsigned>> 28)
# asm 1: hitmp ^= (lo<r5=int64#1 << 4)
# asm 2: hitmp ^= (lo<r5=u0 << 4)
hitmp ^= (lou0 << 4)
# asm 1: carry? lo<r4=int64#5 += lotmp
# asm 2: carry? lo<r4=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r4=int64#5 += hitmp + carry
# asm 2: hi<r4=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r6=int64#2 ^ lo<r7=int64#3
# asm 2: lotmp = lo<r6=u1 ^ lo<r7=u2
lotmp = lou1 ^ lou2
# asm 1: lotmp &= lo<r5=int64#1
# asm 2: lotmp &= lo<r5=u0
lotmp &= lou0
# asm 1: lotmp2 = lo<r6=int64#2 & lo<r7=int64#3
# asm 2: lotmp2 = lo<r6=u1 & lo<r7=u2
lotmp2 = lou1 & lou2
lotmp ^= lotmp2
# asm 1: carry? lo<r4=int64#5 += lotmp
# asm 2: carry? lo<r4=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r6=int64#2 ^ hi<r7=int64#3
# asm 2: hitmp = hi<r6=u1 ^ hi<r7=u2
hitmp = hiu1 ^ hiu2
# asm 1: hitmp &= hi<r5=int64#1
# asm 2: hitmp &= hi<r5=u0
hitmp &= hiu0
# asm 1: hitmp2 = hi<r6=int64#2 & hi<r7=int64#3
# asm 2: hitmp2 = hi<r6=u1 & hi<r7=u2
hitmp2 = hiu1 & hiu2
hitmp ^= hitmp2
# asm 1: hi<r4=int64#5 += hitmp + carry
# asm 2: hi<r4=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r4_spill = r4
# asm 1: lo>r4_spill=spill64#5 = lo<r4=int64#5
# asm 2: lo>r4_spill=d4 = lo<r4=u4
lod4 = lou4
# asm 1: hi>r4_spill=spill64#5 = hi<r4=int64#5
# asm 2: hi>r4_spill=d4 = hi<r4=u4
hid4 = hiu4
# qhasm:     assign 0 to r0_spill
# asm 1: assign 0 to lo<r0_spill=spill64#1
# asm 2: assign 0 to lo<r0_spill=d0
assign 0 to lod0
# asm 1: assign 1 to hi<r0_spill=spill64#1
# asm 2: assign 1 to hi<r0_spill=d0
assign 1 to hid0
# qhasm:     assign 1 to r1_spill
# asm 1: assign 2 to lo<r1_spill=spill64#2
# asm 2: assign 2 to lo<r1_spill=d1
assign 2 to lod1
# asm 1: assign 3 to hi<r1_spill=spill64#2
# asm 2: assign 3 to hi<r1_spill=d1
assign 3 to hid1
# qhasm:     assign 2 to r2_spill
# asm 1: assign 4 to lo<r2_spill=spill64#3
# asm 2: assign 4 to lo<r2_spill=d2
assign 4 to lod2
# asm 1: assign 5 to hi<r2_spill=spill64#3
# asm 2: assign 5 to hi<r2_spill=d2
assign 5 to hid2
# qhasm:     assign 3 to r3_spill
# asm 1: assign 6 to lo<r3_spill=spill64#4
# asm 2: assign 6 to lo<r3_spill=d3
assign 6 to lod3
# asm 1: assign 7 to hi<r3_spill=spill64#4
# asm 2: assign 7 to hi<r3_spill=d3
assign 7 to hid3
# qhasm:     assign 4 to r4_spill
# asm 1: assign 8 to lo<r4_spill=spill64#5
# asm 2: assign 8 to lo<r4_spill=d4
assign 8 to lod4
# asm 1: assign 9 to hi<r4_spill=spill64#5
# asm 2: assign 9 to hi<r4_spill=d4
assign 9 to hid4
# qhasm:     assign 5 to r5_spill
# asm 1: assign 10 to lo<r5_spill=spill64#6
# asm 2: assign 10 to lo<r5_spill=d5
assign 10 to lod5
# asm 1: assign 11 to hi<r5_spill=spill64#6
# asm 2: assign 11 to hi<r5_spill=d5
assign 11 to hid5
# qhasm:     assign 6 to r6_spill
# asm 1: assign 12 to lo<r6_spill=spill64#7
# asm 2: assign 12 to lo<r6_spill=d6
assign 12 to lod6
# asm 1: assign 13 to hi<r6_spill=spill64#7
# asm 2: assign 13 to hi<r6_spill=d6
assign 13 to hid6
# qhasm:     assign 7 to r7_spill
# asm 1: assign 14 to lo<r7_spill=spill64#8
# asm 2: assign 14 to lo<r7_spill=d7
assign 14 to lod7
# asm 1: assign 15 to hi<r7_spill=spill64#8
# asm 2: assign 15 to hi<r7_spill=d7
assign 15 to hid7
# qhasm:       r1 = r1_spill
# asm 1: lo>r1=int64#1 = lo<r1_spill=spill64#2
# asm 2: lo>r1=u0 = lo<r1_spill=d1
lou0 = lod1
# asm 1: hi>r1=int64#1 = hi<r1_spill=spill64#2
# asm 2: hi>r1=u0 = hi<r1_spill=d1
hiu0 = hid1
# qhasm:       r2 = r2_spill
# asm 1: lo>r2=int64#2 = lo<r2_spill=spill64#3
# asm 2: lo>r2=u1 = lo<r2_spill=d2
lou1 = lod2
# asm 1: hi>r2=int64#2 = hi<r2_spill=spill64#3
# asm 2: hi>r2=u1 = hi<r2_spill=d2
hiu1 = hid2
# qhasm:       r3 = r3_spill
# asm 1: lo>r3=int64#5 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u4 = lo<r3_spill=d3
lou4 = lod3
# asm 1: hi>r3=int64#5 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u4 = hi<r3_spill=d3
hiu4 = hid3
# qhasm:       w4 = w4_spill
# asm 1: lo>w4=int64#6 = lo<w4_spill=spill64#13
# asm 2: lo>w4=u5 = lo<w4_spill=d12
lou5 = lod12
# asm 1: hi>w4=int64#6 = hi<w4_spill=spill64#13
# asm 2: hi>w4=u5 = hi<w4_spill=d12
hiu5 = hid12
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r3 += w4 + mem64[constants] + Sigma1(r0) + Ch(r0,r1,r2); constants += 8
# asm 1: carry?  lo<r3=int64#5 += lo<w4=int64#6
# asm 2: carry?  lo<r3=u4 += lo<w4=u5
carry?  lou4 += lou5
# asm 1: hi<r3=int64#5 += hi<w4=int64#6 + carry
# asm 2: hi<r3=u4 += hi<w4=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r3=int64#5 += lotmp
# asm 2: carry? lo<r3=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r3=int64#5 += hitmp + carry
# asm 2: hi<r3=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r0=int64#4 * two23
# asm 2: hitmp lotmp = lo<r0=u3 * two23
hitmp lotmp = lou3 * two23
# asm 1: lotmp hitmp += hi<r0=int64#4 * two23
# asm 2: lotmp hitmp += hi<r0=u3 * two23
lotmp hitmp += hiu3 * two23
# asm 1: lotmp ^= (lo<r0=int64#4 unsigned>> 18)
# asm 2: lotmp ^= (lo<r0=u3 unsigned>> 18)
lotmp ^= (lou3 unsigned>> 18)
# asm 1: lotmp ^= (hi<r0=int64#4 << 14)
# asm 2: lotmp ^= (hi<r0=u3 << 14)
lotmp ^= (hiu3 << 14)
# asm 1: lotmp ^= (lo<r0=int64#4 unsigned>> 14)
# asm 2: lotmp ^= (lo<r0=u3 unsigned>> 14)
lotmp ^= (lou3 unsigned>> 14)
# asm 1: lotmp ^= (hi<r0=int64#4 << 18)
# asm 2: lotmp ^= (hi<r0=u3 << 18)
lotmp ^= (hiu3 << 18)
# asm 1: hitmp ^= (hi<r0=int64#4 unsigned>> 18)
# asm 2: hitmp ^= (hi<r0=u3 unsigned>> 18)
hitmp ^= (hiu3 unsigned>> 18)
# asm 1: hitmp ^= (lo<r0=int64#4 << 14)
# asm 2: hitmp ^= (lo<r0=u3 << 14)
hitmp ^= (lou3 << 14)
# asm 1: hitmp ^= (hi<r0=int64#4 unsigned>> 14)
# asm 2: hitmp ^= (hi<r0=u3 unsigned>> 14)
hitmp ^= (hiu3 unsigned>> 14)
# asm 1: hitmp ^= (lo<r0=int64#4 << 18)
# asm 2: hitmp ^= (lo<r0=u3 << 18)
hitmp ^= (lou3 << 18)
# asm 1: carry? lo<r3=int64#5 += lotmp
# asm 2: carry? lo<r3=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r3=int64#5 += hitmp + carry
# asm 2: hi<r3=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r0=int64#4 & lo<r1=int64#1
# asm 2: lotmp = lo<r0=u3 & lo<r1=u0
lotmp = lou3 & lou0
# asm 1: lotmp2 = lo<r2=int64#2 & ~lo<r0=int64#4
# asm 2: lotmp2 = lo<r2=u1 & ~lo<r0=u3
lotmp2 = lou1 & ~lou3
lotmp ^= lotmp2
# asm 1: carry? lo<r3=int64#5 += lotmp
# asm 2: carry? lo<r3=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r0=int64#4 & hi<r1=int64#1
# asm 2: hitmp = hi<r0=u3 & hi<r1=u0
hitmp = hiu3 & hiu0
# asm 1: hitmp2 = hi<r2=int64#2 & ~hi<r0=int64#4
# asm 2: hitmp2 = hi<r2=u1 & ~hi<r0=u3
hitmp2 = hiu1 & ~hiu3
hitmp ^= hitmp2
# asm 1: hi<r3=int64#5 += hitmp + carry
# asm 2: hi<r3=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r7 += r3
# asm 1: carry? lo<r7=int64#3 += lo<r3=int64#5
# asm 2: carry? lo<r7=u2 += lo<r3=u4
carry? lou2 += lou4
# asm 1: hi<r7=int64#3 += hi<r3=int64#5 + carry
# asm 2: hi<r7=u2 += hi<r3=u4 + carry
hiu2 += hiu4 + carry
# qhasm:       r7_spill = r7
# asm 1: lo>r7_spill=spill64#8 = lo<r7=int64#3
# asm 2: lo>r7_spill=d7 = lo<r7=u2
lod7 = lou2
# asm 1: hi>r7_spill=spill64#8 = hi<r7=int64#3
# asm 2: hi>r7_spill=d7 = hi<r7=u2
hid7 = hiu2
# qhasm:       r4 = r4_spill
# asm 1: lo>r4=int64#1 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u0 = lo<r4_spill=d4
lou0 = lod4
# asm 1: hi>r4=int64#1 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u0 = hi<r4_spill=d4
hiu0 = hid4
# qhasm:       r5 = r5_spill
# asm 1: lo>r5=int64#2 = lo<r5_spill=spill64#6
# asm 2: lo>r5=u1 = lo<r5_spill=d5
lou1 = lod5
# asm 1: hi>r5=int64#2 = hi<r5_spill=spill64#6
# asm 2: hi>r5=u1 = hi<r5_spill=d5
hiu1 = hid5
# qhasm:       r6 = r6_spill
# asm 1: lo>r6=int64#4 = lo<r6_spill=spill64#7
# asm 2: lo>r6=u3 = lo<r6_spill=d6
lou3 = lod6
# asm 1: hi>r6=int64#4 = hi<r6_spill=spill64#7
# asm 2: hi>r6=u3 = hi<r6_spill=d6
hiu3 = hid6
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r3 += Sigma0(r4) + Maj(r4,r5,r6)
# asm 1: hitmp lotmp = lo<r4=int64#1 * two25
# asm 2: hitmp lotmp = lo<r4=u0 * two25
hitmp lotmp = lou0 * two25
# asm 1: lotmp hitmp += hi<r4=int64#1 * two25
# asm 2: lotmp hitmp += hi<r4=u0 * two25
lotmp hitmp += hiu0 * two25
# asm 1: lotmp ^= (hi<r4=int64#1 unsigned>> 2)
# asm 2: lotmp ^= (hi<r4=u0 unsigned>> 2)
lotmp ^= (hiu0 unsigned>> 2)
# asm 1: lotmp ^= (lo<r4=int64#1 << 30)
# asm 2: lotmp ^= (lo<r4=u0 << 30)
lotmp ^= (lou0 << 30)
# asm 1: lotmp ^= (lo<r4=int64#1 unsigned>> 28)
# asm 2: lotmp ^= (lo<r4=u0 unsigned>> 28)
lotmp ^= (lou0 unsigned>> 28)
# asm 1: lotmp ^= (hi<r4=int64#1 << 4)
# asm 2: lotmp ^= (hi<r4=u0 << 4)
lotmp ^= (hiu0 << 4)
# asm 1: hitmp ^= (lo<r4=int64#1 unsigned>> 2)
# asm 2: hitmp ^= (lo<r4=u0 unsigned>> 2)
hitmp ^= (lou0 unsigned>> 2)
# asm 1: hitmp ^= (hi<r4=int64#1 << 30)
# asm 2: hitmp ^= (hi<r4=u0 << 30)
hitmp ^= (hiu0 << 30)
# asm 1: hitmp ^= (hi<r4=int64#1 unsigned>> 28)
# asm 2: hitmp ^= (hi<r4=u0 unsigned>> 28)
hitmp ^= (hiu0 unsigned>> 28)
# asm 1: hitmp ^= (lo<r4=int64#1 << 4)
# asm 2: hitmp ^= (lo<r4=u0 << 4)
hitmp ^= (lou0 << 4)
# asm 1: carry? lo<r3=int64#5 += lotmp
# asm 2: carry? lo<r3=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r3=int64#5 += hitmp + carry
# asm 2: hi<r3=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r5=int64#2 ^ lo<r6=int64#4
# asm 2: lotmp = lo<r5=u1 ^ lo<r6=u3
lotmp = lou1 ^ lou3
# asm 1: lotmp &= lo<r4=int64#1
# asm 2: lotmp &= lo<r4=u0
lotmp &= lou0
# asm 1: lotmp2 = lo<r5=int64#2 & lo<r6=int64#4
# asm 2: lotmp2 = lo<r5=u1 & lo<r6=u3
lotmp2 = lou1 & lou3
lotmp ^= lotmp2
# asm 1: carry? lo<r3=int64#5 += lotmp
# asm 2: carry? lo<r3=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r5=int64#2 ^ hi<r6=int64#4
# asm 2: hitmp = hi<r5=u1 ^ hi<r6=u3
hitmp = hiu1 ^ hiu3
# asm 1: hitmp &= hi<r4=int64#1
# asm 2: hitmp &= hi<r4=u0
hitmp &= hiu0
# asm 1: hitmp2 = hi<r5=int64#2 & hi<r6=int64#4
# asm 2: hitmp2 = hi<r5=u1 & hi<r6=u3
hitmp2 = hiu1 & hiu3
hitmp ^= hitmp2
# asm 1: hi<r3=int64#5 += hitmp + carry
# asm 2: hi<r3=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r3_spill = r3
# asm 1: lo>r3_spill=spill64#4 = lo<r3=int64#5
# asm 2: lo>r3_spill=d3 = lo<r3=u4
lod3 = lou4
# asm 1: hi>r3_spill=spill64#4 = hi<r3=int64#5
# asm 2: hi>r3_spill=d3 = hi<r3=u4
hid3 = hiu4
# qhasm:       r0 = r0_spill
# asm 1: lo>r0=int64#1 = lo<r0_spill=spill64#1
# asm 2: lo>r0=u0 = lo<r0_spill=d0
lou0 = lod0
# asm 1: hi>r0=int64#1 = hi<r0_spill=spill64#1
# asm 2: hi>r0=u0 = hi<r0_spill=d0
hiu0 = hid0
# qhasm:       r1 = r1_spill
# asm 1: lo>r1=int64#2 = lo<r1_spill=spill64#2
# asm 2: lo>r1=u1 = lo<r1_spill=d1
lou1 = lod1
# asm 1: hi>r1=int64#2 = hi<r1_spill=spill64#2
# asm 2: hi>r1=u1 = hi<r1_spill=d1
hiu1 = hid1
# qhasm:       r2 = r2_spill
# asm 1: lo>r2=int64#5 = lo<r2_spill=spill64#3
# asm 2: lo>r2=u4 = lo<r2_spill=d2
lou4 = lod2
# asm 1: hi>r2=int64#5 = hi<r2_spill=spill64#3
# asm 2: hi>r2=u4 = hi<r2_spill=d2
hiu4 = hid2
# qhasm:       w5 = w5_spill
# asm 1: lo>w5=int64#6 = lo<w5_spill=spill64#14
# asm 2: lo>w5=u5 = lo<w5_spill=d13
lou5 = lod13
# asm 1: hi>w5=int64#6 = hi<w5_spill=spill64#14
# asm 2: hi>w5=u5 = hi<w5_spill=d13
hiu5 = hid13
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r2 += w5 + mem64[constants] + Sigma1(r7) + Ch(r7,r0,r1); constants += 8
# asm 1: carry?  lo<r2=int64#5 += lo<w5=int64#6
# asm 2: carry?  lo<r2=u4 += lo<w5=u5
carry?  lou4 += lou5
# asm 1: hi<r2=int64#5 += hi<w5=int64#6 + carry
# asm 2: hi<r2=u4 += hi<w5=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r2=int64#5 += lotmp
# asm 2: carry? lo<r2=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r2=int64#5 += hitmp + carry
# asm 2: hi<r2=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r7=int64#3 * two23
# asm 2: hitmp lotmp = lo<r7=u2 * two23
hitmp lotmp = lou2 * two23
# asm 1: lotmp hitmp += hi<r7=int64#3 * two23
# asm 2: lotmp hitmp += hi<r7=u2 * two23
lotmp hitmp += hiu2 * two23
# asm 1: lotmp ^= (lo<r7=int64#3 unsigned>> 18)
# asm 2: lotmp ^= (lo<r7=u2 unsigned>> 18)
lotmp ^= (lou2 unsigned>> 18)
# asm 1: lotmp ^= (hi<r7=int64#3 << 14)
# asm 2: lotmp ^= (hi<r7=u2 << 14)
lotmp ^= (hiu2 << 14)
# asm 1: lotmp ^= (lo<r7=int64#3 unsigned>> 14)
# asm 2: lotmp ^= (lo<r7=u2 unsigned>> 14)
lotmp ^= (lou2 unsigned>> 14)
# asm 1: lotmp ^= (hi<r7=int64#3 << 18)
# asm 2: lotmp ^= (hi<r7=u2 << 18)
lotmp ^= (hiu2 << 18)
# asm 1: hitmp ^= (hi<r7=int64#3 unsigned>> 18)
# asm 2: hitmp ^= (hi<r7=u2 unsigned>> 18)
hitmp ^= (hiu2 unsigned>> 18)
# asm 1: hitmp ^= (lo<r7=int64#3 << 14)
# asm 2: hitmp ^= (lo<r7=u2 << 14)
hitmp ^= (lou2 << 14)
# asm 1: hitmp ^= (hi<r7=int64#3 unsigned>> 14)
# asm 2: hitmp ^= (hi<r7=u2 unsigned>> 14)
hitmp ^= (hiu2 unsigned>> 14)
# asm 1: hitmp ^= (lo<r7=int64#3 << 18)
# asm 2: hitmp ^= (lo<r7=u2 << 18)
hitmp ^= (lou2 << 18)
# asm 1: carry? lo<r2=int64#5 += lotmp
# asm 2: carry? lo<r2=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r2=int64#5 += hitmp + carry
# asm 2: hi<r2=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r7=int64#3 & lo<r0=int64#1
# asm 2: lotmp = lo<r7=u2 & lo<r0=u0
lotmp = lou2 & lou0
# asm 1: lotmp2 = lo<r1=int64#2 & ~lo<r7=int64#3
# asm 2: lotmp2 = lo<r1=u1 & ~lo<r7=u2
lotmp2 = lou1 & ~lou2
lotmp ^= lotmp2
# asm 1: carry? lo<r2=int64#5 += lotmp
# asm 2: carry? lo<r2=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r7=int64#3 & hi<r0=int64#1
# asm 2: hitmp = hi<r7=u2 & hi<r0=u0
hitmp = hiu2 & hiu0
# asm 1: hitmp2 = hi<r1=int64#2 & ~hi<r7=int64#3
# asm 2: hitmp2 = hi<r1=u1 & ~hi<r7=u2
hitmp2 = hiu1 & ~hiu2
hitmp ^= hitmp2
# asm 1: hi<r2=int64#5 += hitmp + carry
# asm 2: hi<r2=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r6 += r2
# asm 1: carry? lo<r6=int64#4 += lo<r2=int64#5
# asm 2: carry? lo<r6=u3 += lo<r2=u4
carry? lou3 += lou4
# asm 1: hi<r6=int64#4 += hi<r2=int64#5 + carry
# asm 2: hi<r6=u3 += hi<r2=u4 + carry
hiu3 += hiu4 + carry
# qhasm:       r6_spill = r6
# asm 1: lo>r6_spill=spill64#7 = lo<r6=int64#4
# asm 2: lo>r6_spill=d6 = lo<r6=u3
lod6 = lou3
# asm 1: hi>r6_spill=spill64#7 = hi<r6=int64#4
# asm 2: hi>r6_spill=d6 = hi<r6=u3
hid6 = hiu3
# qhasm:       r3 = r3_spill
# asm 1: lo>r3=int64#1 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u0 = lo<r3_spill=d3
lou0 = lod3
# asm 1: hi>r3=int64#1 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u0 = hi<r3_spill=d3
hiu0 = hid3
# qhasm:       r4 = r4_spill
# asm 1: lo>r4=int64#2 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u1 = lo<r4_spill=d4
lou1 = lod4
# asm 1: hi>r4=int64#2 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u1 = hi<r4_spill=d4
hiu1 = hid4
# qhasm:       r5 = r5_spill
# asm 1: lo>r5=int64#3 = lo<r5_spill=spill64#6
# asm 2: lo>r5=u2 = lo<r5_spill=d5
lou2 = lod5
# asm 1: hi>r5=int64#3 = hi<r5_spill=spill64#6
# asm 2: hi>r5=u2 = hi<r5_spill=d5
hiu2 = hid5
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r2 += Sigma0(r3) + Maj(r3,r4,r5)
# asm 1: hitmp lotmp = lo<r3=int64#1 * two25
# asm 2: hitmp lotmp = lo<r3=u0 * two25
hitmp lotmp = lou0 * two25
# asm 1: lotmp hitmp += hi<r3=int64#1 * two25
# asm 2: lotmp hitmp += hi<r3=u0 * two25
lotmp hitmp += hiu0 * two25
# asm 1: lotmp ^= (hi<r3=int64#1 unsigned>> 2)
# asm 2: lotmp ^= (hi<r3=u0 unsigned>> 2)
lotmp ^= (hiu0 unsigned>> 2)
# asm 1: lotmp ^= (lo<r3=int64#1 << 30)
# asm 2: lotmp ^= (lo<r3=u0 << 30)
lotmp ^= (lou0 << 30)
# asm 1: lotmp ^= (lo<r3=int64#1 unsigned>> 28)
# asm 2: lotmp ^= (lo<r3=u0 unsigned>> 28)
lotmp ^= (lou0 unsigned>> 28)
# asm 1: lotmp ^= (hi<r3=int64#1 << 4)
# asm 2: lotmp ^= (hi<r3=u0 << 4)
lotmp ^= (hiu0 << 4)
# asm 1: hitmp ^= (lo<r3=int64#1 unsigned>> 2)
# asm 2: hitmp ^= (lo<r3=u0 unsigned>> 2)
hitmp ^= (lou0 unsigned>> 2)
# asm 1: hitmp ^= (hi<r3=int64#1 << 30)
# asm 2: hitmp ^= (hi<r3=u0 << 30)
hitmp ^= (hiu0 << 30)
# asm 1: hitmp ^= (hi<r3=int64#1 unsigned>> 28)
# asm 2: hitmp ^= (hi<r3=u0 unsigned>> 28)
hitmp ^= (hiu0 unsigned>> 28)
# asm 1: hitmp ^= (lo<r3=int64#1 << 4)
# asm 2: hitmp ^= (lo<r3=u0 << 4)
hitmp ^= (lou0 << 4)
# asm 1: carry? lo<r2=int64#5 += lotmp
# asm 2: carry? lo<r2=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r2=int64#5 += hitmp + carry
# asm 2: hi<r2=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r4=int64#2 ^ lo<r5=int64#3
# asm 2: lotmp = lo<r4=u1 ^ lo<r5=u2
lotmp = lou1 ^ lou2
# asm 1: lotmp &= lo<r3=int64#1
# asm 2: lotmp &= lo<r3=u0
lotmp &= lou0
# asm 1: lotmp2 = lo<r4=int64#2 & lo<r5=int64#3
# asm 2: lotmp2 = lo<r4=u1 & lo<r5=u2
lotmp2 = lou1 & lou2
lotmp ^= lotmp2
# asm 1: carry? lo<r2=int64#5 += lotmp
# asm 2: carry? lo<r2=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r4=int64#2 ^ hi<r5=int64#3
# asm 2: hitmp = hi<r4=u1 ^ hi<r5=u2
hitmp = hiu1 ^ hiu2
# asm 1: hitmp &= hi<r3=int64#1
# asm 2: hitmp &= hi<r3=u0
hitmp &= hiu0
# asm 1: hitmp2 = hi<r4=int64#2 & hi<r5=int64#3
# asm 2: hitmp2 = hi<r4=u1 & hi<r5=u2
hitmp2 = hiu1 & hiu2
hitmp ^= hitmp2
# asm 1: hi<r2=int64#5 += hitmp + carry
# asm 2: hi<r2=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r2_spill = r2
# asm 1: lo>r2_spill=spill64#3 = lo<r2=int64#5
# asm 2: lo>r2_spill=d2 = lo<r2=u4
lod2 = lou4
# asm 1: hi>r2_spill=spill64#3 = hi<r2=int64#5
# asm 2: hi>r2_spill=d2 = hi<r2=u4
hid2 = hiu4
# qhasm:     assign 0 to r0_spill
# asm 1: assign 0 to lo<r0_spill=spill64#1
# asm 2: assign 0 to lo<r0_spill=d0
assign 0 to lod0
# asm 1: assign 1 to hi<r0_spill=spill64#1
# asm 2: assign 1 to hi<r0_spill=d0
assign 1 to hid0
# qhasm:     assign 1 to r1_spill
# asm 1: assign 2 to lo<r1_spill=spill64#2
# asm 2: assign 2 to lo<r1_spill=d1
assign 2 to lod1
# asm 1: assign 3 to hi<r1_spill=spill64#2
# asm 2: assign 3 to hi<r1_spill=d1
assign 3 to hid1
# qhasm:     assign 2 to r2_spill
# asm 1: assign 4 to lo<r2_spill=spill64#3
# asm 2: assign 4 to lo<r2_spill=d2
assign 4 to lod2
# asm 1: assign 5 to hi<r2_spill=spill64#3
# asm 2: assign 5 to hi<r2_spill=d2
assign 5 to hid2
# qhasm:     assign 3 to r3_spill
# asm 1: assign 6 to lo<r3_spill=spill64#4
# asm 2: assign 6 to lo<r3_spill=d3
assign 6 to lod3
# asm 1: assign 7 to hi<r3_spill=spill64#4
# asm 2: assign 7 to hi<r3_spill=d3
assign 7 to hid3
# qhasm:     assign 4 to r4_spill
# asm 1: assign 8 to lo<r4_spill=spill64#5
# asm 2: assign 8 to lo<r4_spill=d4
assign 8 to lod4
# asm 1: assign 9 to hi<r4_spill=spill64#5
# asm 2: assign 9 to hi<r4_spill=d4
assign 9 to hid4
# qhasm:     assign 5 to r5_spill
# asm 1: assign 10 to lo<r5_spill=spill64#6
# asm 2: assign 10 to lo<r5_spill=d5
assign 10 to lod5
# asm 1: assign 11 to hi<r5_spill=spill64#6
# asm 2: assign 11 to hi<r5_spill=d5
assign 11 to hid5
# qhasm:     assign 6 to r6_spill
# asm 1: assign 12 to lo<r6_spill=spill64#7
# asm 2: assign 12 to lo<r6_spill=d6
assign 12 to lod6
# asm 1: assign 13 to hi<r6_spill=spill64#7
# asm 2: assign 13 to hi<r6_spill=d6
assign 13 to hid6
# qhasm:     assign 7 to r7_spill
# asm 1: assign 14 to lo<r7_spill=spill64#8
# asm 2: assign 14 to lo<r7_spill=d7
assign 14 to lod7
# asm 1: assign 15 to hi<r7_spill=spill64#8
# asm 2: assign 15 to hi<r7_spill=d7
assign 15 to hid7
# qhasm:       r7 = r7_spill
# asm 1: lo>r7=int64#1 = lo<r7_spill=spill64#8
# asm 2: lo>r7=u0 = lo<r7_spill=d7
lou0 = lod7
# asm 1: hi>r7=int64#1 = hi<r7_spill=spill64#8
# asm 2: hi>r7=u0 = hi<r7_spill=d7
hiu0 = hid7
# qhasm:       r0 = r0_spill
# asm 1: lo>r0=int64#2 = lo<r0_spill=spill64#1
# asm 2: lo>r0=u1 = lo<r0_spill=d0
lou1 = lod0
# asm 1: hi>r0=int64#2 = hi<r0_spill=spill64#1
# asm 2: hi>r0=u1 = hi<r0_spill=d0
hiu1 = hid0
# qhasm:       r1 = r1_spill
# asm 1: lo>r1=int64#5 = lo<r1_spill=spill64#2
# asm 2: lo>r1=u4 = lo<r1_spill=d1
lou4 = lod1
# asm 1: hi>r1=int64#5 = hi<r1_spill=spill64#2
# asm 2: hi>r1=u4 = hi<r1_spill=d1
hiu4 = hid1
# qhasm:       w6 = w6_spill
# asm 1: lo>w6=int64#6 = lo<w6_spill=spill64#15
# asm 2: lo>w6=u5 = lo<w6_spill=d14
lou5 = lod14
# asm 1: hi>w6=int64#6 = hi<w6_spill=spill64#15
# asm 2: hi>w6=u5 = hi<w6_spill=d14
hiu5 = hid14
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r1 += w6 + mem64[constants] + Sigma1(r6) + Ch(r6,r7,r0); constants += 8
# asm 1: carry?  lo<r1=int64#5 += lo<w6=int64#6
# asm 2: carry?  lo<r1=u4 += lo<w6=u5
carry?  lou4 += lou5
# asm 1: hi<r1=int64#5 += hi<w6=int64#6 + carry
# asm 2: hi<r1=u4 += hi<w6=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r1=int64#5 += lotmp
# asm 2: carry? lo<r1=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r1=int64#5 += hitmp + carry
# asm 2: hi<r1=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r6=int64#4 * two23
# asm 2: hitmp lotmp = lo<r6=u3 * two23
hitmp lotmp = lou3 * two23
# asm 1: lotmp hitmp += hi<r6=int64#4 * two23
# asm 2: lotmp hitmp += hi<r6=u3 * two23
lotmp hitmp += hiu3 * two23
# asm 1: lotmp ^= (lo<r6=int64#4 unsigned>> 18)
# asm 2: lotmp ^= (lo<r6=u3 unsigned>> 18)
lotmp ^= (lou3 unsigned>> 18)
# asm 1: lotmp ^= (hi<r6=int64#4 << 14)
# asm 2: lotmp ^= (hi<r6=u3 << 14)
lotmp ^= (hiu3 << 14)
# asm 1: lotmp ^= (lo<r6=int64#4 unsigned>> 14)
# asm 2: lotmp ^= (lo<r6=u3 unsigned>> 14)
lotmp ^= (lou3 unsigned>> 14)
# asm 1: lotmp ^= (hi<r6=int64#4 << 18)
# asm 2: lotmp ^= (hi<r6=u3 << 18)
lotmp ^= (hiu3 << 18)
# asm 1: hitmp ^= (hi<r6=int64#4 unsigned>> 18)
# asm 2: hitmp ^= (hi<r6=u3 unsigned>> 18)
hitmp ^= (hiu3 unsigned>> 18)
# asm 1: hitmp ^= (lo<r6=int64#4 << 14)
# asm 2: hitmp ^= (lo<r6=u3 << 14)
hitmp ^= (lou3 << 14)
# asm 1: hitmp ^= (hi<r6=int64#4 unsigned>> 14)
# asm 2: hitmp ^= (hi<r6=u3 unsigned>> 14)
hitmp ^= (hiu3 unsigned>> 14)
# asm 1: hitmp ^= (lo<r6=int64#4 << 18)
# asm 2: hitmp ^= (lo<r6=u3 << 18)
hitmp ^= (lou3 << 18)
# asm 1: carry? lo<r1=int64#5 += lotmp
# asm 2: carry? lo<r1=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r1=int64#5 += hitmp + carry
# asm 2: hi<r1=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r6=int64#4 & lo<r7=int64#1
# asm 2: lotmp = lo<r6=u3 & lo<r7=u0
lotmp = lou3 & lou0
# asm 1: lotmp2 = lo<r0=int64#2 & ~lo<r6=int64#4
# asm 2: lotmp2 = lo<r0=u1 & ~lo<r6=u3
lotmp2 = lou1 & ~lou3
lotmp ^= lotmp2
# asm 1: carry? lo<r1=int64#5 += lotmp
# asm 2: carry? lo<r1=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r6=int64#4 & hi<r7=int64#1
# asm 2: hitmp = hi<r6=u3 & hi<r7=u0
hitmp = hiu3 & hiu0
# asm 1: hitmp2 = hi<r0=int64#2 & ~hi<r6=int64#4
# asm 2: hitmp2 = hi<r0=u1 & ~hi<r6=u3
hitmp2 = hiu1 & ~hiu3
hitmp ^= hitmp2
# asm 1: hi<r1=int64#5 += hitmp + carry
# asm 2: hi<r1=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r5 += r1
# asm 1: carry? lo<r5=int64#3 += lo<r1=int64#5
# asm 2: carry? lo<r5=u2 += lo<r1=u4
carry? lou2 += lou4
# asm 1: hi<r5=int64#3 += hi<r1=int64#5 + carry
# asm 2: hi<r5=u2 += hi<r1=u4 + carry
hiu2 += hiu4 + carry
# qhasm:       r5_spill = r5
# asm 1: lo>r5_spill=spill64#6 = lo<r5=int64#3
# asm 2: lo>r5_spill=d5 = lo<r5=u2
lod5 = lou2
# asm 1: hi>r5_spill=spill64#6 = hi<r5=int64#3
# asm 2: hi>r5_spill=d5 = hi<r5=u2
hid5 = hiu2
# qhasm:       r2 = r2_spill
# asm 1: lo>r2=int64#1 = lo<r2_spill=spill64#3
# asm 2: lo>r2=u0 = lo<r2_spill=d2
lou0 = lod2
# asm 1: hi>r2=int64#1 = hi<r2_spill=spill64#3
# asm 2: hi>r2=u0 = hi<r2_spill=d2
hiu0 = hid2
# qhasm:       r3 = r3_spill
# asm 1: lo>r3=int64#2 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u1 = lo<r3_spill=d3
lou1 = lod3
# asm 1: hi>r3=int64#2 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u1 = hi<r3_spill=d3
hiu1 = hid3
# qhasm:       r4 = r4_spill
# asm 1: lo>r4=int64#4 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u3 = lo<r4_spill=d4
lou3 = lod4
# asm 1: hi>r4=int64#4 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u3 = hi<r4_spill=d4
hiu3 = hid4
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r1 += Sigma0(r2) + Maj(r2,r3,r4)
# asm 1: hitmp lotmp = lo<r2=int64#1 * two25
# asm 2: hitmp lotmp = lo<r2=u0 * two25
hitmp lotmp = lou0 * two25
# asm 1: lotmp hitmp += hi<r2=int64#1 * two25
# asm 2: lotmp hitmp += hi<r2=u0 * two25
lotmp hitmp += hiu0 * two25
# asm 1: lotmp ^= (hi<r2=int64#1 unsigned>> 2)
# asm 2: lotmp ^= (hi<r2=u0 unsigned>> 2)
lotmp ^= (hiu0 unsigned>> 2)
# asm 1: lotmp ^= (lo<r2=int64#1 << 30)
# asm 2: lotmp ^= (lo<r2=u0 << 30)
lotmp ^= (lou0 << 30)
# asm 1: lotmp ^= (lo<r2=int64#1 unsigned>> 28)
# asm 2: lotmp ^= (lo<r2=u0 unsigned>> 28)
lotmp ^= (lou0 unsigned>> 28)
# asm 1: lotmp ^= (hi<r2=int64#1 << 4)
# asm 2: lotmp ^= (hi<r2=u0 << 4)
lotmp ^= (hiu0 << 4)
# asm 1: hitmp ^= (lo<r2=int64#1 unsigned>> 2)
# asm 2: hitmp ^= (lo<r2=u0 unsigned>> 2)
hitmp ^= (lou0 unsigned>> 2)
# asm 1: hitmp ^= (hi<r2=int64#1 << 30)
# asm 2: hitmp ^= (hi<r2=u0 << 30)
hitmp ^= (hiu0 << 30)
# asm 1: hitmp ^= (hi<r2=int64#1 unsigned>> 28)
# asm 2: hitmp ^= (hi<r2=u0 unsigned>> 28)
hitmp ^= (hiu0 unsigned>> 28)
# asm 1: hitmp ^= (lo<r2=int64#1 << 4)
# asm 2: hitmp ^= (lo<r2=u0 << 4)
hitmp ^= (lou0 << 4)
# asm 1: carry? lo<r1=int64#5 += lotmp
# asm 2: carry? lo<r1=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r1=int64#5 += hitmp + carry
# asm 2: hi<r1=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r3=int64#2 ^ lo<r4=int64#4
# asm 2: lotmp = lo<r3=u1 ^ lo<r4=u3
lotmp = lou1 ^ lou3
# asm 1: lotmp &= lo<r2=int64#1
# asm 2: lotmp &= lo<r2=u0
lotmp &= lou0
# asm 1: lotmp2 = lo<r3=int64#2 & lo<r4=int64#4
# asm 2: lotmp2 = lo<r3=u1 & lo<r4=u3
lotmp2 = lou1 & lou3
lotmp ^= lotmp2
# asm 1: carry? lo<r1=int64#5 += lotmp
# asm 2: carry? lo<r1=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r3=int64#2 ^ hi<r4=int64#4
# asm 2: hitmp = hi<r3=u1 ^ hi<r4=u3
hitmp = hiu1 ^ hiu3
# asm 1: hitmp &= hi<r2=int64#1
# asm 2: hitmp &= hi<r2=u0
hitmp &= hiu0
# asm 1: hitmp2 = hi<r3=int64#2 & hi<r4=int64#4
# asm 2: hitmp2 = hi<r3=u1 & hi<r4=u3
hitmp2 = hiu1 & hiu3
hitmp ^= hitmp2
# asm 1: hi<r1=int64#5 += hitmp + carry
# asm 2: hi<r1=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r1_spill = r1
# asm 1: lo>r1_spill=spill64#2 = lo<r1=int64#5
# asm 2: lo>r1_spill=d1 = lo<r1=u4
lod1 = lou4
# asm 1: hi>r1_spill=spill64#2 = hi<r1=int64#5
# asm 2: hi>r1_spill=d1 = hi<r1=u4
hid1 = hiu4
# qhasm:       r6 = r6_spill
# asm 1: lo>r6=int64#1 = lo<r6_spill=spill64#7
# asm 2: lo>r6=u0 = lo<r6_spill=d6
lou0 = lod6
# asm 1: hi>r6=int64#1 = hi<r6_spill=spill64#7
# asm 2: hi>r6=u0 = hi<r6_spill=d6
hiu0 = hid6
# qhasm:       r7 = r7_spill
# asm 1: lo>r7=int64#2 = lo<r7_spill=spill64#8
# asm 2: lo>r7=u1 = lo<r7_spill=d7
lou1 = lod7
# asm 1: hi>r7=int64#2 = hi<r7_spill=spill64#8
# asm 2: hi>r7=u1 = hi<r7_spill=d7
hiu1 = hid7
# qhasm:       r0 = r0_spill
# asm 1: lo>r0=int64#5 = lo<r0_spill=spill64#1
# asm 2: lo>r0=u4 = lo<r0_spill=d0
lou4 = lod0
# asm 1: hi>r0=int64#5 = hi<r0_spill=spill64#1
# asm 2: hi>r0=u4 = hi<r0_spill=d0
hiu4 = hid0
# qhasm:       w7 = w7_spill
# asm 1: lo>w7=int64#6 = lo<w7_spill=spill64#16
# asm 2: lo>w7=u5 = lo<w7_spill=d15
lou5 = lod15
# asm 1: hi>w7=int64#6 = hi<w7_spill=spill64#16
# asm 2: hi>w7=u5 = hi<w7_spill=d15
hiu5 = hid15
# qhasm:     Sigma1_setup
two23 = 0x800000 simple
# qhasm:     r0 += w7 + mem64[constants] + Sigma1(r5) + Ch(r5,r6,r7); constants += 8
# asm 1: carry?  lo<r0=int64#5 += lo<w7=int64#6
# asm 2: carry?  lo<r0=u4 += lo<w7=u5
carry?  lou4 += lou5
# asm 1: hi<r0=int64#5 += hi<w7=int64#6 + carry
# asm 2: hi<r0=u4 += hi<w7=u5 + carry
hiu4 += hiu5 + carry
# asm 1: lotmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: lotmp = mem32[<constants=input_0]; <constants=input_0 += 4
lotmp = mem32[input_0]; input_0 += 4
# asm 1: hitmp = mem32[<constants=int32#1]; <constants=int32#1 += 4
# asm 2: hitmp = mem32[<constants=input_0]; <constants=input_0 += 4
hitmp = mem32[input_0]; input_0 += 4
# asm 1: carry? lo<r0=int64#5 += lotmp
# asm 2: carry? lo<r0=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r0=int64#5 += hitmp + carry
# asm 2: hi<r0=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: hitmp lotmp = lo<r5=int64#3 * two23
# asm 2: hitmp lotmp = lo<r5=u2 * two23
hitmp lotmp = lou2 * two23
# asm 1: lotmp hitmp += hi<r5=int64#3 * two23
# asm 2: lotmp hitmp += hi<r5=u2 * two23
lotmp hitmp += hiu2 * two23
# asm 1: lotmp ^= (lo<r5=int64#3 unsigned>> 18)
# asm 2: lotmp ^= (lo<r5=u2 unsigned>> 18)
lotmp ^= (lou2 unsigned>> 18)
# asm 1: lotmp ^= (hi<r5=int64#3 << 14)
# asm 2: lotmp ^= (hi<r5=u2 << 14)
lotmp ^= (hiu2 << 14)
# asm 1: lotmp ^= (lo<r5=int64#3 unsigned>> 14)
# asm 2: lotmp ^= (lo<r5=u2 unsigned>> 14)
lotmp ^= (lou2 unsigned>> 14)
# asm 1: lotmp ^= (hi<r5=int64#3 << 18)
# asm 2: lotmp ^= (hi<r5=u2 << 18)
lotmp ^= (hiu2 << 18)
# asm 1: hitmp ^= (hi<r5=int64#3 unsigned>> 18)
# asm 2: hitmp ^= (hi<r5=u2 unsigned>> 18)
hitmp ^= (hiu2 unsigned>> 18)
# asm 1: hitmp ^= (lo<r5=int64#3 << 14)
# asm 2: hitmp ^= (lo<r5=u2 << 14)
hitmp ^= (lou2 << 14)
# asm 1: hitmp ^= (hi<r5=int64#3 unsigned>> 14)
# asm 2: hitmp ^= (hi<r5=u2 unsigned>> 14)
hitmp ^= (hiu2 unsigned>> 14)
# asm 1: hitmp ^= (lo<r5=int64#3 << 18)
# asm 2: hitmp ^= (lo<r5=u2 << 18)
hitmp ^= (lou2 << 18)
# asm 1: carry? lo<r0=int64#5 += lotmp
# asm 2: carry? lo<r0=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r0=int64#5 += hitmp + carry
# asm 2: hi<r0=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r5=int64#3 & lo<r6=int64#1
# asm 2: lotmp = lo<r5=u2 & lo<r6=u0
lotmp = lou2 & lou0
# asm 1: lotmp2 = lo<r7=int64#2 & ~lo<r5=int64#3
# asm 2: lotmp2 = lo<r7=u1 & ~lo<r5=u2
lotmp2 = lou1 & ~lou2
lotmp ^= lotmp2
# asm 1: carry? lo<r0=int64#5 += lotmp
# asm 2: carry? lo<r0=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r5=int64#3 & hi<r6=int64#1
# asm 2: hitmp = hi<r5=u2 & hi<r6=u0
hitmp = hiu2 & hiu0
# asm 1: hitmp2 = hi<r7=int64#2 & ~hi<r5=int64#3
# asm 2: hitmp2 = hi<r7=u1 & ~hi<r5=u2
hitmp2 = hiu1 & ~hiu2
hitmp ^= hitmp2
# asm 1: hi<r0=int64#5 += hitmp + carry
# asm 2: hi<r0=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:     r4 += r0
# asm 1: carry? lo<r4=int64#4 += lo<r0=int64#5
# asm 2: carry? lo<r4=u3 += lo<r0=u4
carry? lou3 += lou4
# asm 1: hi<r4=int64#4 += hi<r0=int64#5 + carry
# asm 2: hi<r4=u3 += hi<r0=u4 + carry
hiu3 += hiu4 + carry
# qhasm:       r4_spill = r4
# asm 1: lo>r4_spill=spill64#5 = lo<r4=int64#4
# asm 2: lo>r4_spill=d4 = lo<r4=u3
lod4 = lou3
# asm 1: hi>r4_spill=spill64#5 = hi<r4=int64#4
# asm 2: hi>r4_spill=d4 = hi<r4=u3
hid4 = hiu3
# qhasm:       r1 = r1_spill
# asm 1: lo>r1=int64#1 = lo<r1_spill=spill64#2
# asm 2: lo>r1=u0 = lo<r1_spill=d1
lou0 = lod1
# asm 1: hi>r1=int64#1 = hi<r1_spill=spill64#2
# asm 2: hi>r1=u0 = hi<r1_spill=d1
hiu0 = hid1
# qhasm:       r2 = r2_spill
# asm 1: lo>r2=int64#2 = lo<r2_spill=spill64#3
# asm 2: lo>r2=u1 = lo<r2_spill=d2
lou1 = lod2
# asm 1: hi>r2=int64#2 = hi<r2_spill=spill64#3
# asm 2: hi>r2=u1 = hi<r2_spill=d2
hiu1 = hid2
# qhasm:       r3 = r3_spill
# asm 1: lo>r3=int64#3 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u2 = lo<r3_spill=d3
lou2 = lod3
# asm 1: hi>r3=int64#3 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u2 = hi<r3_spill=d3
hiu2 = hid3
# qhasm:     Sigma0_setup
two25 = 0x2000000 simple
# qhasm:     r0 += Sigma0(r1) + Maj(r1,r2,r3)
# asm 1: hitmp lotmp = lo<r1=int64#1 * two25
# asm 2: hitmp lotmp = lo<r1=u0 * two25
hitmp lotmp = lou0 * two25
# asm 1: lotmp hitmp += hi<r1=int64#1 * two25
# asm 2: lotmp hitmp += hi<r1=u0 * two25
lotmp hitmp += hiu0 * two25
# asm 1: lotmp ^= (hi<r1=int64#1 unsigned>> 2)
# asm 2: lotmp ^= (hi<r1=u0 unsigned>> 2)
lotmp ^= (hiu0 unsigned>> 2)
# asm 1: lotmp ^= (lo<r1=int64#1 << 30)
# asm 2: lotmp ^= (lo<r1=u0 << 30)
lotmp ^= (lou0 << 30)
# asm 1: lotmp ^= (lo<r1=int64#1 unsigned>> 28)
# asm 2: lotmp ^= (lo<r1=u0 unsigned>> 28)
lotmp ^= (lou0 unsigned>> 28)
# asm 1: lotmp ^= (hi<r1=int64#1 << 4)
# asm 2: lotmp ^= (hi<r1=u0 << 4)
lotmp ^= (hiu0 << 4)
# asm 1: hitmp ^= (lo<r1=int64#1 unsigned>> 2)
# asm 2: hitmp ^= (lo<r1=u0 unsigned>> 2)
hitmp ^= (lou0 unsigned>> 2)
# asm 1: hitmp ^= (hi<r1=int64#1 << 30)
# asm 2: hitmp ^= (hi<r1=u0 << 30)
hitmp ^= (hiu0 << 30)
# asm 1: hitmp ^= (hi<r1=int64#1 unsigned>> 28)
# asm 2: hitmp ^= (hi<r1=u0 unsigned>> 28)
hitmp ^= (hiu0 unsigned>> 28)
# asm 1: hitmp ^= (lo<r1=int64#1 << 4)
# asm 2: hitmp ^= (lo<r1=u0 << 4)
hitmp ^= (lou0 << 4)
# asm 1: carry? lo<r0=int64#5 += lotmp
# asm 2: carry? lo<r0=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<r0=int64#5 += hitmp + carry
# asm 2: hi<r0=u4 += hitmp + carry
hiu4 += hitmp + carry
# asm 1: lotmp = lo<r2=int64#2 ^ lo<r3=int64#3
# asm 2: lotmp = lo<r2=u1 ^ lo<r3=u2
lotmp = lou1 ^ lou2
# asm 1: lotmp &= lo<r1=int64#1
# asm 2: lotmp &= lo<r1=u0
lotmp &= lou0
# asm 1: lotmp2 = lo<r2=int64#2 & lo<r3=int64#3
# asm 2: lotmp2 = lo<r2=u1 & lo<r3=u2
lotmp2 = lou1 & lou2
lotmp ^= lotmp2
# asm 1: carry? lo<r0=int64#5 += lotmp
# asm 2: carry? lo<r0=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hitmp = hi<r2=int64#2 ^ hi<r3=int64#3
# asm 2: hitmp = hi<r2=u1 ^ hi<r3=u2
hitmp = hiu1 ^ hiu2
# asm 1: hitmp &= hi<r1=int64#1
# asm 2: hitmp &= hi<r1=u0
hitmp &= hiu0
# asm 1: hitmp2 = hi<r2=int64#2 & hi<r3=int64#3
# asm 2: hitmp2 = hi<r2=u1 & hi<r3=u2
hitmp2 = hiu1 & hiu2
hitmp ^= hitmp2
# asm 1: hi<r0=int64#5 += hitmp + carry
# asm 2: hi<r0=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       r0_spill = r0
# asm 1: lo>r0_spill=spill64#1 = lo<r0=int64#5
# asm 2: lo>r0_spill=d0 = lo<r0=u4
lod0 = lou4
# asm 1: hi>r0_spill=spill64#1 = hi<r0=int64#5
# asm 2: hi>r0_spill=d0 = hi<r0=u4
hid0 = hiu4
# qhasm:     constants_stack = constants
# asm 1: >constants_stack=stack32#4 = <constants=int32#1
# asm 2: >constants_stack=o3 = <constants=input_0
o3 = input_0
# qhasm:     assign 8 to w0_spill
# asm 1: assign 16 to lo<w0_spill=spill64#9
# asm 2: assign 16 to lo<w0_spill=d8
assign 16 to lod8
# asm 1: assign 17 to hi<w0_spill=spill64#9
# asm 2: assign 17 to hi<w0_spill=d8
assign 17 to hid8
# qhasm:     assign 9 to w1_spill
# asm 1: assign 18 to lo<w1_spill=spill64#10
# asm 2: assign 18 to lo<w1_spill=d9
assign 18 to lod9
# asm 1: assign 19 to hi<w1_spill=spill64#10
# asm 2: assign 19 to hi<w1_spill=d9
assign 19 to hid9
# qhasm:     assign 10 to w2_spill
# asm 1: assign 20 to lo<w2_spill=spill64#11
# asm 2: assign 20 to lo<w2_spill=d10
assign 20 to lod10
# asm 1: assign 21 to hi<w2_spill=spill64#11
# asm 2: assign 21 to hi<w2_spill=d10
assign 21 to hid10
# qhasm:     assign 11 to w3_spill
# asm 1: assign 22 to lo<w3_spill=spill64#12
# asm 2: assign 22 to lo<w3_spill=d11
assign 22 to lod11
# asm 1: assign 23 to hi<w3_spill=spill64#12
# asm 2: assign 23 to hi<w3_spill=d11
assign 23 to hid11
# qhasm:     assign 12 to w4_spill
# asm 1: assign 24 to lo<w4_spill=spill64#13
# asm 2: assign 24 to lo<w4_spill=d12
assign 24 to lod12
# asm 1: assign 25 to hi<w4_spill=spill64#13
# asm 2: assign 25 to hi<w4_spill=d12
assign 25 to hid12
# qhasm:     assign 13 to w5_spill
# asm 1: assign 26 to lo<w5_spill=spill64#14
# asm 2: assign 26 to lo<w5_spill=d13
assign 26 to lod13
# asm 1: assign 27 to hi<w5_spill=spill64#14
# asm 2: assign 27 to hi<w5_spill=d13
assign 27 to hid13
# qhasm:     assign 14 to w6_spill
# asm 1: assign 28 to lo<w6_spill=spill64#15
# asm 2: assign 28 to lo<w6_spill=d14
assign 28 to lod14
# asm 1: assign 29 to hi<w6_spill=spill64#15
# asm 2: assign 29 to hi<w6_spill=d14
assign 29 to hid14
# qhasm:     assign 15 to w7_spill
# asm 1: assign 30 to lo<w7_spill=spill64#16
# asm 2: assign 30 to lo<w7_spill=d15
assign 30 to lod15
# asm 1: assign 31 to hi<w7_spill=spill64#16
# asm 2: assign 31 to hi<w7_spill=d15
assign 31 to hid15
# qhasm:     i = i_stack
# asm 1: >i=int32#1 = <i_stack=stack32#5
# asm 2: >i=input_0 = <i_stack=o4
input_0 = o4
# qhasm:                          =? i -= 8
# asm 1: =? unsigned<? <i=int32#1 -= 8
# asm 2: =? unsigned<? <i=input_0 -= 8
=? unsigned<? input_0 -= 8
# qhasm:     goto endinnerloop if =
goto endinnerloop if =
# qhasm:     i_stack = i
# asm 1: >i_stack=stack32#5 = <i=int32#1
# asm 2: >i_stack=o4 = <i=input_0
o4 = input_0
# qhasm:                     =? i - 8
# asm 1: =? <i=int32#1 - 8
# asm 2: =? <i=input_0 - 8
=? input_0 - 8
# qhasm:     goto nearend if =
goto nearend if =
# qhasm:       sigma1_setup
two24 = 0x1000000 simple
# qhasm:       sigma0_setup
two13 = 0x2000 simple
# qhasm:       w8 = w0_spill
# asm 1: lo>w8=int64#1 = lo<w0_spill=spill64#9
# asm 2: lo>w8=u0 = lo<w0_spill=d8
lou0 = lod8
# asm 1: hi>w8=int64#1 = hi<w0_spill=spill64#9
# asm 2: hi>w8=u0 = hi<w0_spill=d8
hiu0 = hid8
# qhasm:       w9 = w1_spill
# asm 1: lo>w9=int64#2 = lo<w1_spill=spill64#10
# asm 2: lo>w9=u1 = lo<w1_spill=d9
lou1 = lod9
# asm 1: hi>w9=int64#2 = hi<w1_spill=spill64#10
# asm 2: hi>w9=u1 = hi<w1_spill=d9
hiu1 = hid9
# qhasm:       w6 = w6_next
# asm 1: lo>w6=int64#3 = lo<w6_next=stack64#15
# asm 2: lo>w6=u2 = lo<w6_next=m14
lou2 = lom14
# asm 1: hi>w6=int64#3 = hi<w6_next=stack64#15
# asm 2: hi>w6=u2 = hi<w6_next=m14
hiu2 = him14
# qhasm:       w1 = w1_next
# asm 1: lo>w1=int64#4 = lo<w1_next=stack64#10
# asm 2: lo>w1=u3 = lo<w1_next=m9
lou3 = lom9
# asm 1: hi>w1=int64#4 = hi<w1_next=stack64#10
# asm 2: hi>w1=u3 = hi<w1_next=m9
hiu3 = him9
# qhasm:       w8  += sigma1(w6)
# asm 1: hitmp lotmp = hi<w6=int64#3 * two13
# asm 2: hitmp lotmp = hi<w6=u2 * two13
hitmp lotmp = hiu2 * two13
# asm 1: lotmp hitmp += lo<w6=int64#3 * two13
# asm 2: lotmp hitmp += lo<w6=u2 * two13
lotmp hitmp += lou2 * two13
# asm 1: lotmp ^= (lo<w6=int64#3 unsigned>> 6)
# asm 2: lotmp ^= (lo<w6=u2 unsigned>> 6)
lotmp ^= (lou2 unsigned>> 6)
# asm 1: lotmp ^= (hi<w6=int64#3 << 26)
# asm 2: lotmp ^= (hi<w6=u2 << 26)
lotmp ^= (hiu2 << 26)
# asm 1: lotmp ^= (hi<w6=int64#3 unsigned>> 29)
# asm 2: lotmp ^= (hi<w6=u2 unsigned>> 29)
lotmp ^= (hiu2 unsigned>> 29)
# asm 1: lotmp ^= (lo<w6=int64#3 << 3)
# asm 2: lotmp ^= (lo<w6=u2 << 3)
lotmp ^= (lou2 << 3)
# asm 1: hitmp ^= (hi<w6=int64#3 unsigned>> 6)
# asm 2: hitmp ^= (hi<w6=u2 unsigned>> 6)
hitmp ^= (hiu2 unsigned>> 6)
# asm 1: hitmp ^= (lo<w6=int64#3 unsigned>> 29)
# asm 2: hitmp ^= (lo<w6=u2 unsigned>> 29)
hitmp ^= (lou2 unsigned>> 29)
# asm 1: hitmp ^= (hi<w6=int64#3 << 3)
# asm 2: hitmp ^= (hi<w6=u2 << 3)
hitmp ^= (hiu2 << 3)
# asm 1: carry? lo<w8=int64#1 += lotmp
# asm 2: carry? lo<w8=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<w8=int64#1 += hitmp + carry
# asm 2: hi<w8=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:       w8  += sigma0(w9)
# asm 1: hitmp lotmp = hi<w9=int64#2 * two24
# asm 2: hitmp lotmp = hi<w9=u1 * two24
hitmp lotmp = hiu1 * two24
# asm 1: lotmp hitmp += lo<w9=int64#2 * two24
# asm 2: lotmp hitmp += lo<w9=u1 * two24
lotmp hitmp += lou1 * two24
# asm 1: carry? lotmp ^= (lo<w9=int64#2 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w9=u1 unsigned>> 1)
carry? lotmp ^= (lou1 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w9=int64#2 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w9=u1 unsigned>> 1)
hitmp ^= (carry,hiu1 unsigned>> 1)
# asm 1: lotmp ^= (hi<w9=int64#2 << 31)
# asm 2: lotmp ^= (hi<w9=u1 << 31)
lotmp ^= (hiu1 << 31)
# asm 1: lotmp ^= (lo<w9=int64#2 unsigned>>7)
# asm 2: lotmp ^= (lo<w9=u1 unsigned>>7)
lotmp ^= (lou1 unsigned>>7)
# asm 1: lotmp ^= (hi<w9=int64#2 << 25)
# asm 2: lotmp ^= (hi<w9=u1 << 25)
lotmp ^= (hiu1 << 25)
# asm 1: hitmp ^= (hi<w9=int64#2 unsigned>>7)
# asm 2: hitmp ^= (hi<w9=u1 unsigned>>7)
hitmp ^= (hiu1 unsigned>>7)
# asm 1: carry? lo<w8=int64#1 += lotmp
# asm 2: carry? lo<w8=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<w8=int64#1 += hitmp + carry
# asm 2: hi<w8=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:       w8  += w1
# asm 1: carry? lo<w8=int64#1 += lo<w1=int64#4
# asm 2: carry? lo<w8=u0 += lo<w1=u3
carry? lou0 += lou3
# asm 1: hi<w8=int64#1 += hi<w1=int64#4 + carry
# asm 2: hi<w8=u0 += hi<w1=u3 + carry
hiu0 += hiu3 + carry
# qhasm:         w1_spill = w1
# asm 1: lo>w1_spill=spill64#10 = lo<w1=int64#4
# asm 2: lo>w1_spill=d9 = lo<w1=u3
lod9 = lou3
# asm 1: hi>w1_spill=spill64#10 = hi<w1=int64#4
# asm 2: hi>w1_spill=d9 = hi<w1=u3
hid9 = hiu3
# qhasm:       w7 = w7_next
# asm 1: lo>w7=int64#4 = lo<w7_next=stack64#16
# asm 2: lo>w7=u3 = lo<w7_next=m15
lou3 = lom15
# asm 1: hi>w7=int64#4 = hi<w7_next=stack64#16
# asm 2: hi>w7=u3 = hi<w7_next=m15
hiu3 = him15
# qhasm:         w8_stack = w8
# asm 1: lo>w8_stack=stack64#16 = lo<w8=int64#1
# asm 2: lo>w8_stack=m15 = lo<w8=u0
lom15 = lou0
# asm 1: hi>w8_stack=stack64#16 = hi<w8=int64#1
# asm 2: hi>w8_stack=m15 = hi<w8=u0
him15 = hiu0
# qhasm:       w9  += sigma1(w7)
# asm 1: hitmp lotmp = hi<w7=int64#4 * two13
# asm 2: hitmp lotmp = hi<w7=u3 * two13
hitmp lotmp = hiu3 * two13
# asm 1: lotmp hitmp += lo<w7=int64#4 * two13
# asm 2: lotmp hitmp += lo<w7=u3 * two13
lotmp hitmp += lou3 * two13
# asm 1: lotmp ^= (lo<w7=int64#4 unsigned>> 6)
# asm 2: lotmp ^= (lo<w7=u3 unsigned>> 6)
lotmp ^= (lou3 unsigned>> 6)
# asm 1: lotmp ^= (hi<w7=int64#4 << 26)
# asm 2: lotmp ^= (hi<w7=u3 << 26)
lotmp ^= (hiu3 << 26)
# asm 1: lotmp ^= (hi<w7=int64#4 unsigned>> 29)
# asm 2: lotmp ^= (hi<w7=u3 unsigned>> 29)
lotmp ^= (hiu3 unsigned>> 29)
# asm 1: lotmp ^= (lo<w7=int64#4 << 3)
# asm 2: lotmp ^= (lo<w7=u3 << 3)
lotmp ^= (lou3 << 3)
# asm 1: hitmp ^= (hi<w7=int64#4 unsigned>> 6)
# asm 2: hitmp ^= (hi<w7=u3 unsigned>> 6)
hitmp ^= (hiu3 unsigned>> 6)
# asm 1: hitmp ^= (lo<w7=int64#4 unsigned>> 29)
# asm 2: hitmp ^= (lo<w7=u3 unsigned>> 29)
hitmp ^= (lou3 unsigned>> 29)
# asm 1: hitmp ^= (hi<w7=int64#4 << 3)
# asm 2: hitmp ^= (hi<w7=u3 << 3)
hitmp ^= (hiu3 << 3)
# asm 1: carry? lo<w9=int64#2 += lotmp
# asm 2: carry? lo<w9=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<w9=int64#2 += hitmp + carry
# asm 2: hi<w9=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:       w10 = w2_spill
# asm 1: lo>w10=int64#5 = lo<w2_spill=spill64#11
# asm 2: lo>w10=u4 = lo<w2_spill=d10
lou4 = lod10
# asm 1: hi>w10=int64#5 = hi<w2_spill=spill64#11
# asm 2: hi>w10=u4 = hi<w2_spill=d10
hiu4 = hid10
# qhasm:       w9  += sigma0(w10)
# asm 1: hitmp lotmp = hi<w10=int64#5 * two24
# asm 2: hitmp lotmp = hi<w10=u4 * two24
hitmp lotmp = hiu4 * two24
# asm 1: lotmp hitmp += lo<w10=int64#5 * two24
# asm 2: lotmp hitmp += lo<w10=u4 * two24
lotmp hitmp += lou4 * two24
# asm 1: carry? lotmp ^= (lo<w10=int64#5 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w10=u4 unsigned>> 1)
carry? lotmp ^= (lou4 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w10=int64#5 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w10=u4 unsigned>> 1)
hitmp ^= (carry,hiu4 unsigned>> 1)
# asm 1: lotmp ^= (hi<w10=int64#5 << 31)
# asm 2: lotmp ^= (hi<w10=u4 << 31)
lotmp ^= (hiu4 << 31)
# asm 1: lotmp ^= (lo<w10=int64#5 unsigned>>7)
# asm 2: lotmp ^= (lo<w10=u4 unsigned>>7)
lotmp ^= (lou4 unsigned>>7)
# asm 1: lotmp ^= (hi<w10=int64#5 << 25)
# asm 2: lotmp ^= (hi<w10=u4 << 25)
lotmp ^= (hiu4 << 25)
# asm 1: hitmp ^= (hi<w10=int64#5 unsigned>>7)
# asm 2: hitmp ^= (hi<w10=u4 unsigned>>7)
hitmp ^= (hiu4 unsigned>>7)
# asm 1: carry? lo<w9=int64#2 += lotmp
# asm 2: carry? lo<w9=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<w9=int64#2 += hitmp + carry
# asm 2: hi<w9=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:       w2 = w2_next
# asm 1: lo>w2=int64#6 = lo<w2_next=stack64#11
# asm 2: lo>w2=u5 = lo<w2_next=m10
lou5 = lom10
# asm 1: hi>w2=int64#6 = hi<w2_next=stack64#11
# asm 2: hi>w2=u5 = hi<w2_next=m10
hiu5 = him10
# qhasm:       w9  += w2
# asm 1: carry? lo<w9=int64#2 += lo<w2=int64#6
# asm 2: carry? lo<w9=u1 += lo<w2=u5
carry? lou1 += lou5
# asm 1: hi<w9=int64#2 += hi<w2=int64#6 + carry
# asm 2: hi<w9=u1 += hi<w2=u5 + carry
hiu1 += hiu5 + carry
# qhasm:         w2_spill = w2
# asm 1: lo>w2_spill=spill64#11 = lo<w2=int64#6
# asm 2: lo>w2_spill=d10 = lo<w2=u5
lod10 = lou5
# asm 1: hi>w2_spill=spill64#11 = hi<w2=int64#6
# asm 2: hi>w2_spill=d10 = hi<w2=u5
hid10 = hiu5
# qhasm:         w1_next = w9
# asm 1: lo>w1_next=stack64#10 = lo<w9=int64#2
# asm 2: lo>w1_next=m9 = lo<w9=u1
lom9 = lou1
# asm 1: hi>w1_next=stack64#10 = hi<w9=int64#2
# asm 2: hi>w1_next=m9 = hi<w9=u1
him9 = hiu1
# qhasm:       w10 += sigma1(w8)
# asm 1: hitmp lotmp = hi<w8=int64#1 * two13
# asm 2: hitmp lotmp = hi<w8=u0 * two13
hitmp lotmp = hiu0 * two13
# asm 1: lotmp hitmp += lo<w8=int64#1 * two13
# asm 2: lotmp hitmp += lo<w8=u0 * two13
lotmp hitmp += lou0 * two13
# asm 1: lotmp ^= (lo<w8=int64#1 unsigned>> 6)
# asm 2: lotmp ^= (lo<w8=u0 unsigned>> 6)
lotmp ^= (lou0 unsigned>> 6)
# asm 1: lotmp ^= (hi<w8=int64#1 << 26)
# asm 2: lotmp ^= (hi<w8=u0 << 26)
lotmp ^= (hiu0 << 26)
# asm 1: lotmp ^= (hi<w8=int64#1 unsigned>> 29)
# asm 2: lotmp ^= (hi<w8=u0 unsigned>> 29)
lotmp ^= (hiu0 unsigned>> 29)
# asm 1: lotmp ^= (lo<w8=int64#1 << 3)
# asm 2: lotmp ^= (lo<w8=u0 << 3)
lotmp ^= (lou0 << 3)
# asm 1: hitmp ^= (hi<w8=int64#1 unsigned>> 6)
# asm 2: hitmp ^= (hi<w8=u0 unsigned>> 6)
hitmp ^= (hiu0 unsigned>> 6)
# asm 1: hitmp ^= (lo<w8=int64#1 unsigned>> 29)
# asm 2: hitmp ^= (lo<w8=u0 unsigned>> 29)
hitmp ^= (lou0 unsigned>> 29)
# asm 1: hitmp ^= (hi<w8=int64#1 << 3)
# asm 2: hitmp ^= (hi<w8=u0 << 3)
hitmp ^= (hiu0 << 3)
# asm 1: carry? lo<w10=int64#5 += lotmp
# asm 2: carry? lo<w10=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<w10=int64#5 += hitmp + carry
# asm 2: hi<w10=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       w11 = w3_spill
# asm 1: lo>w11=int64#1 = lo<w3_spill=spill64#12
# asm 2: lo>w11=u0 = lo<w3_spill=d11
lou0 = lod11
# asm 1: hi>w11=int64#1 = hi<w3_spill=spill64#12
# asm 2: hi>w11=u0 = hi<w3_spill=d11
hiu0 = hid11
# qhasm:       w10 += sigma0(w11)
# asm 1: hitmp lotmp = hi<w11=int64#1 * two24
# asm 2: hitmp lotmp = hi<w11=u0 * two24
hitmp lotmp = hiu0 * two24
# asm 1: lotmp hitmp += lo<w11=int64#1 * two24
# asm 2: lotmp hitmp += lo<w11=u0 * two24
lotmp hitmp += lou0 * two24
# asm 1: carry? lotmp ^= (lo<w11=int64#1 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w11=u0 unsigned>> 1)
carry? lotmp ^= (lou0 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w11=int64#1 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w11=u0 unsigned>> 1)
hitmp ^= (carry,hiu0 unsigned>> 1)
# asm 1: lotmp ^= (hi<w11=int64#1 << 31)
# asm 2: lotmp ^= (hi<w11=u0 << 31)
lotmp ^= (hiu0 << 31)
# asm 1: lotmp ^= (lo<w11=int64#1 unsigned>>7)
# asm 2: lotmp ^= (lo<w11=u0 unsigned>>7)
lotmp ^= (lou0 unsigned>>7)
# asm 1: lotmp ^= (hi<w11=int64#1 << 25)
# asm 2: lotmp ^= (hi<w11=u0 << 25)
lotmp ^= (hiu0 << 25)
# asm 1: hitmp ^= (hi<w11=int64#1 unsigned>>7)
# asm 2: hitmp ^= (hi<w11=u0 unsigned>>7)
hitmp ^= (hiu0 unsigned>>7)
# asm 1: carry? lo<w10=int64#5 += lotmp
# asm 2: carry? lo<w10=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<w10=int64#5 += hitmp + carry
# asm 2: hi<w10=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       w3 = w3_next
# asm 1: lo>w3=int64#6 = lo<w3_next=stack64#12
# asm 2: lo>w3=u5 = lo<w3_next=m11
lou5 = lom11
# asm 1: hi>w3=int64#6 = hi<w3_next=stack64#12
# asm 2: hi>w3=u5 = hi<w3_next=m11
hiu5 = him11
# qhasm:       w10 += w3
# asm 1: carry? lo<w10=int64#5 += lo<w3=int64#6
# asm 2: carry? lo<w10=u4 += lo<w3=u5
carry? lou4 += lou5
# asm 1: hi<w10=int64#5 += hi<w3=int64#6 + carry
# asm 2: hi<w10=u4 += hi<w3=u5 + carry
hiu4 += hiu5 + carry
# qhasm:         w3_spill = w3
# asm 1: lo>w3_spill=spill64#12 = lo<w3=int64#6
# asm 2: lo>w3_spill=d11 = lo<w3=u5
lod11 = lou5
# asm 1: hi>w3_spill=spill64#12 = hi<w3=int64#6
# asm 2: hi>w3_spill=d11 = hi<w3=u5
hid11 = hiu5
# qhasm:         w2_next = w10
# asm 1: lo>w2_next=stack64#11 = lo<w10=int64#5
# asm 2: lo>w2_next=m10 = lo<w10=u4
lom10 = lou4
# asm 1: hi>w2_next=stack64#11 = hi<w10=int64#5
# asm 2: hi>w2_next=m10 = hi<w10=u4
him10 = hiu4
# qhasm:       w11 += sigma1(w9)
# asm 1: hitmp lotmp = hi<w9=int64#2 * two13
# asm 2: hitmp lotmp = hi<w9=u1 * two13
hitmp lotmp = hiu1 * two13
# asm 1: lotmp hitmp += lo<w9=int64#2 * two13
# asm 2: lotmp hitmp += lo<w9=u1 * two13
lotmp hitmp += lou1 * two13
# asm 1: lotmp ^= (lo<w9=int64#2 unsigned>> 6)
# asm 2: lotmp ^= (lo<w9=u1 unsigned>> 6)
lotmp ^= (lou1 unsigned>> 6)
# asm 1: lotmp ^= (hi<w9=int64#2 << 26)
# asm 2: lotmp ^= (hi<w9=u1 << 26)
lotmp ^= (hiu1 << 26)
# asm 1: lotmp ^= (hi<w9=int64#2 unsigned>> 29)
# asm 2: lotmp ^= (hi<w9=u1 unsigned>> 29)
lotmp ^= (hiu1 unsigned>> 29)
# asm 1: lotmp ^= (lo<w9=int64#2 << 3)
# asm 2: lotmp ^= (lo<w9=u1 << 3)
lotmp ^= (lou1 << 3)
# asm 1: hitmp ^= (hi<w9=int64#2 unsigned>> 6)
# asm 2: hitmp ^= (hi<w9=u1 unsigned>> 6)
hitmp ^= (hiu1 unsigned>> 6)
# asm 1: hitmp ^= (lo<w9=int64#2 unsigned>> 29)
# asm 2: hitmp ^= (lo<w9=u1 unsigned>> 29)
hitmp ^= (lou1 unsigned>> 29)
# asm 1: hitmp ^= (hi<w9=int64#2 << 3)
# asm 2: hitmp ^= (hi<w9=u1 << 3)
hitmp ^= (hiu1 << 3)
# asm 1: carry? lo<w11=int64#1 += lotmp
# asm 2: carry? lo<w11=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<w11=int64#1 += hitmp + carry
# asm 2: hi<w11=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:       w12 = w4_spill
# asm 1: lo>w12=int64#2 = lo<w4_spill=spill64#13
# asm 2: lo>w12=u1 = lo<w4_spill=d12
lou1 = lod12
# asm 1: hi>w12=int64#2 = hi<w4_spill=spill64#13
# asm 2: hi>w12=u1 = hi<w4_spill=d12
hiu1 = hid12
# qhasm:       w11 += sigma0(w12)
# asm 1: hitmp lotmp = hi<w12=int64#2 * two24
# asm 2: hitmp lotmp = hi<w12=u1 * two24
hitmp lotmp = hiu1 * two24
# asm 1: lotmp hitmp += lo<w12=int64#2 * two24
# asm 2: lotmp hitmp += lo<w12=u1 * two24
lotmp hitmp += lou1 * two24
# asm 1: carry? lotmp ^= (lo<w12=int64#2 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w12=u1 unsigned>> 1)
carry? lotmp ^= (lou1 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w12=int64#2 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w12=u1 unsigned>> 1)
hitmp ^= (carry,hiu1 unsigned>> 1)
# asm 1: lotmp ^= (hi<w12=int64#2 << 31)
# asm 2: lotmp ^= (hi<w12=u1 << 31)
lotmp ^= (hiu1 << 31)
# asm 1: lotmp ^= (lo<w12=int64#2 unsigned>>7)
# asm 2: lotmp ^= (lo<w12=u1 unsigned>>7)
lotmp ^= (lou1 unsigned>>7)
# asm 1: lotmp ^= (hi<w12=int64#2 << 25)
# asm 2: lotmp ^= (hi<w12=u1 << 25)
lotmp ^= (hiu1 << 25)
# asm 1: hitmp ^= (hi<w12=int64#2 unsigned>>7)
# asm 2: hitmp ^= (hi<w12=u1 unsigned>>7)
hitmp ^= (hiu1 unsigned>>7)
# asm 1: carry? lo<w11=int64#1 += lotmp
# asm 2: carry? lo<w11=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<w11=int64#1 += hitmp + carry
# asm 2: hi<w11=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:       w4 = w4_next
# asm 1: lo>w4=int64#6 = lo<w4_next=stack64#13
# asm 2: lo>w4=u5 = lo<w4_next=m12
lou5 = lom12
# asm 1: hi>w4=int64#6 = hi<w4_next=stack64#13
# asm 2: hi>w4=u5 = hi<w4_next=m12
hiu5 = him12
# qhasm:       w11 += w4
# asm 1: carry? lo<w11=int64#1 += lo<w4=int64#6
# asm 2: carry? lo<w11=u0 += lo<w4=u5
carry? lou0 += lou5
# asm 1: hi<w11=int64#1 += hi<w4=int64#6 + carry
# asm 2: hi<w11=u0 += hi<w4=u5 + carry
hiu0 += hiu5 + carry
# qhasm:         w4_spill = w4
# asm 1: lo>w4_spill=spill64#13 = lo<w4=int64#6
# asm 2: lo>w4_spill=d12 = lo<w4=u5
lod12 = lou5
# asm 1: hi>w4_spill=spill64#13 = hi<w4=int64#6
# asm 2: hi>w4_spill=d12 = hi<w4=u5
hid12 = hiu5
# qhasm:         w3_next = w11
# asm 1: lo>w3_next=stack64#12 = lo<w11=int64#1
# asm 2: lo>w3_next=m11 = lo<w11=u0
lom11 = lou0
# asm 1: hi>w3_next=stack64#12 = hi<w11=int64#1
# asm 2: hi>w3_next=m11 = hi<w11=u0
him11 = hiu0
# qhasm:       w12 += sigma1(w10)
# asm 1: hitmp lotmp = hi<w10=int64#5 * two13
# asm 2: hitmp lotmp = hi<w10=u4 * two13
hitmp lotmp = hiu4 * two13
# asm 1: lotmp hitmp += lo<w10=int64#5 * two13
# asm 2: lotmp hitmp += lo<w10=u4 * two13
lotmp hitmp += lou4 * two13
# asm 1: lotmp ^= (lo<w10=int64#5 unsigned>> 6)
# asm 2: lotmp ^= (lo<w10=u4 unsigned>> 6)
lotmp ^= (lou4 unsigned>> 6)
# asm 1: lotmp ^= (hi<w10=int64#5 << 26)
# asm 2: lotmp ^= (hi<w10=u4 << 26)
lotmp ^= (hiu4 << 26)
# asm 1: lotmp ^= (hi<w10=int64#5 unsigned>> 29)
# asm 2: lotmp ^= (hi<w10=u4 unsigned>> 29)
lotmp ^= (hiu4 unsigned>> 29)
# asm 1: lotmp ^= (lo<w10=int64#5 << 3)
# asm 2: lotmp ^= (lo<w10=u4 << 3)
lotmp ^= (lou4 << 3)
# asm 1: hitmp ^= (hi<w10=int64#5 unsigned>> 6)
# asm 2: hitmp ^= (hi<w10=u4 unsigned>> 6)
hitmp ^= (hiu4 unsigned>> 6)
# asm 1: hitmp ^= (lo<w10=int64#5 unsigned>> 29)
# asm 2: hitmp ^= (lo<w10=u4 unsigned>> 29)
hitmp ^= (lou4 unsigned>> 29)
# asm 1: hitmp ^= (hi<w10=int64#5 << 3)
# asm 2: hitmp ^= (hi<w10=u4 << 3)
hitmp ^= (hiu4 << 3)
# asm 1: carry? lo<w12=int64#2 += lotmp
# asm 2: carry? lo<w12=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<w12=int64#2 += hitmp + carry
# asm 2: hi<w12=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:       w13 = w5_spill
# asm 1: lo>w13=int64#5 = lo<w5_spill=spill64#14
# asm 2: lo>w13=u4 = lo<w5_spill=d13
lou4 = lod13
# asm 1: hi>w13=int64#5 = hi<w5_spill=spill64#14
# asm 2: hi>w13=u4 = hi<w5_spill=d13
hiu4 = hid13
# qhasm:       w12 += sigma0(w13)
# asm 1: hitmp lotmp = hi<w13=int64#5 * two24
# asm 2: hitmp lotmp = hi<w13=u4 * two24
hitmp lotmp = hiu4 * two24
# asm 1: lotmp hitmp += lo<w13=int64#5 * two24
# asm 2: lotmp hitmp += lo<w13=u4 * two24
lotmp hitmp += lou4 * two24
# asm 1: carry? lotmp ^= (lo<w13=int64#5 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w13=u4 unsigned>> 1)
carry? lotmp ^= (lou4 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w13=int64#5 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w13=u4 unsigned>> 1)
hitmp ^= (carry,hiu4 unsigned>> 1)
# asm 1: lotmp ^= (hi<w13=int64#5 << 31)
# asm 2: lotmp ^= (hi<w13=u4 << 31)
lotmp ^= (hiu4 << 31)
# asm 1: lotmp ^= (lo<w13=int64#5 unsigned>>7)
# asm 2: lotmp ^= (lo<w13=u4 unsigned>>7)
lotmp ^= (lou4 unsigned>>7)
# asm 1: lotmp ^= (hi<w13=int64#5 << 25)
# asm 2: lotmp ^= (hi<w13=u4 << 25)
lotmp ^= (hiu4 << 25)
# asm 1: hitmp ^= (hi<w13=int64#5 unsigned>>7)
# asm 2: hitmp ^= (hi<w13=u4 unsigned>>7)
hitmp ^= (hiu4 unsigned>>7)
# asm 1: carry? lo<w12=int64#2 += lotmp
# asm 2: carry? lo<w12=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<w12=int64#2 += hitmp + carry
# asm 2: hi<w12=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:       w5 = w5_next
# asm 1: lo>w5=int64#6 = lo<w5_next=stack64#14
# asm 2: lo>w5=u5 = lo<w5_next=m13
lou5 = lom13
# asm 1: hi>w5=int64#6 = hi<w5_next=stack64#14
# asm 2: hi>w5=u5 = hi<w5_next=m13
hiu5 = him13
# qhasm:       w12 += w5
# asm 1: carry? lo<w12=int64#2 += lo<w5=int64#6
# asm 2: carry? lo<w12=u1 += lo<w5=u5
carry? lou1 += lou5
# asm 1: hi<w12=int64#2 += hi<w5=int64#6 + carry
# asm 2: hi<w12=u1 += hi<w5=u5 + carry
hiu1 += hiu5 + carry
# qhasm:         w5_spill = w5
# asm 1: lo>w5_spill=spill64#14 = lo<w5=int64#6
# asm 2: lo>w5_spill=d13 = lo<w5=u5
lod13 = lou5
# asm 1: hi>w5_spill=spill64#14 = hi<w5=int64#6
# asm 2: hi>w5_spill=d13 = hi<w5=u5
hid13 = hiu5
# qhasm:         w4_next = w12
# asm 1: lo>w4_next=stack64#13 = lo<w12=int64#2
# asm 2: lo>w4_next=m12 = lo<w12=u1
lom12 = lou1
# asm 1: hi>w4_next=stack64#13 = hi<w12=int64#2
# asm 2: hi>w4_next=m12 = hi<w12=u1
him12 = hiu1
# qhasm:       w13 += sigma1(w11)
# asm 1: hitmp lotmp = hi<w11=int64#1 * two13
# asm 2: hitmp lotmp = hi<w11=u0 * two13
hitmp lotmp = hiu0 * two13
# asm 1: lotmp hitmp += lo<w11=int64#1 * two13
# asm 2: lotmp hitmp += lo<w11=u0 * two13
lotmp hitmp += lou0 * two13
# asm 1: lotmp ^= (lo<w11=int64#1 unsigned>> 6)
# asm 2: lotmp ^= (lo<w11=u0 unsigned>> 6)
lotmp ^= (lou0 unsigned>> 6)
# asm 1: lotmp ^= (hi<w11=int64#1 << 26)
# asm 2: lotmp ^= (hi<w11=u0 << 26)
lotmp ^= (hiu0 << 26)
# asm 1: lotmp ^= (hi<w11=int64#1 unsigned>> 29)
# asm 2: lotmp ^= (hi<w11=u0 unsigned>> 29)
lotmp ^= (hiu0 unsigned>> 29)
# asm 1: lotmp ^= (lo<w11=int64#1 << 3)
# asm 2: lotmp ^= (lo<w11=u0 << 3)
lotmp ^= (lou0 << 3)
# asm 1: hitmp ^= (hi<w11=int64#1 unsigned>> 6)
# asm 2: hitmp ^= (hi<w11=u0 unsigned>> 6)
hitmp ^= (hiu0 unsigned>> 6)
# asm 1: hitmp ^= (lo<w11=int64#1 unsigned>> 29)
# asm 2: hitmp ^= (lo<w11=u0 unsigned>> 29)
hitmp ^= (lou0 unsigned>> 29)
# asm 1: hitmp ^= (hi<w11=int64#1 << 3)
# asm 2: hitmp ^= (hi<w11=u0 << 3)
hitmp ^= (hiu0 << 3)
# asm 1: carry? lo<w13=int64#5 += lotmp
# asm 2: carry? lo<w13=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<w13=int64#5 += hitmp + carry
# asm 2: hi<w13=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       w14 = w6_spill
# asm 1: lo>w14=int64#1 = lo<w6_spill=spill64#15
# asm 2: lo>w14=u0 = lo<w6_spill=d14
lou0 = lod14
# asm 1: hi>w14=int64#1 = hi<w6_spill=spill64#15
# asm 2: hi>w14=u0 = hi<w6_spill=d14
hiu0 = hid14
# qhasm:         w6_spill = w6
# asm 1: lo>w6_spill=spill64#15 = lo<w6=int64#3
# asm 2: lo>w6_spill=d14 = lo<w6=u2
lod14 = lou2
# asm 1: hi>w6_spill=spill64#15 = hi<w6=int64#3
# asm 2: hi>w6_spill=d14 = hi<w6=u2
hid14 = hiu2
# qhasm:       w13 += sigma0(w14)
# asm 1: hitmp lotmp = hi<w14=int64#1 * two24
# asm 2: hitmp lotmp = hi<w14=u0 * two24
hitmp lotmp = hiu0 * two24
# asm 1: lotmp hitmp += lo<w14=int64#1 * two24
# asm 2: lotmp hitmp += lo<w14=u0 * two24
lotmp hitmp += lou0 * two24
# asm 1: carry? lotmp ^= (lo<w14=int64#1 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w14=u0 unsigned>> 1)
carry? lotmp ^= (lou0 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w14=int64#1 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w14=u0 unsigned>> 1)
hitmp ^= (carry,hiu0 unsigned>> 1)
# asm 1: lotmp ^= (hi<w14=int64#1 << 31)
# asm 2: lotmp ^= (hi<w14=u0 << 31)
lotmp ^= (hiu0 << 31)
# asm 1: lotmp ^= (lo<w14=int64#1 unsigned>>7)
# asm 2: lotmp ^= (lo<w14=u0 unsigned>>7)
lotmp ^= (lou0 unsigned>>7)
# asm 1: lotmp ^= (hi<w14=int64#1 << 25)
# asm 2: lotmp ^= (hi<w14=u0 << 25)
lotmp ^= (hiu0 << 25)
# asm 1: hitmp ^= (hi<w14=int64#1 unsigned>>7)
# asm 2: hitmp ^= (hi<w14=u0 unsigned>>7)
hitmp ^= (hiu0 unsigned>>7)
# asm 1: carry? lo<w13=int64#5 += lotmp
# asm 2: carry? lo<w13=u4 += lotmp
carry? lou4 += lotmp
# asm 1: hi<w13=int64#5 += hitmp + carry
# asm 2: hi<w13=u4 += hitmp + carry
hiu4 += hitmp + carry
# qhasm:       w13 += w6
# asm 1: carry? lo<w13=int64#5 += lo<w6=int64#3
# asm 2: carry? lo<w13=u4 += lo<w6=u2
carry? lou4 += lou2
# asm 1: hi<w13=int64#5 += hi<w6=int64#3 + carry
# asm 2: hi<w13=u4 += hi<w6=u2 + carry
hiu4 += hiu2 + carry
# qhasm:         w5_next = w13
# asm 1: lo>w5_next=stack64#14 = lo<w13=int64#5
# asm 2: lo>w5_next=m13 = lo<w13=u4
lom13 = lou4
# asm 1: hi>w5_next=stack64#14 = hi<w13=int64#5
# asm 2: hi>w5_next=m13 = hi<w13=u4
him13 = hiu4
# qhasm:       w14 += sigma1(w12)
# asm 1: hitmp lotmp = hi<w12=int64#2 * two13
# asm 2: hitmp lotmp = hi<w12=u1 * two13
hitmp lotmp = hiu1 * two13
# asm 1: lotmp hitmp += lo<w12=int64#2 * two13
# asm 2: lotmp hitmp += lo<w12=u1 * two13
lotmp hitmp += lou1 * two13
# asm 1: lotmp ^= (lo<w12=int64#2 unsigned>> 6)
# asm 2: lotmp ^= (lo<w12=u1 unsigned>> 6)
lotmp ^= (lou1 unsigned>> 6)
# asm 1: lotmp ^= (hi<w12=int64#2 << 26)
# asm 2: lotmp ^= (hi<w12=u1 << 26)
lotmp ^= (hiu1 << 26)
# asm 1: lotmp ^= (hi<w12=int64#2 unsigned>> 29)
# asm 2: lotmp ^= (hi<w12=u1 unsigned>> 29)
lotmp ^= (hiu1 unsigned>> 29)
# asm 1: lotmp ^= (lo<w12=int64#2 << 3)
# asm 2: lotmp ^= (lo<w12=u1 << 3)
lotmp ^= (lou1 << 3)
# asm 1: hitmp ^= (hi<w12=int64#2 unsigned>> 6)
# asm 2: hitmp ^= (hi<w12=u1 unsigned>> 6)
hitmp ^= (hiu1 unsigned>> 6)
# asm 1: hitmp ^= (lo<w12=int64#2 unsigned>> 29)
# asm 2: hitmp ^= (lo<w12=u1 unsigned>> 29)
hitmp ^= (lou1 unsigned>> 29)
# asm 1: hitmp ^= (hi<w12=int64#2 << 3)
# asm 2: hitmp ^= (hi<w12=u1 << 3)
hitmp ^= (hiu1 << 3)
# asm 1: carry? lo<w14=int64#1 += lotmp
# asm 2: carry? lo<w14=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<w14=int64#1 += hitmp + carry
# asm 2: hi<w14=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:       w15 = w7_spill
# asm 1: lo>w15=int64#2 = lo<w7_spill=spill64#16
# asm 2: lo>w15=u1 = lo<w7_spill=d15
lou1 = lod15
# asm 1: hi>w15=int64#2 = hi<w7_spill=spill64#16
# asm 2: hi>w15=u1 = hi<w7_spill=d15
hiu1 = hid15
# qhasm:         w7_spill = w7
# asm 1: lo>w7_spill=spill64#16 = lo<w7=int64#4
# asm 2: lo>w7_spill=d15 = lo<w7=u3
lod15 = lou3
# asm 1: hi>w7_spill=spill64#16 = hi<w7=int64#4
# asm 2: hi>w7_spill=d15 = hi<w7=u3
hid15 = hiu3
# qhasm:       w14 += sigma0(w15)
# asm 1: hitmp lotmp = hi<w15=int64#2 * two24
# asm 2: hitmp lotmp = hi<w15=u1 * two24
hitmp lotmp = hiu1 * two24
# asm 1: lotmp hitmp += lo<w15=int64#2 * two24
# asm 2: lotmp hitmp += lo<w15=u1 * two24
lotmp hitmp += lou1 * two24
# asm 1: carry? lotmp ^= (lo<w15=int64#2 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w15=u1 unsigned>> 1)
carry? lotmp ^= (lou1 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w15=int64#2 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w15=u1 unsigned>> 1)
hitmp ^= (carry,hiu1 unsigned>> 1)
# asm 1: lotmp ^= (hi<w15=int64#2 << 31)
# asm 2: lotmp ^= (hi<w15=u1 << 31)
lotmp ^= (hiu1 << 31)
# asm 1: lotmp ^= (lo<w15=int64#2 unsigned>>7)
# asm 2: lotmp ^= (lo<w15=u1 unsigned>>7)
lotmp ^= (lou1 unsigned>>7)
# asm 1: lotmp ^= (hi<w15=int64#2 << 25)
# asm 2: lotmp ^= (hi<w15=u1 << 25)
lotmp ^= (hiu1 << 25)
# asm 1: hitmp ^= (hi<w15=int64#2 unsigned>>7)
# asm 2: hitmp ^= (hi<w15=u1 unsigned>>7)
hitmp ^= (hiu1 unsigned>>7)
# asm 1: carry? lo<w14=int64#1 += lotmp
# asm 2: carry? lo<w14=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<w14=int64#1 += hitmp + carry
# asm 2: hi<w14=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:       w14 += w7
# asm 1: carry? lo<w14=int64#1 += lo<w7=int64#4
# asm 2: carry? lo<w14=u0 += lo<w7=u3
carry? lou0 += lou3
# asm 1: hi<w14=int64#1 += hi<w7=int64#4 + carry
# asm 2: hi<w14=u0 += hi<w7=u3 + carry
hiu0 += hiu3 + carry
# qhasm:         w6_next = w14
# asm 1: lo>w6_next=stack64#15 = lo<w14=int64#1
# asm 2: lo>w6_next=m14 = lo<w14=u0
lom14 = lou0
# asm 1: hi>w6_next=stack64#15 = hi<w14=int64#1
# asm 2: hi>w6_next=m14 = hi<w14=u0
him14 = hiu0
# qhasm:       w15 += sigma1(w13)
# asm 1: hitmp lotmp = hi<w13=int64#5 * two13
# asm 2: hitmp lotmp = hi<w13=u4 * two13
hitmp lotmp = hiu4 * two13
# asm 1: lotmp hitmp += lo<w13=int64#5 * two13
# asm 2: lotmp hitmp += lo<w13=u4 * two13
lotmp hitmp += lou4 * two13
# asm 1: lotmp ^= (lo<w13=int64#5 unsigned>> 6)
# asm 2: lotmp ^= (lo<w13=u4 unsigned>> 6)
lotmp ^= (lou4 unsigned>> 6)
# asm 1: lotmp ^= (hi<w13=int64#5 << 26)
# asm 2: lotmp ^= (hi<w13=u4 << 26)
lotmp ^= (hiu4 << 26)
# asm 1: lotmp ^= (hi<w13=int64#5 unsigned>> 29)
# asm 2: lotmp ^= (hi<w13=u4 unsigned>> 29)
lotmp ^= (hiu4 unsigned>> 29)
# asm 1: lotmp ^= (lo<w13=int64#5 << 3)
# asm 2: lotmp ^= (lo<w13=u4 << 3)
lotmp ^= (lou4 << 3)
# asm 1: hitmp ^= (hi<w13=int64#5 unsigned>> 6)
# asm 2: hitmp ^= (hi<w13=u4 unsigned>> 6)
hitmp ^= (hiu4 unsigned>> 6)
# asm 1: hitmp ^= (lo<w13=int64#5 unsigned>> 29)
# asm 2: hitmp ^= (lo<w13=u4 unsigned>> 29)
hitmp ^= (lou4 unsigned>> 29)
# asm 1: hitmp ^= (hi<w13=int64#5 << 3)
# asm 2: hitmp ^= (hi<w13=u4 << 3)
hitmp ^= (hiu4 << 3)
# asm 1: carry? lo<w15=int64#2 += lotmp
# asm 2: carry? lo<w15=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<w15=int64#2 += hitmp + carry
# asm 2: hi<w15=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:       w0 = w0_next
# asm 1: lo>w0=int64#1 = lo<w0_next=stack64#9
# asm 2: lo>w0=u0 = lo<w0_next=m8
lou0 = lom8
# asm 1: hi>w0=int64#1 = hi<w0_next=stack64#9
# asm 2: hi>w0=u0 = hi<w0_next=m8
hiu0 = him8
# qhasm:         w8 = w8_stack
# asm 1: lo>w8=int64#3 = lo<w8_stack=stack64#16
# asm 2: lo>w8=u2 = lo<w8_stack=m15
lou2 = lom15
# asm 1: hi>w8=int64#3 = hi<w8_stack=stack64#16
# asm 2: hi>w8=u2 = hi<w8_stack=m15
hiu2 = him15
# qhasm:         w0_next = w8
# asm 1: lo>w0_next=stack64#9 = lo<w8=int64#3
# asm 2: lo>w0_next=m8 = lo<w8=u2
lom8 = lou2
# asm 1: hi>w0_next=stack64#9 = hi<w8=int64#3
# asm 2: hi>w0_next=m8 = hi<w8=u2
him8 = hiu2
# qhasm:       w15 += sigma0(w0)
# asm 1: hitmp lotmp = hi<w0=int64#1 * two24
# asm 2: hitmp lotmp = hi<w0=u0 * two24
hitmp lotmp = hiu0 * two24
# asm 1: lotmp hitmp += lo<w0=int64#1 * two24
# asm 2: lotmp hitmp += lo<w0=u0 * two24
lotmp hitmp += lou0 * two24
# asm 1: carry? lotmp ^= (lo<w0=int64#1 unsigned>> 1)
# asm 2: carry? lotmp ^= (lo<w0=u0 unsigned>> 1)
carry? lotmp ^= (lou0 unsigned>> 1)
# asm 1: hitmp ^= (carry,hi<w0=int64#1 unsigned>> 1)
# asm 2: hitmp ^= (carry,hi<w0=u0 unsigned>> 1)
hitmp ^= (carry,hiu0 unsigned>> 1)
# asm 1: lotmp ^= (hi<w0=int64#1 << 31)
# asm 2: lotmp ^= (hi<w0=u0 << 31)
lotmp ^= (hiu0 << 31)
# asm 1: lotmp ^= (lo<w0=int64#1 unsigned>>7)
# asm 2: lotmp ^= (lo<w0=u0 unsigned>>7)
lotmp ^= (lou0 unsigned>>7)
# asm 1: lotmp ^= (hi<w0=int64#1 << 25)
# asm 2: lotmp ^= (hi<w0=u0 << 25)
lotmp ^= (hiu0 << 25)
# asm 1: hitmp ^= (hi<w0=int64#1 unsigned>>7)
# asm 2: hitmp ^= (hi<w0=u0 unsigned>>7)
hitmp ^= (hiu0 unsigned>>7)
# asm 1: carry? lo<w15=int64#2 += lotmp
# asm 2: carry? lo<w15=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<w15=int64#2 += hitmp + carry
# asm 2: hi<w15=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:       w15 += w8
# asm 1: carry? lo<w15=int64#2 += lo<w8=int64#3
# asm 2: carry? lo<w15=u1 += lo<w8=u2
carry? lou1 += lou2
# asm 1: hi<w15=int64#2 += hi<w8=int64#3 + carry
# asm 2: hi<w15=u1 += hi<w8=u2 + carry
hiu1 += hiu2 + carry
# qhasm:         w7_next = w15
# asm 1: lo>w7_next=stack64#16 = lo<w15=int64#2
# asm 2: lo>w7_next=m15 = lo<w15=u1
lom15 = lou1
# asm 1: hi>w7_next=stack64#16 = hi<w15=int64#2
# asm 2: hi>w7_next=m15 = hi<w15=u1
him15 = hiu1
# qhasm:         w0_spill = w0
# asm 1: lo>w0_spill=spill64#9 = lo<w0=int64#1
# asm 2: lo>w0_spill=d8 = lo<w0=u0
lod8 = lou0
# asm 1: hi>w0_spill=spill64#9 = hi<w0=int64#1
# asm 2: hi>w0_spill=d8 = hi<w0=u0
hid8 = hiu0
# qhasm:     goto innerloop
goto innerloop
# qhasm:     nearend:
nearend:
# qhasm:       w0 = w0_next
# asm 1: lo>w0=int64#1 = lo<w0_next=stack64#9
# asm 2: lo>w0=u0 = lo<w0_next=m8
lou0 = lom8
# asm 1: hi>w0=int64#1 = hi<w0_next=stack64#9
# asm 2: hi>w0=u0 = hi<w0_next=m8
hiu0 = him8
# qhasm:       w1 = w1_next
# asm 1: lo>w1=int64#2 = lo<w1_next=stack64#10
# asm 2: lo>w1=u1 = lo<w1_next=m9
lou1 = lom9
# asm 1: hi>w1=int64#2 = hi<w1_next=stack64#10
# asm 2: hi>w1=u1 = hi<w1_next=m9
hiu1 = him9
# qhasm:       w2 = w2_next
# asm 1: lo>w2=int64#3 = lo<w2_next=stack64#11
# asm 2: lo>w2=u2 = lo<w2_next=m10
lou2 = lom10
# asm 1: hi>w2=int64#3 = hi<w2_next=stack64#11
# asm 2: hi>w2=u2 = hi<w2_next=m10
hiu2 = him10
# qhasm:       w3 = w3_next
# asm 1: lo>w3=int64#4 = lo<w3_next=stack64#12
# asm 2: lo>w3=u3 = lo<w3_next=m11
lou3 = lom11
# asm 1: hi>w3=int64#4 = hi<w3_next=stack64#12
# asm 2: hi>w3=u3 = hi<w3_next=m11
hiu3 = him11
# qhasm:       w0_spill = w0
# asm 1: lo>w0_spill=spill64#9 = lo<w0=int64#1
# asm 2: lo>w0_spill=d8 = lo<w0=u0
lod8 = lou0
# asm 1: hi>w0_spill=spill64#9 = hi<w0=int64#1
# asm 2: hi>w0_spill=d8 = hi<w0=u0
hid8 = hiu0
# qhasm:       w1_spill = w1
# asm 1: lo>w1_spill=spill64#10 = lo<w1=int64#2
# asm 2: lo>w1_spill=d9 = lo<w1=u1
lod9 = lou1
# asm 1: hi>w1_spill=spill64#10 = hi<w1=int64#2
# asm 2: hi>w1_spill=d9 = hi<w1=u1
hid9 = hiu1
# qhasm:       w2_spill = w2
# asm 1: lo>w2_spill=spill64#11 = lo<w2=int64#3
# asm 2: lo>w2_spill=d10 = lo<w2=u2
lod10 = lou2
# asm 1: hi>w2_spill=spill64#11 = hi<w2=int64#3
# asm 2: hi>w2_spill=d10 = hi<w2=u2
hid10 = hiu2
# qhasm:       w3_spill = w3
# asm 1: lo>w3_spill=spill64#12 = lo<w3=int64#4
# asm 2: lo>w3_spill=d11 = lo<w3=u3
lod11 = lou3
# asm 1: hi>w3_spill=spill64#12 = hi<w3=int64#4
# asm 2: hi>w3_spill=d11 = hi<w3=u3
hid11 = hiu3
# qhasm:       w4 = w4_next
# asm 1: lo>w4=int64#1 = lo<w4_next=stack64#13
# asm 2: lo>w4=u0 = lo<w4_next=m12
lou0 = lom12
# asm 1: hi>w4=int64#1 = hi<w4_next=stack64#13
# asm 2: hi>w4=u0 = hi<w4_next=m12
hiu0 = him12
# qhasm:       w5 = w5_next
# asm 1: lo>w5=int64#2 = lo<w5_next=stack64#14
# asm 2: lo>w5=u1 = lo<w5_next=m13
lou1 = lom13
# asm 1: hi>w5=int64#2 = hi<w5_next=stack64#14
# asm 2: hi>w5=u1 = hi<w5_next=m13
hiu1 = him13
# qhasm:       w6 = w6_next
# asm 1: lo>w6=int64#3 = lo<w6_next=stack64#15
# asm 2: lo>w6=u2 = lo<w6_next=m14
lou2 = lom14
# asm 1: hi>w6=int64#3 = hi<w6_next=stack64#15
# asm 2: hi>w6=u2 = hi<w6_next=m14
hiu2 = him14
# qhasm:       w7 = w7_next
# asm 1: lo>w7=int64#4 = lo<w7_next=stack64#16
# asm 2: lo>w7=u3 = lo<w7_next=m15
lou3 = lom15
# asm 1: hi>w7=int64#4 = hi<w7_next=stack64#16
# asm 2: hi>w7=u3 = hi<w7_next=m15
hiu3 = him15
# qhasm:       w4_spill = w4
# asm 1: lo>w4_spill=spill64#13 = lo<w4=int64#1
# asm 2: lo>w4_spill=d12 = lo<w4=u0
lod12 = lou0
# asm 1: hi>w4_spill=spill64#13 = hi<w4=int64#1
# asm 2: hi>w4_spill=d12 = hi<w4=u0
hid12 = hiu0
# qhasm:       w5_spill = w5
# asm 1: lo>w5_spill=spill64#14 = lo<w5=int64#2
# asm 2: lo>w5_spill=d13 = lo<w5=u1
lod13 = lou1
# asm 1: hi>w5_spill=spill64#14 = hi<w5=int64#2
# asm 2: hi>w5_spill=d13 = hi<w5=u1
hid13 = hiu1
# qhasm:       w6_spill = w6
# asm 1: lo>w6_spill=spill64#15 = lo<w6=int64#3
# asm 2: lo>w6_spill=d14 = lo<w6=u2
lod14 = lou2
# asm 1: hi>w6_spill=spill64#15 = hi<w6=int64#3
# asm 2: hi>w6_spill=d14 = hi<w6=u2
hid14 = hiu2
# qhasm:       w7_spill = w7
# asm 1: lo>w7_spill=spill64#16 = lo<w7=int64#4
# asm 2: lo>w7_spill=d15 = lo<w7=u3
lod15 = lou3
# asm 1: hi>w7_spill=spill64#16 = hi<w7=int64#4
# asm 2: hi>w7_spill=d15 = hi<w7=u3
hid15 = hiu3
# qhasm:     goto innerloop
goto innerloop
# qhasm:   endinnerloop:
endinnerloop:
# qhasm:   constants = constants_stack
# asm 1: >constants=int32#1 = <constants_stack=stack32#4
# asm 2: >constants=input_0 = <constants_stack=o3
input_0 = o3
# qhasm:   constants -= 640
# asm 1: <constants=int32#1 -= 640
# asm 2: <constants=input_0 -= 640
input_0 -= 640
# qhasm:   constants_stack = constants
# asm 1: >constants_stack=stack32#4 = <constants=int32#1
# asm 2: >constants_stack=o3 = <constants=input_0
o3 = input_0
# qhasm:   r0 = r0_spill
# asm 1: lo>r0=int64#1 = lo<r0_spill=spill64#1
# asm 2: lo>r0=u0 = lo<r0_spill=d0
lou0 = lod0
# asm 1: hi>r0=int64#1 = hi<r0_spill=spill64#1
# asm 2: hi>r0=u0 = hi<r0_spill=d0
hiu0 = hid0
# qhasm:   r1 = r1_spill
# asm 1: lo>r1=int64#2 = lo<r1_spill=spill64#2
# asm 2: lo>r1=u1 = lo<r1_spill=d1
lou1 = lod1
# asm 1: hi>r1=int64#2 = hi<r1_spill=spill64#2
# asm 2: hi>r1=u1 = hi<r1_spill=d1
hiu1 = hid1
# qhasm:   r2 = r2_spill
# asm 1: lo>r2=int64#3 = lo<r2_spill=spill64#3
# asm 2: lo>r2=u2 = lo<r2_spill=d2
lou2 = lod2
# asm 1: hi>r2=int64#3 = hi<r2_spill=spill64#3
# asm 2: hi>r2=u2 = hi<r2_spill=d2
hiu2 = hid2
# qhasm:   r3 = r3_spill
# asm 1: lo>r3=int64#4 = lo<r3_spill=spill64#4
# asm 2: lo>r3=u3 = lo<r3_spill=d3
lou3 = lod3
# asm 1: hi>r3=int64#4 = hi<r3_spill=spill64#4
# asm 2: hi>r3=u3 = hi<r3_spill=d3
hiu3 = hid3
# qhasm:   r0 += state0
# asm 1: lotmp = lo<state0=stack64#1
# asm 2: lotmp = lo<state0=m0
lotmp = lom0
# asm 1: hitmp = hi<state0=stack64#1
# asm 2: hitmp = hi<state0=m0
hitmp = him0
# asm 1: carry? lo<r0=int64#1 += lotmp
# asm 2: carry? lo<r0=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<r0=int64#1 += hitmp + carry
# asm 2: hi<r0=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:   r1 += state1
# asm 1: lotmp = lo<state1=stack64#2
# asm 2: lotmp = lo<state1=m1
lotmp = lom1
# asm 1: hitmp = hi<state1=stack64#2
# asm 2: hitmp = hi<state1=m1
hitmp = him1
# asm 1: carry? lo<r1=int64#2 += lotmp
# asm 2: carry? lo<r1=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<r1=int64#2 += hitmp + carry
# asm 2: hi<r1=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:   r2 += state2
# asm 1: lotmp = lo<state2=stack64#3
# asm 2: lotmp = lo<state2=m2
lotmp = lom2
# asm 1: hitmp = hi<state2=stack64#3
# asm 2: hitmp = hi<state2=m2
hitmp = him2
# asm 1: carry? lo<r2=int64#3 += lotmp
# asm 2: carry? lo<r2=u2 += lotmp
carry? lou2 += lotmp
# asm 1: hi<r2=int64#3 += hitmp + carry
# asm 2: hi<r2=u2 += hitmp + carry
hiu2 += hitmp + carry
# qhasm:   r3 += state3
# asm 1: lotmp = lo<state3=stack64#4
# asm 2: lotmp = lo<state3=m3
lotmp = lom3
# asm 1: hitmp = hi<state3=stack64#4
# asm 2: hitmp = hi<state3=m3
hitmp = him3
# asm 1: carry? lo<r3=int64#4 += lotmp
# asm 2: carry? lo<r3=u3 += lotmp
carry? lou3 += lotmp
# asm 1: hi<r3=int64#4 += hitmp + carry
# asm 2: hi<r3=u3 += hitmp + carry
hiu3 += hitmp + carry
# qhasm:   state0 = r0
# asm 1: lo>state0=stack64#1 = lo<r0=int64#1
# asm 2: lo>state0=m0 = lo<r0=u0
lom0 = lou0
# asm 1: hi>state0=stack64#1 = hi<r0=int64#1
# asm 2: hi>state0=m0 = hi<r0=u0
him0 = hiu0
# qhasm:   state1 = r1
# asm 1: lo>state1=stack64#2 = lo<r1=int64#2
# asm 2: lo>state1=m1 = lo<r1=u1
lom1 = lou1
# asm 1: hi>state1=stack64#2 = hi<r1=int64#2
# asm 2: hi>state1=m1 = hi<r1=u1
him1 = hiu1
# qhasm:   state2 = r2
# asm 1: lo>state2=stack64#3 = lo<r2=int64#3
# asm 2: lo>state2=m2 = lo<r2=u2
lom2 = lou2
# asm 1: hi>state2=stack64#3 = hi<r2=int64#3
# asm 2: hi>state2=m2 = hi<r2=u2
him2 = hiu2
# qhasm:   state3 = r3
# asm 1: lo>state3=stack64#4 = lo<r3=int64#4
# asm 2: lo>state3=m3 = lo<r3=u3
lom3 = lou3
# asm 1: hi>state3=stack64#4 = hi<r3=int64#4
# asm 2: hi>state3=m3 = hi<r3=u3
him3 = hiu3
# qhasm:   r0_spill = r0
# asm 1: lo>r0_spill=spill64#1 = lo<r0=int64#1
# asm 2: lo>r0_spill=d0 = lo<r0=u0
lod0 = lou0
# asm 1: hi>r0_spill=spill64#1 = hi<r0=int64#1
# asm 2: hi>r0_spill=d0 = hi<r0=u0
hid0 = hiu0
# qhasm:   r1_spill = r1
# asm 1: lo>r1_spill=spill64#2 = lo<r1=int64#2
# asm 2: lo>r1_spill=d1 = lo<r1=u1
lod1 = lou1
# asm 1: hi>r1_spill=spill64#2 = hi<r1=int64#2
# asm 2: hi>r1_spill=d1 = hi<r1=u1
hid1 = hiu1
# qhasm:   r2_spill = r2
# asm 1: lo>r2_spill=spill64#3 = lo<r2=int64#3
# asm 2: lo>r2_spill=d2 = lo<r2=u2
lod2 = lou2
# asm 1: hi>r2_spill=spill64#3 = hi<r2=int64#3
# asm 2: hi>r2_spill=d2 = hi<r2=u2
hid2 = hiu2
# qhasm:   r3_spill = r3
# asm 1: lo>r3_spill=spill64#4 = lo<r3=int64#4
# asm 2: lo>r3_spill=d3 = lo<r3=u3
lod3 = lou3
# asm 1: hi>r3_spill=spill64#4 = hi<r3=int64#4
# asm 2: hi>r3_spill=d3 = hi<r3=u3
hid3 = hiu3
# qhasm:   r4 = r4_spill
# asm 1: lo>r4=int64#1 = lo<r4_spill=spill64#5
# asm 2: lo>r4=u0 = lo<r4_spill=d4
lou0 = lod4
# asm 1: hi>r4=int64#1 = hi<r4_spill=spill64#5
# asm 2: hi>r4=u0 = hi<r4_spill=d4
hiu0 = hid4
# qhasm:   r5 = r5_spill
# asm 1: lo>r5=int64#2 = lo<r5_spill=spill64#6
# asm 2: lo>r5=u1 = lo<r5_spill=d5
lou1 = lod5
# asm 1: hi>r5=int64#2 = hi<r5_spill=spill64#6
# asm 2: hi>r5=u1 = hi<r5_spill=d5
hiu1 = hid5
# qhasm:   r6 = r6_spill
# asm 1: lo>r6=int64#3 = lo<r6_spill=spill64#7
# asm 2: lo>r6=u2 = lo<r6_spill=d6
lou2 = lod6
# asm 1: hi>r6=int64#3 = hi<r6_spill=spill64#7
# asm 2: hi>r6=u2 = hi<r6_spill=d6
hiu2 = hid6
# qhasm:   r7 = r7_spill
# asm 1: lo>r7=int64#4 = lo<r7_spill=spill64#8
# asm 2: lo>r7=u3 = lo<r7_spill=d7
lou3 = lod7
# asm 1: hi>r7=int64#4 = hi<r7_spill=spill64#8
# asm 2: hi>r7=u3 = hi<r7_spill=d7
hiu3 = hid7
# qhasm:   r4 += state4
# asm 1: lotmp = lo<state4=stack64#5
# asm 2: lotmp = lo<state4=m4
lotmp = lom4
# asm 1: hitmp = hi<state4=stack64#5
# asm 2: hitmp = hi<state4=m4
hitmp = him4
# asm 1: carry? lo<r4=int64#1 += lotmp
# asm 2: carry? lo<r4=u0 += lotmp
carry? lou0 += lotmp
# asm 1: hi<r4=int64#1 += hitmp + carry
# asm 2: hi<r4=u0 += hitmp + carry
hiu0 += hitmp + carry
# qhasm:   r5 += state5
# asm 1: lotmp = lo<state5=stack64#6
# asm 2: lotmp = lo<state5=m5
lotmp = lom5
# asm 1: hitmp = hi<state5=stack64#6
# asm 2: hitmp = hi<state5=m5
hitmp = him5
# asm 1: carry? lo<r5=int64#2 += lotmp
# asm 2: carry? lo<r5=u1 += lotmp
carry? lou1 += lotmp
# asm 1: hi<r5=int64#2 += hitmp + carry
# asm 2: hi<r5=u1 += hitmp + carry
hiu1 += hitmp + carry
# qhasm:   r6 += state6
# asm 1: lotmp = lo<state6=stack64#7
# asm 2: lotmp = lo<state6=m6
lotmp = lom6
# asm 1: hitmp = hi<state6=stack64#7
# asm 2: hitmp = hi<state6=m6
hitmp = him6
# asm 1: carry? lo<r6=int64#3 += lotmp
# asm 2: carry? lo<r6=u2 += lotmp
carry? lou2 += lotmp
# asm 1: hi<r6=int64#3 += hitmp + carry
# asm 2: hi<r6=u2 += hitmp + carry
hiu2 += hitmp + carry
# qhasm:   r7 += state7
# asm 1: lotmp = lo<state7=stack64#8
# asm 2: lotmp = lo<state7=m7
lotmp = lom7
# asm 1: hitmp = hi<state7=stack64#8
# asm 2: hitmp = hi<state7=m7
hitmp = him7
# asm 1: carry? lo<r7=int64#4 += lotmp
# asm 2: carry? lo<r7=u3 += lotmp
carry? lou3 += lotmp
# asm 1: hi<r7=int64#4 += hitmp + carry
# asm 2: hi<r7=u3 += hitmp + carry
hiu3 += hitmp + carry
# qhasm:   state4 = r4
# asm 1: lo>state4=stack64#5 = lo<r4=int64#1
# asm 2: lo>state4=m4 = lo<r4=u0
lom4 = lou0
# asm 1: hi>state4=stack64#5 = hi<r4=int64#1
# asm 2: hi>state4=m4 = hi<r4=u0
him4 = hiu0
# qhasm:   state5 = r5
# asm 1: lo>state5=stack64#6 = lo<r5=int64#2
# asm 2: lo>state5=m5 = lo<r5=u1
lom5 = lou1
# asm 1: hi>state5=stack64#6 = hi<r5=int64#2
# asm 2: hi>state5=m5 = hi<r5=u1
him5 = hiu1
# qhasm:   state6 = r6
# asm 1: lo>state6=stack64#7 = lo<r6=int64#3
# asm 2: lo>state6=m6 = lo<r6=u2
lom6 = lou2
# asm 1: hi>state6=stack64#7 = hi<r6=int64#3
# asm 2: hi>state6=m6 = hi<r6=u2
him6 = hiu2
# qhasm:   state7 = r7
# asm 1: lo>state7=stack64#8 = lo<r7=int64#4
# asm 2: lo>state7=m7 = lo<r7=u3
lom7 = lou3
# asm 1: hi>state7=stack64#8 = hi<r7=int64#4
# asm 2: hi>state7=m7 = hi<r7=u3
him7 = hiu3
# qhasm:   r4_spill = r4
# asm 1: lo>r4_spill=spill64#5 = lo<r4=int64#1
# asm 2: lo>r4_spill=d4 = lo<r4=u0
lod4 = lou0
# asm 1: hi>r4_spill=spill64#5 = hi<r4=int64#1
# asm 2: hi>r4_spill=d4 = hi<r4=u0
hid4 = hiu0
# qhasm:   r5_spill = r5
# asm 1: lo>r5_spill=spill64#6 = lo<r5=int64#2
# asm 2: lo>r5_spill=d5 = lo<r5=u1
lod5 = lou1
# asm 1: hi>r5_spill=spill64#6 = hi<r5=int64#2
# asm 2: hi>r5_spill=d5 = hi<r5=u1
hid5 = hiu1
# qhasm:   r6_spill = r6
# asm 1: lo>r6_spill=spill64#7 = lo<r6=int64#3
# asm 2: lo>r6_spill=d6 = lo<r6=u2
lod6 = lou2
# asm 1: hi>r6_spill=spill64#7 = hi<r6=int64#3
# asm 2: hi>r6_spill=d6 = hi<r6=u2
hid6 = hiu2
# qhasm:   r7_spill = r7
# asm 1: lo>r7_spill=spill64#8 = lo<r7=int64#4
# asm 2: lo>r7_spill=d7 = lo<r7=u3
lod7 = lou3
# asm 1: hi>r7_spill=spill64#8 = hi<r7=int64#4
# asm 2: hi>r7_spill=d7 = hi<r7=u3
hid7 = hiu3
# qhasm:   inlen = inlen_stack
# asm 1: >inlen=int32#1 = <inlen_stack=stack32#3
# asm 2: >inlen=input_0 = <inlen_stack=o2
input_0 = o2
# qhasm:                    unsigned>=? inlen -= 128
# asm 1: =? unsigned<? <inlen=int32#1 -= 128
# asm 2: =? unsigned<? <inlen=input_0 -= 128
=? unsigned<? input_0 -= 128
# qhasm:   inlen_stack = inlen
# asm 1: >inlen_stack=stack32#3 = <inlen=int32#1
# asm 2: >inlen_stack=o2 = <inlen=input_0
o2 = input_0
# qhasm:   goto mainloop if unsigned>=
goto mainloop if !unsigned<
# qhasm: endmainloop:
endmainloop:
# qhasm: statebytes = statebytes_stack
# asm 1: >statebytes=int32#2 = <statebytes_stack=stack32#1
# asm 2: >statebytes=input_1 = <statebytes_stack=o0
input_1 = o0
# qhasm: r0 = state0
# asm 1: lo>r0=int64#1 = lo<state0=stack64#1
# asm 2: lo>r0=u0 = lo<state0=m0
lou0 = lom0
# asm 1: hi>r0=int64#1 = hi<state0=stack64#1
# asm 2: hi>r0=u0 = hi<state0=m0
hiu0 = him0
# qhasm: r1 = state1
# asm 1: lo>r1=int64#2 = lo<state1=stack64#2
# asm 2: lo>r1=u1 = lo<state1=m1
lou1 = lom1
# asm 1: hi>r1=int64#2 = hi<state1=stack64#2
# asm 2: hi>r1=u1 = hi<state1=m1
hiu1 = him1
# qhasm: r2 = state2
# asm 1: lo>r2=int64#3 = lo<state2=stack64#3
# asm 2: lo>r2=u2 = lo<state2=m2
lou2 = lom2
# asm 1: hi>r2=int64#3 = hi<state2=stack64#3
# asm 2: hi>r2=u2 = hi<state2=m2
hiu2 = him2
# qhasm: r3 = state3
# asm 1: lo>r3=int64#4 = lo<state3=stack64#4
# asm 2: lo>r3=u3 = lo<state3=m3
lou3 = lom3
# asm 1: hi>r3=int64#4 = hi<state3=stack64#4
# asm 2: hi>r3=u3 = hi<state3=m3
hiu3 = him3
# qhasm: r0 = reverse flip r0
# asm 1: lo>r0=int64#1 = lo<r0=int64#1[3]lo<r0=int64#1[2]lo<r0=int64#1[1]lo<r0=int64#1[0]
# asm 2: lo>r0=u0 = lo<r0=u0[3]lo<r0=u0[2]lo<r0=u0[1]lo<r0=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>r0=int64#1 = hi<r0=int64#1[3]hi<r0=int64#1[2]hi<r0=int64#1[1]hi<r0=int64#1[0]
# asm 2: hi>r0=u0 = hi<r0=u0[3]hi<r0=u0[2]hi<r0=u0[1]hi<r0=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm: r1 = reverse flip r1
# asm 1: lo>r1=int64#2 = lo<r1=int64#2[3]lo<r1=int64#2[2]lo<r1=int64#2[1]lo<r1=int64#2[0]
# asm 2: lo>r1=u1 = lo<r1=u1[3]lo<r1=u1[2]lo<r1=u1[1]lo<r1=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>r1=int64#2 = hi<r1=int64#2[3]hi<r1=int64#2[2]hi<r1=int64#2[1]hi<r1=int64#2[0]
# asm 2: hi>r1=u1 = hi<r1=u1[3]hi<r1=u1[2]hi<r1=u1[1]hi<r1=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm: r2 = reverse flip r2
# asm 1: lo>r2=int64#3 = lo<r2=int64#3[3]lo<r2=int64#3[2]lo<r2=int64#3[1]lo<r2=int64#3[0]
# asm 2: lo>r2=u2 = lo<r2=u2[3]lo<r2=u2[2]lo<r2=u2[1]lo<r2=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>r2=int64#3 = hi<r2=int64#3[3]hi<r2=int64#3[2]hi<r2=int64#3[1]hi<r2=int64#3[0]
# asm 2: hi>r2=u2 = hi<r2=u2[3]hi<r2=u2[2]hi<r2=u2[1]hi<r2=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm: r3 = reverse flip r3
# asm 1: lo>r3=int64#4 = lo<r3=int64#4[3]lo<r3=int64#4[2]lo<r3=int64#4[1]lo<r3=int64#4[0]
# asm 2: lo>r3=u3 = lo<r3=u3[3]lo<r3=u3[2]lo<r3=u3[1]lo<r3=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>r3=int64#4 = hi<r3=int64#4[3]hi<r3=int64#4[2]hi<r3=int64#4[1]hi<r3=int64#4[0]
# asm 2: hi>r3=u3 = hi<r3=u3[3]hi<r3=u3[2]hi<r3=u3[1]hi<r3=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm: mem64[statebytes] = flip r0
# asm 1: mem32[<statebytes=int32#2] = hi<r0=int64#1
# asm 2: mem32[<statebytes=input_1] = hi<r0=u0
mem32[input_1] = hiu0
# asm 1: mem32[<statebytes=int32#2+4] = lo<r0=int64#1
# asm 2: mem32[<statebytes=input_1+4] = lo<r0=u0
mem32[input_1+4] = lou0
# qhasm: mem64[statebytes+8] = flip r1
# asm 1: mem32[<statebytes=int32#2+8] = hi<r1=int64#2
# asm 2: mem32[<statebytes=input_1+8] = hi<r1=u1
mem32[input_1+8] = hiu1
# asm 1: mem32[<statebytes=int32#2+12] = lo<r1=int64#2
# asm 2: mem32[<statebytes=input_1+12] = lo<r1=u1
mem32[input_1+12] = lou1
# qhasm: mem64[statebytes+16] = flip r2
# asm 1: mem32[<statebytes=int32#2+16] = hi<r2=int64#3
# asm 2: mem32[<statebytes=input_1+16] = hi<r2=u2
mem32[input_1+16] = hiu2
# asm 1: mem32[<statebytes=int32#2+20] = lo<r2=int64#3
# asm 2: mem32[<statebytes=input_1+20] = lo<r2=u2
mem32[input_1+20] = lou2
# qhasm: mem64[statebytes+24] = flip r3
# asm 1: mem32[<statebytes=int32#2+24] = hi<r3=int64#4
# asm 2: mem32[<statebytes=input_1+24] = hi<r3=u3
mem32[input_1+24] = hiu3
# asm 1: mem32[<statebytes=int32#2+28] = lo<r3=int64#4
# asm 2: mem32[<statebytes=input_1+28] = lo<r3=u3
mem32[input_1+28] = lou3
# qhasm: r4 = state4
# asm 1: lo>r4=int64#1 = lo<state4=stack64#5
# asm 2: lo>r4=u0 = lo<state4=m4
lou0 = lom4
# asm 1: hi>r4=int64#1 = hi<state4=stack64#5
# asm 2: hi>r4=u0 = hi<state4=m4
hiu0 = him4
# qhasm: r5 = state5
# asm 1: lo>r5=int64#2 = lo<state5=stack64#6
# asm 2: lo>r5=u1 = lo<state5=m5
lou1 = lom5
# asm 1: hi>r5=int64#2 = hi<state5=stack64#6
# asm 2: hi>r5=u1 = hi<state5=m5
hiu1 = him5
# qhasm: r6 = state6
# asm 1: lo>r6=int64#3 = lo<state6=stack64#7
# asm 2: lo>r6=u2 = lo<state6=m6
lou2 = lom6
# asm 1: hi>r6=int64#3 = hi<state6=stack64#7
# asm 2: hi>r6=u2 = hi<state6=m6
hiu2 = him6
# qhasm: r7 = state7
# asm 1: lo>r7=int64#4 = lo<state7=stack64#8
# asm 2: lo>r7=u3 = lo<state7=m7
lou3 = lom7
# asm 1: hi>r7=int64#4 = hi<state7=stack64#8
# asm 2: hi>r7=u3 = hi<state7=m7
hiu3 = him7
# qhasm: r4 = reverse flip r4
# asm 1: lo>r4=int64#1 = lo<r4=int64#1[3]lo<r4=int64#1[2]lo<r4=int64#1[1]lo<r4=int64#1[0]
# asm 2: lo>r4=u0 = lo<r4=u0[3]lo<r4=u0[2]lo<r4=u0[1]lo<r4=u0[0]
lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: hi>r4=int64#1 = hi<r4=int64#1[3]hi<r4=int64#1[2]hi<r4=int64#1[1]hi<r4=int64#1[0]
# asm 2: hi>r4=u0 = hi<r4=u0[3]hi<r4=u0[2]hi<r4=u0[1]hi<r4=u0[0]
hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# qhasm: r5 = reverse flip r5
# asm 1: lo>r5=int64#2 = lo<r5=int64#2[3]lo<r5=int64#2[2]lo<r5=int64#2[1]lo<r5=int64#2[0]
# asm 2: lo>r5=u1 = lo<r5=u1[3]lo<r5=u1[2]lo<r5=u1[1]lo<r5=u1[0]
lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: hi>r5=int64#2 = hi<r5=int64#2[3]hi<r5=int64#2[2]hi<r5=int64#2[1]hi<r5=int64#2[0]
# asm 2: hi>r5=u1 = hi<r5=u1[3]hi<r5=u1[2]hi<r5=u1[1]hi<r5=u1[0]
hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# qhasm: r6 = reverse flip r6
# asm 1: lo>r6=int64#3 = lo<r6=int64#3[3]lo<r6=int64#3[2]lo<r6=int64#3[1]lo<r6=int64#3[0]
# asm 2: lo>r6=u2 = lo<r6=u2[3]lo<r6=u2[2]lo<r6=u2[1]lo<r6=u2[0]
lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: hi>r6=int64#3 = hi<r6=int64#3[3]hi<r6=int64#3[2]hi<r6=int64#3[1]hi<r6=int64#3[0]
# asm 2: hi>r6=u2 = hi<r6=u2[3]hi<r6=u2[2]hi<r6=u2[1]hi<r6=u2[0]
hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# qhasm: r7 = reverse flip r7
# asm 1: lo>r7=int64#4 = lo<r7=int64#4[3]lo<r7=int64#4[2]lo<r7=int64#4[1]lo<r7=int64#4[0]
# asm 2: lo>r7=u3 = lo<r7=u3[3]lo<r7=u3[2]lo<r7=u3[1]lo<r7=u3[0]
lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: hi>r7=int64#4 = hi<r7=int64#4[3]hi<r7=int64#4[2]hi<r7=int64#4[1]hi<r7=int64#4[0]
# asm 2: hi>r7=u3 = hi<r7=u3[3]hi<r7=u3[2]hi<r7=u3[1]hi<r7=u3[0]
hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# qhasm: mem64[statebytes+32] = flip r4
# asm 1: mem32[<statebytes=int32#2+32] = hi<r4=int64#1
# asm 2: mem32[<statebytes=input_1+32] = hi<r4=u0
mem32[input_1+32] = hiu0
# asm 1: mem32[<statebytes=int32#2+36] = lo<r4=int64#1
# asm 2: mem32[<statebytes=input_1+36] = lo<r4=u0
mem32[input_1+36] = lou0
# qhasm: mem64[statebytes+40] = flip r5
# asm 1: mem32[<statebytes=int32#2+40] = hi<r5=int64#2
# asm 2: mem32[<statebytes=input_1+40] = hi<r5=u1
mem32[input_1+40] = hiu1
# asm 1: mem32[<statebytes=int32#2+44] = lo<r5=int64#2
# asm 2: mem32[<statebytes=input_1+44] = lo<r5=u1
mem32[input_1+44] = lou1
# qhasm: mem64[statebytes+48] = flip r6
# asm 1: mem32[<statebytes=int32#2+48] = hi<r6=int64#3
# asm 2: mem32[<statebytes=input_1+48] = hi<r6=u2
mem32[input_1+48] = hiu2
# asm 1: mem32[<statebytes=int32#2+52] = lo<r6=int64#3
# asm 2: mem32[<statebytes=input_1+52] = lo<r6=u2
mem32[input_1+52] = lou2
# qhasm: mem64[statebytes+56] = flip r7
# asm 1: mem32[<statebytes=int32#2+56] = hi<r7=int64#4
# asm 2: mem32[<statebytes=input_1+56] = hi<r7=u3
mem32[input_1+56] = hiu3
# asm 1: mem32[<statebytes=int32#2+60] = lo<r7=int64#4
# asm 2: mem32[<statebytes=input_1+60] = lo<r7=u3
mem32[input_1+60] = lou3
# qhasm: inlen += 128
# asm 1: <inlen=int32#1 += 128
# asm 2: <inlen=input_0 += 128
input_0 += 128
# qhasm: popreturn inlen
popreturn input_0
