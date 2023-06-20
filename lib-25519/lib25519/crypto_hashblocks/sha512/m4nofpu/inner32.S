// linker define inner

# qhasm: int32 input_0

# qhasm: int32 input_1

# qhasm: int32 input_2

# qhasm: int32 input_3

# qhasm: stack32 input_4

# qhasm: stack32 input_5

# qhasm: stack32 input_6

# qhasm: stack32 input_7

# qhasm: int32 caller_r4

# qhasm: int32 caller_r5

# qhasm: int32 caller_r6

# qhasm: int32 caller_r7

# qhasm: int32 caller_r8

# qhasm: int32 caller_r9

# qhasm: int32 caller_r10

# qhasm: int32 caller_r11

# qhasm: int32 caller_r12

# qhasm: int32 caller_r14

# qhasm: startcode
.text
.arch armv7
.syntax unified

# qhasm: int32 two13

# qhasm: int32 two23

# qhasm: int32 two24

# qhasm: int32 two25

# qhasm: int32 lotmp

# qhasm: int32 lotmp2

# qhasm: int32 hitmp

# qhasm: int32 hitmp2

# qhasm: int32 lou0

# qhasm: int32 lou1

# qhasm: int32 lou2

# qhasm: int32 lou3

# qhasm: int32 lou4

# qhasm: int32 lou5

# qhasm: int32 hiu0

# qhasm: int32 hiu1

# qhasm: int32 hiu2

# qhasm: int32 hiu3

# qhasm: int32 hiu4

# qhasm: int32 hiu5

# qhasm: stack32 hid0

# qhasm: stack32 hid1

# qhasm: stack32 hid2

# qhasm: stack32 hid3

# qhasm: stack32 hid4

# qhasm: stack32 hid5

# qhasm: stack32 hid6

# qhasm: stack32 hid7

# qhasm: stack32 hid8

# qhasm: stack32 hid9

# qhasm: stack32 hid10

# qhasm: stack32 hid11

# qhasm: stack32 hid12

# qhasm: stack32 hid13

# qhasm: stack32 hid14

# qhasm: stack32 hid15

# qhasm: stack32 lod0

# qhasm: stack32 lod1

# qhasm: stack32 lod2

# qhasm: stack32 lod3

# qhasm: stack32 lod4

# qhasm: stack32 lod5

# qhasm: stack32 lod6

# qhasm: stack32 lod7

# qhasm: stack32 lod8

# qhasm: stack32 lod9

# qhasm: stack32 lod10

# qhasm: stack32 lod11

# qhasm: stack32 lod12

# qhasm: stack32 lod13

# qhasm: stack32 lod14

# qhasm: stack32 lod15

# qhasm: stack32 him0

# qhasm: stack32 him1

# qhasm: stack32 him2

# qhasm: stack32 him3

# qhasm: stack32 him4

# qhasm: stack32 him5

# qhasm: stack32 him6

# qhasm: stack32 him7

# qhasm: stack32 him8

# qhasm: stack32 him9

# qhasm: stack32 him10

# qhasm: stack32 him11

# qhasm: stack32 him12

# qhasm: stack32 him13

# qhasm: stack32 him14

# qhasm: stack32 him15

# qhasm: stack32 lom0

# qhasm: stack32 lom1

# qhasm: stack32 lom2

# qhasm: stack32 lom3

# qhasm: stack32 lom4

# qhasm: stack32 lom5

# qhasm: stack32 lom6

# qhasm: stack32 lom7

# qhasm: stack32 lom8

# qhasm: stack32 lom9

# qhasm: stack32 lom10

# qhasm: stack32 lom11

# qhasm: stack32 lom12

# qhasm: stack32 lom13

# qhasm: stack32 lom14

# qhasm: stack32 lom15

# qhasm: stack32 o0

# qhasm: stack32 o1

# qhasm: stack32 o2

# qhasm: stack32 o3

# qhasm: stack32 o4

# qhasm: rpushenter CRYPTO_SHARED_NAMESPACE(inner)
.p2align 2
.global CRYPTO_SHARED_NAMESPACE(inner)
.type CRYPTO_SHARED_NAMESPACE(inner),%function
.thumb
.thumb_func
CRYPTO_SHARED_NAMESPACE(inner):
push {r4,r5,r6,r7,r8,r9,r10,r11,r14}
sub.w sp,sp,#288

# qhasm: o0 = input_0
# asm 1: str <input_0=int32#1,>o0=stack32#1
# asm 2: str <input_0=r0,>o0=[sp,#0]
# copy-collector input: str r0,[sp,#0]

# qhasm: o1 = input_1
# asm 1: str <input_1=int32#2,>o1=stack32#2
# asm 2: str <input_1=r1,>o1=[sp,#4]
# copy-collector input: str r1,[sp,#4]

# qhasm: input_1 = input_2 - 128
# asm 1: sub >input_1=int32#2,<input_2=int32#3,#128
# asm 2: sub >input_1=r1,<input_2=r2,#128
# copy-collector output starts
strd r0,r1,[sp,#0]
# copy-collector output ends
sub r1,r2,#128

# qhasm: o2 = input_1
# asm 1: str <input_1=int32#2,>o2=stack32#3
# asm 2: str <input_1=r1,>o2=[sp,#8]
# copy-collector input: str r1,[sp,#8]

# qhasm: o3 = input_3
# asm 1: str <input_3=int32#4,>o3=stack32#4
# asm 2: str <input_3=r3,>o3=[sp,#12]
# copy-collector input: str r3,[sp,#12]

# qhasm: hiu0 = mem32[input_0]
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1]
# asm 2: ldr >hiu0=r1,[<input_0=r0]
# copy-collector input: ldr r1,[r0]

# qhasm: lou0 = mem32[input_0+4]
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1,#4]
# asm 2: ldr >lou0=r2,[<input_0=r0,#4]
# copy-collector input: ldr r2,[r0,#4]

# qhasm: hiu1 = mem32[input_0+8]
# asm 1: ldr >hiu1=int32#4,[<input_0=int32#1,#8]
# asm 2: ldr >hiu1=r3,[<input_0=r0,#8]
# copy-collector input: ldr r3,[r0,#8]

# qhasm: lou1 = mem32[input_0+12]
# asm 1: ldr >lou1=int32#5,[<input_0=int32#1,#12]
# asm 2: ldr >lou1=r4,[<input_0=r0,#12]
# copy-collector input: ldr r4,[r0,#12]

# qhasm: hiu2 = mem32[input_0+16]
# asm 1: ldr >hiu2=int32#6,[<input_0=int32#1,#16]
# asm 2: ldr >hiu2=r5,[<input_0=r0,#16]
# copy-collector input: ldr r5,[r0,#16]

# qhasm: lou2 = mem32[input_0+20]
# asm 1: ldr >lou2=int32#7,[<input_0=int32#1,#20]
# asm 2: ldr >lou2=r6,[<input_0=r0,#20]
# copy-collector input: ldr r6,[r0,#20]

# qhasm: hiu3 = mem32[input_0+24]
# asm 1: ldr >hiu3=int32#8,[<input_0=int32#1,#24]
# asm 2: ldr >hiu3=r7,[<input_0=r0,#24]
# copy-collector input: ldr r7,[r0,#24]

# qhasm: lou3 = mem32[input_0+28]
# asm 1: ldr >lou3=int32#9,[<input_0=int32#1,#28]
# asm 2: ldr >lou3=r8,[<input_0=r0,#28]
# copy-collector input: ldr r8,[r0,#28]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
strd r1,r3,[sp,#8]
ldr r1,[r0]
ldr r2,[r0,#4]
ldr r3,[r0,#8]
ldr r4,[r0,#12]
ldr r5,[r0,#16]
ldr r6,[r0,#20]
ldr.w r7,[r0,#24]
ldr r8,[r0,#28]
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#4,<hiu1=int32#4
# asm 2: rev >hiu1=r3,<hiu1=r3
rev r3,r3

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#6,<hiu2=int32#6
# asm 2: rev >hiu2=r5,<hiu2=r5
rev r5,r5

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#9,<lou3=int32#9
# asm 2: rev >lou3=r8,<lou3=r8
rev r8,r8

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#8,<hiu3=int32#8
# asm 2: rev >hiu3=r7,<hiu3=r7
rev.w r7,r7

# qhasm: lom0 = lou0
# asm 1: str <lou0=int32#3,>lom0=stack32#5
# asm 2: str <lou0=r2,>lom0=[sp,#16]
# copy-collector input: str r2,[sp,#16]

# qhasm: him0 = hiu0
# asm 1: str <hiu0=int32#2,>him0=stack32#6
# asm 2: str <hiu0=r1,>him0=[sp,#20]
# copy-collector input: str r1,[sp,#20]

# qhasm: lom1 = lou1
# asm 1: str <lou1=int32#5,>lom1=stack32#7
# asm 2: str <lou1=r4,>lom1=[sp,#24]
# copy-collector input: str r4,[sp,#24]

# qhasm: him1 = hiu1
# asm 1: str <hiu1=int32#4,>him1=stack32#8
# asm 2: str <hiu1=r3,>him1=[sp,#28]
# copy-collector input: str r3,[sp,#28]

# qhasm: lom2 = lou2
# asm 1: str <lou2=int32#7,>lom2=stack32#9
# asm 2: str <lou2=r6,>lom2=[sp,#32]
# copy-collector input: str r6,[sp,#32]

# qhasm: him2 = hiu2
# asm 1: str <hiu2=int32#6,>him2=stack32#10
# asm 2: str <hiu2=r5,>him2=[sp,#36]
# copy-collector input: str r5,[sp,#36]

# qhasm: lom3 = lou3
# asm 1: str <lou3=int32#9,>lom3=stack32#11
# asm 2: str <lou3=r8,>lom3=[sp,#40]
# copy-collector input: str r8,[sp,#40]

# qhasm: him3 = hiu3
# asm 1: str <hiu3=int32#8,>him3=stack32#12
# asm 2: str <hiu3=r7,>him3=[sp,#44]
# copy-collector input: str r7,[sp,#44]

# qhasm: lod0 = lou0
# asm 1: str <lou0=int32#3,>lod0=stack32#13
# asm 2: str <lou0=r2,>lod0=[sp,#48]
# copy-collector input: str r2,[sp,#48]

# qhasm: hid0 = hiu0
# asm 1: str <hiu0=int32#2,>hid0=stack32#14
# asm 2: str <hiu0=r1,>hid0=[sp,#52]
# copy-collector input: str r1,[sp,#52]

# qhasm: lod1 = lou1
# asm 1: str <lou1=int32#5,>lod1=stack32#15
# asm 2: str <lou1=r4,>lod1=[sp,#56]
# copy-collector input: str r4,[sp,#56]

# qhasm: hid1 = hiu1
# asm 1: str <hiu1=int32#4,>hid1=stack32#16
# asm 2: str <hiu1=r3,>hid1=[sp,#60]
# copy-collector input: str r3,[sp,#60]

# qhasm: lod2 = lou2
# asm 1: str <lou2=int32#7,>lod2=stack32#17
# asm 2: str <lou2=r6,>lod2=[sp,#64]
# copy-collector input: str r6,[sp,#64]

# qhasm: hid2 = hiu2
# asm 1: str <hiu2=int32#6,>hid2=stack32#18
# asm 2: str <hiu2=r5,>hid2=[sp,#68]
# copy-collector input: str r5,[sp,#68]

# qhasm: lod3 = lou3
# asm 1: str <lou3=int32#9,>lod3=stack32#19
# asm 2: str <lou3=r8,>lod3=[sp,#72]
# copy-collector input: str r8,[sp,#72]

# qhasm: hid3 = hiu3
# asm 1: str <hiu3=int32#8,>hid3=stack32#20
# asm 2: str <hiu3=r7,>hid3=[sp,#76]
# copy-collector input: str r7,[sp,#76]

# qhasm: hiu0 = mem32[input_0+32]
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1,#32]
# asm 2: ldr >hiu0=r1,[<input_0=r0,#32]
# copy-collector input: ldr r1,[r0,#32]

# qhasm: lou0 = mem32[input_0+36]
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1,#36]
# asm 2: ldr >lou0=r2,[<input_0=r0,#36]
# copy-collector input: ldr r2,[r0,#36]

# qhasm: hiu1 = mem32[input_0+40]
# asm 1: ldr >hiu1=int32#4,[<input_0=int32#1,#40]
# asm 2: ldr >hiu1=r3,[<input_0=r0,#40]
# copy-collector input: ldr r3,[r0,#40]

# qhasm: lou1 = mem32[input_0+44]
# asm 1: ldr >lou1=int32#5,[<input_0=int32#1,#44]
# asm 2: ldr >lou1=r4,[<input_0=r0,#44]
# copy-collector input: ldr r4,[r0,#44]

# qhasm: hiu2 = mem32[input_0+48]
# asm 1: ldr >hiu2=int32#6,[<input_0=int32#1,#48]
# asm 2: ldr >hiu2=r5,[<input_0=r0,#48]
# copy-collector input: ldr r5,[r0,#48]

# qhasm: lou2 = mem32[input_0+52]
# asm 1: ldr >lou2=int32#7,[<input_0=int32#1,#52]
# asm 2: ldr >lou2=r6,[<input_0=r0,#52]
# copy-collector input: ldr r6,[r0,#52]

# qhasm: hiu3 = mem32[input_0+56]
# asm 1: ldr >hiu3=int32#8,[<input_0=int32#1,#56]
# asm 2: ldr >hiu3=r7,[<input_0=r0,#56]
# copy-collector input: ldr r7,[r0,#56]

# qhasm: lou3 = mem32[input_0+60]
# asm 1: ldr >lou3=int32#1,[<input_0=int32#1,#60]
# asm 2: ldr >lou3=r0,[<input_0=r0,#60]
# copy-collector input: ldr r0,[r0,#60]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
strd r2,r1,[sp,#16]
strd r4,r3,[sp,#24]
strd r6,r5,[sp,#32]
strd r8,r7,[sp,#40]
strd r2,r1,[sp,#48]
strd r4,r3,[sp,#56]
strd r6,r5,[sp,#64]
strd r8,r7,[sp,#72]
ldr r1,[r0,#32]
ldr r2,[r0,#36]
ldr r3,[r0,#40]
ldr r4,[r0,#44]
ldr r5,[r0,#48]
ldr r6,[r0,#52]
ldr r7,[r0,#56]
ldr r0,[r0,#60]
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#4,<hiu1=int32#4
# asm 2: rev >hiu1=r3,<hiu1=r3
rev r3,r3

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#6,<hiu2=int32#6
# asm 2: rev >hiu2=r5,<hiu2=r5
rev r5,r5

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#1,<lou3=int32#1
# asm 2: rev >lou3=r0,<lou3=r0
rev r0,r0

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#8,<hiu3=int32#8
# asm 2: rev >hiu3=r7,<hiu3=r7
rev r7,r7

# qhasm: lom4 = lou0
# asm 1: str <lou0=int32#3,>lom4=stack32#21
# asm 2: str <lou0=r2,>lom4=[sp,#80]
# copy-collector input: str r2,[sp,#80]

# qhasm: him4 = hiu0
# asm 1: str <hiu0=int32#2,>him4=stack32#22
# asm 2: str <hiu0=r1,>him4=[sp,#84]
# copy-collector input: str r1,[sp,#84]

# qhasm: lom5 = lou1
# asm 1: str <lou1=int32#5,>lom5=stack32#23
# asm 2: str <lou1=r4,>lom5=[sp,#88]
# copy-collector input: str r4,[sp,#88]

# qhasm: him5 = hiu1
# asm 1: str <hiu1=int32#4,>him5=stack32#24
# asm 2: str <hiu1=r3,>him5=[sp,#92]
# copy-collector input: str r3,[sp,#92]

# qhasm: lom6 = lou2
# asm 1: str <lou2=int32#7,>lom6=stack32#25
# asm 2: str <lou2=r6,>lom6=[sp,#96]
# copy-collector input: str r6,[sp,#96]

# qhasm: him6 = hiu2
# asm 1: str <hiu2=int32#6,>him6=stack32#26
# asm 2: str <hiu2=r5,>him6=[sp,#100]
# copy-collector input: str r5,[sp,#100]

# qhasm: lom7 = lou3
# asm 1: str <lou3=int32#1,>lom7=stack32#27
# asm 2: str <lou3=r0,>lom7=[sp,#104]
# copy-collector input: str r0,[sp,#104]

# qhasm: him7 = hiu3
# asm 1: str <hiu3=int32#8,>him7=stack32#28
# asm 2: str <hiu3=r7,>him7=[sp,#108]
# copy-collector input: str r7,[sp,#108]

# qhasm: lod4 = lou0
# asm 1: str <lou0=int32#3,>lod4=stack32#29
# asm 2: str <lou0=r2,>lod4=[sp,#112]
# copy-collector input: str r2,[sp,#112]

# qhasm: hid4 = hiu0
# asm 1: str <hiu0=int32#2,>hid4=stack32#30
# asm 2: str <hiu0=r1,>hid4=[sp,#116]
# copy-collector input: str r1,[sp,#116]

# qhasm: lod5 = lou1
# asm 1: str <lou1=int32#5,>lod5=stack32#31
# asm 2: str <lou1=r4,>lod5=[sp,#120]
# copy-collector input: str r4,[sp,#120]

# qhasm: hid5 = hiu1
# asm 1: str <hiu1=int32#4,>hid5=stack32#32
# asm 2: str <hiu1=r3,>hid5=[sp,#124]
# copy-collector input: str r3,[sp,#124]

# qhasm: lod6 = lou2
# asm 1: str <lou2=int32#7,>lod6=stack32#33
# asm 2: str <lou2=r6,>lod6=[sp,#128]
# copy-collector input: str r6,[sp,#128]

# qhasm: hid6 = hiu2
# asm 1: str <hiu2=int32#6,>hid6=stack32#34
# asm 2: str <hiu2=r5,>hid6=[sp,#132]
# copy-collector input: str r5,[sp,#132]

# qhasm: lod7 = lou3
# asm 1: str <lou3=int32#1,>lod7=stack32#35
# asm 2: str <lou3=r0,>lod7=[sp,#136]
# copy-collector input: str r0,[sp,#136]

# qhasm: hid7 = hiu3
# asm 1: str <hiu3=int32#8,>hid7=stack32#36
# asm 2: str <hiu3=r7,>hid7=[sp,#140]
# copy-collector input: str r7,[sp,#140]

# qhasm: mainloop:
# copy-collector output starts
strd r2,r1,[sp,#80]
strd r4,r3,[sp,#88]
strd r6,r5,[sp,#96]
strd r0,r7,[sp,#104]
strd r2,r1,[sp,#112]
strd r4,r3,[sp,#120]
strd r6,r5,[sp,#128]
strd r0,r7,[sp,#136]
# copy-collector output ends
._mainloop:

# qhasm: input_0 = o1
# asm 1: ldr >input_0=int32#1,<o1=stack32#2
# asm 2: ldr >input_0=r0,<o1=[sp,#4]
# copy-collector input: ldr r0,[sp,#4]

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1],#4
# asm 2: ldr >hiu0=r1,[<input_0=r0],#4
# copy-collector input: ldr r1,[r0],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1],#4
# asm 2: ldr >lou0=r2,[<input_0=r0],#4
# copy-collector input: ldr r2,[r0],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#4,[<input_0=int32#1],#4
# asm 2: ldr >hiu1=r3,[<input_0=r0],#4
# copy-collector input: ldr r3,[r0],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#5,[<input_0=int32#1],#4
# asm 2: ldr >lou1=r4,[<input_0=r0],#4
# copy-collector input: ldr r4,[r0],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#6,[<input_0=int32#1],#4
# asm 2: ldr >hiu2=r5,[<input_0=r0],#4
# copy-collector input: ldr r5,[r0],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#7,[<input_0=int32#1],#4
# asm 2: ldr >lou2=r6,[<input_0=r0],#4
# copy-collector input: ldr r6,[r0],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#8,[<input_0=int32#1],#4
# asm 2: ldr >hiu3=r7,[<input_0=r0],#4
# copy-collector input: ldr r7,[r0],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#9,[<input_0=int32#1],#4
# asm 2: ldr >lou3=r8,[<input_0=r0],#4
# copy-collector input: ldr r8,[r0],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
ldr.w r0,[sp,#4]
ldr r1,[r0],#4
ldr r2,[r0],#4
ldr r3,[r0],#4
ldr r4,[r0],#4
ldr r5,[r0],#4
ldr r6,[r0],#4
ldr r7,[r0],#4
ldr r8,[r0],#4
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#4,<hiu1=int32#4
# asm 2: rev >hiu1=r3,<hiu1=r3
rev r3,r3

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#6,<hiu2=int32#6
# asm 2: rev >hiu2=r5,<hiu2=r5
rev r5,r5

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#9,<lou3=int32#9
# asm 2: rev >lou3=r8,<lou3=r8
rev r8,r8

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#8,<hiu3=int32#8
# asm 2: rev >hiu3=r7,<hiu3=r7
rev.w r7,r7

# qhasm: lod8 = lou0
# asm 1: str <lou0=int32#3,>lod8=stack32#37
# asm 2: str <lou0=r2,>lod8=[sp,#144]
# copy-collector input: str r2,[sp,#144]

# qhasm: hid8 = hiu0
# asm 1: str <hiu0=int32#2,>hid8=stack32#38
# asm 2: str <hiu0=r1,>hid8=[sp,#148]
# copy-collector input: str r1,[sp,#148]

# qhasm: lod9 = lou1
# asm 1: str <lou1=int32#5,>lod9=stack32#39
# asm 2: str <lou1=r4,>lod9=[sp,#152]
# copy-collector input: str r4,[sp,#152]

# qhasm: hid9 = hiu1
# asm 1: str <hiu1=int32#4,>hid9=stack32#40
# asm 2: str <hiu1=r3,>hid9=[sp,#156]
# copy-collector input: str r3,[sp,#156]

# qhasm: lod10 = lou2
# asm 1: str <lou2=int32#7,>lod10=stack32#41
# asm 2: str <lou2=r6,>lod10=[sp,#160]
# copy-collector input: str r6,[sp,#160]

# qhasm: hid10 = hiu2
# asm 1: str <hiu2=int32#6,>hid10=stack32#42
# asm 2: str <hiu2=r5,>hid10=[sp,#164]
# copy-collector input: str r5,[sp,#164]

# qhasm: lod11 = lou3
# asm 1: str <lou3=int32#9,>lod11=stack32#43
# asm 2: str <lou3=r8,>lod11=[sp,#168]
# copy-collector input: str r8,[sp,#168]

# qhasm: hid11 = hiu3
# asm 1: str <hiu3=int32#8,>hid11=stack32#44
# asm 2: str <hiu3=r7,>hid11=[sp,#172]
# copy-collector input: str r7,[sp,#172]

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1],#4
# asm 2: ldr >hiu0=r1,[<input_0=r0],#4
# copy-collector input: ldr r1,[r0],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1],#4
# asm 2: ldr >lou0=r2,[<input_0=r0],#4
# copy-collector input: ldr r2,[r0],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#4,[<input_0=int32#1],#4
# asm 2: ldr >hiu1=r3,[<input_0=r0],#4
# copy-collector input: ldr r3,[r0],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#5,[<input_0=int32#1],#4
# asm 2: ldr >lou1=r4,[<input_0=r0],#4
# copy-collector input: ldr r4,[r0],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#6,[<input_0=int32#1],#4
# asm 2: ldr >hiu2=r5,[<input_0=r0],#4
# copy-collector input: ldr r5,[r0],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#7,[<input_0=int32#1],#4
# asm 2: ldr >lou2=r6,[<input_0=r0],#4
# copy-collector input: ldr r6,[r0],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#8,[<input_0=int32#1],#4
# asm 2: ldr >hiu3=r7,[<input_0=r0],#4
# copy-collector input: ldr r7,[r0],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#9,[<input_0=int32#1],#4
# asm 2: ldr >lou3=r8,[<input_0=r0],#4
# copy-collector input: ldr r8,[r0],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
strd r2,r1,[sp,#144]
strd r4,r3,[sp,#152]
strd r6,r5,[sp,#160]
strd r8,r7,[sp,#168]
ldr r1,[r0],#4
ldr r2,[r0],#4
ldr r3,[r0],#4
ldr r4,[r0],#4
ldr r5,[r0],#4
ldr r6,[r0],#4
ldr r7,[r0],#4
ldr r8,[r0],#4
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#4,<hiu1=int32#4
# asm 2: rev >hiu1=r3,<hiu1=r3
rev r3,r3

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#6,<hiu2=int32#6
# asm 2: rev >hiu2=r5,<hiu2=r5
rev r5,r5

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#9,<lou3=int32#9
# asm 2: rev >lou3=r8,<lou3=r8
rev r8,r8

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#8,<hiu3=int32#8
# asm 2: rev >hiu3=r7,<hiu3=r7
rev.w r7,r7

# qhasm: lod12 = lou0
# asm 1: str <lou0=int32#3,>lod12=stack32#45
# asm 2: str <lou0=r2,>lod12=[sp,#176]
# copy-collector input: str r2,[sp,#176]

# qhasm: hid12 = hiu0
# asm 1: str <hiu0=int32#2,>hid12=stack32#46
# asm 2: str <hiu0=r1,>hid12=[sp,#180]
# copy-collector input: str r1,[sp,#180]

# qhasm: lod13 = lou1
# asm 1: str <lou1=int32#5,>lod13=stack32#47
# asm 2: str <lou1=r4,>lod13=[sp,#184]
# copy-collector input: str r4,[sp,#184]

# qhasm: hid13 = hiu1
# asm 1: str <hiu1=int32#4,>hid13=stack32#48
# asm 2: str <hiu1=r3,>hid13=[sp,#188]
# copy-collector input: str r3,[sp,#188]

# qhasm: lod14 = lou2
# asm 1: str <lou2=int32#7,>lod14=stack32#49
# asm 2: str <lou2=r6,>lod14=[sp,#192]
# copy-collector input: str r6,[sp,#192]

# qhasm: hid14 = hiu2
# asm 1: str <hiu2=int32#6,>hid14=stack32#50
# asm 2: str <hiu2=r5,>hid14=[sp,#196]
# copy-collector input: str r5,[sp,#196]

# qhasm: lod15 = lou3
# asm 1: str <lou3=int32#9,>lod15=stack32#51
# asm 2: str <lou3=r8,>lod15=[sp,#200]
# copy-collector input: str r8,[sp,#200]

# qhasm: hid15 = hiu3
# asm 1: str <hiu3=int32#8,>hid15=stack32#52
# asm 2: str <hiu3=r7,>hid15=[sp,#204]
# copy-collector input: str r7,[sp,#204]

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1],#4
# asm 2: ldr >hiu0=r1,[<input_0=r0],#4
# copy-collector input: ldr r1,[r0],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1],#4
# asm 2: ldr >lou0=r2,[<input_0=r0],#4
# copy-collector input: ldr r2,[r0],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#4,[<input_0=int32#1],#4
# asm 2: ldr >hiu1=r3,[<input_0=r0],#4
# copy-collector input: ldr r3,[r0],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#5,[<input_0=int32#1],#4
# asm 2: ldr >lou1=r4,[<input_0=r0],#4
# copy-collector input: ldr r4,[r0],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#6,[<input_0=int32#1],#4
# asm 2: ldr >hiu2=r5,[<input_0=r0],#4
# copy-collector input: ldr r5,[r0],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#7,[<input_0=int32#1],#4
# asm 2: ldr >lou2=r6,[<input_0=r0],#4
# copy-collector input: ldr r6,[r0],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#8,[<input_0=int32#1],#4
# asm 2: ldr >hiu3=r7,[<input_0=r0],#4
# copy-collector input: ldr r7,[r0],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#9,[<input_0=int32#1],#4
# asm 2: ldr >lou3=r8,[<input_0=r0],#4
# copy-collector input: ldr r8,[r0],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
strd r2,r1,[sp,#176]
strd r4,r3,[sp,#184]
strd r6,r5,[sp,#192]
strd r8,r7,[sp,#200]
ldr r1,[r0],#4
ldr r2,[r0],#4
ldr r3,[r0],#4
ldr r4,[r0],#4
ldr r5,[r0],#4
ldr r6,[r0],#4
ldr r7,[r0],#4
ldr r8,[r0],#4
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#4,<hiu1=int32#4
# asm 2: rev >hiu1=r3,<hiu1=r3
rev r3,r3

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#6,<hiu2=int32#6
# asm 2: rev >hiu2=r5,<hiu2=r5
rev r5,r5

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#9,<lou3=int32#9
# asm 2: rev >lou3=r8,<lou3=r8
rev r8,r8

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#8,<hiu3=int32#8
# asm 2: rev >hiu3=r7,<hiu3=r7
rev.w r7,r7

# qhasm: lom8 = lou0
# asm 1: str <lou0=int32#3,>lom8=stack32#53
# asm 2: str <lou0=r2,>lom8=[sp,#208]
# copy-collector input: str r2,[sp,#208]

# qhasm: him8 = hiu0
# asm 1: str <hiu0=int32#2,>him8=stack32#54
# asm 2: str <hiu0=r1,>him8=[sp,#212]
# copy-collector input: str r1,[sp,#212]

# qhasm: lom9 = lou1
# asm 1: str <lou1=int32#5,>lom9=stack32#55
# asm 2: str <lou1=r4,>lom9=[sp,#216]
# copy-collector input: str r4,[sp,#216]

# qhasm: him9 = hiu1
# asm 1: str <hiu1=int32#4,>him9=stack32#56
# asm 2: str <hiu1=r3,>him9=[sp,#220]
# copy-collector input: str r3,[sp,#220]

# qhasm: lom10 = lou2
# asm 1: str <lou2=int32#7,>lom10=stack32#57
# asm 2: str <lou2=r6,>lom10=[sp,#224]
# copy-collector input: str r6,[sp,#224]

# qhasm: him10 = hiu2
# asm 1: str <hiu2=int32#6,>him10=stack32#58
# asm 2: str <hiu2=r5,>him10=[sp,#228]
# copy-collector input: str r5,[sp,#228]

# qhasm: lom11 = lou3
# asm 1: str <lou3=int32#9,>lom11=stack32#59
# asm 2: str <lou3=r8,>lom11=[sp,#232]
# copy-collector input: str r8,[sp,#232]

# qhasm: him11 = hiu3
# asm 1: str <hiu3=int32#8,>him11=stack32#60
# asm 2: str <hiu3=r7,>him11=[sp,#236]
# copy-collector input: str r7,[sp,#236]

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1],#4
# asm 2: ldr >hiu0=r1,[<input_0=r0],#4
# copy-collector input: ldr r1,[r0],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1],#4
# asm 2: ldr >lou0=r2,[<input_0=r0],#4
# copy-collector input: ldr r2,[r0],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#4,[<input_0=int32#1],#4
# asm 2: ldr >hiu1=r3,[<input_0=r0],#4
# copy-collector input: ldr r3,[r0],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#5,[<input_0=int32#1],#4
# asm 2: ldr >lou1=r4,[<input_0=r0],#4
# copy-collector input: ldr r4,[r0],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#6,[<input_0=int32#1],#4
# asm 2: ldr >hiu2=r5,[<input_0=r0],#4
# copy-collector input: ldr r5,[r0],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#7,[<input_0=int32#1],#4
# asm 2: ldr >lou2=r6,[<input_0=r0],#4
# copy-collector input: ldr r6,[r0],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#8,[<input_0=int32#1],#4
# asm 2: ldr >hiu3=r7,[<input_0=r0],#4
# copy-collector input: ldr r7,[r0],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#9,[<input_0=int32#1],#4
# asm 2: ldr >lou3=r8,[<input_0=r0],#4
# copy-collector input: ldr r8,[r0],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
strd r2,r1,[sp,#208]
strd r4,r3,[sp,#216]
strd r6,r5,[sp,#224]
strd r8,r7,[sp,#232]
ldr r1,[r0],#4
ldr r2,[r0],#4
ldr r3,[r0],#4
ldr r4,[r0],#4
ldr r5,[r0],#4
ldr r6,[r0],#4
ldr r7,[r0],#4
ldr r8,[r0],#4
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#4,<hiu1=int32#4
# asm 2: rev >hiu1=r3,<hiu1=r3
rev r3,r3

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#6,<hiu2=int32#6
# asm 2: rev >hiu2=r5,<hiu2=r5
rev r5,r5

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#9,<lou3=int32#9
# asm 2: rev >lou3=r8,<lou3=r8
rev r8,r8

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#8,<hiu3=int32#8
# asm 2: rev >hiu3=r7,<hiu3=r7
rev.w r7,r7

# qhasm: lom12 = lou0
# asm 1: str <lou0=int32#3,>lom12=stack32#61
# asm 2: str <lou0=r2,>lom12=[sp,#240]
# copy-collector input: str r2,[sp,#240]

# qhasm: him12 = hiu0
# asm 1: str <hiu0=int32#2,>him12=stack32#62
# asm 2: str <hiu0=r1,>him12=[sp,#244]
# copy-collector input: str r1,[sp,#244]

# qhasm: lom13 = lou1
# asm 1: str <lou1=int32#5,>lom13=stack32#63
# asm 2: str <lou1=r4,>lom13=[sp,#248]
# copy-collector input: str r4,[sp,#248]

# qhasm: him13 = hiu1
# asm 1: str <hiu1=int32#4,>him13=stack32#64
# asm 2: str <hiu1=r3,>him13=[sp,#252]
# copy-collector input: str r3,[sp,#252]

# qhasm: lom14 = lou2
# asm 1: str <lou2=int32#7,>lom14=stack32#65
# asm 2: str <lou2=r6,>lom14=[sp,#256]
# copy-collector input: str r6,[sp,#256]

# qhasm: him14 = hiu2
# asm 1: str <hiu2=int32#6,>him14=stack32#66
# asm 2: str <hiu2=r5,>him14=[sp,#260]
# copy-collector input: str r5,[sp,#260]

# qhasm: lom15 = lou3
# asm 1: str <lou3=int32#9,>lom15=stack32#67
# asm 2: str <lou3=r8,>lom15=[sp,#264]
# copy-collector input: str r8,[sp,#264]

# qhasm: him15 = hiu3
# asm 1: str <hiu3=int32#8,>him15=stack32#68
# asm 2: str <hiu3=r7,>him15=[sp,#268]
# copy-collector input: str r7,[sp,#268]

# qhasm: o1 = input_0
# asm 1: str <input_0=int32#1,>o1=stack32#2
# asm 2: str <input_0=r0,>o1=[sp,#4]
# copy-collector input: str r0,[sp,#4]

# qhasm: input_0 = 80 simple
# asm 1: mov >input_0=int32#1,80
# asm 2: mov >input_0=r0,80
# copy-collector output starts
strd r2,r1,[sp,#240]
strd r4,r3,[sp,#248]
strd r6,r5,[sp,#256]
strd r8,r7,[sp,#264]
str r0,[sp,#4]
# copy-collector output ends
mov r0,80

# qhasm: o4 = input_0
# asm 1: str <input_0=int32#1,>o4=stack32#69
# asm 2: str <input_0=r0,>o4=[sp,#272]
# copy-collector input: str r0,[sp,#272]

# qhasm: innerloop:
# copy-collector output starts
str r0,[sp,#272]
# copy-collector output ends
._innerloop:

# qhasm: input_0 = o3
# asm 1: ldr >input_0=int32#1,<o3=stack32#4
# asm 2: ldr >input_0=r0,<o3=[sp,#12]
# copy-collector input: ldr r0,[sp,#12]

# qhasm: lou0 = lod3
# asm 1: ldr >lou0=int32#2,<lod3=stack32#19
# asm 2: ldr >lou0=r1,<lod3=[sp,#72]
# copy-collector input: ldr r1,[sp,#72]

# qhasm: hiu0 = hid3
# asm 1: ldr >hiu0=int32#3,<hid3=stack32#20
# asm 2: ldr >hiu0=r2,<hid3=[sp,#76]
# copy-collector input: ldr r2,[sp,#76]

# qhasm: lou1 = lod4
# asm 1: ldr >lou1=int32#4,<lod4=stack32#29
# asm 2: ldr >lou1=r3,<lod4=[sp,#112]
# copy-collector input: ldr r3,[sp,#112]

# qhasm: hiu1 = hid4
# asm 1: ldr >hiu1=int32#5,<hid4=stack32#30
# asm 2: ldr >hiu1=r4,<hid4=[sp,#116]
# copy-collector input: ldr r4,[sp,#116]

# qhasm: lou2 = lod5
# asm 1: ldr >lou2=int32#6,<lod5=stack32#31
# asm 2: ldr >lou2=r5,<lod5=[sp,#120]
# copy-collector input: ldr r5,[sp,#120]

# qhasm: hiu2 = hid5
# asm 1: ldr >hiu2=int32#7,<hid5=stack32#32
# asm 2: ldr >hiu2=r6,<hid5=[sp,#124]
# copy-collector input: ldr r6,[sp,#124]

# qhasm: lou3 = lod6
# asm 1: ldr >lou3=int32#8,<lod6=stack32#33
# asm 2: ldr >lou3=r7,<lod6=[sp,#128]
# copy-collector input: ldr r7,[sp,#128]

# qhasm: hiu3 = hid6
# asm 1: ldr >hiu3=int32#9,<hid6=stack32#34
# asm 2: ldr >hiu3=r8,<hid6=[sp,#132]
# copy-collector input: ldr r8,[sp,#132]

# qhasm: lou4 = lod7
# asm 1: ldr >lou4=int32#10,<lod7=stack32#35
# asm 2: ldr >lou4=r9,<lod7=[sp,#136]
# copy-collector input: ldr r9,[sp,#136]

# qhasm: hiu4 = hid7
# asm 1: ldr >hiu4=int32#11,<hid7=stack32#36
# asm 2: ldr >hiu4=r10,<hid7=[sp,#140]
# copy-collector input: ldr r10,[sp,#140]

# qhasm: lou5 = lod8
# asm 1: ldr >lou5=int32#12,<lod8=stack32#37
# asm 2: ldr >lou5=r11,<lod8=[sp,#144]
# copy-collector input: ldr r11,[sp,#144]

# qhasm: hiu5 = hid8
# asm 1: ldr >hiu5=int32#13,<hid8=stack32#38
# asm 2: ldr >hiu5=r12,<hid8=[sp,#148]
# copy-collector input: ldr r12,[sp,#148]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
ldr r0,[sp,#12]
ldr r1,[sp,#72]
ldr r2,[sp,#76]
ldr r3,[sp,#112]
ldr r4,[sp,#116]
ldr r5,[sp,#120]
ldr r6,[sp,#124]
ldr r7,[sp,#128]
ldr r8,[sp,#132]
ldr r9,[sp,#136]
ldr r10,[sp,#140]
ldr r11,[sp,#144]
ldr r12,[sp,#148]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lou5=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lou5=r11
adds r9,r9,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#11,<hiu4=int32#11,<hiu5=int32#13
# asm 2: adc >hiu4=r10,<hiu4=r10,<hiu5=r12
adc r10,r10,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lotmp=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r9,r9,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#11,<hiu4=int32#11,<hitmp=int32#13
# asm 2: adc >hiu4=r10,<hiu4=r10,<hitmp=r12
adc r10,r10,r12

# qhasm: hitmp lotmp = lou1 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou1=int32#4,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou1=r3,<two23=r14
umull r12,r11,r3,r14

# qhasm: lotmp hitmp += hiu1 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu1=int32#5,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu1=r4,<two23=r14
umlal r11,r12,r4,r14

# qhasm: lotmp ^= (lou1 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou1=int32#4,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou1=r3,LSR #18
eor r12,r12,r3,LSR #18

# qhasm: lotmp ^= (hiu1 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu1=int32#5,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu1=r4,LSL #14
eor r12,r12,r4,LSL #14

# qhasm: lotmp ^= (lou1 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou1=int32#4,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou1=r3,LSR #14
eor r12,r12,r3,LSR #14

# qhasm: lotmp ^= (hiu1 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu1=int32#5,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu1=r4,LSL #18
eor r12,r12,r4,LSL #18

# qhasm: hitmp ^= (hiu1 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu1=int32#5,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu1=r4,LSR #18
eor r11,r11,r4,LSR #18

# qhasm: hitmp ^= (lou1 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou1=int32#4,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou1=r3,LSL #14
eor r11,r11,r3,LSL #14

# qhasm: hitmp ^= (hiu1 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu1=int32#5,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu1=r4,LSR #14
eor r11,r11,r4,LSR #14

# qhasm: hitmp ^= (lou1 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou1=int32#4,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou1=r3,LSL #18
eor r11,r11,r3,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lotmp=int32#13
# asm 2: adds >lou4=r9,<lou4=r9,<lotmp=r12
adds r9,r9,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#11,<hiu4=int32#11,<hitmp=int32#12
# asm 2: adc >hiu4=r10,<hiu4=r10,<hitmp=r11
adc r10,r10,r11

# qhasm: lotmp = lou1 & lou2
# asm 1: and >lotmp=int32#6,<lou1=int32#4,<lou2=int32#6
# asm 2: and >lotmp=r5,<lou1=r3,<lou2=r5
and r5,r3,r5

# qhasm: lotmp2 = lou3 & ~lou1
# asm 1: bic >lotmp2=int32#4,<lou3=int32#8,<lou1=int32#4
# asm 2: bic >lotmp2=r3,<lou3=r7,<lou1=r3
bic r3,r7,r3

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#4,<lotmp=int32#6,<lotmp2=int32#4
# asm 2: eor >lotmp=r3,<lotmp=r5,<lotmp2=r3
eor r3,r5,r3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#4,<lou4=int32#10,<lotmp=int32#4
# asm 2: adds >lou4=r3,<lou4=r9,<lotmp=r3
adds r3,r9,r3

# qhasm: hitmp = hiu1 & hiu2
# asm 1: and >hitmp=int32#6,<hiu1=int32#5,<hiu2=int32#7
# asm 2: and >hitmp=r5,<hiu1=r4,<hiu2=r6
and r5,r4,r6

# qhasm: hitmp2 = hiu3 & ~hiu1
# asm 1: bic >hitmp2=int32#5,<hiu3=int32#9,<hiu1=int32#5
# asm 2: bic >hitmp2=r4,<hiu3=r8,<hiu1=r4
bic r4,r8,r4

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#5,<hitmp=int32#6,<hitmp2=int32#5
# asm 2: eor >hitmp=r4,<hitmp=r5,<hitmp2=r4
eor r4,r5,r4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#5,<hiu4=int32#11,<hitmp=int32#5
# asm 2: adc >hiu4=r4,<hiu4=r10,<hitmp=r4
adc r4,r10,r4

# qhasm: carry? lou0 += lou4
# asm 1: adds >lou0=int32#2,<lou0=int32#2,<lou4=int32#4
# asm 2: adds >lou0=r1,<lou0=r1,<lou4=r3
adds r1,r1,r3

# qhasm: hiu0 += hiu4 + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hiu4=int32#5
# asm 2: adc >hiu0=r2,<hiu0=r2,<hiu4=r4
adc r2,r2,r4

# qhasm: lod3 = lou0
# asm 1: str <lou0=int32#2,>lod3=stack32#4
# asm 2: str <lou0=r1,>lod3=[sp,#12]
# copy-collector input: str r1,[sp,#12]

# qhasm: hid3 = hiu0
# asm 1: str <hiu0=int32#3,>hid3=stack32#19
# asm 2: str <hiu0=r2,>hid3=[sp,#72]
# copy-collector input: str r2,[sp,#72]

# qhasm: lou1 = lod0
# asm 1: ldr >lou1=int32#6,<lod0=stack32#13
# asm 2: ldr >lou1=r5,<lod0=[sp,#48]
# copy-collector input: ldr r5,[sp,#48]

# qhasm: hiu1 = hid0
# asm 1: ldr >hiu1=int32#7,<hid0=stack32#14
# asm 2: ldr >hiu1=r6,<hid0=[sp,#52]
# copy-collector input: ldr r6,[sp,#52]

# qhasm: lou2 = lod1
# asm 1: ldr >lou2=int32#8,<lod1=stack32#15
# asm 2: ldr >lou2=r7,<lod1=[sp,#56]
# copy-collector input: ldr r7,[sp,#56]

# qhasm: hiu2 = hid1
# asm 1: ldr >hiu2=int32#9,<hid1=stack32#16
# asm 2: ldr >hiu2=r8,<hid1=[sp,#60]
# copy-collector input: ldr r8,[sp,#60]

# qhasm: lou3 = lod2
# asm 1: ldr >lou3=int32#10,<lod2=stack32#17
# asm 2: ldr >lou3=r9,<lod2=[sp,#64]
# copy-collector input: ldr r9,[sp,#64]

# qhasm: hiu3 = hid2
# asm 1: ldr >hiu3=int32#11,<hid2=stack32#18
# asm 2: ldr >hiu3=r10,<hid2=[sp,#68]
# copy-collector input: ldr r10,[sp,#68]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#12,0x2000000
# asm 2: mov >two25=r11,0x2000000
# copy-collector output starts
str r1,[sp,#12]
str r2,[sp,#72]
ldr r5,[sp,#48]
ldr r6,[sp,#52]
ldr r7,[sp,#56]
ldr r8,[sp,#60]
ldr r9,[sp,#64]
ldr r10,[sp,#68]
# copy-collector output ends
mov r11,0x2000000

# qhasm: hitmp lotmp = lou1 * two25
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<lou1=int32#6,<two25=int32#12
# asm 2: umull >lotmp=r14,>hitmp=r12,<lou1=r5,<two25=r11
umull r14,r12,r5,r11

# qhasm: lotmp hitmp += hiu1 * two25
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<hiu1=int32#7,<two25=int32#12
# asm 2: umlal <hitmp=r12,<lotmp=r14,<hiu1=r6,<two25=r11
umlal r12,r14,r6,r11

# qhasm: lotmp ^= (hiu1 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#14,<hiu1=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r14,<hiu1=r6,LSR #2
eor r11,r14,r6,LSR #2

# qhasm: lotmp ^= (lou1 << 30)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#6,LSL #30
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r5,LSL #30
eor r11,r11,r5,LSL #30

# qhasm: lotmp ^= (lou1 unsigned>> 28)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#6,LSR #28
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r5,LSR #28
eor r11,r11,r5,LSR #28

# qhasm: lotmp ^= (hiu1 << 4)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#7,LSL #4
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r6,LSL #4
eor r11,r11,r6,LSL #4

# qhasm: hitmp ^= (lou1 unsigned>> 2)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou1=int32#6,LSR #2
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou1=r5,LSR #2
eor r12,r12,r5,LSR #2

# qhasm: hitmp ^= (hiu1 << 30)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu1=int32#7,LSL #30
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu1=r6,LSL #30
eor r12,r12,r6,LSL #30

# qhasm: hitmp ^= (hiu1 unsigned>> 28)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu1=int32#7,LSR #28
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu1=r6,LSR #28
eor r12,r12,r6,LSR #28

# qhasm: hitmp ^= (lou1 << 4)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou1=int32#6,LSL #4
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou1=r5,LSL #4
eor r12,r12,r5,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#4,<lou4=int32#4,<lotmp=int32#12
# asm 2: adds >lou4=r3,<lou4=r3,<lotmp=r11
adds r3,r3,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#5,<hiu4=int32#5,<hitmp=int32#13
# asm 2: adc >hiu4=r4,<hiu4=r4,<hitmp=r12
adc r4,r4,r12

# qhasm: lotmp = lou2 ^ lou3
# asm 1: eor >lotmp=int32#12,<lou2=int32#8,<lou3=int32#10
# asm 2: eor >lotmp=r11,<lou2=r7,<lou3=r9
eor r11,r7,r9

# qhasm: lotmp &= lou1
# asm 1: and >lotmp=int32#6,<lotmp=int32#12,<lou1=int32#6
# asm 2: and >lotmp=r5,<lotmp=r11,<lou1=r5
and r5,r11,r5

# qhasm: lotmp2 = lou2 & lou3
# asm 1: and >lotmp2=int32#8,<lou2=int32#8,<lou3=int32#10
# asm 2: and >lotmp2=r7,<lou2=r7,<lou3=r9
and r7,r7,r9

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#6,<lotmp=int32#6,<lotmp2=int32#8
# asm 2: eor >lotmp=r5,<lotmp=r5,<lotmp2=r7
eor r5,r5,r7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#4,<lou4=int32#4,<lotmp=int32#6
# asm 2: adds >lou4=r3,<lou4=r3,<lotmp=r5
adds.w r3,r3,r5

# qhasm: hitmp = hiu2 ^ hiu3
# asm 1: eor >hitmp=int32#6,<hiu2=int32#9,<hiu3=int32#11
# asm 2: eor >hitmp=r5,<hiu2=r8,<hiu3=r10
eor r5,r8,r10

# qhasm: hitmp &= hiu1
# asm 1: and >hitmp=int32#6,<hitmp=int32#6,<hiu1=int32#7
# asm 2: and >hitmp=r5,<hitmp=r5,<hiu1=r6
and r5,r5,r6

# qhasm: hitmp2 = hiu2 & hiu3
# asm 1: and >hitmp2=int32#7,<hiu2=int32#9,<hiu3=int32#11
# asm 2: and >hitmp2=r6,<hiu2=r8,<hiu3=r10
and r6,r8,r10

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#6,<hitmp=int32#6,<hitmp2=int32#7
# asm 2: eor >hitmp=r5,<hitmp=r5,<hitmp2=r6
eor r5,r5,r6

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#5,<hiu4=int32#5,<hitmp=int32#6
# asm 2: adc >hiu4=r4,<hiu4=r4,<hitmp=r5
adc r4,r4,r5

# qhasm: lod7 = lou4
# asm 1: str <lou4=int32#4,>lod7=stack32#17
# asm 2: str <lou4=r3,>lod7=[sp,#64]
# copy-collector input: str r3,[sp,#64]

# qhasm: hid7 = hiu4
# asm 1: str <hiu4=int32#5,>hid7=stack32#18
# asm 2: str <hiu4=r4,>hid7=[sp,#68]
# copy-collector input: str r4,[sp,#68]

# qhasm: lou1 = lod4
# asm 1: ldr >lou1=int32#4,<lod4=stack32#29
# asm 2: ldr >lou1=r3,<lod4=[sp,#112]
# copy-collector input: ldr r3,[sp,#112]

# qhasm: hiu1 = hid4
# asm 1: ldr >hiu1=int32#5,<hid4=stack32#30
# asm 2: ldr >hiu1=r4,<hid4=[sp,#116]
# copy-collector input: ldr r4,[sp,#116]

# qhasm: lou2 = lod5
# asm 1: ldr >lou2=int32#6,<lod5=stack32#31
# asm 2: ldr >lou2=r5,<lod5=[sp,#120]
# copy-collector input: ldr r5,[sp,#120]

# qhasm: hiu2 = hid5
# asm 1: ldr >hiu2=int32#7,<hid5=stack32#32
# asm 2: ldr >hiu2=r6,<hid5=[sp,#124]
# copy-collector input: ldr r6,[sp,#124]

# qhasm: lou4 = lod6
# asm 1: ldr >lou4=int32#8,<lod6=stack32#33
# asm 2: ldr >lou4=r7,<lod6=[sp,#128]
# copy-collector input: ldr r7,[sp,#128]

# qhasm: hiu4 = hid6
# asm 1: ldr >hiu4=int32#9,<hid6=stack32#34
# asm 2: ldr >hiu4=r8,<hid6=[sp,#132]
# copy-collector input: ldr r8,[sp,#132]

# qhasm: lou5 = lod9
# asm 1: ldr >lou5=int32#12,<lod9=stack32#39
# asm 2: ldr >lou5=r11,<lod9=[sp,#152]
# copy-collector input: ldr r11,[sp,#152]

# qhasm: hiu5 = hid9
# asm 1: ldr >hiu5=int32#13,<hid9=stack32#40
# asm 2: ldr >hiu5=r12,<hid9=[sp,#156]
# copy-collector input: ldr r12,[sp,#156]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
strd r3,r4,[sp,#64]
ldr r3,[sp,#112]
ldr r4,[sp,#116]
ldr r5,[sp,#120]
ldr r6,[sp,#124]
ldr.w r7,[sp,#128]
ldr r8,[sp,#132]
ldr r11,[sp,#152]
ldr r12,[sp,#156]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r12
adc r8,r8,r12

# qhasm: hitmp lotmp = lou0 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou0=int32#2,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou0=r1,<two23=r14
umull r12,r11,r1,r14

# qhasm: lotmp hitmp += hiu0 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu0=int32#3,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu0=r2,<two23=r14
umlal r11,r12,r2,r14

# qhasm: lotmp ^= (lou0 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou0=int32#2,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou0=r1,LSR #18
eor r12,r12,r1,LSR #18

# qhasm: lotmp ^= (hiu0 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu0=int32#3,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu0=r2,LSL #14
eor r12,r12,r2,LSL #14

# qhasm: lotmp ^= (lou0 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou0=int32#2,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou0=r1,LSR #14
eor r12,r12,r1,LSR #14

# qhasm: lotmp ^= (hiu0 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu0=int32#3,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu0=r2,LSL #18
eor r12,r12,r2,LSL #18

# qhasm: hitmp ^= (hiu0 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#3,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r2,LSR #18
eor r11,r11,r2,LSR #18

# qhasm: hitmp ^= (lou0 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#2,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r1,LSL #14
eor r11,r11,r1,LSL #14

# qhasm: hitmp ^= (hiu0 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#3,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r2,LSR #14
eor r11,r11,r2,LSR #14

# qhasm: hitmp ^= (lou0 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#2,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r1,LSL #18
eor r11,r11,r1,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#13
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r12
adds r7,r7,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#12
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r11
adc r8,r8,r11

# qhasm: lotmp = lou0 & lou1
# asm 1: and >lotmp=int32#4,<lou0=int32#2,<lou1=int32#4
# asm 2: and >lotmp=r3,<lou0=r1,<lou1=r3
and r3,r1,r3

# qhasm: lotmp2 = lou2 & ~lou0
# asm 1: bic >lotmp2=int32#2,<lou2=int32#6,<lou0=int32#2
# asm 2: bic >lotmp2=r1,<lou2=r5,<lou0=r1
bic r1,r5,r1

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#2,<lotmp=int32#4,<lotmp2=int32#2
# asm 2: eor >lotmp=r1,<lotmp=r3,<lotmp2=r1
eor r1,r3,r1

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#8,<lotmp=int32#2
# asm 2: adds >lou4=r1,<lou4=r7,<lotmp=r1
adds.w r1,r7,r1

# qhasm: hitmp = hiu0 & hiu1
# asm 1: and >hitmp=int32#4,<hiu0=int32#3,<hiu1=int32#5
# asm 2: and >hitmp=r3,<hiu0=r2,<hiu1=r4
and r3,r2,r4

# qhasm: hitmp2 = hiu2 & ~hiu0
# asm 1: bic >hitmp2=int32#3,<hiu2=int32#7,<hiu0=int32#3
# asm 2: bic >hitmp2=r2,<hiu2=r6,<hiu0=r2
bic r2,r6,r2

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#3,<hitmp=int32#4,<hitmp2=int32#3
# asm 2: eor >hitmp=r2,<hitmp=r3,<hitmp2=r2
eor r2,r3,r2

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#9,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r8,<hitmp=r2
adc r2,r8,r2

# qhasm: carry? lou3 += lou4
# asm 1: adds >lou3=int32#4,<lou3=int32#10,<lou4=int32#2
# asm 2: adds >lou3=r3,<lou3=r9,<lou4=r1
adds r3,r9,r1

# qhasm: hiu3 += hiu4 + carry
# asm 1: adc >hiu3=int32#5,<hiu3=int32#11,<hiu4=int32#3
# asm 2: adc >hiu3=r4,<hiu3=r10,<hiu4=r2
adc r4,r10,r2

# qhasm: lod2 = lou3
# asm 1: str <lou3=int32#4,>lod2=stack32#33
# asm 2: str <lou3=r3,>lod2=[sp,#128]
# copy-collector input: str r3,[sp,#128]

# qhasm: hid2 = hiu3
# asm 1: str <hiu3=int32#5,>hid2=stack32#34
# asm 2: str <hiu3=r4,>hid2=[sp,#132]
# copy-collector input: str r4,[sp,#132]

# qhasm: lou0 = lod7
# asm 1: ldr >lou0=int32#6,<lod7=stack32#17
# asm 2: ldr >lou0=r5,<lod7=[sp,#64]
# copy-collector input: ldr r5,[sp,#64]

# qhasm: hiu0 = hid7
# asm 1: ldr >hiu0=int32#7,<hid7=stack32#18
# asm 2: ldr >hiu0=r6,<hid7=[sp,#68]
# copy-collector input: ldr r6,[sp,#68]

# qhasm: lou1 = lod0
# asm 1: ldr >lou1=int32#8,<lod0=stack32#13
# asm 2: ldr >lou1=r7,<lod0=[sp,#48]
# copy-collector input: ldr r7,[sp,#48]

# qhasm: hiu1 = hid0
# asm 1: ldr >hiu1=int32#9,<hid0=stack32#14
# asm 2: ldr >hiu1=r8,<hid0=[sp,#52]
# copy-collector input: ldr r8,[sp,#52]

# qhasm: lou2 = lod1
# asm 1: ldr >lou2=int32#10,<lod1=stack32#15
# asm 2: ldr >lou2=r9,<lod1=[sp,#56]
# copy-collector input: ldr r9,[sp,#56]

# qhasm: hiu2 = hid1
# asm 1: ldr >hiu2=int32#11,<hid1=stack32#16
# asm 2: ldr >hiu2=r10,<hid1=[sp,#60]
# copy-collector input: ldr r10,[sp,#60]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#12,0x2000000
# asm 2: mov >two25=r11,0x2000000
# copy-collector output starts
strd r3,r4,[sp,#128]
ldr r5,[sp,#64]
ldr r6,[sp,#68]
ldr.w r7,[sp,#48]
ldr r8,[sp,#52]
ldr r9,[sp,#56]
ldr r10,[sp,#60]
# copy-collector output ends
mov r11,0x2000000

# qhasm: hitmp lotmp = lou0 * two25
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<lou0=int32#6,<two25=int32#12
# asm 2: umull >lotmp=r14,>hitmp=r12,<lou0=r5,<two25=r11
umull r14,r12,r5,r11

# qhasm: lotmp hitmp += hiu0 * two25
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<hiu0=int32#7,<two25=int32#12
# asm 2: umlal <hitmp=r12,<lotmp=r14,<hiu0=r6,<two25=r11
umlal r12,r14,r6,r11

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#14,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r14,<hiu0=r6,LSR #2
eor r11,r14,r6,LSR #2

# qhasm: lotmp ^= (lou0 << 30)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #30
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #30
eor r11,r11,r5,LSL #30

# qhasm: lotmp ^= (lou0 unsigned>> 28)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSR #28
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSR #28
eor r11,r11,r5,LSR #28

# qhasm: lotmp ^= (hiu0 << 4)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSL #4
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSL #4
eor r11,r11,r6,LSL #4

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSR #2
eor r12,r12,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSL #30
eor r12,r12,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSR #28
eor r12,r12,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSL #4
eor r12,r12,r5,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lotmp = lou1 ^ lou2
# asm 1: eor >lotmp=int32#12,<lou1=int32#8,<lou2=int32#10
# asm 2: eor >lotmp=r11,<lou1=r7,<lou2=r9
eor r11,r7,r9

# qhasm: lotmp &= lou0
# asm 1: and >lotmp=int32#6,<lotmp=int32#12,<lou0=int32#6
# asm 2: and >lotmp=r5,<lotmp=r11,<lou0=r5
and r5,r11,r5

# qhasm: lotmp2 = lou1 & lou2
# asm 1: and >lotmp2=int32#8,<lou1=int32#8,<lou2=int32#10
# asm 2: and >lotmp2=r7,<lou1=r7,<lou2=r9
and r7,r7,r9

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#6,<lotmp=int32#6,<lotmp2=int32#8
# asm 2: eor >lotmp=r5,<lotmp=r5,<lotmp2=r7
eor r5,r5,r7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#6
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r5
adds.w r1,r1,r5

# qhasm: hitmp = hiu1 ^ hiu2
# asm 1: eor >hitmp=int32#6,<hiu1=int32#9,<hiu2=int32#11
# asm 2: eor >hitmp=r5,<hiu1=r8,<hiu2=r10
eor r5,r8,r10

# qhasm: hitmp &= hiu0
# asm 1: and >hitmp=int32#6,<hitmp=int32#6,<hiu0=int32#7
# asm 2: and >hitmp=r5,<hitmp=r5,<hiu0=r6
and r5,r5,r6

# qhasm: hitmp2 = hiu1 & hiu2
# asm 1: and >hitmp2=int32#7,<hiu1=int32#9,<hiu2=int32#11
# asm 2: and >hitmp2=r6,<hiu1=r8,<hiu2=r10
and r6,r8,r10

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#6,<hitmp=int32#6,<hitmp2=int32#7
# asm 2: eor >hitmp=r5,<hitmp=r5,<hitmp2=r6
eor r5,r5,r6

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#6
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r5
adc r2,r2,r5

# qhasm: lod6 = lou4
# asm 1: str <lou4=int32#2,>lod6=stack32#15
# asm 2: str <lou4=r1,>lod6=[sp,#56]
# copy-collector input: str r1,[sp,#56]

# qhasm: hid6 = hiu4
# asm 1: str <hiu4=int32#3,>hid6=stack32#16
# asm 2: str <hiu4=r2,>hid6=[sp,#60]
# copy-collector input: str r2,[sp,#60]

# qhasm: lou0 = lod3
# asm 1: ldr >lou0=int32#2,<lod3=stack32#4
# asm 2: ldr >lou0=r1,<lod3=[sp,#12]
# copy-collector input: ldr r1,[sp,#12]

# qhasm: hiu0 = hid3
# asm 1: ldr >hiu0=int32#3,<hid3=stack32#19
# asm 2: ldr >hiu0=r2,<hid3=[sp,#72]
# copy-collector input: ldr r2,[sp,#72]

# qhasm: lou1 = lod4
# asm 1: ldr >lou1=int32#6,<lod4=stack32#29
# asm 2: ldr >lou1=r5,<lod4=[sp,#112]
# copy-collector input: ldr r5,[sp,#112]

# qhasm: hiu1 = hid4
# asm 1: ldr >hiu1=int32#7,<hid4=stack32#30
# asm 2: ldr >hiu1=r6,<hid4=[sp,#116]
# copy-collector input: ldr r6,[sp,#116]

# qhasm: lou4 = lod5
# asm 1: ldr >lou4=int32#8,<lod5=stack32#31
# asm 2: ldr >lou4=r7,<lod5=[sp,#120]
# copy-collector input: ldr r7,[sp,#120]

# qhasm: hiu4 = hid5
# asm 1: ldr >hiu4=int32#9,<hid5=stack32#32
# asm 2: ldr >hiu4=r8,<hid5=[sp,#124]
# copy-collector input: ldr r8,[sp,#124]

# qhasm: lou5 = lod10
# asm 1: ldr >lou5=int32#12,<lod10=stack32#41
# asm 2: ldr >lou5=r11,<lod10=[sp,#160]
# copy-collector input: ldr r11,[sp,#160]

# qhasm: hiu5 = hid10
# asm 1: ldr >hiu5=int32#13,<hid10=stack32#42
# asm 2: ldr >hiu5=r12,<hid10=[sp,#164]
# copy-collector input: ldr r12,[sp,#164]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
strd r1,r2,[sp,#56]
ldr r1,[sp,#12]
ldr r2,[sp,#72]
ldr r5,[sp,#112]
ldr r6,[sp,#116]
ldr.w r7,[sp,#120]
ldr r8,[sp,#124]
ldr r11,[sp,#160]
ldr r12,[sp,#164]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r12
adc r8,r8,r12

# qhasm: hitmp lotmp = lou3 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou3=int32#4,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou3=r3,<two23=r14
umull r12,r11,r3,r14

# qhasm: lotmp hitmp += hiu3 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu3=int32#5,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu3=r4,<two23=r14
umlal r11,r12,r4,r14

# qhasm: lotmp ^= (lou3 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou3=int32#4,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou3=r3,LSR #18
eor r12,r12,r3,LSR #18

# qhasm: lotmp ^= (hiu3 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu3=int32#5,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu3=r4,LSL #14
eor r12,r12,r4,LSL #14

# qhasm: lotmp ^= (lou3 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou3=int32#4,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou3=r3,LSR #14
eor r12,r12,r3,LSR #14

# qhasm: lotmp ^= (hiu3 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu3=int32#5,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu3=r4,LSL #18
eor r12,r12,r4,LSL #18

# qhasm: hitmp ^= (hiu3 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSR #18
eor r11,r11,r4,LSR #18

# qhasm: hitmp ^= (lou3 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou3=int32#4,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou3=r3,LSL #14
eor r11,r11,r3,LSL #14

# qhasm: hitmp ^= (hiu3 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSR #14
eor r11,r11,r4,LSR #14

# qhasm: hitmp ^= (lou3 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou3=int32#4,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou3=r3,LSL #18
eor r11,r11,r3,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#13
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r12
adds r7,r7,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#12
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r11
adc r8,r8,r11

# qhasm: lotmp = lou3 & lou0
# asm 1: and >lotmp=int32#2,<lou3=int32#4,<lou0=int32#2
# asm 2: and >lotmp=r1,<lou3=r3,<lou0=r1
and r1,r3,r1

# qhasm: lotmp2 = lou1 & ~lou3
# asm 1: bic >lotmp2=int32#4,<lou1=int32#6,<lou3=int32#4
# asm 2: bic >lotmp2=r3,<lou1=r5,<lou3=r3
bic r3,r5,r3

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lotmp2=int32#4
# asm 2: eor >lotmp=r1,<lotmp=r1,<lotmp2=r3
eor r1,r1,r3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#8,<lotmp=int32#2
# asm 2: adds >lou4=r1,<lou4=r7,<lotmp=r1
adds.w r1,r7,r1

# qhasm: hitmp = hiu3 & hiu0
# asm 1: and >hitmp=int32#3,<hiu3=int32#5,<hiu0=int32#3
# asm 2: and >hitmp=r2,<hiu3=r4,<hiu0=r2
and r2,r4,r2

# qhasm: hitmp2 = hiu1 & ~hiu3
# asm 1: bic >hitmp2=int32#4,<hiu1=int32#7,<hiu3=int32#5
# asm 2: bic >hitmp2=r3,<hiu1=r6,<hiu3=r4
bic r3,r6,r4

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hitmp2=int32#4
# asm 2: eor >hitmp=r2,<hitmp=r2,<hitmp2=r3
eor r2,r2,r3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#9,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r8,<hitmp=r2
adc r2,r8,r2

# qhasm: carry? lou2 += lou4
# asm 1: adds >lou2=int32#4,<lou2=int32#10,<lou4=int32#2
# asm 2: adds >lou2=r3,<lou2=r9,<lou4=r1
adds r3,r9,r1

# qhasm: hiu2 += hiu4 + carry
# asm 1: adc >hiu2=int32#5,<hiu2=int32#11,<hiu4=int32#3
# asm 2: adc >hiu2=r4,<hiu2=r10,<hiu4=r2
adc r4,r10,r2

# qhasm: lod1 = lou2
# asm 1: str <lou2=int32#4,>lod1=stack32#31
# asm 2: str <lou2=r3,>lod1=[sp,#120]
# copy-collector input: str r3,[sp,#120]

# qhasm: hid1 = hiu2
# asm 1: str <hiu2=int32#5,>hid1=stack32#32
# asm 2: str <hiu2=r4,>hid1=[sp,#124]
# copy-collector input: str r4,[sp,#124]

# qhasm: lou0 = lod6
# asm 1: ldr >lou0=int32#6,<lod6=stack32#15
# asm 2: ldr >lou0=r5,<lod6=[sp,#56]
# copy-collector input: ldr r5,[sp,#56]

# qhasm: hiu0 = hid6
# asm 1: ldr >hiu0=int32#7,<hid6=stack32#16
# asm 2: ldr >hiu0=r6,<hid6=[sp,#60]
# copy-collector input: ldr r6,[sp,#60]

# qhasm: lou1 = lod7
# asm 1: ldr >lou1=int32#8,<lod7=stack32#17
# asm 2: ldr >lou1=r7,<lod7=[sp,#64]
# copy-collector input: ldr r7,[sp,#64]

# qhasm: hiu1 = hid7
# asm 1: ldr >hiu1=int32#9,<hid7=stack32#18
# asm 2: ldr >hiu1=r8,<hid7=[sp,#68]
# copy-collector input: ldr r8,[sp,#68]

# qhasm: lou3 = lod0
# asm 1: ldr >lou3=int32#10,<lod0=stack32#13
# asm 2: ldr >lou3=r9,<lod0=[sp,#48]
# copy-collector input: ldr r9,[sp,#48]

# qhasm: hiu3 = hid0
# asm 1: ldr >hiu3=int32#11,<hid0=stack32#14
# asm 2: ldr >hiu3=r10,<hid0=[sp,#52]
# copy-collector input: ldr r10,[sp,#52]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#12,0x2000000
# asm 2: mov >two25=r11,0x2000000
# copy-collector output starts
strd r3,r4,[sp,#120]
ldr r5,[sp,#56]
ldr r6,[sp,#60]
ldr.w r7,[sp,#64]
ldr r8,[sp,#68]
ldr r9,[sp,#48]
ldr r10,[sp,#52]
# copy-collector output ends
mov r11,0x2000000

# qhasm: hitmp lotmp = lou0 * two25
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<lou0=int32#6,<two25=int32#12
# asm 2: umull >lotmp=r14,>hitmp=r12,<lou0=r5,<two25=r11
umull r14,r12,r5,r11

# qhasm: lotmp hitmp += hiu0 * two25
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<hiu0=int32#7,<two25=int32#12
# asm 2: umlal <hitmp=r12,<lotmp=r14,<hiu0=r6,<two25=r11
umlal r12,r14,r6,r11

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#14,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r14,<hiu0=r6,LSR #2
eor r11,r14,r6,LSR #2

# qhasm: lotmp ^= (lou0 << 30)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #30
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #30
eor r11,r11,r5,LSL #30

# qhasm: lotmp ^= (lou0 unsigned>> 28)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSR #28
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSR #28
eor r11,r11,r5,LSR #28

# qhasm: lotmp ^= (hiu0 << 4)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSL #4
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSL #4
eor r11,r11,r6,LSL #4

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSR #2
eor r12,r12,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSL #30
eor r12,r12,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSR #28
eor r12,r12,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSL #4
eor r12,r12,r5,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lotmp = lou1 ^ lou3
# asm 1: eor >lotmp=int32#12,<lou1=int32#8,<lou3=int32#10
# asm 2: eor >lotmp=r11,<lou1=r7,<lou3=r9
eor r11,r7,r9

# qhasm: lotmp &= lou0
# asm 1: and >lotmp=int32#6,<lotmp=int32#12,<lou0=int32#6
# asm 2: and >lotmp=r5,<lotmp=r11,<lou0=r5
and r5,r11,r5

# qhasm: lotmp2 = lou1 & lou3
# asm 1: and >lotmp2=int32#8,<lou1=int32#8,<lou3=int32#10
# asm 2: and >lotmp2=r7,<lou1=r7,<lou3=r9
and r7,r7,r9

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#6,<lotmp=int32#6,<lotmp2=int32#8
# asm 2: eor >lotmp=r5,<lotmp=r5,<lotmp2=r7
eor r5,r5,r7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#6
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r5
adds.w r1,r1,r5

# qhasm: hitmp = hiu1 ^ hiu3
# asm 1: eor >hitmp=int32#6,<hiu1=int32#9,<hiu3=int32#11
# asm 2: eor >hitmp=r5,<hiu1=r8,<hiu3=r10
eor r5,r8,r10

# qhasm: hitmp &= hiu0
# asm 1: and >hitmp=int32#6,<hitmp=int32#6,<hiu0=int32#7
# asm 2: and >hitmp=r5,<hitmp=r5,<hiu0=r6
and r5,r5,r6

# qhasm: hitmp2 = hiu1 & hiu3
# asm 1: and >hitmp2=int32#7,<hiu1=int32#9,<hiu3=int32#11
# asm 2: and >hitmp2=r6,<hiu1=r8,<hiu3=r10
and r6,r8,r10

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#6,<hitmp=int32#6,<hitmp2=int32#7
# asm 2: eor >hitmp=r5,<hitmp=r5,<hitmp2=r6
eor r5,r5,r6

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#6
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r5
adc r2,r2,r5

# qhasm: lod5 = lou4
# asm 1: str <lou4=int32#2,>lod5=stack32#13
# asm 2: str <lou4=r1,>lod5=[sp,#48]
# copy-collector input: str r1,[sp,#48]

# qhasm: hid5 = hiu4
# asm 1: str <hiu4=int32#3,>hid5=stack32#14
# asm 2: str <hiu4=r2,>hid5=[sp,#52]
# copy-collector input: str r2,[sp,#52]

# qhasm: lou0 = lod2
# asm 1: ldr >lou0=int32#2,<lod2=stack32#33
# asm 2: ldr >lou0=r1,<lod2=[sp,#128]
# copy-collector input: ldr r1,[sp,#128]

# qhasm: hiu0 = hid2
# asm 1: ldr >hiu0=int32#3,<hid2=stack32#34
# asm 2: ldr >hiu0=r2,<hid2=[sp,#132]
# copy-collector input: ldr r2,[sp,#132]

# qhasm: lou1 = lod3
# asm 1: ldr >lou1=int32#6,<lod3=stack32#4
# asm 2: ldr >lou1=r5,<lod3=[sp,#12]
# copy-collector input: ldr r5,[sp,#12]

# qhasm: hiu1 = hid3
# asm 1: ldr >hiu1=int32#7,<hid3=stack32#19
# asm 2: ldr >hiu1=r6,<hid3=[sp,#72]
# copy-collector input: ldr r6,[sp,#72]

# qhasm: lou4 = lod4
# asm 1: ldr >lou4=int32#8,<lod4=stack32#29
# asm 2: ldr >lou4=r7,<lod4=[sp,#112]
# copy-collector input: ldr r7,[sp,#112]

# qhasm: hiu4 = hid4
# asm 1: ldr >hiu4=int32#9,<hid4=stack32#30
# asm 2: ldr >hiu4=r8,<hid4=[sp,#116]
# copy-collector input: ldr r8,[sp,#116]

# qhasm: lou5 = lod11
# asm 1: ldr >lou5=int32#12,<lod11=stack32#43
# asm 2: ldr >lou5=r11,<lod11=[sp,#168]
# copy-collector input: ldr r11,[sp,#168]

# qhasm: hiu5 = hid11
# asm 1: ldr >hiu5=int32#13,<hid11=stack32#44
# asm 2: ldr >hiu5=r12,<hid11=[sp,#172]
# copy-collector input: ldr r12,[sp,#172]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
strd r1,r2,[sp,#48]
ldr r1,[sp,#128]
ldr r2,[sp,#132]
ldr r5,[sp,#12]
ldr r6,[sp,#72]
ldr.w r7,[sp,#112]
ldr r8,[sp,#116]
ldr r11,[sp,#168]
ldr r12,[sp,#172]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r12
adc r8,r8,r12

# qhasm: hitmp lotmp = lou2 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou2=int32#4,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou2=r3,<two23=r14
umull r12,r11,r3,r14

# qhasm: lotmp hitmp += hiu2 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu2=int32#5,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu2=r4,<two23=r14
umlal r11,r12,r4,r14

# qhasm: lotmp ^= (lou2 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou2=int32#4,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou2=r3,LSR #18
eor r12,r12,r3,LSR #18

# qhasm: lotmp ^= (hiu2 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu2=int32#5,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu2=r4,LSL #14
eor r12,r12,r4,LSL #14

# qhasm: lotmp ^= (lou2 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou2=int32#4,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou2=r3,LSR #14
eor r12,r12,r3,LSR #14

# qhasm: lotmp ^= (hiu2 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu2=int32#5,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu2=r4,LSL #18
eor r12,r12,r4,LSL #18

# qhasm: hitmp ^= (hiu2 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSR #18
eor r11,r11,r4,LSR #18

# qhasm: hitmp ^= (lou2 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou2=int32#4,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou2=r3,LSL #14
eor r11,r11,r3,LSL #14

# qhasm: hitmp ^= (hiu2 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSR #14
eor r11,r11,r4,LSR #14

# qhasm: hitmp ^= (lou2 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou2=int32#4,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou2=r3,LSL #18
eor r11,r11,r3,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#13
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r12
adds r7,r7,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#12
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r11
adc r8,r8,r11

# qhasm: lotmp = lou2 & lou0
# asm 1: and >lotmp=int32#2,<lou2=int32#4,<lou0=int32#2
# asm 2: and >lotmp=r1,<lou2=r3,<lou0=r1
and r1,r3,r1

# qhasm: lotmp2 = lou1 & ~lou2
# asm 1: bic >lotmp2=int32#4,<lou1=int32#6,<lou2=int32#4
# asm 2: bic >lotmp2=r3,<lou1=r5,<lou2=r3
bic r3,r5,r3

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lotmp2=int32#4
# asm 2: eor >lotmp=r1,<lotmp=r1,<lotmp2=r3
eor r1,r1,r3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#8,<lotmp=int32#2
# asm 2: adds >lou4=r1,<lou4=r7,<lotmp=r1
adds.w r1,r7,r1

# qhasm: hitmp = hiu2 & hiu0
# asm 1: and >hitmp=int32#3,<hiu2=int32#5,<hiu0=int32#3
# asm 2: and >hitmp=r2,<hiu2=r4,<hiu0=r2
and r2,r4,r2

# qhasm: hitmp2 = hiu1 & ~hiu2
# asm 1: bic >hitmp2=int32#4,<hiu1=int32#7,<hiu2=int32#5
# asm 2: bic >hitmp2=r3,<hiu1=r6,<hiu2=r4
bic r3,r6,r4

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hitmp2=int32#4
# asm 2: eor >hitmp=r2,<hitmp=r2,<hitmp2=r3
eor r2,r2,r3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#9,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r8,<hitmp=r2
adc r2,r8,r2

# qhasm: carry? lou3 += lou4
# asm 1: adds >lou3=int32#4,<lou3=int32#10,<lou4=int32#2
# asm 2: adds >lou3=r3,<lou3=r9,<lou4=r1
adds r3,r9,r1

# qhasm: hiu3 += hiu4 + carry
# asm 1: adc >hiu3=int32#5,<hiu3=int32#11,<hiu4=int32#3
# asm 2: adc >hiu3=r4,<hiu3=r10,<hiu4=r2
adc r4,r10,r2

# qhasm: lod0 = lou3
# asm 1: str <lou3=int32#4,>lod0=stack32#29
# asm 2: str <lou3=r3,>lod0=[sp,#112]
# copy-collector input: str r3,[sp,#112]

# qhasm: hid0 = hiu3
# asm 1: str <hiu3=int32#5,>hid0=stack32#30
# asm 2: str <hiu3=r4,>hid0=[sp,#116]
# copy-collector input: str r4,[sp,#116]

# qhasm: lou0 = lod5
# asm 1: ldr >lou0=int32#6,<lod5=stack32#13
# asm 2: ldr >lou0=r5,<lod5=[sp,#48]
# copy-collector input: ldr r5,[sp,#48]

# qhasm: hiu0 = hid5
# asm 1: ldr >hiu0=int32#7,<hid5=stack32#14
# asm 2: ldr >hiu0=r6,<hid5=[sp,#52]
# copy-collector input: ldr r6,[sp,#52]

# qhasm: lou1 = lod6
# asm 1: ldr >lou1=int32#8,<lod6=stack32#15
# asm 2: ldr >lou1=r7,<lod6=[sp,#56]
# copy-collector input: ldr r7,[sp,#56]

# qhasm: hiu1 = hid6
# asm 1: ldr >hiu1=int32#9,<hid6=stack32#16
# asm 2: ldr >hiu1=r8,<hid6=[sp,#60]
# copy-collector input: ldr r8,[sp,#60]

# qhasm: lou2 = lod7
# asm 1: ldr >lou2=int32#10,<lod7=stack32#17
# asm 2: ldr >lou2=r9,<lod7=[sp,#64]
# copy-collector input: ldr r9,[sp,#64]

# qhasm: hiu2 = hid7
# asm 1: ldr >hiu2=int32#11,<hid7=stack32#18
# asm 2: ldr >hiu2=r10,<hid7=[sp,#68]
# copy-collector input: ldr r10,[sp,#68]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#12,0x2000000
# asm 2: mov >two25=r11,0x2000000
# copy-collector output starts
strd r3,r4,[sp,#112]
ldr r5,[sp,#48]
ldr r6,[sp,#52]
ldr.w r7,[sp,#56]
ldr r8,[sp,#60]
ldr r9,[sp,#64]
ldr r10,[sp,#68]
# copy-collector output ends
mov r11,0x2000000

# qhasm: hitmp lotmp = lou0 * two25
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<lou0=int32#6,<two25=int32#12
# asm 2: umull >lotmp=r14,>hitmp=r12,<lou0=r5,<two25=r11
umull r14,r12,r5,r11

# qhasm: lotmp hitmp += hiu0 * two25
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<hiu0=int32#7,<two25=int32#12
# asm 2: umlal <hitmp=r12,<lotmp=r14,<hiu0=r6,<two25=r11
umlal r12,r14,r6,r11

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#14,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r14,<hiu0=r6,LSR #2
eor r11,r14,r6,LSR #2

# qhasm: lotmp ^= (lou0 << 30)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #30
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #30
eor r11,r11,r5,LSL #30

# qhasm: lotmp ^= (lou0 unsigned>> 28)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSR #28
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSR #28
eor r11,r11,r5,LSR #28

# qhasm: lotmp ^= (hiu0 << 4)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSL #4
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSL #4
eor r11,r11,r6,LSL #4

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSR #2
eor r12,r12,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSL #30
eor r12,r12,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSR #28
eor r12,r12,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSL #4
eor r12,r12,r5,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lotmp = lou1 ^ lou2
# asm 1: eor >lotmp=int32#12,<lou1=int32#8,<lou2=int32#10
# asm 2: eor >lotmp=r11,<lou1=r7,<lou2=r9
eor r11,r7,r9

# qhasm: lotmp &= lou0
# asm 1: and >lotmp=int32#6,<lotmp=int32#12,<lou0=int32#6
# asm 2: and >lotmp=r5,<lotmp=r11,<lou0=r5
and r5,r11,r5

# qhasm: lotmp2 = lou1 & lou2
# asm 1: and >lotmp2=int32#8,<lou1=int32#8,<lou2=int32#10
# asm 2: and >lotmp2=r7,<lou1=r7,<lou2=r9
and r7,r7,r9

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#6,<lotmp=int32#6,<lotmp2=int32#8
# asm 2: eor >lotmp=r5,<lotmp=r5,<lotmp2=r7
eor r5,r5,r7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#6
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r5
adds.w r1,r1,r5

# qhasm: hitmp = hiu1 ^ hiu2
# asm 1: eor >hitmp=int32#6,<hiu1=int32#9,<hiu2=int32#11
# asm 2: eor >hitmp=r5,<hiu1=r8,<hiu2=r10
eor r5,r8,r10

# qhasm: hitmp &= hiu0
# asm 1: and >hitmp=int32#6,<hitmp=int32#6,<hiu0=int32#7
# asm 2: and >hitmp=r5,<hitmp=r5,<hiu0=r6
and r5,r5,r6

# qhasm: hitmp2 = hiu1 & hiu2
# asm 1: and >hitmp2=int32#7,<hiu1=int32#9,<hiu2=int32#11
# asm 2: and >hitmp2=r6,<hiu1=r8,<hiu2=r10
and r6,r8,r10

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#6,<hitmp=int32#6,<hitmp2=int32#7
# asm 2: eor >hitmp=r5,<hitmp=r5,<hitmp2=r6
eor r5,r5,r6

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#6
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r5
adc r2,r2,r5

# qhasm: lod4 = lou4
# asm 1: str <lou4=int32#2,>lod4=stack32#70
# asm 2: str <lou4=r1,>lod4=[sp,#276]
# copy-collector input: str r1,[sp,#276]

# qhasm: hid4 = hiu4
# asm 1: str <hiu4=int32#3,>hid4=stack32#71
# asm 2: str <hiu4=r2,>hid4=[sp,#280]
# copy-collector input: str r2,[sp,#280]

# qhasm: lou0 = lod1
# asm 1: ldr >lou0=int32#2,<lod1=stack32#31
# asm 2: ldr >lou0=r1,<lod1=[sp,#120]
# copy-collector input: ldr r1,[sp,#120]

# qhasm: hiu0 = hid1
# asm 1: ldr >hiu0=int32#3,<hid1=stack32#32
# asm 2: ldr >hiu0=r2,<hid1=[sp,#124]
# copy-collector input: ldr r2,[sp,#124]

# qhasm: lou1 = lod2
# asm 1: ldr >lou1=int32#6,<lod2=stack32#33
# asm 2: ldr >lou1=r5,<lod2=[sp,#128]
# copy-collector input: ldr r5,[sp,#128]

# qhasm: hiu1 = hid2
# asm 1: ldr >hiu1=int32#7,<hid2=stack32#34
# asm 2: ldr >hiu1=r6,<hid2=[sp,#132]
# copy-collector input: ldr r6,[sp,#132]

# qhasm: lou4 = lod3
# asm 1: ldr >lou4=int32#8,<lod3=stack32#4
# asm 2: ldr >lou4=r7,<lod3=[sp,#12]
# copy-collector input: ldr r7,[sp,#12]

# qhasm: hiu4 = hid3
# asm 1: ldr >hiu4=int32#9,<hid3=stack32#19
# asm 2: ldr >hiu4=r8,<hid3=[sp,#72]
# copy-collector input: ldr r8,[sp,#72]

# qhasm: lou5 = lod12
# asm 1: ldr >lou5=int32#12,<lod12=stack32#45
# asm 2: ldr >lou5=r11,<lod12=[sp,#176]
# copy-collector input: ldr r11,[sp,#176]

# qhasm: hiu5 = hid12
# asm 1: ldr >hiu5=int32#13,<hid12=stack32#46
# asm 2: ldr >hiu5=r12,<hid12=[sp,#180]
# copy-collector input: ldr r12,[sp,#180]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
strd r1,r2,[sp,#276]
ldr r1,[sp,#120]
ldr r2,[sp,#124]
ldr r5,[sp,#128]
ldr r6,[sp,#132]
ldr.w r7,[sp,#12]
ldr r8,[sp,#72]
ldr r11,[sp,#176]
ldr r12,[sp,#180]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r12
adc r8,r8,r12

# qhasm: hitmp lotmp = lou3 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou3=int32#4,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou3=r3,<two23=r14
umull r12,r11,r3,r14

# qhasm: lotmp hitmp += hiu3 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu3=int32#5,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu3=r4,<two23=r14
umlal r11,r12,r4,r14

# qhasm: lotmp ^= (lou3 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou3=int32#4,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou3=r3,LSR #18
eor r12,r12,r3,LSR #18

# qhasm: lotmp ^= (hiu3 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu3=int32#5,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu3=r4,LSL #14
eor r12,r12,r4,LSL #14

# qhasm: lotmp ^= (lou3 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou3=int32#4,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou3=r3,LSR #14
eor r12,r12,r3,LSR #14

# qhasm: lotmp ^= (hiu3 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu3=int32#5,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu3=r4,LSL #18
eor r12,r12,r4,LSL #18

# qhasm: hitmp ^= (hiu3 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSR #18
eor r11,r11,r4,LSR #18

# qhasm: hitmp ^= (lou3 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou3=int32#4,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou3=r3,LSL #14
eor r11,r11,r3,LSL #14

# qhasm: hitmp ^= (hiu3 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSR #14
eor r11,r11,r4,LSR #14

# qhasm: hitmp ^= (lou3 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou3=int32#4,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou3=r3,LSL #18
eor r11,r11,r3,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#13
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r12
adds r7,r7,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#12
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r11
adc r8,r8,r11

# qhasm: lotmp = lou3 & lou0
# asm 1: and >lotmp=int32#2,<lou3=int32#4,<lou0=int32#2
# asm 2: and >lotmp=r1,<lou3=r3,<lou0=r1
and r1,r3,r1

# qhasm: lotmp2 = lou1 & ~lou3
# asm 1: bic >lotmp2=int32#4,<lou1=int32#6,<lou3=int32#4
# asm 2: bic >lotmp2=r3,<lou1=r5,<lou3=r3
bic r3,r5,r3

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lotmp2=int32#4
# asm 2: eor >lotmp=r1,<lotmp=r1,<lotmp2=r3
eor r1,r1,r3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#8,<lotmp=int32#2
# asm 2: adds >lou4=r1,<lou4=r7,<lotmp=r1
adds.w r1,r7,r1

# qhasm: hitmp = hiu3 & hiu0
# asm 1: and >hitmp=int32#3,<hiu3=int32#5,<hiu0=int32#3
# asm 2: and >hitmp=r2,<hiu3=r4,<hiu0=r2
and r2,r4,r2

# qhasm: hitmp2 = hiu1 & ~hiu3
# asm 1: bic >hitmp2=int32#4,<hiu1=int32#7,<hiu3=int32#5
# asm 2: bic >hitmp2=r3,<hiu1=r6,<hiu3=r4
bic r3,r6,r4

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hitmp2=int32#4
# asm 2: eor >hitmp=r2,<hitmp=r2,<hitmp2=r3
eor r2,r2,r3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#9,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r8,<hitmp=r2
adc r2,r8,r2

# qhasm: carry? lou2 += lou4
# asm 1: adds >lou2=int32#4,<lou2=int32#10,<lou4=int32#2
# asm 2: adds >lou2=r3,<lou2=r9,<lou4=r1
adds r3,r9,r1

# qhasm: hiu2 += hiu4 + carry
# asm 1: adc >hiu2=int32#5,<hiu2=int32#11,<hiu4=int32#3
# asm 2: adc >hiu2=r4,<hiu2=r10,<hiu4=r2
adc r4,r10,r2

# qhasm: lod7 = lou2
# asm 1: str <lou2=int32#4,>lod7=stack32#35
# asm 2: str <lou2=r3,>lod7=[sp,#136]
# copy-collector input: str r3,[sp,#136]

# qhasm: hid7 = hiu2
# asm 1: str <hiu2=int32#5,>hid7=stack32#36
# asm 2: str <hiu2=r4,>hid7=[sp,#140]
# copy-collector input: str r4,[sp,#140]

# qhasm: lou0 = lod4
# asm 1: ldr >lou0=int32#6,<lod4=stack32#70
# asm 2: ldr >lou0=r5,<lod4=[sp,#276]
# copy-collector input: ldr r5,[sp,#276]

# qhasm: hiu0 = hid4
# asm 1: ldr >hiu0=int32#7,<hid4=stack32#71
# asm 2: ldr >hiu0=r6,<hid4=[sp,#280]
# copy-collector input: ldr r6,[sp,#280]

# qhasm: lou1 = lod5
# asm 1: ldr >lou1=int32#8,<lod5=stack32#13
# asm 2: ldr >lou1=r7,<lod5=[sp,#48]
# copy-collector input: ldr r7,[sp,#48]

# qhasm: hiu1 = hid5
# asm 1: ldr >hiu1=int32#9,<hid5=stack32#14
# asm 2: ldr >hiu1=r8,<hid5=[sp,#52]
# copy-collector input: ldr r8,[sp,#52]

# qhasm: lou3 = lod6
# asm 1: ldr >lou3=int32#10,<lod6=stack32#15
# asm 2: ldr >lou3=r9,<lod6=[sp,#56]
# copy-collector input: ldr r9,[sp,#56]

# qhasm: hiu3 = hid6
# asm 1: ldr >hiu3=int32#11,<hid6=stack32#16
# asm 2: ldr >hiu3=r10,<hid6=[sp,#60]
# copy-collector input: ldr r10,[sp,#60]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#12,0x2000000
# asm 2: mov >two25=r11,0x2000000
# copy-collector output starts
strd r3,r4,[sp,#136]
ldr r5,[sp,#276]
ldr r6,[sp,#280]
ldr.w r7,[sp,#48]
ldr r8,[sp,#52]
ldr r9,[sp,#56]
ldr r10,[sp,#60]
# copy-collector output ends
mov r11,0x2000000

# qhasm: hitmp lotmp = lou0 * two25
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<lou0=int32#6,<two25=int32#12
# asm 2: umull >lotmp=r14,>hitmp=r12,<lou0=r5,<two25=r11
umull r14,r12,r5,r11

# qhasm: lotmp hitmp += hiu0 * two25
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<hiu0=int32#7,<two25=int32#12
# asm 2: umlal <hitmp=r12,<lotmp=r14,<hiu0=r6,<two25=r11
umlal r12,r14,r6,r11

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#14,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r14,<hiu0=r6,LSR #2
eor r11,r14,r6,LSR #2

# qhasm: lotmp ^= (lou0 << 30)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #30
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #30
eor r11,r11,r5,LSL #30

# qhasm: lotmp ^= (lou0 unsigned>> 28)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSR #28
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSR #28
eor r11,r11,r5,LSR #28

# qhasm: lotmp ^= (hiu0 << 4)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSL #4
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSL #4
eor r11,r11,r6,LSL #4

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSR #2
eor r12,r12,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSL #30
eor r12,r12,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSR #28
eor r12,r12,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSL #4
eor r12,r12,r5,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lotmp = lou1 ^ lou3
# asm 1: eor >lotmp=int32#12,<lou1=int32#8,<lou3=int32#10
# asm 2: eor >lotmp=r11,<lou1=r7,<lou3=r9
eor r11,r7,r9

# qhasm: lotmp &= lou0
# asm 1: and >lotmp=int32#6,<lotmp=int32#12,<lou0=int32#6
# asm 2: and >lotmp=r5,<lotmp=r11,<lou0=r5
and r5,r11,r5

# qhasm: lotmp2 = lou1 & lou3
# asm 1: and >lotmp2=int32#8,<lou1=int32#8,<lou3=int32#10
# asm 2: and >lotmp2=r7,<lou1=r7,<lou3=r9
and r7,r7,r9

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#6,<lotmp=int32#6,<lotmp2=int32#8
# asm 2: eor >lotmp=r5,<lotmp=r5,<lotmp2=r7
eor r5,r5,r7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#6
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r5
adds.w r1,r1,r5

# qhasm: hitmp = hiu1 ^ hiu3
# asm 1: eor >hitmp=int32#6,<hiu1=int32#9,<hiu3=int32#11
# asm 2: eor >hitmp=r5,<hiu1=r8,<hiu3=r10
eor r5,r8,r10

# qhasm: hitmp &= hiu0
# asm 1: and >hitmp=int32#6,<hitmp=int32#6,<hiu0=int32#7
# asm 2: and >hitmp=r5,<hitmp=r5,<hiu0=r6
and r5,r5,r6

# qhasm: hitmp2 = hiu1 & hiu3
# asm 1: and >hitmp2=int32#7,<hiu1=int32#9,<hiu3=int32#11
# asm 2: and >hitmp2=r6,<hiu1=r8,<hiu3=r10
and r6,r8,r10

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#6,<hitmp=int32#6,<hitmp2=int32#7
# asm 2: eor >hitmp=r5,<hitmp=r5,<hitmp2=r6
eor r5,r5,r6

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#6
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r5
adc r2,r2,r5

# qhasm: lod3 = lou4
# asm 1: str <lou4=int32#2,>lod3=stack32#19
# asm 2: str <lou4=r1,>lod3=[sp,#72]
# copy-collector input: str r1,[sp,#72]

# qhasm: hid3 = hiu4
# asm 1: str <hiu4=int32#3,>hid3=stack32#20
# asm 2: str <hiu4=r2,>hid3=[sp,#76]
# copy-collector input: str r2,[sp,#76]

# qhasm: lou0 = lod0
# asm 1: ldr >lou0=int32#2,<lod0=stack32#29
# asm 2: ldr >lou0=r1,<lod0=[sp,#112]
# copy-collector input: ldr r1,[sp,#112]

# qhasm: hiu0 = hid0
# asm 1: ldr >hiu0=int32#3,<hid0=stack32#30
# asm 2: ldr >hiu0=r2,<hid0=[sp,#116]
# copy-collector input: ldr r2,[sp,#116]

# qhasm: lou1 = lod1
# asm 1: ldr >lou1=int32#6,<lod1=stack32#31
# asm 2: ldr >lou1=r5,<lod1=[sp,#120]
# copy-collector input: ldr r5,[sp,#120]

# qhasm: hiu1 = hid1
# asm 1: ldr >hiu1=int32#7,<hid1=stack32#32
# asm 2: ldr >hiu1=r6,<hid1=[sp,#124]
# copy-collector input: ldr r6,[sp,#124]

# qhasm: lou4 = lod2
# asm 1: ldr >lou4=int32#8,<lod2=stack32#33
# asm 2: ldr >lou4=r7,<lod2=[sp,#128]
# copy-collector input: ldr r7,[sp,#128]

# qhasm: hiu4 = hid2
# asm 1: ldr >hiu4=int32#9,<hid2=stack32#34
# asm 2: ldr >hiu4=r8,<hid2=[sp,#132]
# copy-collector input: ldr r8,[sp,#132]

# qhasm: lou5 = lod13
# asm 1: ldr >lou5=int32#12,<lod13=stack32#47
# asm 2: ldr >lou5=r11,<lod13=[sp,#184]
# copy-collector input: ldr r11,[sp,#184]

# qhasm: hiu5 = hid13
# asm 1: ldr >hiu5=int32#13,<hid13=stack32#48
# asm 2: ldr >hiu5=r12,<hid13=[sp,#188]
# copy-collector input: ldr r12,[sp,#188]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
strd r1,r2,[sp,#72]
ldr r1,[sp,#112]
ldr r2,[sp,#116]
ldr r5,[sp,#120]
ldr r6,[sp,#124]
ldr.w r7,[sp,#128]
ldr r8,[sp,#132]
ldr r11,[sp,#184]
ldr r12,[sp,#188]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r12
adc r8,r8,r12

# qhasm: hitmp lotmp = lou2 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou2=int32#4,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou2=r3,<two23=r14
umull r12,r11,r3,r14

# qhasm: lotmp hitmp += hiu2 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu2=int32#5,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu2=r4,<two23=r14
umlal r11,r12,r4,r14

# qhasm: lotmp ^= (lou2 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou2=int32#4,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou2=r3,LSR #18
eor r12,r12,r3,LSR #18

# qhasm: lotmp ^= (hiu2 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu2=int32#5,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu2=r4,LSL #14
eor r12,r12,r4,LSL #14

# qhasm: lotmp ^= (lou2 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou2=int32#4,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou2=r3,LSR #14
eor r12,r12,r3,LSR #14

# qhasm: lotmp ^= (hiu2 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu2=int32#5,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu2=r4,LSL #18
eor r12,r12,r4,LSL #18

# qhasm: hitmp ^= (hiu2 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSR #18
eor r11,r11,r4,LSR #18

# qhasm: hitmp ^= (lou2 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou2=int32#4,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou2=r3,LSL #14
eor r11,r11,r3,LSL #14

# qhasm: hitmp ^= (hiu2 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSR #14
eor r11,r11,r4,LSR #14

# qhasm: hitmp ^= (lou2 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou2=int32#4,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou2=r3,LSL #18
eor r11,r11,r3,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#13
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r12
adds r7,r7,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#12
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r11
adc r8,r8,r11

# qhasm: lotmp = lou2 & lou0
# asm 1: and >lotmp=int32#2,<lou2=int32#4,<lou0=int32#2
# asm 2: and >lotmp=r1,<lou2=r3,<lou0=r1
and r1,r3,r1

# qhasm: lotmp2 = lou1 & ~lou2
# asm 1: bic >lotmp2=int32#4,<lou1=int32#6,<lou2=int32#4
# asm 2: bic >lotmp2=r3,<lou1=r5,<lou2=r3
bic r3,r5,r3

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lotmp2=int32#4
# asm 2: eor >lotmp=r1,<lotmp=r1,<lotmp2=r3
eor r1,r1,r3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#8,<lotmp=int32#2
# asm 2: adds >lou4=r1,<lou4=r7,<lotmp=r1
adds.w r1,r7,r1

# qhasm: hitmp = hiu2 & hiu0
# asm 1: and >hitmp=int32#3,<hiu2=int32#5,<hiu0=int32#3
# asm 2: and >hitmp=r2,<hiu2=r4,<hiu0=r2
and r2,r4,r2

# qhasm: hitmp2 = hiu1 & ~hiu2
# asm 1: bic >hitmp2=int32#4,<hiu1=int32#7,<hiu2=int32#5
# asm 2: bic >hitmp2=r3,<hiu1=r6,<hiu2=r4
bic r3,r6,r4

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hitmp2=int32#4
# asm 2: eor >hitmp=r2,<hitmp=r2,<hitmp2=r3
eor r2,r2,r3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#9,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r8,<hitmp=r2
adc r2,r8,r2

# qhasm: carry? lou3 += lou4
# asm 1: adds >lou3=int32#4,<lou3=int32#10,<lou4=int32#2
# asm 2: adds >lou3=r3,<lou3=r9,<lou4=r1
adds r3,r9,r1

# qhasm: hiu3 += hiu4 + carry
# asm 1: adc >hiu3=int32#5,<hiu3=int32#11,<hiu4=int32#3
# asm 2: adc >hiu3=r4,<hiu3=r10,<hiu4=r2
adc r4,r10,r2

# qhasm: lod6 = lou3
# asm 1: str <lou3=int32#4,>lod6=stack32#33
# asm 2: str <lou3=r3,>lod6=[sp,#128]
# copy-collector input: str r3,[sp,#128]

# qhasm: hid6 = hiu3
# asm 1: str <hiu3=int32#5,>hid6=stack32#34
# asm 2: str <hiu3=r4,>hid6=[sp,#132]
# copy-collector input: str r4,[sp,#132]

# qhasm: lou0 = lod3
# asm 1: ldr >lou0=int32#6,<lod3=stack32#19
# asm 2: ldr >lou0=r5,<lod3=[sp,#72]
# copy-collector input: ldr r5,[sp,#72]

# qhasm: hiu0 = hid3
# asm 1: ldr >hiu0=int32#7,<hid3=stack32#20
# asm 2: ldr >hiu0=r6,<hid3=[sp,#76]
# copy-collector input: ldr r6,[sp,#76]

# qhasm: lou1 = lod4
# asm 1: ldr >lou1=int32#8,<lod4=stack32#70
# asm 2: ldr >lou1=r7,<lod4=[sp,#276]
# copy-collector input: ldr r7,[sp,#276]

# qhasm: hiu1 = hid4
# asm 1: ldr >hiu1=int32#9,<hid4=stack32#71
# asm 2: ldr >hiu1=r8,<hid4=[sp,#280]
# copy-collector input: ldr r8,[sp,#280]

# qhasm: lou2 = lod5
# asm 1: ldr >lou2=int32#10,<lod5=stack32#13
# asm 2: ldr >lou2=r9,<lod5=[sp,#48]
# copy-collector input: ldr r9,[sp,#48]

# qhasm: hiu2 = hid5
# asm 1: ldr >hiu2=int32#11,<hid5=stack32#14
# asm 2: ldr >hiu2=r10,<hid5=[sp,#52]
# copy-collector input: ldr r10,[sp,#52]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#12,0x2000000
# asm 2: mov >two25=r11,0x2000000
# copy-collector output starts
strd r3,r4,[sp,#128]
ldr r5,[sp,#72]
ldr r6,[sp,#76]
ldr.w r7,[sp,#276]
ldr r8,[sp,#280]
ldr r9,[sp,#48]
ldr r10,[sp,#52]
# copy-collector output ends
mov r11,0x2000000

# qhasm: hitmp lotmp = lou0 * two25
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<lou0=int32#6,<two25=int32#12
# asm 2: umull >lotmp=r14,>hitmp=r12,<lou0=r5,<two25=r11
umull r14,r12,r5,r11

# qhasm: lotmp hitmp += hiu0 * two25
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<hiu0=int32#7,<two25=int32#12
# asm 2: umlal <hitmp=r12,<lotmp=r14,<hiu0=r6,<two25=r11
umlal r12,r14,r6,r11

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#14,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r14,<hiu0=r6,LSR #2
eor r11,r14,r6,LSR #2

# qhasm: lotmp ^= (lou0 << 30)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #30
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #30
eor r11,r11,r5,LSL #30

# qhasm: lotmp ^= (lou0 unsigned>> 28)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSR #28
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSR #28
eor r11,r11,r5,LSR #28

# qhasm: lotmp ^= (hiu0 << 4)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSL #4
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSL #4
eor r11,r11,r6,LSL #4

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSR #2
eor r12,r12,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSL #30
eor r12,r12,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSR #28
eor r12,r12,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSL #4
eor r12,r12,r5,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lotmp = lou1 ^ lou2
# asm 1: eor >lotmp=int32#12,<lou1=int32#8,<lou2=int32#10
# asm 2: eor >lotmp=r11,<lou1=r7,<lou2=r9
eor r11,r7,r9

# qhasm: lotmp &= lou0
# asm 1: and >lotmp=int32#6,<lotmp=int32#12,<lou0=int32#6
# asm 2: and >lotmp=r5,<lotmp=r11,<lou0=r5
and r5,r11,r5

# qhasm: lotmp2 = lou1 & lou2
# asm 1: and >lotmp2=int32#8,<lou1=int32#8,<lou2=int32#10
# asm 2: and >lotmp2=r7,<lou1=r7,<lou2=r9
and r7,r7,r9

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#6,<lotmp=int32#6,<lotmp2=int32#8
# asm 2: eor >lotmp=r5,<lotmp=r5,<lotmp2=r7
eor r5,r5,r7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#6
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r5
adds.w r1,r1,r5

# qhasm: hitmp = hiu1 ^ hiu2
# asm 1: eor >hitmp=int32#6,<hiu1=int32#9,<hiu2=int32#11
# asm 2: eor >hitmp=r5,<hiu1=r8,<hiu2=r10
eor r5,r8,r10

# qhasm: hitmp &= hiu0
# asm 1: and >hitmp=int32#6,<hitmp=int32#6,<hiu0=int32#7
# asm 2: and >hitmp=r5,<hitmp=r5,<hiu0=r6
and r5,r5,r6

# qhasm: hitmp2 = hiu1 & hiu2
# asm 1: and >hitmp2=int32#7,<hiu1=int32#9,<hiu2=int32#11
# asm 2: and >hitmp2=r6,<hiu1=r8,<hiu2=r10
and r6,r8,r10

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#6,<hitmp=int32#6,<hitmp2=int32#7
# asm 2: eor >hitmp=r5,<hitmp=r5,<hitmp2=r6
eor r5,r5,r6

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#6
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r5
adc r2,r2,r5

# qhasm: lod2 = lou4
# asm 1: str <lou4=int32#2,>lod2=stack32#17
# asm 2: str <lou4=r1,>lod2=[sp,#64]
# copy-collector input: str r1,[sp,#64]

# qhasm: hid2 = hiu4
# asm 1: str <hiu4=int32#3,>hid2=stack32#18
# asm 2: str <hiu4=r2,>hid2=[sp,#68]
# copy-collector input: str r2,[sp,#68]

# qhasm: lou0 = lod7
# asm 1: ldr >lou0=int32#2,<lod7=stack32#35
# asm 2: ldr >lou0=r1,<lod7=[sp,#136]
# copy-collector input: ldr r1,[sp,#136]

# qhasm: hiu0 = hid7
# asm 1: ldr >hiu0=int32#3,<hid7=stack32#36
# asm 2: ldr >hiu0=r2,<hid7=[sp,#140]
# copy-collector input: ldr r2,[sp,#140]

# qhasm: lou1 = lod0
# asm 1: ldr >lou1=int32#6,<lod0=stack32#29
# asm 2: ldr >lou1=r5,<lod0=[sp,#112]
# copy-collector input: ldr r5,[sp,#112]

# qhasm: hiu1 = hid0
# asm 1: ldr >hiu1=int32#7,<hid0=stack32#30
# asm 2: ldr >hiu1=r6,<hid0=[sp,#116]
# copy-collector input: ldr r6,[sp,#116]

# qhasm: lou4 = lod1
# asm 1: ldr >lou4=int32#8,<lod1=stack32#31
# asm 2: ldr >lou4=r7,<lod1=[sp,#120]
# copy-collector input: ldr r7,[sp,#120]

# qhasm: hiu4 = hid1
# asm 1: ldr >hiu4=int32#9,<hid1=stack32#32
# asm 2: ldr >hiu4=r8,<hid1=[sp,#124]
# copy-collector input: ldr r8,[sp,#124]

# qhasm: lou5 = lod14
# asm 1: ldr >lou5=int32#12,<lod14=stack32#49
# asm 2: ldr >lou5=r11,<lod14=[sp,#192]
# copy-collector input: ldr r11,[sp,#192]

# qhasm: hiu5 = hid14
# asm 1: ldr >hiu5=int32#13,<hid14=stack32#50
# asm 2: ldr >hiu5=r12,<hid14=[sp,#196]
# copy-collector input: ldr r12,[sp,#196]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
strd r1,r2,[sp,#64]
ldr r1,[sp,#136]
ldr r2,[sp,#140]
ldr r5,[sp,#112]
ldr r6,[sp,#116]
ldr.w r7,[sp,#120]
ldr r8,[sp,#124]
ldr r11,[sp,#192]
ldr r12,[sp,#196]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r12
adc r8,r8,r12

# qhasm: hitmp lotmp = lou3 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou3=int32#4,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou3=r3,<two23=r14
umull r12,r11,r3,r14

# qhasm: lotmp hitmp += hiu3 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu3=int32#5,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu3=r4,<two23=r14
umlal r11,r12,r4,r14

# qhasm: lotmp ^= (lou3 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou3=int32#4,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou3=r3,LSR #18
eor r12,r12,r3,LSR #18

# qhasm: lotmp ^= (hiu3 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu3=int32#5,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu3=r4,LSL #14
eor r12,r12,r4,LSL #14

# qhasm: lotmp ^= (lou3 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou3=int32#4,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou3=r3,LSR #14
eor r12,r12,r3,LSR #14

# qhasm: lotmp ^= (hiu3 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu3=int32#5,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu3=r4,LSL #18
eor r12,r12,r4,LSL #18

# qhasm: hitmp ^= (hiu3 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSR #18
eor r11,r11,r4,LSR #18

# qhasm: hitmp ^= (lou3 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou3=int32#4,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou3=r3,LSL #14
eor r11,r11,r3,LSL #14

# qhasm: hitmp ^= (hiu3 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSR #14
eor r11,r11,r4,LSR #14

# qhasm: hitmp ^= (lou3 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou3=int32#4,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou3=r3,LSL #18
eor r11,r11,r3,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#13
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r12
adds r7,r7,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#12
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r11
adc r8,r8,r11

# qhasm: lotmp = lou3 & lou0
# asm 1: and >lotmp=int32#2,<lou3=int32#4,<lou0=int32#2
# asm 2: and >lotmp=r1,<lou3=r3,<lou0=r1
and r1,r3,r1

# qhasm: lotmp2 = lou1 & ~lou3
# asm 1: bic >lotmp2=int32#4,<lou1=int32#6,<lou3=int32#4
# asm 2: bic >lotmp2=r3,<lou1=r5,<lou3=r3
bic r3,r5,r3

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lotmp2=int32#4
# asm 2: eor >lotmp=r1,<lotmp=r1,<lotmp2=r3
eor r1,r1,r3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#8,<lotmp=int32#2
# asm 2: adds >lou4=r1,<lou4=r7,<lotmp=r1
adds.w r1,r7,r1

# qhasm: hitmp = hiu3 & hiu0
# asm 1: and >hitmp=int32#3,<hiu3=int32#5,<hiu0=int32#3
# asm 2: and >hitmp=r2,<hiu3=r4,<hiu0=r2
and r2,r4,r2

# qhasm: hitmp2 = hiu1 & ~hiu3
# asm 1: bic >hitmp2=int32#4,<hiu1=int32#7,<hiu3=int32#5
# asm 2: bic >hitmp2=r3,<hiu1=r6,<hiu3=r4
bic r3,r6,r4

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hitmp2=int32#4
# asm 2: eor >hitmp=r2,<hitmp=r2,<hitmp2=r3
eor r2,r2,r3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#9,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r8,<hitmp=r2
adc r2,r8,r2

# qhasm: carry? lou2 += lou4
# asm 1: adds >lou2=int32#4,<lou2=int32#10,<lou4=int32#2
# asm 2: adds >lou2=r3,<lou2=r9,<lou4=r1
adds r3,r9,r1

# qhasm: hiu2 += hiu4 + carry
# asm 1: adc >hiu2=int32#5,<hiu2=int32#11,<hiu4=int32#3
# asm 2: adc >hiu2=r4,<hiu2=r10,<hiu4=r2
adc r4,r10,r2

# qhasm: lod5 = lou2
# asm 1: str <lou2=int32#4,>lod5=stack32#31
# asm 2: str <lou2=r3,>lod5=[sp,#120]
# copy-collector input: str r3,[sp,#120]

# qhasm: hid5 = hiu2
# asm 1: str <hiu2=int32#5,>hid5=stack32#32
# asm 2: str <hiu2=r4,>hid5=[sp,#124]
# copy-collector input: str r4,[sp,#124]

# qhasm: lou0 = lod2
# asm 1: ldr >lou0=int32#6,<lod2=stack32#17
# asm 2: ldr >lou0=r5,<lod2=[sp,#64]
# copy-collector input: ldr r5,[sp,#64]

# qhasm: hiu0 = hid2
# asm 1: ldr >hiu0=int32#7,<hid2=stack32#18
# asm 2: ldr >hiu0=r6,<hid2=[sp,#68]
# copy-collector input: ldr r6,[sp,#68]

# qhasm: lou1 = lod3
# asm 1: ldr >lou1=int32#8,<lod3=stack32#19
# asm 2: ldr >lou1=r7,<lod3=[sp,#72]
# copy-collector input: ldr r7,[sp,#72]

# qhasm: hiu1 = hid3
# asm 1: ldr >hiu1=int32#9,<hid3=stack32#20
# asm 2: ldr >hiu1=r8,<hid3=[sp,#76]
# copy-collector input: ldr r8,[sp,#76]

# qhasm: lou3 = lod4
# asm 1: ldr >lou3=int32#10,<lod4=stack32#70
# asm 2: ldr >lou3=r9,<lod4=[sp,#276]
# copy-collector input: ldr r9,[sp,#276]

# qhasm: hiu3 = hid4
# asm 1: ldr >hiu3=int32#11,<hid4=stack32#71
# asm 2: ldr >hiu3=r10,<hid4=[sp,#280]
# copy-collector input: ldr r10,[sp,#280]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#12,0x2000000
# asm 2: mov >two25=r11,0x2000000
# copy-collector output starts
strd r3,r4,[sp,#120]
ldr r5,[sp,#64]
ldr r6,[sp,#68]
ldr.w r7,[sp,#72]
ldr r8,[sp,#76]
ldr r9,[sp,#276]
ldr r10,[sp,#280]
# copy-collector output ends
mov r11,0x2000000

# qhasm: hitmp lotmp = lou0 * two25
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<lou0=int32#6,<two25=int32#12
# asm 2: umull >lotmp=r14,>hitmp=r12,<lou0=r5,<two25=r11
umull r14,r12,r5,r11

# qhasm: lotmp hitmp += hiu0 * two25
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<hiu0=int32#7,<two25=int32#12
# asm 2: umlal <hitmp=r12,<lotmp=r14,<hiu0=r6,<two25=r11
umlal r12,r14,r6,r11

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#14,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r14,<hiu0=r6,LSR #2
eor r11,r14,r6,LSR #2

# qhasm: lotmp ^= (lou0 << 30)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #30
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #30
eor r11,r11,r5,LSL #30

# qhasm: lotmp ^= (lou0 unsigned>> 28)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSR #28
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSR #28
eor r11,r11,r5,LSR #28

# qhasm: lotmp ^= (hiu0 << 4)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSL #4
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSL #4
eor r11,r11,r6,LSL #4

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSR #2
eor r12,r12,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSL #30
eor r12,r12,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r6,LSR #28
eor r12,r12,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r5,LSL #4
eor r12,r12,r5,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lotmp = lou1 ^ lou3
# asm 1: eor >lotmp=int32#12,<lou1=int32#8,<lou3=int32#10
# asm 2: eor >lotmp=r11,<lou1=r7,<lou3=r9
eor r11,r7,r9

# qhasm: lotmp &= lou0
# asm 1: and >lotmp=int32#6,<lotmp=int32#12,<lou0=int32#6
# asm 2: and >lotmp=r5,<lotmp=r11,<lou0=r5
and r5,r11,r5

# qhasm: lotmp2 = lou1 & lou3
# asm 1: and >lotmp2=int32#8,<lou1=int32#8,<lou3=int32#10
# asm 2: and >lotmp2=r7,<lou1=r7,<lou3=r9
and r7,r7,r9

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#6,<lotmp=int32#6,<lotmp2=int32#8
# asm 2: eor >lotmp=r5,<lotmp=r5,<lotmp2=r7
eor r5,r5,r7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#6
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r5
adds.w r1,r1,r5

# qhasm: hitmp = hiu1 ^ hiu3
# asm 1: eor >hitmp=int32#6,<hiu1=int32#9,<hiu3=int32#11
# asm 2: eor >hitmp=r5,<hiu1=r8,<hiu3=r10
eor r5,r8,r10

# qhasm: hitmp &= hiu0
# asm 1: and >hitmp=int32#6,<hitmp=int32#6,<hiu0=int32#7
# asm 2: and >hitmp=r5,<hitmp=r5,<hiu0=r6
and r5,r5,r6

# qhasm: hitmp2 = hiu1 & hiu3
# asm 1: and >hitmp2=int32#7,<hiu1=int32#9,<hiu3=int32#11
# asm 2: and >hitmp2=r6,<hiu1=r8,<hiu3=r10
and r6,r8,r10

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#6,<hitmp=int32#6,<hitmp2=int32#7
# asm 2: eor >hitmp=r5,<hitmp=r5,<hitmp2=r6
eor r5,r5,r6

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#6
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r5
adc r2,r2,r5

# qhasm: lod1 = lou4
# asm 1: str <lou4=int32#2,>lod1=stack32#15
# asm 2: str <lou4=r1,>lod1=[sp,#56]
# copy-collector input: str r1,[sp,#56]

# qhasm: hid1 = hiu4
# asm 1: str <hiu4=int32#3,>hid1=stack32#16
# asm 2: str <hiu4=r2,>hid1=[sp,#60]
# copy-collector input: str r2,[sp,#60]

# qhasm: lou0 = lod6
# asm 1: ldr >lou0=int32#2,<lod6=stack32#33
# asm 2: ldr >lou0=r1,<lod6=[sp,#128]
# copy-collector input: ldr r1,[sp,#128]

# qhasm: hiu0 = hid6
# asm 1: ldr >hiu0=int32#3,<hid6=stack32#34
# asm 2: ldr >hiu0=r2,<hid6=[sp,#132]
# copy-collector input: ldr r2,[sp,#132]

# qhasm: lou1 = lod7
# asm 1: ldr >lou1=int32#6,<lod7=stack32#35
# asm 2: ldr >lou1=r5,<lod7=[sp,#136]
# copy-collector input: ldr r5,[sp,#136]

# qhasm: hiu1 = hid7
# asm 1: ldr >hiu1=int32#7,<hid7=stack32#36
# asm 2: ldr >hiu1=r6,<hid7=[sp,#140]
# copy-collector input: ldr r6,[sp,#140]

# qhasm: lou4 = lod0
# asm 1: ldr >lou4=int32#8,<lod0=stack32#29
# asm 2: ldr >lou4=r7,<lod0=[sp,#112]
# copy-collector input: ldr r7,[sp,#112]

# qhasm: hiu4 = hid0
# asm 1: ldr >hiu4=int32#9,<hid0=stack32#30
# asm 2: ldr >hiu4=r8,<hid0=[sp,#116]
# copy-collector input: ldr r8,[sp,#116]

# qhasm: lou5 = lod15
# asm 1: ldr >lou5=int32#12,<lod15=stack32#51
# asm 2: ldr >lou5=r11,<lod15=[sp,#200]
# copy-collector input: ldr r11,[sp,#200]

# qhasm: hiu5 = hid15
# asm 1: ldr >hiu5=int32#13,<hid15=stack32#52
# asm 2: ldr >hiu5=r12,<hid15=[sp,#204]
# copy-collector input: ldr r12,[sp,#204]

# qhasm: two23 = 0x800000 simple
# asm 1: mov >two23=int32#14,0x800000
# asm 2: mov >two23=r14,0x800000
# copy-collector output starts
strd r1,r2,[sp,#56]
ldr r1,[sp,#128]
ldr r2,[sp,#132]
ldr r5,[sp,#136]
ldr r6,[sp,#140]
ldr.w r7,[sp,#112]
ldr r8,[sp,#116]
ldr r11,[sp,#200]
ldr r12,[sp,#204]
# copy-collector output ends
mov r14,0x800000

# qhasm: carry?  lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >lotmp=int32#12,[<input_0=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_0=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_0]; input_0 += 4
# asm 1: ldr >hitmp=int32#13,[<input_0=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_0=r0],#4
# copy-collector input: ldr r12,[r0],#4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
# copy-collector output starts
ldr r11,[r0],#4
ldr r12,[r0],#4
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r12
adc r8,r8,r12

# qhasm: hitmp lotmp = lou2 * two23
# asm 1: umull >lotmp=int32#13,>hitmp=int32#12,<lou2=int32#4,<two23=int32#14
# asm 2: umull >lotmp=r12,>hitmp=r11,<lou2=r3,<two23=r14
umull r12,r11,r3,r14

# qhasm: lotmp hitmp += hiu2 * two23
# asm 1: umlal <hitmp=int32#12,<lotmp=int32#13,<hiu2=int32#5,<two23=int32#14
# asm 2: umlal <hitmp=r11,<lotmp=r12,<hiu2=r4,<two23=r14
umlal r11,r12,r4,r14

# qhasm: lotmp ^= (lou2 unsigned>> 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou2=int32#4,LSR #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou2=r3,LSR #18
eor r12,r12,r3,LSR #18

# qhasm: lotmp ^= (hiu2 << 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu2=int32#5,LSL #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu2=r4,LSL #14
eor r12,r12,r4,LSL #14

# qhasm: lotmp ^= (lou2 unsigned>> 14)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<lou2=int32#4,LSR #14
# asm 2: eor >lotmp=r12,<lotmp=r12,<lou2=r3,LSR #14
eor r12,r12,r3,LSR #14

# qhasm: lotmp ^= (hiu2 << 18)
# asm 1: eor >lotmp=int32#13,<lotmp=int32#13,<hiu2=int32#5,LSL #18
# asm 2: eor >lotmp=r12,<lotmp=r12,<hiu2=r4,LSL #18
eor r12,r12,r4,LSL #18

# qhasm: hitmp ^= (hiu2 unsigned>> 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSR #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSR #18
eor r11,r11,r4,LSR #18

# qhasm: hitmp ^= (lou2 << 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou2=int32#4,LSL #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou2=r3,LSL #14
eor r11,r11,r3,LSL #14

# qhasm: hitmp ^= (hiu2 unsigned>> 14)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSR #14
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSR #14
eor r11,r11,r4,LSR #14

# qhasm: hitmp ^= (lou2 << 18)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou2=int32#4,LSL #18
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou2=r3,LSL #18
eor r11,r11,r3,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#13
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r12
adds r7,r7,r12

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hitmp=int32#12
# asm 2: adc >hiu4=r8,<hiu4=r8,<hitmp=r11
adc r8,r8,r11

# qhasm: lotmp = lou2 & lou0
# asm 1: and >lotmp=int32#2,<lou2=int32#4,<lou0=int32#2
# asm 2: and >lotmp=r1,<lou2=r3,<lou0=r1
and r1,r3,r1

# qhasm: lotmp2 = lou1 & ~lou2
# asm 1: bic >lotmp2=int32#4,<lou1=int32#6,<lou2=int32#4
# asm 2: bic >lotmp2=r3,<lou1=r5,<lou2=r3
bic r3,r5,r3

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lotmp2=int32#4
# asm 2: eor >lotmp=r1,<lotmp=r1,<lotmp2=r3
eor r1,r1,r3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#8,<lotmp=int32#2
# asm 2: adds >lou4=r1,<lou4=r7,<lotmp=r1
adds.w r1,r7,r1

# qhasm: hitmp = hiu2 & hiu0
# asm 1: and >hitmp=int32#3,<hiu2=int32#5,<hiu0=int32#3
# asm 2: and >hitmp=r2,<hiu2=r4,<hiu0=r2
and r2,r4,r2

# qhasm: hitmp2 = hiu1 & ~hiu2
# asm 1: bic >hitmp2=int32#4,<hiu1=int32#7,<hiu2=int32#5
# asm 2: bic >hitmp2=r3,<hiu1=r6,<hiu2=r4
bic r3,r6,r4

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hitmp2=int32#4
# asm 2: eor >hitmp=r2,<hitmp=r2,<hitmp2=r3
eor r2,r2,r3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#9,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r8,<hitmp=r2
adc r2,r8,r2

# qhasm: carry? lou3 += lou4
# asm 1: adds >lou3=int32#4,<lou3=int32#10,<lou4=int32#2
# asm 2: adds >lou3=r3,<lou3=r9,<lou4=r1
adds r3,r9,r1

# qhasm: hiu3 += hiu4 + carry
# asm 1: adc >hiu3=int32#5,<hiu3=int32#11,<hiu4=int32#3
# asm 2: adc >hiu3=r4,<hiu3=r10,<hiu4=r2
adc r4,r10,r2

# qhasm: lod4 = lou3
# asm 1: str <lou3=int32#4,>lod4=stack32#29
# asm 2: str <lou3=r3,>lod4=[sp,#112]
# copy-collector input: str r3,[sp,#112]

# qhasm: hid4 = hiu3
# asm 1: str <hiu3=int32#5,>hid4=stack32#30
# asm 2: str <hiu3=r4,>hid4=[sp,#116]
# copy-collector input: str r4,[sp,#116]

# qhasm: lou0 = lod1
# asm 1: ldr >lou0=int32#4,<lod1=stack32#15
# asm 2: ldr >lou0=r3,<lod1=[sp,#56]
# copy-collector input: ldr r3,[sp,#56]

# qhasm: hiu0 = hid1
# asm 1: ldr >hiu0=int32#5,<hid1=stack32#16
# asm 2: ldr >hiu0=r4,<hid1=[sp,#60]
# copy-collector input: ldr r4,[sp,#60]

# qhasm: lou1 = lod2
# asm 1: ldr >lou1=int32#6,<lod2=stack32#17
# asm 2: ldr >lou1=r5,<lod2=[sp,#64]
# copy-collector input: ldr r5,[sp,#64]

# qhasm: hiu1 = hid2
# asm 1: ldr >hiu1=int32#7,<hid2=stack32#18
# asm 2: ldr >hiu1=r6,<hid2=[sp,#68]
# copy-collector input: ldr r6,[sp,#68]

# qhasm: lou2 = lod3
# asm 1: ldr >lou2=int32#8,<lod3=stack32#19
# asm 2: ldr >lou2=r7,<lod3=[sp,#72]
# copy-collector input: ldr r7,[sp,#72]

# qhasm: hiu2 = hid3
# asm 1: ldr >hiu2=int32#9,<hid3=stack32#20
# asm 2: ldr >hiu2=r8,<hid3=[sp,#76]
# copy-collector input: ldr r8,[sp,#76]

# qhasm: two25 = 0x2000000 simple
# asm 1: mov >two25=int32#10,0x2000000
# asm 2: mov >two25=r9,0x2000000
# copy-collector output starts
strd r3,r4,[sp,#112]
ldr r3,[sp,#56]
ldr r4,[sp,#60]
ldr r5,[sp,#64]
ldr r6,[sp,#68]
ldr.w r7,[sp,#72]
ldr r8,[sp,#76]
# copy-collector output ends
mov r9,0x2000000

# qhasm: hitmp lotmp = lou0 * two25
# asm 1: umull >lotmp=int32#12,>hitmp=int32#11,<lou0=int32#4,<two25=int32#10
# asm 2: umull >lotmp=r11,>hitmp=r10,<lou0=r3,<two25=r9
umull r11,r10,r3,r9

# qhasm: lotmp hitmp += hiu0 * two25
# asm 1: umlal <hitmp=int32#11,<lotmp=int32#12,<hiu0=int32#5,<two25=int32#10
# asm 2: umlal <hitmp=r10,<lotmp=r11,<hiu0=r4,<two25=r9
umlal r10,r11,r4,r9

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#12,<hiu0=int32#5,LSR #2
# asm 2: eor >lotmp=r9,<lotmp=r11,<hiu0=r4,LSR #2
eor r9,r11,r4,LSR #2

# qhasm: lotmp ^= (lou0 << 30)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou0=int32#4,LSL #30
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou0=r3,LSL #30
eor r9,r9,r3,LSL #30

# qhasm: lotmp ^= (lou0 unsigned>> 28)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou0=int32#4,LSR #28
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou0=r3,LSR #28
eor r9,r9,r3,LSR #28

# qhasm: lotmp ^= (hiu0 << 4)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu0=int32#5,LSL #4
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu0=r4,LSL #4
eor r9,r9,r4,LSL #4

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<lou0=int32#4,LSR #2
# asm 2: eor >hitmp=r10,<hitmp=r10,<lou0=r3,LSR #2
eor r10,r10,r3,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu0=int32#5,LSL #30
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu0=r4,LSL #30
eor r10,r10,r4,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu0=int32#5,LSR #28
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu0=r4,LSR #28
eor r10,r10,r4,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<lou0=int32#4,LSL #4
# asm 2: eor >hitmp=r10,<hitmp=r10,<lou0=r3,LSL #4
eor r10,r10,r3,LSL #4

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#10
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r9
adds r1,r1,r9

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#11
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r10
adc r2,r2,r10

# qhasm: lotmp = lou1 ^ lou2
# asm 1: eor >lotmp=int32#10,<lou1=int32#6,<lou2=int32#8
# asm 2: eor >lotmp=r9,<lou1=r5,<lou2=r7
eor r9,r5,r7

# qhasm: lotmp &= lou0
# asm 1: and >lotmp=int32#4,<lotmp=int32#10,<lou0=int32#4
# asm 2: and >lotmp=r3,<lotmp=r9,<lou0=r3
and r3,r9,r3

# qhasm: lotmp2 = lou1 & lou2
# asm 1: and >lotmp2=int32#6,<lou1=int32#6,<lou2=int32#8
# asm 2: and >lotmp2=r5,<lou1=r5,<lou2=r7
and r5,r5,r7

# qhasm: lotmp ^= lotmp2
# asm 1: eor >lotmp=int32#4,<lotmp=int32#4,<lotmp2=int32#6
# asm 2: eor >lotmp=r3,<lotmp=r3,<lotmp2=r5
eor r3,r3,r5

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#4
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r3
adds.w r1,r1,r3

# qhasm: hitmp = hiu1 ^ hiu2
# asm 1: eor >hitmp=int32#4,<hiu1=int32#7,<hiu2=int32#9
# asm 2: eor >hitmp=r3,<hiu1=r6,<hiu2=r8
eor r3,r6,r8

# qhasm: hitmp &= hiu0
# asm 1: and >hitmp=int32#4,<hitmp=int32#4,<hiu0=int32#5
# asm 2: and >hitmp=r3,<hitmp=r3,<hiu0=r4
and r3,r3,r4

# qhasm: hitmp2 = hiu1 & hiu2
# asm 1: and >hitmp2=int32#5,<hiu1=int32#7,<hiu2=int32#9
# asm 2: and >hitmp2=r4,<hiu1=r6,<hiu2=r8
and r4,r6,r8

# qhasm: hitmp ^= hitmp2
# asm 1: eor >hitmp=int32#4,<hitmp=int32#4,<hitmp2=int32#5
# asm 2: eor >hitmp=r3,<hitmp=r3,<hitmp2=r4
eor r3,r3,r4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#4
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r3
adc r2,r2,r3

# qhasm: lod0 = lou4
# asm 1: str <lou4=int32#2,>lod0=stack32#13
# asm 2: str <lou4=r1,>lod0=[sp,#48]
# copy-collector input: str r1,[sp,#48]

# qhasm: hid0 = hiu4
# asm 1: str <hiu4=int32#3,>hid0=stack32#14
# asm 2: str <hiu4=r2,>hid0=[sp,#52]
# copy-collector input: str r2,[sp,#52]

# qhasm: o3 = input_0
# asm 1: str <input_0=int32#1,>o3=stack32#4
# asm 2: str <input_0=r0,>o3=[sp,#12]
# copy-collector input: str r0,[sp,#12]

# qhasm: input_0 = o4
# asm 1: ldr >input_0=int32#1,<o4=stack32#69
# asm 2: ldr >input_0=r0,<o4=[sp,#272]
# copy-collector input: ldr r0,[sp,#272]

# qhasm: =? unsigned<? input_0 -= 8
# asm 1: subs >input_0=int32#1,<input_0=int32#1,#8
# asm 2: subs >input_0=r0,<input_0=r0,#8
# copy-collector output starts
strd r1,r2,[sp,#48]
str r0,[sp,#12]
ldr r0,[sp,#272]
# copy-collector output ends
subs r0,r0,#8

# qhasm: goto endinnerloop if =
beq ._endinnerloop

# qhasm: o4 = input_0
# asm 1: str <input_0=int32#1,>o4=stack32#69
# asm 2: str <input_0=r0,>o4=[sp,#272]
# copy-collector input: str r0,[sp,#272]

# qhasm: =? input_0 - 8
# asm 1: cmp <input_0=int32#1,#8
# asm 2: cmp <input_0=r0,#8
# copy-collector output starts
str r0,[sp,#272]
# copy-collector output ends
cmp r0,#8

# qhasm: goto nearend if =
beq ._nearend

# qhasm: two24 = 0x1000000 simple
# asm 1: mov >two24=int32#1,0x1000000
# asm 2: mov >two24=r0,0x1000000
mov r0,0x1000000

# qhasm: two13 = 0x2000 simple
# asm 1: mov >two13=int32#2,0x2000
# asm 2: mov >two13=r1,0x2000
mov r1,0x2000

# qhasm: lou0 = lod8
# asm 1: ldr >lou0=int32#3,<lod8=stack32#37
# asm 2: ldr >lou0=r2,<lod8=[sp,#144]
# copy-collector input: ldr r2,[sp,#144]

# qhasm: hiu0 = hid8
# asm 1: ldr >hiu0=int32#4,<hid8=stack32#38
# asm 2: ldr >hiu0=r3,<hid8=[sp,#148]
# copy-collector input: ldr r3,[sp,#148]

# qhasm: lou1 = lod9
# asm 1: ldr >lou1=int32#5,<lod9=stack32#39
# asm 2: ldr >lou1=r4,<lod9=[sp,#152]
# copy-collector input: ldr r4,[sp,#152]

# qhasm: hiu1 = hid9
# asm 1: ldr >hiu1=int32#6,<hid9=stack32#40
# asm 2: ldr >hiu1=r5,<hid9=[sp,#156]
# copy-collector input: ldr r5,[sp,#156]

# qhasm: lou2 = lom14
# asm 1: ldr >lou2=int32#7,<lom14=stack32#65
# asm 2: ldr >lou2=r6,<lom14=[sp,#256]
# copy-collector input: ldr r6,[sp,#256]

# qhasm: hiu2 = him14
# asm 1: ldr >hiu2=int32#8,<him14=stack32#66
# asm 2: ldr >hiu2=r7,<him14=[sp,#260]
# copy-collector input: ldr r7,[sp,#260]

# qhasm: lou3 = lom9
# asm 1: ldr >lou3=int32#9,<lom9=stack32#55
# asm 2: ldr >lou3=r8,<lom9=[sp,#216]
# copy-collector input: ldr r8,[sp,#216]

# qhasm: hiu3 = him9
# asm 1: ldr >hiu3=int32#10,<him9=stack32#56
# asm 2: ldr >hiu3=r9,<him9=[sp,#220]
# copy-collector input: ldr r9,[sp,#220]

# qhasm: hitmp lotmp = hiu2 * two13
# asm 1: umull >lotmp=int32#12,>hitmp=int32#11,<hiu2=int32#8,<two13=int32#2
# asm 2: umull >lotmp=r11,>hitmp=r10,<hiu2=r7,<two13=r1
# copy-collector output starts
ldr r2,[sp,#144]
ldr r3,[sp,#148]
ldr r4,[sp,#152]
ldr r5,[sp,#156]
ldr r6,[sp,#256]
ldr.w r7,[sp,#260]
ldr r8,[sp,#216]
ldr r9,[sp,#220]
# copy-collector output ends
umull r11,r10,r7,r1

# qhasm: lotmp hitmp += lou2 * two13
# asm 1: umlal <hitmp=int32#11,<lotmp=int32#12,<lou2=int32#7,<two13=int32#2
# asm 2: umlal <hitmp=r10,<lotmp=r11,<lou2=r6,<two13=r1
umlal r10,r11,r6,r1

# qhasm: lotmp ^= (lou2 unsigned>> 6)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#7,LSR #6
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r6,LSR #6
eor r11,r11,r6,LSR #6

# qhasm: lotmp ^= (hiu2 << 26)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#8,LSL #26
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r7,LSL #26
eor r11,r11,r7,LSL #26

# qhasm: lotmp ^= (hiu2 unsigned>> 29)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#8,LSR #29
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r7,LSR #29
eor r11,r11,r7,LSR #29

# qhasm: lotmp ^= (lou2 << 3)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#7,LSL #3
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r6,LSL #3
eor r11,r11,r6,LSL #3

# qhasm: hitmp ^= (hiu2 unsigned>> 6)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu2=int32#8,LSR #6
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu2=r7,LSR #6
eor r10,r10,r7,LSR #6

# qhasm: hitmp ^= (lou2 unsigned>> 29)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<lou2=int32#7,LSR #29
# asm 2: eor >hitmp=r10,<hitmp=r10,<lou2=r6,LSR #29
eor r10,r10,r6,LSR #29

# qhasm: hitmp ^= (hiu2 << 3)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu2=int32#8,LSL #3
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu2=r7,LSL #3
eor r10,r10,r7,LSL #3

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#3,<lou0=int32#3,<lotmp=int32#12
# asm 2: adds >lou0=r2,<lou0=r2,<lotmp=r11
adds r2,r2,r11

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#4,<hiu0=int32#4,<hitmp=int32#11
# asm 2: adc >hiu0=r3,<hiu0=r3,<hitmp=r10
adc r3,r3,r10

# qhasm: hitmp lotmp = hiu1 * two24
# asm 1: umull >lotmp=int32#12,>hitmp=int32#11,<hiu1=int32#6,<two24=int32#1
# asm 2: umull >lotmp=r11,>hitmp=r10,<hiu1=r5,<two24=r0
umull r11,r10,r5,r0

# qhasm: lotmp hitmp += lou1 * two24
# asm 1: umlal <hitmp=int32#11,<lotmp=int32#12,<lou1=int32#5,<two24=int32#1
# asm 2: umlal <hitmp=r10,<lotmp=r11,<lou1=r4,<two24=r0
umlal r10,r11,r4,r0

# qhasm: carry? lotmp ^= (lou1 unsigned>> 1)
# asm 1: eors >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#5,LSR #1
# asm 2: eors >lotmp=r11,<lotmp=r11,<lou1=r4,LSR #1
eors r11,r11,r4,LSR #1

# qhasm: hitmp ^= (carry,hiu1 unsigned>> 1)
# asm 1: eors >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#6,RRX
# asm 2: eors >hitmp=r10,<hitmp=r10,<hiu1=r5,RRX
eors r10,r10,r5,RRX

# qhasm: lotmp ^= (hiu1 << 31)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#6,LSL #31
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r5,LSL #31
eor r11,r11,r5,LSL #31

# qhasm: lotmp ^= (lou1 unsigned>>7)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#5,LSR #7
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r4,LSR #7
eor r11,r11,r4,LSR #7

# qhasm: lotmp ^= (hiu1 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: hitmp ^= (hiu1 unsigned>>7)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#6,LSR #7
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu1=r5,LSR #7
eor r10,r10,r5,LSR #7

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#3,<lou0=int32#3,<lotmp=int32#12
# asm 2: adds >lou0=r2,<lou0=r2,<lotmp=r11
adds r2,r2,r11

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#4,<hiu0=int32#4,<hitmp=int32#11
# asm 2: adc >hiu0=r3,<hiu0=r3,<hitmp=r10
adc r3,r3,r10

# qhasm: carry? lou0 += lou3
# asm 1: adds >lou0=int32#3,<lou0=int32#3,<lou3=int32#9
# asm 2: adds >lou0=r2,<lou0=r2,<lou3=r8
adds r2,r2,r8

# qhasm: hiu0 += hiu3 + carry
# asm 1: adc >hiu0=int32#4,<hiu0=int32#4,<hiu3=int32#10
# asm 2: adc >hiu0=r3,<hiu0=r3,<hiu3=r9
adc r3,r3,r9

# qhasm: lod9 = lou3
# asm 1: str <lou3=int32#9,>lod9=stack32#39
# asm 2: str <lou3=r8,>lod9=[sp,#152]
# copy-collector input: str r8,[sp,#152]

# qhasm: hid9 = hiu3
# asm 1: str <hiu3=int32#10,>hid9=stack32#40
# asm 2: str <hiu3=r9,>hid9=[sp,#156]
# copy-collector input: str r9,[sp,#156]

# qhasm: lou3 = lom15
# asm 1: ldr >lou3=int32#9,<lom15=stack32#67
# asm 2: ldr >lou3=r8,<lom15=[sp,#264]
# copy-collector input: ldr r8,[sp,#264]

# qhasm: hiu3 = him15
# asm 1: ldr >hiu3=int32#10,<him15=stack32#68
# asm 2: ldr >hiu3=r9,<him15=[sp,#268]
# copy-collector input: ldr r9,[sp,#268]

# qhasm: lom15 = lou0
# asm 1: str <lou0=int32#3,>lom15=stack32#37
# asm 2: str <lou0=r2,>lom15=[sp,#144]
# copy-collector input: str r2,[sp,#144]

# qhasm: him15 = hiu0
# asm 1: str <hiu0=int32#4,>him15=stack32#38
# asm 2: str <hiu0=r3,>him15=[sp,#148]
# copy-collector input: str r3,[sp,#148]

# qhasm: hitmp lotmp = hiu3 * two13
# asm 1: umull >lotmp=int32#12,>hitmp=int32#11,<hiu3=int32#10,<two13=int32#2
# asm 2: umull >lotmp=r11,>hitmp=r10,<hiu3=r9,<two13=r1
# copy-collector output starts
strd r8,r9,[sp,#152]
ldrd r8,r9,[sp,#264]
strd r2,r3,[sp,#144]
# copy-collector output ends
umull r11,r10,r9,r1

# qhasm: lotmp hitmp += lou3 * two13
# asm 1: umlal <hitmp=int32#11,<lotmp=int32#12,<lou3=int32#9,<two13=int32#2
# asm 2: umlal <hitmp=r10,<lotmp=r11,<lou3=r8,<two13=r1
umlal r10,r11,r8,r1

# qhasm: lotmp ^= (lou3 unsigned>> 6)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#9,LSR #6
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r8,LSR #6
eor r11,r11,r8,LSR #6

# qhasm: lotmp ^= (hiu3 << 26)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#10,LSL #26
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r9,LSL #26
eor r11,r11,r9,LSL #26

# qhasm: lotmp ^= (hiu3 unsigned>> 29)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#10,LSR #29
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r9,LSR #29
eor r11,r11,r9,LSR #29

# qhasm: lotmp ^= (lou3 << 3)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#9,LSL #3
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r8,LSL #3
eor r11,r11,r8,LSL #3

# qhasm: hitmp ^= (hiu3 unsigned>> 6)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu3=int32#10,LSR #6
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu3=r9,LSR #6
eor r10,r10,r9,LSR #6

# qhasm: hitmp ^= (lou3 unsigned>> 29)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<lou3=int32#9,LSR #29
# asm 2: eor >hitmp=r10,<hitmp=r10,<lou3=r8,LSR #29
eor r10,r10,r8,LSR #29

# qhasm: hitmp ^= (hiu3 << 3)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu3=int32#10,LSL #3
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu3=r9,LSL #3
eor r10,r10,r9,LSL #3

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#5,<lou1=int32#5,<lotmp=int32#12
# asm 2: adds >lou1=r4,<lou1=r4,<lotmp=r11
adds r4,r4,r11

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#6,<hiu1=int32#6,<hitmp=int32#11
# asm 2: adc >hiu1=r5,<hiu1=r5,<hitmp=r10
adc r5,r5,r10

# qhasm: lou4 = lod10
# asm 1: ldr >lou4=int32#11,<lod10=stack32#41
# asm 2: ldr >lou4=r10,<lod10=[sp,#160]
# copy-collector input: ldr r10,[sp,#160]

# qhasm: hiu4 = hid10
# asm 1: ldr >hiu4=int32#12,<hid10=stack32#42
# asm 2: ldr >hiu4=r11,<hid10=[sp,#164]
# copy-collector input: ldr r11,[sp,#164]

# qhasm: hitmp lotmp = hiu4 * two24
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu4=int32#12,<two24=int32#1
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu4=r11,<two24=r0
# copy-collector output starts
ldrd r10,r11,[sp,#160]
# copy-collector output ends
umull r14,r12,r11,r0

# qhasm: lotmp hitmp += lou4 * two24
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou4=int32#11,<two24=int32#1
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou4=r10,<two24=r0
umlal r12,r14,r10,r0

# qhasm: carry? lotmp ^= (lou4 unsigned>> 1)
# asm 1: eors >lotmp=int32#14,<lotmp=int32#14,<lou4=int32#11,LSR #1
# asm 2: eors >lotmp=r14,<lotmp=r14,<lou4=r10,LSR #1
eors r14,r14,r10,LSR #1

# qhasm: hitmp ^= (carry,hiu4 unsigned>> 1)
# asm 1: eors >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#12,RRX
# asm 2: eors >hitmp=r12,<hitmp=r12,<hiu4=r11,RRX
eors r12,r12,r11,RRX

# qhasm: lotmp ^= (hiu4 << 31)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu4=int32#12,LSL #31
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu4=r11,LSL #31
eor r14,r14,r11,LSL #31

# qhasm: lotmp ^= (lou4 unsigned>>7)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou4=int32#11,LSR #7
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou4=r10,LSR #7
eor r14,r14,r10,LSR #7

# qhasm: lotmp ^= (hiu4 << 25)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu4=int32#12,LSL #25
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu4=r11,LSL #25
eor r14,r14,r11,LSL #25

# qhasm: hitmp ^= (hiu4 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#12,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu4=r11,LSR #7
eor r12,r12,r11,LSR #7

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#5,<lou1=int32#5,<lotmp=int32#14
# asm 2: adds >lou1=r4,<lou1=r4,<lotmp=r14
adds r4,r4,r14

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#6,<hiu1=int32#6,<hitmp=int32#13
# asm 2: adc >hiu1=r5,<hiu1=r5,<hitmp=r12
adc r5,r5,r12

# qhasm: lou5 = lom10
# asm 1: ldr >lou5=int32#13,<lom10=stack32#57
# asm 2: ldr >lou5=r12,<lom10=[sp,#224]
# copy-collector input: ldr r12,[sp,#224]

# qhasm: hiu5 = him10
# asm 1: ldr >hiu5=int32#14,<him10=stack32#58
# asm 2: ldr >hiu5=r14,<him10=[sp,#228]
# copy-collector input: ldr r14,[sp,#228]

# qhasm: carry? lou1 += lou5
# asm 1: adds >lou1=int32#5,<lou1=int32#5,<lou5=int32#13
# asm 2: adds >lou1=r4,<lou1=r4,<lou5=r12
# copy-collector output starts
ldrd r12,r14,[sp,#224]
# copy-collector output ends
adds r4,r4,r12

# qhasm: hiu1 += hiu5 + carry
# asm 1: adc >hiu1=int32#6,<hiu1=int32#6,<hiu5=int32#14
# asm 2: adc >hiu1=r5,<hiu1=r5,<hiu5=r14
adc r5,r5,r14

# qhasm: lod10 = lou5
# asm 1: str <lou5=int32#13,>lod10=stack32#41
# asm 2: str <lou5=r12,>lod10=[sp,#160]
# copy-collector input: str r12,[sp,#160]

# qhasm: hid10 = hiu5
# asm 1: str <hiu5=int32#14,>hid10=stack32#42
# asm 2: str <hiu5=r14,>hid10=[sp,#164]
# copy-collector input: str r14,[sp,#164]

# qhasm: lom9 = lou1
# asm 1: str <lou1=int32#5,>lom9=stack32#55
# asm 2: str <lou1=r4,>lom9=[sp,#216]
# copy-collector input: str r4,[sp,#216]

# qhasm: him9 = hiu1
# asm 1: str <hiu1=int32#6,>him9=stack32#56
# asm 2: str <hiu1=r5,>him9=[sp,#220]
# copy-collector input: str r5,[sp,#220]

# qhasm: hitmp lotmp = hiu0 * two13
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu0=int32#4,<two13=int32#2
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu0=r3,<two13=r1
# copy-collector output starts
strd r12,r14,[sp,#160]
strd r4,r5,[sp,#216]
# copy-collector output ends
umull r14,r12,r3,r1

# qhasm: lotmp hitmp += lou0 * two13
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou0=int32#3,<two13=int32#2
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou0=r2,<two13=r1
umlal r12,r14,r2,r1

# qhasm: lotmp ^= (lou0 unsigned>> 6)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#3,LSR #6
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou0=r2,LSR #6
eor r14,r14,r2,LSR #6

# qhasm: lotmp ^= (hiu0 << 26)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#4,LSL #26
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r3,LSL #26
eor r14,r14,r3,LSL #26

# qhasm: lotmp ^= (hiu0 unsigned>> 29)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#4,LSR #29
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r3,LSR #29
eor r14,r14,r3,LSR #29

# qhasm: lotmp ^= (lou0 << 3)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#3,LSL #3
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou0=r2,LSL #3
eor r14,r14,r2,LSL #3

# qhasm: hitmp ^= (hiu0 unsigned>> 6)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#4,LSR #6
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r3,LSR #6
eor r12,r12,r3,LSR #6

# qhasm: hitmp ^= (lou0 unsigned>> 29)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#13,<lou0=int32#3,LSR #29
# asm 2: eor >hitmp=r2,<hitmp=r12,<lou0=r2,LSR #29
eor r2,r12,r2,LSR #29

# qhasm: hitmp ^= (hiu0 << 3)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hiu0=int32#4,LSL #3
# asm 2: eor >hitmp=r2,<hitmp=r2,<hiu0=r3,LSL #3
eor r2,r2,r3,LSL #3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#4,<lou4=int32#11,<lotmp=int32#14
# asm 2: adds >lou4=r3,<lou4=r10,<lotmp=r14
adds r3,r10,r14

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#12,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r11,<hitmp=r2
adc r2,r11,r2

# qhasm: lou0 = lod11
# asm 1: ldr >lou0=int32#11,<lod11=stack32#43
# asm 2: ldr >lou0=r10,<lod11=[sp,#168]
# copy-collector input: ldr r10,[sp,#168]

# qhasm: hiu0 = hid11
# asm 1: ldr >hiu0=int32#12,<hid11=stack32#44
# asm 2: ldr >hiu0=r11,<hid11=[sp,#172]
# copy-collector input: ldr r11,[sp,#172]

# qhasm: hitmp lotmp = hiu0 * two24
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu0=int32#12,<two24=int32#1
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu0=r11,<two24=r0
# copy-collector output starts
ldrd r10,r11,[sp,#168]
# copy-collector output ends
umull r14,r12,r11,r0

# qhasm: lotmp hitmp += lou0 * two24
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou0=int32#11,<two24=int32#1
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou0=r10,<two24=r0
umlal r12,r14,r10,r0

# qhasm: carry? lotmp ^= (lou0 unsigned>> 1)
# asm 1: eors >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#11,LSR #1
# asm 2: eors >lotmp=r14,<lotmp=r14,<lou0=r10,LSR #1
eors r14,r14,r10,LSR #1

# qhasm: hitmp ^= (carry,hiu0 unsigned>> 1)
# asm 1: eors >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#12,RRX
# asm 2: eors >hitmp=r12,<hitmp=r12,<hiu0=r11,RRX
eors r12,r12,r11,RRX

# qhasm: lotmp ^= (hiu0 << 31)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#12,LSL #31
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r11,LSL #31
eor r14,r14,r11,LSL #31

# qhasm: lotmp ^= (lou0 unsigned>>7)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#11,LSR #7
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou0=r10,LSR #7
eor r14,r14,r10,LSR #7

# qhasm: lotmp ^= (hiu0 << 25)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#12,LSL #25
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r11,LSL #25
eor r14,r14,r11,LSL #25

# qhasm: hitmp ^= (hiu0 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#12,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r11,LSR #7
eor r12,r12,r11,LSR #7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#4,<lou4=int32#4,<lotmp=int32#14
# asm 2: adds >lou4=r3,<lou4=r3,<lotmp=r14
adds r3,r3,r14

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lou5 = lom11
# asm 1: ldr >lou5=int32#13,<lom11=stack32#59
# asm 2: ldr >lou5=r12,<lom11=[sp,#232]
# copy-collector input: ldr r12,[sp,#232]

# qhasm: hiu5 = him11
# asm 1: ldr >hiu5=int32#14,<him11=stack32#60
# asm 2: ldr >hiu5=r14,<him11=[sp,#236]
# copy-collector input: ldr r14,[sp,#236]

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#4,<lou4=int32#4,<lou5=int32#13
# asm 2: adds >lou4=r3,<lou4=r3,<lou5=r12
# copy-collector output starts
ldrd r12,r14,[sp,#232]
# copy-collector output ends
adds r3,r3,r12

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hiu5=int32#14
# asm 2: adc >hiu4=r2,<hiu4=r2,<hiu5=r14
adc r2,r2,r14

# qhasm: lod11 = lou5
# asm 1: str <lou5=int32#13,>lod11=stack32#43
# asm 2: str <lou5=r12,>lod11=[sp,#168]
# copy-collector input: str r12,[sp,#168]

# qhasm: hid11 = hiu5
# asm 1: str <hiu5=int32#14,>hid11=stack32#44
# asm 2: str <hiu5=r14,>hid11=[sp,#172]
# copy-collector input: str r14,[sp,#172]

# qhasm: lom10 = lou4
# asm 1: str <lou4=int32#4,>lom10=stack32#57
# asm 2: str <lou4=r3,>lom10=[sp,#224]
# copy-collector input: str r3,[sp,#224]

# qhasm: him10 = hiu4
# asm 1: str <hiu4=int32#3,>him10=stack32#58
# asm 2: str <hiu4=r2,>him10=[sp,#228]
# copy-collector input: str r2,[sp,#228]

# qhasm: hitmp lotmp = hiu1 * two13
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu1=int32#6,<two13=int32#2
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu1=r5,<two13=r1
# copy-collector output starts
strd r12,r14,[sp,#168]
strd r3,r2,[sp,#224]
# copy-collector output ends
umull r14,r12,r5,r1

# qhasm: lotmp hitmp += lou1 * two13
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou1=int32#5,<two13=int32#2
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou1=r4,<two13=r1
umlal r12,r14,r4,r1

# qhasm: lotmp ^= (lou1 unsigned>> 6)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou1=int32#5,LSR #6
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou1=r4,LSR #6
eor r14,r14,r4,LSR #6

# qhasm: lotmp ^= (hiu1 << 26)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu1=int32#6,LSL #26
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu1=r5,LSL #26
eor r14,r14,r5,LSL #26

# qhasm: lotmp ^= (hiu1 unsigned>> 29)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu1=int32#6,LSR #29
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu1=r5,LSR #29
eor r14,r14,r5,LSR #29

# qhasm: lotmp ^= (lou1 << 3)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou1=int32#5,LSL #3
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou1=r4,LSL #3
eor r14,r14,r4,LSL #3

# qhasm: hitmp ^= (hiu1 unsigned>> 6)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu1=int32#6,LSR #6
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu1=r5,LSR #6
eor r12,r12,r5,LSR #6

# qhasm: hitmp ^= (lou1 unsigned>> 29)
# asm 1: eor >hitmp=int32#5,<hitmp=int32#13,<lou1=int32#5,LSR #29
# asm 2: eor >hitmp=r4,<hitmp=r12,<lou1=r4,LSR #29
eor r4,r12,r4,LSR #29

# qhasm: hitmp ^= (hiu1 << 3)
# asm 1: eor >hitmp=int32#5,<hitmp=int32#5,<hiu1=int32#6,LSL #3
# asm 2: eor >hitmp=r4,<hitmp=r4,<hiu1=r5,LSL #3
eor r4,r4,r5,LSL #3

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#6,<lou0=int32#11,<lotmp=int32#14
# asm 2: adds >lou0=r5,<lou0=r10,<lotmp=r14
adds r5,r10,r14

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#5,<hiu0=int32#12,<hitmp=int32#5
# asm 2: adc >hiu0=r4,<hiu0=r11,<hitmp=r4
adc r4,r11,r4

# qhasm: lou1 = lod12
# asm 1: ldr >lou1=int32#11,<lod12=stack32#45
# asm 2: ldr >lou1=r10,<lod12=[sp,#176]
# copy-collector input: ldr r10,[sp,#176]

# qhasm: hiu1 = hid12
# asm 1: ldr >hiu1=int32#12,<hid12=stack32#46
# asm 2: ldr >hiu1=r11,<hid12=[sp,#180]
# copy-collector input: ldr r11,[sp,#180]

# qhasm: hitmp lotmp = hiu1 * two24
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu1=int32#12,<two24=int32#1
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu1=r11,<two24=r0
# copy-collector output starts
ldrd r10,r11,[sp,#176]
# copy-collector output ends
umull r14,r12,r11,r0

# qhasm: lotmp hitmp += lou1 * two24
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou1=int32#11,<two24=int32#1
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou1=r10,<two24=r0
umlal r12,r14,r10,r0

# qhasm: carry? lotmp ^= (lou1 unsigned>> 1)
# asm 1: eors >lotmp=int32#14,<lotmp=int32#14,<lou1=int32#11,LSR #1
# asm 2: eors >lotmp=r14,<lotmp=r14,<lou1=r10,LSR #1
eors r14,r14,r10,LSR #1

# qhasm: hitmp ^= (carry,hiu1 unsigned>> 1)
# asm 1: eors >hitmp=int32#13,<hitmp=int32#13,<hiu1=int32#12,RRX
# asm 2: eors >hitmp=r12,<hitmp=r12,<hiu1=r11,RRX
eors r12,r12,r11,RRX

# qhasm: lotmp ^= (hiu1 << 31)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu1=int32#12,LSL #31
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu1=r11,LSL #31
eor r14,r14,r11,LSL #31

# qhasm: lotmp ^= (lou1 unsigned>>7)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou1=int32#11,LSR #7
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou1=r10,LSR #7
eor r14,r14,r10,LSR #7

# qhasm: lotmp ^= (hiu1 << 25)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu1=int32#12,LSL #25
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu1=r11,LSL #25
eor r14,r14,r11,LSL #25

# qhasm: hitmp ^= (hiu1 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu1=int32#12,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu1=r11,LSR #7
eor r12,r12,r11,LSR #7

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#6,<lou0=int32#6,<lotmp=int32#14
# asm 2: adds >lou0=r5,<lou0=r5,<lotmp=r14
adds r5,r5,r14

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#5,<hiu0=int32#5,<hitmp=int32#13
# asm 2: adc >hiu0=r4,<hiu0=r4,<hitmp=r12
adc r4,r4,r12

# qhasm: lou5 = lom12
# asm 1: ldr >lou5=int32#13,<lom12=stack32#61
# asm 2: ldr >lou5=r12,<lom12=[sp,#240]
# copy-collector input: ldr r12,[sp,#240]

# qhasm: hiu5 = him12
# asm 1: ldr >hiu5=int32#14,<him12=stack32#62
# asm 2: ldr >hiu5=r14,<him12=[sp,#244]
# copy-collector input: ldr r14,[sp,#244]

# qhasm: carry? lou0 += lou5
# asm 1: adds >lou0=int32#6,<lou0=int32#6,<lou5=int32#13
# asm 2: adds >lou0=r5,<lou0=r5,<lou5=r12
# copy-collector output starts
ldrd r12,r14,[sp,#240]
# copy-collector output ends
adds r5,r5,r12

# qhasm: hiu0 += hiu5 + carry
# asm 1: adc >hiu0=int32#5,<hiu0=int32#5,<hiu5=int32#14
# asm 2: adc >hiu0=r4,<hiu0=r4,<hiu5=r14
adc r4,r4,r14

# qhasm: lod12 = lou5
# asm 1: str <lou5=int32#13,>lod12=stack32#45
# asm 2: str <lou5=r12,>lod12=[sp,#176]
# copy-collector input: str r12,[sp,#176]

# qhasm: hid12 = hiu5
# asm 1: str <hiu5=int32#14,>hid12=stack32#46
# asm 2: str <hiu5=r14,>hid12=[sp,#180]
# copy-collector input: str r14,[sp,#180]

# qhasm: lom11 = lou0
# asm 1: str <lou0=int32#6,>lom11=stack32#59
# asm 2: str <lou0=r5,>lom11=[sp,#232]
# copy-collector input: str r5,[sp,#232]

# qhasm: him11 = hiu0
# asm 1: str <hiu0=int32#5,>him11=stack32#60
# asm 2: str <hiu0=r4,>him11=[sp,#236]
# copy-collector input: str r4,[sp,#236]

# qhasm: hitmp lotmp = hiu4 * two13
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu4=int32#3,<two13=int32#2
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu4=r2,<two13=r1
# copy-collector output starts
strd r12,r14,[sp,#176]
strd r5,r4,[sp,#232]
# copy-collector output ends
umull r14,r12,r2,r1

# qhasm: lotmp hitmp += lou4 * two13
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou4=int32#4,<two13=int32#2
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou4=r3,<two13=r1
umlal r12,r14,r3,r1

# qhasm: lotmp ^= (lou4 unsigned>> 6)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou4=int32#4,LSR #6
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou4=r3,LSR #6
eor r14,r14,r3,LSR #6

# qhasm: lotmp ^= (hiu4 << 26)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu4=int32#3,LSL #26
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu4=r2,LSL #26
eor r14,r14,r2,LSL #26

# qhasm: lotmp ^= (hiu4 unsigned>> 29)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu4=int32#3,LSR #29
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu4=r2,LSR #29
eor r14,r14,r2,LSR #29

# qhasm: lotmp ^= (lou4 << 3)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou4=int32#4,LSL #3
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou4=r3,LSL #3
eor r14,r14,r3,LSL #3

# qhasm: hitmp ^= (hiu4 unsigned>> 6)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#3,LSR #6
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu4=r2,LSR #6
eor r12,r12,r2,LSR #6

# qhasm: hitmp ^= (lou4 unsigned>> 29)
# asm 1: eor >hitmp=int32#4,<hitmp=int32#13,<lou4=int32#4,LSR #29
# asm 2: eor >hitmp=r3,<hitmp=r12,<lou4=r3,LSR #29
eor r3,r12,r3,LSR #29

# qhasm: hitmp ^= (hiu4 << 3)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#4,<hiu4=int32#3,LSL #3
# asm 2: eor >hitmp=r2,<hitmp=r3,<hiu4=r2,LSL #3
eor r2,r3,r2,LSL #3

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#4,<lou1=int32#11,<lotmp=int32#14
# asm 2: adds >lou1=r3,<lou1=r10,<lotmp=r14
adds r3,r10,r14

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#3,<hiu1=int32#12,<hitmp=int32#3
# asm 2: adc >hiu1=r2,<hiu1=r11,<hitmp=r2
adc r2,r11,r2

# qhasm: lou4 = lod13
# asm 1: ldr >lou4=int32#11,<lod13=stack32#47
# asm 2: ldr >lou4=r10,<lod13=[sp,#184]
# copy-collector input: ldr r10,[sp,#184]

# qhasm: hiu4 = hid13
# asm 1: ldr >hiu4=int32#12,<hid13=stack32#48
# asm 2: ldr >hiu4=r11,<hid13=[sp,#188]
# copy-collector input: ldr r11,[sp,#188]

# qhasm: hitmp lotmp = hiu4 * two24
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu4=int32#12,<two24=int32#1
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu4=r11,<two24=r0
# copy-collector output starts
ldrd r10,r11,[sp,#184]
# copy-collector output ends
umull r14,r12,r11,r0

# qhasm: lotmp hitmp += lou4 * two24
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou4=int32#11,<two24=int32#1
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou4=r10,<two24=r0
umlal r12,r14,r10,r0

# qhasm: carry? lotmp ^= (lou4 unsigned>> 1)
# asm 1: eors >lotmp=int32#14,<lotmp=int32#14,<lou4=int32#11,LSR #1
# asm 2: eors >lotmp=r14,<lotmp=r14,<lou4=r10,LSR #1
eors r14,r14,r10,LSR #1

# qhasm: hitmp ^= (carry,hiu4 unsigned>> 1)
# asm 1: eors >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#12,RRX
# asm 2: eors >hitmp=r12,<hitmp=r12,<hiu4=r11,RRX
eors r12,r12,r11,RRX

# qhasm: lotmp ^= (hiu4 << 31)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu4=int32#12,LSL #31
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu4=r11,LSL #31
eor r14,r14,r11,LSL #31

# qhasm: lotmp ^= (lou4 unsigned>>7)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou4=int32#11,LSR #7
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou4=r10,LSR #7
eor r14,r14,r10,LSR #7

# qhasm: lotmp ^= (hiu4 << 25)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu4=int32#12,LSL #25
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu4=r11,LSL #25
eor r14,r14,r11,LSL #25

# qhasm: hitmp ^= (hiu4 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#12,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu4=r11,LSR #7
eor r12,r12,r11,LSR #7

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#4,<lou1=int32#4,<lotmp=int32#14
# asm 2: adds >lou1=r3,<lou1=r3,<lotmp=r14
adds r3,r3,r14

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#3,<hiu1=int32#3,<hitmp=int32#13
# asm 2: adc >hiu1=r2,<hiu1=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: lou5 = lom13
# asm 1: ldr >lou5=int32#13,<lom13=stack32#63
# asm 2: ldr >lou5=r12,<lom13=[sp,#248]
# copy-collector input: ldr r12,[sp,#248]

# qhasm: hiu5 = him13
# asm 1: ldr >hiu5=int32#14,<him13=stack32#64
# asm 2: ldr >hiu5=r14,<him13=[sp,#252]
# copy-collector input: ldr r14,[sp,#252]

# qhasm: carry? lou1 += lou5
# asm 1: adds >lou1=int32#4,<lou1=int32#4,<lou5=int32#13
# asm 2: adds >lou1=r3,<lou1=r3,<lou5=r12
# copy-collector output starts
ldrd r12,r14,[sp,#248]
# copy-collector output ends
adds r3,r3,r12

# qhasm: hiu1 += hiu5 + carry
# asm 1: adc >hiu1=int32#3,<hiu1=int32#3,<hiu5=int32#14
# asm 2: adc >hiu1=r2,<hiu1=r2,<hiu5=r14
adc r2,r2,r14

# qhasm: lod13 = lou5
# asm 1: str <lou5=int32#13,>lod13=stack32#47
# asm 2: str <lou5=r12,>lod13=[sp,#184]
# copy-collector input: str r12,[sp,#184]

# qhasm: hid13 = hiu5
# asm 1: str <hiu5=int32#14,>hid13=stack32#48
# asm 2: str <hiu5=r14,>hid13=[sp,#188]
# copy-collector input: str r14,[sp,#188]

# qhasm: lom12 = lou1
# asm 1: str <lou1=int32#4,>lom12=stack32#61
# asm 2: str <lou1=r3,>lom12=[sp,#240]
# copy-collector input: str r3,[sp,#240]

# qhasm: him12 = hiu1
# asm 1: str <hiu1=int32#3,>him12=stack32#62
# asm 2: str <hiu1=r2,>him12=[sp,#244]
# copy-collector input: str r2,[sp,#244]

# qhasm: hitmp lotmp = hiu0 * two13
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu0=int32#5,<two13=int32#2
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu0=r4,<two13=r1
# copy-collector output starts
strd r12,r14,[sp,#184]
strd r3,r2,[sp,#240]
# copy-collector output ends
umull r14,r12,r4,r1

# qhasm: lotmp hitmp += lou0 * two13
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou0=int32#6,<two13=int32#2
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou0=r5,<two13=r1
umlal r12,r14,r5,r1

# qhasm: lotmp ^= (lou0 unsigned>> 6)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#6,LSR #6
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou0=r5,LSR #6
eor r14,r14,r5,LSR #6

# qhasm: lotmp ^= (hiu0 << 26)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#5,LSL #26
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r4,LSL #26
eor r14,r14,r4,LSL #26

# qhasm: lotmp ^= (hiu0 unsigned>> 29)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#5,LSR #29
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r4,LSR #29
eor r14,r14,r4,LSR #29

# qhasm: lotmp ^= (lou0 << 3)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#6,LSL #3
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou0=r5,LSL #3
eor r14,r14,r5,LSL #3

# qhasm: hitmp ^= (hiu0 unsigned>> 6)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#5,LSR #6
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r4,LSR #6
eor r12,r12,r4,LSR #6

# qhasm: hitmp ^= (lou0 unsigned>> 29)
# asm 1: eor >hitmp=int32#6,<hitmp=int32#13,<lou0=int32#6,LSR #29
# asm 2: eor >hitmp=r5,<hitmp=r12,<lou0=r5,LSR #29
eor r5,r12,r5,LSR #29

# qhasm: hitmp ^= (hiu0 << 3)
# asm 1: eor >hitmp=int32#5,<hitmp=int32#6,<hiu0=int32#5,LSL #3
# asm 2: eor >hitmp=r4,<hitmp=r5,<hiu0=r4,LSL #3
eor r4,r5,r4,LSL #3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#6,<lou4=int32#11,<lotmp=int32#14
# asm 2: adds >lou4=r5,<lou4=r10,<lotmp=r14
adds r5,r10,r14

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#5,<hiu4=int32#12,<hitmp=int32#5
# asm 2: adc >hiu4=r4,<hiu4=r11,<hitmp=r4
adc r4,r11,r4

# qhasm: lou0 = lod14
# asm 1: ldr >lou0=int32#11,<lod14=stack32#49
# asm 2: ldr >lou0=r10,<lod14=[sp,#192]
# copy-collector input: ldr r10,[sp,#192]

# qhasm: hiu0 = hid14
# asm 1: ldr >hiu0=int32#12,<hid14=stack32#50
# asm 2: ldr >hiu0=r11,<hid14=[sp,#196]
# copy-collector input: ldr r11,[sp,#196]

# qhasm: lod14 = lou2
# asm 1: str <lou2=int32#7,>lod14=stack32#49
# asm 2: str <lou2=r6,>lod14=[sp,#192]
# copy-collector input: str r6,[sp,#192]

# qhasm: hid14 = hiu2
# asm 1: str <hiu2=int32#8,>hid14=stack32#50
# asm 2: str <hiu2=r7,>hid14=[sp,#196]
# copy-collector input: str r7,[sp,#196]

# qhasm: hitmp lotmp = hiu0 * two24
# asm 1: umull >lotmp=int32#14,>hitmp=int32#13,<hiu0=int32#12,<two24=int32#1
# asm 2: umull >lotmp=r14,>hitmp=r12,<hiu0=r11,<two24=r0
# copy-collector output starts
ldrd r10,r11,[sp,#192]
strd r6,r7,[sp,#192]
# copy-collector output ends
umull r14,r12,r11,r0

# qhasm: lotmp hitmp += lou0 * two24
# asm 1: umlal <hitmp=int32#13,<lotmp=int32#14,<lou0=int32#11,<two24=int32#1
# asm 2: umlal <hitmp=r12,<lotmp=r14,<lou0=r10,<two24=r0
umlal r12,r14,r10,r0

# qhasm: carry? lotmp ^= (lou0 unsigned>> 1)
# asm 1: eors >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#11,LSR #1
# asm 2: eors >lotmp=r14,<lotmp=r14,<lou0=r10,LSR #1
eors r14,r14,r10,LSR #1

# qhasm: hitmp ^= (carry,hiu0 unsigned>> 1)
# asm 1: eors >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#12,RRX
# asm 2: eors >hitmp=r12,<hitmp=r12,<hiu0=r11,RRX
eors r12,r12,r11,RRX

# qhasm: lotmp ^= (hiu0 << 31)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#12,LSL #31
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r11,LSL #31
eor r14,r14,r11,LSL #31

# qhasm: lotmp ^= (lou0 unsigned>>7)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<lou0=int32#11,LSR #7
# asm 2: eor >lotmp=r14,<lotmp=r14,<lou0=r10,LSR #7
eor r14,r14,r10,LSR #7

# qhasm: lotmp ^= (hiu0 << 25)
# asm 1: eor >lotmp=int32#14,<lotmp=int32#14,<hiu0=int32#12,LSL #25
# asm 2: eor >lotmp=r14,<lotmp=r14,<hiu0=r11,LSL #25
eor r14,r14,r11,LSL #25

# qhasm: hitmp ^= (hiu0 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#12,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r11,LSR #7
eor r12,r12,r11,LSR #7

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#6,<lou4=int32#6,<lotmp=int32#14
# asm 2: adds >lou4=r5,<lou4=r5,<lotmp=r14
adds r5,r5,r14

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#5,<hiu4=int32#5,<hitmp=int32#13
# asm 2: adc >hiu4=r4,<hiu4=r4,<hitmp=r12
adc r4,r4,r12

# qhasm: carry? lou4 += lou2
# asm 1: adds >lou4=int32#6,<lou4=int32#6,<lou2=int32#7
# asm 2: adds >lou4=r5,<lou4=r5,<lou2=r6
adds.w r5,r5,r6

# qhasm: hiu4 += hiu2 + carry
# asm 1: adc >hiu4=int32#5,<hiu4=int32#5,<hiu2=int32#8
# asm 2: adc >hiu4=r4,<hiu4=r4,<hiu2=r7
adc r4,r4,r7

# qhasm: lom13 = lou4
# asm 1: str <lou4=int32#6,>lom13=stack32#63
# asm 2: str <lou4=r5,>lom13=[sp,#248]
# copy-collector input: str r5,[sp,#248]

# qhasm: him13 = hiu4
# asm 1: str <hiu4=int32#5,>him13=stack32#64
# asm 2: str <hiu4=r4,>him13=[sp,#252]
# copy-collector input: str r4,[sp,#252]

# qhasm: hitmp lotmp = hiu1 * two13
# asm 1: umull >lotmp=int32#8,>hitmp=int32#7,<hiu1=int32#3,<two13=int32#2
# asm 2: umull >lotmp=r7,>hitmp=r6,<hiu1=r2,<two13=r1
# copy-collector output starts
strd r5,r4,[sp,#248]
# copy-collector output ends
umull r7,r6,r2,r1

# qhasm: lotmp hitmp += lou1 * two13
# asm 1: umlal <hitmp=int32#7,<lotmp=int32#8,<lou1=int32#4,<two13=int32#2
# asm 2: umlal <hitmp=r6,<lotmp=r7,<lou1=r3,<two13=r1
umlal r6,r7,r3,r1

# qhasm: lotmp ^= (lou1 unsigned>> 6)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<lou1=int32#4,LSR #6
# asm 2: eor >lotmp=r7,<lotmp=r7,<lou1=r3,LSR #6
eor r7,r7,r3,LSR #6

# qhasm: lotmp ^= (hiu1 << 26)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<hiu1=int32#3,LSL #26
# asm 2: eor >lotmp=r7,<lotmp=r7,<hiu1=r2,LSL #26
eor r7,r7,r2,LSL #26

# qhasm: lotmp ^= (hiu1 unsigned>> 29)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<hiu1=int32#3,LSR #29
# asm 2: eor >lotmp=r7,<lotmp=r7,<hiu1=r2,LSR #29
eor r7,r7,r2,LSR #29

# qhasm: lotmp ^= (lou1 << 3)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<lou1=int32#4,LSL #3
# asm 2: eor >lotmp=r7,<lotmp=r7,<lou1=r3,LSL #3
eor r7,r7,r3,LSL #3

# qhasm: hitmp ^= (hiu1 unsigned>> 6)
# asm 1: eor >hitmp=int32#7,<hitmp=int32#7,<hiu1=int32#3,LSR #6
# asm 2: eor >hitmp=r6,<hitmp=r6,<hiu1=r2,LSR #6
eor r6,r6,r2,LSR #6

# qhasm: hitmp ^= (lou1 unsigned>> 29)
# asm 1: eor >hitmp=int32#4,<hitmp=int32#7,<lou1=int32#4,LSR #29
# asm 2: eor >hitmp=r3,<hitmp=r6,<lou1=r3,LSR #29
eor r3,r6,r3,LSR #29

# qhasm: hitmp ^= (hiu1 << 3)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#4,<hiu1=int32#3,LSL #3
# asm 2: eor >hitmp=r2,<hitmp=r3,<hiu1=r2,LSL #3
eor r2,r3,r2,LSL #3

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#4,<lou0=int32#11,<lotmp=int32#8
# asm 2: adds >lou0=r3,<lou0=r10,<lotmp=r7
adds r3,r10,r7

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#12,<hitmp=int32#3
# asm 2: adc >hiu0=r2,<hiu0=r11,<hitmp=r2
adc r2,r11,r2

# qhasm: lou1 = lod15
# asm 1: ldr >lou1=int32#7,<lod15=stack32#51
# asm 2: ldr >lou1=r6,<lod15=[sp,#200]
# copy-collector input: ldr r6,[sp,#200]

# qhasm: hiu1 = hid15
# asm 1: ldr >hiu1=int32#8,<hid15=stack32#52
# asm 2: ldr >hiu1=r7,<hid15=[sp,#204]
# copy-collector input: ldr r7,[sp,#204]

# qhasm: lod15 = lou3
# asm 1: str <lou3=int32#9,>lod15=stack32#51
# asm 2: str <lou3=r8,>lod15=[sp,#200]
# copy-collector input: str r8,[sp,#200]

# qhasm: hid15 = hiu3
# asm 1: str <hiu3=int32#10,>hid15=stack32#52
# asm 2: str <hiu3=r9,>hid15=[sp,#204]
# copy-collector input: str r9,[sp,#204]

# qhasm: hitmp lotmp = hiu1 * two24
# asm 1: umull >lotmp=int32#12,>hitmp=int32#11,<hiu1=int32#8,<two24=int32#1
# asm 2: umull >lotmp=r11,>hitmp=r10,<hiu1=r7,<two24=r0
# copy-collector output starts
ldrd r6,r7,[sp,#200]
strd r8,r9,[sp,#200]
# copy-collector output ends
umull r11,r10,r7,r0

# qhasm: lotmp hitmp += lou1 * two24
# asm 1: umlal <hitmp=int32#11,<lotmp=int32#12,<lou1=int32#7,<two24=int32#1
# asm 2: umlal <hitmp=r10,<lotmp=r11,<lou1=r6,<two24=r0
umlal r10,r11,r6,r0

# qhasm: carry? lotmp ^= (lou1 unsigned>> 1)
# asm 1: eors >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#7,LSR #1
# asm 2: eors >lotmp=r11,<lotmp=r11,<lou1=r6,LSR #1
eors r11,r11,r6,LSR #1

# qhasm: hitmp ^= (carry,hiu1 unsigned>> 1)
# asm 1: eors >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#8,RRX
# asm 2: eors >hitmp=r10,<hitmp=r10,<hiu1=r7,RRX
eors r10,r10,r7,RRX

# qhasm: lotmp ^= (hiu1 << 31)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#8,LSL #31
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r7,LSL #31
eor r11,r11,r7,LSL #31

# qhasm: lotmp ^= (lou1 unsigned>>7)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#7,LSR #7
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r6,LSR #7
eor r11,r11,r6,LSR #7

# qhasm: lotmp ^= (hiu1 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#8,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r7,LSL #25
eor r11,r11,r7,LSL #25

# qhasm: hitmp ^= (hiu1 unsigned>>7)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#8,LSR #7
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu1=r7,LSR #7
eor r10,r10,r7,LSR #7

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#4,<lou0=int32#4,<lotmp=int32#12
# asm 2: adds >lou0=r3,<lou0=r3,<lotmp=r11
adds r3,r3,r11

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hitmp=int32#11
# asm 2: adc >hiu0=r2,<hiu0=r2,<hitmp=r10
adc r2,r2,r10

# qhasm: carry? lou0 += lou3
# asm 1: adds >lou0=int32#4,<lou0=int32#4,<lou3=int32#9
# asm 2: adds >lou0=r3,<lou0=r3,<lou3=r8
adds r3,r3,r8

# qhasm: hiu0 += hiu3 + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hiu3=int32#10
# asm 2: adc >hiu0=r2,<hiu0=r2,<hiu3=r9
adc r2,r2,r9

# qhasm: lom14 = lou0
# asm 1: str <lou0=int32#4,>lom14=stack32#65
# asm 2: str <lou0=r3,>lom14=[sp,#256]
# copy-collector input: str r3,[sp,#256]

# qhasm: him14 = hiu0
# asm 1: str <hiu0=int32#3,>him14=stack32#66
# asm 2: str <hiu0=r2,>him14=[sp,#260]
# copy-collector input: str r2,[sp,#260]

# qhasm: hitmp lotmp = hiu4 * two13
# asm 1: umull >lotmp=int32#4,>hitmp=int32#3,<hiu4=int32#5,<two13=int32#2
# asm 2: umull >lotmp=r3,>hitmp=r2,<hiu4=r4,<two13=r1
# copy-collector output starts
strd r3,r2,[sp,#256]
# copy-collector output ends
umull r3,r2,r4,r1

# qhasm: lotmp hitmp += lou4 * two13
# asm 1: umlal <hitmp=int32#3,<lotmp=int32#4,<lou4=int32#6,<two13=int32#2
# asm 2: umlal <hitmp=r2,<lotmp=r3,<lou4=r5,<two13=r1
umlal r2,r3,r5,r1

# qhasm: lotmp ^= (lou4 unsigned>> 6)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#4,<lou4=int32#6,LSR #6
# asm 2: eor >lotmp=r1,<lotmp=r3,<lou4=r5,LSR #6
eor r1,r3,r5,LSR #6

# qhasm: lotmp ^= (hiu4 << 26)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<hiu4=int32#5,LSL #26
# asm 2: eor >lotmp=r1,<lotmp=r1,<hiu4=r4,LSL #26
eor r1,r1,r4,LSL #26

# qhasm: lotmp ^= (hiu4 unsigned>> 29)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<hiu4=int32#5,LSR #29
# asm 2: eor >lotmp=r1,<lotmp=r1,<hiu4=r4,LSR #29
eor r1,r1,r4,LSR #29

# qhasm: lotmp ^= (lou4 << 3)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lou4=int32#6,LSL #3
# asm 2: eor >lotmp=r1,<lotmp=r1,<lou4=r5,LSL #3
eor r1,r1,r5,LSL #3

# qhasm: hitmp ^= (hiu4 unsigned>> 6)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hiu4=int32#5,LSR #6
# asm 2: eor >hitmp=r2,<hitmp=r2,<hiu4=r4,LSR #6
eor r2,r2,r4,LSR #6

# qhasm: hitmp ^= (lou4 unsigned>> 29)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<lou4=int32#6,LSR #29
# asm 2: eor >hitmp=r2,<hitmp=r2,<lou4=r5,LSR #29
eor r2,r2,r5,LSR #29

# qhasm: hitmp ^= (hiu4 << 3)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hiu4=int32#5,LSL #3
# asm 2: eor >hitmp=r2,<hitmp=r2,<hiu4=r4,LSL #3
eor r2,r2,r4,LSL #3

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#2,<lou1=int32#7,<lotmp=int32#2
# asm 2: adds >lou1=r1,<lou1=r6,<lotmp=r1
adds r1,r6,r1

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#3,<hiu1=int32#8,<hitmp=int32#3
# asm 2: adc >hiu1=r2,<hiu1=r7,<hitmp=r2
adc r2,r7,r2

# qhasm: lou0 = lom8
# asm 1: ldr >lou0=int32#4,<lom8=stack32#53
# asm 2: ldr >lou0=r3,<lom8=[sp,#208]
# copy-collector input: ldr r3,[sp,#208]

# qhasm: hiu0 = him8
# asm 1: ldr >hiu0=int32#5,<him8=stack32#54
# asm 2: ldr >hiu0=r4,<him8=[sp,#212]
# copy-collector input: ldr r4,[sp,#212]

# qhasm: lou2 = lom15
# asm 1: ldr >lou2=int32#6,<lom15=stack32#37
# asm 2: ldr >lou2=r5,<lom15=[sp,#144]
# copy-collector input: ldr r5,[sp,#144]

# qhasm: hiu2 = him15
# asm 1: ldr >hiu2=int32#7,<him15=stack32#38
# asm 2: ldr >hiu2=r6,<him15=[sp,#148]
# copy-collector input: ldr r6,[sp,#148]

# qhasm: lom8 = lou2
# asm 1: str <lou2=int32#6,>lom8=stack32#53
# asm 2: str <lou2=r5,>lom8=[sp,#208]
# copy-collector input: str r5,[sp,#208]

# qhasm: him8 = hiu2
# asm 1: str <hiu2=int32#7,>him8=stack32#54
# asm 2: str <hiu2=r6,>him8=[sp,#212]
# copy-collector input: str r6,[sp,#212]

# qhasm: hitmp lotmp = hiu0 * two24
# asm 1: umull >lotmp=int32#9,>hitmp=int32#8,<hiu0=int32#5,<two24=int32#1
# asm 2: umull >lotmp=r8,>hitmp=r7,<hiu0=r4,<two24=r0
# copy-collector output starts
ldr r3,[sp,#208]
ldr r4,[sp,#212]
ldr r5,[sp,#144]
ldr.w r6,[sp,#148]
strd r5,r6,[sp,#208]
# copy-collector output ends
umull r8,r7,r4,r0

# qhasm: lotmp hitmp += lou0 * two24
# asm 1: umlal <hitmp=int32#8,<lotmp=int32#9,<lou0=int32#4,<two24=int32#1
# asm 2: umlal <hitmp=r7,<lotmp=r8,<lou0=r3,<two24=r0
umlal r7,r8,r3,r0

# qhasm: carry? lotmp ^= (lou0 unsigned>> 1)
# asm 1: eors >lotmp=int32#1,<lotmp=int32#9,<lou0=int32#4,LSR #1
# asm 2: eors >lotmp=r0,<lotmp=r8,<lou0=r3,LSR #1
eors r0,r8,r3,LSR #1

# qhasm: hitmp ^= (carry,hiu0 unsigned>> 1)
# asm 1: eors >hitmp=int32#8,<hitmp=int32#8,<hiu0=int32#5,RRX
# asm 2: eors >hitmp=r7,<hitmp=r7,<hiu0=r4,RRX
eors r7,r7,r4,RRX

# qhasm: lotmp ^= (hiu0 << 31)
# asm 1: eor >lotmp=int32#1,<lotmp=int32#1,<hiu0=int32#5,LSL #31
# asm 2: eor >lotmp=r0,<lotmp=r0,<hiu0=r4,LSL #31
eor r0,r0,r4,LSL #31

# qhasm: lotmp ^= (lou0 unsigned>>7)
# asm 1: eor >lotmp=int32#1,<lotmp=int32#1,<lou0=int32#4,LSR #7
# asm 2: eor >lotmp=r0,<lotmp=r0,<lou0=r3,LSR #7
eor r0,r0,r3,LSR #7

# qhasm: lotmp ^= (hiu0 << 25)
# asm 1: eor >lotmp=int32#1,<lotmp=int32#1,<hiu0=int32#5,LSL #25
# asm 2: eor >lotmp=r0,<lotmp=r0,<hiu0=r4,LSL #25
eor r0,r0,r4,LSL #25

# qhasm: hitmp ^= (hiu0 unsigned>>7)
# asm 1: eor >hitmp=int32#8,<hitmp=int32#8,<hiu0=int32#5,LSR #7
# asm 2: eor >hitmp=r7,<hitmp=r7,<hiu0=r4,LSR #7
eor r7,r7,r4,LSR #7

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#1,<lou1=int32#2,<lotmp=int32#1
# asm 2: adds >lou1=r0,<lou1=r1,<lotmp=r0
adds r0,r1,r0

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#2,<hiu1=int32#3,<hitmp=int32#8
# asm 2: adc >hiu1=r1,<hiu1=r2,<hitmp=r7
adc r1,r2,r7

# qhasm: carry? lou1 += lou2
# asm 1: adds >lou1=int32#1,<lou1=int32#1,<lou2=int32#6
# asm 2: adds >lou1=r0,<lou1=r0,<lou2=r5
adds r0,r0,r5

# qhasm: hiu1 += hiu2 + carry
# asm 1: adc >hiu1=int32#2,<hiu1=int32#2,<hiu2=int32#7
# asm 2: adc >hiu1=r1,<hiu1=r1,<hiu2=r6
adc r1,r1,r6

# qhasm: lom15 = lou1
# asm 1: str <lou1=int32#1,>lom15=stack32#67
# asm 2: str <lou1=r0,>lom15=[sp,#264]
# copy-collector input: str r0,[sp,#264]

# qhasm: him15 = hiu1
# asm 1: str <hiu1=int32#2,>him15=stack32#68
# asm 2: str <hiu1=r1,>him15=[sp,#268]
# copy-collector input: str r1,[sp,#268]

# qhasm: lod8 = lou0
# asm 1: str <lou0=int32#4,>lod8=stack32#37
# asm 2: str <lou0=r3,>lod8=[sp,#144]
# copy-collector input: str r3,[sp,#144]

# qhasm: hid8 = hiu0
# asm 1: str <hiu0=int32#5,>hid8=stack32#38
# asm 2: str <hiu0=r4,>hid8=[sp,#148]
# copy-collector input: str r4,[sp,#148]

# qhasm: goto innerloop
# copy-collector output starts
strd r0,r1,[sp,#264]
strd r3,r4,[sp,#144]
# copy-collector output ends
b ._innerloop

# qhasm: nearend:
._nearend:

# qhasm: lou0 = lom8
# asm 1: ldr >lou0=int32#1,<lom8=stack32#53
# asm 2: ldr >lou0=r0,<lom8=[sp,#208]
# copy-collector input: ldr r0,[sp,#208]

# qhasm: hiu0 = him8
# asm 1: ldr >hiu0=int32#2,<him8=stack32#54
# asm 2: ldr >hiu0=r1,<him8=[sp,#212]
# copy-collector input: ldr r1,[sp,#212]

# qhasm: lou1 = lom9
# asm 1: ldr >lou1=int32#3,<lom9=stack32#55
# asm 2: ldr >lou1=r2,<lom9=[sp,#216]
# copy-collector input: ldr r2,[sp,#216]

# qhasm: hiu1 = him9
# asm 1: ldr >hiu1=int32#4,<him9=stack32#56
# asm 2: ldr >hiu1=r3,<him9=[sp,#220]
# copy-collector input: ldr r3,[sp,#220]

# qhasm: lou2 = lom10
# asm 1: ldr >lou2=int32#5,<lom10=stack32#57
# asm 2: ldr >lou2=r4,<lom10=[sp,#224]
# copy-collector input: ldr r4,[sp,#224]

# qhasm: hiu2 = him10
# asm 1: ldr >hiu2=int32#6,<him10=stack32#58
# asm 2: ldr >hiu2=r5,<him10=[sp,#228]
# copy-collector input: ldr r5,[sp,#228]

# qhasm: lou3 = lom11
# asm 1: ldr >lou3=int32#7,<lom11=stack32#59
# asm 2: ldr >lou3=r6,<lom11=[sp,#232]
# copy-collector input: ldr r6,[sp,#232]

# qhasm: hiu3 = him11
# asm 1: ldr >hiu3=int32#8,<him11=stack32#60
# asm 2: ldr >hiu3=r7,<him11=[sp,#236]
# copy-collector input: ldr r7,[sp,#236]

# qhasm: lod8 = lou0
# asm 1: str <lou0=int32#1,>lod8=stack32#37
# asm 2: str <lou0=r0,>lod8=[sp,#144]
# copy-collector input: str r0,[sp,#144]

# qhasm: hid8 = hiu0
# asm 1: str <hiu0=int32#2,>hid8=stack32#38
# asm 2: str <hiu0=r1,>hid8=[sp,#148]
# copy-collector input: str r1,[sp,#148]

# qhasm: lod9 = lou1
# asm 1: str <lou1=int32#3,>lod9=stack32#39
# asm 2: str <lou1=r2,>lod9=[sp,#152]
# copy-collector input: str r2,[sp,#152]

# qhasm: hid9 = hiu1
# asm 1: str <hiu1=int32#4,>hid9=stack32#40
# asm 2: str <hiu1=r3,>hid9=[sp,#156]
# copy-collector input: str r3,[sp,#156]

# qhasm: lod10 = lou2
# asm 1: str <lou2=int32#5,>lod10=stack32#41
# asm 2: str <lou2=r4,>lod10=[sp,#160]
# copy-collector input: str r4,[sp,#160]

# qhasm: hid10 = hiu2
# asm 1: str <hiu2=int32#6,>hid10=stack32#42
# asm 2: str <hiu2=r5,>hid10=[sp,#164]
# copy-collector input: str r5,[sp,#164]

# qhasm: lod11 = lou3
# asm 1: str <lou3=int32#7,>lod11=stack32#43
# asm 2: str <lou3=r6,>lod11=[sp,#168]
# copy-collector input: str r6,[sp,#168]

# qhasm: hid11 = hiu3
# asm 1: str <hiu3=int32#8,>hid11=stack32#44
# asm 2: str <hiu3=r7,>hid11=[sp,#172]
# copy-collector input: str r7,[sp,#172]

# qhasm: lou0 = lom12
# asm 1: ldr >lou0=int32#1,<lom12=stack32#61
# asm 2: ldr >lou0=r0,<lom12=[sp,#240]
# copy-collector input: ldr r0,[sp,#240]

# qhasm: hiu0 = him12
# asm 1: ldr >hiu0=int32#2,<him12=stack32#62
# asm 2: ldr >hiu0=r1,<him12=[sp,#244]
# copy-collector input: ldr r1,[sp,#244]

# qhasm: lou1 = lom13
# asm 1: ldr >lou1=int32#3,<lom13=stack32#63
# asm 2: ldr >lou1=r2,<lom13=[sp,#248]
# copy-collector input: ldr r2,[sp,#248]

# qhasm: hiu1 = him13
# asm 1: ldr >hiu1=int32#4,<him13=stack32#64
# asm 2: ldr >hiu1=r3,<him13=[sp,#252]
# copy-collector input: ldr r3,[sp,#252]

# qhasm: lou2 = lom14
# asm 1: ldr >lou2=int32#5,<lom14=stack32#65
# asm 2: ldr >lou2=r4,<lom14=[sp,#256]
# copy-collector input: ldr r4,[sp,#256]

# qhasm: hiu2 = him14
# asm 1: ldr >hiu2=int32#6,<him14=stack32#66
# asm 2: ldr >hiu2=r5,<him14=[sp,#260]
# copy-collector input: ldr r5,[sp,#260]

# qhasm: lou3 = lom15
# asm 1: ldr >lou3=int32#7,<lom15=stack32#67
# asm 2: ldr >lou3=r6,<lom15=[sp,#264]
# copy-collector input: ldr r6,[sp,#264]

# qhasm: hiu3 = him15
# asm 1: ldr >hiu3=int32#8,<him15=stack32#68
# asm 2: ldr >hiu3=r7,<him15=[sp,#268]
# copy-collector input: ldr r7,[sp,#268]

# qhasm: lod12 = lou0
# asm 1: str <lou0=int32#1,>lod12=stack32#45
# asm 2: str <lou0=r0,>lod12=[sp,#176]
# copy-collector input: str r0,[sp,#176]

# qhasm: hid12 = hiu0
# asm 1: str <hiu0=int32#2,>hid12=stack32#46
# asm 2: str <hiu0=r1,>hid12=[sp,#180]
# copy-collector input: str r1,[sp,#180]

# qhasm: lod13 = lou1
# asm 1: str <lou1=int32#3,>lod13=stack32#47
# asm 2: str <lou1=r2,>lod13=[sp,#184]
# copy-collector input: str r2,[sp,#184]

# qhasm: hid13 = hiu1
# asm 1: str <hiu1=int32#4,>hid13=stack32#48
# asm 2: str <hiu1=r3,>hid13=[sp,#188]
# copy-collector input: str r3,[sp,#188]

# qhasm: lod14 = lou2
# asm 1: str <lou2=int32#5,>lod14=stack32#49
# asm 2: str <lou2=r4,>lod14=[sp,#192]
# copy-collector input: str r4,[sp,#192]

# qhasm: hid14 = hiu2
# asm 1: str <hiu2=int32#6,>hid14=stack32#50
# asm 2: str <hiu2=r5,>hid14=[sp,#196]
# copy-collector input: str r5,[sp,#196]

# qhasm: lod15 = lou3
# asm 1: str <lou3=int32#7,>lod15=stack32#51
# asm 2: str <lou3=r6,>lod15=[sp,#200]
# copy-collector input: str r6,[sp,#200]

# qhasm: hid15 = hiu3
# asm 1: str <hiu3=int32#8,>hid15=stack32#52
# asm 2: str <hiu3=r7,>hid15=[sp,#204]
# copy-collector input: str r7,[sp,#204]

# qhasm: goto innerloop
# copy-collector output starts
ldr r0,[sp,#208]
ldr r1,[sp,#212]
ldr r2,[sp,#216]
ldr r3,[sp,#220]
ldr r4,[sp,#224]
ldr r5,[sp,#228]
ldr r6,[sp,#232]
ldr r7,[sp,#236]
strd r0,r1,[sp,#144]
strd r2,r3,[sp,#152]
strd r4,r5,[sp,#160]
strd r6,r7,[sp,#168]
ldr r0,[sp,#240]
ldr r1,[sp,#244]
ldr r2,[sp,#248]
ldr r3,[sp,#252]
ldr r4,[sp,#256]
ldr r5,[sp,#260]
ldr r6,[sp,#264]
ldr r7,[sp,#268]
strd r0,r1,[sp,#176]
strd r2,r3,[sp,#184]
strd r4,r5,[sp,#192]
strd r6,r7,[sp,#200]
# copy-collector output ends
b ._innerloop

# qhasm: endinnerloop:
._endinnerloop:

# qhasm: input_0 = o3
# asm 1: ldr >input_0=int32#1,<o3=stack32#4
# asm 2: ldr >input_0=r0,<o3=[sp,#12]
# copy-collector input: ldr r0,[sp,#12]

# qhasm: input_0 -= 640
# asm 1: sub >input_0=int32#1,<input_0=int32#1,#640
# asm 2: sub >input_0=r0,<input_0=r0,#640
# copy-collector output starts
ldr r0,[sp,#12]
# copy-collector output ends
sub r0,r0,#640

# qhasm: o3 = input_0
# asm 1: str <input_0=int32#1,>o3=stack32#4
# asm 2: str <input_0=r0,>o3=[sp,#12]
# copy-collector input: str r0,[sp,#12]

# qhasm: lou0 = lod0
# asm 1: ldr >lou0=int32#1,<lod0=stack32#13
# asm 2: ldr >lou0=r0,<lod0=[sp,#48]
# copy-collector input: ldr r0,[sp,#48]

# qhasm: hiu0 = hid0
# asm 1: ldr >hiu0=int32#2,<hid0=stack32#14
# asm 2: ldr >hiu0=r1,<hid0=[sp,#52]
# copy-collector input: ldr r1,[sp,#52]

# qhasm: lou1 = lod1
# asm 1: ldr >lou1=int32#3,<lod1=stack32#15
# asm 2: ldr >lou1=r2,<lod1=[sp,#56]
# copy-collector input: ldr r2,[sp,#56]

# qhasm: hiu1 = hid1
# asm 1: ldr >hiu1=int32#4,<hid1=stack32#16
# asm 2: ldr >hiu1=r3,<hid1=[sp,#60]
# copy-collector input: ldr r3,[sp,#60]

# qhasm: lou2 = lod2
# asm 1: ldr >lou2=int32#5,<lod2=stack32#17
# asm 2: ldr >lou2=r4,<lod2=[sp,#64]
# copy-collector input: ldr r4,[sp,#64]

# qhasm: hiu2 = hid2
# asm 1: ldr >hiu2=int32#6,<hid2=stack32#18
# asm 2: ldr >hiu2=r5,<hid2=[sp,#68]
# copy-collector input: ldr r5,[sp,#68]

# qhasm: lou3 = lod3
# asm 1: ldr >lou3=int32#7,<lod3=stack32#19
# asm 2: ldr >lou3=r6,<lod3=[sp,#72]
# copy-collector input: ldr r6,[sp,#72]

# qhasm: hiu3 = hid3
# asm 1: ldr >hiu3=int32#8,<hid3=stack32#20
# asm 2: ldr >hiu3=r7,<hid3=[sp,#76]
# copy-collector input: ldr r7,[sp,#76]

# qhasm: lotmp = lom0
# asm 1: ldr >lotmp=int32#9,<lom0=stack32#5
# asm 2: ldr >lotmp=r8,<lom0=[sp,#16]
# copy-collector input: ldr r8,[sp,#16]

# qhasm: hitmp = him0
# asm 1: ldr >hitmp=int32#10,<him0=stack32#6
# asm 2: ldr >hitmp=r9,<him0=[sp,#20]
# copy-collector input: ldr r9,[sp,#20]

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#1,<lou0=int32#1,<lotmp=int32#9
# asm 2: adds >lou0=r0,<lou0=r0,<lotmp=r8
# copy-collector output starts
str r0,[sp,#12]
ldr r0,[sp,#48]
ldr r1,[sp,#52]
ldr r2,[sp,#56]
ldr r3,[sp,#60]
ldr r4,[sp,#64]
ldr r5,[sp,#68]
ldr r6,[sp,#72]
ldr r7,[sp,#76]
ldr r8,[sp,#16]
ldr r9,[sp,#20]
# copy-collector output ends
adds r0,r0,r8

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#2,<hiu0=int32#2,<hitmp=int32#10
# asm 2: adc >hiu0=r1,<hiu0=r1,<hitmp=r9
adc r1,r1,r9

# qhasm: lotmp = lom1
# asm 1: ldr >lotmp=int32#9,<lom1=stack32#7
# asm 2: ldr >lotmp=r8,<lom1=[sp,#24]
# copy-collector input: ldr r8,[sp,#24]

# qhasm: hitmp = him1
# asm 1: ldr >hitmp=int32#10,<him1=stack32#8
# asm 2: ldr >hitmp=r9,<him1=[sp,#28]
# copy-collector input: ldr r9,[sp,#28]

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#3,<lou1=int32#3,<lotmp=int32#9
# asm 2: adds >lou1=r2,<lou1=r2,<lotmp=r8
# copy-collector output starts
ldrd r8,r9,[sp,#24]
# copy-collector output ends
adds r2,r2,r8

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#4,<hiu1=int32#4,<hitmp=int32#10
# asm 2: adc >hiu1=r3,<hiu1=r3,<hitmp=r9
adc r3,r3,r9

# qhasm: lotmp = lom2
# asm 1: ldr >lotmp=int32#9,<lom2=stack32#9
# asm 2: ldr >lotmp=r8,<lom2=[sp,#32]
# copy-collector input: ldr r8,[sp,#32]

# qhasm: hitmp = him2
# asm 1: ldr >hitmp=int32#10,<him2=stack32#10
# asm 2: ldr >hitmp=r9,<him2=[sp,#36]
# copy-collector input: ldr r9,[sp,#36]

# qhasm: carry? lou2 += lotmp
# asm 1: adds >lou2=int32#5,<lou2=int32#5,<lotmp=int32#9
# asm 2: adds >lou2=r4,<lou2=r4,<lotmp=r8
# copy-collector output starts
ldrd r8,r9,[sp,#32]
# copy-collector output ends
adds r4,r4,r8

# qhasm: hiu2 += hitmp + carry
# asm 1: adc >hiu2=int32#6,<hiu2=int32#6,<hitmp=int32#10
# asm 2: adc >hiu2=r5,<hiu2=r5,<hitmp=r9
adc r5,r5,r9

# qhasm: lotmp = lom3
# asm 1: ldr >lotmp=int32#9,<lom3=stack32#11
# asm 2: ldr >lotmp=r8,<lom3=[sp,#40]
# copy-collector input: ldr r8,[sp,#40]

# qhasm: hitmp = him3
# asm 1: ldr >hitmp=int32#10,<him3=stack32#12
# asm 2: ldr >hitmp=r9,<him3=[sp,#44]
# copy-collector input: ldr r9,[sp,#44]

# qhasm: carry? lou3 += lotmp
# asm 1: adds >lou3=int32#7,<lou3=int32#7,<lotmp=int32#9
# asm 2: adds >lou3=r6,<lou3=r6,<lotmp=r8
# copy-collector output starts
ldrd r8,r9,[sp,#40]
# copy-collector output ends
adds r6,r6,r8

# qhasm: hiu3 += hitmp + carry
# asm 1: adc >hiu3=int32#8,<hiu3=int32#8,<hitmp=int32#10
# asm 2: adc >hiu3=r7,<hiu3=r7,<hitmp=r9
adc r7,r7,r9

# qhasm: lom0 = lou0
# asm 1: str <lou0=int32#1,>lom0=stack32#5
# asm 2: str <lou0=r0,>lom0=[sp,#16]
# copy-collector input: str r0,[sp,#16]

# qhasm: him0 = hiu0
# asm 1: str <hiu0=int32#2,>him0=stack32#6
# asm 2: str <hiu0=r1,>him0=[sp,#20]
# copy-collector input: str r1,[sp,#20]

# qhasm: lom1 = lou1
# asm 1: str <lou1=int32#3,>lom1=stack32#7
# asm 2: str <lou1=r2,>lom1=[sp,#24]
# copy-collector input: str r2,[sp,#24]

# qhasm: him1 = hiu1
# asm 1: str <hiu1=int32#4,>him1=stack32#8
# asm 2: str <hiu1=r3,>him1=[sp,#28]
# copy-collector input: str r3,[sp,#28]

# qhasm: lom2 = lou2
# asm 1: str <lou2=int32#5,>lom2=stack32#9
# asm 2: str <lou2=r4,>lom2=[sp,#32]
# copy-collector input: str r4,[sp,#32]

# qhasm: him2 = hiu2
# asm 1: str <hiu2=int32#6,>him2=stack32#10
# asm 2: str <hiu2=r5,>him2=[sp,#36]
# copy-collector input: str r5,[sp,#36]

# qhasm: lom3 = lou3
# asm 1: str <lou3=int32#7,>lom3=stack32#11
# asm 2: str <lou3=r6,>lom3=[sp,#40]
# copy-collector input: str r6,[sp,#40]

# qhasm: him3 = hiu3
# asm 1: str <hiu3=int32#8,>him3=stack32#12
# asm 2: str <hiu3=r7,>him3=[sp,#44]
# copy-collector input: str r7,[sp,#44]

# qhasm: lod0 = lou0
# asm 1: str <lou0=int32#1,>lod0=stack32#13
# asm 2: str <lou0=r0,>lod0=[sp,#48]
# copy-collector input: str r0,[sp,#48]

# qhasm: hid0 = hiu0
# asm 1: str <hiu0=int32#2,>hid0=stack32#14
# asm 2: str <hiu0=r1,>hid0=[sp,#52]
# copy-collector input: str r1,[sp,#52]

# qhasm: lod1 = lou1
# asm 1: str <lou1=int32#3,>lod1=stack32#15
# asm 2: str <lou1=r2,>lod1=[sp,#56]
# copy-collector input: str r2,[sp,#56]

# qhasm: hid1 = hiu1
# asm 1: str <hiu1=int32#4,>hid1=stack32#16
# asm 2: str <hiu1=r3,>hid1=[sp,#60]
# copy-collector input: str r3,[sp,#60]

# qhasm: lod2 = lou2
# asm 1: str <lou2=int32#5,>lod2=stack32#17
# asm 2: str <lou2=r4,>lod2=[sp,#64]
# copy-collector input: str r4,[sp,#64]

# qhasm: hid2 = hiu2
# asm 1: str <hiu2=int32#6,>hid2=stack32#18
# asm 2: str <hiu2=r5,>hid2=[sp,#68]
# copy-collector input: str r5,[sp,#68]

# qhasm: lod3 = lou3
# asm 1: str <lou3=int32#7,>lod3=stack32#19
# asm 2: str <lou3=r6,>lod3=[sp,#72]
# copy-collector input: str r6,[sp,#72]

# qhasm: hid3 = hiu3
# asm 1: str <hiu3=int32#8,>hid3=stack32#20
# asm 2: str <hiu3=r7,>hid3=[sp,#76]
# copy-collector input: str r7,[sp,#76]

# qhasm: lou0 = lod4
# asm 1: ldr >lou0=int32#1,<lod4=stack32#29
# asm 2: ldr >lou0=r0,<lod4=[sp,#112]
# copy-collector input: ldr r0,[sp,#112]

# qhasm: hiu0 = hid4
# asm 1: ldr >hiu0=int32#2,<hid4=stack32#30
# asm 2: ldr >hiu0=r1,<hid4=[sp,#116]
# copy-collector input: ldr r1,[sp,#116]

# qhasm: lou1 = lod5
# asm 1: ldr >lou1=int32#3,<lod5=stack32#31
# asm 2: ldr >lou1=r2,<lod5=[sp,#120]
# copy-collector input: ldr r2,[sp,#120]

# qhasm: hiu1 = hid5
# asm 1: ldr >hiu1=int32#4,<hid5=stack32#32
# asm 2: ldr >hiu1=r3,<hid5=[sp,#124]
# copy-collector input: ldr r3,[sp,#124]

# qhasm: lou2 = lod6
# asm 1: ldr >lou2=int32#5,<lod6=stack32#33
# asm 2: ldr >lou2=r4,<lod6=[sp,#128]
# copy-collector input: ldr r4,[sp,#128]

# qhasm: hiu2 = hid6
# asm 1: ldr >hiu2=int32#6,<hid6=stack32#34
# asm 2: ldr >hiu2=r5,<hid6=[sp,#132]
# copy-collector input: ldr r5,[sp,#132]

# qhasm: lou3 = lod7
# asm 1: ldr >lou3=int32#7,<lod7=stack32#35
# asm 2: ldr >lou3=r6,<lod7=[sp,#136]
# copy-collector input: ldr r6,[sp,#136]

# qhasm: hiu3 = hid7
# asm 1: ldr >hiu3=int32#8,<hid7=stack32#36
# asm 2: ldr >hiu3=r7,<hid7=[sp,#140]
# copy-collector input: ldr r7,[sp,#140]

# qhasm: lotmp = lom4
# asm 1: ldr >lotmp=int32#9,<lom4=stack32#21
# asm 2: ldr >lotmp=r8,<lom4=[sp,#80]
# copy-collector input: ldr r8,[sp,#80]

# qhasm: hitmp = him4
# asm 1: ldr >hitmp=int32#10,<him4=stack32#22
# asm 2: ldr >hitmp=r9,<him4=[sp,#84]
# copy-collector input: ldr r9,[sp,#84]

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#1,<lou0=int32#1,<lotmp=int32#9
# asm 2: adds >lou0=r0,<lou0=r0,<lotmp=r8
# copy-collector output starts
strd r0,r1,[sp,#16]
strd r2,r3,[sp,#24]
strd r4,r5,[sp,#32]
strd r6,r7,[sp,#40]
strd r0,r1,[sp,#48]
strd r2,r3,[sp,#56]
strd r4,r5,[sp,#64]
strd r6,r7,[sp,#72]
ldr r0,[sp,#112]
ldr r1,[sp,#116]
ldr r2,[sp,#120]
ldr r3,[sp,#124]
ldr r4,[sp,#128]
ldr r5,[sp,#132]
ldr r6,[sp,#136]
ldr r7,[sp,#140]
ldr r8,[sp,#80]
ldr r9,[sp,#84]
# copy-collector output ends
adds r0,r0,r8

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#2,<hiu0=int32#2,<hitmp=int32#10
# asm 2: adc >hiu0=r1,<hiu0=r1,<hitmp=r9
adc r1,r1,r9

# qhasm: lotmp = lom5
# asm 1: ldr >lotmp=int32#9,<lom5=stack32#23
# asm 2: ldr >lotmp=r8,<lom5=[sp,#88]
# copy-collector input: ldr r8,[sp,#88]

# qhasm: hitmp = him5
# asm 1: ldr >hitmp=int32#10,<him5=stack32#24
# asm 2: ldr >hitmp=r9,<him5=[sp,#92]
# copy-collector input: ldr r9,[sp,#92]

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#3,<lou1=int32#3,<lotmp=int32#9
# asm 2: adds >lou1=r2,<lou1=r2,<lotmp=r8
# copy-collector output starts
ldrd r8,r9,[sp,#88]
# copy-collector output ends
adds r2,r2,r8

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#4,<hiu1=int32#4,<hitmp=int32#10
# asm 2: adc >hiu1=r3,<hiu1=r3,<hitmp=r9
adc r3,r3,r9

# qhasm: lotmp = lom6
# asm 1: ldr >lotmp=int32#9,<lom6=stack32#25
# asm 2: ldr >lotmp=r8,<lom6=[sp,#96]
# copy-collector input: ldr r8,[sp,#96]

# qhasm: hitmp = him6
# asm 1: ldr >hitmp=int32#10,<him6=stack32#26
# asm 2: ldr >hitmp=r9,<him6=[sp,#100]
# copy-collector input: ldr r9,[sp,#100]

# qhasm: carry? lou2 += lotmp
# asm 1: adds >lou2=int32#5,<lou2=int32#5,<lotmp=int32#9
# asm 2: adds >lou2=r4,<lou2=r4,<lotmp=r8
# copy-collector output starts
ldrd r8,r9,[sp,#96]
# copy-collector output ends
adds r4,r4,r8

# qhasm: hiu2 += hitmp + carry
# asm 1: adc >hiu2=int32#6,<hiu2=int32#6,<hitmp=int32#10
# asm 2: adc >hiu2=r5,<hiu2=r5,<hitmp=r9
adc r5,r5,r9

# qhasm: lotmp = lom7
# asm 1: ldr >lotmp=int32#9,<lom7=stack32#27
# asm 2: ldr >lotmp=r8,<lom7=[sp,#104]
# copy-collector input: ldr r8,[sp,#104]

# qhasm: hitmp = him7
# asm 1: ldr >hitmp=int32#10,<him7=stack32#28
# asm 2: ldr >hitmp=r9,<him7=[sp,#108]
# copy-collector input: ldr r9,[sp,#108]

# qhasm: carry? lou3 += lotmp
# asm 1: adds >lou3=int32#7,<lou3=int32#7,<lotmp=int32#9
# asm 2: adds >lou3=r6,<lou3=r6,<lotmp=r8
# copy-collector output starts
ldrd r8,r9,[sp,#104]
# copy-collector output ends
adds r6,r6,r8

# qhasm: hiu3 += hitmp + carry
# asm 1: adc >hiu3=int32#8,<hiu3=int32#8,<hitmp=int32#10
# asm 2: adc >hiu3=r7,<hiu3=r7,<hitmp=r9
adc r7,r7,r9

# qhasm: lom4 = lou0
# asm 1: str <lou0=int32#1,>lom4=stack32#21
# asm 2: str <lou0=r0,>lom4=[sp,#80]
# copy-collector input: str r0,[sp,#80]

# qhasm: him4 = hiu0
# asm 1: str <hiu0=int32#2,>him4=stack32#22
# asm 2: str <hiu0=r1,>him4=[sp,#84]
# copy-collector input: str r1,[sp,#84]

# qhasm: lom5 = lou1
# asm 1: str <lou1=int32#3,>lom5=stack32#23
# asm 2: str <lou1=r2,>lom5=[sp,#88]
# copy-collector input: str r2,[sp,#88]

# qhasm: him5 = hiu1
# asm 1: str <hiu1=int32#4,>him5=stack32#24
# asm 2: str <hiu1=r3,>him5=[sp,#92]
# copy-collector input: str r3,[sp,#92]

# qhasm: lom6 = lou2
# asm 1: str <lou2=int32#5,>lom6=stack32#25
# asm 2: str <lou2=r4,>lom6=[sp,#96]
# copy-collector input: str r4,[sp,#96]

# qhasm: him6 = hiu2
# asm 1: str <hiu2=int32#6,>him6=stack32#26
# asm 2: str <hiu2=r5,>him6=[sp,#100]
# copy-collector input: str r5,[sp,#100]

# qhasm: lom7 = lou3
# asm 1: str <lou3=int32#7,>lom7=stack32#27
# asm 2: str <lou3=r6,>lom7=[sp,#104]
# copy-collector input: str r6,[sp,#104]

# qhasm: him7 = hiu3
# asm 1: str <hiu3=int32#8,>him7=stack32#28
# asm 2: str <hiu3=r7,>him7=[sp,#108]
# copy-collector input: str r7,[sp,#108]

# qhasm: lod4 = lou0
# asm 1: str <lou0=int32#1,>lod4=stack32#29
# asm 2: str <lou0=r0,>lod4=[sp,#112]
# copy-collector input: str r0,[sp,#112]

# qhasm: hid4 = hiu0
# asm 1: str <hiu0=int32#2,>hid4=stack32#30
# asm 2: str <hiu0=r1,>hid4=[sp,#116]
# copy-collector input: str r1,[sp,#116]

# qhasm: lod5 = lou1
# asm 1: str <lou1=int32#3,>lod5=stack32#31
# asm 2: str <lou1=r2,>lod5=[sp,#120]
# copy-collector input: str r2,[sp,#120]

# qhasm: hid5 = hiu1
# asm 1: str <hiu1=int32#4,>hid5=stack32#32
# asm 2: str <hiu1=r3,>hid5=[sp,#124]
# copy-collector input: str r3,[sp,#124]

# qhasm: lod6 = lou2
# asm 1: str <lou2=int32#5,>lod6=stack32#33
# asm 2: str <lou2=r4,>lod6=[sp,#128]
# copy-collector input: str r4,[sp,#128]

# qhasm: hid6 = hiu2
# asm 1: str <hiu2=int32#6,>hid6=stack32#34
# asm 2: str <hiu2=r5,>hid6=[sp,#132]
# copy-collector input: str r5,[sp,#132]

# qhasm: lod7 = lou3
# asm 1: str <lou3=int32#7,>lod7=stack32#35
# asm 2: str <lou3=r6,>lod7=[sp,#136]
# copy-collector input: str r6,[sp,#136]

# qhasm: hid7 = hiu3
# asm 1: str <hiu3=int32#8,>hid7=stack32#36
# asm 2: str <hiu3=r7,>hid7=[sp,#140]
# copy-collector input: str r7,[sp,#140]

# qhasm: input_0 = o2
# asm 1: ldr >input_0=int32#1,<o2=stack32#3
# asm 2: ldr >input_0=r0,<o2=[sp,#8]
# copy-collector input: ldr r0,[sp,#8]

# qhasm: =? unsigned<? input_0 -= 128
# asm 1: subs >input_0=int32#1,<input_0=int32#1,#128
# asm 2: subs >input_0=r0,<input_0=r0,#128
# copy-collector output starts
strd r0,r1,[sp,#80]
strd r2,r3,[sp,#88]
strd r4,r5,[sp,#96]
strd r6,r7,[sp,#104]
strd r0,r1,[sp,#112]
strd r2,r3,[sp,#120]
strd r4,r5,[sp,#128]
strd r6,r7,[sp,#136]
ldr r0,[sp,#8]
# copy-collector output ends
subs r0,r0,#128

# qhasm: o2 = input_0
# asm 1: str <input_0=int32#1,>o2=stack32#3
# asm 2: str <input_0=r0,>o2=[sp,#8]
# copy-collector input: str r0,[sp,#8]

# qhasm: goto mainloop if !unsigned<
# copy-collector output starts
str r0,[sp,#8]
# copy-collector output ends
bhs ._mainloop

# qhasm: endmainloop:
._endmainloop:

# qhasm: input_1 = o0
# asm 1: ldr >input_1=int32#2,<o0=stack32#1
# asm 2: ldr >input_1=r1,<o0=[sp,#0]
# copy-collector input: ldr r1,[sp,#0]

# qhasm: lou0 = lom0
# asm 1: ldr >lou0=int32#3,<lom0=stack32#5
# asm 2: ldr >lou0=r2,<lom0=[sp,#16]
# copy-collector input: ldr r2,[sp,#16]

# qhasm: hiu0 = him0
# asm 1: ldr >hiu0=int32#4,<him0=stack32#6
# asm 2: ldr >hiu0=r3,<him0=[sp,#20]
# copy-collector input: ldr r3,[sp,#20]

# qhasm: lou1 = lom1
# asm 1: ldr >lou1=int32#5,<lom1=stack32#7
# asm 2: ldr >lou1=r4,<lom1=[sp,#24]
# copy-collector input: ldr r4,[sp,#24]

# qhasm: hiu1 = him1
# asm 1: ldr >hiu1=int32#6,<him1=stack32#8
# asm 2: ldr >hiu1=r5,<him1=[sp,#28]
# copy-collector input: ldr r5,[sp,#28]

# qhasm: lou2 = lom2
# asm 1: ldr >lou2=int32#7,<lom2=stack32#9
# asm 2: ldr >lou2=r6,<lom2=[sp,#32]
# copy-collector input: ldr r6,[sp,#32]

# qhasm: hiu2 = him2
# asm 1: ldr >hiu2=int32#8,<him2=stack32#10
# asm 2: ldr >hiu2=r7,<him2=[sp,#36]
# copy-collector input: ldr r7,[sp,#36]

# qhasm: lou3 = lom3
# asm 1: ldr >lou3=int32#9,<lom3=stack32#11
# asm 2: ldr >lou3=r8,<lom3=[sp,#40]
# copy-collector input: ldr r8,[sp,#40]

# qhasm: hiu3 = him3
# asm 1: ldr >hiu3=int32#10,<him3=stack32#12
# asm 2: ldr >hiu3=r9,<him3=[sp,#44]
# copy-collector input: ldr r9,[sp,#44]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
ldr r1,[sp,#0]
ldr r2,[sp,#16]
ldr r3,[sp,#20]
ldr r4,[sp,#24]
ldr r5,[sp,#28]
ldr r6,[sp,#32]
ldr r7,[sp,#36]
ldr r8,[sp,#40]
ldr r9,[sp,#44]
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#4,<hiu0=int32#4
# asm 2: rev >hiu0=r3,<hiu0=r3
rev r3,r3

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#6,<hiu1=int32#6
# asm 2: rev >hiu1=r5,<hiu1=r5
rev r5,r5

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#8,<hiu2=int32#8
# asm 2: rev >hiu2=r7,<hiu2=r7
rev r7,r7

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#9,<lou3=int32#9
# asm 2: rev >lou3=r8,<lou3=r8
rev r8,r8

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#10,<hiu3=int32#10
# asm 2: rev >hiu3=r9,<hiu3=r9
rev r9,r9

# qhasm: mem32[input_1] = hiu0
# asm 1: str <hiu0=int32#4,[<input_1=int32#2]
# asm 2: str <hiu0=r3,[<input_1=r1]
# copy-collector input: str r3,[r1]

# qhasm: mem32[input_1+4] = lou0
# asm 1: str <lou0=int32#3,[<input_1=int32#2,#4]
# asm 2: str <lou0=r2,[<input_1=r1,#4]
# copy-collector input: str r2,[r1,#4]

# qhasm: mem32[input_1+8] = hiu1
# asm 1: str <hiu1=int32#6,[<input_1=int32#2,#8]
# asm 2: str <hiu1=r5,[<input_1=r1,#8]
# copy-collector input: str r5,[r1,#8]

# qhasm: mem32[input_1+12] = lou1
# asm 1: str <lou1=int32#5,[<input_1=int32#2,#12]
# asm 2: str <lou1=r4,[<input_1=r1,#12]
# copy-collector input: str r4,[r1,#12]

# qhasm: mem32[input_1+16] = hiu2
# asm 1: str <hiu2=int32#8,[<input_1=int32#2,#16]
# asm 2: str <hiu2=r7,[<input_1=r1,#16]
# copy-collector input: str r7,[r1,#16]

# qhasm: mem32[input_1+20] = lou2
# asm 1: str <lou2=int32#7,[<input_1=int32#2,#20]
# asm 2: str <lou2=r6,[<input_1=r1,#20]
# copy-collector input: str r6,[r1,#20]

# qhasm: mem32[input_1+24] = hiu3
# asm 1: str <hiu3=int32#10,[<input_1=int32#2,#24]
# asm 2: str <hiu3=r9,[<input_1=r1,#24]
# copy-collector input: str r9,[r1,#24]

# qhasm: mem32[input_1+28] = lou3
# asm 1: str <lou3=int32#9,[<input_1=int32#2,#28]
# asm 2: str <lou3=r8,[<input_1=r1,#28]
# copy-collector input: str r8,[r1,#28]

# qhasm: lou0 = lom4
# asm 1: ldr >lou0=int32#3,<lom4=stack32#21
# asm 2: ldr >lou0=r2,<lom4=[sp,#80]
# copy-collector input: ldr r2,[sp,#80]

# qhasm: hiu0 = him4
# asm 1: ldr >hiu0=int32#4,<him4=stack32#22
# asm 2: ldr >hiu0=r3,<him4=[sp,#84]
# copy-collector input: ldr r3,[sp,#84]

# qhasm: lou1 = lom5
# asm 1: ldr >lou1=int32#5,<lom5=stack32#23
# asm 2: ldr >lou1=r4,<lom5=[sp,#88]
# copy-collector input: ldr r4,[sp,#88]

# qhasm: hiu1 = him5
# asm 1: ldr >hiu1=int32#6,<him5=stack32#24
# asm 2: ldr >hiu1=r5,<him5=[sp,#92]
# copy-collector input: ldr r5,[sp,#92]

# qhasm: lou2 = lom6
# asm 1: ldr >lou2=int32#7,<lom6=stack32#25
# asm 2: ldr >lou2=r6,<lom6=[sp,#96]
# copy-collector input: ldr r6,[sp,#96]

# qhasm: hiu2 = him6
# asm 1: ldr >hiu2=int32#8,<him6=stack32#26
# asm 2: ldr >hiu2=r7,<him6=[sp,#100]
# copy-collector input: ldr r7,[sp,#100]

# qhasm: lou3 = lom7
# asm 1: ldr >lou3=int32#9,<lom7=stack32#27
# asm 2: ldr >lou3=r8,<lom7=[sp,#104]
# copy-collector input: ldr r8,[sp,#104]

# qhasm: hiu3 = him7
# asm 1: ldr >hiu3=int32#10,<him7=stack32#28
# asm 2: ldr >hiu3=r9,<him7=[sp,#108]
# copy-collector input: ldr r9,[sp,#108]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
str r3,[r1]
str r2,[r1,#4]
str r5,[r1,#8]
str r4,[r1,#12]
str r7,[r1,#16]
str r6,[r1,#20]
str r9,[r1,#24]
str r8,[r1,#28]
ldr r2,[sp,#80]
ldr r3,[sp,#84]
ldr r4,[sp,#88]
ldr r5,[sp,#92]
ldr r6,[sp,#96]
ldr r7,[sp,#100]
ldr r8,[sp,#104]
ldr r9,[sp,#108]
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#4,<hiu0=int32#4
# asm 2: rev >hiu0=r3,<hiu0=r3
rev r3,r3

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#5,<lou1=int32#5
# asm 2: rev >lou1=r4,<lou1=r4
rev r4,r4

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#6,<hiu1=int32#6
# asm 2: rev >hiu1=r5,<hiu1=r5
rev r5,r5

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#7,<lou2=int32#7
# asm 2: rev >lou2=r6,<lou2=r6
rev r6,r6

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#8,<hiu2=int32#8
# asm 2: rev >hiu2=r7,<hiu2=r7
rev r7,r7

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#9,<lou3=int32#9
# asm 2: rev >lou3=r8,<lou3=r8
rev r8,r8

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#10,<hiu3=int32#10
# asm 2: rev >hiu3=r9,<hiu3=r9
rev r9,r9

# qhasm: mem32[input_1+32] = hiu0
# asm 1: str <hiu0=int32#4,[<input_1=int32#2,#32]
# asm 2: str <hiu0=r3,[<input_1=r1,#32]
# copy-collector input: str r3,[r1,#32]

# qhasm: mem32[input_1+36] = lou0
# asm 1: str <lou0=int32#3,[<input_1=int32#2,#36]
# asm 2: str <lou0=r2,[<input_1=r1,#36]
# copy-collector input: str r2,[r1,#36]

# qhasm: mem32[input_1+40] = hiu1
# asm 1: str <hiu1=int32#6,[<input_1=int32#2,#40]
# asm 2: str <hiu1=r5,[<input_1=r1,#40]
# copy-collector input: str r5,[r1,#40]

# qhasm: mem32[input_1+44] = lou1
# asm 1: str <lou1=int32#5,[<input_1=int32#2,#44]
# asm 2: str <lou1=r4,[<input_1=r1,#44]
# copy-collector input: str r4,[r1,#44]

# qhasm: mem32[input_1+48] = hiu2
# asm 1: str <hiu2=int32#8,[<input_1=int32#2,#48]
# asm 2: str <hiu2=r7,[<input_1=r1,#48]
# copy-collector input: str r7,[r1,#48]

# qhasm: mem32[input_1+52] = lou2
# asm 1: str <lou2=int32#7,[<input_1=int32#2,#52]
# asm 2: str <lou2=r6,[<input_1=r1,#52]
# copy-collector input: str r6,[r1,#52]

# qhasm: mem32[input_1+56] = hiu3
# asm 1: str <hiu3=int32#10,[<input_1=int32#2,#56]
# asm 2: str <hiu3=r9,[<input_1=r1,#56]
# copy-collector input: str r9,[r1,#56]

# qhasm: mem32[input_1+60] = lou3
# asm 1: str <lou3=int32#9,[<input_1=int32#2,#60]
# asm 2: str <lou3=r8,[<input_1=r1,#60]
# copy-collector input: str r8,[r1,#60]

# qhasm: input_0 += 128
# asm 1: add >input_0=int32#1,<input_0=int32#1,#128
# asm 2: add >input_0=r0,<input_0=r0,#128
# copy-collector output starts
str r3,[r1,#32]
str r2,[r1,#36]
str r5,[r1,#40]
str r4,[r1,#44]
str r7,[r1,#48]
str r6,[r1,#52]
str r9,[r1,#56]
str r8,[r1,#60]
# copy-collector output ends
add r0,r0,#128

# qhasm: rpopreturn input_0
add.w sp,sp,#288
pop {r4,r5,r6,r7,r8,r9,r10,r11,r14}
bx lr
