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
.fpu fpv4-sp-d16
.syntax unified

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

# qhasm: float32 hid0

# qhasm: float32 hid1

# qhasm: float32 hid2

# qhasm: float32 hid3

# qhasm: float32 hid4

# qhasm: float32 hid5

# qhasm: float32 hid6

# qhasm: float32 hid7

# qhasm: float32 hid8

# qhasm: float32 hid9

# qhasm: float32 hid10

# qhasm: float32 hid11

# qhasm: float32 hid12

# qhasm: float32 hid13

# qhasm: float32 hid14

# qhasm: float32 hid15

# qhasm: float32 lod0

# qhasm: float32 lod1

# qhasm: float32 lod2

# qhasm: float32 lod3

# qhasm: float32 lod4

# qhasm: float32 lod5

# qhasm: float32 lod6

# qhasm: float32 lod7

# qhasm: float32 lod8

# qhasm: float32 lod9

# qhasm: float32 lod10

# qhasm: float32 lod11

# qhasm: float32 lod12

# qhasm: float32 lod13

# qhasm: float32 lod14

# qhasm: float32 lod15

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

# qhasm: pushenter CRYPTO_SHARED_NAMESPACE(inner)
.p2align 2
.global CRYPTO_SHARED_NAMESPACE(inner)
.type CRYPTO_SHARED_NAMESPACE(inner),%function
.thumb
.thumb_func
CRYPTO_SHARED_NAMESPACE(inner):
push {r4,r5,r6,r7,r8,r9,r10,r11,r14}
vpush {d8-d15}
sub.w sp,sp,#160

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

# qhasm: hiu0 = mem32[input_0]
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1]
# asm 2: ldr >hiu0=r1,[<input_0=r0]
# copy-collector input: ldr r1,[r0]

# qhasm: lou0 = mem32[input_0+4]
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1,#4]
# asm 2: ldr >lou0=r2,[<input_0=r0,#4]
# copy-collector input: ldr r2,[r0,#4]

# qhasm: hiu1 = mem32[input_0+8]
# asm 1: ldr >hiu1=int32#5,[<input_0=int32#1,#8]
# asm 2: ldr >hiu1=r4,[<input_0=r0,#8]
# copy-collector input: ldr r4,[r0,#8]

# qhasm: lou1 = mem32[input_0+12]
# asm 1: ldr >lou1=int32#6,[<input_0=int32#1,#12]
# asm 2: ldr >lou1=r5,[<input_0=r0,#12]
# copy-collector input: ldr r5,[r0,#12]

# qhasm: hiu2 = mem32[input_0+16]
# asm 1: ldr >hiu2=int32#7,[<input_0=int32#1,#16]
# asm 2: ldr >hiu2=r6,[<input_0=r0,#16]
# copy-collector input: ldr r6,[r0,#16]

# qhasm: lou2 = mem32[input_0+20]
# asm 1: ldr >lou2=int32#8,[<input_0=int32#1,#20]
# asm 2: ldr >lou2=r7,[<input_0=r0,#20]
# copy-collector input: ldr r7,[r0,#20]

# qhasm: hiu3 = mem32[input_0+24]
# asm 1: ldr >hiu3=int32#9,[<input_0=int32#1,#24]
# asm 2: ldr >hiu3=r8,[<input_0=r0,#24]
# copy-collector input: ldr r8,[r0,#24]

# qhasm: lou3 = mem32[input_0+28]
# asm 1: ldr >lou3=int32#10,[<input_0=int32#1,#28]
# asm 2: ldr >lou3=r9,[<input_0=r0,#28]
# copy-collector input: ldr r9,[r0,#28]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
str r1,[sp,#8]
ldr r1,[r0]
ldr r2,[r0,#4]
ldr r4,[r0,#8]
ldr r5,[r0,#12]
ldr r6,[r0,#16]
ldr.w r7,[r0,#20]
ldr r8,[r0,#24]
ldr r9,[r0,#28]
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#6,<lou1=int32#6
# asm 2: rev >lou1=r5,<lou1=r5
rev r5,r5

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#5,<hiu1=int32#5
# asm 2: rev >hiu1=r4,<hiu1=r4
rev r4,r4

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#8,<lou2=int32#8
# asm 2: rev >lou2=r7,<lou2=r7
rev r7,r7

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#7,<hiu2=int32#7
# asm 2: rev >hiu2=r6,<hiu2=r6
rev r6,r6

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#10,<lou3=int32#10
# asm 2: rev >lou3=r9,<lou3=r9
rev r9,r9

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#9,<hiu3=int32#9
# asm 2: rev >hiu3=r8,<hiu3=r8
rev r8,r8

# qhasm: lom0 = lou0
# asm 1: str <lou0=int32#3,>lom0=stack32#4
# asm 2: str <lou0=r2,>lom0=[sp,#12]
# copy-collector input: str r2,[sp,#12]

# qhasm: him0 = hiu0
# asm 1: str <hiu0=int32#2,>him0=stack32#5
# asm 2: str <hiu0=r1,>him0=[sp,#16]
# copy-collector input: str r1,[sp,#16]

# qhasm: lom1 = lou1
# asm 1: str <lou1=int32#6,>lom1=stack32#6
# asm 2: str <lou1=r5,>lom1=[sp,#20]
# copy-collector input: str r5,[sp,#20]

# qhasm: him1 = hiu1
# asm 1: str <hiu1=int32#5,>him1=stack32#7
# asm 2: str <hiu1=r4,>him1=[sp,#24]
# copy-collector input: str r4,[sp,#24]

# qhasm: lom2 = lou2
# asm 1: str <lou2=int32#8,>lom2=stack32#8
# asm 2: str <lou2=r7,>lom2=[sp,#28]
# copy-collector input: str r7,[sp,#28]

# qhasm: him2 = hiu2
# asm 1: str <hiu2=int32#7,>him2=stack32#9
# asm 2: str <hiu2=r6,>him2=[sp,#32]
# copy-collector input: str r6,[sp,#32]

# qhasm: lom3 = lou3
# asm 1: str <lou3=int32#10,>lom3=stack32#10
# asm 2: str <lou3=r9,>lom3=[sp,#36]
# copy-collector input: str r9,[sp,#36]

# qhasm: him3 = hiu3
# asm 1: str <hiu3=int32#9,>him3=stack32#11
# asm 2: str <hiu3=r8,>him3=[sp,#40]
# copy-collector input: str r8,[sp,#40]

# qhasm: lod0 = lou0
# asm 1: vmov >lod0=float32#1,<lou0=int32#3
# asm 2: vmov >lod0=s0,<lou0=r2
# copy-collector input: vmov s0,r2

# qhasm: hid0 = hiu0
# asm 1: vmov >hid0=float32#2,<hiu0=int32#2
# asm 2: vmov >hid0=s1,<hiu0=r1
# copy-collector input: vmov s1,r1

# qhasm: lod1 = lou1
# asm 1: vmov >lod1=float32#3,<lou1=int32#6
# asm 2: vmov >lod1=s2,<lou1=r5
# copy-collector input: vmov s2,r5

# qhasm: hid1 = hiu1
# asm 1: vmov >hid1=float32#4,<hiu1=int32#5
# asm 2: vmov >hid1=s3,<hiu1=r4
# copy-collector input: vmov s3,r4

# qhasm: lod2 = lou2
# asm 1: vmov >lod2=float32#5,<lou2=int32#8
# asm 2: vmov >lod2=s4,<lou2=r7
# copy-collector input: vmov s4,r7

# qhasm: hid2 = hiu2
# asm 1: vmov >hid2=float32#6,<hiu2=int32#7
# asm 2: vmov >hid2=s5,<hiu2=r6
# copy-collector input: vmov s5,r6

# qhasm: lod3 = lou3
# asm 1: vmov >lod3=float32#7,<lou3=int32#10
# asm 2: vmov >lod3=s6,<lou3=r9
# copy-collector input: vmov s6,r9

# qhasm: hid3 = hiu3
# asm 1: vmov >hid3=float32#8,<hiu3=int32#9
# asm 2: vmov >hid3=s7,<hiu3=r8
# copy-collector input: vmov s7,r8

# qhasm: hiu0 = mem32[input_0+32]
# asm 1: ldr >hiu0=int32#2,[<input_0=int32#1,#32]
# asm 2: ldr >hiu0=r1,[<input_0=r0,#32]
# copy-collector input: ldr r1,[r0,#32]

# qhasm: lou0 = mem32[input_0+36]
# asm 1: ldr >lou0=int32#3,[<input_0=int32#1,#36]
# asm 2: ldr >lou0=r2,[<input_0=r0,#36]
# copy-collector input: ldr r2,[r0,#36]

# qhasm: hiu1 = mem32[input_0+40]
# asm 1: ldr >hiu1=int32#5,[<input_0=int32#1,#40]
# asm 2: ldr >hiu1=r4,[<input_0=r0,#40]
# copy-collector input: ldr r4,[r0,#40]

# qhasm: lou1 = mem32[input_0+44]
# asm 1: ldr >lou1=int32#6,[<input_0=int32#1,#44]
# asm 2: ldr >lou1=r5,[<input_0=r0,#44]
# copy-collector input: ldr r5,[r0,#44]

# qhasm: hiu2 = mem32[input_0+48]
# asm 1: ldr >hiu2=int32#7,[<input_0=int32#1,#48]
# asm 2: ldr >hiu2=r6,[<input_0=r0,#48]
# copy-collector input: ldr r6,[r0,#48]

# qhasm: lou2 = mem32[input_0+52]
# asm 1: ldr >lou2=int32#8,[<input_0=int32#1,#52]
# asm 2: ldr >lou2=r7,[<input_0=r0,#52]
# copy-collector input: ldr r7,[r0,#52]

# qhasm: hiu3 = mem32[input_0+56]
# asm 1: ldr >hiu3=int32#9,[<input_0=int32#1,#56]
# asm 2: ldr >hiu3=r8,[<input_0=r0,#56]
# copy-collector input: ldr r8,[r0,#56]

# qhasm: lou3 = mem32[input_0+60]
# asm 1: ldr >lou3=int32#1,[<input_0=int32#1,#60]
# asm 2: ldr >lou3=r0,[<input_0=r0,#60]
# copy-collector input: ldr r0,[r0,#60]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
strd r2,r1,[sp,#12]
strd r5,r4,[sp,#20]
strd r7,r6,[sp,#28]
strd r9,r8,[sp,#36]
vmov d0,r2,r1
vmov d1,r5,r4
vmov d2,r7,r6
vmov d3,r9,r8
ldr r1,[r0,#32]
ldr r2,[r0,#36]
ldr r4,[r0,#40]
ldr r5,[r0,#44]
ldr r6,[r0,#48]
ldr r7,[r0,#52]
ldr r8,[r0,#56]
ldr r0,[r0,#60]
# copy-collector output ends
rev r2,r2

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#2,<hiu0=int32#2
# asm 2: rev >hiu0=r1,<hiu0=r1
rev r1,r1

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#6,<lou1=int32#6
# asm 2: rev >lou1=r5,<lou1=r5
rev r5,r5

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#5,<hiu1=int32#5
# asm 2: rev >hiu1=r4,<hiu1=r4
rev r4,r4

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#8,<lou2=int32#8
# asm 2: rev >lou2=r7,<lou2=r7
rev r7,r7

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#7,<hiu2=int32#7
# asm 2: rev >hiu2=r6,<hiu2=r6
rev r6,r6

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#1,<lou3=int32#1
# asm 2: rev >lou3=r0,<lou3=r0
rev r0,r0

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#9,<hiu3=int32#9
# asm 2: rev >hiu3=r8,<hiu3=r8
rev r8,r8

# qhasm: lom4 = lou0
# asm 1: str <lou0=int32#3,>lom4=stack32#12
# asm 2: str <lou0=r2,>lom4=[sp,#44]
# copy-collector input: str r2,[sp,#44]

# qhasm: him4 = hiu0
# asm 1: str <hiu0=int32#2,>him4=stack32#13
# asm 2: str <hiu0=r1,>him4=[sp,#48]
# copy-collector input: str r1,[sp,#48]

# qhasm: lom5 = lou1
# asm 1: str <lou1=int32#6,>lom5=stack32#14
# asm 2: str <lou1=r5,>lom5=[sp,#52]
# copy-collector input: str r5,[sp,#52]

# qhasm: him5 = hiu1
# asm 1: str <hiu1=int32#5,>him5=stack32#15
# asm 2: str <hiu1=r4,>him5=[sp,#56]
# copy-collector input: str r4,[sp,#56]

# qhasm: lom6 = lou2
# asm 1: str <lou2=int32#8,>lom6=stack32#16
# asm 2: str <lou2=r7,>lom6=[sp,#60]
# copy-collector input: str r7,[sp,#60]

# qhasm: him6 = hiu2
# asm 1: str <hiu2=int32#7,>him6=stack32#17
# asm 2: str <hiu2=r6,>him6=[sp,#64]
# copy-collector input: str r6,[sp,#64]

# qhasm: lom7 = lou3
# asm 1: str <lou3=int32#1,>lom7=stack32#18
# asm 2: str <lou3=r0,>lom7=[sp,#68]
# copy-collector input: str r0,[sp,#68]

# qhasm: him7 = hiu3
# asm 1: str <hiu3=int32#9,>him7=stack32#19
# asm 2: str <hiu3=r8,>him7=[sp,#72]
# copy-collector input: str r8,[sp,#72]

# qhasm: lod4 = lou0
# asm 1: vmov >lod4=float32#9,<lou0=int32#3
# asm 2: vmov >lod4=s8,<lou0=r2
# copy-collector input: vmov s8,r2

# qhasm: hid4 = hiu0
# asm 1: vmov >hid4=float32#10,<hiu0=int32#2
# asm 2: vmov >hid4=s9,<hiu0=r1
# copy-collector input: vmov s9,r1

# qhasm: lod5 = lou1
# asm 1: vmov >lod5=float32#11,<lou1=int32#6
# asm 2: vmov >lod5=s10,<lou1=r5
# copy-collector input: vmov s10,r5

# qhasm: hid5 = hiu1
# asm 1: vmov >hid5=float32#12,<hiu1=int32#5
# asm 2: vmov >hid5=s11,<hiu1=r4
# copy-collector input: vmov s11,r4

# qhasm: lod6 = lou2
# asm 1: vmov >lod6=float32#13,<lou2=int32#8
# asm 2: vmov >lod6=s12,<lou2=r7
# copy-collector input: vmov s12,r7

# qhasm: hid6 = hiu2
# asm 1: vmov >hid6=float32#14,<hiu2=int32#7
# asm 2: vmov >hid6=s13,<hiu2=r6
# copy-collector input: vmov s13,r6

# qhasm: lod7 = lou3
# asm 1: vmov >lod7=float32#15,<lou3=int32#1
# asm 2: vmov >lod7=s14,<lou3=r0
# copy-collector input: vmov s14,r0

# qhasm: hid7 = hiu3
# asm 1: vmov >hid7=float32#16,<hiu3=int32#9
# asm 2: vmov >hid7=s15,<hiu3=r8
# copy-collector input: vmov s15,r8

# qhasm: input_1 = input_3
# asm 1: mov >input_1=int32#1,<input_3=int32#4
# asm 2: mov >input_1=r0,<input_3=r3
# copy-collector output starts
strd r2,r1,[sp,#44]
strd r5,r4,[sp,#52]
strd r7,r6,[sp,#60]
strd r0,r8,[sp,#68]
vmov d4,r2,r1
vmov d5,r5,r4
vmov d6,r7,r6
vmov d7,r0,r8
# copy-collector output ends
mov r0,r3

# qhasm: mainloop:
._mainloop:

# qhasm: input_0 = o1
# asm 1: ldr >input_0=int32#2,<o1=stack32#2
# asm 2: ldr >input_0=r1,<o1=[sp,#4]
# copy-collector input: ldr r1,[sp,#4]

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#3,[<input_0=int32#2],#4
# asm 2: ldr >hiu0=r2,[<input_0=r1],#4
# copy-collector input: ldr r2,[r1],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#4,[<input_0=int32#2],#4
# asm 2: ldr >lou0=r3,[<input_0=r1],#4
# copy-collector input: ldr r3,[r1],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#5,[<input_0=int32#2],#4
# asm 2: ldr >hiu1=r4,[<input_0=r1],#4
# copy-collector input: ldr r4,[r1],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#6,[<input_0=int32#2],#4
# asm 2: ldr >lou1=r5,[<input_0=r1],#4
# copy-collector input: ldr r5,[r1],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#7,[<input_0=int32#2],#4
# asm 2: ldr >hiu2=r6,[<input_0=r1],#4
# copy-collector input: ldr r6,[r1],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#8,[<input_0=int32#2],#4
# asm 2: ldr >lou2=r7,[<input_0=r1],#4
# copy-collector input: ldr r7,[r1],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#9,[<input_0=int32#2],#4
# asm 2: ldr >hiu3=r8,[<input_0=r1],#4
# copy-collector input: ldr r8,[r1],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#10,[<input_0=int32#2],#4
# asm 2: ldr >lou3=r9,[<input_0=r1],#4
# copy-collector input: ldr r9,[r1],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#4,<lou0=int32#4
# asm 2: rev >lou0=r3,<lou0=r3
# copy-collector output starts
ldr r1,[sp,#4]
ldr r2,[r1],#4
ldr r3,[r1],#4
ldr r4,[r1],#4
ldr r5,[r1],#4
ldr r6,[r1],#4
ldr r7,[r1],#4
ldr r8,[r1],#4
ldr r9,[r1],#4
# copy-collector output ends
rev r3,r3

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#3,<hiu0=int32#3
# asm 2: rev >hiu0=r2,<hiu0=r2
rev r2,r2

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#6,<lou1=int32#6
# asm 2: rev >lou1=r5,<lou1=r5
rev r5,r5

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#5,<hiu1=int32#5
# asm 2: rev >hiu1=r4,<hiu1=r4
rev r4,r4

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#8,<lou2=int32#8
# asm 2: rev >lou2=r7,<lou2=r7
rev r7,r7

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#7,<hiu2=int32#7
# asm 2: rev >hiu2=r6,<hiu2=r6
rev r6,r6

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#10,<lou3=int32#10
# asm 2: rev >lou3=r9,<lou3=r9
rev r9,r9

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#9,<hiu3=int32#9
# asm 2: rev >hiu3=r8,<hiu3=r8
rev r8,r8

# qhasm: lod8 = lou0
# asm 1: vmov >lod8=float32#17,<lou0=int32#4
# asm 2: vmov >lod8=s16,<lou0=r3
# copy-collector input: vmov s16,r3

# qhasm: hid8 = hiu0
# asm 1: vmov >hid8=float32#18,<hiu0=int32#3
# asm 2: vmov >hid8=s17,<hiu0=r2
# copy-collector input: vmov s17,r2

# qhasm: lod9 = lou1
# asm 1: vmov >lod9=float32#19,<lou1=int32#6
# asm 2: vmov >lod9=s18,<lou1=r5
# copy-collector input: vmov s18,r5

# qhasm: hid9 = hiu1
# asm 1: vmov >hid9=float32#20,<hiu1=int32#5
# asm 2: vmov >hid9=s19,<hiu1=r4
# copy-collector input: vmov s19,r4

# qhasm: lod10 = lou2
# asm 1: vmov >lod10=float32#21,<lou2=int32#8
# asm 2: vmov >lod10=s20,<lou2=r7
# copy-collector input: vmov s20,r7

# qhasm: hid10 = hiu2
# asm 1: vmov >hid10=float32#22,<hiu2=int32#7
# asm 2: vmov >hid10=s21,<hiu2=r6
# copy-collector input: vmov s21,r6

# qhasm: lod11 = lou3
# asm 1: vmov >lod11=float32#23,<lou3=int32#10
# asm 2: vmov >lod11=s22,<lou3=r9
# copy-collector input: vmov s22,r9

# qhasm: hid11 = hiu3
# asm 1: vmov >hid11=float32#24,<hiu3=int32#9
# asm 2: vmov >hid11=s23,<hiu3=r8
# copy-collector input: vmov s23,r8

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#3,[<input_0=int32#2],#4
# asm 2: ldr >hiu0=r2,[<input_0=r1],#4
# copy-collector input: ldr r2,[r1],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#4,[<input_0=int32#2],#4
# asm 2: ldr >lou0=r3,[<input_0=r1],#4
# copy-collector input: ldr r3,[r1],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#5,[<input_0=int32#2],#4
# asm 2: ldr >hiu1=r4,[<input_0=r1],#4
# copy-collector input: ldr r4,[r1],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#6,[<input_0=int32#2],#4
# asm 2: ldr >lou1=r5,[<input_0=r1],#4
# copy-collector input: ldr r5,[r1],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#7,[<input_0=int32#2],#4
# asm 2: ldr >hiu2=r6,[<input_0=r1],#4
# copy-collector input: ldr r6,[r1],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#8,[<input_0=int32#2],#4
# asm 2: ldr >lou2=r7,[<input_0=r1],#4
# copy-collector input: ldr r7,[r1],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#9,[<input_0=int32#2],#4
# asm 2: ldr >hiu3=r8,[<input_0=r1],#4
# copy-collector input: ldr r8,[r1],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#10,[<input_0=int32#2],#4
# asm 2: ldr >lou3=r9,[<input_0=r1],#4
# copy-collector input: ldr r9,[r1],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#4,<lou0=int32#4
# asm 2: rev >lou0=r3,<lou0=r3
# copy-collector output starts
vmov d8,r3,r2
vmov d9,r5,r4
vmov d10,r7,r6
vmov d11,r9,r8
ldr r2,[r1],#4
ldr r3,[r1],#4
ldr r4,[r1],#4
ldr r5,[r1],#4
ldr r6,[r1],#4
ldr r7,[r1],#4
ldr r8,[r1],#4
ldr r9,[r1],#4
# copy-collector output ends
rev r3,r3

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#3,<hiu0=int32#3
# asm 2: rev >hiu0=r2,<hiu0=r2
rev r2,r2

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#6,<lou1=int32#6
# asm 2: rev >lou1=r5,<lou1=r5
rev r5,r5

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#5,<hiu1=int32#5
# asm 2: rev >hiu1=r4,<hiu1=r4
rev r4,r4

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#8,<lou2=int32#8
# asm 2: rev >lou2=r7,<lou2=r7
rev r7,r7

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#7,<hiu2=int32#7
# asm 2: rev >hiu2=r6,<hiu2=r6
rev r6,r6

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#10,<lou3=int32#10
# asm 2: rev >lou3=r9,<lou3=r9
rev r9,r9

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#9,<hiu3=int32#9
# asm 2: rev >hiu3=r8,<hiu3=r8
rev r8,r8

# qhasm: lod12 = lou0
# asm 1: vmov >lod12=float32#25,<lou0=int32#4
# asm 2: vmov >lod12=s24,<lou0=r3
# copy-collector input: vmov s24,r3

# qhasm: hid12 = hiu0
# asm 1: vmov >hid12=float32#26,<hiu0=int32#3
# asm 2: vmov >hid12=s25,<hiu0=r2
# copy-collector input: vmov s25,r2

# qhasm: lod13 = lou1
# asm 1: vmov >lod13=float32#27,<lou1=int32#6
# asm 2: vmov >lod13=s26,<lou1=r5
# copy-collector input: vmov s26,r5

# qhasm: hid13 = hiu1
# asm 1: vmov >hid13=float32#28,<hiu1=int32#5
# asm 2: vmov >hid13=s27,<hiu1=r4
# copy-collector input: vmov s27,r4

# qhasm: lod14 = lou2
# asm 1: vmov >lod14=float32#29,<lou2=int32#8
# asm 2: vmov >lod14=s28,<lou2=r7
# copy-collector input: vmov s28,r7

# qhasm: hid14 = hiu2
# asm 1: vmov >hid14=float32#30,<hiu2=int32#7
# asm 2: vmov >hid14=s29,<hiu2=r6
# copy-collector input: vmov s29,r6

# qhasm: lod15 = lou3
# asm 1: vmov >lod15=float32#31,<lou3=int32#10
# asm 2: vmov >lod15=s30,<lou3=r9
# copy-collector input: vmov s30,r9

# qhasm: hid15 = hiu3
# asm 1: vmov >hid15=float32#32,<hiu3=int32#9
# asm 2: vmov >hid15=s31,<hiu3=r8
# copy-collector input: vmov s31,r8

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#3,[<input_0=int32#2],#4
# asm 2: ldr >hiu0=r2,[<input_0=r1],#4
# copy-collector input: ldr r2,[r1],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#4,[<input_0=int32#2],#4
# asm 2: ldr >lou0=r3,[<input_0=r1],#4
# copy-collector input: ldr r3,[r1],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#5,[<input_0=int32#2],#4
# asm 2: ldr >hiu1=r4,[<input_0=r1],#4
# copy-collector input: ldr r4,[r1],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#6,[<input_0=int32#2],#4
# asm 2: ldr >lou1=r5,[<input_0=r1],#4
# copy-collector input: ldr r5,[r1],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#7,[<input_0=int32#2],#4
# asm 2: ldr >hiu2=r6,[<input_0=r1],#4
# copy-collector input: ldr r6,[r1],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#8,[<input_0=int32#2],#4
# asm 2: ldr >lou2=r7,[<input_0=r1],#4
# copy-collector input: ldr r7,[r1],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#9,[<input_0=int32#2],#4
# asm 2: ldr >hiu3=r8,[<input_0=r1],#4
# copy-collector input: ldr r8,[r1],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#10,[<input_0=int32#2],#4
# asm 2: ldr >lou3=r9,[<input_0=r1],#4
# copy-collector input: ldr r9,[r1],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#4,<lou0=int32#4
# asm 2: rev >lou0=r3,<lou0=r3
# copy-collector output starts
vmov d12,r3,r2
vmov d13,r5,r4
vmov d14,r7,r6
vmov d15,r9,r8
ldr r2,[r1],#4
ldr r3,[r1],#4
ldr r4,[r1],#4
ldr r5,[r1],#4
ldr r6,[r1],#4
ldr r7,[r1],#4
ldr r8,[r1],#4
ldr r9,[r1],#4
# copy-collector output ends
rev r3,r3

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#3,<hiu0=int32#3
# asm 2: rev >hiu0=r2,<hiu0=r2
rev r2,r2

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#6,<lou1=int32#6
# asm 2: rev >lou1=r5,<lou1=r5
rev r5,r5

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#5,<hiu1=int32#5
# asm 2: rev >hiu1=r4,<hiu1=r4
rev r4,r4

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#8,<lou2=int32#8
# asm 2: rev >lou2=r7,<lou2=r7
rev r7,r7

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#7,<hiu2=int32#7
# asm 2: rev >hiu2=r6,<hiu2=r6
rev r6,r6

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#10,<lou3=int32#10
# asm 2: rev >lou3=r9,<lou3=r9
rev r9,r9

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#9,<hiu3=int32#9
# asm 2: rev >hiu3=r8,<hiu3=r8
rev r8,r8

# qhasm: lom8 = lou0
# asm 1: str <lou0=int32#4,>lom8=stack32#20
# asm 2: str <lou0=r3,>lom8=[sp,#76]
# copy-collector input: str r3,[sp,#76]

# qhasm: him8 = hiu0
# asm 1: str <hiu0=int32#3,>him8=stack32#21
# asm 2: str <hiu0=r2,>him8=[sp,#80]
# copy-collector input: str r2,[sp,#80]

# qhasm: lom9 = lou1
# asm 1: str <lou1=int32#6,>lom9=stack32#22
# asm 2: str <lou1=r5,>lom9=[sp,#84]
# copy-collector input: str r5,[sp,#84]

# qhasm: him9 = hiu1
# asm 1: str <hiu1=int32#5,>him9=stack32#23
# asm 2: str <hiu1=r4,>him9=[sp,#88]
# copy-collector input: str r4,[sp,#88]

# qhasm: lom10 = lou2
# asm 1: str <lou2=int32#8,>lom10=stack32#24
# asm 2: str <lou2=r7,>lom10=[sp,#92]
# copy-collector input: str r7,[sp,#92]

# qhasm: him10 = hiu2
# asm 1: str <hiu2=int32#7,>him10=stack32#25
# asm 2: str <hiu2=r6,>him10=[sp,#96]
# copy-collector input: str r6,[sp,#96]

# qhasm: lom11 = lou3
# asm 1: str <lou3=int32#10,>lom11=stack32#26
# asm 2: str <lou3=r9,>lom11=[sp,#100]
# copy-collector input: str r9,[sp,#100]

# qhasm: him11 = hiu3
# asm 1: str <hiu3=int32#9,>him11=stack32#27
# asm 2: str <hiu3=r8,>him11=[sp,#104]
# copy-collector input: str r8,[sp,#104]

# qhasm: hiu0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu0=int32#3,[<input_0=int32#2],#4
# asm 2: ldr >hiu0=r2,[<input_0=r1],#4
# copy-collector input: ldr r2,[r1],#4

# qhasm: lou0 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou0=int32#4,[<input_0=int32#2],#4
# asm 2: ldr >lou0=r3,[<input_0=r1],#4
# copy-collector input: ldr r3,[r1],#4

# qhasm: hiu1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu1=int32#5,[<input_0=int32#2],#4
# asm 2: ldr >hiu1=r4,[<input_0=r1],#4
# copy-collector input: ldr r4,[r1],#4

# qhasm: lou1 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou1=int32#6,[<input_0=int32#2],#4
# asm 2: ldr >lou1=r5,[<input_0=r1],#4
# copy-collector input: ldr r5,[r1],#4

# qhasm: hiu2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu2=int32#7,[<input_0=int32#2],#4
# asm 2: ldr >hiu2=r6,[<input_0=r1],#4
# copy-collector input: ldr r6,[r1],#4

# qhasm: lou2 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou2=int32#8,[<input_0=int32#2],#4
# asm 2: ldr >lou2=r7,[<input_0=r1],#4
# copy-collector input: ldr r7,[r1],#4

# qhasm: hiu3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >hiu3=int32#9,[<input_0=int32#2],#4
# asm 2: ldr >hiu3=r8,[<input_0=r1],#4
# copy-collector input: ldr r8,[r1],#4

# qhasm: lou3 = mem32[input_0]; input_0 += 4
# asm 1: ldr >lou3=int32#10,[<input_0=int32#2],#4
# asm 2: ldr >lou3=r9,[<input_0=r1],#4
# copy-collector input: ldr r9,[r1],#4

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#4,<lou0=int32#4
# asm 2: rev >lou0=r3,<lou0=r3
# copy-collector output starts
strd r3,r2,[sp,#76]
strd r5,r4,[sp,#84]
strd r7,r6,[sp,#92]
strd r9,r8,[sp,#100]
ldr r2,[r1],#4
ldr r3,[r1],#4
ldr r4,[r1],#4
ldr r5,[r1],#4
ldr r6,[r1],#4
ldr r7,[r1],#4
ldr r8,[r1],#4
ldr r9,[r1],#4
# copy-collector output ends
rev r3,r3

# qhasm: hiu0 = hiu0[3]hiu0[2]hiu0[1]hiu0[0]
# asm 1: rev >hiu0=int32#3,<hiu0=int32#3
# asm 2: rev >hiu0=r2,<hiu0=r2
rev r2,r2

# qhasm: lou1 = lou1[3]lou1[2]lou1[1]lou1[0]
# asm 1: rev >lou1=int32#6,<lou1=int32#6
# asm 2: rev >lou1=r5,<lou1=r5
rev r5,r5

# qhasm: hiu1 = hiu1[3]hiu1[2]hiu1[1]hiu1[0]
# asm 1: rev >hiu1=int32#5,<hiu1=int32#5
# asm 2: rev >hiu1=r4,<hiu1=r4
rev r4,r4

# qhasm: lou2 = lou2[3]lou2[2]lou2[1]lou2[0]
# asm 1: rev >lou2=int32#8,<lou2=int32#8
# asm 2: rev >lou2=r7,<lou2=r7
rev r7,r7

# qhasm: hiu2 = hiu2[3]hiu2[2]hiu2[1]hiu2[0]
# asm 1: rev >hiu2=int32#7,<hiu2=int32#7
# asm 2: rev >hiu2=r6,<hiu2=r6
rev r6,r6

# qhasm: lou3 = lou3[3]lou3[2]lou3[1]lou3[0]
# asm 1: rev >lou3=int32#10,<lou3=int32#10
# asm 2: rev >lou3=r9,<lou3=r9
rev r9,r9

# qhasm: hiu3 = hiu3[3]hiu3[2]hiu3[1]hiu3[0]
# asm 1: rev >hiu3=int32#9,<hiu3=int32#9
# asm 2: rev >hiu3=r8,<hiu3=r8
rev r8,r8

# qhasm: lom12 = lou0
# asm 1: str <lou0=int32#4,>lom12=stack32#28
# asm 2: str <lou0=r3,>lom12=[sp,#108]
# copy-collector input: str r3,[sp,#108]

# qhasm: him12 = hiu0
# asm 1: str <hiu0=int32#3,>him12=stack32#29
# asm 2: str <hiu0=r2,>him12=[sp,#112]
# copy-collector input: str r2,[sp,#112]

# qhasm: lom13 = lou1
# asm 1: str <lou1=int32#6,>lom13=stack32#30
# asm 2: str <lou1=r5,>lom13=[sp,#116]
# copy-collector input: str r5,[sp,#116]

# qhasm: him13 = hiu1
# asm 1: str <hiu1=int32#5,>him13=stack32#31
# asm 2: str <hiu1=r4,>him13=[sp,#120]
# copy-collector input: str r4,[sp,#120]

# qhasm: lom14 = lou2
# asm 1: str <lou2=int32#8,>lom14=stack32#32
# asm 2: str <lou2=r7,>lom14=[sp,#124]
# copy-collector input: str r7,[sp,#124]

# qhasm: him14 = hiu2
# asm 1: str <hiu2=int32#7,>him14=stack32#33
# asm 2: str <hiu2=r6,>him14=[sp,#128]
# copy-collector input: str r6,[sp,#128]

# qhasm: lom15 = lou3
# asm 1: str <lou3=int32#10,>lom15=stack32#34
# asm 2: str <lou3=r9,>lom15=[sp,#132]
# copy-collector input: str r9,[sp,#132]

# qhasm: him15 = hiu3
# asm 1: str <hiu3=int32#9,>him15=stack32#35
# asm 2: str <hiu3=r8,>him15=[sp,#136]
# copy-collector input: str r8,[sp,#136]

# qhasm: o1 = input_0
# asm 1: str <input_0=int32#2,>o1=stack32#2
# asm 2: str <input_0=r1,>o1=[sp,#4]
# copy-collector input: str r1,[sp,#4]

# qhasm: input_0 = 80 simple
# asm 1: mov >input_0=int32#2,80
# asm 2: mov >input_0=r1,80
# copy-collector output starts
strd r3,r2,[sp,#108]
strd r5,r4,[sp,#116]
strd r7,r6,[sp,#124]
strd r9,r8,[sp,#132]
str r1,[sp,#4]
# copy-collector output ends
mov r1,80

# qhasm: o3 = input_0
# asm 1: str <input_0=int32#2,>o3=stack32#36
# asm 2: str <input_0=r1,>o3=[sp,#140]
# copy-collector input: str r1,[sp,#140]

# qhasm: innerloop:
# copy-collector output starts
str r1,[sp,#140]
# copy-collector output ends
._innerloop:

# qhasm: assign 0 to lod0

# qhasm: assign 1 to hid0

# qhasm: assign 2 to lod1

# qhasm: assign 3 to hid1

# qhasm: assign 4 to lod2

# qhasm: assign 5 to hid2

# qhasm: assign 6 to lod3

# qhasm: assign 7 to hid3

# qhasm: assign 8 to lod4

# qhasm: assign 9 to hid4

# qhasm: assign 10 to lod5

# qhasm: assign 11 to hid5

# qhasm: assign 12 to lod6

# qhasm: assign 13 to hid6

# qhasm: assign 14 to lod7

# qhasm: assign 15 to hid7

# qhasm: lou0 = lod3
# asm 1: vmov >lou0=int32#2,<lod3=float32#7
# asm 2: vmov >lou0=r1,<lod3=s6
# copy-collector input: vmov r1,s6

# qhasm: hiu0 = hid3
# asm 1: vmov >hiu0=int32#3,<hid3=float32#8
# asm 2: vmov >hiu0=r2,<hid3=s7
# copy-collector input: vmov r2,s7

# qhasm: lou1 = lod4
# asm 1: vmov >lou1=int32#4,<lod4=float32#9
# asm 2: vmov >lou1=r3,<lod4=s8
# copy-collector input: vmov r3,s8

# qhasm: hiu1 = hid4
# asm 1: vmov >hiu1=int32#5,<hid4=float32#10
# asm 2: vmov >hiu1=r4,<hid4=s9
# copy-collector input: vmov r4,s9

# qhasm: lou2 = lod5
# asm 1: vmov >lou2=int32#6,<lod5=float32#11
# asm 2: vmov >lou2=r5,<lod5=s10
# copy-collector input: vmov r5,s10

# qhasm: hiu2 = hid5
# asm 1: vmov >hiu2=int32#7,<hid5=float32#12
# asm 2: vmov >hiu2=r6,<hid5=s11
# copy-collector input: vmov r6,s11

# qhasm: lou3 = lod6
# asm 1: vmov >lou3=int32#8,<lod6=float32#13
# asm 2: vmov >lou3=r7,<lod6=s12
# copy-collector input: vmov r7,s12

# qhasm: hiu3 = hid6
# asm 1: vmov >hiu3=int32#9,<hid6=float32#14
# asm 2: vmov >hiu3=r8,<hid6=s13
# copy-collector input: vmov r8,s13

# qhasm: lou4 = lod7
# asm 1: vmov >lou4=int32#10,<lod7=float32#15
# asm 2: vmov >lou4=r9,<lod7=s14
# copy-collector input: vmov r9,s14

# qhasm: hiu4 = hid7
# asm 1: vmov >hiu4=int32#11,<hid7=float32#16
# asm 2: vmov >hiu4=r10,<hid7=s15
# copy-collector input: vmov r10,s15

# qhasm: lou5 = lod8
# asm 1: vmov >lou5=int32#12,<lod8=float32#17
# asm 2: vmov >lou5=r11,<lod8=s16
# copy-collector input: vmov r11,s16

# qhasm: hiu5 = hid8
# asm 1: vmov >hiu5=int32#13,<hid8=float32#18
# asm 2: vmov >hiu5=r12,<hid8=s17
# copy-collector input: vmov r12,s17

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lou5=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lou5=r11
# copy-collector output starts
vmov r1,r2,d3
vmov r3,r4,d4
vmov r5,r6,d5
vmov r7,r8,d6
vmov r9,r10,d7
vmov r11,r12,d8
# copy-collector output ends
adds r9,r9,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#11,<hiu4=int32#11,<hiu5=int32#13
# asm 2: adc >hiu4=r10,<hiu4=r10,<hiu5=r12
adc r10,r10,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu1 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu1=int32#5,LSR #9
# asm 2: mov >lotmp=r11,<hiu1=r4,LSR #9
mov r11,r4,LSR #9

# qhasm: lotmp ^= (lou1 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#4,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r3,LSL #23
eor r11,r11,r3,LSL #23

# qhasm: lotmp ^= (lou1 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#4,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r3,LSR #18
eor r11,r11,r3,LSR #18

# qhasm: lotmp ^= (hiu1 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#5,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r4,LSL #14
eor r11,r11,r4,LSL #14

# qhasm: lotmp ^= (lou1 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#4,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r3,LSR #14
eor r11,r11,r3,LSR #14

# qhasm: lotmp ^= (hiu1 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#5,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r4,LSL #18
eor r11,r11,r4,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lotmp=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lotmp=r11
adds r9,r9,r11

# qhasm: hitmp = (lou1 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou1=int32#4,LSR #9
# asm 2: mov >hitmp=r11,<lou1=r3,LSR #9
mov r11,r3,LSR #9

# qhasm: hitmp ^= (hiu1 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu1=int32#5,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu1=r4,LSL #23
eor r11,r11,r4,LSL #23

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
# asm 1: vmov >lod3=float32#7,<lou0=int32#2
# asm 2: vmov >lod3=s6,<lou0=r1
# copy-collector input: vmov s6,r1

# qhasm: hid3 = hiu0
# asm 1: vmov >hid3=float32#8,<hiu0=int32#3
# asm 2: vmov >hid3=s7,<hiu0=r2
# copy-collector input: vmov s7,r2

# qhasm: lou1 = lod0
# asm 1: vmov >lou1=int32#6,<lod0=float32#1
# asm 2: vmov >lou1=r5,<lod0=s0
# copy-collector input: vmov r5,s0

# qhasm: hiu1 = hid0
# asm 1: vmov >hiu1=int32#7,<hid0=float32#2
# asm 2: vmov >hiu1=r6,<hid0=s1
# copy-collector input: vmov r6,s1

# qhasm: lou2 = lod1
# asm 1: vmov >lou2=int32#8,<lod1=float32#3
# asm 2: vmov >lou2=r7,<lod1=s2
# copy-collector input: vmov r7,s2

# qhasm: hiu2 = hid1
# asm 1: vmov >hiu2=int32#9,<hid1=float32#4
# asm 2: vmov >hiu2=r8,<hid1=s3
# copy-collector input: vmov r8,s3

# qhasm: lou3 = lod2
# asm 1: vmov >lou3=int32#10,<lod2=float32#5
# asm 2: vmov >lou3=r9,<lod2=s4
# copy-collector input: vmov r9,s4

# qhasm: hiu3 = hid2
# asm 1: vmov >hiu3=int32#11,<hid2=float32#6
# asm 2: vmov >hiu3=r10,<hid2=s5
# copy-collector input: vmov r10,s5

# qhasm: lotmp = (hiu1 unsigned>> 7)
# asm 1: mov >lotmp=int32#12,<hiu1=int32#7,LSR #7
# asm 2: mov >lotmp=r11,<hiu1=r6,LSR #7
# copy-collector output starts
vmov d3,r1,r2
vmov r5,r6,d0
vmov r7,r8,d1
vmov r9,r10,d2
# copy-collector output ends
mov r11,r6,LSR #7

# qhasm: lotmp ^= (lou1 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: lotmp ^= (hiu1 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r6,LSR #2
eor r11,r11,r6,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#4,<lou4=int32#4,<lotmp=int32#12
# asm 2: adds >lou4=r3,<lou4=r3,<lotmp=r11
adds r3,r3,r11

# qhasm: hitmp = (lou1 unsigned>> 7)
# asm 1: mov >hitmp=int32#12,<lou1=int32#6,LSR #7
# asm 2: mov >hitmp=r11,<lou1=r5,LSR #7
mov r11,r5,LSR #7

# qhasm: hitmp ^= (hiu1 << 25)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu1=int32#7,LSL #25
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu1=r6,LSL #25
eor r11,r11,r6,LSL #25

# qhasm: hitmp ^= (lou1 unsigned>> 2)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou1=int32#6,LSR #2
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou1=r5,LSR #2
eor r11,r11,r5,LSR #2

# qhasm: hitmp ^= (hiu1 << 30)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu1=int32#7,LSL #30
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu1=r6,LSL #30
eor r11,r11,r6,LSL #30

# qhasm: hitmp ^= (hiu1 unsigned>> 28)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu1=int32#7,LSR #28
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu1=r6,LSR #28
eor r11,r11,r6,LSR #28

# qhasm: hitmp ^= (lou1 << 4)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou1=int32#6,LSL #4
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou1=r5,LSL #4
eor r11,r11,r5,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#5,<hiu4=int32#5,<hitmp=int32#12
# asm 2: adc >hiu4=r4,<hiu4=r4,<hitmp=r11
adc r4,r4,r11

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
adds r3,r3,r5

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
# asm 1: vmov >lod7=float32#15,<lou4=int32#4
# asm 2: vmov >lod7=s14,<lou4=r3
# copy-collector input: vmov s14,r3

# qhasm: hid7 = hiu4
# asm 1: vmov >hid7=float32#16,<hiu4=int32#5
# asm 2: vmov >hid7=s15,<hiu4=r4
# copy-collector input: vmov s15,r4

# qhasm: lou1 = lod4
# asm 1: vmov >lou1=int32#4,<lod4=float32#9
# asm 2: vmov >lou1=r3,<lod4=s8
# copy-collector input: vmov r3,s8

# qhasm: hiu1 = hid4
# asm 1: vmov >hiu1=int32#5,<hid4=float32#10
# asm 2: vmov >hiu1=r4,<hid4=s9
# copy-collector input: vmov r4,s9

# qhasm: lou2 = lod5
# asm 1: vmov >lou2=int32#6,<lod5=float32#11
# asm 2: vmov >lou2=r5,<lod5=s10
# copy-collector input: vmov r5,s10

# qhasm: hiu2 = hid5
# asm 1: vmov >hiu2=int32#7,<hid5=float32#12
# asm 2: vmov >hiu2=r6,<hid5=s11
# copy-collector input: vmov r6,s11

# qhasm: lou4 = lod6
# asm 1: vmov >lou4=int32#8,<lod6=float32#13
# asm 2: vmov >lou4=r7,<lod6=s12
# copy-collector input: vmov r7,s12

# qhasm: hiu4 = hid6
# asm 1: vmov >hiu4=int32#9,<hid6=float32#14
# asm 2: vmov >hiu4=r8,<hid6=s13
# copy-collector input: vmov r8,s13

# qhasm: lou5 = lod9
# asm 1: vmov >lou5=int32#12,<lod9=float32#19
# asm 2: vmov >lou5=r11,<lod9=s18
# copy-collector input: vmov r11,s18

# qhasm: hiu5 = hid9
# asm 1: vmov >hiu5=int32#13,<hid9=float32#20
# asm 2: vmov >hiu5=r12,<hid9=s19
# copy-collector input: vmov r12,s19

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
# copy-collector output starts
vmov d7,r3,r4
vmov r3,r4,d4
vmov r5,r6,d5
vmov r7,r8,d6
vmov r11,r12,d9
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu0 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu0=int32#3,LSR #9
# asm 2: mov >lotmp=r11,<hiu0=r2,LSR #9
mov r11,r2,LSR #9

# qhasm: lotmp ^= (lou0 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#2,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r1,LSL #23
eor r11,r11,r1,LSL #23

# qhasm: lotmp ^= (lou0 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#2,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r1,LSR #18
eor r11,r11,r1,LSR #18

# qhasm: lotmp ^= (hiu0 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#3,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r2,LSL #14
eor r11,r11,r2,LSL #14

# qhasm: lotmp ^= (lou0 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#2,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r1,LSR #14
eor r11,r11,r1,LSR #14

# qhasm: lotmp ^= (hiu0 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#3,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r2,LSL #18
eor r11,r11,r2,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
adds r7,r7,r11

# qhasm: hitmp = (lou0 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou0=int32#2,LSR #9
# asm 2: mov >hitmp=r11,<lou0=r1,LSR #9
mov r11,r1,LSR #9

# qhasm: hitmp ^= (hiu0 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#3,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r2,LSL #23
eor r11,r11,r2,LSL #23

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
adds r1,r7,r1

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
# asm 1: vmov >lod2=float32#5,<lou3=int32#4
# asm 2: vmov >lod2=s4,<lou3=r3
# copy-collector input: vmov s4,r3

# qhasm: hid2 = hiu3
# asm 1: vmov >hid2=float32#6,<hiu3=int32#5
# asm 2: vmov >hid2=s5,<hiu3=r4
# copy-collector input: vmov s5,r4

# qhasm: lou0 = lod7
# asm 1: vmov >lou0=int32#6,<lod7=float32#15
# asm 2: vmov >lou0=r5,<lod7=s14
# copy-collector input: vmov r5,s14

# qhasm: hiu0 = hid7
# asm 1: vmov >hiu0=int32#7,<hid7=float32#16
# asm 2: vmov >hiu0=r6,<hid7=s15
# copy-collector input: vmov r6,s15

# qhasm: lou1 = lod0
# asm 1: vmov >lou1=int32#8,<lod0=float32#1
# asm 2: vmov >lou1=r7,<lod0=s0
# copy-collector input: vmov r7,s0

# qhasm: hiu1 = hid0
# asm 1: vmov >hiu1=int32#9,<hid0=float32#2
# asm 2: vmov >hiu1=r8,<hid0=s1
# copy-collector input: vmov r8,s1

# qhasm: lou2 = lod1
# asm 1: vmov >lou2=int32#10,<lod1=float32#3
# asm 2: vmov >lou2=r9,<lod1=s2
# copy-collector input: vmov r9,s2

# qhasm: hiu2 = hid1
# asm 1: vmov >hiu2=int32#11,<hid1=float32#4
# asm 2: vmov >hiu2=r10,<hid1=s3
# copy-collector input: vmov r10,s3

# qhasm: lotmp = (hiu0 unsigned>> 7)
# asm 1: mov >lotmp=int32#12,<hiu0=int32#7,LSR #7
# asm 2: mov >lotmp=r11,<hiu0=r6,LSR #7
# copy-collector output starts
vmov d2,r3,r4
vmov r5,r6,d7
vmov r7,r8,d0
vmov r9,r10,d1
# copy-collector output ends
mov r11,r6,LSR #7

# qhasm: lotmp ^= (lou0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSR #2
eor r11,r11,r6,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hitmp = (lou0 unsigned>> 7)
# asm 1: mov >hitmp=int32#12,<lou0=int32#6,LSR #7
# asm 2: mov >hitmp=r11,<lou0=r5,LSR #7
mov r11,r5,LSR #7

# qhasm: hitmp ^= (hiu0 << 25)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #25
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #25
eor r11,r11,r6,LSL #25

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSR #2
eor r11,r11,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #30
eor r11,r11,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSR #28
eor r11,r11,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSL #4
eor r11,r11,r5,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#12
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r11
adc r2,r2,r11

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
adds r1,r1,r5

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
# asm 1: vmov >lod6=float32#13,<lou4=int32#2
# asm 2: vmov >lod6=s12,<lou4=r1
# copy-collector input: vmov s12,r1

# qhasm: hid6 = hiu4
# asm 1: vmov >hid6=float32#14,<hiu4=int32#3
# asm 2: vmov >hid6=s13,<hiu4=r2
# copy-collector input: vmov s13,r2

# qhasm: assign 0 to lod0

# qhasm: assign 1 to hid0

# qhasm: assign 2 to lod1

# qhasm: assign 3 to hid1

# qhasm: assign 4 to lod2

# qhasm: assign 5 to hid2

# qhasm: assign 6 to lod3

# qhasm: assign 7 to hid3

# qhasm: assign 8 to lod4

# qhasm: assign 9 to hid4

# qhasm: assign 10 to lod5

# qhasm: assign 11 to hid5

# qhasm: assign 12 to lod6

# qhasm: assign 13 to hid6

# qhasm: assign 14 to lod7

# qhasm: assign 15 to hid7

# qhasm: lou0 = lod3
# asm 1: vmov >lou0=int32#2,<lod3=float32#7
# asm 2: vmov >lou0=r1,<lod3=s6
# copy-collector input: vmov r1,s6

# qhasm: hiu0 = hid3
# asm 1: vmov >hiu0=int32#3,<hid3=float32#8
# asm 2: vmov >hiu0=r2,<hid3=s7
# copy-collector input: vmov r2,s7

# qhasm: lou1 = lod4
# asm 1: vmov >lou1=int32#6,<lod4=float32#9
# asm 2: vmov >lou1=r5,<lod4=s8
# copy-collector input: vmov r5,s8

# qhasm: hiu1 = hid4
# asm 1: vmov >hiu1=int32#7,<hid4=float32#10
# asm 2: vmov >hiu1=r6,<hid4=s9
# copy-collector input: vmov r6,s9

# qhasm: lou4 = lod5
# asm 1: vmov >lou4=int32#8,<lod5=float32#11
# asm 2: vmov >lou4=r7,<lod5=s10
# copy-collector input: vmov r7,s10

# qhasm: hiu4 = hid5
# asm 1: vmov >hiu4=int32#9,<hid5=float32#12
# asm 2: vmov >hiu4=r8,<hid5=s11
# copy-collector input: vmov r8,s11

# qhasm: lou5 = lod10
# asm 1: vmov >lou5=int32#12,<lod10=float32#21
# asm 2: vmov >lou5=r11,<lod10=s20
# copy-collector input: vmov r11,s20

# qhasm: hiu5 = hid10
# asm 1: vmov >hiu5=int32#13,<hid10=float32#22
# asm 2: vmov >hiu5=r12,<hid10=s21
# copy-collector input: vmov r12,s21

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
# copy-collector output starts
vmov d6,r1,r2
vmov r1,r2,d3
vmov r5,r6,d4
vmov r7,r8,d5
vmov r11,r12,d10
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu3 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu3=int32#5,LSR #9
# asm 2: mov >lotmp=r11,<hiu3=r4,LSR #9
mov r11,r4,LSR #9

# qhasm: lotmp ^= (lou3 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSL #23
eor r11,r11,r3,LSL #23

# qhasm: lotmp ^= (lou3 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSR #18
eor r11,r11,r3,LSR #18

# qhasm: lotmp ^= (hiu3 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#5,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r4,LSL #14
eor r11,r11,r4,LSL #14

# qhasm: lotmp ^= (lou3 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSR #14
eor r11,r11,r3,LSR #14

# qhasm: lotmp ^= (hiu3 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#5,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r4,LSL #18
eor r11,r11,r4,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
adds r7,r7,r11

# qhasm: hitmp = (lou3 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou3=int32#4,LSR #9
# asm 2: mov >hitmp=r11,<lou3=r3,LSR #9
mov r11,r3,LSR #9

# qhasm: hitmp ^= (hiu3 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSL #23
eor r11,r11,r4,LSL #23

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
adds r1,r7,r1

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
# asm 1: vmov >lod1=float32#3,<lou2=int32#4
# asm 2: vmov >lod1=s2,<lou2=r3
# copy-collector input: vmov s2,r3

# qhasm: hid1 = hiu2
# asm 1: vmov >hid1=float32#4,<hiu2=int32#5
# asm 2: vmov >hid1=s3,<hiu2=r4
# copy-collector input: vmov s3,r4

# qhasm: lou0 = lod6
# asm 1: vmov >lou0=int32#6,<lod6=float32#13
# asm 2: vmov >lou0=r5,<lod6=s12
# copy-collector input: vmov r5,s12

# qhasm: hiu0 = hid6
# asm 1: vmov >hiu0=int32#7,<hid6=float32#14
# asm 2: vmov >hiu0=r6,<hid6=s13
# copy-collector input: vmov r6,s13

# qhasm: lou1 = lod7
# asm 1: vmov >lou1=int32#8,<lod7=float32#15
# asm 2: vmov >lou1=r7,<lod7=s14
# copy-collector input: vmov r7,s14

# qhasm: hiu1 = hid7
# asm 1: vmov >hiu1=int32#9,<hid7=float32#16
# asm 2: vmov >hiu1=r8,<hid7=s15
# copy-collector input: vmov r8,s15

# qhasm: lou3 = lod0
# asm 1: vmov >lou3=int32#10,<lod0=float32#1
# asm 2: vmov >lou3=r9,<lod0=s0
# copy-collector input: vmov r9,s0

# qhasm: hiu3 = hid0
# asm 1: vmov >hiu3=int32#11,<hid0=float32#2
# asm 2: vmov >hiu3=r10,<hid0=s1
# copy-collector input: vmov r10,s1

# qhasm: lotmp = (hiu0 unsigned>> 7)
# asm 1: mov >lotmp=int32#12,<hiu0=int32#7,LSR #7
# asm 2: mov >lotmp=r11,<hiu0=r6,LSR #7
# copy-collector output starts
vmov d1,r3,r4
vmov r5,r6,d6
vmov r7,r8,d7
vmov r9,r10,d0
# copy-collector output ends
mov r11,r6,LSR #7

# qhasm: lotmp ^= (lou0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSR #2
eor r11,r11,r6,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hitmp = (lou0 unsigned>> 7)
# asm 1: mov >hitmp=int32#12,<lou0=int32#6,LSR #7
# asm 2: mov >hitmp=r11,<lou0=r5,LSR #7
mov r11,r5,LSR #7

# qhasm: hitmp ^= (hiu0 << 25)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #25
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #25
eor r11,r11,r6,LSL #25

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSR #2
eor r11,r11,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #30
eor r11,r11,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSR #28
eor r11,r11,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSL #4
eor r11,r11,r5,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#12
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r11
adc r2,r2,r11

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
adds r1,r1,r5

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
# asm 1: vmov >lod5=float32#11,<lou4=int32#2
# asm 2: vmov >lod5=s10,<lou4=r1
# copy-collector input: vmov s10,r1

# qhasm: hid5 = hiu4
# asm 1: vmov >hid5=float32#12,<hiu4=int32#3
# asm 2: vmov >hid5=s11,<hiu4=r2
# copy-collector input: vmov s11,r2

# qhasm: lou0 = lod2
# asm 1: vmov >lou0=int32#2,<lod2=float32#5
# asm 2: vmov >lou0=r1,<lod2=s4
# copy-collector input: vmov r1,s4

# qhasm: hiu0 = hid2
# asm 1: vmov >hiu0=int32#3,<hid2=float32#6
# asm 2: vmov >hiu0=r2,<hid2=s5
# copy-collector input: vmov r2,s5

# qhasm: lou1 = lod3
# asm 1: vmov >lou1=int32#6,<lod3=float32#7
# asm 2: vmov >lou1=r5,<lod3=s6
# copy-collector input: vmov r5,s6

# qhasm: hiu1 = hid3
# asm 1: vmov >hiu1=int32#7,<hid3=float32#8
# asm 2: vmov >hiu1=r6,<hid3=s7
# copy-collector input: vmov r6,s7

# qhasm: lou4 = lod4
# asm 1: vmov >lou4=int32#8,<lod4=float32#9
# asm 2: vmov >lou4=r7,<lod4=s8
# copy-collector input: vmov r7,s8

# qhasm: hiu4 = hid4
# asm 1: vmov >hiu4=int32#9,<hid4=float32#10
# asm 2: vmov >hiu4=r8,<hid4=s9
# copy-collector input: vmov r8,s9

# qhasm: lou5 = lod11
# asm 1: vmov >lou5=int32#12,<lod11=float32#23
# asm 2: vmov >lou5=r11,<lod11=s22
# copy-collector input: vmov r11,s22

# qhasm: hiu5 = hid11
# asm 1: vmov >hiu5=int32#13,<hid11=float32#24
# asm 2: vmov >hiu5=r12,<hid11=s23
# copy-collector input: vmov r12,s23

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
# copy-collector output starts
vmov d5,r1,r2
vmov r1,r2,d2
vmov r5,r6,d3
vmov r7,r8,d4
vmov r11,r12,d11
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu2 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu2=int32#5,LSR #9
# asm 2: mov >lotmp=r11,<hiu2=r4,LSR #9
mov r11,r4,LSR #9

# qhasm: lotmp ^= (lou2 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSL #23
eor r11,r11,r3,LSL #23

# qhasm: lotmp ^= (lou2 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSR #18
eor r11,r11,r3,LSR #18

# qhasm: lotmp ^= (hiu2 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#5,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r4,LSL #14
eor r11,r11,r4,LSL #14

# qhasm: lotmp ^= (lou2 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSR #14
eor r11,r11,r3,LSR #14

# qhasm: lotmp ^= (hiu2 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#5,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r4,LSL #18
eor r11,r11,r4,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
adds r7,r7,r11

# qhasm: hitmp = (lou2 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou2=int32#4,LSR #9
# asm 2: mov >hitmp=r11,<lou2=r3,LSR #9
mov r11,r3,LSR #9

# qhasm: hitmp ^= (hiu2 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSL #23
eor r11,r11,r4,LSL #23

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
adds r1,r7,r1

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
# asm 1: vmov >lod0=float32#1,<lou3=int32#4
# asm 2: vmov >lod0=s0,<lou3=r3
# copy-collector input: vmov s0,r3

# qhasm: hid0 = hiu3
# asm 1: vmov >hid0=float32#2,<hiu3=int32#5
# asm 2: vmov >hid0=s1,<hiu3=r4
# copy-collector input: vmov s1,r4

# qhasm: lou0 = lod5
# asm 1: vmov >lou0=int32#6,<lod5=float32#11
# asm 2: vmov >lou0=r5,<lod5=s10
# copy-collector input: vmov r5,s10

# qhasm: hiu0 = hid5
# asm 1: vmov >hiu0=int32#7,<hid5=float32#12
# asm 2: vmov >hiu0=r6,<hid5=s11
# copy-collector input: vmov r6,s11

# qhasm: lou1 = lod6
# asm 1: vmov >lou1=int32#8,<lod6=float32#13
# asm 2: vmov >lou1=r7,<lod6=s12
# copy-collector input: vmov r7,s12

# qhasm: hiu1 = hid6
# asm 1: vmov >hiu1=int32#9,<hid6=float32#14
# asm 2: vmov >hiu1=r8,<hid6=s13
# copy-collector input: vmov r8,s13

# qhasm: lou2 = lod7
# asm 1: vmov >lou2=int32#10,<lod7=float32#15
# asm 2: vmov >lou2=r9,<lod7=s14
# copy-collector input: vmov r9,s14

# qhasm: hiu2 = hid7
# asm 1: vmov >hiu2=int32#11,<hid7=float32#16
# asm 2: vmov >hiu2=r10,<hid7=s15
# copy-collector input: vmov r10,s15

# qhasm: lotmp = (hiu0 unsigned>> 7)
# asm 1: mov >lotmp=int32#12,<hiu0=int32#7,LSR #7
# asm 2: mov >lotmp=r11,<hiu0=r6,LSR #7
# copy-collector output starts
vmov d0,r3,r4
vmov r5,r6,d5
vmov r7,r8,d6
vmov r9,r10,d7
# copy-collector output ends
mov r11,r6,LSR #7

# qhasm: lotmp ^= (lou0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSR #2
eor r11,r11,r6,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hitmp = (lou0 unsigned>> 7)
# asm 1: mov >hitmp=int32#12,<lou0=int32#6,LSR #7
# asm 2: mov >hitmp=r11,<lou0=r5,LSR #7
mov r11,r5,LSR #7

# qhasm: hitmp ^= (hiu0 << 25)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #25
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #25
eor r11,r11,r6,LSL #25

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSR #2
eor r11,r11,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #30
eor r11,r11,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSR #28
eor r11,r11,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSL #4
eor r11,r11,r5,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#12
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r11
adc r2,r2,r11

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
adds r1,r1,r5

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
# asm 1: vmov >lod4=float32#9,<lou4=int32#2
# asm 2: vmov >lod4=s8,<lou4=r1
# copy-collector input: vmov s8,r1

# qhasm: hid4 = hiu4
# asm 1: vmov >hid4=float32#10,<hiu4=int32#3
# asm 2: vmov >hid4=s9,<hiu4=r2
# copy-collector input: vmov s9,r2

# qhasm: assign 0 to lod0

# qhasm: assign 1 to hid0

# qhasm: assign 2 to lod1

# qhasm: assign 3 to hid1

# qhasm: assign 4 to lod2

# qhasm: assign 5 to hid2

# qhasm: assign 6 to lod3

# qhasm: assign 7 to hid3

# qhasm: assign 8 to lod4

# qhasm: assign 9 to hid4

# qhasm: assign 10 to lod5

# qhasm: assign 11 to hid5

# qhasm: assign 12 to lod6

# qhasm: assign 13 to hid6

# qhasm: assign 14 to lod7

# qhasm: assign 15 to hid7

# qhasm: lou0 = lod1
# asm 1: vmov >lou0=int32#2,<lod1=float32#3
# asm 2: vmov >lou0=r1,<lod1=s2
# copy-collector input: vmov r1,s2

# qhasm: hiu0 = hid1
# asm 1: vmov >hiu0=int32#3,<hid1=float32#4
# asm 2: vmov >hiu0=r2,<hid1=s3
# copy-collector input: vmov r2,s3

# qhasm: lou1 = lod2
# asm 1: vmov >lou1=int32#6,<lod2=float32#5
# asm 2: vmov >lou1=r5,<lod2=s4
# copy-collector input: vmov r5,s4

# qhasm: hiu1 = hid2
# asm 1: vmov >hiu1=int32#7,<hid2=float32#6
# asm 2: vmov >hiu1=r6,<hid2=s5
# copy-collector input: vmov r6,s5

# qhasm: lou4 = lod3
# asm 1: vmov >lou4=int32#8,<lod3=float32#7
# asm 2: vmov >lou4=r7,<lod3=s6
# copy-collector input: vmov r7,s6

# qhasm: hiu4 = hid3
# asm 1: vmov >hiu4=int32#9,<hid3=float32#8
# asm 2: vmov >hiu4=r8,<hid3=s7
# copy-collector input: vmov r8,s7

# qhasm: lou5 = lod12
# asm 1: vmov >lou5=int32#12,<lod12=float32#25
# asm 2: vmov >lou5=r11,<lod12=s24
# copy-collector input: vmov r11,s24

# qhasm: hiu5 = hid12
# asm 1: vmov >hiu5=int32#13,<hid12=float32#26
# asm 2: vmov >hiu5=r12,<hid12=s25
# copy-collector input: vmov r12,s25

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
# copy-collector output starts
vmov d4,r1,r2
vmov r1,r2,d1
vmov r5,r6,d2
vmov r7,r8,d3
vmov r11,r12,d12
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu3 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu3=int32#5,LSR #9
# asm 2: mov >lotmp=r11,<hiu3=r4,LSR #9
mov r11,r4,LSR #9

# qhasm: lotmp ^= (lou3 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSL #23
eor r11,r11,r3,LSL #23

# qhasm: lotmp ^= (lou3 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSR #18
eor r11,r11,r3,LSR #18

# qhasm: lotmp ^= (hiu3 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#5,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r4,LSL #14
eor r11,r11,r4,LSL #14

# qhasm: lotmp ^= (lou3 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSR #14
eor r11,r11,r3,LSR #14

# qhasm: lotmp ^= (hiu3 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#5,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r4,LSL #18
eor r11,r11,r4,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
adds r7,r7,r11

# qhasm: hitmp = (lou3 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou3=int32#4,LSR #9
# asm 2: mov >hitmp=r11,<lou3=r3,LSR #9
mov r11,r3,LSR #9

# qhasm: hitmp ^= (hiu3 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSL #23
eor r11,r11,r4,LSL #23

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
adds r1,r7,r1

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
# asm 1: vmov >lod7=float32#15,<lou2=int32#4
# asm 2: vmov >lod7=s14,<lou2=r3
# copy-collector input: vmov s14,r3

# qhasm: hid7 = hiu2
# asm 1: vmov >hid7=float32#16,<hiu2=int32#5
# asm 2: vmov >hid7=s15,<hiu2=r4
# copy-collector input: vmov s15,r4

# qhasm: lou0 = lod4
# asm 1: vmov >lou0=int32#6,<lod4=float32#9
# asm 2: vmov >lou0=r5,<lod4=s8
# copy-collector input: vmov r5,s8

# qhasm: hiu0 = hid4
# asm 1: vmov >hiu0=int32#7,<hid4=float32#10
# asm 2: vmov >hiu0=r6,<hid4=s9
# copy-collector input: vmov r6,s9

# qhasm: lou1 = lod5
# asm 1: vmov >lou1=int32#8,<lod5=float32#11
# asm 2: vmov >lou1=r7,<lod5=s10
# copy-collector input: vmov r7,s10

# qhasm: hiu1 = hid5
# asm 1: vmov >hiu1=int32#9,<hid5=float32#12
# asm 2: vmov >hiu1=r8,<hid5=s11
# copy-collector input: vmov r8,s11

# qhasm: lou3 = lod6
# asm 1: vmov >lou3=int32#10,<lod6=float32#13
# asm 2: vmov >lou3=r9,<lod6=s12
# copy-collector input: vmov r9,s12

# qhasm: hiu3 = hid6
# asm 1: vmov >hiu3=int32#11,<hid6=float32#14
# asm 2: vmov >hiu3=r10,<hid6=s13
# copy-collector input: vmov r10,s13

# qhasm: lotmp = (hiu0 unsigned>> 7)
# asm 1: mov >lotmp=int32#12,<hiu0=int32#7,LSR #7
# asm 2: mov >lotmp=r11,<hiu0=r6,LSR #7
# copy-collector output starts
vmov d7,r3,r4
vmov r5,r6,d4
vmov r7,r8,d5
vmov r9,r10,d6
# copy-collector output ends
mov r11,r6,LSR #7

# qhasm: lotmp ^= (lou0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSR #2
eor r11,r11,r6,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hitmp = (lou0 unsigned>> 7)
# asm 1: mov >hitmp=int32#12,<lou0=int32#6,LSR #7
# asm 2: mov >hitmp=r11,<lou0=r5,LSR #7
mov r11,r5,LSR #7

# qhasm: hitmp ^= (hiu0 << 25)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #25
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #25
eor r11,r11,r6,LSL #25

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSR #2
eor r11,r11,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #30
eor r11,r11,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSR #28
eor r11,r11,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSL #4
eor r11,r11,r5,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#12
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r11
adc r2,r2,r11

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
adds r1,r1,r5

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
# asm 1: vmov >lod3=float32#7,<lou4=int32#2
# asm 2: vmov >lod3=s6,<lou4=r1
# copy-collector input: vmov s6,r1

# qhasm: hid3 = hiu4
# asm 1: vmov >hid3=float32#8,<hiu4=int32#3
# asm 2: vmov >hid3=s7,<hiu4=r2
# copy-collector input: vmov s7,r2

# qhasm: lou0 = lod0
# asm 1: vmov >lou0=int32#2,<lod0=float32#1
# asm 2: vmov >lou0=r1,<lod0=s0
# copy-collector input: vmov r1,s0

# qhasm: hiu0 = hid0
# asm 1: vmov >hiu0=int32#3,<hid0=float32#2
# asm 2: vmov >hiu0=r2,<hid0=s1
# copy-collector input: vmov r2,s1

# qhasm: lou1 = lod1
# asm 1: vmov >lou1=int32#6,<lod1=float32#3
# asm 2: vmov >lou1=r5,<lod1=s2
# copy-collector input: vmov r5,s2

# qhasm: hiu1 = hid1
# asm 1: vmov >hiu1=int32#7,<hid1=float32#4
# asm 2: vmov >hiu1=r6,<hid1=s3
# copy-collector input: vmov r6,s3

# qhasm: lou4 = lod2
# asm 1: vmov >lou4=int32#8,<lod2=float32#5
# asm 2: vmov >lou4=r7,<lod2=s4
# copy-collector input: vmov r7,s4

# qhasm: hiu4 = hid2
# asm 1: vmov >hiu4=int32#9,<hid2=float32#6
# asm 2: vmov >hiu4=r8,<hid2=s5
# copy-collector input: vmov r8,s5

# qhasm: lou5 = lod13
# asm 1: vmov >lou5=int32#12,<lod13=float32#27
# asm 2: vmov >lou5=r11,<lod13=s26
# copy-collector input: vmov r11,s26

# qhasm: hiu5 = hid13
# asm 1: vmov >hiu5=int32#13,<hid13=float32#28
# asm 2: vmov >hiu5=r12,<hid13=s27
# copy-collector input: vmov r12,s27

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
# copy-collector output starts
vmov d3,r1,r2
vmov r1,r2,d0
vmov r5,r6,d1
vmov r7,r8,d2
vmov r11,r12,d13
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu2 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu2=int32#5,LSR #9
# asm 2: mov >lotmp=r11,<hiu2=r4,LSR #9
mov r11,r4,LSR #9

# qhasm: lotmp ^= (lou2 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSL #23
eor r11,r11,r3,LSL #23

# qhasm: lotmp ^= (lou2 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSR #18
eor r11,r11,r3,LSR #18

# qhasm: lotmp ^= (hiu2 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#5,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r4,LSL #14
eor r11,r11,r4,LSL #14

# qhasm: lotmp ^= (lou2 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSR #14
eor r11,r11,r3,LSR #14

# qhasm: lotmp ^= (hiu2 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#5,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r4,LSL #18
eor r11,r11,r4,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
adds r7,r7,r11

# qhasm: hitmp = (lou2 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou2=int32#4,LSR #9
# asm 2: mov >hitmp=r11,<lou2=r3,LSR #9
mov r11,r3,LSR #9

# qhasm: hitmp ^= (hiu2 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSL #23
eor r11,r11,r4,LSL #23

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
adds r1,r7,r1

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
# asm 1: vmov >lod6=float32#13,<lou3=int32#4
# asm 2: vmov >lod6=s12,<lou3=r3
# copy-collector input: vmov s12,r3

# qhasm: hid6 = hiu3
# asm 1: vmov >hid6=float32#14,<hiu3=int32#5
# asm 2: vmov >hid6=s13,<hiu3=r4
# copy-collector input: vmov s13,r4

# qhasm: lou0 = lod3
# asm 1: vmov >lou0=int32#6,<lod3=float32#7
# asm 2: vmov >lou0=r5,<lod3=s6
# copy-collector input: vmov r5,s6

# qhasm: hiu0 = hid3
# asm 1: vmov >hiu0=int32#7,<hid3=float32#8
# asm 2: vmov >hiu0=r6,<hid3=s7
# copy-collector input: vmov r6,s7

# qhasm: lou1 = lod4
# asm 1: vmov >lou1=int32#8,<lod4=float32#9
# asm 2: vmov >lou1=r7,<lod4=s8
# copy-collector input: vmov r7,s8

# qhasm: hiu1 = hid4
# asm 1: vmov >hiu1=int32#9,<hid4=float32#10
# asm 2: vmov >hiu1=r8,<hid4=s9
# copy-collector input: vmov r8,s9

# qhasm: lou2 = lod5
# asm 1: vmov >lou2=int32#10,<lod5=float32#11
# asm 2: vmov >lou2=r9,<lod5=s10
# copy-collector input: vmov r9,s10

# qhasm: hiu2 = hid5
# asm 1: vmov >hiu2=int32#11,<hid5=float32#12
# asm 2: vmov >hiu2=r10,<hid5=s11
# copy-collector input: vmov r10,s11

# qhasm: lotmp = (hiu0 unsigned>> 7)
# asm 1: mov >lotmp=int32#12,<hiu0=int32#7,LSR #7
# asm 2: mov >lotmp=r11,<hiu0=r6,LSR #7
# copy-collector output starts
vmov d6,r3,r4
vmov r5,r6,d3
vmov r7,r8,d4
vmov r9,r10,d5
# copy-collector output ends
mov r11,r6,LSR #7

# qhasm: lotmp ^= (lou0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSR #2
eor r11,r11,r6,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hitmp = (lou0 unsigned>> 7)
# asm 1: mov >hitmp=int32#12,<lou0=int32#6,LSR #7
# asm 2: mov >hitmp=r11,<lou0=r5,LSR #7
mov r11,r5,LSR #7

# qhasm: hitmp ^= (hiu0 << 25)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #25
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #25
eor r11,r11,r6,LSL #25

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSR #2
eor r11,r11,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #30
eor r11,r11,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSR #28
eor r11,r11,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSL #4
eor r11,r11,r5,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#12
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r11
adc r2,r2,r11

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
adds r1,r1,r5

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
# asm 1: vmov >lod2=float32#5,<lou4=int32#2
# asm 2: vmov >lod2=s4,<lou4=r1
# copy-collector input: vmov s4,r1

# qhasm: hid2 = hiu4
# asm 1: vmov >hid2=float32#6,<hiu4=int32#3
# asm 2: vmov >hid2=s5,<hiu4=r2
# copy-collector input: vmov s5,r2

# qhasm: assign 0 to lod0

# qhasm: assign 1 to hid0

# qhasm: assign 2 to lod1

# qhasm: assign 3 to hid1

# qhasm: assign 4 to lod2

# qhasm: assign 5 to hid2

# qhasm: assign 6 to lod3

# qhasm: assign 7 to hid3

# qhasm: assign 8 to lod4

# qhasm: assign 9 to hid4

# qhasm: assign 10 to lod5

# qhasm: assign 11 to hid5

# qhasm: assign 12 to lod6

# qhasm: assign 13 to hid6

# qhasm: assign 14 to lod7

# qhasm: assign 15 to hid7

# qhasm: lou0 = lod7
# asm 1: vmov >lou0=int32#2,<lod7=float32#15
# asm 2: vmov >lou0=r1,<lod7=s14
# copy-collector input: vmov r1,s14

# qhasm: hiu0 = hid7
# asm 1: vmov >hiu0=int32#3,<hid7=float32#16
# asm 2: vmov >hiu0=r2,<hid7=s15
# copy-collector input: vmov r2,s15

# qhasm: lou1 = lod0
# asm 1: vmov >lou1=int32#6,<lod0=float32#1
# asm 2: vmov >lou1=r5,<lod0=s0
# copy-collector input: vmov r5,s0

# qhasm: hiu1 = hid0
# asm 1: vmov >hiu1=int32#7,<hid0=float32#2
# asm 2: vmov >hiu1=r6,<hid0=s1
# copy-collector input: vmov r6,s1

# qhasm: lou4 = lod1
# asm 1: vmov >lou4=int32#8,<lod1=float32#3
# asm 2: vmov >lou4=r7,<lod1=s2
# copy-collector input: vmov r7,s2

# qhasm: hiu4 = hid1
# asm 1: vmov >hiu4=int32#9,<hid1=float32#4
# asm 2: vmov >hiu4=r8,<hid1=s3
# copy-collector input: vmov r8,s3

# qhasm: lou5 = lod14
# asm 1: vmov >lou5=int32#12,<lod14=float32#29
# asm 2: vmov >lou5=r11,<lod14=s28
# copy-collector input: vmov r11,s28

# qhasm: hiu5 = hid14
# asm 1: vmov >hiu5=int32#13,<hid14=float32#30
# asm 2: vmov >hiu5=r12,<hid14=s29
# copy-collector input: vmov r12,s29

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
# copy-collector output starts
vmov d2,r1,r2
vmov r1,r2,d7
vmov r5,r6,d0
vmov r7,r8,d1
vmov r11,r12,d14
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu3 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu3=int32#5,LSR #9
# asm 2: mov >lotmp=r11,<hiu3=r4,LSR #9
mov r11,r4,LSR #9

# qhasm: lotmp ^= (lou3 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSL #23
eor r11,r11,r3,LSL #23

# qhasm: lotmp ^= (lou3 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSR #18
eor r11,r11,r3,LSR #18

# qhasm: lotmp ^= (hiu3 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#5,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r4,LSL #14
eor r11,r11,r4,LSL #14

# qhasm: lotmp ^= (lou3 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou3=int32#4,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou3=r3,LSR #14
eor r11,r11,r3,LSR #14

# qhasm: lotmp ^= (hiu3 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu3=int32#5,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu3=r4,LSL #18
eor r11,r11,r4,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
adds r7,r7,r11

# qhasm: hitmp = (lou3 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou3=int32#4,LSR #9
# asm 2: mov >hitmp=r11,<lou3=r3,LSR #9
mov r11,r3,LSR #9

# qhasm: hitmp ^= (hiu3 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu3=int32#5,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu3=r4,LSL #23
eor r11,r11,r4,LSL #23

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
adds r1,r7,r1

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
# asm 1: vmov >lod5=float32#11,<lou2=int32#4
# asm 2: vmov >lod5=s10,<lou2=r3
# copy-collector input: vmov s10,r3

# qhasm: hid5 = hiu2
# asm 1: vmov >hid5=float32#12,<hiu2=int32#5
# asm 2: vmov >hid5=s11,<hiu2=r4
# copy-collector input: vmov s11,r4

# qhasm: lou0 = lod2
# asm 1: vmov >lou0=int32#6,<lod2=float32#5
# asm 2: vmov >lou0=r5,<lod2=s4
# copy-collector input: vmov r5,s4

# qhasm: hiu0 = hid2
# asm 1: vmov >hiu0=int32#7,<hid2=float32#6
# asm 2: vmov >hiu0=r6,<hid2=s5
# copy-collector input: vmov r6,s5

# qhasm: lou1 = lod3
# asm 1: vmov >lou1=int32#8,<lod3=float32#7
# asm 2: vmov >lou1=r7,<lod3=s6
# copy-collector input: vmov r7,s6

# qhasm: hiu1 = hid3
# asm 1: vmov >hiu1=int32#9,<hid3=float32#8
# asm 2: vmov >hiu1=r8,<hid3=s7
# copy-collector input: vmov r8,s7

# qhasm: lou3 = lod4
# asm 1: vmov >lou3=int32#10,<lod4=float32#9
# asm 2: vmov >lou3=r9,<lod4=s8
# copy-collector input: vmov r9,s8

# qhasm: hiu3 = hid4
# asm 1: vmov >hiu3=int32#11,<hid4=float32#10
# asm 2: vmov >hiu3=r10,<hid4=s9
# copy-collector input: vmov r10,s9

# qhasm: lotmp = (hiu0 unsigned>> 7)
# asm 1: mov >lotmp=int32#12,<hiu0=int32#7,LSR #7
# asm 2: mov >lotmp=r11,<hiu0=r6,LSR #7
# copy-collector output starts
vmov d5,r3,r4
vmov r5,r6,d2
vmov r7,r8,d3
vmov r9,r10,d4
# copy-collector output ends
mov r11,r6,LSR #7

# qhasm: lotmp ^= (lou0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#6,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r5,LSL #25
eor r11,r11,r5,LSL #25

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#7,LSR #2
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r6,LSR #2
eor r11,r11,r6,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#12
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r11
adds r1,r1,r11

# qhasm: hitmp = (lou0 unsigned>> 7)
# asm 1: mov >hitmp=int32#12,<lou0=int32#6,LSR #7
# asm 2: mov >hitmp=r11,<lou0=r5,LSR #7
mov r11,r5,LSR #7

# qhasm: hitmp ^= (hiu0 << 25)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #25
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #25
eor r11,r11,r6,LSL #25

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSR #2
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSR #2
eor r11,r11,r5,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSL #30
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSL #30
eor r11,r11,r6,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#7,LSR #28
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r6,LSR #28
eor r11,r11,r6,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#6,LSL #4
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r5,LSL #4
eor r11,r11,r5,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#12
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r11
adc r2,r2,r11

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
adds r1,r1,r5

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
# asm 1: vmov >lod1=float32#3,<lou4=int32#2
# asm 2: vmov >lod1=s2,<lou4=r1
# copy-collector input: vmov s2,r1

# qhasm: hid1 = hiu4
# asm 1: vmov >hid1=float32#4,<hiu4=int32#3
# asm 2: vmov >hid1=s3,<hiu4=r2
# copy-collector input: vmov s3,r2

# qhasm: lou0 = lod6
# asm 1: vmov >lou0=int32#2,<lod6=float32#13
# asm 2: vmov >lou0=r1,<lod6=s12
# copy-collector input: vmov r1,s12

# qhasm: hiu0 = hid6
# asm 1: vmov >hiu0=int32#3,<hid6=float32#14
# asm 2: vmov >hiu0=r2,<hid6=s13
# copy-collector input: vmov r2,s13

# qhasm: lou1 = lod7
# asm 1: vmov >lou1=int32#6,<lod7=float32#15
# asm 2: vmov >lou1=r5,<lod7=s14
# copy-collector input: vmov r5,s14

# qhasm: hiu1 = hid7
# asm 1: vmov >hiu1=int32#7,<hid7=float32#16
# asm 2: vmov >hiu1=r6,<hid7=s15
# copy-collector input: vmov r6,s15

# qhasm: lou4 = lod0
# asm 1: vmov >lou4=int32#8,<lod0=float32#1
# asm 2: vmov >lou4=r7,<lod0=s0
# copy-collector input: vmov r7,s0

# qhasm: hiu4 = hid0
# asm 1: vmov >hiu4=int32#9,<hid0=float32#2
# asm 2: vmov >hiu4=r8,<hid0=s1
# copy-collector input: vmov r8,s1

# qhasm: lou5 = lod15
# asm 1: vmov >lou5=int32#12,<lod15=float32#31
# asm 2: vmov >lou5=r11,<lod15=s30
# copy-collector input: vmov r11,s30

# qhasm: hiu5 = hid15
# asm 1: vmov >hiu5=int32#13,<hid15=float32#32
# asm 2: vmov >hiu5=r12,<hid15=s31
# copy-collector input: vmov r12,s31

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lou5=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lou5=r11
# copy-collector output starts
vmov d1,r1,r2
vmov r1,r2,d6
vmov r5,r6,d7
vmov r7,r8,d0
vmov r11,r12,d15
# copy-collector output ends
adds r7,r7,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#9,<hiu4=int32#9,<hiu5=int32#13
# asm 2: adc >hiu4=r8,<hiu4=r8,<hiu5=r12
adc r8,r8,r12

# qhasm: lotmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >lotmp=int32#12,[<input_1=int32#1],#4
# asm 2: ldr >lotmp=r11,[<input_1=r0],#4
# copy-collector input: ldr r11,[r0],#4

# qhasm: hitmp = mem32[input_1]; input_1 += 4
# asm 1: ldr >hitmp=int32#13,[<input_1=int32#1],#4
# asm 2: ldr >hitmp=r12,[<input_1=r0],#4
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

# qhasm: lotmp = (hiu2 unsigned>> 9)
# asm 1: mov >lotmp=int32#12,<hiu2=int32#5,LSR #9
# asm 2: mov >lotmp=r11,<hiu2=r4,LSR #9
mov r11,r4,LSR #9

# qhasm: lotmp ^= (lou2 << 23)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSL #23
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSL #23
eor r11,r11,r3,LSL #23

# qhasm: lotmp ^= (lou2 unsigned>> 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSR #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSR #18
eor r11,r11,r3,LSR #18

# qhasm: lotmp ^= (hiu2 << 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#5,LSL #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r4,LSL #14
eor r11,r11,r4,LSL #14

# qhasm: lotmp ^= (lou2 unsigned>> 14)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou2=int32#4,LSR #14
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou2=r3,LSR #14
eor r11,r11,r3,LSR #14

# qhasm: lotmp ^= (hiu2 << 18)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu2=int32#5,LSL #18
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu2=r4,LSL #18
eor r11,r11,r4,LSL #18

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#8,<lou4=int32#8,<lotmp=int32#12
# asm 2: adds >lou4=r7,<lou4=r7,<lotmp=r11
adds r7,r7,r11

# qhasm: hitmp = (lou2 unsigned>> 9)
# asm 1: mov >hitmp=int32#12,<lou2=int32#4,LSR #9
# asm 2: mov >hitmp=r11,<lou2=r3,LSR #9
mov r11,r3,LSR #9

# qhasm: hitmp ^= (hiu2 << 23)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu2=int32#5,LSL #23
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu2=r4,LSL #23
eor r11,r11,r4,LSL #23

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
adds r1,r7,r1

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
# asm 1: vmov >lod4=float32#9,<lou3=int32#4
# asm 2: vmov >lod4=s8,<lou3=r3
# copy-collector input: vmov s8,r3

# qhasm: hid4 = hiu3
# asm 1: vmov >hid4=float32#10,<hiu3=int32#5
# asm 2: vmov >hid4=s9,<hiu3=r4
# copy-collector input: vmov s9,r4

# qhasm: lou0 = lod1
# asm 1: vmov >lou0=int32#4,<lod1=float32#3
# asm 2: vmov >lou0=r3,<lod1=s2
# copy-collector input: vmov r3,s2

# qhasm: hiu0 = hid1
# asm 1: vmov >hiu0=int32#5,<hid1=float32#4
# asm 2: vmov >hiu0=r4,<hid1=s3
# copy-collector input: vmov r4,s3

# qhasm: lou1 = lod2
# asm 1: vmov >lou1=int32#6,<lod2=float32#5
# asm 2: vmov >lou1=r5,<lod2=s4
# copy-collector input: vmov r5,s4

# qhasm: hiu1 = hid2
# asm 1: vmov >hiu1=int32#7,<hid2=float32#6
# asm 2: vmov >hiu1=r6,<hid2=s5
# copy-collector input: vmov r6,s5

# qhasm: lou2 = lod3
# asm 1: vmov >lou2=int32#8,<lod3=float32#7
# asm 2: vmov >lou2=r7,<lod3=s6
# copy-collector input: vmov r7,s6

# qhasm: hiu2 = hid3
# asm 1: vmov >hiu2=int32#9,<hid3=float32#8
# asm 2: vmov >hiu2=r8,<hid3=s7
# copy-collector input: vmov r8,s7

# qhasm: lotmp = (hiu0 unsigned>> 7)
# asm 1: mov >lotmp=int32#10,<hiu0=int32#5,LSR #7
# asm 2: mov >lotmp=r9,<hiu0=r4,LSR #7
# copy-collector output starts
vmov d4,r3,r4
vmov r3,r4,d1
vmov r5,r6,d2
vmov r7,r8,d3
# copy-collector output ends
mov r9,r4,LSR #7

# qhasm: lotmp ^= (lou0 << 25)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou0=int32#4,LSL #25
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou0=r3,LSL #25
eor r9,r9,r3,LSL #25

# qhasm: lotmp ^= (hiu0 unsigned>> 2)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu0=int32#5,LSR #2
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu0=r4,LSR #2
eor r9,r9,r4,LSR #2

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

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#2,<lou4=int32#2,<lotmp=int32#10
# asm 2: adds >lou4=r1,<lou4=r1,<lotmp=r9
adds r1,r1,r9

# qhasm: hitmp = (lou0 unsigned>> 7)
# asm 1: mov >hitmp=int32#10,<lou0=int32#4,LSR #7
# asm 2: mov >hitmp=r9,<lou0=r3,LSR #7
mov r9,r3,LSR #7

# qhasm: hitmp ^= (hiu0 << 25)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<hiu0=int32#5,LSL #25
# asm 2: eor >hitmp=r9,<hitmp=r9,<hiu0=r4,LSL #25
eor r9,r9,r4,LSL #25

# qhasm: hitmp ^= (lou0 unsigned>> 2)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<lou0=int32#4,LSR #2
# asm 2: eor >hitmp=r9,<hitmp=r9,<lou0=r3,LSR #2
eor r9,r9,r3,LSR #2

# qhasm: hitmp ^= (hiu0 << 30)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<hiu0=int32#5,LSL #30
# asm 2: eor >hitmp=r9,<hitmp=r9,<hiu0=r4,LSL #30
eor r9,r9,r4,LSL #30

# qhasm: hitmp ^= (hiu0 unsigned>> 28)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<hiu0=int32#5,LSR #28
# asm 2: eor >hitmp=r9,<hitmp=r9,<hiu0=r4,LSR #28
eor r9,r9,r4,LSR #28

# qhasm: hitmp ^= (lou0 << 4)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<lou0=int32#4,LSL #4
# asm 2: eor >hitmp=r9,<hitmp=r9,<lou0=r3,LSL #4
eor r9,r9,r3,LSL #4

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#10
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r9
adc r2,r2,r9

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
adds r1,r1,r3

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
# asm 1: vmov >lod0=float32#1,<lou4=int32#2
# asm 2: vmov >lod0=s0,<lou4=r1
# copy-collector input: vmov s0,r1

# qhasm: hid0 = hiu4
# asm 1: vmov >hid0=float32#2,<hiu4=int32#3
# asm 2: vmov >hid0=s1,<hiu4=r2
# copy-collector input: vmov s1,r2

# qhasm: assign 16 to lod8

# qhasm: assign 17 to hid8

# qhasm: assign 18 to lod9

# qhasm: assign 19 to hid9

# qhasm: assign 20 to lod10

# qhasm: assign 21 to hid10

# qhasm: assign 22 to lod11

# qhasm: assign 23 to hid11

# qhasm: assign 24 to lod12

# qhasm: assign 25 to hid12

# qhasm: assign 26 to lod13

# qhasm: assign 27 to hid13

# qhasm: assign 28 to lod14

# qhasm: assign 29 to hid14

# qhasm: assign 30 to lod15

# qhasm: assign 31 to hid15

# qhasm: input_0 = o3
# asm 1: ldr >input_0=int32#2,<o3=stack32#36
# asm 2: ldr >input_0=r1,<o3=[sp,#140]
# copy-collector input: ldr r1,[sp,#140]

# qhasm: =? unsigned<? input_0 -= 8
# asm 1: subs >input_0=int32#2,<input_0=int32#2,#8
# asm 2: subs >input_0=r1,<input_0=r1,#8
# copy-collector output starts
vmov d0,r1,r2
ldr r1,[sp,#140]
# copy-collector output ends
subs r1,r1,#8

# qhasm: goto endinnerloop if =
beq ._endinnerloop

# qhasm: o3 = input_0
# asm 1: str <input_0=int32#2,>o3=stack32#36
# asm 2: str <input_0=r1,>o3=[sp,#140]
# copy-collector input: str r1,[sp,#140]

# qhasm: =? input_0 - 8
# asm 1: cmp <input_0=int32#2,#8
# asm 2: cmp <input_0=r1,#8
# copy-collector output starts
str r1,[sp,#140]
# copy-collector output ends
cmp r1,#8

# qhasm: goto nearend if =
beq ._nearend

# qhasm: lou0 = lod8
# asm 1: vmov >lou0=int32#2,<lod8=float32#17
# asm 2: vmov >lou0=r1,<lod8=s16
# copy-collector input: vmov r1,s16

# qhasm: hiu0 = hid8
# asm 1: vmov >hiu0=int32#3,<hid8=float32#18
# asm 2: vmov >hiu0=r2,<hid8=s17
# copy-collector input: vmov r2,s17

# qhasm: lou1 = lod9
# asm 1: vmov >lou1=int32#4,<lod9=float32#19
# asm 2: vmov >lou1=r3,<lod9=s18
# copy-collector input: vmov r3,s18

# qhasm: hiu1 = hid9
# asm 1: vmov >hiu1=int32#5,<hid9=float32#20
# asm 2: vmov >hiu1=r4,<hid9=s19
# copy-collector input: vmov r4,s19

# qhasm: lou2 = lom14
# asm 1: ldr >lou2=int32#6,<lom14=stack32#32
# asm 2: ldr >lou2=r5,<lom14=[sp,#124]
# copy-collector input: ldr r5,[sp,#124]

# qhasm: hiu2 = him14
# asm 1: ldr >hiu2=int32#7,<him14=stack32#33
# asm 2: ldr >hiu2=r6,<him14=[sp,#128]
# copy-collector input: ldr r6,[sp,#128]

# qhasm: lou3 = lom9
# asm 1: ldr >lou3=int32#8,<lom9=stack32#22
# asm 2: ldr >lou3=r7,<lom9=[sp,#84]
# copy-collector input: ldr r7,[sp,#84]

# qhasm: hiu3 = him9
# asm 1: ldr >hiu3=int32#9,<him9=stack32#23
# asm 2: ldr >hiu3=r8,<him9=[sp,#88]
# copy-collector input: ldr r8,[sp,#88]

# qhasm: lotmp = (lou2 unsigned>> 6)
# asm 1: mov >lotmp=int32#10,<lou2=int32#6,LSR #6
# asm 2: mov >lotmp=r9,<lou2=r5,LSR #6
# copy-collector output starts
vmov r1,r2,d8
vmov r3,r4,d9
ldr r5,[sp,#124]
ldr r6,[sp,#128]
ldr.w r7,[sp,#84]
ldr r8,[sp,#88]
# copy-collector output ends
mov r9,r5,LSR #6

# qhasm: lotmp ^= (hiu2 << 26)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu2=int32#7,LSL #26
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu2=r6,LSL #26
eor r9,r9,r6,LSL #26

# qhasm: lotmp ^= (lou2 unsigned>> 19)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou2=int32#6,LSR #19
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou2=r5,LSR #19
eor r9,r9,r5,LSR #19

# qhasm: lotmp ^= (hiu2 << 13)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu2=int32#7,LSL #13
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu2=r6,LSL #13
eor r9,r9,r6,LSL #13

# qhasm: lotmp ^= (hiu2 unsigned>> 29)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu2=int32#7,LSR #29
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu2=r6,LSR #29
eor r9,r9,r6,LSR #29

# qhasm: lotmp ^= (lou2 << 3)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou2=int32#6,LSL #3
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou2=r5,LSL #3
eor r9,r9,r5,LSL #3

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#2,<lou0=int32#2,<lotmp=int32#10
# asm 2: adds >lou0=r1,<lou0=r1,<lotmp=r9
adds r1,r1,r9

# qhasm: hitmp = (hiu2 unsigned>> 6)
# asm 1: mov >hitmp=int32#10,<hiu2=int32#7,LSR #6
# asm 2: mov >hitmp=r9,<hiu2=r6,LSR #6
mov r9,r6,LSR #6

# qhasm: hitmp ^= (hiu2 unsigned>> 19)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<hiu2=int32#7,LSR #19
# asm 2: eor >hitmp=r9,<hitmp=r9,<hiu2=r6,LSR #19
eor r9,r9,r6,LSR #19

# qhasm: hitmp ^= (lou2 << 13)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<lou2=int32#6,LSL #13
# asm 2: eor >hitmp=r9,<hitmp=r9,<lou2=r5,LSL #13
eor r9,r9,r5,LSL #13

# qhasm: hitmp ^= (lou2 unsigned>> 29)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<lou2=int32#6,LSR #29
# asm 2: eor >hitmp=r9,<hitmp=r9,<lou2=r5,LSR #29
eor r9,r9,r5,LSR #29

# qhasm: hitmp ^= (hiu2 << 3)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<hiu2=int32#7,LSL #3
# asm 2: eor >hitmp=r9,<hitmp=r9,<hiu2=r6,LSL #3
eor r9,r9,r6,LSL #3

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hitmp=int32#10
# asm 2: adc >hiu0=r2,<hiu0=r2,<hitmp=r9
adc r2,r2,r9

# qhasm: carry? lotmp = (lou1 unsigned>> 1)
# asm 1: movs >lotmp=int32#10,<lou1=int32#4,LSR #1
# asm 2: movs >lotmp=r9,<lou1=r3,LSR #1
movs r9,r3,LSR #1

# qhasm: hitmp = (carry,hiu1 unsigned>> 1)
# asm 1: mov >hitmp=int32#11,<hiu1=int32#5,RRX
# asm 2: mov >hitmp=r10,<hiu1=r4,RRX
mov r10,r4,RRX

# qhasm: lotmp ^= (hiu1 << 31)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu1=int32#5,LSL #31
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu1=r4,LSL #31
eor r9,r9,r4,LSL #31

# qhasm: lotmp ^= (lou1 unsigned>>7)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou1=int32#4,LSR #7
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou1=r3,LSR #7
eor r9,r9,r3,LSR #7

# qhasm: lotmp ^= (hiu1 << 25)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu1=int32#5,LSL #25
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu1=r4,LSL #25
eor r9,r9,r4,LSL #25

# qhasm: hitmp ^= (hiu1 unsigned>>7)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#5,LSR #7
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu1=r4,LSR #7
eor r10,r10,r4,LSR #7

# qhasm: lotmp ^= (lou1 unsigned>>8)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou1=int32#4,LSR #8
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou1=r3,LSR #8
eor r9,r9,r3,LSR #8

# qhasm: lotmp ^= (hiu1 << 24)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu1=int32#5,LSL #24
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu1=r4,LSL #24
eor r9,r9,r4,LSL #24

# qhasm: hitmp ^= (hiu1 unsigned>>8)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#5,LSR #8
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu1=r4,LSR #8
eor r10,r10,r4,LSR #8

# qhasm: hitmp ^= (lou1 << 24)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<lou1=int32#4,LSL #24
# asm 2: eor >hitmp=r10,<hitmp=r10,<lou1=r3,LSL #24
eor r10,r10,r3,LSL #24

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#2,<lou0=int32#2,<lotmp=int32#10
# asm 2: adds >lou0=r1,<lou0=r1,<lotmp=r9
adds r1,r1,r9

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hitmp=int32#11
# asm 2: adc >hiu0=r2,<hiu0=r2,<hitmp=r10
adc r2,r2,r10

# qhasm: carry? lou0 += lou3
# asm 1: adds >lou0=int32#2,<lou0=int32#2,<lou3=int32#8
# asm 2: adds >lou0=r1,<lou0=r1,<lou3=r7
adds.w r1,r1,r7

# qhasm: hiu0 += hiu3 + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hiu3=int32#9
# asm 2: adc >hiu0=r2,<hiu0=r2,<hiu3=r8
adc r2,r2,r8

# qhasm: lod9 = lou3
# asm 1: vmov >lod9=float32#19,<lou3=int32#8
# asm 2: vmov >lod9=s18,<lou3=r7
# copy-collector input: vmov s18,r7

# qhasm: hid9 = hiu3
# asm 1: vmov >hid9=float32#20,<hiu3=int32#9
# asm 2: vmov >hid9=s19,<hiu3=r8
# copy-collector input: vmov s19,r8

# qhasm: lou3 = lom15
# asm 1: ldr >lou3=int32#8,<lom15=stack32#34
# asm 2: ldr >lou3=r7,<lom15=[sp,#132]
# copy-collector input: ldr r7,[sp,#132]

# qhasm: hiu3 = him15
# asm 1: ldr >hiu3=int32#9,<him15=stack32#35
# asm 2: ldr >hiu3=r8,<him15=[sp,#136]
# copy-collector input: ldr r8,[sp,#136]

# qhasm: lom15 = lou0
# asm 1: str <lou0=int32#2,>lom15=stack32#34
# asm 2: str <lou0=r1,>lom15=[sp,#132]
# copy-collector input: str r1,[sp,#132]

# qhasm: him15 = hiu0
# asm 1: str <hiu0=int32#3,>him15=stack32#35
# asm 2: str <hiu0=r2,>him15=[sp,#136]
# copy-collector input: str r2,[sp,#136]

# qhasm: lotmp = (lou3 unsigned>> 6)
# asm 1: mov >lotmp=int32#10,<lou3=int32#8,LSR #6
# asm 2: mov >lotmp=r9,<lou3=r7,LSR #6
# copy-collector output starts
vmov d9,r7,r8
ldrd r7,r8,[sp,#132]
strd r1,r2,[sp,#132]
# copy-collector output ends
mov r9,r7,LSR #6

# qhasm: lotmp ^= (hiu3 << 26)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu3=int32#9,LSL #26
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu3=r8,LSL #26
eor r9,r9,r8,LSL #26

# qhasm: lotmp ^= (lou3 unsigned>> 19)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou3=int32#8,LSR #19
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou3=r7,LSR #19
eor r9,r9,r7,LSR #19

# qhasm: lotmp ^= (hiu3 << 13)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu3=int32#9,LSL #13
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu3=r8,LSL #13
eor r9,r9,r8,LSL #13

# qhasm: lotmp ^= (hiu3 unsigned>> 29)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu3=int32#9,LSR #29
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu3=r8,LSR #29
eor r9,r9,r8,LSR #29

# qhasm: lotmp ^= (lou3 << 3)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou3=int32#8,LSL #3
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou3=r7,LSL #3
eor r9,r9,r7,LSL #3

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#4,<lou1=int32#4,<lotmp=int32#10
# asm 2: adds >lou1=r3,<lou1=r3,<lotmp=r9
adds r3,r3,r9

# qhasm: hitmp = (hiu3 unsigned>> 6)
# asm 1: mov >hitmp=int32#10,<hiu3=int32#9,LSR #6
# asm 2: mov >hitmp=r9,<hiu3=r8,LSR #6
mov r9,r8,LSR #6

# qhasm: hitmp ^= (hiu3 unsigned>> 19)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<hiu3=int32#9,LSR #19
# asm 2: eor >hitmp=r9,<hitmp=r9,<hiu3=r8,LSR #19
eor r9,r9,r8,LSR #19

# qhasm: hitmp ^= (lou3 << 13)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<lou3=int32#8,LSL #13
# asm 2: eor >hitmp=r9,<hitmp=r9,<lou3=r7,LSL #13
eor r9,r9,r7,LSL #13

# qhasm: hitmp ^= (lou3 unsigned>> 29)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<lou3=int32#8,LSR #29
# asm 2: eor >hitmp=r9,<hitmp=r9,<lou3=r7,LSR #29
eor r9,r9,r7,LSR #29

# qhasm: hitmp ^= (hiu3 << 3)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#10,<hiu3=int32#9,LSL #3
# asm 2: eor >hitmp=r9,<hitmp=r9,<hiu3=r8,LSL #3
eor r9,r9,r8,LSL #3

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#5,<hiu1=int32#5,<hitmp=int32#10
# asm 2: adc >hiu1=r4,<hiu1=r4,<hitmp=r9
adc r4,r4,r9

# qhasm: lou4 = lod10
# asm 1: vmov >lou4=int32#10,<lod10=float32#21
# asm 2: vmov >lou4=r9,<lod10=s20
# copy-collector input: vmov r9,s20

# qhasm: hiu4 = hid10
# asm 1: vmov >hiu4=int32#11,<hid10=float32#22
# asm 2: vmov >hiu4=r10,<hid10=s21
# copy-collector input: vmov r10,s21

# qhasm: carry? lotmp = (lou4 unsigned>> 1)
# asm 1: movs >lotmp=int32#12,<lou4=int32#10,LSR #1
# asm 2: movs >lotmp=r11,<lou4=r9,LSR #1
# copy-collector output starts
vmov r9,r10,d10
# copy-collector output ends
movs r11,r9,LSR #1

# qhasm: hitmp = (carry,hiu4 unsigned>> 1)
# asm 1: mov >hitmp=int32#13,<hiu4=int32#11,RRX
# asm 2: mov >hitmp=r12,<hiu4=r10,RRX
mov r12,r10,RRX

# qhasm: lotmp ^= (hiu4 << 31)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#11,LSL #31
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r10,LSL #31
eor r11,r11,r10,LSL #31

# qhasm: lotmp ^= (lou4 unsigned>>7)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou4=int32#10,LSR #7
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou4=r9,LSR #7
eor r11,r11,r9,LSR #7

# qhasm: lotmp ^= (hiu4 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#11,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r10,LSL #25
eor r11,r11,r10,LSL #25

# qhasm: hitmp ^= (hiu4 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#11,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu4=r10,LSR #7
eor r12,r12,r10,LSR #7

# qhasm: lotmp ^= (lou4 unsigned>>8)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou4=int32#10,LSR #8
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou4=r9,LSR #8
eor r11,r11,r9,LSR #8

# qhasm: lotmp ^= (hiu4 << 24)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#11,LSL #24
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r10,LSL #24
eor r11,r11,r10,LSL #24

# qhasm: hitmp ^= (hiu4 unsigned>>8)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#11,LSR #8
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu4=r10,LSR #8
eor r12,r12,r10,LSR #8

# qhasm: hitmp ^= (lou4 << 24)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou4=int32#10,LSL #24
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou4=r9,LSL #24
eor r12,r12,r9,LSL #24

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#4,<lou1=int32#4,<lotmp=int32#12
# asm 2: adds >lou1=r3,<lou1=r3,<lotmp=r11
adds r3,r3,r11

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#5,<hiu1=int32#5,<hitmp=int32#13
# asm 2: adc >hiu1=r4,<hiu1=r4,<hitmp=r12
adc r4,r4,r12

# qhasm: lou5 = lom10
# asm 1: ldr >lou5=int32#12,<lom10=stack32#24
# asm 2: ldr >lou5=r11,<lom10=[sp,#92]
# copy-collector input: ldr r11,[sp,#92]

# qhasm: hiu5 = him10
# asm 1: ldr >hiu5=int32#13,<him10=stack32#25
# asm 2: ldr >hiu5=r12,<him10=[sp,#96]
# copy-collector input: ldr r12,[sp,#96]

# qhasm: carry? lou1 += lou5
# asm 1: adds >lou1=int32#4,<lou1=int32#4,<lou5=int32#12
# asm 2: adds >lou1=r3,<lou1=r3,<lou5=r11
# copy-collector output starts
ldrd r11,r12,[sp,#92]
# copy-collector output ends
adds r3,r3,r11

# qhasm: hiu1 += hiu5 + carry
# asm 1: adc >hiu1=int32#5,<hiu1=int32#5,<hiu5=int32#13
# asm 2: adc >hiu1=r4,<hiu1=r4,<hiu5=r12
adc r4,r4,r12

# qhasm: lod10 = lou5
# asm 1: vmov >lod10=float32#21,<lou5=int32#12
# asm 2: vmov >lod10=s20,<lou5=r11
# copy-collector input: vmov s20,r11

# qhasm: hid10 = hiu5
# asm 1: vmov >hid10=float32#22,<hiu5=int32#13
# asm 2: vmov >hid10=s21,<hiu5=r12
# copy-collector input: vmov s21,r12

# qhasm: lom9 = lou1
# asm 1: str <lou1=int32#4,>lom9=stack32#22
# asm 2: str <lou1=r3,>lom9=[sp,#84]
# copy-collector input: str r3,[sp,#84]

# qhasm: him9 = hiu1
# asm 1: str <hiu1=int32#5,>him9=stack32#23
# asm 2: str <hiu1=r4,>him9=[sp,#88]
# copy-collector input: str r4,[sp,#88]

# qhasm: lotmp = (lou0 unsigned>> 6)
# asm 1: mov >lotmp=int32#12,<lou0=int32#2,LSR #6
# asm 2: mov >lotmp=r11,<lou0=r1,LSR #6
# copy-collector output starts
vmov d10,r11,r12
strd r3,r4,[sp,#84]
# copy-collector output ends
mov r11,r1,LSR #6

# qhasm: lotmp ^= (hiu0 << 26)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#3,LSL #26
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r2,LSL #26
eor r11,r11,r2,LSL #26

# qhasm: lotmp ^= (lou0 unsigned>> 19)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#2,LSR #19
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r1,LSR #19
eor r11,r11,r1,LSR #19

# qhasm: lotmp ^= (hiu0 << 13)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#3,LSL #13
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r2,LSL #13
eor r11,r11,r2,LSL #13

# qhasm: lotmp ^= (hiu0 unsigned>> 29)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#3,LSR #29
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r2,LSR #29
eor r11,r11,r2,LSR #29

# qhasm: lotmp ^= (lou0 << 3)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#2,LSL #3
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r1,LSL #3
eor r11,r11,r1,LSL #3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lotmp=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lotmp=r11
adds r9,r9,r11

# qhasm: hitmp = (hiu0 unsigned>> 6)
# asm 1: mov >hitmp=int32#12,<hiu0=int32#3,LSR #6
# asm 2: mov >hitmp=r11,<hiu0=r2,LSR #6
mov r11,r2,LSR #6

# qhasm: hitmp ^= (hiu0 unsigned>> 19)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#3,LSR #19
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r2,LSR #19
eor r11,r11,r2,LSR #19

# qhasm: hitmp ^= (lou0 << 13)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#2,LSL #13
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r1,LSL #13
eor r11,r11,r1,LSL #13

# qhasm: hitmp ^= (lou0 unsigned>> 29)
# asm 1: eor >hitmp=int32#2,<hitmp=int32#12,<lou0=int32#2,LSR #29
# asm 2: eor >hitmp=r1,<hitmp=r11,<lou0=r1,LSR #29
eor r1,r11,r1,LSR #29

# qhasm: hitmp ^= (hiu0 << 3)
# asm 1: eor >hitmp=int32#2,<hitmp=int32#2,<hiu0=int32#3,LSL #3
# asm 2: eor >hitmp=r1,<hitmp=r1,<hiu0=r2,LSL #3
eor r1,r1,r2,LSL #3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#2,<hiu4=int32#11,<hitmp=int32#2
# asm 2: adc >hiu4=r1,<hiu4=r10,<hitmp=r1
adc r1,r10,r1

# qhasm: lou0 = lod11
# asm 1: vmov >lou0=int32#3,<lod11=float32#23
# asm 2: vmov >lou0=r2,<lod11=s22
# copy-collector input: vmov r2,s22

# qhasm: hiu0 = hid11
# asm 1: vmov >hiu0=int32#11,<hid11=float32#24
# asm 2: vmov >hiu0=r10,<hid11=s23
# copy-collector input: vmov r10,s23

# qhasm: carry? lotmp = (lou0 unsigned>> 1)
# asm 1: movs >lotmp=int32#12,<lou0=int32#3,LSR #1
# asm 2: movs >lotmp=r11,<lou0=r2,LSR #1
# copy-collector output starts
vmov r2,r10,d11
# copy-collector output ends
movs r11,r2,LSR #1

# qhasm: hitmp = (carry,hiu0 unsigned>> 1)
# asm 1: mov >hitmp=int32#13,<hiu0=int32#11,RRX
# asm 2: mov >hitmp=r12,<hiu0=r10,RRX
mov r12,r10,RRX

# qhasm: lotmp ^= (hiu0 << 31)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#11,LSL #31
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r10,LSL #31
eor r11,r11,r10,LSL #31

# qhasm: lotmp ^= (lou0 unsigned>>7)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#3,LSR #7
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r2,LSR #7
eor r11,r11,r2,LSR #7

# qhasm: lotmp ^= (hiu0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#11,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r10,LSL #25
eor r11,r11,r10,LSL #25

# qhasm: hitmp ^= (hiu0 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#11,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r10,LSR #7
eor r12,r12,r10,LSR #7

# qhasm: lotmp ^= (lou0 unsigned>>8)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#3,LSR #8
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r2,LSR #8
eor r11,r11,r2,LSR #8

# qhasm: lotmp ^= (hiu0 << 24)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#11,LSL #24
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r10,LSL #24
eor r11,r11,r10,LSL #24

# qhasm: hitmp ^= (hiu0 unsigned>>8)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#11,LSR #8
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r10,LSR #8
eor r12,r12,r10,LSR #8

# qhasm: hitmp ^= (lou0 << 24)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#3,LSL #24
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r2,LSL #24
eor r12,r12,r2,LSL #24

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lotmp=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lotmp=r11
adds r9,r9,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#2,<hiu4=int32#2,<hitmp=int32#13
# asm 2: adc >hiu4=r1,<hiu4=r1,<hitmp=r12
adc r1,r1,r12

# qhasm: lou5 = lom11
# asm 1: ldr >lou5=int32#12,<lom11=stack32#26
# asm 2: ldr >lou5=r11,<lom11=[sp,#100]
# copy-collector input: ldr r11,[sp,#100]

# qhasm: hiu5 = him11
# asm 1: ldr >hiu5=int32#13,<him11=stack32#27
# asm 2: ldr >hiu5=r12,<him11=[sp,#104]
# copy-collector input: ldr r12,[sp,#104]

# qhasm: carry? lou4 += lou5
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lou5=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lou5=r11
# copy-collector output starts
ldrd r11,r12,[sp,#100]
# copy-collector output ends
adds r9,r9,r11

# qhasm: hiu4 += hiu5 + carry
# asm 1: adc >hiu4=int32#2,<hiu4=int32#2,<hiu5=int32#13
# asm 2: adc >hiu4=r1,<hiu4=r1,<hiu5=r12
adc r1,r1,r12

# qhasm: lod11 = lou5
# asm 1: vmov >lod11=float32#23,<lou5=int32#12
# asm 2: vmov >lod11=s22,<lou5=r11
# copy-collector input: vmov s22,r11

# qhasm: hid11 = hiu5
# asm 1: vmov >hid11=float32#24,<hiu5=int32#13
# asm 2: vmov >hid11=s23,<hiu5=r12
# copy-collector input: vmov s23,r12

# qhasm: lom10 = lou4
# asm 1: str <lou4=int32#10,>lom10=stack32#24
# asm 2: str <lou4=r9,>lom10=[sp,#92]
# copy-collector input: str r9,[sp,#92]

# qhasm: him10 = hiu4
# asm 1: str <hiu4=int32#2,>him10=stack32#25
# asm 2: str <hiu4=r1,>him10=[sp,#96]
# copy-collector input: str r1,[sp,#96]

# qhasm: lotmp = (lou1 unsigned>> 6)
# asm 1: mov >lotmp=int32#12,<lou1=int32#4,LSR #6
# asm 2: mov >lotmp=r11,<lou1=r3,LSR #6
# copy-collector output starts
vmov d11,r11,r12
strd r9,r1,[sp,#92]
# copy-collector output ends
mov r11,r3,LSR #6

# qhasm: lotmp ^= (hiu1 << 26)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#5,LSL #26
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r4,LSL #26
eor r11,r11,r4,LSL #26

# qhasm: lotmp ^= (lou1 unsigned>> 19)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#4,LSR #19
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r3,LSR #19
eor r11,r11,r3,LSR #19

# qhasm: lotmp ^= (hiu1 << 13)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#5,LSL #13
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r4,LSL #13
eor r11,r11,r4,LSL #13

# qhasm: lotmp ^= (hiu1 unsigned>> 29)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#5,LSR #29
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r4,LSR #29
eor r11,r11,r4,LSR #29

# qhasm: lotmp ^= (lou1 << 3)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#4,LSL #3
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r3,LSL #3
eor r11,r11,r3,LSL #3

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#3,<lou0=int32#3,<lotmp=int32#12
# asm 2: adds >lou0=r2,<lou0=r2,<lotmp=r11
adds r2,r2,r11

# qhasm: hitmp = (hiu1 unsigned>> 6)
# asm 1: mov >hitmp=int32#12,<hiu1=int32#5,LSR #6
# asm 2: mov >hitmp=r11,<hiu1=r4,LSR #6
mov r11,r4,LSR #6

# qhasm: hitmp ^= (hiu1 unsigned>> 19)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu1=int32#5,LSR #19
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu1=r4,LSR #19
eor r11,r11,r4,LSR #19

# qhasm: hitmp ^= (lou1 << 13)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou1=int32#4,LSL #13
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou1=r3,LSL #13
eor r11,r11,r3,LSL #13

# qhasm: hitmp ^= (lou1 unsigned>> 29)
# asm 1: eor >hitmp=int32#4,<hitmp=int32#12,<lou1=int32#4,LSR #29
# asm 2: eor >hitmp=r3,<hitmp=r11,<lou1=r3,LSR #29
eor r3,r11,r3,LSR #29

# qhasm: hitmp ^= (hiu1 << 3)
# asm 1: eor >hitmp=int32#4,<hitmp=int32#4,<hiu1=int32#5,LSL #3
# asm 2: eor >hitmp=r3,<hitmp=r3,<hiu1=r4,LSL #3
eor r3,r3,r4,LSL #3

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#4,<hiu0=int32#11,<hitmp=int32#4
# asm 2: adc >hiu0=r3,<hiu0=r10,<hitmp=r3
adc r3,r10,r3

# qhasm: lou1 = lod12
# asm 1: vmov >lou1=int32#5,<lod12=float32#25
# asm 2: vmov >lou1=r4,<lod12=s24
# copy-collector input: vmov r4,s24

# qhasm: hiu1 = hid12
# asm 1: vmov >hiu1=int32#11,<hid12=float32#26
# asm 2: vmov >hiu1=r10,<hid12=s25
# copy-collector input: vmov r10,s25

# qhasm: carry? lotmp = (lou1 unsigned>> 1)
# asm 1: movs >lotmp=int32#12,<lou1=int32#5,LSR #1
# asm 2: movs >lotmp=r11,<lou1=r4,LSR #1
# copy-collector output starts
vmov r4,r10,d12
# copy-collector output ends
movs r11,r4,LSR #1

# qhasm: hitmp = (carry,hiu1 unsigned>> 1)
# asm 1: mov >hitmp=int32#13,<hiu1=int32#11,RRX
# asm 2: mov >hitmp=r12,<hiu1=r10,RRX
mov r12,r10,RRX

# qhasm: lotmp ^= (hiu1 << 31)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#11,LSL #31
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r10,LSL #31
eor r11,r11,r10,LSL #31

# qhasm: lotmp ^= (lou1 unsigned>>7)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#5,LSR #7
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r4,LSR #7
eor r11,r11,r4,LSR #7

# qhasm: lotmp ^= (hiu1 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#11,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r10,LSL #25
eor r11,r11,r10,LSL #25

# qhasm: hitmp ^= (hiu1 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu1=int32#11,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu1=r10,LSR #7
eor r12,r12,r10,LSR #7

# qhasm: lotmp ^= (lou1 unsigned>>8)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou1=int32#5,LSR #8
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou1=r4,LSR #8
eor r11,r11,r4,LSR #8

# qhasm: lotmp ^= (hiu1 << 24)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu1=int32#11,LSL #24
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu1=r10,LSL #24
eor r11,r11,r10,LSL #24

# qhasm: hitmp ^= (hiu1 unsigned>>8)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu1=int32#11,LSR #8
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu1=r10,LSR #8
eor r12,r12,r10,LSR #8

# qhasm: hitmp ^= (lou1 << 24)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou1=int32#5,LSL #24
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou1=r4,LSL #24
eor r12,r12,r4,LSL #24

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#3,<lou0=int32#3,<lotmp=int32#12
# asm 2: adds >lou0=r2,<lou0=r2,<lotmp=r11
adds r2,r2,r11

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#4,<hiu0=int32#4,<hitmp=int32#13
# asm 2: adc >hiu0=r3,<hiu0=r3,<hitmp=r12
adc r3,r3,r12

# qhasm: lou5 = lom12
# asm 1: ldr >lou5=int32#12,<lom12=stack32#28
# asm 2: ldr >lou5=r11,<lom12=[sp,#108]
# copy-collector input: ldr r11,[sp,#108]

# qhasm: hiu5 = him12
# asm 1: ldr >hiu5=int32#13,<him12=stack32#29
# asm 2: ldr >hiu5=r12,<him12=[sp,#112]
# copy-collector input: ldr r12,[sp,#112]

# qhasm: carry? lou0 += lou5
# asm 1: adds >lou0=int32#3,<lou0=int32#3,<lou5=int32#12
# asm 2: adds >lou0=r2,<lou0=r2,<lou5=r11
# copy-collector output starts
ldrd r11,r12,[sp,#108]
# copy-collector output ends
adds r2,r2,r11

# qhasm: hiu0 += hiu5 + carry
# asm 1: adc >hiu0=int32#4,<hiu0=int32#4,<hiu5=int32#13
# asm 2: adc >hiu0=r3,<hiu0=r3,<hiu5=r12
adc r3,r3,r12

# qhasm: lod12 = lou5
# asm 1: vmov >lod12=float32#25,<lou5=int32#12
# asm 2: vmov >lod12=s24,<lou5=r11
# copy-collector input: vmov s24,r11

# qhasm: hid12 = hiu5
# asm 1: vmov >hid12=float32#26,<hiu5=int32#13
# asm 2: vmov >hid12=s25,<hiu5=r12
# copy-collector input: vmov s25,r12

# qhasm: lom11 = lou0
# asm 1: str <lou0=int32#3,>lom11=stack32#26
# asm 2: str <lou0=r2,>lom11=[sp,#100]
# copy-collector input: str r2,[sp,#100]

# qhasm: him11 = hiu0
# asm 1: str <hiu0=int32#4,>him11=stack32#27
# asm 2: str <hiu0=r3,>him11=[sp,#104]
# copy-collector input: str r3,[sp,#104]

# qhasm: lotmp = (lou4 unsigned>> 6)
# asm 1: mov >lotmp=int32#12,<lou4=int32#10,LSR #6
# asm 2: mov >lotmp=r11,<lou4=r9,LSR #6
# copy-collector output starts
vmov d12,r11,r12
strd r2,r3,[sp,#100]
# copy-collector output ends
mov r11,r9,LSR #6

# qhasm: lotmp ^= (hiu4 << 26)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#2,LSL #26
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r1,LSL #26
eor r11,r11,r1,LSL #26

# qhasm: lotmp ^= (lou4 unsigned>> 19)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou4=int32#10,LSR #19
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou4=r9,LSR #19
eor r11,r11,r9,LSR #19

# qhasm: lotmp ^= (hiu4 << 13)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#2,LSL #13
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r1,LSL #13
eor r11,r11,r1,LSL #13

# qhasm: lotmp ^= (hiu4 unsigned>> 29)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#2,LSR #29
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r1,LSR #29
eor r11,r11,r1,LSR #29

# qhasm: lotmp ^= (lou4 << 3)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou4=int32#10,LSL #3
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou4=r9,LSL #3
eor r11,r11,r9,LSL #3

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#5,<lou1=int32#5,<lotmp=int32#12
# asm 2: adds >lou1=r4,<lou1=r4,<lotmp=r11
adds r4,r4,r11

# qhasm: hitmp = (hiu4 unsigned>> 6)
# asm 1: mov >hitmp=int32#12,<hiu4=int32#2,LSR #6
# asm 2: mov >hitmp=r11,<hiu4=r1,LSR #6
mov r11,r1,LSR #6

# qhasm: hitmp ^= (hiu4 unsigned>> 19)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu4=int32#2,LSR #19
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu4=r1,LSR #19
eor r11,r11,r1,LSR #19

# qhasm: hitmp ^= (lou4 << 13)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou4=int32#10,LSL #13
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou4=r9,LSL #13
eor r11,r11,r9,LSL #13

# qhasm: hitmp ^= (lou4 unsigned>> 29)
# asm 1: eor >hitmp=int32#10,<hitmp=int32#12,<lou4=int32#10,LSR #29
# asm 2: eor >hitmp=r9,<hitmp=r11,<lou4=r9,LSR #29
eor r9,r11,r9,LSR #29

# qhasm: hitmp ^= (hiu4 << 3)
# asm 1: eor >hitmp=int32#2,<hitmp=int32#10,<hiu4=int32#2,LSL #3
# asm 2: eor >hitmp=r1,<hitmp=r9,<hiu4=r1,LSL #3
eor r1,r9,r1,LSL #3

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#2,<hiu1=int32#11,<hitmp=int32#2
# asm 2: adc >hiu1=r1,<hiu1=r10,<hitmp=r1
adc r1,r10,r1

# qhasm: lou4 = lod13
# asm 1: vmov >lou4=int32#10,<lod13=float32#27
# asm 2: vmov >lou4=r9,<lod13=s26
# copy-collector input: vmov r9,s26

# qhasm: hiu4 = hid13
# asm 1: vmov >hiu4=int32#11,<hid13=float32#28
# asm 2: vmov >hiu4=r10,<hid13=s27
# copy-collector input: vmov r10,s27

# qhasm: carry? lotmp = (lou4 unsigned>> 1)
# asm 1: movs >lotmp=int32#12,<lou4=int32#10,LSR #1
# asm 2: movs >lotmp=r11,<lou4=r9,LSR #1
# copy-collector output starts
vmov r9,r10,d13
# copy-collector output ends
movs r11,r9,LSR #1

# qhasm: hitmp = (carry,hiu4 unsigned>> 1)
# asm 1: mov >hitmp=int32#13,<hiu4=int32#11,RRX
# asm 2: mov >hitmp=r12,<hiu4=r10,RRX
mov r12,r10,RRX

# qhasm: lotmp ^= (hiu4 << 31)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#11,LSL #31
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r10,LSL #31
eor r11,r11,r10,LSL #31

# qhasm: lotmp ^= (lou4 unsigned>>7)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou4=int32#10,LSR #7
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou4=r9,LSR #7
eor r11,r11,r9,LSR #7

# qhasm: lotmp ^= (hiu4 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#11,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r10,LSL #25
eor r11,r11,r10,LSL #25

# qhasm: hitmp ^= (hiu4 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#11,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu4=r10,LSR #7
eor r12,r12,r10,LSR #7

# qhasm: lotmp ^= (lou4 unsigned>>8)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou4=int32#10,LSR #8
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou4=r9,LSR #8
eor r11,r11,r9,LSR #8

# qhasm: lotmp ^= (hiu4 << 24)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu4=int32#11,LSL #24
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu4=r10,LSL #24
eor r11,r11,r10,LSL #24

# qhasm: hitmp ^= (hiu4 unsigned>>8)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu4=int32#11,LSR #8
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu4=r10,LSR #8
eor r12,r12,r10,LSR #8

# qhasm: hitmp ^= (lou4 << 24)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou4=int32#10,LSL #24
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou4=r9,LSL #24
eor r12,r12,r9,LSL #24

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#5,<lou1=int32#5,<lotmp=int32#12
# asm 2: adds >lou1=r4,<lou1=r4,<lotmp=r11
adds r4,r4,r11

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#2,<hiu1=int32#2,<hitmp=int32#13
# asm 2: adc >hiu1=r1,<hiu1=r1,<hitmp=r12
adc r1,r1,r12

# qhasm: lou5 = lom13
# asm 1: ldr >lou5=int32#12,<lom13=stack32#30
# asm 2: ldr >lou5=r11,<lom13=[sp,#116]
# copy-collector input: ldr r11,[sp,#116]

# qhasm: hiu5 = him13
# asm 1: ldr >hiu5=int32#13,<him13=stack32#31
# asm 2: ldr >hiu5=r12,<him13=[sp,#120]
# copy-collector input: ldr r12,[sp,#120]

# qhasm: carry? lou1 += lou5
# asm 1: adds >lou1=int32#5,<lou1=int32#5,<lou5=int32#12
# asm 2: adds >lou1=r4,<lou1=r4,<lou5=r11
# copy-collector output starts
ldrd r11,r12,[sp,#116]
# copy-collector output ends
adds r4,r4,r11

# qhasm: hiu1 += hiu5 + carry
# asm 1: adc >hiu1=int32#2,<hiu1=int32#2,<hiu5=int32#13
# asm 2: adc >hiu1=r1,<hiu1=r1,<hiu5=r12
adc r1,r1,r12

# qhasm: lod13 = lou5
# asm 1: vmov >lod13=float32#27,<lou5=int32#12
# asm 2: vmov >lod13=s26,<lou5=r11
# copy-collector input: vmov s26,r11

# qhasm: hid13 = hiu5
# asm 1: vmov >hid13=float32#28,<hiu5=int32#13
# asm 2: vmov >hid13=s27,<hiu5=r12
# copy-collector input: vmov s27,r12

# qhasm: lom12 = lou1
# asm 1: str <lou1=int32#5,>lom12=stack32#28
# asm 2: str <lou1=r4,>lom12=[sp,#108]
# copy-collector input: str r4,[sp,#108]

# qhasm: him12 = hiu1
# asm 1: str <hiu1=int32#2,>him12=stack32#29
# asm 2: str <hiu1=r1,>him12=[sp,#112]
# copy-collector input: str r1,[sp,#112]

# qhasm: lotmp = (lou0 unsigned>> 6)
# asm 1: mov >lotmp=int32#12,<lou0=int32#3,LSR #6
# asm 2: mov >lotmp=r11,<lou0=r2,LSR #6
# copy-collector output starts
vmov d13,r11,r12
strd r4,r1,[sp,#108]
# copy-collector output ends
mov r11,r2,LSR #6

# qhasm: lotmp ^= (hiu0 << 26)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#4,LSL #26
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r3,LSL #26
eor r11,r11,r3,LSL #26

# qhasm: lotmp ^= (lou0 unsigned>> 19)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#3,LSR #19
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r2,LSR #19
eor r11,r11,r2,LSR #19

# qhasm: lotmp ^= (hiu0 << 13)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#4,LSL #13
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r3,LSL #13
eor r11,r11,r3,LSL #13

# qhasm: lotmp ^= (hiu0 unsigned>> 29)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#4,LSR #29
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r3,LSR #29
eor r11,r11,r3,LSR #29

# qhasm: lotmp ^= (lou0 << 3)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#3,LSL #3
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r2,LSL #3
eor r11,r11,r2,LSL #3

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lotmp=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lotmp=r11
adds r9,r9,r11

# qhasm: hitmp = (hiu0 unsigned>> 6)
# asm 1: mov >hitmp=int32#12,<hiu0=int32#4,LSR #6
# asm 2: mov >hitmp=r11,<hiu0=r3,LSR #6
mov r11,r3,LSR #6

# qhasm: hitmp ^= (hiu0 unsigned>> 19)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<hiu0=int32#4,LSR #19
# asm 2: eor >hitmp=r11,<hitmp=r11,<hiu0=r3,LSR #19
eor r11,r11,r3,LSR #19

# qhasm: hitmp ^= (lou0 << 13)
# asm 1: eor >hitmp=int32#12,<hitmp=int32#12,<lou0=int32#3,LSL #13
# asm 2: eor >hitmp=r11,<hitmp=r11,<lou0=r2,LSL #13
eor r11,r11,r2,LSL #13

# qhasm: hitmp ^= (lou0 unsigned>> 29)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#12,<lou0=int32#3,LSR #29
# asm 2: eor >hitmp=r2,<hitmp=r11,<lou0=r2,LSR #29
eor r2,r11,r2,LSR #29

# qhasm: hitmp ^= (hiu0 << 3)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#3,<hiu0=int32#4,LSL #3
# asm 2: eor >hitmp=r2,<hitmp=r2,<hiu0=r3,LSL #3
eor r2,r2,r3,LSL #3

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#11,<hitmp=int32#3
# asm 2: adc >hiu4=r2,<hiu4=r10,<hitmp=r2
adc r2,r10,r2

# qhasm: lou0 = lod14
# asm 1: vmov >lou0=int32#4,<lod14=float32#29
# asm 2: vmov >lou0=r3,<lod14=s28
# copy-collector input: vmov r3,s28

# qhasm: hiu0 = hid14
# asm 1: vmov >hiu0=int32#11,<hid14=float32#30
# asm 2: vmov >hiu0=r10,<hid14=s29
# copy-collector input: vmov r10,s29

# qhasm: lod14 = lou2
# asm 1: vmov >lod14=float32#29,<lou2=int32#6
# asm 2: vmov >lod14=s28,<lou2=r5
# copy-collector input: vmov s28,r5

# qhasm: hid14 = hiu2
# asm 1: vmov >hid14=float32#30,<hiu2=int32#7
# asm 2: vmov >hid14=s29,<hiu2=r6
# copy-collector input: vmov s29,r6

# qhasm: carry? lotmp = (lou0 unsigned>> 1)
# asm 1: movs >lotmp=int32#12,<lou0=int32#4,LSR #1
# asm 2: movs >lotmp=r11,<lou0=r3,LSR #1
# copy-collector output starts
vmov r3,r10,d14
vmov d14,r5,r6
# copy-collector output ends
movs r11,r3,LSR #1

# qhasm: hitmp = (carry,hiu0 unsigned>> 1)
# asm 1: mov >hitmp=int32#13,<hiu0=int32#11,RRX
# asm 2: mov >hitmp=r12,<hiu0=r10,RRX
mov r12,r10,RRX

# qhasm: lotmp ^= (hiu0 << 31)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#11,LSL #31
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r10,LSL #31
eor r11,r11,r10,LSL #31

# qhasm: lotmp ^= (lou0 unsigned>>7)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#4,LSR #7
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r3,LSR #7
eor r11,r11,r3,LSR #7

# qhasm: lotmp ^= (hiu0 << 25)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#11,LSL #25
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r10,LSL #25
eor r11,r11,r10,LSL #25

# qhasm: hitmp ^= (hiu0 unsigned>>7)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#11,LSR #7
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r10,LSR #7
eor r12,r12,r10,LSR #7

# qhasm: lotmp ^= (lou0 unsigned>>8)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<lou0=int32#4,LSR #8
# asm 2: eor >lotmp=r11,<lotmp=r11,<lou0=r3,LSR #8
eor r11,r11,r3,LSR #8

# qhasm: lotmp ^= (hiu0 << 24)
# asm 1: eor >lotmp=int32#12,<lotmp=int32#12,<hiu0=int32#11,LSL #24
# asm 2: eor >lotmp=r11,<lotmp=r11,<hiu0=r10,LSL #24
eor r11,r11,r10,LSL #24

# qhasm: hitmp ^= (hiu0 unsigned>>8)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<hiu0=int32#11,LSR #8
# asm 2: eor >hitmp=r12,<hitmp=r12,<hiu0=r10,LSR #8
eor r12,r12,r10,LSR #8

# qhasm: hitmp ^= (lou0 << 24)
# asm 1: eor >hitmp=int32#13,<hitmp=int32#13,<lou0=int32#4,LSL #24
# asm 2: eor >hitmp=r12,<hitmp=r12,<lou0=r3,LSL #24
eor r12,r12,r3,LSL #24

# qhasm: carry? lou4 += lotmp
# asm 1: adds >lou4=int32#10,<lou4=int32#10,<lotmp=int32#12
# asm 2: adds >lou4=r9,<lou4=r9,<lotmp=r11
adds r9,r9,r11

# qhasm: hiu4 += hitmp + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hitmp=int32#13
# asm 2: adc >hiu4=r2,<hiu4=r2,<hitmp=r12
adc r2,r2,r12

# qhasm: carry? lou4 += lou2
# asm 1: adds >lou4=int32#6,<lou4=int32#10,<lou2=int32#6
# asm 2: adds >lou4=r5,<lou4=r9,<lou2=r5
adds r5,r9,r5

# qhasm: hiu4 += hiu2 + carry
# asm 1: adc >hiu4=int32#3,<hiu4=int32#3,<hiu2=int32#7
# asm 2: adc >hiu4=r2,<hiu4=r2,<hiu2=r6
adc r2,r2,r6

# qhasm: lom13 = lou4
# asm 1: str <lou4=int32#6,>lom13=stack32#30
# asm 2: str <lou4=r5,>lom13=[sp,#116]
# copy-collector input: str r5,[sp,#116]

# qhasm: him13 = hiu4
# asm 1: str <hiu4=int32#3,>him13=stack32#31
# asm 2: str <hiu4=r2,>him13=[sp,#120]
# copy-collector input: str r2,[sp,#120]

# qhasm: lotmp = (lou1 unsigned>> 6)
# asm 1: mov >lotmp=int32#7,<lou1=int32#5,LSR #6
# asm 2: mov >lotmp=r6,<lou1=r4,LSR #6
# copy-collector output starts
strd r5,r2,[sp,#116]
# copy-collector output ends
mov r6,r4,LSR #6

# qhasm: lotmp ^= (hiu1 << 26)
# asm 1: eor >lotmp=int32#7,<lotmp=int32#7,<hiu1=int32#2,LSL #26
# asm 2: eor >lotmp=r6,<lotmp=r6,<hiu1=r1,LSL #26
eor r6,r6,r1,LSL #26

# qhasm: lotmp ^= (lou1 unsigned>> 19)
# asm 1: eor >lotmp=int32#7,<lotmp=int32#7,<lou1=int32#5,LSR #19
# asm 2: eor >lotmp=r6,<lotmp=r6,<lou1=r4,LSR #19
eor r6,r6,r4,LSR #19

# qhasm: lotmp ^= (hiu1 << 13)
# asm 1: eor >lotmp=int32#7,<lotmp=int32#7,<hiu1=int32#2,LSL #13
# asm 2: eor >lotmp=r6,<lotmp=r6,<hiu1=r1,LSL #13
eor r6,r6,r1,LSL #13

# qhasm: lotmp ^= (hiu1 unsigned>> 29)
# asm 1: eor >lotmp=int32#7,<lotmp=int32#7,<hiu1=int32#2,LSR #29
# asm 2: eor >lotmp=r6,<lotmp=r6,<hiu1=r1,LSR #29
eor r6,r6,r1,LSR #29

# qhasm: lotmp ^= (lou1 << 3)
# asm 1: eor >lotmp=int32#7,<lotmp=int32#7,<lou1=int32#5,LSL #3
# asm 2: eor >lotmp=r6,<lotmp=r6,<lou1=r4,LSL #3
eor r6,r6,r4,LSL #3

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#4,<lou0=int32#4,<lotmp=int32#7
# asm 2: adds >lou0=r3,<lou0=r3,<lotmp=r6
adds r3,r3,r6

# qhasm: hitmp = (hiu1 unsigned>> 6)
# asm 1: mov >hitmp=int32#7,<hiu1=int32#2,LSR #6
# asm 2: mov >hitmp=r6,<hiu1=r1,LSR #6
mov r6,r1,LSR #6

# qhasm: hitmp ^= (hiu1 unsigned>> 19)
# asm 1: eor >hitmp=int32#7,<hitmp=int32#7,<hiu1=int32#2,LSR #19
# asm 2: eor >hitmp=r6,<hitmp=r6,<hiu1=r1,LSR #19
eor r6,r6,r1,LSR #19

# qhasm: hitmp ^= (lou1 << 13)
# asm 1: eor >hitmp=int32#7,<hitmp=int32#7,<lou1=int32#5,LSL #13
# asm 2: eor >hitmp=r6,<hitmp=r6,<lou1=r4,LSL #13
eor r6,r6,r4,LSL #13

# qhasm: hitmp ^= (lou1 unsigned>> 29)
# asm 1: eor >hitmp=int32#5,<hitmp=int32#7,<lou1=int32#5,LSR #29
# asm 2: eor >hitmp=r4,<hitmp=r6,<lou1=r4,LSR #29
eor r4,r6,r4,LSR #29

# qhasm: hitmp ^= (hiu1 << 3)
# asm 1: eor >hitmp=int32#2,<hitmp=int32#5,<hiu1=int32#2,LSL #3
# asm 2: eor >hitmp=r1,<hitmp=r4,<hiu1=r1,LSL #3
eor r1,r4,r1,LSL #3

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#2,<hiu0=int32#11,<hitmp=int32#2
# asm 2: adc >hiu0=r1,<hiu0=r10,<hitmp=r1
adc r1,r10,r1

# qhasm: lou1 = lod15
# asm 1: vmov >lou1=int32#5,<lod15=float32#31
# asm 2: vmov >lou1=r4,<lod15=s30
# copy-collector input: vmov r4,s30

# qhasm: hiu1 = hid15
# asm 1: vmov >hiu1=int32#7,<hid15=float32#32
# asm 2: vmov >hiu1=r6,<hid15=s31
# copy-collector input: vmov r6,s31

# qhasm: lod15 = lou3
# asm 1: vmov >lod15=float32#31,<lou3=int32#8
# asm 2: vmov >lod15=s30,<lou3=r7
# copy-collector input: vmov s30,r7

# qhasm: hid15 = hiu3
# asm 1: vmov >hid15=float32#32,<hiu3=int32#9
# asm 2: vmov >hid15=s31,<hiu3=r8
# copy-collector input: vmov s31,r8

# qhasm: carry? lotmp = (lou1 unsigned>> 1)
# asm 1: movs >lotmp=int32#10,<lou1=int32#5,LSR #1
# asm 2: movs >lotmp=r9,<lou1=r4,LSR #1
# copy-collector output starts
vmov r4,r6,d15
vmov d15,r7,r8
# copy-collector output ends
movs r9,r4,LSR #1

# qhasm: hitmp = (carry,hiu1 unsigned>> 1)
# asm 1: mov >hitmp=int32#11,<hiu1=int32#7,RRX
# asm 2: mov >hitmp=r10,<hiu1=r6,RRX
mov r10,r6,RRX

# qhasm: lotmp ^= (hiu1 << 31)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu1=int32#7,LSL #31
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu1=r6,LSL #31
eor r9,r9,r6,LSL #31

# qhasm: lotmp ^= (lou1 unsigned>>7)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou1=int32#5,LSR #7
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou1=r4,LSR #7
eor r9,r9,r4,LSR #7

# qhasm: lotmp ^= (hiu1 << 25)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu1=int32#7,LSL #25
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu1=r6,LSL #25
eor r9,r9,r6,LSL #25

# qhasm: hitmp ^= (hiu1 unsigned>>7)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#7,LSR #7
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu1=r6,LSR #7
eor r10,r10,r6,LSR #7

# qhasm: lotmp ^= (lou1 unsigned>>8)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<lou1=int32#5,LSR #8
# asm 2: eor >lotmp=r9,<lotmp=r9,<lou1=r4,LSR #8
eor r9,r9,r4,LSR #8

# qhasm: lotmp ^= (hiu1 << 24)
# asm 1: eor >lotmp=int32#10,<lotmp=int32#10,<hiu1=int32#7,LSL #24
# asm 2: eor >lotmp=r9,<lotmp=r9,<hiu1=r6,LSL #24
eor r9,r9,r6,LSL #24

# qhasm: hitmp ^= (hiu1 unsigned>>8)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<hiu1=int32#7,LSR #8
# asm 2: eor >hitmp=r10,<hitmp=r10,<hiu1=r6,LSR #8
eor r10,r10,r6,LSR #8

# qhasm: hitmp ^= (lou1 << 24)
# asm 1: eor >hitmp=int32#11,<hitmp=int32#11,<lou1=int32#5,LSL #24
# asm 2: eor >hitmp=r10,<hitmp=r10,<lou1=r4,LSL #24
eor r10,r10,r4,LSL #24

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#4,<lou0=int32#4,<lotmp=int32#10
# asm 2: adds >lou0=r3,<lou0=r3,<lotmp=r9
adds r3,r3,r9

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#2,<hiu0=int32#2,<hitmp=int32#11
# asm 2: adc >hiu0=r1,<hiu0=r1,<hitmp=r10
adc r1,r1,r10

# qhasm: carry? lou0 += lou3
# asm 1: adds >lou0=int32#4,<lou0=int32#4,<lou3=int32#8
# asm 2: adds >lou0=r3,<lou0=r3,<lou3=r7
adds r3,r3,r7

# qhasm: hiu0 += hiu3 + carry
# asm 1: adc >hiu0=int32#2,<hiu0=int32#2,<hiu3=int32#9
# asm 2: adc >hiu0=r1,<hiu0=r1,<hiu3=r8
adc r1,r1,r8

# qhasm: lom14 = lou0
# asm 1: str <lou0=int32#4,>lom14=stack32#32
# asm 2: str <lou0=r3,>lom14=[sp,#124]
# copy-collector input: str r3,[sp,#124]

# qhasm: him14 = hiu0
# asm 1: str <hiu0=int32#2,>him14=stack32#33
# asm 2: str <hiu0=r1,>him14=[sp,#128]
# copy-collector input: str r1,[sp,#128]

# qhasm: lotmp = (lou4 unsigned>> 6)
# asm 1: mov >lotmp=int32#2,<lou4=int32#6,LSR #6
# asm 2: mov >lotmp=r1,<lou4=r5,LSR #6
# copy-collector output starts
strd r3,r1,[sp,#124]
# copy-collector output ends
mov r1,r5,LSR #6

# qhasm: lotmp ^= (hiu4 << 26)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<hiu4=int32#3,LSL #26
# asm 2: eor >lotmp=r1,<lotmp=r1,<hiu4=r2,LSL #26
eor r1,r1,r2,LSL #26

# qhasm: lotmp ^= (lou4 unsigned>> 19)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lou4=int32#6,LSR #19
# asm 2: eor >lotmp=r1,<lotmp=r1,<lou4=r5,LSR #19
eor r1,r1,r5,LSR #19

# qhasm: lotmp ^= (hiu4 << 13)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<hiu4=int32#3,LSL #13
# asm 2: eor >lotmp=r1,<lotmp=r1,<hiu4=r2,LSL #13
eor r1,r1,r2,LSL #13

# qhasm: lotmp ^= (hiu4 unsigned>> 29)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<hiu4=int32#3,LSR #29
# asm 2: eor >lotmp=r1,<lotmp=r1,<hiu4=r2,LSR #29
eor r1,r1,r2,LSR #29

# qhasm: lotmp ^= (lou4 << 3)
# asm 1: eor >lotmp=int32#2,<lotmp=int32#2,<lou4=int32#6,LSL #3
# asm 2: eor >lotmp=r1,<lotmp=r1,<lou4=r5,LSL #3
eor r1,r1,r5,LSL #3

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#2,<lou1=int32#5,<lotmp=int32#2
# asm 2: adds >lou1=r1,<lou1=r4,<lotmp=r1
adds r1,r4,r1

# qhasm: hitmp = (hiu4 unsigned>> 6)
# asm 1: mov >hitmp=int32#4,<hiu4=int32#3,LSR #6
# asm 2: mov >hitmp=r3,<hiu4=r2,LSR #6
mov r3,r2,LSR #6

# qhasm: hitmp ^= (hiu4 unsigned>> 19)
# asm 1: eor >hitmp=int32#4,<hitmp=int32#4,<hiu4=int32#3,LSR #19
# asm 2: eor >hitmp=r3,<hitmp=r3,<hiu4=r2,LSR #19
eor r3,r3,r2,LSR #19

# qhasm: hitmp ^= (lou4 << 13)
# asm 1: eor >hitmp=int32#4,<hitmp=int32#4,<lou4=int32#6,LSL #13
# asm 2: eor >hitmp=r3,<hitmp=r3,<lou4=r5,LSL #13
eor r3,r3,r5,LSL #13

# qhasm: hitmp ^= (lou4 unsigned>> 29)
# asm 1: eor >hitmp=int32#4,<hitmp=int32#4,<lou4=int32#6,LSR #29
# asm 2: eor >hitmp=r3,<hitmp=r3,<lou4=r5,LSR #29
eor r3,r3,r5,LSR #29

# qhasm: hitmp ^= (hiu4 << 3)
# asm 1: eor >hitmp=int32#3,<hitmp=int32#4,<hiu4=int32#3,LSL #3
# asm 2: eor >hitmp=r2,<hitmp=r3,<hiu4=r2,LSL #3
eor r2,r3,r2,LSL #3

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#3,<hiu1=int32#7,<hitmp=int32#3
# asm 2: adc >hiu1=r2,<hiu1=r6,<hitmp=r2
adc r2,r6,r2

# qhasm: lou0 = lom8
# asm 1: ldr >lou0=int32#4,<lom8=stack32#20
# asm 2: ldr >lou0=r3,<lom8=[sp,#76]
# copy-collector input: ldr r3,[sp,#76]

# qhasm: hiu0 = him8
# asm 1: ldr >hiu0=int32#5,<him8=stack32#21
# asm 2: ldr >hiu0=r4,<him8=[sp,#80]
# copy-collector input: ldr r4,[sp,#80]

# qhasm: lou2 = lom15
# asm 1: ldr >lou2=int32#6,<lom15=stack32#34
# asm 2: ldr >lou2=r5,<lom15=[sp,#132]
# copy-collector input: ldr r5,[sp,#132]

# qhasm: hiu2 = him15
# asm 1: ldr >hiu2=int32#7,<him15=stack32#35
# asm 2: ldr >hiu2=r6,<him15=[sp,#136]
# copy-collector input: ldr r6,[sp,#136]

# qhasm: lom8 = lou2
# asm 1: str <lou2=int32#6,>lom8=stack32#20
# asm 2: str <lou2=r5,>lom8=[sp,#76]
# copy-collector input: str r5,[sp,#76]

# qhasm: him8 = hiu2
# asm 1: str <hiu2=int32#7,>him8=stack32#21
# asm 2: str <hiu2=r6,>him8=[sp,#80]
# copy-collector input: str r6,[sp,#80]

# qhasm: carry? lotmp = (lou0 unsigned>> 1)
# asm 1: movs >lotmp=int32#8,<lou0=int32#4,LSR #1
# asm 2: movs >lotmp=r7,<lou0=r3,LSR #1
# copy-collector output starts
ldr r3,[sp,#76]
ldr r4,[sp,#80]
ldr r5,[sp,#132]
ldr.w r6,[sp,#136]
strd r5,r6,[sp,#76]
# copy-collector output ends
movs r7,r3,LSR #1

# qhasm: hitmp = (carry,hiu0 unsigned>> 1)
# asm 1: mov >hitmp=int32#9,<hiu0=int32#5,RRX
# asm 2: mov >hitmp=r8,<hiu0=r4,RRX
mov r8,r4,RRX

# qhasm: lotmp ^= (hiu0 << 31)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<hiu0=int32#5,LSL #31
# asm 2: eor >lotmp=r7,<lotmp=r7,<hiu0=r4,LSL #31
eor r7,r7,r4,LSL #31

# qhasm: lotmp ^= (lou0 unsigned>>7)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<lou0=int32#4,LSR #7
# asm 2: eor >lotmp=r7,<lotmp=r7,<lou0=r3,LSR #7
eor r7,r7,r3,LSR #7

# qhasm: lotmp ^= (hiu0 << 25)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<hiu0=int32#5,LSL #25
# asm 2: eor >lotmp=r7,<lotmp=r7,<hiu0=r4,LSL #25
eor r7,r7,r4,LSL #25

# qhasm: hitmp ^= (hiu0 unsigned>>7)
# asm 1: eor >hitmp=int32#9,<hitmp=int32#9,<hiu0=int32#5,LSR #7
# asm 2: eor >hitmp=r8,<hitmp=r8,<hiu0=r4,LSR #7
eor r8,r8,r4,LSR #7

# qhasm: lotmp ^= (lou0 unsigned>>8)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<lou0=int32#4,LSR #8
# asm 2: eor >lotmp=r7,<lotmp=r7,<lou0=r3,LSR #8
eor r7,r7,r3,LSR #8

# qhasm: lotmp ^= (hiu0 << 24)
# asm 1: eor >lotmp=int32#8,<lotmp=int32#8,<hiu0=int32#5,LSL #24
# asm 2: eor >lotmp=r7,<lotmp=r7,<hiu0=r4,LSL #24
eor r7,r7,r4,LSL #24

# qhasm: hitmp ^= (hiu0 unsigned>>8)
# asm 1: eor >hitmp=int32#9,<hitmp=int32#9,<hiu0=int32#5,LSR #8
# asm 2: eor >hitmp=r8,<hitmp=r8,<hiu0=r4,LSR #8
eor r8,r8,r4,LSR #8

# qhasm: hitmp ^= (lou0 << 24)
# asm 1: eor >hitmp=int32#9,<hitmp=int32#9,<lou0=int32#4,LSL #24
# asm 2: eor >hitmp=r8,<hitmp=r8,<lou0=r3,LSL #24
eor r8,r8,r3,LSL #24

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#2,<lou1=int32#2,<lotmp=int32#8
# asm 2: adds >lou1=r1,<lou1=r1,<lotmp=r7
adds r1,r1,r7

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#3,<hiu1=int32#3,<hitmp=int32#9
# asm 2: adc >hiu1=r2,<hiu1=r2,<hitmp=r8
adc r2,r2,r8

# qhasm: carry? lou1 += lou2
# asm 1: adds >lou1=int32#2,<lou1=int32#2,<lou2=int32#6
# asm 2: adds >lou1=r1,<lou1=r1,<lou2=r5
adds.w r1,r1,r5

# qhasm: hiu1 += hiu2 + carry
# asm 1: adc >hiu1=int32#3,<hiu1=int32#3,<hiu2=int32#7
# asm 2: adc >hiu1=r2,<hiu1=r2,<hiu2=r6
adc r2,r2,r6

# qhasm: lom15 = lou1
# asm 1: str <lou1=int32#2,>lom15=stack32#34
# asm 2: str <lou1=r1,>lom15=[sp,#132]
# copy-collector input: str r1,[sp,#132]

# qhasm: him15 = hiu1
# asm 1: str <hiu1=int32#3,>him15=stack32#35
# asm 2: str <hiu1=r2,>him15=[sp,#136]
# copy-collector input: str r2,[sp,#136]

# qhasm: lod8 = lou0
# asm 1: vmov >lod8=float32#17,<lou0=int32#4
# asm 2: vmov >lod8=s16,<lou0=r3
# copy-collector input: vmov s16,r3

# qhasm: hid8 = hiu0
# asm 1: vmov >hid8=float32#18,<hiu0=int32#5
# asm 2: vmov >hid8=s17,<hiu0=r4
# copy-collector input: vmov s17,r4

# qhasm: goto innerloop
# copy-collector output starts
strd r1,r2,[sp,#132]
vmov d8,r3,r4
# copy-collector output ends
b ._innerloop

# qhasm: nearend:
._nearend:

# qhasm: lou0 = lom8
# asm 1: ldr >lou0=int32#2,<lom8=stack32#20
# asm 2: ldr >lou0=r1,<lom8=[sp,#76]
# copy-collector input: ldr r1,[sp,#76]

# qhasm: hiu0 = him8
# asm 1: ldr >hiu0=int32#3,<him8=stack32#21
# asm 2: ldr >hiu0=r2,<him8=[sp,#80]
# copy-collector input: ldr r2,[sp,#80]

# qhasm: lou1 = lom9
# asm 1: ldr >lou1=int32#4,<lom9=stack32#22
# asm 2: ldr >lou1=r3,<lom9=[sp,#84]
# copy-collector input: ldr r3,[sp,#84]

# qhasm: hiu1 = him9
# asm 1: ldr >hiu1=int32#5,<him9=stack32#23
# asm 2: ldr >hiu1=r4,<him9=[sp,#88]
# copy-collector input: ldr r4,[sp,#88]

# qhasm: lou2 = lom10
# asm 1: ldr >lou2=int32#6,<lom10=stack32#24
# asm 2: ldr >lou2=r5,<lom10=[sp,#92]
# copy-collector input: ldr r5,[sp,#92]

# qhasm: hiu2 = him10
# asm 1: ldr >hiu2=int32#7,<him10=stack32#25
# asm 2: ldr >hiu2=r6,<him10=[sp,#96]
# copy-collector input: ldr r6,[sp,#96]

# qhasm: lou3 = lom11
# asm 1: ldr >lou3=int32#8,<lom11=stack32#26
# asm 2: ldr >lou3=r7,<lom11=[sp,#100]
# copy-collector input: ldr r7,[sp,#100]

# qhasm: hiu3 = him11
# asm 1: ldr >hiu3=int32#9,<him11=stack32#27
# asm 2: ldr >hiu3=r8,<him11=[sp,#104]
# copy-collector input: ldr r8,[sp,#104]

# qhasm: lod8 = lou0
# asm 1: vmov >lod8=float32#17,<lou0=int32#2
# asm 2: vmov >lod8=s16,<lou0=r1
# copy-collector input: vmov s16,r1

# qhasm: hid8 = hiu0
# asm 1: vmov >hid8=float32#18,<hiu0=int32#3
# asm 2: vmov >hid8=s17,<hiu0=r2
# copy-collector input: vmov s17,r2

# qhasm: lod9 = lou1
# asm 1: vmov >lod9=float32#19,<lou1=int32#4
# asm 2: vmov >lod9=s18,<lou1=r3
# copy-collector input: vmov s18,r3

# qhasm: hid9 = hiu1
# asm 1: vmov >hid9=float32#20,<hiu1=int32#5
# asm 2: vmov >hid9=s19,<hiu1=r4
# copy-collector input: vmov s19,r4

# qhasm: lod10 = lou2
# asm 1: vmov >lod10=float32#21,<lou2=int32#6
# asm 2: vmov >lod10=s20,<lou2=r5
# copy-collector input: vmov s20,r5

# qhasm: hid10 = hiu2
# asm 1: vmov >hid10=float32#22,<hiu2=int32#7
# asm 2: vmov >hid10=s21,<hiu2=r6
# copy-collector input: vmov s21,r6

# qhasm: lod11 = lou3
# asm 1: vmov >lod11=float32#23,<lou3=int32#8
# asm 2: vmov >lod11=s22,<lou3=r7
# copy-collector input: vmov s22,r7

# qhasm: hid11 = hiu3
# asm 1: vmov >hid11=float32#24,<hiu3=int32#9
# asm 2: vmov >hid11=s23,<hiu3=r8
# copy-collector input: vmov s23,r8

# qhasm: lou0 = lom12
# asm 1: ldr >lou0=int32#2,<lom12=stack32#28
# asm 2: ldr >lou0=r1,<lom12=[sp,#108]
# copy-collector input: ldr r1,[sp,#108]

# qhasm: hiu0 = him12
# asm 1: ldr >hiu0=int32#3,<him12=stack32#29
# asm 2: ldr >hiu0=r2,<him12=[sp,#112]
# copy-collector input: ldr r2,[sp,#112]

# qhasm: lou1 = lom13
# asm 1: ldr >lou1=int32#4,<lom13=stack32#30
# asm 2: ldr >lou1=r3,<lom13=[sp,#116]
# copy-collector input: ldr r3,[sp,#116]

# qhasm: hiu1 = him13
# asm 1: ldr >hiu1=int32#5,<him13=stack32#31
# asm 2: ldr >hiu1=r4,<him13=[sp,#120]
# copy-collector input: ldr r4,[sp,#120]

# qhasm: lou2 = lom14
# asm 1: ldr >lou2=int32#6,<lom14=stack32#32
# asm 2: ldr >lou2=r5,<lom14=[sp,#124]
# copy-collector input: ldr r5,[sp,#124]

# qhasm: hiu2 = him14
# asm 1: ldr >hiu2=int32#7,<him14=stack32#33
# asm 2: ldr >hiu2=r6,<him14=[sp,#128]
# copy-collector input: ldr r6,[sp,#128]

# qhasm: lou3 = lom15
# asm 1: ldr >lou3=int32#8,<lom15=stack32#34
# asm 2: ldr >lou3=r7,<lom15=[sp,#132]
# copy-collector input: ldr r7,[sp,#132]

# qhasm: hiu3 = him15
# asm 1: ldr >hiu3=int32#9,<him15=stack32#35
# asm 2: ldr >hiu3=r8,<him15=[sp,#136]
# copy-collector input: ldr r8,[sp,#136]

# qhasm: lod12 = lou0
# asm 1: vmov >lod12=float32#25,<lou0=int32#2
# asm 2: vmov >lod12=s24,<lou0=r1
# copy-collector input: vmov s24,r1

# qhasm: hid12 = hiu0
# asm 1: vmov >hid12=float32#26,<hiu0=int32#3
# asm 2: vmov >hid12=s25,<hiu0=r2
# copy-collector input: vmov s25,r2

# qhasm: lod13 = lou1
# asm 1: vmov >lod13=float32#27,<lou1=int32#4
# asm 2: vmov >lod13=s26,<lou1=r3
# copy-collector input: vmov s26,r3

# qhasm: hid13 = hiu1
# asm 1: vmov >hid13=float32#28,<hiu1=int32#5
# asm 2: vmov >hid13=s27,<hiu1=r4
# copy-collector input: vmov s27,r4

# qhasm: lod14 = lou2
# asm 1: vmov >lod14=float32#29,<lou2=int32#6
# asm 2: vmov >lod14=s28,<lou2=r5
# copy-collector input: vmov s28,r5

# qhasm: hid14 = hiu2
# asm 1: vmov >hid14=float32#30,<hiu2=int32#7
# asm 2: vmov >hid14=s29,<hiu2=r6
# copy-collector input: vmov s29,r6

# qhasm: lod15 = lou3
# asm 1: vmov >lod15=float32#31,<lou3=int32#8
# asm 2: vmov >lod15=s30,<lou3=r7
# copy-collector input: vmov s30,r7

# qhasm: hid15 = hiu3
# asm 1: vmov >hid15=float32#32,<hiu3=int32#9
# asm 2: vmov >hid15=s31,<hiu3=r8
# copy-collector input: vmov s31,r8

# qhasm: goto innerloop
# copy-collector output starts
ldr r1,[sp,#76]
ldr r2,[sp,#80]
ldr r3,[sp,#84]
ldr r4,[sp,#88]
ldr r5,[sp,#92]
ldr r6,[sp,#96]
ldr.w r7,[sp,#100]
ldr r8,[sp,#104]
vmov d8,r1,r2
vmov d9,r3,r4
vmov d10,r5,r6
vmov d11,r7,r8
ldr r1,[sp,#108]
ldr r2,[sp,#112]
ldr r3,[sp,#116]
ldr r4,[sp,#120]
ldr r5,[sp,#124]
ldr r6,[sp,#128]
ldr.w r7,[sp,#132]
ldr r8,[sp,#136]
vmov d12,r1,r2
vmov d13,r3,r4
vmov d14,r5,r6
vmov d15,r7,r8
# copy-collector output ends
b ._innerloop

# qhasm: endinnerloop:
._endinnerloop:

# qhasm: input_1 -= 640
# asm 1: sub >input_1=int32#1,<input_1=int32#1,#640
# asm 2: sub >input_1=r0,<input_1=r0,#640
sub r0,r0,#640

# qhasm: lou0 = lod0
# asm 1: vmov >lou0=int32#2,<lod0=float32#1
# asm 2: vmov >lou0=r1,<lod0=s0
# copy-collector input: vmov r1,s0

# qhasm: hiu0 = hid0
# asm 1: vmov >hiu0=int32#3,<hid0=float32#2
# asm 2: vmov >hiu0=r2,<hid0=s1
# copy-collector input: vmov r2,s1

# qhasm: lou1 = lod1
# asm 1: vmov >lou1=int32#4,<lod1=float32#3
# asm 2: vmov >lou1=r3,<lod1=s2
# copy-collector input: vmov r3,s2

# qhasm: hiu1 = hid1
# asm 1: vmov >hiu1=int32#5,<hid1=float32#4
# asm 2: vmov >hiu1=r4,<hid1=s3
# copy-collector input: vmov r4,s3

# qhasm: lou2 = lod2
# asm 1: vmov >lou2=int32#6,<lod2=float32#5
# asm 2: vmov >lou2=r5,<lod2=s4
# copy-collector input: vmov r5,s4

# qhasm: hiu2 = hid2
# asm 1: vmov >hiu2=int32#7,<hid2=float32#6
# asm 2: vmov >hiu2=r6,<hid2=s5
# copy-collector input: vmov r6,s5

# qhasm: lou3 = lod3
# asm 1: vmov >lou3=int32#8,<lod3=float32#7
# asm 2: vmov >lou3=r7,<lod3=s6
# copy-collector input: vmov r7,s6

# qhasm: hiu3 = hid3
# asm 1: vmov >hiu3=int32#9,<hid3=float32#8
# asm 2: vmov >hiu3=r8,<hid3=s7
# copy-collector input: vmov r8,s7

# qhasm: lotmp = lom0
# asm 1: ldr >lotmp=int32#10,<lom0=stack32#4
# asm 2: ldr >lotmp=r9,<lom0=[sp,#12]
# copy-collector input: ldr r9,[sp,#12]

# qhasm: hitmp = him0
# asm 1: ldr >hitmp=int32#11,<him0=stack32#5
# asm 2: ldr >hitmp=r10,<him0=[sp,#16]
# copy-collector input: ldr r10,[sp,#16]

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#2,<lou0=int32#2,<lotmp=int32#10
# asm 2: adds >lou0=r1,<lou0=r1,<lotmp=r9
# copy-collector output starts
vmov r1,r2,d0
vmov r3,r4,d1
vmov r5,r6,d2
vmov r7,r8,d3
ldrd r9,r10,[sp,#12]
# copy-collector output ends
adds r1,r1,r9

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hitmp=int32#11
# asm 2: adc >hiu0=r2,<hiu0=r2,<hitmp=r10
adc r2,r2,r10

# qhasm: lotmp = lom1
# asm 1: ldr >lotmp=int32#10,<lom1=stack32#6
# asm 2: ldr >lotmp=r9,<lom1=[sp,#20]
# copy-collector input: ldr r9,[sp,#20]

# qhasm: hitmp = him1
# asm 1: ldr >hitmp=int32#11,<him1=stack32#7
# asm 2: ldr >hitmp=r10,<him1=[sp,#24]
# copy-collector input: ldr r10,[sp,#24]

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#4,<lou1=int32#4,<lotmp=int32#10
# asm 2: adds >lou1=r3,<lou1=r3,<lotmp=r9
# copy-collector output starts
ldrd r9,r10,[sp,#20]
# copy-collector output ends
adds r3,r3,r9

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#5,<hiu1=int32#5,<hitmp=int32#11
# asm 2: adc >hiu1=r4,<hiu1=r4,<hitmp=r10
adc r4,r4,r10

# qhasm: lotmp = lom2
# asm 1: ldr >lotmp=int32#10,<lom2=stack32#8
# asm 2: ldr >lotmp=r9,<lom2=[sp,#28]
# copy-collector input: ldr r9,[sp,#28]

# qhasm: hitmp = him2
# asm 1: ldr >hitmp=int32#11,<him2=stack32#9
# asm 2: ldr >hitmp=r10,<him2=[sp,#32]
# copy-collector input: ldr r10,[sp,#32]

# qhasm: carry? lou2 += lotmp
# asm 1: adds >lou2=int32#6,<lou2=int32#6,<lotmp=int32#10
# asm 2: adds >lou2=r5,<lou2=r5,<lotmp=r9
# copy-collector output starts
ldrd r9,r10,[sp,#28]
# copy-collector output ends
adds r5,r5,r9

# qhasm: hiu2 += hitmp + carry
# asm 1: adc >hiu2=int32#7,<hiu2=int32#7,<hitmp=int32#11
# asm 2: adc >hiu2=r6,<hiu2=r6,<hitmp=r10
adc r6,r6,r10

# qhasm: lotmp = lom3
# asm 1: ldr >lotmp=int32#10,<lom3=stack32#10
# asm 2: ldr >lotmp=r9,<lom3=[sp,#36]
# copy-collector input: ldr r9,[sp,#36]

# qhasm: hitmp = him3
# asm 1: ldr >hitmp=int32#11,<him3=stack32#11
# asm 2: ldr >hitmp=r10,<him3=[sp,#40]
# copy-collector input: ldr r10,[sp,#40]

# qhasm: carry? lou3 += lotmp
# asm 1: adds >lou3=int32#8,<lou3=int32#8,<lotmp=int32#10
# asm 2: adds >lou3=r7,<lou3=r7,<lotmp=r9
# copy-collector output starts
ldrd r9,r10,[sp,#36]
# copy-collector output ends
adds r7,r7,r9

# qhasm: hiu3 += hitmp + carry
# asm 1: adc >hiu3=int32#9,<hiu3=int32#9,<hitmp=int32#11
# asm 2: adc >hiu3=r8,<hiu3=r8,<hitmp=r10
adc r8,r8,r10

# qhasm: lom0 = lou0
# asm 1: str <lou0=int32#2,>lom0=stack32#4
# asm 2: str <lou0=r1,>lom0=[sp,#12]
# copy-collector input: str r1,[sp,#12]

# qhasm: him0 = hiu0
# asm 1: str <hiu0=int32#3,>him0=stack32#5
# asm 2: str <hiu0=r2,>him0=[sp,#16]
# copy-collector input: str r2,[sp,#16]

# qhasm: lom1 = lou1
# asm 1: str <lou1=int32#4,>lom1=stack32#6
# asm 2: str <lou1=r3,>lom1=[sp,#20]
# copy-collector input: str r3,[sp,#20]

# qhasm: him1 = hiu1
# asm 1: str <hiu1=int32#5,>him1=stack32#7
# asm 2: str <hiu1=r4,>him1=[sp,#24]
# copy-collector input: str r4,[sp,#24]

# qhasm: lom2 = lou2
# asm 1: str <lou2=int32#6,>lom2=stack32#8
# asm 2: str <lou2=r5,>lom2=[sp,#28]
# copy-collector input: str r5,[sp,#28]

# qhasm: him2 = hiu2
# asm 1: str <hiu2=int32#7,>him2=stack32#9
# asm 2: str <hiu2=r6,>him2=[sp,#32]
# copy-collector input: str r6,[sp,#32]

# qhasm: lom3 = lou3
# asm 1: str <lou3=int32#8,>lom3=stack32#10
# asm 2: str <lou3=r7,>lom3=[sp,#36]
# copy-collector input: str r7,[sp,#36]

# qhasm: him3 = hiu3
# asm 1: str <hiu3=int32#9,>him3=stack32#11
# asm 2: str <hiu3=r8,>him3=[sp,#40]
# copy-collector input: str r8,[sp,#40]

# qhasm: lod0 = lou0
# asm 1: vmov >lod0=float32#1,<lou0=int32#2
# asm 2: vmov >lod0=s0,<lou0=r1
# copy-collector input: vmov s0,r1

# qhasm: hid0 = hiu0
# asm 1: vmov >hid0=float32#2,<hiu0=int32#3
# asm 2: vmov >hid0=s1,<hiu0=r2
# copy-collector input: vmov s1,r2

# qhasm: lod1 = lou1
# asm 1: vmov >lod1=float32#3,<lou1=int32#4
# asm 2: vmov >lod1=s2,<lou1=r3
# copy-collector input: vmov s2,r3

# qhasm: hid1 = hiu1
# asm 1: vmov >hid1=float32#4,<hiu1=int32#5
# asm 2: vmov >hid1=s3,<hiu1=r4
# copy-collector input: vmov s3,r4

# qhasm: lod2 = lou2
# asm 1: vmov >lod2=float32#5,<lou2=int32#6
# asm 2: vmov >lod2=s4,<lou2=r5
# copy-collector input: vmov s4,r5

# qhasm: hid2 = hiu2
# asm 1: vmov >hid2=float32#6,<hiu2=int32#7
# asm 2: vmov >hid2=s5,<hiu2=r6
# copy-collector input: vmov s5,r6

# qhasm: lod3 = lou3
# asm 1: vmov >lod3=float32#7,<lou3=int32#8
# asm 2: vmov >lod3=s6,<lou3=r7
# copy-collector input: vmov s6,r7

# qhasm: hid3 = hiu3
# asm 1: vmov >hid3=float32#8,<hiu3=int32#9
# asm 2: vmov >hid3=s7,<hiu3=r8
# copy-collector input: vmov s7,r8

# qhasm: lou0 = lod4
# asm 1: vmov >lou0=int32#2,<lod4=float32#9
# asm 2: vmov >lou0=r1,<lod4=s8
# copy-collector input: vmov r1,s8

# qhasm: hiu0 = hid4
# asm 1: vmov >hiu0=int32#3,<hid4=float32#10
# asm 2: vmov >hiu0=r2,<hid4=s9
# copy-collector input: vmov r2,s9

# qhasm: lou1 = lod5
# asm 1: vmov >lou1=int32#4,<lod5=float32#11
# asm 2: vmov >lou1=r3,<lod5=s10
# copy-collector input: vmov r3,s10

# qhasm: hiu1 = hid5
# asm 1: vmov >hiu1=int32#5,<hid5=float32#12
# asm 2: vmov >hiu1=r4,<hid5=s11
# copy-collector input: vmov r4,s11

# qhasm: lou2 = lod6
# asm 1: vmov >lou2=int32#6,<lod6=float32#13
# asm 2: vmov >lou2=r5,<lod6=s12
# copy-collector input: vmov r5,s12

# qhasm: hiu2 = hid6
# asm 1: vmov >hiu2=int32#7,<hid6=float32#14
# asm 2: vmov >hiu2=r6,<hid6=s13
# copy-collector input: vmov r6,s13

# qhasm: lou3 = lod7
# asm 1: vmov >lou3=int32#8,<lod7=float32#15
# asm 2: vmov >lou3=r7,<lod7=s14
# copy-collector input: vmov r7,s14

# qhasm: hiu3 = hid7
# asm 1: vmov >hiu3=int32#9,<hid7=float32#16
# asm 2: vmov >hiu3=r8,<hid7=s15
# copy-collector input: vmov r8,s15

# qhasm: lotmp = lom4
# asm 1: ldr >lotmp=int32#10,<lom4=stack32#12
# asm 2: ldr >lotmp=r9,<lom4=[sp,#44]
# copy-collector input: ldr r9,[sp,#44]

# qhasm: hitmp = him4
# asm 1: ldr >hitmp=int32#11,<him4=stack32#13
# asm 2: ldr >hitmp=r10,<him4=[sp,#48]
# copy-collector input: ldr r10,[sp,#48]

# qhasm: carry? lou0 += lotmp
# asm 1: adds >lou0=int32#2,<lou0=int32#2,<lotmp=int32#10
# asm 2: adds >lou0=r1,<lou0=r1,<lotmp=r9
# copy-collector output starts
strd r1,r2,[sp,#12]
strd r3,r4,[sp,#20]
strd r5,r6,[sp,#28]
strd r7,r8,[sp,#36]
vmov d0,r1,r2
vmov d1,r3,r4
vmov d2,r5,r6
vmov d3,r7,r8
vmov r1,r2,d4
vmov r3,r4,d5
vmov r5,r6,d6
vmov r7,r8,d7
ldrd r9,r10,[sp,#44]
# copy-collector output ends
adds r1,r1,r9

# qhasm: hiu0 += hitmp + carry
# asm 1: adc >hiu0=int32#3,<hiu0=int32#3,<hitmp=int32#11
# asm 2: adc >hiu0=r2,<hiu0=r2,<hitmp=r10
adc r2,r2,r10

# qhasm: lotmp = lom5
# asm 1: ldr >lotmp=int32#10,<lom5=stack32#14
# asm 2: ldr >lotmp=r9,<lom5=[sp,#52]
# copy-collector input: ldr r9,[sp,#52]

# qhasm: hitmp = him5
# asm 1: ldr >hitmp=int32#11,<him5=stack32#15
# asm 2: ldr >hitmp=r10,<him5=[sp,#56]
# copy-collector input: ldr r10,[sp,#56]

# qhasm: carry? lou1 += lotmp
# asm 1: adds >lou1=int32#4,<lou1=int32#4,<lotmp=int32#10
# asm 2: adds >lou1=r3,<lou1=r3,<lotmp=r9
# copy-collector output starts
ldrd r9,r10,[sp,#52]
# copy-collector output ends
adds r3,r3,r9

# qhasm: hiu1 += hitmp + carry
# asm 1: adc >hiu1=int32#5,<hiu1=int32#5,<hitmp=int32#11
# asm 2: adc >hiu1=r4,<hiu1=r4,<hitmp=r10
adc r4,r4,r10

# qhasm: lotmp = lom6
# asm 1: ldr >lotmp=int32#10,<lom6=stack32#16
# asm 2: ldr >lotmp=r9,<lom6=[sp,#60]
# copy-collector input: ldr r9,[sp,#60]

# qhasm: hitmp = him6
# asm 1: ldr >hitmp=int32#11,<him6=stack32#17
# asm 2: ldr >hitmp=r10,<him6=[sp,#64]
# copy-collector input: ldr r10,[sp,#64]

# qhasm: carry? lou2 += lotmp
# asm 1: adds >lou2=int32#6,<lou2=int32#6,<lotmp=int32#10
# asm 2: adds >lou2=r5,<lou2=r5,<lotmp=r9
# copy-collector output starts
ldrd r9,r10,[sp,#60]
# copy-collector output ends
adds r5,r5,r9

# qhasm: hiu2 += hitmp + carry
# asm 1: adc >hiu2=int32#7,<hiu2=int32#7,<hitmp=int32#11
# asm 2: adc >hiu2=r6,<hiu2=r6,<hitmp=r10
adc r6,r6,r10

# qhasm: lotmp = lom7
# asm 1: ldr >lotmp=int32#10,<lom7=stack32#18
# asm 2: ldr >lotmp=r9,<lom7=[sp,#68]
# copy-collector input: ldr r9,[sp,#68]

# qhasm: hitmp = him7
# asm 1: ldr >hitmp=int32#11,<him7=stack32#19
# asm 2: ldr >hitmp=r10,<him7=[sp,#72]
# copy-collector input: ldr r10,[sp,#72]

# qhasm: carry? lou3 += lotmp
# asm 1: adds >lou3=int32#8,<lou3=int32#8,<lotmp=int32#10
# asm 2: adds >lou3=r7,<lou3=r7,<lotmp=r9
# copy-collector output starts
ldrd r9,r10,[sp,#68]
# copy-collector output ends
adds r7,r7,r9

# qhasm: hiu3 += hitmp + carry
# asm 1: adc >hiu3=int32#9,<hiu3=int32#9,<hitmp=int32#11
# asm 2: adc >hiu3=r8,<hiu3=r8,<hitmp=r10
adc r8,r8,r10

# qhasm: lom4 = lou0
# asm 1: str <lou0=int32#2,>lom4=stack32#12
# asm 2: str <lou0=r1,>lom4=[sp,#44]
# copy-collector input: str r1,[sp,#44]

# qhasm: him4 = hiu0
# asm 1: str <hiu0=int32#3,>him4=stack32#13
# asm 2: str <hiu0=r2,>him4=[sp,#48]
# copy-collector input: str r2,[sp,#48]

# qhasm: lom5 = lou1
# asm 1: str <lou1=int32#4,>lom5=stack32#14
# asm 2: str <lou1=r3,>lom5=[sp,#52]
# copy-collector input: str r3,[sp,#52]

# qhasm: him5 = hiu1
# asm 1: str <hiu1=int32#5,>him5=stack32#15
# asm 2: str <hiu1=r4,>him5=[sp,#56]
# copy-collector input: str r4,[sp,#56]

# qhasm: lom6 = lou2
# asm 1: str <lou2=int32#6,>lom6=stack32#16
# asm 2: str <lou2=r5,>lom6=[sp,#60]
# copy-collector input: str r5,[sp,#60]

# qhasm: him6 = hiu2
# asm 1: str <hiu2=int32#7,>him6=stack32#17
# asm 2: str <hiu2=r6,>him6=[sp,#64]
# copy-collector input: str r6,[sp,#64]

# qhasm: lom7 = lou3
# asm 1: str <lou3=int32#8,>lom7=stack32#18
# asm 2: str <lou3=r7,>lom7=[sp,#68]
# copy-collector input: str r7,[sp,#68]

# qhasm: him7 = hiu3
# asm 1: str <hiu3=int32#9,>him7=stack32#19
# asm 2: str <hiu3=r8,>him7=[sp,#72]
# copy-collector input: str r8,[sp,#72]

# qhasm: lod4 = lou0
# asm 1: vmov >lod4=float32#9,<lou0=int32#2
# asm 2: vmov >lod4=s8,<lou0=r1
# copy-collector input: vmov s8,r1

# qhasm: hid4 = hiu0
# asm 1: vmov >hid4=float32#10,<hiu0=int32#3
# asm 2: vmov >hid4=s9,<hiu0=r2
# copy-collector input: vmov s9,r2

# qhasm: lod5 = lou1
# asm 1: vmov >lod5=float32#11,<lou1=int32#4
# asm 2: vmov >lod5=s10,<lou1=r3
# copy-collector input: vmov s10,r3

# qhasm: hid5 = hiu1
# asm 1: vmov >hid5=float32#12,<hiu1=int32#5
# asm 2: vmov >hid5=s11,<hiu1=r4
# copy-collector input: vmov s11,r4

# qhasm: lod6 = lou2
# asm 1: vmov >lod6=float32#13,<lou2=int32#6
# asm 2: vmov >lod6=s12,<lou2=r5
# copy-collector input: vmov s12,r5

# qhasm: hid6 = hiu2
# asm 1: vmov >hid6=float32#14,<hiu2=int32#7
# asm 2: vmov >hid6=s13,<hiu2=r6
# copy-collector input: vmov s13,r6

# qhasm: lod7 = lou3
# asm 1: vmov >lod7=float32#15,<lou3=int32#8
# asm 2: vmov >lod7=s14,<lou3=r7
# copy-collector input: vmov s14,r7

# qhasm: hid7 = hiu3
# asm 1: vmov >hid7=float32#16,<hiu3=int32#9
# asm 2: vmov >hid7=s15,<hiu3=r8
# copy-collector input: vmov s15,r8

# qhasm: input_0 = o2
# asm 1: ldr >input_0=int32#2,<o2=stack32#3
# asm 2: ldr >input_0=r1,<o2=[sp,#8]
# copy-collector input: ldr r1,[sp,#8]

# qhasm: =? unsigned<? input_0 -= 128
# asm 1: subs >input_0=int32#2,<input_0=int32#2,#128
# asm 2: subs >input_0=r1,<input_0=r1,#128
# copy-collector output starts
strd r1,r2,[sp,#44]
strd r3,r4,[sp,#52]
strd r5,r6,[sp,#60]
strd r7,r8,[sp,#68]
vmov d4,r1,r2
vmov d5,r3,r4
vmov d6,r5,r6
vmov d7,r7,r8
ldr r1,[sp,#8]
# copy-collector output ends
subs r1,r1,#128

# qhasm: o2 = input_0
# asm 1: str <input_0=int32#2,>o2=stack32#3
# asm 2: str <input_0=r1,>o2=[sp,#8]
# copy-collector input: str r1,[sp,#8]

# qhasm: goto mainloop if !unsigned<
# copy-collector output starts
str r1,[sp,#8]
# copy-collector output ends
bhs ._mainloop

# qhasm: endmainloop:
._endmainloop:

# qhasm: input_1 = o0
# asm 1: ldr >input_1=int32#1,<o0=stack32#1
# asm 2: ldr >input_1=r0,<o0=[sp,#0]
# copy-collector input: ldr r0,[sp,#0]

# qhasm: lou0 = lom0
# asm 1: ldr >lou0=int32#3,<lom0=stack32#4
# asm 2: ldr >lou0=r2,<lom0=[sp,#12]
# copy-collector input: ldr r2,[sp,#12]

# qhasm: hiu0 = him0
# asm 1: ldr >hiu0=int32#4,<him0=stack32#5
# asm 2: ldr >hiu0=r3,<him0=[sp,#16]
# copy-collector input: ldr r3,[sp,#16]

# qhasm: lou1 = lom1
# asm 1: ldr >lou1=int32#5,<lom1=stack32#6
# asm 2: ldr >lou1=r4,<lom1=[sp,#20]
# copy-collector input: ldr r4,[sp,#20]

# qhasm: hiu1 = him1
# asm 1: ldr >hiu1=int32#6,<him1=stack32#7
# asm 2: ldr >hiu1=r5,<him1=[sp,#24]
# copy-collector input: ldr r5,[sp,#24]

# qhasm: lou2 = lom2
# asm 1: ldr >lou2=int32#7,<lom2=stack32#8
# asm 2: ldr >lou2=r6,<lom2=[sp,#28]
# copy-collector input: ldr r6,[sp,#28]

# qhasm: hiu2 = him2
# asm 1: ldr >hiu2=int32#8,<him2=stack32#9
# asm 2: ldr >hiu2=r7,<him2=[sp,#32]
# copy-collector input: ldr r7,[sp,#32]

# qhasm: lou3 = lom3
# asm 1: ldr >lou3=int32#9,<lom3=stack32#10
# asm 2: ldr >lou3=r8,<lom3=[sp,#36]
# copy-collector input: ldr r8,[sp,#36]

# qhasm: hiu3 = him3
# asm 1: ldr >hiu3=int32#10,<him3=stack32#11
# asm 2: ldr >hiu3=r9,<him3=[sp,#40]
# copy-collector input: ldr r9,[sp,#40]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
ldr r0,[sp,#0]
ldr r2,[sp,#12]
ldr r3,[sp,#16]
ldr r4,[sp,#20]
ldr r5,[sp,#24]
ldr r6,[sp,#28]
ldr r7,[sp,#32]
ldr r8,[sp,#36]
ldr r9,[sp,#40]
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
# asm 1: str <hiu0=int32#4,[<input_1=int32#1]
# asm 2: str <hiu0=r3,[<input_1=r0]
# copy-collector input: str r3,[r0]

# qhasm: mem32[input_1+4] = lou0
# asm 1: str <lou0=int32#3,[<input_1=int32#1,#4]
# asm 2: str <lou0=r2,[<input_1=r0,#4]
# copy-collector input: str r2,[r0,#4]

# qhasm: mem32[input_1+8] = hiu1
# asm 1: str <hiu1=int32#6,[<input_1=int32#1,#8]
# asm 2: str <hiu1=r5,[<input_1=r0,#8]
# copy-collector input: str r5,[r0,#8]

# qhasm: mem32[input_1+12] = lou1
# asm 1: str <lou1=int32#5,[<input_1=int32#1,#12]
# asm 2: str <lou1=r4,[<input_1=r0,#12]
# copy-collector input: str r4,[r0,#12]

# qhasm: mem32[input_1+16] = hiu2
# asm 1: str <hiu2=int32#8,[<input_1=int32#1,#16]
# asm 2: str <hiu2=r7,[<input_1=r0,#16]
# copy-collector input: str r7,[r0,#16]

# qhasm: mem32[input_1+20] = lou2
# asm 1: str <lou2=int32#7,[<input_1=int32#1,#20]
# asm 2: str <lou2=r6,[<input_1=r0,#20]
# copy-collector input: str r6,[r0,#20]

# qhasm: mem32[input_1+24] = hiu3
# asm 1: str <hiu3=int32#10,[<input_1=int32#1,#24]
# asm 2: str <hiu3=r9,[<input_1=r0,#24]
# copy-collector input: str r9,[r0,#24]

# qhasm: mem32[input_1+28] = lou3
# asm 1: str <lou3=int32#9,[<input_1=int32#1,#28]
# asm 2: str <lou3=r8,[<input_1=r0,#28]
# copy-collector input: str r8,[r0,#28]

# qhasm: lou0 = lom4
# asm 1: ldr >lou0=int32#3,<lom4=stack32#12
# asm 2: ldr >lou0=r2,<lom4=[sp,#44]
# copy-collector input: ldr r2,[sp,#44]

# qhasm: hiu0 = him4
# asm 1: ldr >hiu0=int32#4,<him4=stack32#13
# asm 2: ldr >hiu0=r3,<him4=[sp,#48]
# copy-collector input: ldr r3,[sp,#48]

# qhasm: lou1 = lom5
# asm 1: ldr >lou1=int32#5,<lom5=stack32#14
# asm 2: ldr >lou1=r4,<lom5=[sp,#52]
# copy-collector input: ldr r4,[sp,#52]

# qhasm: hiu1 = him5
# asm 1: ldr >hiu1=int32#6,<him5=stack32#15
# asm 2: ldr >hiu1=r5,<him5=[sp,#56]
# copy-collector input: ldr r5,[sp,#56]

# qhasm: lou2 = lom6
# asm 1: ldr >lou2=int32#7,<lom6=stack32#16
# asm 2: ldr >lou2=r6,<lom6=[sp,#60]
# copy-collector input: ldr r6,[sp,#60]

# qhasm: hiu2 = him6
# asm 1: ldr >hiu2=int32#8,<him6=stack32#17
# asm 2: ldr >hiu2=r7,<him6=[sp,#64]
# copy-collector input: ldr r7,[sp,#64]

# qhasm: lou3 = lom7
# asm 1: ldr >lou3=int32#9,<lom7=stack32#18
# asm 2: ldr >lou3=r8,<lom7=[sp,#68]
# copy-collector input: ldr r8,[sp,#68]

# qhasm: hiu3 = him7
# asm 1: ldr >hiu3=int32#10,<him7=stack32#19
# asm 2: ldr >hiu3=r9,<him7=[sp,#72]
# copy-collector input: ldr r9,[sp,#72]

# qhasm: lou0 = lou0[3]lou0[2]lou0[1]lou0[0]
# asm 1: rev >lou0=int32#3,<lou0=int32#3
# asm 2: rev >lou0=r2,<lou0=r2
# copy-collector output starts
str r3,[r0]
str r2,[r0,#4]
str r5,[r0,#8]
str r4,[r0,#12]
str r7,[r0,#16]
str r6,[r0,#20]
str r9,[r0,#24]
str r8,[r0,#28]
ldr r2,[sp,#44]
ldr r3,[sp,#48]
ldr r4,[sp,#52]
ldr r5,[sp,#56]
ldr r6,[sp,#60]
ldr r7,[sp,#64]
ldr r8,[sp,#68]
ldr r9,[sp,#72]
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
# asm 1: str <hiu0=int32#4,[<input_1=int32#1,#32]
# asm 2: str <hiu0=r3,[<input_1=r0,#32]
# copy-collector input: str r3,[r0,#32]

# qhasm: mem32[input_1+36] = lou0
# asm 1: str <lou0=int32#3,[<input_1=int32#1,#36]
# asm 2: str <lou0=r2,[<input_1=r0,#36]
# copy-collector input: str r2,[r0,#36]

# qhasm: mem32[input_1+40] = hiu1
# asm 1: str <hiu1=int32#6,[<input_1=int32#1,#40]
# asm 2: str <hiu1=r5,[<input_1=r0,#40]
# copy-collector input: str r5,[r0,#40]

# qhasm: mem32[input_1+44] = lou1
# asm 1: str <lou1=int32#5,[<input_1=int32#1,#44]
# asm 2: str <lou1=r4,[<input_1=r0,#44]
# copy-collector input: str r4,[r0,#44]

# qhasm: mem32[input_1+48] = hiu2
# asm 1: str <hiu2=int32#8,[<input_1=int32#1,#48]
# asm 2: str <hiu2=r7,[<input_1=r0,#48]
# copy-collector input: str r7,[r0,#48]

# qhasm: mem32[input_1+52] = lou2
# asm 1: str <lou2=int32#7,[<input_1=int32#1,#52]
# asm 2: str <lou2=r6,[<input_1=r0,#52]
# copy-collector input: str r6,[r0,#52]

# qhasm: mem32[input_1+56] = hiu3
# asm 1: str <hiu3=int32#10,[<input_1=int32#1,#56]
# asm 2: str <hiu3=r9,[<input_1=r0,#56]
# copy-collector input: str r9,[r0,#56]

# qhasm: mem32[input_1+60] = lou3
# asm 1: str <lou3=int32#9,[<input_1=int32#1,#60]
# asm 2: str <lou3=r8,[<input_1=r0,#60]
# copy-collector input: str r8,[r0,#60]

# qhasm: input_0 += 128
# asm 1: add >input_0=int32#1,<input_0=int32#2,#128
# asm 2: add >input_0=r0,<input_0=r1,#128
# copy-collector output starts
str r3,[r0,#32]
str r2,[r0,#36]
str r5,[r0,#40]
str r4,[r0,#44]
str r7,[r0,#48]
str r6,[r0,#52]
str r9,[r0,#56]
str r8,[r0,#60]
# copy-collector output ends
add r0,r1,#128

# qhasm: popreturn input_0
add.w sp,sp,#160
vpop {d8-d15}
pop {r4,r5,r6,r7,r8,r9,r10,r11,r14}
bx lr
