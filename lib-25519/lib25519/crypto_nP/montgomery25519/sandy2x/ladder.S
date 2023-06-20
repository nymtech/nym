// linker define ladder
// linker use v0_0
// linker use v1_0
// linker use v2_1
// linker use v19_19
// linker use v38_1
// linker use v38_38
// linker use v121666_121666
// linker use m25
// linker use m26
// linker use subc0
// linker use subc2

#include "ladder_namespace.h"
#include "consts_namespace.h"

# qhasm: int64 input_0

# qhasm: int64 input_1

# qhasm: int64 input_2

# qhasm: int64 input_3

# qhasm: int64 input_4

# qhasm: int64 input_5

# qhasm: stack64 input_6

# qhasm: stack64 input_7

# qhasm: int64 caller_r11

# qhasm: int64 caller_r12

# qhasm: int64 caller_r13

# qhasm: int64 caller_r14

# qhasm: int64 caller_r15

# qhasm: int64 caller_rbx

# qhasm: int64 caller_rbp

# qhasm: stack128 p01_stack

# qhasm: stack128 p23_stack

# qhasm: stack128 p23_19_19_stack

# qhasm: stack128 p45_stack

# qhasm: stack128 p45_19_19_stack

# qhasm: stack128 p67_stack

# qhasm: stack128 p67_19_19_stack

# qhasm: stack128 p89_stack

# qhasm: stack128 p89_19_19_stack

# qhasm: stack128 p12_2_1_stack

# qhasm: stack128 p12_38_19_stack

# qhasm: stack128 p34_2_1_stack

# qhasm: stack128 p34_38_19_stack

# qhasm: stack128 p56_2_1_stack

# qhasm: stack128 p56_38_19_stack

# qhasm: stack128 p78_2_1_stack

# qhasm: stack128 p78_38_19_stack

# qhasm: stack128 p90_38_1_stack

# qhasm: reg128 g_01

# qhasm: reg128 g_89

# qhasm: stack128 x3_0

# qhasm: stack128 x3_2

# qhasm: stack128 x3_4

# qhasm: stack128 x3_6

# qhasm: stack128 x3_8

# qhasm: stack128 z3_0

# qhasm: stack128 z3_2

# qhasm: stack128 z3_4

# qhasm: stack128 z3_6

# qhasm: stack128 z3_8

# qhasm: stack128 t0_0

# qhasm: stack128 t0_2

# qhasm: stack128 t0_4

# qhasm: stack128 t0_6

# qhasm: stack128 t0_8

# qhasm: reg128 init0

# qhasm: reg128 init1

# qhasm: int64 b

# qhasm: int64 b0

# qhasm: int64 tmp0

# qhasm: int64 b1

# qhasm: int64 tmp1

# qhasm: int64 b2

# qhasm: int64 tmp2

# qhasm: int64 b3

# qhasm: int64 tmp3

# qhasm: int64 byte

# qhasm: int64 ptr

# qhasm: int64 pos

# qhasm: reg128 r

# qhasm: reg128 r0

# qhasm: reg128 r1

# qhasm: reg128 r2

# qhasm: reg128 r3

# qhasm: reg128 r4

# qhasm: reg128 r5

# qhasm: reg128 r6

# qhasm: reg128 r7

# qhasm: reg128 r8

# qhasm: reg128 r9

# qhasm: reg128 f0

# qhasm: reg128 f1

# qhasm: reg128 f2

# qhasm: reg128 f3

# qhasm: reg128 f4

# qhasm: reg128 f5

# qhasm: reg128 f6

# qhasm: reg128 f7

# qhasm: reg128 f8

# qhasm: reg128 f9

# qhasm: stack128 f0_stack

# qhasm: stack128 f1_stack

# qhasm: stack128 f2_stack

# qhasm: stack128 f3_stack

# qhasm: stack128 f4_stack

# qhasm: stack128 f5_stack

# qhasm: stack128 f6_stack

# qhasm: stack128 f7_stack

# qhasm: stack128 f8_stack

# qhasm: stack128 f9_stack

# qhasm: stack128 f0_2_stack

# qhasm: stack128 f1_2_stack

# qhasm: stack128 f2_2_stack

# qhasm: stack128 f3_2_stack

# qhasm: stack128 f4_2_stack

# qhasm: stack128 f5_2_stack

# qhasm: stack128 f6_2_stack

# qhasm: stack128 f7_2_stack

# qhasm: stack128 f8_2_stack

# qhasm: stack128 f9_2_stack

# qhasm: reg128 f1_2

# qhasm: reg128 f2_2

# qhasm: reg128 f3_2

# qhasm: reg128 f4_2

# qhasm: reg128 f9_38

# qhasm: stack128 f5_38_stack

# qhasm: stack128 f6_19_stack

# qhasm: stack128 f7_38_stack

# qhasm: stack128 f8_19_stack

# qhasm: reg128 g0

# qhasm: reg128 g1

# qhasm: reg128 g2

# qhasm: reg128 g3

# qhasm: reg128 g4

# qhasm: reg128 g5

# qhasm: reg128 g6

# qhasm: reg128 g7

# qhasm: reg128 g8

# qhasm: reg128 g9

# qhasm: reg128 h0

# qhasm: reg128 h1

# qhasm: reg128 h2

# qhasm: reg128 h3

# qhasm: reg128 h4

# qhasm: reg128 h5

# qhasm: reg128 h6

# qhasm: reg128 h7

# qhasm: reg128 h8

# qhasm: reg128 h9

# qhasm: reg128 h1_2

# qhasm: reg128 h2_2

# qhasm: reg128 h3_2

# qhasm: reg128 h4_2

# qhasm: reg128 h9_38

# qhasm: reg128 carry0

# qhasm: reg128 carry1

# qhasm: reg128 carry2

# qhasm: reg128 carry3

# qhasm: reg128 carry4

# qhasm: reg128 carry5

# qhasm: reg128 carry6

# qhasm: reg128 carry7

# qhasm: reg128 carry8

# qhasm: reg128 carry9

# qhasm: stack128 s

# qhasm: stack8192 masks

# qhasm: reg128 mask

# qhasm: reg128 diff

# qhasm: stack128 h5_stack

# qhasm: stack128 h6_stack

# qhasm: stack128 h7_stack

# qhasm: stack128 h8_stack

# qhasm: stack128 h5_2_stack

# qhasm: stack128 h6_2_stack

# qhasm: stack128 h7_2_stack

# qhasm: stack128 h8_2_stack

# qhasm: stack128 h5_38_stack

# qhasm: stack128 h6_19_stack

# qhasm: stack128 h7_38_stack

# qhasm: stack128 h8_19_stack

# qhasm: reg128 m0

# qhasm: reg128 m1

# qhasm: reg128 m2

# qhasm: reg128 m3

# qhasm: reg128 m4

# qhasm: reg128 m5

# qhasm: reg128 m6

# qhasm: reg128 m7

# qhasm: reg128 m8

# qhasm: reg128 m9

# qhasm: reg128 h_01

# qhasm: reg128 h_23

# qhasm: reg128 h_45

# qhasm: reg128 h_67

# qhasm: reg128 h_89

# qhasm: int64 h_0

# qhasm: int64 h_1

# qhasm: int64 h_2

# qhasm: int64 h_3

# qhasm: int64 h_4

# qhasm: int64 h_5

# qhasm: int64 h_6

# qhasm: int64 h_7

# qhasm: int64 h_8

# qhasm: int64 h_9

# qhasm: int64 carry_0

# qhasm: int64 carry_1

# qhasm: int64 carry_2

# qhasm: int64 carry_3

# qhasm: int64 carry_4

# qhasm: int64 carry_5

# qhasm: int64 carry_6

# qhasm: int64 carry_7

# qhasm: int64 carry_8

# qhasm: int64 carry_9

# qhasm: stack128 buf0

# qhasm: stack128 buf1

# qhasm: stack128 buf2

# qhasm: stack128 buf3

# qhasm: stack128 buf4

# qhasm: stack128 buf5

# qhasm: stack128 buf6

# qhasm: stack128 buf7

# qhasm: stack128 buf8

# qhasm: stack128 buf9

# qhasm: stack64 r11_stack

# qhasm: stack64 r12_stack

# qhasm: stack64 r13_stack

# qhasm: stack64 r14_stack

# qhasm: stack64 r15_stack

# qhasm: stack64 rbx_stack

# qhasm: stack64 rbp_stack

# qhasm: enter ladder
.p2align 5
.global _ladder
.global ladder
_ladder:
ladder:
mov %rsp,%r11
and $31,%r11
add $1856,%r11
sub %r11,%rsp

# qhasm: r11_stack = caller_r11
# asm 1: movq <caller_r11=int64#9,>r11_stack=stack64#1
# asm 2: movq <caller_r11=%r11,>r11_stack=1824(%rsp)
movq %r11,1824(%rsp)

# qhasm: r12_stack = caller_r12
# asm 1: movq <caller_r12=int64#10,>r12_stack=stack64#2
# asm 2: movq <caller_r12=%r12,>r12_stack=1832(%rsp)
movq %r12,1832(%rsp)

# qhasm: r13_stack = caller_r13
# asm 1: movq <caller_r13=int64#11,>r13_stack=stack64#3
# asm 2: movq <caller_r13=%r13,>r13_stack=1840(%rsp)
movq %r13,1840(%rsp)

# qhasm: r14_stack = caller_r14
# asm 1: movq <caller_r14=int64#12,>r14_stack=stack64#4
# asm 2: movq <caller_r14=%r14,>r14_stack=1848(%rsp)
movq %r14,1848(%rsp)

# qhasm: init0 = v0_0
# asm 1: movdqa v0_0,>init0=reg128#1
# asm 2: movdqa v0_0,>init0=%xmm0
movdqa v0_0(%rip),%xmm0

# qhasm: init1 = v1_0
# asm 1: movdqa v1_0,>init1=reg128#2
# asm 2: movdqa v1_0,>init1=%xmm1
movdqa v1_0(%rip),%xmm1

# qhasm: r = mem128[input_0 + 0]
# asm 1: movdqu   0(<input_0=int64#1),>r=reg128#3
# asm 2: movdqu   0(<input_0=%rdi),>r=%xmm2
movdqu   0(%rdi),%xmm2

# qhasm: x3_0 = r
# asm 1: movdqa <r=reg128#3,>x3_0=stack128#1
# asm 2: movdqa <r=%xmm2,>x3_0=0(%rsp)
movdqa %xmm2,0(%rsp)

# qhasm: r = mem128[input_0 + 16]
# asm 1: movdqu   16(<input_0=int64#1),>r=reg128#3
# asm 2: movdqu   16(<input_0=%rdi),>r=%xmm2
movdqu   16(%rdi),%xmm2

# qhasm: x3_2 = r
# asm 1: movdqa <r=reg128#3,>x3_2=stack128#2
# asm 2: movdqa <r=%xmm2,>x3_2=16(%rsp)
movdqa %xmm2,16(%rsp)

# qhasm: r = mem128[input_0 + 32]
# asm 1: movdqu   32(<input_0=int64#1),>r=reg128#3
# asm 2: movdqu   32(<input_0=%rdi),>r=%xmm2
movdqu   32(%rdi),%xmm2

# qhasm: x3_4 = r
# asm 1: movdqa <r=reg128#3,>x3_4=stack128#3
# asm 2: movdqa <r=%xmm2,>x3_4=32(%rsp)
movdqa %xmm2,32(%rsp)

# qhasm: r = mem128[input_0 + 48]
# asm 1: movdqu   48(<input_0=int64#1),>r=reg128#3
# asm 2: movdqu   48(<input_0=%rdi),>r=%xmm2
movdqu   48(%rdi),%xmm2

# qhasm: x3_6 = r
# asm 1: movdqa <r=reg128#3,>x3_6=stack128#4
# asm 2: movdqa <r=%xmm2,>x3_6=48(%rsp)
movdqa %xmm2,48(%rsp)

# qhasm: r = mem128[input_0 + 64]
# asm 1: movdqu   64(<input_0=int64#1),>r=reg128#3
# asm 2: movdqu   64(<input_0=%rdi),>r=%xmm2
movdqu   64(%rdi),%xmm2

# qhasm: x3_8 = r
# asm 1: movdqa <r=reg128#3,>x3_8=stack128#5
# asm 2: movdqa <r=%xmm2,>x3_8=64(%rsp)
movdqa %xmm2,64(%rsp)

# qhasm: z3_0 = init1
# asm 1: movdqa <init1=reg128#2,>z3_0=stack128#6
# asm 2: movdqa <init1=%xmm1,>z3_0=80(%rsp)
movdqa %xmm1,80(%rsp)

# qhasm: z3_2 = init0
# asm 1: movdqa <init0=reg128#1,>z3_2=stack128#7
# asm 2: movdqa <init0=%xmm0,>z3_2=96(%rsp)
movdqa %xmm0,96(%rsp)

# qhasm: z3_4 = init0
# asm 1: movdqa <init0=reg128#1,>z3_4=stack128#8
# asm 2: movdqa <init0=%xmm0,>z3_4=112(%rsp)
movdqa %xmm0,112(%rsp)

# qhasm: z3_6 = init0
# asm 1: movdqa <init0=reg128#1,>z3_6=stack128#9
# asm 2: movdqa <init0=%xmm0,>z3_6=128(%rsp)
movdqa %xmm0,128(%rsp)

# qhasm: z3_8 = init0
# asm 1: movdqa <init0=reg128#1,>z3_8=stack128#10
# asm 2: movdqa <init0=%xmm0,>z3_8=144(%rsp)
movdqa %xmm0,144(%rsp)

# qhasm: f0 = init1
# asm 1: movdqa <init1=reg128#2,>f0=reg128#1
# asm 2: movdqa <init1=%xmm1,>f0=%xmm0
movdqa %xmm1,%xmm0

# qhasm: f1 ^= f1
# asm 1: pxor >f1=reg128#2,>f1=reg128#2
# asm 2: pxor >f1=%xmm1,>f1=%xmm1
pxor %xmm1,%xmm1

# qhasm: f2 ^= f2
# asm 1: pxor >f2=reg128#3,>f2=reg128#3
# asm 2: pxor >f2=%xmm2,>f2=%xmm2
pxor %xmm2,%xmm2

# qhasm: f3 ^= f3
# asm 1: pxor >f3=reg128#4,>f3=reg128#4
# asm 2: pxor >f3=%xmm3,>f3=%xmm3
pxor %xmm3,%xmm3

# qhasm: f4 ^= f4
# asm 1: pxor >f4=reg128#5,>f4=reg128#5
# asm 2: pxor >f4=%xmm4,>f4=%xmm4
pxor %xmm4,%xmm4

# qhasm: f5 ^= f5
# asm 1: pxor >f5=reg128#6,>f5=reg128#6
# asm 2: pxor >f5=%xmm5,>f5=%xmm5
pxor %xmm5,%xmm5

# qhasm: f6 ^= f6
# asm 1: pxor >f6=reg128#7,>f6=reg128#7
# asm 2: pxor >f6=%xmm6,>f6=%xmm6
pxor %xmm6,%xmm6

# qhasm: f7 ^= f7
# asm 1: pxor >f7=reg128#8,>f7=reg128#8
# asm 2: pxor >f7=%xmm7,>f7=%xmm7
pxor %xmm7,%xmm7

# qhasm: f8 ^= f8
# asm 1: pxor >f8=reg128#9,>f8=reg128#9
# asm 2: pxor >f8=%xmm8,>f8=%xmm8
pxor %xmm8,%xmm8

# qhasm: f9 ^= f9
# asm 1: pxor >f9=reg128#10,>f9=reg128#10
# asm 2: pxor >f9=%xmm9,>f9=%xmm9
pxor %xmm9,%xmm9

# qhasm: r = mem128[input_0 + 0]
# asm 1: movdqu   0(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   0(<input_0=%rdi),>r=%xmm10
movdqu   0(%rdi),%xmm10

# qhasm: p01_stack = r
# asm 1: movdqa <r=reg128#11,>p01_stack=stack128#11
# asm 2: movdqa <r=%xmm10,>p01_stack=160(%rsp)
movdqa %xmm10,160(%rsp)

# qhasm: r = mem128[input_0 + 16]
# asm 1: movdqu   16(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   16(<input_0=%rdi),>r=%xmm10
movdqu   16(%rdi),%xmm10

# qhasm: p23_stack = r
# asm 1: movdqa <r=reg128#11,>p23_stack=stack128#12
# asm 2: movdqa <r=%xmm10,>p23_stack=176(%rsp)
movdqa %xmm10,176(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p23_19_19_stack = r
# asm 1: movdqa <r=reg128#11,>p23_19_19_stack=stack128#13
# asm 2: movdqa <r=%xmm10,>p23_19_19_stack=192(%rsp)
movdqa %xmm10,192(%rsp)

# qhasm: r = mem128[input_0 + 32]
# asm 1: movdqu   32(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   32(<input_0=%rdi),>r=%xmm10
movdqu   32(%rdi),%xmm10

# qhasm: p45_stack = r
# asm 1: movdqa <r=reg128#11,>p45_stack=stack128#14
# asm 2: movdqa <r=%xmm10,>p45_stack=208(%rsp)
movdqa %xmm10,208(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p45_19_19_stack = r
# asm 1: movdqa <r=reg128#11,>p45_19_19_stack=stack128#15
# asm 2: movdqa <r=%xmm10,>p45_19_19_stack=224(%rsp)
movdqa %xmm10,224(%rsp)

# qhasm: r = mem128[input_0 + 48]
# asm 1: movdqu   48(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   48(<input_0=%rdi),>r=%xmm10
movdqu   48(%rdi),%xmm10

# qhasm: p67_stack = r
# asm 1: movdqa <r=reg128#11,>p67_stack=stack128#16
# asm 2: movdqa <r=%xmm10,>p67_stack=240(%rsp)
movdqa %xmm10,240(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p67_19_19_stack = r
# asm 1: movdqa <r=reg128#11,>p67_19_19_stack=stack128#17
# asm 2: movdqa <r=%xmm10,>p67_19_19_stack=256(%rsp)
movdqa %xmm10,256(%rsp)

# qhasm: r = mem128[input_0 + 64]
# asm 1: movdqu   64(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   64(<input_0=%rdi),>r=%xmm10
movdqu   64(%rdi),%xmm10

# qhasm: p89_stack = r
# asm 1: movdqa <r=reg128#11,>p89_stack=stack128#18
# asm 2: movdqa <r=%xmm10,>p89_stack=272(%rsp)
movdqa %xmm10,272(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p89_19_19_stack = r
# asm 1: movdqa <r=reg128#11,>p89_19_19_stack=stack128#19
# asm 2: movdqa <r=%xmm10,>p89_19_19_stack=288(%rsp)
movdqa %xmm10,288(%rsp)

# qhasm: r = mem128[input_0 + 8]
# asm 1: movdqu   8(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   8(<input_0=%rdi),>r=%xmm10
movdqu   8(%rdi),%xmm10

# qhasm: 2x r *= mem128[ v2_1 ]
# asm 1: pmuludq v2_1,<r=reg128#11
# asm 2: pmuludq v2_1,<r=%xmm10
pmuludq v2_1(%rip),%xmm10

# qhasm: p12_2_1_stack = r
# asm 1: movdqa <r=reg128#11,>p12_2_1_stack=stack128#20
# asm 2: movdqa <r=%xmm10,>p12_2_1_stack=304(%rsp)
movdqa %xmm10,304(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p12_38_19_stack = r
# asm 1: movdqa <r=reg128#11,>p12_38_19_stack=stack128#21
# asm 2: movdqa <r=%xmm10,>p12_38_19_stack=320(%rsp)
movdqa %xmm10,320(%rsp)

# qhasm: r = mem128[input_0 + 24]
# asm 1: movdqu   24(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   24(<input_0=%rdi),>r=%xmm10
movdqu   24(%rdi),%xmm10

# qhasm: 2x r *= mem128[ v2_1 ]
# asm 1: pmuludq v2_1,<r=reg128#11
# asm 2: pmuludq v2_1,<r=%xmm10
pmuludq v2_1(%rip),%xmm10

# qhasm: p34_2_1_stack = r
# asm 1: movdqa <r=reg128#11,>p34_2_1_stack=stack128#22
# asm 2: movdqa <r=%xmm10,>p34_2_1_stack=336(%rsp)
movdqa %xmm10,336(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p34_38_19_stack = r
# asm 1: movdqa <r=reg128#11,>p34_38_19_stack=stack128#23
# asm 2: movdqa <r=%xmm10,>p34_38_19_stack=352(%rsp)
movdqa %xmm10,352(%rsp)

# qhasm: r = mem128[input_0 + 40]
# asm 1: movdqu   40(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   40(<input_0=%rdi),>r=%xmm10
movdqu   40(%rdi),%xmm10

# qhasm: 2x r *= mem128[ v2_1 ]
# asm 1: pmuludq v2_1,<r=reg128#11
# asm 2: pmuludq v2_1,<r=%xmm10
pmuludq v2_1(%rip),%xmm10

# qhasm: p56_2_1_stack = r
# asm 1: movdqa <r=reg128#11,>p56_2_1_stack=stack128#24
# asm 2: movdqa <r=%xmm10,>p56_2_1_stack=368(%rsp)
movdqa %xmm10,368(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p56_38_19_stack = r
# asm 1: movdqa <r=reg128#11,>p56_38_19_stack=stack128#25
# asm 2: movdqa <r=%xmm10,>p56_38_19_stack=384(%rsp)
movdqa %xmm10,384(%rsp)

# qhasm: r = mem128[input_0 + 56]
# asm 1: movdqu   56(<input_0=int64#1),>r=reg128#11
# asm 2: movdqu   56(<input_0=%rdi),>r=%xmm10
movdqu   56(%rdi),%xmm10

# qhasm: 2x r *= mem128[ v2_1 ]
# asm 1: pmuludq v2_1,<r=reg128#11
# asm 2: pmuludq v2_1,<r=%xmm10
pmuludq v2_1(%rip),%xmm10

# qhasm: p78_2_1_stack = r
# asm 1: movdqa <r=reg128#11,>p78_2_1_stack=stack128#26
# asm 2: movdqa <r=%xmm10,>p78_2_1_stack=400(%rsp)
movdqa %xmm10,400(%rsp)

# qhasm: 2x r *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<r=reg128#11
# asm 2: pmuludq v19_19,<r=%xmm10
pmuludq v19_19(%rip),%xmm10

# qhasm: p78_38_19_stack = r
# asm 1: movdqa <r=reg128#11,>p78_38_19_stack=stack128#27
# asm 2: movdqa <r=%xmm10,>p78_38_19_stack=416(%rsp)
movdqa %xmm10,416(%rsp)

# qhasm: g_01 = mem128[input_0 + 0]
# asm 1: movdqu   0(<input_0=int64#1),>g_01=reg128#11
# asm 2: movdqu   0(<input_0=%rdi),>g_01=%xmm10
movdqu   0(%rdi),%xmm10

# qhasm: g_89 = mem128[input_0 + 64]
# asm 1: movdqu   64(<input_0=int64#1),>g_89=reg128#12
# asm 2: movdqu   64(<input_0=%rdi),>g_89=%xmm11
movdqu   64(%rdi),%xmm11

# qhasm: g_01 = blend dwords of g_01 g_89 by 12
# asm 1: blendps $12, <g_89=reg128#12, <g_01=reg128#11
# asm 2: blendps $12, <g_89=%xmm11, <g_01=%xmm10
blendps $12, %xmm11, %xmm10

# qhasm: g_89 = shuffle dwords of g_01 by 2
# asm 1: pshufd $2,<g_01=reg128#11,>g_89=reg128#11
# asm 2: pshufd $2,<g_01=%xmm10,>g_89=%xmm10
pshufd $2,%xmm10,%xmm10

# qhasm: 2x g_89 *= mem128[ v38_1 ]
# asm 1: pmuludq v38_1,<g_89=reg128#11
# asm 2: pmuludq v38_1,<g_89=%xmm10
pmuludq v38_1(%rip),%xmm10

# qhasm: p90_38_1_stack = g_89
# asm 1: movdqa <g_89=reg128#11,>p90_38_1_stack=stack128#28
# asm 2: movdqa <g_89=%xmm10,>p90_38_1_stack=432(%rsp)
movdqa %xmm10,432(%rsp)

# qhasm: b0 = mem64[input_1 + 0]
# asm 1: movq   0(<input_1=int64#2),>b0=int64#3
# asm 2: movq   0(<input_1=%rsi),>b0=%rdx
movq   0(%rsi),%rdx

# qhasm: b1 = mem64[input_1 + 8]
# asm 1: movq   8(<input_1=int64#2),>b1=int64#4
# asm 2: movq   8(<input_1=%rsi),>b1=%rcx
movq   8(%rsi),%rcx

# qhasm: b2 = mem64[input_1 + 16]
# asm 1: movq   16(<input_1=int64#2),>b2=int64#5
# asm 2: movq   16(<input_1=%rsi),>b2=%r8
movq   16(%rsi),%r8

# qhasm: b3 = mem64[input_1 + 24]
# asm 1: movq   24(<input_1=int64#2),>b3=int64#6
# asm 2: movq   24(<input_1=%rsi),>b3=%r9
movq   24(%rsi),%r9

# qhasm: b0 = (b1 b0) >> 1
# asm 1: shrd $1,<b1=int64#4,<b0=int64#3
# asm 2: shrd $1,<b1=%rcx,<b0=%rdx
shrd $1,%rcx,%rdx

# qhasm: b1 = (b2 b1) >> 1
# asm 1: shrd $1,<b2=int64#5,<b1=int64#4
# asm 2: shrd $1,<b2=%r8,<b1=%rcx
shrd $1,%r8,%rcx

# qhasm: b2 = (b3 b2) >> 1
# asm 1: shrd $1,<b3=int64#6,<b2=int64#5
# asm 2: shrd $1,<b3=%r9,<b2=%r8
shrd $1,%r9,%r8

# qhasm: (uint64) b3 >>= 1
# asm 1: shr  $1,<b3=int64#6
# asm 2: shr  $1,<b3=%r9
shr  $1,%r9

# qhasm: b0 ^= mem64[input_1 + 0]
# asm 1: xorq 0(<input_1=int64#2),<b0=int64#3
# asm 2: xorq 0(<input_1=%rsi),<b0=%rdx
xorq 0(%rsi),%rdx

# qhasm: b1 ^= mem64[input_1 + 8]
# asm 1: xorq 8(<input_1=int64#2),<b1=int64#4
# asm 2: xorq 8(<input_1=%rsi),<b1=%rcx
xorq 8(%rsi),%rcx

# qhasm: b2 ^= mem64[input_1 + 16]
# asm 1: xorq 16(<input_1=int64#2),<b2=int64#5
# asm 2: xorq 16(<input_1=%rsi),<b2=%r8
xorq 16(%rsi),%r8

# qhasm: b3 ^= mem64[input_1 + 24]
# asm 1: xorq 24(<input_1=int64#2),<b3=int64#6
# asm 2: xorq 24(<input_1=%rsi),<b3=%r9
xorq 24(%rsi),%r9

# qhasm: ptr = &masks
# asm 1: leaq <masks=stack8192#1,>ptr=int64#2
# asm 2: leaq <masks=800(%rsp),>ptr=%rsi
leaq 800(%rsp),%rsi

# qhasm: pos = 64
# asm 1: mov  $64,>pos=int64#7
# asm 2: mov  $64,>pos=%rax
mov  $64,%rax

# qhasm: small_loop:
._small_loop:

# qhasm: tmp0 = b0
# asm 1: mov  <b0=int64#3,>tmp0=int64#8
# asm 2: mov  <b0=%rdx,>tmp0=%r10
mov  %rdx,%r10

# qhasm: tmp1 = b1
# asm 1: mov  <b1=int64#4,>tmp1=int64#9
# asm 2: mov  <b1=%rcx,>tmp1=%r11
mov  %rcx,%r11

# qhasm: tmp2 = b2
# asm 1: mov  <b2=int64#5,>tmp2=int64#10
# asm 2: mov  <b2=%r8,>tmp2=%r12
mov  %r8,%r12

# qhasm: tmp3 = b3
# asm 1: mov  <b3=int64#6,>tmp3=int64#11
# asm 2: mov  <b3=%r9,>tmp3=%r13
mov  %r9,%r13

# qhasm: (uint64) b0 >>= 1
# asm 1: shr  $1,<b0=int64#3
# asm 2: shr  $1,<b0=%rdx
shr  $1,%rdx

# qhasm: (uint64) b1 >>= 1
# asm 1: shr  $1,<b1=int64#4
# asm 2: shr  $1,<b1=%rcx
shr  $1,%rcx

# qhasm: (uint64) b2 >>= 1
# asm 1: shr  $1,<b2=int64#5
# asm 2: shr  $1,<b2=%r8
shr  $1,%r8

# qhasm: (uint64) b3 >>= 1
# asm 1: shr  $1,<b3=int64#6
# asm 2: shr  $1,<b3=%r9
shr  $1,%r9

# qhasm: (uint32) tmp0 &= 1
# asm 1: and  $1,<tmp0=int64#8d
# asm 2: and  $1,<tmp0=%r10d
and  $1,%r10d

# qhasm: (uint32) tmp1 &= 1
# asm 1: and  $1,<tmp1=int64#9d
# asm 2: and  $1,<tmp1=%r11d
and  $1,%r11d

# qhasm: (uint32) tmp2 &= 1
# asm 1: and  $1,<tmp2=int64#10d
# asm 2: and  $1,<tmp2=%r12d
and  $1,%r12d

# qhasm: (uint32) tmp3 &= 1
# asm 1: and  $1,<tmp3=int64#11d
# asm 2: and  $1,<tmp3=%r13d
and  $1,%r13d

# qhasm: tmp0 = -tmp0
# asm 1: neg  <tmp0=int64#8
# asm 2: neg  <tmp0=%r10
neg  %r10

# qhasm: tmp1 = -tmp1
# asm 1: neg  <tmp1=int64#9
# asm 2: neg  <tmp1=%r11
neg  %r11

# qhasm: tmp2 = -tmp2
# asm 1: neg  <tmp2=int64#10
# asm 2: neg  <tmp2=%r12
neg  %r12

# qhasm: tmp3 = -tmp3
# asm 1: neg  <tmp3=int64#11
# asm 2: neg  <tmp3=%r13
neg  %r13

# qhasm: mem32[ptr + 0] = tmp0
# asm 1: movl   <tmp0=int64#8d,0(<ptr=int64#2)
# asm 2: movl   <tmp0=%r10d,0(<ptr=%rsi)
movl   %r10d,0(%rsi)

# qhasm: mem32[ptr + 256] = tmp1
# asm 1: movl   <tmp1=int64#9d,256(<ptr=int64#2)
# asm 2: movl   <tmp1=%r11d,256(<ptr=%rsi)
movl   %r11d,256(%rsi)

# qhasm: mem32[ptr + 512] = tmp2
# asm 1: movl   <tmp2=int64#10d,512(<ptr=int64#2)
# asm 2: movl   <tmp2=%r12d,512(<ptr=%rsi)
movl   %r12d,512(%rsi)

# qhasm: mem32[ptr + 768] = tmp3
# asm 1: movl   <tmp3=int64#11d,768(<ptr=int64#2)
# asm 2: movl   <tmp3=%r13d,768(<ptr=%rsi)
movl   %r13d,768(%rsi)

# qhasm: ptr += 4
# asm 1: add  $4,<ptr=int64#2
# asm 2: add  $4,<ptr=%rsi
add  $4,%rsi

# qhasm: =? pos -= 1
# asm 1: sub  $1,<pos=int64#7
# asm 2: sub  $1,<pos=%rax
sub  $1,%rax
# comment:fp stack unchanged by jump

# qhasm: goto small_loop if !=
jne ._small_loop

# qhasm: pos = 255
# asm 1: mov  $255,>pos=int64#3
# asm 2: mov  $255,>pos=%rdx
mov  $255,%rdx

# qhasm: ptr += 760
# asm 1: add  $760,<ptr=int64#2
# asm 2: add  $760,<ptr=%rsi
add  $760,%rsi

# qhasm: loop:
._loop:

# qhasm: pos -= 1
# asm 1: sub  $1,<pos=int64#3
# asm 2: sub  $1,<pos=%rdx
sub  $1,%rdx

# qhasm: mask = mem32[ptr + 0] x4
# asm 1: vbroadcastss 0(<ptr=int64#2),>mask=reg128#11
# asm 2: vbroadcastss 0(<ptr=%rsi),>mask=%xmm10
vbroadcastss 0(%rsi),%xmm10

# qhasm: ptr -= 4
# asm 1: sub  $4,<ptr=int64#2
# asm 2: sub  $4,<ptr=%rsi
sub  $4,%rsi

# qhasm: g0 = x3_0
# asm 1: movdqa <x3_0=stack128#1,>g0=reg128#12
# asm 2: movdqa <x3_0=0(%rsp),>g0=%xmm11
movdqa 0(%rsp),%xmm11

# qhasm: g1 = z3_0
# asm 1: movdqa <z3_0=stack128#6,>g1=reg128#13
# asm 2: movdqa <z3_0=80(%rsp),>g1=%xmm12
movdqa 80(%rsp),%xmm12

# qhasm: diff = f0 ^ g0
# asm 1: vpxor <g0=reg128#12,<f0=reg128#1,>diff=reg128#14
# asm 2: vpxor <g0=%xmm11,<f0=%xmm0,>diff=%xmm13
vpxor %xmm11,%xmm0,%xmm13

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#14
# asm 2: pand <mask=%xmm10,<diff=%xmm13
pand %xmm10,%xmm13

# qhasm: f0 ^= diff
# asm 1: pxor <diff=reg128#14,<f0=reg128#1
# asm 2: pxor <diff=%xmm13,<f0=%xmm0
pxor %xmm13,%xmm0

# qhasm: g0 ^= diff
# asm 1: pxor <diff=reg128#14,<g0=reg128#12
# asm 2: pxor <diff=%xmm13,<g0=%xmm11
pxor %xmm13,%xmm11

# qhasm: diff = f1 ^ g1
# asm 1: vpxor <g1=reg128#13,<f1=reg128#2,>diff=reg128#14
# asm 2: vpxor <g1=%xmm12,<f1=%xmm1,>diff=%xmm13
vpxor %xmm12,%xmm1,%xmm13

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#14
# asm 2: pand <mask=%xmm10,<diff=%xmm13
pand %xmm10,%xmm13

# qhasm: f1 ^= diff
# asm 1: pxor <diff=reg128#14,<f1=reg128#2
# asm 2: pxor <diff=%xmm13,<f1=%xmm1
pxor %xmm13,%xmm1

# qhasm: g1 ^= diff
# asm 1: pxor <diff=reg128#14,<g1=reg128#13
# asm 2: pxor <diff=%xmm13,<g1=%xmm12
pxor %xmm13,%xmm12

# qhasm: g2 = x3_2
# asm 1: movdqa <x3_2=stack128#2,>g2=reg128#14
# asm 2: movdqa <x3_2=16(%rsp),>g2=%xmm13
movdqa 16(%rsp),%xmm13

# qhasm: g3 = z3_2
# asm 1: movdqa <z3_2=stack128#7,>g3=reg128#15
# asm 2: movdqa <z3_2=96(%rsp),>g3=%xmm14
movdqa 96(%rsp),%xmm14

# qhasm: diff = f2 ^ g2
# asm 1: vpxor <g2=reg128#14,<f2=reg128#3,>diff=reg128#16
# asm 2: vpxor <g2=%xmm13,<f2=%xmm2,>diff=%xmm15
vpxor %xmm13,%xmm2,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f2 ^= diff
# asm 1: pxor <diff=reg128#16,<f2=reg128#3
# asm 2: pxor <diff=%xmm15,<f2=%xmm2
pxor %xmm15,%xmm2

# qhasm: g2 ^= diff
# asm 1: pxor <diff=reg128#16,<g2=reg128#14
# asm 2: pxor <diff=%xmm15,<g2=%xmm13
pxor %xmm15,%xmm13

# qhasm: diff = f3 ^ g3
# asm 1: vpxor <g3=reg128#15,<f3=reg128#4,>diff=reg128#16
# asm 2: vpxor <g3=%xmm14,<f3=%xmm3,>diff=%xmm15
vpxor %xmm14,%xmm3,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f3 ^= diff
# asm 1: pxor <diff=reg128#16,<f3=reg128#4
# asm 2: pxor <diff=%xmm15,<f3=%xmm3
pxor %xmm15,%xmm3

# qhasm: g3 ^= diff
# asm 1: pxor <diff=reg128#16,<g3=reg128#15
# asm 2: pxor <diff=%xmm15,<g3=%xmm14
pxor %xmm15,%xmm14

# qhasm: x3_2 = g2
# asm 1: movdqa <g2=reg128#14,>x3_2=stack128#1
# asm 2: movdqa <g2=%xmm13,>x3_2=0(%rsp)
movdqa %xmm13,0(%rsp)

# qhasm: z3_2 = g3
# asm 1: movdqa <g3=reg128#15,>z3_2=stack128#2
# asm 2: movdqa <g3=%xmm14,>z3_2=16(%rsp)
movdqa %xmm14,16(%rsp)

# qhasm: g4 = x3_4
# asm 1: movdqa <x3_4=stack128#3,>g4=reg128#14
# asm 2: movdqa <x3_4=32(%rsp),>g4=%xmm13
movdqa 32(%rsp),%xmm13

# qhasm: g5 = z3_4
# asm 1: movdqa <z3_4=stack128#8,>g5=reg128#15
# asm 2: movdqa <z3_4=112(%rsp),>g5=%xmm14
movdqa 112(%rsp),%xmm14

# qhasm: diff = f4 ^ g4
# asm 1: vpxor <g4=reg128#14,<f4=reg128#5,>diff=reg128#16
# asm 2: vpxor <g4=%xmm13,<f4=%xmm4,>diff=%xmm15
vpxor %xmm13,%xmm4,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f4 ^= diff
# asm 1: pxor <diff=reg128#16,<f4=reg128#5
# asm 2: pxor <diff=%xmm15,<f4=%xmm4
pxor %xmm15,%xmm4

# qhasm: g4 ^= diff
# asm 1: pxor <diff=reg128#16,<g4=reg128#14
# asm 2: pxor <diff=%xmm15,<g4=%xmm13
pxor %xmm15,%xmm13

# qhasm: diff = f5 ^ g5
# asm 1: vpxor <g5=reg128#15,<f5=reg128#6,>diff=reg128#16
# asm 2: vpxor <g5=%xmm14,<f5=%xmm5,>diff=%xmm15
vpxor %xmm14,%xmm5,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f5 ^= diff
# asm 1: pxor <diff=reg128#16,<f5=reg128#6
# asm 2: pxor <diff=%xmm15,<f5=%xmm5
pxor %xmm15,%xmm5

# qhasm: g5 ^= diff
# asm 1: pxor <diff=reg128#16,<g5=reg128#15
# asm 2: pxor <diff=%xmm15,<g5=%xmm14
pxor %xmm15,%xmm14

# qhasm: x3_4 = g4
# asm 1: movdqa <g4=reg128#14,>x3_4=stack128#3
# asm 2: movdqa <g4=%xmm13,>x3_4=32(%rsp)
movdqa %xmm13,32(%rsp)

# qhasm: z3_4 = g5
# asm 1: movdqa <g5=reg128#15,>z3_4=stack128#6
# asm 2: movdqa <g5=%xmm14,>z3_4=80(%rsp)
movdqa %xmm14,80(%rsp)

# qhasm: g6 = x3_6
# asm 1: movdqa <x3_6=stack128#4,>g6=reg128#14
# asm 2: movdqa <x3_6=48(%rsp),>g6=%xmm13
movdqa 48(%rsp),%xmm13

# qhasm: g7 = z3_6
# asm 1: movdqa <z3_6=stack128#9,>g7=reg128#15
# asm 2: movdqa <z3_6=128(%rsp),>g7=%xmm14
movdqa 128(%rsp),%xmm14

# qhasm: diff = f6 ^ g6
# asm 1: vpxor <g6=reg128#14,<f6=reg128#7,>diff=reg128#16
# asm 2: vpxor <g6=%xmm13,<f6=%xmm6,>diff=%xmm15
vpxor %xmm13,%xmm6,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f6 ^= diff
# asm 1: pxor <diff=reg128#16,<f6=reg128#7
# asm 2: pxor <diff=%xmm15,<f6=%xmm6
pxor %xmm15,%xmm6

# qhasm: g6 ^= diff
# asm 1: pxor <diff=reg128#16,<g6=reg128#14
# asm 2: pxor <diff=%xmm15,<g6=%xmm13
pxor %xmm15,%xmm13

# qhasm: diff = f7 ^ g7
# asm 1: vpxor <g7=reg128#15,<f7=reg128#8,>diff=reg128#16
# asm 2: vpxor <g7=%xmm14,<f7=%xmm7,>diff=%xmm15
vpxor %xmm14,%xmm7,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f7 ^= diff
# asm 1: pxor <diff=reg128#16,<f7=reg128#8
# asm 2: pxor <diff=%xmm15,<f7=%xmm7
pxor %xmm15,%xmm7

# qhasm: g7 ^= diff
# asm 1: pxor <diff=reg128#16,<g7=reg128#15
# asm 2: pxor <diff=%xmm15,<g7=%xmm14
pxor %xmm15,%xmm14

# qhasm: x3_6 = g6
# asm 1: movdqa <g6=reg128#14,>x3_6=stack128#4
# asm 2: movdqa <g6=%xmm13,>x3_6=48(%rsp)
movdqa %xmm13,48(%rsp)

# qhasm: z3_6 = g7
# asm 1: movdqa <g7=reg128#15,>z3_6=stack128#7
# asm 2: movdqa <g7=%xmm14,>z3_6=96(%rsp)
movdqa %xmm14,96(%rsp)

# qhasm: g8 = x3_8
# asm 1: movdqa <x3_8=stack128#5,>g8=reg128#14
# asm 2: movdqa <x3_8=64(%rsp),>g8=%xmm13
movdqa 64(%rsp),%xmm13

# qhasm: g9 = z3_8
# asm 1: movdqa <z3_8=stack128#10,>g9=reg128#15
# asm 2: movdqa <z3_8=144(%rsp),>g9=%xmm14
movdqa 144(%rsp),%xmm14

# qhasm: diff = f8 ^ g8
# asm 1: vpxor <g8=reg128#14,<f8=reg128#9,>diff=reg128#16
# asm 2: vpxor <g8=%xmm13,<f8=%xmm8,>diff=%xmm15
vpxor %xmm13,%xmm8,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f8 ^= diff
# asm 1: pxor <diff=reg128#16,<f8=reg128#9
# asm 2: pxor <diff=%xmm15,<f8=%xmm8
pxor %xmm15,%xmm8

# qhasm: g8 ^= diff
# asm 1: pxor <diff=reg128#16,<g8=reg128#14
# asm 2: pxor <diff=%xmm15,<g8=%xmm13
pxor %xmm15,%xmm13

# qhasm: diff = f9 ^ g9
# asm 1: vpxor <g9=reg128#15,<f9=reg128#10,>diff=reg128#16
# asm 2: vpxor <g9=%xmm14,<f9=%xmm9,>diff=%xmm15
vpxor %xmm14,%xmm9,%xmm15

# qhasm: diff &= mask
# asm 1: pand <mask=reg128#11,<diff=reg128#16
# asm 2: pand <mask=%xmm10,<diff=%xmm15
pand %xmm10,%xmm15

# qhasm: f9 ^= diff
# asm 1: pxor <diff=reg128#16,<f9=reg128#10
# asm 2: pxor <diff=%xmm15,<f9=%xmm9
pxor %xmm15,%xmm9

# qhasm: g9 ^= diff
# asm 1: pxor <diff=reg128#16,<g9=reg128#15
# asm 2: pxor <diff=%xmm15,<g9=%xmm14
pxor %xmm15,%xmm14

# qhasm: x3_8 = g8
# asm 1: movdqa <g8=reg128#14,>x3_8=stack128#5
# asm 2: movdqa <g8=%xmm13,>x3_8=64(%rsp)
movdqa %xmm13,64(%rsp)

# qhasm: z3_8 = g9
# asm 1: movdqa <g9=reg128#15,>z3_8=stack128#8
# asm 2: movdqa <g9=%xmm14,>z3_8=112(%rsp)
movdqa %xmm14,112(%rsp)

# qhasm: 2x r = g0 + mem128[ subc0 ]
# asm 1: vpaddq subc0,<g0=reg128#12,>r=reg128#11
# asm 2: vpaddq subc0,<g0=%xmm11,>r=%xmm10
vpaddq subc0(%rip),%xmm11,%xmm10

# qhasm: 2x r -= g1
# asm 1: psubq <g1=reg128#13,<r=reg128#11
# asm 2: psubq <g1=%xmm12,<r=%xmm10
psubq %xmm12,%xmm10

# qhasm: 2x g0 += g1
# asm 1: paddq <g1=reg128#13,<g0=reg128#12
# asm 2: paddq <g1=%xmm12,<g0=%xmm11
paddq %xmm12,%xmm11

# qhasm: g1 = unpack_high(g0, r)
# asm 1: vpunpckhqdq <r=reg128#11,<g0=reg128#12,>g1=reg128#13
# asm 2: vpunpckhqdq <r=%xmm10,<g0=%xmm11,>g1=%xmm12
vpunpckhqdq %xmm10,%xmm11,%xmm12

# qhasm: g0 = unpack_low(g0, r)
# asm 1: vpunpcklqdq <r=reg128#11,<g0=reg128#12,>g0=reg128#11
# asm 2: vpunpcklqdq <r=%xmm10,<g0=%xmm11,>g0=%xmm10
vpunpcklqdq %xmm10,%xmm11,%xmm10

# qhasm: 2x r = f0 + f1
# asm 1: vpaddq <f1=reg128#2,<f0=reg128#1,>r=reg128#12
# asm 2: vpaddq <f1=%xmm1,<f0=%xmm0,>r=%xmm11
vpaddq %xmm1,%xmm0,%xmm11

# qhasm: 2x f0 += mem128[ subc0 ]
# asm 1: paddq subc0,<f0=reg128#1
# asm 2: paddq subc0,<f0=%xmm0
paddq subc0(%rip),%xmm0

# qhasm: 2x f0 -= f1
# asm 1: psubq <f1=reg128#2,<f0=reg128#1
# asm 2: psubq <f1=%xmm1,<f0=%xmm0
psubq %xmm1,%xmm0

# qhasm: f1 = unpack_high(f0, r)
# asm 1: vpunpckhqdq <r=reg128#12,<f0=reg128#1,>f1=reg128#2
# asm 2: vpunpckhqdq <r=%xmm11,<f0=%xmm0,>f1=%xmm1
vpunpckhqdq %xmm11,%xmm0,%xmm1

# qhasm: f0 = unpack_low(f0, r)
# asm 1: vpunpcklqdq <r=reg128#12,<f0=reg128#1,>f0=reg128#1
# asm 2: vpunpcklqdq <r=%xmm11,<f0=%xmm0,>f0=%xmm0
vpunpcklqdq %xmm11,%xmm0,%xmm0

# qhasm: 2x h0 = g0 * f0
# asm 1: vpmuludq <f0=reg128#1,<g0=reg128#11,>h0=reg128#12
# asm 2: vpmuludq <f0=%xmm0,<g0=%xmm10,>h0=%xmm11
vpmuludq %xmm0,%xmm10,%xmm11

# qhasm: 2x h1 = g0 * f1
# asm 1: vpmuludq <f1=reg128#2,<g0=reg128#11,>h1=reg128#14
# asm 2: vpmuludq <f1=%xmm1,<g0=%xmm10,>h1=%xmm13
vpmuludq %xmm1,%xmm10,%xmm13

# qhasm: f1_stack = f1
# asm 1: movdqa <f1=reg128#2,>f1_stack=stack128#9
# asm 2: movdqa <f1=%xmm1,>f1_stack=128(%rsp)
movdqa %xmm1,128(%rsp)

# qhasm: 2x f1 += f1
# asm 1: paddq <f1=reg128#2,<f1=reg128#2
# asm 2: paddq <f1=%xmm1,<f1=%xmm1
paddq %xmm1,%xmm1

# qhasm: 2x r = g1 * f0
# asm 1: vpmuludq <f0=reg128#1,<g1=reg128#13,>r=reg128#15
# asm 2: vpmuludq <f0=%xmm0,<g1=%xmm12,>r=%xmm14
vpmuludq %xmm0,%xmm12,%xmm14

# qhasm: f0_stack = f0
# asm 1: movdqa <f0=reg128#1,>f0_stack=stack128#10
# asm 2: movdqa <f0=%xmm0,>f0_stack=144(%rsp)
movdqa %xmm0,144(%rsp)

# qhasm: 2x h1 += r
# asm 1: paddq <r=reg128#15,<h1=reg128#14
# asm 2: paddq <r=%xmm14,<h1=%xmm13
paddq %xmm14,%xmm13

# qhasm: 2x h2 = g1 * f1
# asm 1: vpmuludq <f1=reg128#2,<g1=reg128#13,>h2=reg128#1
# asm 2: vpmuludq <f1=%xmm1,<g1=%xmm12,>h2=%xmm0
vpmuludq %xmm1,%xmm12,%xmm0

# qhasm: f1_2_stack = f1
# asm 1: movdqa <f1=reg128#2,>f1_2_stack=stack128#29
# asm 2: movdqa <f1=%xmm1,>f1_2_stack=448(%rsp)
movdqa %xmm1,448(%rsp)

# qhasm: 2x r = f2 + f3
# asm 1: vpaddq <f3=reg128#4,<f2=reg128#3,>r=reg128#2
# asm 2: vpaddq <f3=%xmm3,<f2=%xmm2,>r=%xmm1
vpaddq %xmm3,%xmm2,%xmm1

# qhasm: 2x f2 += mem128[ subc2 ]
# asm 1: paddq subc2,<f2=reg128#3
# asm 2: paddq subc2,<f2=%xmm2
paddq subc2(%rip),%xmm2

# qhasm: 2x f2 -= f3
# asm 1: psubq <f3=reg128#4,<f2=reg128#3
# asm 2: psubq <f3=%xmm3,<f2=%xmm2
psubq %xmm3,%xmm2

# qhasm: f3 = unpack_high(f2, r)
# asm 1: vpunpckhqdq <r=reg128#2,<f2=reg128#3,>f3=reg128#4
# asm 2: vpunpckhqdq <r=%xmm1,<f2=%xmm2,>f3=%xmm3
vpunpckhqdq %xmm1,%xmm2,%xmm3

# qhasm: f2 = unpack_low(f2, r)
# asm 1: vpunpcklqdq <r=reg128#2,<f2=reg128#3,>f2=reg128#2
# asm 2: vpunpcklqdq <r=%xmm1,<f2=%xmm2,>f2=%xmm1
vpunpcklqdq %xmm1,%xmm2,%xmm1

# qhasm: 2x r = g0 * f2
# asm 1: vpmuludq <f2=reg128#2,<g0=reg128#11,>r=reg128#3
# asm 2: vpmuludq <f2=%xmm1,<g0=%xmm10,>r=%xmm2
vpmuludq %xmm1,%xmm10,%xmm2

# qhasm: 2x h2 += r
# asm 1: paddq <r=reg128#3,<h2=reg128#1
# asm 2: paddq <r=%xmm2,<h2=%xmm0
paddq %xmm2,%xmm0

# qhasm: 2x h3 = g0 * f3
# asm 1: vpmuludq <f3=reg128#4,<g0=reg128#11,>h3=reg128#3
# asm 2: vpmuludq <f3=%xmm3,<g0=%xmm10,>h3=%xmm2
vpmuludq %xmm3,%xmm10,%xmm2

# qhasm: f3_stack = f3
# asm 1: movdqa <f3=reg128#4,>f3_stack=stack128#30
# asm 2: movdqa <f3=%xmm3,>f3_stack=464(%rsp)
movdqa %xmm3,464(%rsp)

# qhasm: 2x f3 += f3
# asm 1: paddq <f3=reg128#4,<f3=reg128#4
# asm 2: paddq <f3=%xmm3,<f3=%xmm3
paddq %xmm3,%xmm3

# qhasm: 2x r = g1 * f2
# asm 1: vpmuludq <f2=reg128#2,<g1=reg128#13,>r=reg128#15
# asm 2: vpmuludq <f2=%xmm1,<g1=%xmm12,>r=%xmm14
vpmuludq %xmm1,%xmm12,%xmm14

# qhasm: f2_stack = f2
# asm 1: movdqa <f2=reg128#2,>f2_stack=stack128#31
# asm 2: movdqa <f2=%xmm1,>f2_stack=480(%rsp)
movdqa %xmm1,480(%rsp)

# qhasm: 2x h3 += r
# asm 1: paddq <r=reg128#15,<h3=reg128#3
# asm 2: paddq <r=%xmm14,<h3=%xmm2
paddq %xmm14,%xmm2

# qhasm: 2x h4 = g1 * f3
# asm 1: vpmuludq <f3=reg128#4,<g1=reg128#13,>h4=reg128#2
# asm 2: vpmuludq <f3=%xmm3,<g1=%xmm12,>h4=%xmm1
vpmuludq %xmm3,%xmm12,%xmm1

# qhasm: f3_2_stack = f3
# asm 1: movdqa <f3=reg128#4,>f3_2_stack=stack128#32
# asm 2: movdqa <f3=%xmm3,>f3_2_stack=496(%rsp)
movdqa %xmm3,496(%rsp)

# qhasm: 2x r = f4 + f5
# asm 1: vpaddq <f5=reg128#6,<f4=reg128#5,>r=reg128#4
# asm 2: vpaddq <f5=%xmm5,<f4=%xmm4,>r=%xmm3
vpaddq %xmm5,%xmm4,%xmm3

# qhasm: 2x f4 += mem128[ subc2 ]
# asm 1: paddq subc2,<f4=reg128#5
# asm 2: paddq subc2,<f4=%xmm4
paddq subc2(%rip),%xmm4

# qhasm: 2x f4 -= f5
# asm 1: psubq <f5=reg128#6,<f4=reg128#5
# asm 2: psubq <f5=%xmm5,<f4=%xmm4
psubq %xmm5,%xmm4

# qhasm: f5 = unpack_high(f4, r)
# asm 1: vpunpckhqdq <r=reg128#4,<f4=reg128#5,>f5=reg128#6
# asm 2: vpunpckhqdq <r=%xmm3,<f4=%xmm4,>f5=%xmm5
vpunpckhqdq %xmm3,%xmm4,%xmm5

# qhasm: f4 = unpack_low(f4, r)
# asm 1: vpunpcklqdq <r=reg128#4,<f4=reg128#5,>f4=reg128#4
# asm 2: vpunpcklqdq <r=%xmm3,<f4=%xmm4,>f4=%xmm3
vpunpcklqdq %xmm3,%xmm4,%xmm3

# qhasm: 2x r = g0 * f4
# asm 1: vpmuludq <f4=reg128#4,<g0=reg128#11,>r=reg128#5
# asm 2: vpmuludq <f4=%xmm3,<g0=%xmm10,>r=%xmm4
vpmuludq %xmm3,%xmm10,%xmm4

# qhasm: 2x h4 += r
# asm 1: paddq <r=reg128#5,<h4=reg128#2
# asm 2: paddq <r=%xmm4,<h4=%xmm1
paddq %xmm4,%xmm1

# qhasm: 2x h5 = g0 * f5
# asm 1: vpmuludq <f5=reg128#6,<g0=reg128#11,>h5=reg128#5
# asm 2: vpmuludq <f5=%xmm5,<g0=%xmm10,>h5=%xmm4
vpmuludq %xmm5,%xmm10,%xmm4

# qhasm: f5_stack = f5
# asm 1: movdqa <f5=reg128#6,>f5_stack=stack128#33
# asm 2: movdqa <f5=%xmm5,>f5_stack=512(%rsp)
movdqa %xmm5,512(%rsp)

# qhasm: 2x f5 += f5
# asm 1: paddq <f5=reg128#6,<f5=reg128#6
# asm 2: paddq <f5=%xmm5,<f5=%xmm5
paddq %xmm5,%xmm5

# qhasm: 2x r = g1 * f4
# asm 1: vpmuludq <f4=reg128#4,<g1=reg128#13,>r=reg128#15
# asm 2: vpmuludq <f4=%xmm3,<g1=%xmm12,>r=%xmm14
vpmuludq %xmm3,%xmm12,%xmm14

# qhasm: f4_stack = f4
# asm 1: movdqa <f4=reg128#4,>f4_stack=stack128#34
# asm 2: movdqa <f4=%xmm3,>f4_stack=528(%rsp)
movdqa %xmm3,528(%rsp)

# qhasm: 2x h5 += r
# asm 1: paddq <r=reg128#15,<h5=reg128#5
# asm 2: paddq <r=%xmm14,<h5=%xmm4
paddq %xmm14,%xmm4

# qhasm: 2x r = f6 + f7
# asm 1: vpaddq <f7=reg128#8,<f6=reg128#7,>r=reg128#4
# asm 2: vpaddq <f7=%xmm7,<f6=%xmm6,>r=%xmm3
vpaddq %xmm7,%xmm6,%xmm3

# qhasm: 2x f6 += mem128[ subc2 ]
# asm 1: paddq subc2,<f6=reg128#7
# asm 2: paddq subc2,<f6=%xmm6
paddq subc2(%rip),%xmm6

# qhasm: 2x f6 -= f7
# asm 1: psubq <f7=reg128#8,<f6=reg128#7
# asm 2: psubq <f7=%xmm7,<f6=%xmm6
psubq %xmm7,%xmm6

# qhasm: f7 = unpack_high(f6, r)
# asm 1: vpunpckhqdq <r=reg128#4,<f6=reg128#7,>f7=reg128#8
# asm 2: vpunpckhqdq <r=%xmm3,<f6=%xmm6,>f7=%xmm7
vpunpckhqdq %xmm3,%xmm6,%xmm7

# qhasm: f6 = unpack_low(f6, r)
# asm 1: vpunpcklqdq <r=reg128#4,<f6=reg128#7,>f6=reg128#4
# asm 2: vpunpcklqdq <r=%xmm3,<f6=%xmm6,>f6=%xmm3
vpunpcklqdq %xmm3,%xmm6,%xmm3

# qhasm: 2x h6 = g0 * f6
# asm 1: vpmuludq <f6=reg128#4,<g0=reg128#11,>h6=reg128#7
# asm 2: vpmuludq <f6=%xmm3,<g0=%xmm10,>h6=%xmm6
vpmuludq %xmm3,%xmm10,%xmm6

# qhasm: 2x r = g1 * f5
# asm 1: vpmuludq <f5=reg128#6,<g1=reg128#13,>r=reg128#15
# asm 2: vpmuludq <f5=%xmm5,<g1=%xmm12,>r=%xmm14
vpmuludq %xmm5,%xmm12,%xmm14

# qhasm: f5_2_stack = f5
# asm 1: movdqa <f5=reg128#6,>f5_2_stack=stack128#35
# asm 2: movdqa <f5=%xmm5,>f5_2_stack=544(%rsp)
movdqa %xmm5,544(%rsp)

# qhasm: 2x f5 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<f5=reg128#6
# asm 2: pmuludq v19_19,<f5=%xmm5
pmuludq v19_19(%rip),%xmm5

# qhasm: f5_38_stack = f5
# asm 1: movdqa <f5=reg128#6,>f5_38_stack=stack128#36
# asm 2: movdqa <f5=%xmm5,>f5_38_stack=560(%rsp)
movdqa %xmm5,560(%rsp)

# qhasm: 2x h6 += r
# asm 1: paddq <r=reg128#15,<h6=reg128#7
# asm 2: paddq <r=%xmm14,<h6=%xmm6
paddq %xmm14,%xmm6

# qhasm: 2x h7 = g0 * f7
# asm 1: vpmuludq <f7=reg128#8,<g0=reg128#11,>h7=reg128#6
# asm 2: vpmuludq <f7=%xmm7,<g0=%xmm10,>h7=%xmm5
vpmuludq %xmm7,%xmm10,%xmm5

# qhasm: f7_stack = f7
# asm 1: movdqa <f7=reg128#8,>f7_stack=stack128#37
# asm 2: movdqa <f7=%xmm7,>f7_stack=576(%rsp)
movdqa %xmm7,576(%rsp)

# qhasm: 2x f7 += f7
# asm 1: paddq <f7=reg128#8,<f7=reg128#8
# asm 2: paddq <f7=%xmm7,<f7=%xmm7
paddq %xmm7,%xmm7

# qhasm: 2x r = g1 * f6
# asm 1: vpmuludq <f6=reg128#4,<g1=reg128#13,>r=reg128#15
# asm 2: vpmuludq <f6=%xmm3,<g1=%xmm12,>r=%xmm14
vpmuludq %xmm3,%xmm12,%xmm14

# qhasm: f6_stack = f6
# asm 1: movdqa <f6=reg128#4,>f6_stack=stack128#38
# asm 2: movdqa <f6=%xmm3,>f6_stack=592(%rsp)
movdqa %xmm3,592(%rsp)

# qhasm: 2x h7 += r
# asm 1: paddq <r=reg128#15,<h7=reg128#6
# asm 2: paddq <r=%xmm14,<h7=%xmm5
paddq %xmm14,%xmm5

# qhasm: 2x f6 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<f6=reg128#4
# asm 2: pmuludq v19_19,<f6=%xmm3
pmuludq v19_19(%rip),%xmm3

# qhasm: f6_19_stack = f6
# asm 1: movdqa <f6=reg128#4,>f6_19_stack=stack128#39
# asm 2: movdqa <f6=%xmm3,>f6_19_stack=608(%rsp)
movdqa %xmm3,608(%rsp)

# qhasm: 2x r = f8 + f9
# asm 1: vpaddq <f9=reg128#10,<f8=reg128#9,>r=reg128#4
# asm 2: vpaddq <f9=%xmm9,<f8=%xmm8,>r=%xmm3
vpaddq %xmm9,%xmm8,%xmm3

# qhasm: 2x f8 += mem128[ subc2 ]
# asm 1: paddq subc2,<f8=reg128#9
# asm 2: paddq subc2,<f8=%xmm8
paddq subc2(%rip),%xmm8

# qhasm: 2x f8 -= f9
# asm 1: psubq <f9=reg128#10,<f8=reg128#9
# asm 2: psubq <f9=%xmm9,<f8=%xmm8
psubq %xmm9,%xmm8

# qhasm: f9 = unpack_high(f8, r)
# asm 1: vpunpckhqdq <r=reg128#4,<f8=reg128#9,>f9=reg128#10
# asm 2: vpunpckhqdq <r=%xmm3,<f8=%xmm8,>f9=%xmm9
vpunpckhqdq %xmm3,%xmm8,%xmm9

# qhasm: f8 = unpack_low(f8, r)
# asm 1: vpunpcklqdq <r=reg128#4,<f8=reg128#9,>f8=reg128#4
# asm 2: vpunpcklqdq <r=%xmm3,<f8=%xmm8,>f8=%xmm3
vpunpcklqdq %xmm3,%xmm8,%xmm3

# qhasm: f8_stack = f8
# asm 1: movdqa <f8=reg128#4,>f8_stack=stack128#40
# asm 2: movdqa <f8=%xmm3,>f8_stack=624(%rsp)
movdqa %xmm3,624(%rsp)

# qhasm: 2x h8 = g1 * f7
# asm 1: vpmuludq <f7=reg128#8,<g1=reg128#13,>h8=reg128#9
# asm 2: vpmuludq <f7=%xmm7,<g1=%xmm12,>h8=%xmm8
vpmuludq %xmm7,%xmm12,%xmm8

# qhasm: f7_2_stack = f7
# asm 1: movdqa <f7=reg128#8,>f7_2_stack=stack128#41
# asm 2: movdqa <f7=%xmm7,>f7_2_stack=640(%rsp)
movdqa %xmm7,640(%rsp)

# qhasm: 2x f7 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<f7=reg128#8
# asm 2: pmuludq v19_19,<f7=%xmm7
pmuludq v19_19(%rip),%xmm7

# qhasm: f7_38_stack = f7
# asm 1: movdqa <f7=reg128#8,>f7_38_stack=stack128#42
# asm 2: movdqa <f7=%xmm7,>f7_38_stack=656(%rsp)
movdqa %xmm7,656(%rsp)

# qhasm: 2x r = g0 * f8
# asm 1: vpmuludq <f8=reg128#4,<g0=reg128#11,>r=reg128#8
# asm 2: vpmuludq <f8=%xmm3,<g0=%xmm10,>r=%xmm7
vpmuludq %xmm3,%xmm10,%xmm7

# qhasm: 2x h8 += r
# asm 1: paddq <r=reg128#8,<h8=reg128#9
# asm 2: paddq <r=%xmm7,<h8=%xmm8
paddq %xmm7,%xmm8

# qhasm: 2x h9 = g0 * f9
# asm 1: vpmuludq <f9=reg128#10,<g0=reg128#11,>h9=reg128#8
# asm 2: vpmuludq <f9=%xmm9,<g0=%xmm10,>h9=%xmm7
vpmuludq %xmm9,%xmm10,%xmm7

# qhasm: f9_stack = f9
# asm 1: movdqa <f9=reg128#10,>f9_stack=stack128#43
# asm 2: movdqa <f9=%xmm9,>f9_stack=672(%rsp)
movdqa %xmm9,672(%rsp)

# qhasm: 2x f9 += f9
# asm 1: paddq <f9=reg128#10,<f9=reg128#10
# asm 2: paddq <f9=%xmm9,<f9=%xmm9
paddq %xmm9,%xmm9

# qhasm: 2x r = g1 * f8
# asm 1: vpmuludq <f8=reg128#4,<g1=reg128#13,>r=reg128#11
# asm 2: vpmuludq <f8=%xmm3,<g1=%xmm12,>r=%xmm10
vpmuludq %xmm3,%xmm12,%xmm10

# qhasm: 2x h9 += r
# asm 1: paddq <r=reg128#11,<h9=reg128#8
# asm 2: paddq <r=%xmm10,<h9=%xmm7
paddq %xmm10,%xmm7

# qhasm: 2x f8 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<f8=reg128#4
# asm 2: pmuludq v19_19,<f8=%xmm3
pmuludq v19_19(%rip),%xmm3

# qhasm: f8_19_stack = f8
# asm 1: movdqa <f8=reg128#4,>f8_19_stack=stack128#44
# asm 2: movdqa <f8=%xmm3,>f8_19_stack=688(%rsp)
movdqa %xmm3,688(%rsp)

# qhasm: 2x g1 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g1=reg128#13
# asm 2: pmuludq v19_19,<g1=%xmm12
pmuludq v19_19(%rip),%xmm12

# qhasm: 2x r = g1 * f9
# asm 1: vpmuludq <f9=reg128#10,<g1=reg128#13,>r=reg128#4
# asm 2: vpmuludq <f9=%xmm9,<g1=%xmm12,>r=%xmm3
vpmuludq %xmm9,%xmm12,%xmm3

# qhasm: f9_2_stack = f9
# asm 1: movdqa <f9=reg128#10,>f9_2_stack=stack128#45
# asm 2: movdqa <f9=%xmm9,>f9_2_stack=704(%rsp)
movdqa %xmm9,704(%rsp)

# qhasm: 2x h0 += r
# asm 1: paddq <r=reg128#4,<h0=reg128#12
# asm 2: paddq <r=%xmm3,<h0=%xmm11
paddq %xmm3,%xmm11

# qhasm: g2 = x3_2
# asm 1: movdqa <x3_2=stack128#1,>g2=reg128#4
# asm 2: movdqa <x3_2=0(%rsp),>g2=%xmm3
movdqa 0(%rsp),%xmm3

# qhasm: g3 = z3_2
# asm 1: movdqa <z3_2=stack128#2,>g3=reg128#10
# asm 2: movdqa <z3_2=16(%rsp),>g3=%xmm9
movdqa 16(%rsp),%xmm9

# qhasm: 2x r = g2 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<g2=reg128#4,>r=reg128#11
# asm 2: vpaddq subc2,<g2=%xmm3,>r=%xmm10
vpaddq subc2(%rip),%xmm3,%xmm10

# qhasm: 2x r -= g3
# asm 1: psubq <g3=reg128#10,<r=reg128#11
# asm 2: psubq <g3=%xmm9,<r=%xmm10
psubq %xmm9,%xmm10

# qhasm: 2x g2 += g3
# asm 1: paddq <g3=reg128#10,<g2=reg128#4
# asm 2: paddq <g3=%xmm9,<g2=%xmm3
paddq %xmm9,%xmm3

# qhasm: g3 = unpack_high(g2, r)
# asm 1: vpunpckhqdq <r=reg128#11,<g2=reg128#4,>g3=reg128#10
# asm 2: vpunpckhqdq <r=%xmm10,<g2=%xmm3,>g3=%xmm9
vpunpckhqdq %xmm10,%xmm3,%xmm9

# qhasm: g2 = unpack_low(g2, r)
# asm 1: vpunpcklqdq <r=reg128#11,<g2=reg128#4,>g2=reg128#4
# asm 2: vpunpcklqdq <r=%xmm10,<g2=%xmm3,>g2=%xmm3
vpunpcklqdq %xmm10,%xmm3,%xmm3

# qhasm: 2x r2 = g2 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f0_stack=144(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 144(%rsp),%xmm3,%xmm10

# qhasm: 2x h2 += r2
# asm 1: paddq <r2=reg128#11,<h2=reg128#1
# asm 2: paddq <r2=%xmm10,<h2=%xmm0
paddq %xmm10,%xmm0

# qhasm: 2x r2 = g2 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#9,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f1_stack=128(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 128(%rsp),%xmm3,%xmm10

# qhasm: 2x h3 += r2
# asm 1: paddq <r2=reg128#11,<h3=reg128#3
# asm 2: paddq <r2=%xmm10,<h3=%xmm2
paddq %xmm10,%xmm2

# qhasm: 2x r2 = g2 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f2_stack=480(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 480(%rsp),%xmm3,%xmm10

# qhasm: 2x h4 += r2
# asm 1: paddq <r2=reg128#11,<h4=reg128#2
# asm 2: paddq <r2=%xmm10,<h4=%xmm1
paddq %xmm10,%xmm1

# qhasm: 2x r2 = g2 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#30,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f3_stack=464(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 464(%rsp),%xmm3,%xmm10

# qhasm: 2x h5 += r2
# asm 1: paddq <r2=reg128#11,<h5=reg128#5
# asm 2: paddq <r2=%xmm10,<h5=%xmm4
paddq %xmm10,%xmm4

# qhasm: 2x r2 = g2 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f4_stack=528(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 528(%rsp),%xmm3,%xmm10

# qhasm: 2x h6 += r2
# asm 1: paddq <r2=reg128#11,<h6=reg128#7
# asm 2: paddq <r2=%xmm10,<h6=%xmm6
paddq %xmm10,%xmm6

# qhasm: 2x r2 = g2 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#33,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f5_stack=512(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 512(%rsp),%xmm3,%xmm10

# qhasm: 2x h7 += r2
# asm 1: paddq <r2=reg128#11,<h7=reg128#6
# asm 2: paddq <r2=%xmm10,<h7=%xmm5
paddq %xmm10,%xmm5

# qhasm: 2x r2 = g2 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f6_stack=592(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 592(%rsp),%xmm3,%xmm10

# qhasm: 2x h8 += r2
# asm 1: paddq <r2=reg128#11,<h8=reg128#9
# asm 2: paddq <r2=%xmm10,<h8=%xmm8
paddq %xmm10,%xmm8

# qhasm: 2x r2 = g2 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#37,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f7_stack=576(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 576(%rsp),%xmm3,%xmm10

# qhasm: 2x h9 += r2
# asm 1: paddq <r2=reg128#11,<h9=reg128#8
# asm 2: paddq <r2=%xmm10,<h9=%xmm7
paddq %xmm10,%xmm7

# qhasm: 2x g2 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g2=reg128#4
# asm 2: pmuludq v19_19,<g2=%xmm3
pmuludq v19_19(%rip),%xmm3

# qhasm: 2x r2 = g2 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g2=reg128#4,>r2=reg128#11
# asm 2: vpmuludq <f8_stack=624(%rsp),<g2=%xmm3,>r2=%xmm10
vpmuludq 624(%rsp),%xmm3,%xmm10

# qhasm: 2x h0 += r2
# asm 1: paddq <r2=reg128#11,<h0=reg128#12
# asm 2: paddq <r2=%xmm10,<h0=%xmm11
paddq %xmm10,%xmm11

# qhasm: 2x g2 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#43,<g2=reg128#4
# asm 2: pmuludq <f9_stack=672(%rsp),<g2=%xmm3
pmuludq 672(%rsp),%xmm3

# qhasm: 2x h1 += g2
# asm 1: paddq <g2=reg128#4,<h1=reg128#14
# asm 2: paddq <g2=%xmm3,<h1=%xmm13
paddq %xmm3,%xmm13

# qhasm: 2x r3 = g3 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f0_stack=144(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 144(%rsp),%xmm9,%xmm3

# qhasm: 2x h3 += r3
# asm 1: paddq <r3=reg128#4,<h3=reg128#3
# asm 2: paddq <r3=%xmm3,<h3=%xmm2
paddq %xmm3,%xmm2

# qhasm: 2x r3 = g3 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#29,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f1_2_stack=448(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 448(%rsp),%xmm9,%xmm3

# qhasm: 2x h4 += r3
# asm 1: paddq <r3=reg128#4,<h4=reg128#2
# asm 2: paddq <r3=%xmm3,<h4=%xmm1
paddq %xmm3,%xmm1

# qhasm: 2x r3 = g3 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f2_stack=480(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 480(%rsp),%xmm9,%xmm3

# qhasm: 2x h5 += r3
# asm 1: paddq <r3=reg128#4,<h5=reg128#5
# asm 2: paddq <r3=%xmm3,<h5=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r3 = g3 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#32,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f3_2_stack=496(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 496(%rsp),%xmm9,%xmm3

# qhasm: 2x h6 += r3
# asm 1: paddq <r3=reg128#4,<h6=reg128#7
# asm 2: paddq <r3=%xmm3,<h6=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r3 = g3 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f4_stack=528(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 528(%rsp),%xmm9,%xmm3

# qhasm: 2x h7 += r3
# asm 1: paddq <r3=reg128#4,<h7=reg128#6
# asm 2: paddq <r3=%xmm3,<h7=%xmm5
paddq %xmm3,%xmm5

# qhasm: 2x r3 = g3 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#35,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f5_2_stack=544(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 544(%rsp),%xmm9,%xmm3

# qhasm: 2x h8 += r3
# asm 1: paddq <r3=reg128#4,<h8=reg128#9
# asm 2: paddq <r3=%xmm3,<h8=%xmm8
paddq %xmm3,%xmm8

# qhasm: 2x r3 = g3 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f6_stack=592(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 592(%rsp),%xmm9,%xmm3

# qhasm: 2x h9 += r3
# asm 1: paddq <r3=reg128#4,<h9=reg128#8
# asm 2: paddq <r3=%xmm3,<h9=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x g3 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g3=reg128#10
# asm 2: pmuludq v19_19,<g3=%xmm9
pmuludq v19_19(%rip),%xmm9

# qhasm: 2x r3 = g3 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#41,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f7_2_stack=640(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 640(%rsp),%xmm9,%xmm3

# qhasm: 2x h0 += r3
# asm 1: paddq <r3=reg128#4,<h0=reg128#12
# asm 2: paddq <r3=%xmm3,<h0=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r3 = g3 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g3=reg128#10,>r3=reg128#4
# asm 2: vpmuludq <f8_stack=624(%rsp),<g3=%xmm9,>r3=%xmm3
vpmuludq 624(%rsp),%xmm9,%xmm3

# qhasm: 2x h1 += r3
# asm 1: paddq <r3=reg128#4,<h1=reg128#14
# asm 2: paddq <r3=%xmm3,<h1=%xmm13
paddq %xmm3,%xmm13

# qhasm: 2x g3 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#45,<g3=reg128#10
# asm 2: pmuludq <f9_2_stack=704(%rsp),<g3=%xmm9
pmuludq 704(%rsp),%xmm9

# qhasm: 2x h2 += g3
# asm 1: paddq <g3=reg128#10,<h2=reg128#1
# asm 2: paddq <g3=%xmm9,<h2=%xmm0
paddq %xmm9,%xmm0

# qhasm: g4 = x3_4
# asm 1: movdqa <x3_4=stack128#3,>g4=reg128#4
# asm 2: movdqa <x3_4=32(%rsp),>g4=%xmm3
movdqa 32(%rsp),%xmm3

# qhasm: g5 = z3_4
# asm 1: movdqa <z3_4=stack128#6,>g5=reg128#10
# asm 2: movdqa <z3_4=80(%rsp),>g5=%xmm9
movdqa 80(%rsp),%xmm9

# qhasm: 2x r = g4 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<g4=reg128#4,>r=reg128#11
# asm 2: vpaddq subc2,<g4=%xmm3,>r=%xmm10
vpaddq subc2(%rip),%xmm3,%xmm10

# qhasm: 2x r -= g5
# asm 1: psubq <g5=reg128#10,<r=reg128#11
# asm 2: psubq <g5=%xmm9,<r=%xmm10
psubq %xmm9,%xmm10

# qhasm: 2x g4 += g5
# asm 1: paddq <g5=reg128#10,<g4=reg128#4
# asm 2: paddq <g5=%xmm9,<g4=%xmm3
paddq %xmm9,%xmm3

# qhasm: g5 = unpack_high(g4, r)
# asm 1: vpunpckhqdq <r=reg128#11,<g4=reg128#4,>g5=reg128#10
# asm 2: vpunpckhqdq <r=%xmm10,<g4=%xmm3,>g5=%xmm9
vpunpckhqdq %xmm10,%xmm3,%xmm9

# qhasm: g4 = unpack_low(g4, r)
# asm 1: vpunpcklqdq <r=reg128#11,<g4=reg128#4,>g4=reg128#4
# asm 2: vpunpcklqdq <r=%xmm10,<g4=%xmm3,>g4=%xmm3
vpunpcklqdq %xmm10,%xmm3,%xmm3

# qhasm: 2x r4 = g4 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f0_stack=144(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 144(%rsp),%xmm3,%xmm10

# qhasm: 2x h4 += r4
# asm 1: paddq <r4=reg128#11,<h4=reg128#2
# asm 2: paddq <r4=%xmm10,<h4=%xmm1
paddq %xmm10,%xmm1

# qhasm: 2x r4 = g4 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#9,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f1_stack=128(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 128(%rsp),%xmm3,%xmm10

# qhasm: 2x h5 += r4
# asm 1: paddq <r4=reg128#11,<h5=reg128#5
# asm 2: paddq <r4=%xmm10,<h5=%xmm4
paddq %xmm10,%xmm4

# qhasm: 2x r4 = g4 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f2_stack=480(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 480(%rsp),%xmm3,%xmm10

# qhasm: 2x h6 += r4
# asm 1: paddq <r4=reg128#11,<h6=reg128#7
# asm 2: paddq <r4=%xmm10,<h6=%xmm6
paddq %xmm10,%xmm6

# qhasm: 2x r4 = g4 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#30,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f3_stack=464(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 464(%rsp),%xmm3,%xmm10

# qhasm: 2x h7 += r4
# asm 1: paddq <r4=reg128#11,<h7=reg128#6
# asm 2: paddq <r4=%xmm10,<h7=%xmm5
paddq %xmm10,%xmm5

# qhasm: 2x r4 = g4 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f4_stack=528(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 528(%rsp),%xmm3,%xmm10

# qhasm: 2x h8 += r4
# asm 1: paddq <r4=reg128#11,<h8=reg128#9
# asm 2: paddq <r4=%xmm10,<h8=%xmm8
paddq %xmm10,%xmm8

# qhasm: 2x r4 = g4 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#33,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f5_stack=512(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 512(%rsp),%xmm3,%xmm10

# qhasm: 2x h9 += r4
# asm 1: paddq <r4=reg128#11,<h9=reg128#8
# asm 2: paddq <r4=%xmm10,<h9=%xmm7
paddq %xmm10,%xmm7

# qhasm: 2x g4 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g4=reg128#4
# asm 2: pmuludq v19_19,<g4=%xmm3
pmuludq v19_19(%rip),%xmm3

# qhasm: 2x r4 = g4 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f6_stack=592(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 592(%rsp),%xmm3,%xmm10

# qhasm: 2x h0 += r4
# asm 1: paddq <r4=reg128#11,<h0=reg128#12
# asm 2: paddq <r4=%xmm10,<h0=%xmm11
paddq %xmm10,%xmm11

# qhasm: 2x r4 = g4 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#37,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f7_stack=576(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 576(%rsp),%xmm3,%xmm10

# qhasm: 2x h1 += r4
# asm 1: paddq <r4=reg128#11,<h1=reg128#14
# asm 2: paddq <r4=%xmm10,<h1=%xmm13
paddq %xmm10,%xmm13

# qhasm: 2x r4 = g4 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g4=reg128#4,>r4=reg128#11
# asm 2: vpmuludq <f8_stack=624(%rsp),<g4=%xmm3,>r4=%xmm10
vpmuludq 624(%rsp),%xmm3,%xmm10

# qhasm: 2x h2 += r4
# asm 1: paddq <r4=reg128#11,<h2=reg128#1
# asm 2: paddq <r4=%xmm10,<h2=%xmm0
paddq %xmm10,%xmm0

# qhasm: 2x g4 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#43,<g4=reg128#4
# asm 2: pmuludq <f9_stack=672(%rsp),<g4=%xmm3
pmuludq 672(%rsp),%xmm3

# qhasm: 2x h3 += g4
# asm 1: paddq <g4=reg128#4,<h3=reg128#3
# asm 2: paddq <g4=%xmm3,<h3=%xmm2
paddq %xmm3,%xmm2

# qhasm: 2x r5 = g5 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f0_stack=144(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 144(%rsp),%xmm9,%xmm3

# qhasm: 2x h5 += r5
# asm 1: paddq <r5=reg128#4,<h5=reg128#5
# asm 2: paddq <r5=%xmm3,<h5=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r5 = g5 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#29,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f1_2_stack=448(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 448(%rsp),%xmm9,%xmm3

# qhasm: 2x h6 += r5
# asm 1: paddq <r5=reg128#4,<h6=reg128#7
# asm 2: paddq <r5=%xmm3,<h6=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r5 = g5 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f2_stack=480(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 480(%rsp),%xmm9,%xmm3

# qhasm: 2x h7 += r5
# asm 1: paddq <r5=reg128#4,<h7=reg128#6
# asm 2: paddq <r5=%xmm3,<h7=%xmm5
paddq %xmm3,%xmm5

# qhasm: 2x r5 = g5 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#32,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f3_2_stack=496(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 496(%rsp),%xmm9,%xmm3

# qhasm: 2x h8 += r5
# asm 1: paddq <r5=reg128#4,<h8=reg128#9
# asm 2: paddq <r5=%xmm3,<h8=%xmm8
paddq %xmm3,%xmm8

# qhasm: 2x r5 = g5 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f4_stack=528(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 528(%rsp),%xmm9,%xmm3

# qhasm: 2x h9 += r5
# asm 1: paddq <r5=reg128#4,<h9=reg128#8
# asm 2: paddq <r5=%xmm3,<h9=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x g5 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g5=reg128#10
# asm 2: pmuludq v19_19,<g5=%xmm9
pmuludq v19_19(%rip),%xmm9

# qhasm: 2x r5 = g5 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#35,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f5_2_stack=544(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 544(%rsp),%xmm9,%xmm3

# qhasm: 2x h0 += r5
# asm 1: paddq <r5=reg128#4,<h0=reg128#12
# asm 2: paddq <r5=%xmm3,<h0=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r5 = g5 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f6_stack=592(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 592(%rsp),%xmm9,%xmm3

# qhasm: 2x h1 += r5
# asm 1: paddq <r5=reg128#4,<h1=reg128#14
# asm 2: paddq <r5=%xmm3,<h1=%xmm13
paddq %xmm3,%xmm13

# qhasm: 2x r5 = g5 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#41,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f7_2_stack=640(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 640(%rsp),%xmm9,%xmm3

# qhasm: 2x h2 += r5
# asm 1: paddq <r5=reg128#4,<h2=reg128#1
# asm 2: paddq <r5=%xmm3,<h2=%xmm0
paddq %xmm3,%xmm0

# qhasm: 2x r5 = g5 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g5=reg128#10,>r5=reg128#4
# asm 2: vpmuludq <f8_stack=624(%rsp),<g5=%xmm9,>r5=%xmm3
vpmuludq 624(%rsp),%xmm9,%xmm3

# qhasm: 2x h3 += r5
# asm 1: paddq <r5=reg128#4,<h3=reg128#3
# asm 2: paddq <r5=%xmm3,<h3=%xmm2
paddq %xmm3,%xmm2

# qhasm: 2x g5 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#45,<g5=reg128#10
# asm 2: pmuludq <f9_2_stack=704(%rsp),<g5=%xmm9
pmuludq 704(%rsp),%xmm9

# qhasm: 2x h4 += g5
# asm 1: paddq <g5=reg128#10,<h4=reg128#2
# asm 2: paddq <g5=%xmm9,<h4=%xmm1
paddq %xmm9,%xmm1

# qhasm: g6 = x3_6
# asm 1: movdqa <x3_6=stack128#4,>g6=reg128#4
# asm 2: movdqa <x3_6=48(%rsp),>g6=%xmm3
movdqa 48(%rsp),%xmm3

# qhasm: g7 = z3_6
# asm 1: movdqa <z3_6=stack128#7,>g7=reg128#10
# asm 2: movdqa <z3_6=96(%rsp),>g7=%xmm9
movdqa 96(%rsp),%xmm9

# qhasm: 2x r = g6 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<g6=reg128#4,>r=reg128#11
# asm 2: vpaddq subc2,<g6=%xmm3,>r=%xmm10
vpaddq subc2(%rip),%xmm3,%xmm10

# qhasm: 2x r -= g7
# asm 1: psubq <g7=reg128#10,<r=reg128#11
# asm 2: psubq <g7=%xmm9,<r=%xmm10
psubq %xmm9,%xmm10

# qhasm: 2x g6 += g7
# asm 1: paddq <g7=reg128#10,<g6=reg128#4
# asm 2: paddq <g7=%xmm9,<g6=%xmm3
paddq %xmm9,%xmm3

# qhasm: g7 = unpack_high(g6, r)
# asm 1: vpunpckhqdq <r=reg128#11,<g6=reg128#4,>g7=reg128#10
# asm 2: vpunpckhqdq <r=%xmm10,<g6=%xmm3,>g7=%xmm9
vpunpckhqdq %xmm10,%xmm3,%xmm9

# qhasm: g6 = unpack_low(g6, r)
# asm 1: vpunpcklqdq <r=reg128#11,<g6=reg128#4,>g6=reg128#4
# asm 2: vpunpcklqdq <r=%xmm10,<g6=%xmm3,>g6=%xmm3
vpunpcklqdq %xmm10,%xmm3,%xmm3

# qhasm: 2x r6 = g6 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f0_stack=144(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 144(%rsp),%xmm3,%xmm10

# qhasm: 2x h6 += r6
# asm 1: paddq <r6=reg128#11,<h6=reg128#7
# asm 2: paddq <r6=%xmm10,<h6=%xmm6
paddq %xmm10,%xmm6

# qhasm: 2x r6 = g6 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#9,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f1_stack=128(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 128(%rsp),%xmm3,%xmm10

# qhasm: 2x h7 += r6
# asm 1: paddq <r6=reg128#11,<h7=reg128#6
# asm 2: paddq <r6=%xmm10,<h7=%xmm5
paddq %xmm10,%xmm5

# qhasm: 2x r6 = g6 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f2_stack=480(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 480(%rsp),%xmm3,%xmm10

# qhasm: 2x h8 += r6
# asm 1: paddq <r6=reg128#11,<h8=reg128#9
# asm 2: paddq <r6=%xmm10,<h8=%xmm8
paddq %xmm10,%xmm8

# qhasm: 2x r6 = g6 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#30,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f3_stack=464(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 464(%rsp),%xmm3,%xmm10

# qhasm: 2x h9 += r6
# asm 1: paddq <r6=reg128#11,<h9=reg128#8
# asm 2: paddq <r6=%xmm10,<h9=%xmm7
paddq %xmm10,%xmm7

# qhasm: 2x g6 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g6=reg128#4
# asm 2: pmuludq v19_19,<g6=%xmm3
pmuludq v19_19(%rip),%xmm3

# qhasm: 2x r6 = g6 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f4_stack=528(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 528(%rsp),%xmm3,%xmm10

# qhasm: 2x h0 += r6
# asm 1: paddq <r6=reg128#11,<h0=reg128#12
# asm 2: paddq <r6=%xmm10,<h0=%xmm11
paddq %xmm10,%xmm11

# qhasm: 2x r6 = g6 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#33,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f5_stack=512(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 512(%rsp),%xmm3,%xmm10

# qhasm: 2x h1 += r6
# asm 1: paddq <r6=reg128#11,<h1=reg128#14
# asm 2: paddq <r6=%xmm10,<h1=%xmm13
paddq %xmm10,%xmm13

# qhasm: 2x r6 = g6 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f6_stack=592(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 592(%rsp),%xmm3,%xmm10

# qhasm: 2x h2 += r6
# asm 1: paddq <r6=reg128#11,<h2=reg128#1
# asm 2: paddq <r6=%xmm10,<h2=%xmm0
paddq %xmm10,%xmm0

# qhasm: 2x r6 = g6 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#37,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f7_stack=576(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 576(%rsp),%xmm3,%xmm10

# qhasm: 2x h3 += r6
# asm 1: paddq <r6=reg128#11,<h3=reg128#3
# asm 2: paddq <r6=%xmm10,<h3=%xmm2
paddq %xmm10,%xmm2

# qhasm: 2x r6 = g6 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g6=reg128#4,>r6=reg128#11
# asm 2: vpmuludq <f8_stack=624(%rsp),<g6=%xmm3,>r6=%xmm10
vpmuludq 624(%rsp),%xmm3,%xmm10

# qhasm: 2x h4 += r6
# asm 1: paddq <r6=reg128#11,<h4=reg128#2
# asm 2: paddq <r6=%xmm10,<h4=%xmm1
paddq %xmm10,%xmm1

# qhasm: 2x g6 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#43,<g6=reg128#4
# asm 2: pmuludq <f9_stack=672(%rsp),<g6=%xmm3
pmuludq 672(%rsp),%xmm3

# qhasm: 2x h5 += g6
# asm 1: paddq <g6=reg128#4,<h5=reg128#5
# asm 2: paddq <g6=%xmm3,<h5=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r7 = g7 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f0_stack=144(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 144(%rsp),%xmm9,%xmm3

# qhasm: 2x h7 += r7
# asm 1: paddq <r7=reg128#4,<h7=reg128#6
# asm 2: paddq <r7=%xmm3,<h7=%xmm5
paddq %xmm3,%xmm5

# qhasm: 2x r7 = g7 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#29,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f1_2_stack=448(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 448(%rsp),%xmm9,%xmm3

# qhasm: 2x h8 += r7
# asm 1: paddq <r7=reg128#4,<h8=reg128#9
# asm 2: paddq <r7=%xmm3,<h8=%xmm8
paddq %xmm3,%xmm8

# qhasm: 2x r7 = g7 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f2_stack=480(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 480(%rsp),%xmm9,%xmm3

# qhasm: 2x h9 += r7
# asm 1: paddq <r7=reg128#4,<h9=reg128#8
# asm 2: paddq <r7=%xmm3,<h9=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x g7 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g7=reg128#10
# asm 2: pmuludq v19_19,<g7=%xmm9
pmuludq v19_19(%rip),%xmm9

# qhasm: 2x r7 = g7 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#32,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f3_2_stack=496(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 496(%rsp),%xmm9,%xmm3

# qhasm: 2x h0 += r7
# asm 1: paddq <r7=reg128#4,<h0=reg128#12
# asm 2: paddq <r7=%xmm3,<h0=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r7 = g7 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f4_stack=528(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 528(%rsp),%xmm9,%xmm3

# qhasm: 2x h1 += r7
# asm 1: paddq <r7=reg128#4,<h1=reg128#14
# asm 2: paddq <r7=%xmm3,<h1=%xmm13
paddq %xmm3,%xmm13

# qhasm: 2x r7 = g7 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#35,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f5_2_stack=544(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 544(%rsp),%xmm9,%xmm3

# qhasm: 2x h2 += r7
# asm 1: paddq <r7=reg128#4,<h2=reg128#1
# asm 2: paddq <r7=%xmm3,<h2=%xmm0
paddq %xmm3,%xmm0

# qhasm: 2x r7 = g7 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f6_stack=592(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 592(%rsp),%xmm9,%xmm3

# qhasm: 2x h3 += r7
# asm 1: paddq <r7=reg128#4,<h3=reg128#3
# asm 2: paddq <r7=%xmm3,<h3=%xmm2
paddq %xmm3,%xmm2

# qhasm: 2x r7 = g7 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#41,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f7_2_stack=640(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 640(%rsp),%xmm9,%xmm3

# qhasm: 2x h4 += r7
# asm 1: paddq <r7=reg128#4,<h4=reg128#2
# asm 2: paddq <r7=%xmm3,<h4=%xmm1
paddq %xmm3,%xmm1

# qhasm: 2x r7 = g7 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g7=reg128#10,>r7=reg128#4
# asm 2: vpmuludq <f8_stack=624(%rsp),<g7=%xmm9,>r7=%xmm3
vpmuludq 624(%rsp),%xmm9,%xmm3

# qhasm: 2x h5 += r7
# asm 1: paddq <r7=reg128#4,<h5=reg128#5
# asm 2: paddq <r7=%xmm3,<h5=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x g7 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#45,<g7=reg128#10
# asm 2: pmuludq <f9_2_stack=704(%rsp),<g7=%xmm9
pmuludq 704(%rsp),%xmm9

# qhasm: 2x h6 += g7
# asm 1: paddq <g7=reg128#10,<h6=reg128#7
# asm 2: paddq <g7=%xmm9,<h6=%xmm6
paddq %xmm9,%xmm6

# qhasm: g8 = x3_8
# asm 1: movdqa <x3_8=stack128#5,>g8=reg128#4
# asm 2: movdqa <x3_8=64(%rsp),>g8=%xmm3
movdqa 64(%rsp),%xmm3

# qhasm: g9 = z3_8
# asm 1: movdqa <z3_8=stack128#8,>g9=reg128#10
# asm 2: movdqa <z3_8=112(%rsp),>g9=%xmm9
movdqa 112(%rsp),%xmm9

# qhasm: 2x r = g8 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<g8=reg128#4,>r=reg128#11
# asm 2: vpaddq subc2,<g8=%xmm3,>r=%xmm10
vpaddq subc2(%rip),%xmm3,%xmm10

# qhasm: 2x r -= g9
# asm 1: psubq <g9=reg128#10,<r=reg128#11
# asm 2: psubq <g9=%xmm9,<r=%xmm10
psubq %xmm9,%xmm10

# qhasm: 2x g8 += g9
# asm 1: paddq <g9=reg128#10,<g8=reg128#4
# asm 2: paddq <g9=%xmm9,<g8=%xmm3
paddq %xmm9,%xmm3

# qhasm: g9 = unpack_high(g8, r)
# asm 1: vpunpckhqdq <r=reg128#11,<g8=reg128#4,>g9=reg128#10
# asm 2: vpunpckhqdq <r=%xmm10,<g8=%xmm3,>g9=%xmm9
vpunpckhqdq %xmm10,%xmm3,%xmm9

# qhasm: g8 = unpack_low(g8, r)
# asm 1: vpunpcklqdq <r=reg128#11,<g8=reg128#4,>g8=reg128#4
# asm 2: vpunpcklqdq <r=%xmm10,<g8=%xmm3,>g8=%xmm3
vpunpcklqdq %xmm10,%xmm3,%xmm3

# qhasm: 2x r8 = g8 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f0_stack=144(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 144(%rsp),%xmm3,%xmm10

# qhasm: 2x h8 += r8
# asm 1: paddq <r8=reg128#11,<h8=reg128#9
# asm 2: paddq <r8=%xmm10,<h8=%xmm8
paddq %xmm10,%xmm8

# qhasm: 2x r8 = g8 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#9,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f1_stack=128(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 128(%rsp),%xmm3,%xmm10

# qhasm: 2x h9 += r8
# asm 1: paddq <r8=reg128#11,<h9=reg128#8
# asm 2: paddq <r8=%xmm10,<h9=%xmm7
paddq %xmm10,%xmm7

# qhasm: 2x g8 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g8=reg128#4
# asm 2: pmuludq v19_19,<g8=%xmm3
pmuludq v19_19(%rip),%xmm3

# qhasm: 2x r8 = g8 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f2_stack=480(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 480(%rsp),%xmm3,%xmm10

# qhasm: 2x h0 += r8
# asm 1: paddq <r8=reg128#11,<h0=reg128#12
# asm 2: paddq <r8=%xmm10,<h0=%xmm11
paddq %xmm10,%xmm11

# qhasm: 2x r8 = g8 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#30,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f3_stack=464(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 464(%rsp),%xmm3,%xmm10

# qhasm: 2x h1 += r8
# asm 1: paddq <r8=reg128#11,<h1=reg128#14
# asm 2: paddq <r8=%xmm10,<h1=%xmm13
paddq %xmm10,%xmm13

# qhasm: 2x r8 = g8 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f4_stack=528(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 528(%rsp),%xmm3,%xmm10

# qhasm: 2x h2 += r8
# asm 1: paddq <r8=reg128#11,<h2=reg128#1
# asm 2: paddq <r8=%xmm10,<h2=%xmm0
paddq %xmm10,%xmm0

# qhasm: 2x r8 = g8 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#33,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f5_stack=512(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 512(%rsp),%xmm3,%xmm10

# qhasm: 2x h3 += r8
# asm 1: paddq <r8=reg128#11,<h3=reg128#3
# asm 2: paddq <r8=%xmm10,<h3=%xmm2
paddq %xmm10,%xmm2

# qhasm: 2x r8 = g8 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f6_stack=592(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 592(%rsp),%xmm3,%xmm10

# qhasm: 2x h4 += r8
# asm 1: paddq <r8=reg128#11,<h4=reg128#2
# asm 2: paddq <r8=%xmm10,<h4=%xmm1
paddq %xmm10,%xmm1

# qhasm: 2x r8 = g8 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#37,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f7_stack=576(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 576(%rsp),%xmm3,%xmm10

# qhasm: 2x h5 += r8
# asm 1: paddq <r8=reg128#11,<h5=reg128#5
# asm 2: paddq <r8=%xmm10,<h5=%xmm4
paddq %xmm10,%xmm4

# qhasm: 2x r8 = g8 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g8=reg128#4,>r8=reg128#11
# asm 2: vpmuludq <f8_stack=624(%rsp),<g8=%xmm3,>r8=%xmm10
vpmuludq 624(%rsp),%xmm3,%xmm10

# qhasm: 2x h6 += r8
# asm 1: paddq <r8=reg128#11,<h6=reg128#7
# asm 2: paddq <r8=%xmm10,<h6=%xmm6
paddq %xmm10,%xmm6

# qhasm: 2x g8 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#43,<g8=reg128#4
# asm 2: pmuludq <f9_stack=672(%rsp),<g8=%xmm3
pmuludq 672(%rsp),%xmm3

# qhasm: 2x h7 += g8
# asm 1: paddq <g8=reg128#4,<h7=reg128#6
# asm 2: paddq <g8=%xmm3,<h7=%xmm5
paddq %xmm3,%xmm5

# qhasm: 2x r9 = g9 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#10,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f0_stack=144(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 144(%rsp),%xmm9,%xmm3

# qhasm: 2x h9 += r9
# asm 1: paddq <r9=reg128#4,<h9=reg128#8
# asm 2: paddq <r9=%xmm3,<h9=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x g9 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g9=reg128#10
# asm 2: pmuludq v19_19,<g9=%xmm9
pmuludq v19_19(%rip),%xmm9

# qhasm: 2x r9 = g9 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#29,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f1_2_stack=448(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 448(%rsp),%xmm9,%xmm3

# qhasm: 2x h0 += r9
# asm 1: paddq <r9=reg128#4,<h0=reg128#12
# asm 2: paddq <r9=%xmm3,<h0=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r9 = g9 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#31,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f2_stack=480(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 480(%rsp),%xmm9,%xmm3

# qhasm: 2x h1 += r9
# asm 1: paddq <r9=reg128#4,<h1=reg128#14
# asm 2: paddq <r9=%xmm3,<h1=%xmm13
paddq %xmm3,%xmm13

# qhasm: 2x r9 = g9 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#32,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f3_2_stack=496(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 496(%rsp),%xmm9,%xmm3

# qhasm: 2x h2 += r9
# asm 1: paddq <r9=reg128#4,<h2=reg128#1
# asm 2: paddq <r9=%xmm3,<h2=%xmm0
paddq %xmm3,%xmm0

# qhasm: 2x r9 = g9 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#34,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f4_stack=528(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 528(%rsp),%xmm9,%xmm3

# qhasm: 2x h3 += r9
# asm 1: paddq <r9=reg128#4,<h3=reg128#3
# asm 2: paddq <r9=%xmm3,<h3=%xmm2
paddq %xmm3,%xmm2

# qhasm: 2x r9 = g9 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#35,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f5_2_stack=544(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 544(%rsp),%xmm9,%xmm3

# qhasm: 2x h4 += r9
# asm 1: paddq <r9=reg128#4,<h4=reg128#2
# asm 2: paddq <r9=%xmm3,<h4=%xmm1
paddq %xmm3,%xmm1

# qhasm: 2x r9 = g9 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f6_stack=592(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 592(%rsp),%xmm9,%xmm3

# qhasm: 2x h5 += r9
# asm 1: paddq <r9=reg128#4,<h5=reg128#5
# asm 2: paddq <r9=%xmm3,<h5=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r9 = g9 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#41,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f7_2_stack=640(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 640(%rsp),%xmm9,%xmm3

# qhasm: 2x h6 += r9
# asm 1: paddq <r9=reg128#4,<h6=reg128#7
# asm 2: paddq <r9=%xmm3,<h6=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r9 = g9 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<g9=reg128#10,>r9=reg128#4
# asm 2: vpmuludq <f8_stack=624(%rsp),<g9=%xmm9,>r9=%xmm3
vpmuludq 624(%rsp),%xmm9,%xmm3

# qhasm: 2x h7 += r9
# asm 1: paddq <r9=reg128#4,<h7=reg128#6
# asm 2: paddq <r9=%xmm3,<h7=%xmm5
paddq %xmm3,%xmm5

# qhasm: 2x g9 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#45,<g9=reg128#10
# asm 2: pmuludq <f9_2_stack=704(%rsp),<g9=%xmm9
pmuludq 704(%rsp),%xmm9

# qhasm: 2x h8 += g9
# asm 1: paddq <g9=reg128#10,<h8=reg128#9
# asm 2: paddq <g9=%xmm9,<h8=%xmm8
paddq %xmm9,%xmm8

# qhasm: 2x carry5 = h5 unsigned>>= 25
# asm 1: vpsrlq $25,<h5=reg128#5,>carry5=reg128#4
# asm 2: vpsrlq $25,<h5=%xmm4,>carry5=%xmm3
vpsrlq $25,%xmm4,%xmm3

# qhasm: 2x h6 += carry5
# asm 1: paddq <carry5=reg128#4,<h6=reg128#7
# asm 2: paddq <carry5=%xmm3,<h6=%xmm6
paddq %xmm3,%xmm6

# qhasm: h5 &= mem128[ m25 ]
# asm 1: pand m25,<h5=reg128#5
# asm 2: pand m25,<h5=%xmm4
pand m25(%rip),%xmm4

# qhasm: 2x carry0 = h0 unsigned>>= 26
# asm 1: vpsrlq $26,<h0=reg128#12,>carry0=reg128#4
# asm 2: vpsrlq $26,<h0=%xmm11,>carry0=%xmm3
vpsrlq $26,%xmm11,%xmm3

# qhasm: 2x h1 += carry0
# asm 1: paddq <carry0=reg128#4,<h1=reg128#14
# asm 2: paddq <carry0=%xmm3,<h1=%xmm13
paddq %xmm3,%xmm13

# qhasm: h0 &= mem128[ m26 ]
# asm 1: pand m26,<h0=reg128#12
# asm 2: pand m26,<h0=%xmm11
pand m26(%rip),%xmm11

# qhasm: 2x carry6 = h6 unsigned>>= 26
# asm 1: vpsrlq $26,<h6=reg128#7,>carry6=reg128#4
# asm 2: vpsrlq $26,<h6=%xmm6,>carry6=%xmm3
vpsrlq $26,%xmm6,%xmm3

# qhasm: 2x h7 += carry6
# asm 1: paddq <carry6=reg128#4,<h7=reg128#6
# asm 2: paddq <carry6=%xmm3,<h7=%xmm5
paddq %xmm3,%xmm5

# qhasm: h6 &= mem128[ m26 ]
# asm 1: pand m26,<h6=reg128#7
# asm 2: pand m26,<h6=%xmm6
pand m26(%rip),%xmm6

# qhasm: 2x carry1 = h1 unsigned>>= 25
# asm 1: vpsrlq $25,<h1=reg128#14,>carry1=reg128#4
# asm 2: vpsrlq $25,<h1=%xmm13,>carry1=%xmm3
vpsrlq $25,%xmm13,%xmm3

# qhasm: 2x h2 += carry1
# asm 1: paddq <carry1=reg128#4,<h2=reg128#1
# asm 2: paddq <carry1=%xmm3,<h2=%xmm0
paddq %xmm3,%xmm0

# qhasm: h1 &= mem128[ m25 ]
# asm 1: pand m25,<h1=reg128#14
# asm 2: pand m25,<h1=%xmm13
pand m25(%rip),%xmm13

# qhasm: 2x carry7 = h7 unsigned>>= 25
# asm 1: vpsrlq $25,<h7=reg128#6,>carry7=reg128#4
# asm 2: vpsrlq $25,<h7=%xmm5,>carry7=%xmm3
vpsrlq $25,%xmm5,%xmm3

# qhasm: 2x h8 += carry7
# asm 1: paddq <carry7=reg128#4,<h8=reg128#9
# asm 2: paddq <carry7=%xmm3,<h8=%xmm8
paddq %xmm3,%xmm8

# qhasm: h7 &= mem128[ m25 ]
# asm 1: pand m25,<h7=reg128#6
# asm 2: pand m25,<h7=%xmm5
pand m25(%rip),%xmm5

# qhasm: 2x carry2 = h2 unsigned>>= 26
# asm 1: vpsrlq $26,<h2=reg128#1,>carry2=reg128#4
# asm 2: vpsrlq $26,<h2=%xmm0,>carry2=%xmm3
vpsrlq $26,%xmm0,%xmm3

# qhasm: 2x h3 += carry2
# asm 1: paddq <carry2=reg128#4,<h3=reg128#3
# asm 2: paddq <carry2=%xmm3,<h3=%xmm2
paddq %xmm3,%xmm2

# qhasm: h2 &= mem128[ m26 ]
# asm 1: pand m26,<h2=reg128#1
# asm 2: pand m26,<h2=%xmm0
pand m26(%rip),%xmm0

# qhasm: 2x carry8 = h8 unsigned>>= 26
# asm 1: vpsrlq $26,<h8=reg128#9,>carry8=reg128#4
# asm 2: vpsrlq $26,<h8=%xmm8,>carry8=%xmm3
vpsrlq $26,%xmm8,%xmm3

# qhasm: 2x h9 += carry8
# asm 1: paddq <carry8=reg128#4,<h9=reg128#8
# asm 2: paddq <carry8=%xmm3,<h9=%xmm7
paddq %xmm3,%xmm7

# qhasm: h8 &= mem128[ m26 ]
# asm 1: pand m26,<h8=reg128#9
# asm 2: pand m26,<h8=%xmm8
pand m26(%rip),%xmm8

# qhasm: 2x carry3 = h3 unsigned>>= 25
# asm 1: vpsrlq $25,<h3=reg128#3,>carry3=reg128#4
# asm 2: vpsrlq $25,<h3=%xmm2,>carry3=%xmm3
vpsrlq $25,%xmm2,%xmm3

# qhasm: 2x h4 += carry3
# asm 1: paddq <carry3=reg128#4,<h4=reg128#2
# asm 2: paddq <carry3=%xmm3,<h4=%xmm1
paddq %xmm3,%xmm1

# qhasm: h3 &= mem128[ m25 ]
# asm 1: pand m25,<h3=reg128#3
# asm 2: pand m25,<h3=%xmm2
pand m25(%rip),%xmm2

# qhasm: 2x carry9 = h9 unsigned>>= 25
# asm 1: vpsrlq $25,<h9=reg128#8,>carry9=reg128#4
# asm 2: vpsrlq $25,<h9=%xmm7,>carry9=%xmm3
vpsrlq $25,%xmm7,%xmm3

# qhasm: 2x r0 = carry9 << 4
# asm 1: vpsllq $4,<carry9=reg128#4,>r0=reg128#10
# asm 2: vpsllq $4,<carry9=%xmm3,>r0=%xmm9
vpsllq $4,%xmm3,%xmm9

# qhasm: 2x h0 += carry9
# asm 1: paddq <carry9=reg128#4,<h0=reg128#12
# asm 2: paddq <carry9=%xmm3,<h0=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x carry9 <<= 1
# asm 1: psllq $1,<carry9=reg128#4
# asm 2: psllq $1,<carry9=%xmm3
psllq $1,%xmm3

# qhasm: 2x r0 += carry9
# asm 1: paddq <carry9=reg128#4,<r0=reg128#10
# asm 2: paddq <carry9=%xmm3,<r0=%xmm9
paddq %xmm3,%xmm9

# qhasm: 2x h0 += r0
# asm 1: paddq <r0=reg128#10,<h0=reg128#12
# asm 2: paddq <r0=%xmm9,<h0=%xmm11
paddq %xmm9,%xmm11

# qhasm: h9 &= mem128[ m25 ]
# asm 1: pand m25,<h9=reg128#8
# asm 2: pand m25,<h9=%xmm7
pand m25(%rip),%xmm7

# qhasm: 2x carry4 = h4 unsigned>>= 26
# asm 1: vpsrlq $26,<h4=reg128#2,>carry4=reg128#4
# asm 2: vpsrlq $26,<h4=%xmm1,>carry4=%xmm3
vpsrlq $26,%xmm1,%xmm3

# qhasm: 2x h5 += carry4
# asm 1: paddq <carry4=reg128#4,<h5=reg128#5
# asm 2: paddq <carry4=%xmm3,<h5=%xmm4
paddq %xmm3,%xmm4

# qhasm: h4 &= mem128[ m26 ]
# asm 1: pand m26,<h4=reg128#2
# asm 2: pand m26,<h4=%xmm1
pand m26(%rip),%xmm1

# qhasm: 2x carry0 = h0 unsigned>>= 26
# asm 1: vpsrlq $26,<h0=reg128#12,>carry0=reg128#4
# asm 2: vpsrlq $26,<h0=%xmm11,>carry0=%xmm3
vpsrlq $26,%xmm11,%xmm3

# qhasm: 2x h1 += carry0
# asm 1: paddq <carry0=reg128#4,<h1=reg128#14
# asm 2: paddq <carry0=%xmm3,<h1=%xmm13
paddq %xmm3,%xmm13

# qhasm: h0 &= mem128[ m26 ]
# asm 1: pand m26,<h0=reg128#12
# asm 2: pand m26,<h0=%xmm11
pand m26(%rip),%xmm11

# qhasm: 2x carry5 = h5 unsigned>>= 25
# asm 1: vpsrlq $25,<h5=reg128#5,>carry5=reg128#4
# asm 2: vpsrlq $25,<h5=%xmm4,>carry5=%xmm3
vpsrlq $25,%xmm4,%xmm3

# qhasm: 2x h6 += carry5
# asm 1: paddq <carry5=reg128#4,<h6=reg128#7
# asm 2: paddq <carry5=%xmm3,<h6=%xmm6
paddq %xmm3,%xmm6

# qhasm: h5 &= mem128[ m25 ]
# asm 1: pand m25,<h5=reg128#5
# asm 2: pand m25,<h5=%xmm4
pand m25(%rip),%xmm4

# qhasm: r = unpack_low(h0, h1)
# asm 1: vpunpcklqdq <h1=reg128#14,<h0=reg128#12,>r=reg128#4
# asm 2: vpunpcklqdq <h1=%xmm13,<h0=%xmm11,>r=%xmm3
vpunpcklqdq %xmm13,%xmm11,%xmm3

# qhasm: h1 = unpack_high(h0, h1)
# asm 1: vpunpckhqdq <h1=reg128#14,<h0=reg128#12,>h1=reg128#10
# asm 2: vpunpckhqdq <h1=%xmm13,<h0=%xmm11,>h1=%xmm9
vpunpckhqdq %xmm13,%xmm11,%xmm9

# qhasm: 2x h0 = h1 + mem128[ subc0 ]
# asm 1: vpaddq subc0,<h1=reg128#10,>h0=reg128#11
# asm 2: vpaddq subc0,<h1=%xmm9,>h0=%xmm10
vpaddq subc0(%rip),%xmm9,%xmm10

# qhasm: 2x h0 -= r
# asm 1: psubq <r=reg128#4,<h0=reg128#11
# asm 2: psubq <r=%xmm3,<h0=%xmm10
psubq %xmm3,%xmm10

# qhasm: 2x r += h1
# asm 1: paddq <h1=reg128#10,<r=reg128#4
# asm 2: paddq <h1=%xmm9,<r=%xmm3
paddq %xmm9,%xmm3

# qhasm: h1 = unpack_high(h0, r)
# asm 1: vpunpckhqdq <r=reg128#4,<h0=reg128#11,>h1=reg128#10
# asm 2: vpunpckhqdq <r=%xmm3,<h0=%xmm10,>h1=%xmm9
vpunpckhqdq %xmm3,%xmm10,%xmm9

# qhasm: unpack low qwords of h0 and r
# asm 1: punpcklqdq <r=reg128#4,<h0=reg128#11
# asm 2: punpcklqdq <r=%xmm3,<h0=%xmm10
punpcklqdq %xmm3,%xmm10

# qhasm: 2x m0 = h0 * h0
# asm 1: vpmuludq <h0=reg128#11,<h0=reg128#11,>m0=reg128#4
# asm 2: vpmuludq <h0=%xmm10,<h0=%xmm10,>m0=%xmm3
vpmuludq %xmm10,%xmm10,%xmm3

# qhasm: 2x h0 += h0
# asm 1: paddq <h0=reg128#11,<h0=reg128#11
# asm 2: paddq <h0=%xmm10,<h0=%xmm10
paddq %xmm10,%xmm10

# qhasm: 2x m1 = h0 * h1
# asm 1: vpmuludq <h1=reg128#10,<h0=reg128#11,>m1=reg128#12
# asm 2: vpmuludq <h1=%xmm9,<h0=%xmm10,>m1=%xmm11
vpmuludq %xmm9,%xmm10,%xmm11

# qhasm: r = unpack_low(h2, h3)
# asm 1: vpunpcklqdq <h3=reg128#3,<h2=reg128#1,>r=reg128#13
# asm 2: vpunpcklqdq <h3=%xmm2,<h2=%xmm0,>r=%xmm12
vpunpcklqdq %xmm2,%xmm0,%xmm12

# qhasm: h3 = unpack_high(h2, h3)
# asm 1: vpunpckhqdq <h3=reg128#3,<h2=reg128#1,>h3=reg128#1
# asm 2: vpunpckhqdq <h3=%xmm2,<h2=%xmm0,>h3=%xmm0
vpunpckhqdq %xmm2,%xmm0,%xmm0

# qhasm: 2x h2 = h3 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<h3=reg128#1,>h2=reg128#3
# asm 2: vpaddq subc2,<h3=%xmm0,>h2=%xmm2
vpaddq subc2(%rip),%xmm0,%xmm2

# qhasm: 2x h2 -= r
# asm 1: psubq <r=reg128#13,<h2=reg128#3
# asm 2: psubq <r=%xmm12,<h2=%xmm2
psubq %xmm12,%xmm2

# qhasm: 2x r += h3
# asm 1: paddq <h3=reg128#1,<r=reg128#13
# asm 2: paddq <h3=%xmm0,<r=%xmm12
paddq %xmm0,%xmm12

# qhasm: h3 = unpack_high(h2, r)
# asm 1: vpunpckhqdq <r=reg128#13,<h2=reg128#3,>h3=reg128#1
# asm 2: vpunpckhqdq <r=%xmm12,<h2=%xmm2,>h3=%xmm0
vpunpckhqdq %xmm12,%xmm2,%xmm0

# qhasm: unpack low qwords of h2 and r
# asm 1: punpcklqdq <r=reg128#13,<h2=reg128#3
# asm 2: punpcklqdq <r=%xmm12,<h2=%xmm2
punpcklqdq %xmm12,%xmm2

# qhasm: 2x m2 = h0 * h2
# asm 1: vpmuludq <h2=reg128#3,<h0=reg128#11,>m2=reg128#13
# asm 2: vpmuludq <h2=%xmm2,<h0=%xmm10,>m2=%xmm12
vpmuludq %xmm2,%xmm10,%xmm12

# qhasm: 2x h1_2 = h1 + h1
# asm 1: vpaddq <h1=reg128#10,<h1=reg128#10,>h1_2=reg128#14
# asm 2: vpaddq <h1=%xmm9,<h1=%xmm9,>h1_2=%xmm13
vpaddq %xmm9,%xmm9,%xmm13

# qhasm: 2x r = h1 * h1_2
# asm 1: vpmuludq <h1_2=reg128#14,<h1=reg128#10,>r=reg128#10
# asm 2: vpmuludq <h1_2=%xmm13,<h1=%xmm9,>r=%xmm9
vpmuludq %xmm13,%xmm9,%xmm9

# qhasm: 2x m2 += r
# asm 1: paddq <r=reg128#10,<m2=reg128#13
# asm 2: paddq <r=%xmm9,<m2=%xmm12
paddq %xmm9,%xmm12

# qhasm: 2x m3 = h0 * h3
# asm 1: vpmuludq <h3=reg128#1,<h0=reg128#11,>m3=reg128#10
# asm 2: vpmuludq <h3=%xmm0,<h0=%xmm10,>m3=%xmm9
vpmuludq %xmm0,%xmm10,%xmm9

# qhasm: 2x r = h1_2 * h2
# asm 1: vpmuludq <h2=reg128#3,<h1_2=reg128#14,>r=reg128#15
# asm 2: vpmuludq <h2=%xmm2,<h1_2=%xmm13,>r=%xmm14
vpmuludq %xmm2,%xmm13,%xmm14

# qhasm: 2x m3 += r
# asm 1: paddq <r=reg128#15,<m3=reg128#10
# asm 2: paddq <r=%xmm14,<m3=%xmm9
paddq %xmm14,%xmm9

# qhasm: r = unpack_low(h4, h5)
# asm 1: vpunpcklqdq <h5=reg128#5,<h4=reg128#2,>r=reg128#15
# asm 2: vpunpcklqdq <h5=%xmm4,<h4=%xmm1,>r=%xmm14
vpunpcklqdq %xmm4,%xmm1,%xmm14

# qhasm: h5 = unpack_high(h4, h5)
# asm 1: vpunpckhqdq <h5=reg128#5,<h4=reg128#2,>h5=reg128#2
# asm 2: vpunpckhqdq <h5=%xmm4,<h4=%xmm1,>h5=%xmm1
vpunpckhqdq %xmm4,%xmm1,%xmm1

# qhasm: 2x h4 = h5 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<h5=reg128#2,>h4=reg128#5
# asm 2: vpaddq subc2,<h5=%xmm1,>h4=%xmm4
vpaddq subc2(%rip),%xmm1,%xmm4

# qhasm: 2x h4 -= r
# asm 1: psubq <r=reg128#15,<h4=reg128#5
# asm 2: psubq <r=%xmm14,<h4=%xmm4
psubq %xmm14,%xmm4

# qhasm: 2x r += h5
# asm 1: paddq <h5=reg128#2,<r=reg128#15
# asm 2: paddq <h5=%xmm1,<r=%xmm14
paddq %xmm1,%xmm14

# qhasm: h5 = unpack_high(h4, r)
# asm 1: vpunpckhqdq <r=reg128#15,<h4=reg128#5,>h5=reg128#2
# asm 2: vpunpckhqdq <r=%xmm14,<h4=%xmm4,>h5=%xmm1
vpunpckhqdq %xmm14,%xmm4,%xmm1

# qhasm: unpack low qwords of h4 and r
# asm 1: punpcklqdq <r=reg128#15,<h4=reg128#5
# asm 2: punpcklqdq <r=%xmm14,<h4=%xmm4
punpcklqdq %xmm14,%xmm4

# qhasm: h5_stack = h5
# asm 1: movdqa <h5=reg128#2,>h5_stack=stack128#1
# asm 2: movdqa <h5=%xmm1,>h5_stack=0(%rsp)
movdqa %xmm1,0(%rsp)

# qhasm: 2x h5 += h5
# asm 1: paddq <h5=reg128#2,<h5=reg128#2
# asm 2: paddq <h5=%xmm1,<h5=%xmm1
paddq %xmm1,%xmm1

# qhasm: h5_2_stack = h5
# asm 1: movdqa <h5=reg128#2,>h5_2_stack=stack128#2
# asm 2: movdqa <h5=%xmm1,>h5_2_stack=16(%rsp)
movdqa %xmm1,16(%rsp)

# qhasm: 2x h5 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<h5=reg128#2
# asm 2: pmuludq v19_19,<h5=%xmm1
pmuludq v19_19(%rip),%xmm1

# qhasm: h5_38_stack = h5
# asm 1: movdqa <h5=reg128#2,>h5_38_stack=stack128#3
# asm 2: movdqa <h5=%xmm1,>h5_38_stack=32(%rsp)
movdqa %xmm1,32(%rsp)

# qhasm: 2x m4 = h0 * h4
# asm 1: vpmuludq <h4=reg128#5,<h0=reg128#11,>m4=reg128#2
# asm 2: vpmuludq <h4=%xmm4,<h0=%xmm10,>m4=%xmm1
vpmuludq %xmm4,%xmm10,%xmm1

# qhasm: 2x r = h2 * h2
# asm 1: vpmuludq <h2=reg128#3,<h2=reg128#3,>r=reg128#15
# asm 2: vpmuludq <h2=%xmm2,<h2=%xmm2,>r=%xmm14
vpmuludq %xmm2,%xmm2,%xmm14

# qhasm: 2x m4 += r
# asm 1: paddq <r=reg128#15,<m4=reg128#2
# asm 2: paddq <r=%xmm14,<m4=%xmm1
paddq %xmm14,%xmm1

# qhasm: 2x m5 = h0 * h5_stack
# asm 1: vpmuludq <h5_stack=stack128#1,<h0=reg128#11,>m5=reg128#15
# asm 2: vpmuludq <h5_stack=0(%rsp),<h0=%xmm10,>m5=%xmm14
vpmuludq 0(%rsp),%xmm10,%xmm14

# qhasm: 2x r = h1_2 * h4
# asm 1: vpmuludq <h4=reg128#5,<h1_2=reg128#14,>r=reg128#16
# asm 2: vpmuludq <h4=%xmm4,<h1_2=%xmm13,>r=%xmm15
vpmuludq %xmm4,%xmm13,%xmm15

# qhasm: 2x m5 += r
# asm 1: paddq <r=reg128#16,<m5=reg128#15
# asm 2: paddq <r=%xmm15,<m5=%xmm14
paddq %xmm15,%xmm14

# qhasm: r = unpack_low(h6, h7)
# asm 1: vpunpcklqdq <h7=reg128#6,<h6=reg128#7,>r=reg128#16
# asm 2: vpunpcklqdq <h7=%xmm5,<h6=%xmm6,>r=%xmm15
vpunpcklqdq %xmm5,%xmm6,%xmm15

# qhasm: h7 = unpack_high(h6, h7)
# asm 1: vpunpckhqdq <h7=reg128#6,<h6=reg128#7,>h7=reg128#6
# asm 2: vpunpckhqdq <h7=%xmm5,<h6=%xmm6,>h7=%xmm5
vpunpckhqdq %xmm5,%xmm6,%xmm5

# qhasm: 2x h6 = h7 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<h7=reg128#6,>h6=reg128#7
# asm 2: vpaddq subc2,<h7=%xmm5,>h6=%xmm6
vpaddq subc2(%rip),%xmm5,%xmm6

# qhasm: 2x h6 -= r
# asm 1: psubq <r=reg128#16,<h6=reg128#7
# asm 2: psubq <r=%xmm15,<h6=%xmm6
psubq %xmm15,%xmm6

# qhasm: 2x r += h7
# asm 1: paddq <h7=reg128#6,<r=reg128#16
# asm 2: paddq <h7=%xmm5,<r=%xmm15
paddq %xmm5,%xmm15

# qhasm: h7 = unpack_high(h6, r)
# asm 1: vpunpckhqdq <r=reg128#16,<h6=reg128#7,>h7=reg128#6
# asm 2: vpunpckhqdq <r=%xmm15,<h6=%xmm6,>h7=%xmm5
vpunpckhqdq %xmm15,%xmm6,%xmm5

# qhasm: unpack low qwords of h6 and r
# asm 1: punpcklqdq <r=reg128#16,<h6=reg128#7
# asm 2: punpcklqdq <r=%xmm15,<h6=%xmm6
punpcklqdq %xmm15,%xmm6

# qhasm: h6_stack = h6
# asm 1: movdqa <h6=reg128#7,>h6_stack=stack128#4
# asm 2: movdqa <h6=%xmm6,>h6_stack=48(%rsp)
movdqa %xmm6,48(%rsp)

# qhasm: 2x h6 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<h6=reg128#7
# asm 2: pmuludq v19_19,<h6=%xmm6
pmuludq v19_19(%rip),%xmm6

# qhasm: h6_19_stack = h6
# asm 1: movdqa <h6=reg128#7,>h6_19_stack=stack128#5
# asm 2: movdqa <h6=%xmm6,>h6_19_stack=64(%rsp)
movdqa %xmm6,64(%rsp)

# qhasm: h7_stack = h7
# asm 1: movdqa <h7=reg128#6,>h7_stack=stack128#6
# asm 2: movdqa <h7=%xmm5,>h7_stack=80(%rsp)
movdqa %xmm5,80(%rsp)

# qhasm: 2x h7 *= mem128[ v38_38 ]
# asm 1: pmuludq v38_38,<h7=reg128#6
# asm 2: pmuludq v38_38,<h7=%xmm5
pmuludq v38_38(%rip),%xmm5

# qhasm: h7_38_stack = h7
# asm 1: movdqa <h7=reg128#6,>h7_38_stack=stack128#7
# asm 2: movdqa <h7=%xmm5,>h7_38_stack=96(%rsp)
movdqa %xmm5,96(%rsp)

# qhasm: 2x m6 = h0 * h6_stack
# asm 1: vpmuludq <h6_stack=stack128#4,<h0=reg128#11,>m6=reg128#6
# asm 2: vpmuludq <h6_stack=48(%rsp),<h0=%xmm10,>m6=%xmm5
vpmuludq 48(%rsp),%xmm10,%xmm5

# qhasm: 2x h3_2 = h3 + h3
# asm 1: vpaddq <h3=reg128#1,<h3=reg128#1,>h3_2=reg128#7
# asm 2: vpaddq <h3=%xmm0,<h3=%xmm0,>h3_2=%xmm6
vpaddq %xmm0,%xmm0,%xmm6

# qhasm: 2x r = h3 * h3_2
# asm 1: vpmuludq <h3_2=reg128#7,<h3=reg128#1,>r=reg128#1
# asm 2: vpmuludq <h3_2=%xmm6,<h3=%xmm0,>r=%xmm0
vpmuludq %xmm6,%xmm0,%xmm0

# qhasm: 2x m6 += r
# asm 1: paddq <r=reg128#1,<m6=reg128#6
# asm 2: paddq <r=%xmm0,<m6=%xmm5
paddq %xmm0,%xmm5

# qhasm: 2x m7 = h0 * h7_stack
# asm 1: vpmuludq <h7_stack=stack128#6,<h0=reg128#11,>m7=reg128#1
# asm 2: vpmuludq <h7_stack=80(%rsp),<h0=%xmm10,>m7=%xmm0
vpmuludq 80(%rsp),%xmm10,%xmm0

# qhasm: 2x r = h3_2 * h4
# asm 1: vpmuludq <h4=reg128#5,<h3_2=reg128#7,>r=reg128#16
# asm 2: vpmuludq <h4=%xmm4,<h3_2=%xmm6,>r=%xmm15
vpmuludq %xmm4,%xmm6,%xmm15

# qhasm: 2x m7 += r
# asm 1: paddq <r=reg128#16,<m7=reg128#1
# asm 2: paddq <r=%xmm15,<m7=%xmm0
paddq %xmm15,%xmm0

# qhasm: 2x r = h1_2 * h3_2
# asm 1: vpmuludq <h3_2=reg128#7,<h1_2=reg128#14,>r=reg128#16
# asm 2: vpmuludq <h3_2=%xmm6,<h1_2=%xmm13,>r=%xmm15
vpmuludq %xmm6,%xmm13,%xmm15

# qhasm: 2x m4 += r
# asm 1: paddq <r=reg128#16,<m4=reg128#2
# asm 2: paddq <r=%xmm15,<m4=%xmm1
paddq %xmm15,%xmm1

# qhasm: 2x r = h2 * h3_2
# asm 1: vpmuludq <h3_2=reg128#7,<h2=reg128#3,>r=reg128#16
# asm 2: vpmuludq <h3_2=%xmm6,<h2=%xmm2,>r=%xmm15
vpmuludq %xmm6,%xmm2,%xmm15

# qhasm: 2x m5 += r
# asm 1: paddq <r=reg128#16,<m5=reg128#15
# asm 2: paddq <r=%xmm15,<m5=%xmm14
paddq %xmm15,%xmm14

# qhasm: r = unpack_low(h8, h9)
# asm 1: vpunpcklqdq <h9=reg128#8,<h8=reg128#9,>r=reg128#16
# asm 2: vpunpcklqdq <h9=%xmm7,<h8=%xmm8,>r=%xmm15
vpunpcklqdq %xmm7,%xmm8,%xmm15

# qhasm: h9 = unpack_high(h8, h9)
# asm 1: vpunpckhqdq <h9=reg128#8,<h8=reg128#9,>h9=reg128#8
# asm 2: vpunpckhqdq <h9=%xmm7,<h8=%xmm8,>h9=%xmm7
vpunpckhqdq %xmm7,%xmm8,%xmm7

# qhasm: 2x h8 = h9 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<h9=reg128#8,>h8=reg128#9
# asm 2: vpaddq subc2,<h9=%xmm7,>h8=%xmm8
vpaddq subc2(%rip),%xmm7,%xmm8

# qhasm: 2x h8 -= r
# asm 1: psubq <r=reg128#16,<h8=reg128#9
# asm 2: psubq <r=%xmm15,<h8=%xmm8
psubq %xmm15,%xmm8

# qhasm: 2x r += h9
# asm 1: paddq <h9=reg128#8,<r=reg128#16
# asm 2: paddq <h9=%xmm7,<r=%xmm15
paddq %xmm7,%xmm15

# qhasm: h9 = unpack_high(h8, r)
# asm 1: vpunpckhqdq <r=reg128#16,<h8=reg128#9,>h9=reg128#8
# asm 2: vpunpckhqdq <r=%xmm15,<h8=%xmm8,>h9=%xmm7
vpunpckhqdq %xmm15,%xmm8,%xmm7

# qhasm: unpack low qwords of h8 and r
# asm 1: punpcklqdq <r=reg128#16,<h8=reg128#9
# asm 2: punpcklqdq <r=%xmm15,<h8=%xmm8
punpcklqdq %xmm15,%xmm8

# qhasm: h8_stack = h8
# asm 1: movdqa <h8=reg128#9,>h8_stack=stack128#8
# asm 2: movdqa <h8=%xmm8,>h8_stack=112(%rsp)
movdqa %xmm8,112(%rsp)

# qhasm: 2x h8 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<h8=reg128#9
# asm 2: pmuludq v19_19,<h8=%xmm8
pmuludq v19_19(%rip),%xmm8

# qhasm: h8_19_stack = h8
# asm 1: movdqa <h8=reg128#9,>h8_19_stack=stack128#29
# asm 2: movdqa <h8=%xmm8,>h8_19_stack=448(%rsp)
movdqa %xmm8,448(%rsp)

# qhasm: 2x m8 = h0 * h8_stack
# asm 1: vpmuludq <h8_stack=stack128#8,<h0=reg128#11,>m8=reg128#9
# asm 2: vpmuludq <h8_stack=112(%rsp),<h0=%xmm10,>m8=%xmm8
vpmuludq 112(%rsp),%xmm10,%xmm8

# qhasm: 2x m9 = h0 * h9
# asm 1: vpmuludq <h9=reg128#8,<h0=reg128#11,>m9=reg128#11
# asm 2: vpmuludq <h9=%xmm7,<h0=%xmm10,>m9=%xmm10
vpmuludq %xmm7,%xmm10,%xmm10

# qhasm: 2x h9_38 = h9 * mem128[ v38_38 ]
# asm 1: vpmuludq v38_38,<h9=reg128#8,>h9_38=reg128#16
# asm 2: vpmuludq v38_38,<h9=%xmm7,>h9_38=%xmm15
vpmuludq v38_38(%rip),%xmm7,%xmm15

# qhasm: 2x r = h9 * h9_38
# asm 1: vpmuludq <h9_38=reg128#16,<h9=reg128#8,>r=reg128#8
# asm 2: vpmuludq <h9_38=%xmm15,<h9=%xmm7,>r=%xmm7
vpmuludq %xmm15,%xmm7,%xmm7

# qhasm: 2x m8 += r
# asm 1: paddq <r=reg128#8,<m8=reg128#9
# asm 2: paddq <r=%xmm7,<m8=%xmm8
paddq %xmm7,%xmm8

# qhasm: 2x r = h1_2 * h9_38
# asm 1: vpmuludq <h9_38=reg128#16,<h1_2=reg128#14,>r=reg128#8
# asm 2: vpmuludq <h9_38=%xmm15,<h1_2=%xmm13,>r=%xmm7
vpmuludq %xmm15,%xmm13,%xmm7

# qhasm: 2x m0 += r
# asm 1: paddq <r=reg128#8,<m0=reg128#4
# asm 2: paddq <r=%xmm7,<m0=%xmm3
paddq %xmm7,%xmm3

# qhasm: 2x r = h2 * h9_38
# asm 1: vpmuludq <h9_38=reg128#16,<h2=reg128#3,>r=reg128#8
# asm 2: vpmuludq <h9_38=%xmm15,<h2=%xmm2,>r=%xmm7
vpmuludq %xmm15,%xmm2,%xmm7

# qhasm: 2x m1 += r
# asm 1: paddq <r=reg128#8,<m1=reg128#12
# asm 2: paddq <r=%xmm7,<m1=%xmm11
paddq %xmm7,%xmm11

# qhasm: 2x r = h1_2 * h7_stack
# asm 1: vpmuludq <h7_stack=stack128#6,<h1_2=reg128#14,>r=reg128#8
# asm 2: vpmuludq <h7_stack=80(%rsp),<h1_2=%xmm13,>r=%xmm7
vpmuludq 80(%rsp),%xmm13,%xmm7

# qhasm: 2x r += r
# asm 1: paddq <r=reg128#8,<r=reg128#8
# asm 2: paddq <r=%xmm7,<r=%xmm7
paddq %xmm7,%xmm7

# qhasm: 2x m8 += r
# asm 1: paddq <r=reg128#8,<m8=reg128#9
# asm 2: paddq <r=%xmm7,<m8=%xmm8
paddq %xmm7,%xmm8

# qhasm: 2x r = h1_2 * h5_2_stack
# asm 1: vpmuludq <h5_2_stack=stack128#2,<h1_2=reg128#14,>r=reg128#8
# asm 2: vpmuludq <h5_2_stack=16(%rsp),<h1_2=%xmm13,>r=%xmm7
vpmuludq 16(%rsp),%xmm13,%xmm7

# qhasm: 2x m6 += r
# asm 1: paddq <r=reg128#8,<m6=reg128#6
# asm 2: paddq <r=%xmm7,<m6=%xmm5
paddq %xmm7,%xmm5

# qhasm: 2x r = h1_2 * h6_stack
# asm 1: vpmuludq <h6_stack=stack128#4,<h1_2=reg128#14,>r=reg128#8
# asm 2: vpmuludq <h6_stack=48(%rsp),<h1_2=%xmm13,>r=%xmm7
vpmuludq 48(%rsp),%xmm13,%xmm7

# qhasm: 2x m7 += r
# asm 1: paddq <r=reg128#8,<m7=reg128#1
# asm 2: paddq <r=%xmm7,<m7=%xmm0
paddq %xmm7,%xmm0

# qhasm: 2x r = h1_2 * h8_stack
# asm 1: vpmuludq <h8_stack=stack128#8,<h1_2=reg128#14,>r=reg128#8
# asm 2: vpmuludq <h8_stack=112(%rsp),<h1_2=%xmm13,>r=%xmm7
vpmuludq 112(%rsp),%xmm13,%xmm7

# qhasm: 2x m9 += r
# asm 1: paddq <r=reg128#8,<m9=reg128#11
# asm 2: paddq <r=%xmm7,<m9=%xmm10
paddq %xmm7,%xmm10

# qhasm: 2x r = h3_2 * h9_38
# asm 1: vpmuludq <h9_38=reg128#16,<h3_2=reg128#7,>r=reg128#8
# asm 2: vpmuludq <h9_38=%xmm15,<h3_2=%xmm6,>r=%xmm7
vpmuludq %xmm15,%xmm6,%xmm7

# qhasm: 2x m2 += r
# asm 1: paddq <r=reg128#8,<m2=reg128#13
# asm 2: paddq <r=%xmm7,<m2=%xmm12
paddq %xmm7,%xmm12

# qhasm: 2x r = h4 * h9_38
# asm 1: vpmuludq <h9_38=reg128#16,<h4=reg128#5,>r=reg128#8
# asm 2: vpmuludq <h9_38=%xmm15,<h4=%xmm4,>r=%xmm7
vpmuludq %xmm15,%xmm4,%xmm7

# qhasm: 2x m3 += r
# asm 1: paddq <r=reg128#8,<m3=reg128#10
# asm 2: paddq <r=%xmm7,<m3=%xmm9
paddq %xmm7,%xmm9

# qhasm: 2x h2_2 = h2 + h2
# asm 1: vpaddq <h2=reg128#3,<h2=reg128#3,>h2_2=reg128#3
# asm 2: vpaddq <h2=%xmm2,<h2=%xmm2,>h2_2=%xmm2
vpaddq %xmm2,%xmm2,%xmm2

# qhasm: 2x r = h2_2 * h4
# asm 1: vpmuludq <h4=reg128#5,<h2_2=reg128#3,>r=reg128#8
# asm 2: vpmuludq <h4=%xmm4,<h2_2=%xmm2,>r=%xmm7
vpmuludq %xmm4,%xmm2,%xmm7

# qhasm: 2x m6 += r
# asm 1: paddq <r=reg128#8,<m6=reg128#6
# asm 2: paddq <r=%xmm7,<m6=%xmm5
paddq %xmm7,%xmm5

# qhasm: 2x r = h2_2 * h8_19_stack
# asm 1: vpmuludq <h8_19_stack=stack128#29,<h2_2=reg128#3,>r=reg128#8
# asm 2: vpmuludq <h8_19_stack=448(%rsp),<h2_2=%xmm2,>r=%xmm7
vpmuludq 448(%rsp),%xmm2,%xmm7

# qhasm: 2x m0 += r
# asm 1: paddq <r=reg128#8,<m0=reg128#4
# asm 2: paddq <r=%xmm7,<m0=%xmm3
paddq %xmm7,%xmm3

# qhasm: 2x r = h3_2 * h8_19_stack
# asm 1: vpmuludq <h8_19_stack=stack128#29,<h3_2=reg128#7,>r=reg128#8
# asm 2: vpmuludq <h8_19_stack=448(%rsp),<h3_2=%xmm6,>r=%xmm7
vpmuludq 448(%rsp),%xmm6,%xmm7

# qhasm: 2x m1 += r
# asm 1: paddq <r=reg128#8,<m1=reg128#12
# asm 2: paddq <r=%xmm7,<m1=%xmm11
paddq %xmm7,%xmm11

# qhasm: 2x r = h2_2 * h5_stack
# asm 1: vpmuludq <h5_stack=stack128#1,<h2_2=reg128#3,>r=reg128#8
# asm 2: vpmuludq <h5_stack=0(%rsp),<h2_2=%xmm2,>r=%xmm7
vpmuludq 0(%rsp),%xmm2,%xmm7

# qhasm: 2x m7 += r
# asm 1: paddq <r=reg128#8,<m7=reg128#1
# asm 2: paddq <r=%xmm7,<m7=%xmm0
paddq %xmm7,%xmm0

# qhasm: 2x r = h2_2 * h6_stack
# asm 1: vpmuludq <h6_stack=stack128#4,<h2_2=reg128#3,>r=reg128#8
# asm 2: vpmuludq <h6_stack=48(%rsp),<h2_2=%xmm2,>r=%xmm7
vpmuludq 48(%rsp),%xmm2,%xmm7

# qhasm: 2x m8 += r
# asm 1: paddq <r=reg128#8,<m8=reg128#9
# asm 2: paddq <r=%xmm7,<m8=%xmm8
paddq %xmm7,%xmm8

# qhasm: 2x r = h2_2 * h7_stack
# asm 1: vpmuludq <h7_stack=stack128#6,<h2_2=reg128#3,>r=reg128#3
# asm 2: vpmuludq <h7_stack=80(%rsp),<h2_2=%xmm2,>r=%xmm2
vpmuludq 80(%rsp),%xmm2,%xmm2

# qhasm: 2x m9 += r
# asm 1: paddq <r=reg128#3,<m9=reg128#11
# asm 2: paddq <r=%xmm2,<m9=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x r = h4 * h7_38_stack
# asm 1: vpmuludq <h7_38_stack=stack128#7,<h4=reg128#5,>r=reg128#3
# asm 2: vpmuludq <h7_38_stack=96(%rsp),<h4=%xmm4,>r=%xmm2
vpmuludq 96(%rsp),%xmm4,%xmm2

# qhasm: 2x m1 += r
# asm 1: paddq <r=reg128#3,<m1=reg128#12
# asm 2: paddq <r=%xmm2,<m1=%xmm11
paddq %xmm2,%xmm11

# qhasm: 2x r = h4 * h4
# asm 1: vpmuludq <h4=reg128#5,<h4=reg128#5,>r=reg128#3
# asm 2: vpmuludq <h4=%xmm4,<h4=%xmm4,>r=%xmm2
vpmuludq %xmm4,%xmm4,%xmm2

# qhasm: 2x m8 += r
# asm 1: paddq <r=reg128#3,<m8=reg128#9
# asm 2: paddq <r=%xmm2,<m8=%xmm8
paddq %xmm2,%xmm8

# qhasm: 2x h4_2 = h4 + h4
# asm 1: vpaddq <h4=reg128#5,<h4=reg128#5,>h4_2=reg128#3
# asm 2: vpaddq <h4=%xmm4,<h4=%xmm4,>h4_2=%xmm2
vpaddq %xmm4,%xmm4,%xmm2

# qhasm: 2x r = h4_2 * h8_19_stack
# asm 1: vpmuludq <h8_19_stack=stack128#29,<h4_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <h8_19_stack=448(%rsp),<h4_2=%xmm2,>r=%xmm4
vpmuludq 448(%rsp),%xmm2,%xmm4

# qhasm: 2x m2 += r
# asm 1: paddq <r=reg128#5,<m2=reg128#13
# asm 2: paddq <r=%xmm4,<m2=%xmm12
paddq %xmm4,%xmm12

# qhasm: 2x r = h9_38 * h5_2_stack
# asm 1: vpmuludq <h5_2_stack=stack128#2,<h9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <h5_2_stack=16(%rsp),<h9_38=%xmm15,>r=%xmm4
vpmuludq 16(%rsp),%xmm15,%xmm4

# qhasm: 2x m4 += r
# asm 1: paddq <r=reg128#5,<m4=reg128#2
# asm 2: paddq <r=%xmm4,<m4=%xmm1
paddq %xmm4,%xmm1

# qhasm: 2x r = h9_38 * h6_stack
# asm 1: vpmuludq <h6_stack=stack128#4,<h9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <h6_stack=48(%rsp),<h9_38=%xmm15,>r=%xmm4
vpmuludq 48(%rsp),%xmm15,%xmm4

# qhasm: 2x m5 += r
# asm 1: paddq <r=reg128#5,<m5=reg128#15
# asm 2: paddq <r=%xmm4,<m5=%xmm14
paddq %xmm4,%xmm14

# qhasm: 2x r = h3_2 * h7_38_stack
# asm 1: vpmuludq <h7_38_stack=stack128#7,<h3_2=reg128#7,>r=reg128#5
# asm 2: vpmuludq <h7_38_stack=96(%rsp),<h3_2=%xmm6,>r=%xmm4
vpmuludq 96(%rsp),%xmm6,%xmm4

# qhasm: 2x m0 += r
# asm 1: paddq <r=reg128#5,<m0=reg128#4
# asm 2: paddq <r=%xmm4,<m0=%xmm3
paddq %xmm4,%xmm3

# qhasm: r = h5_2_stack
# asm 1: movdqa <h5_2_stack=stack128#2,>r=reg128#5
# asm 2: movdqa <h5_2_stack=16(%rsp),>r=%xmm4
movdqa 16(%rsp),%xmm4

# qhasm: 2x r *= h8_19_stack
# asm 1: pmuludq <h8_19_stack=stack128#29,<r=reg128#5
# asm 2: pmuludq <h8_19_stack=448(%rsp),<r=%xmm4
pmuludq 448(%rsp),%xmm4

# qhasm: 2x m3 += r
# asm 1: paddq <r=reg128#5,<m3=reg128#10
# asm 2: paddq <r=%xmm4,<m3=%xmm9
paddq %xmm4,%xmm9

# qhasm: 2x r = h3_2 * h5_2_stack
# asm 1: vpmuludq <h5_2_stack=stack128#2,<h3_2=reg128#7,>r=reg128#5
# asm 2: vpmuludq <h5_2_stack=16(%rsp),<h3_2=%xmm6,>r=%xmm4
vpmuludq 16(%rsp),%xmm6,%xmm4

# qhasm: 2x m8 += r
# asm 1: paddq <r=reg128#5,<m8=reg128#9
# asm 2: paddq <r=%xmm4,<m8=%xmm8
paddq %xmm4,%xmm8

# qhasm: 2x r = h3_2 * h6_stack
# asm 1: vpmuludq <h6_stack=stack128#4,<h3_2=reg128#7,>r=reg128#5
# asm 2: vpmuludq <h6_stack=48(%rsp),<h3_2=%xmm6,>r=%xmm4
vpmuludq 48(%rsp),%xmm6,%xmm4

# qhasm: 2x m9 += r
# asm 1: paddq <r=reg128#5,<m9=reg128#11
# asm 2: paddq <r=%xmm4,<m9=%xmm10
paddq %xmm4,%xmm10

# qhasm: 2x r = h9_38 * h7_stack
# asm 1: vpmuludq <h7_stack=stack128#6,<h9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <h7_stack=80(%rsp),<h9_38=%xmm15,>r=%xmm4
vpmuludq 80(%rsp),%xmm15,%xmm4

# qhasm: 2x r += r
# asm 1: paddq <r=reg128#5,<r=reg128#5
# asm 2: paddq <r=%xmm4,<r=%xmm4
paddq %xmm4,%xmm4

# qhasm: 2x m6 += r
# asm 1: paddq <r=reg128#5,<m6=reg128#6
# asm 2: paddq <r=%xmm4,<m6=%xmm5
paddq %xmm4,%xmm5

# qhasm: 2x r = h9_38 * h8_stack
# asm 1: vpmuludq <h8_stack=stack128#8,<h9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <h8_stack=112(%rsp),<h9_38=%xmm15,>r=%xmm4
vpmuludq 112(%rsp),%xmm15,%xmm4

# qhasm: 2x m7 += r
# asm 1: paddq <r=reg128#5,<m7=reg128#1
# asm 2: paddq <r=%xmm4,<m7=%xmm0
paddq %xmm4,%xmm0

# qhasm: r = h6_stack
# asm 1: movdqa <h6_stack=stack128#4,>r=reg128#5
# asm 2: movdqa <h6_stack=48(%rsp),>r=%xmm4
movdqa 48(%rsp),%xmm4

# qhasm: 2x r += r
# asm 1: paddq <r=reg128#5,<r=reg128#5
# asm 2: paddq <r=%xmm4,<r=%xmm4
paddq %xmm4,%xmm4

# qhasm: 2x r *= h8_19_stack
# asm 1: pmuludq <h8_19_stack=stack128#29,<r=reg128#5
# asm 2: pmuludq <h8_19_stack=448(%rsp),<r=%xmm4
pmuludq 448(%rsp),%xmm4

# qhasm: 2x m4 += r
# asm 1: paddq <r=reg128#5,<m4=reg128#2
# asm 2: paddq <r=%xmm4,<m4=%xmm1
paddq %xmm4,%xmm1

# qhasm: r = h7_stack
# asm 1: movdqa <h7_stack=stack128#6,>r=reg128#5
# asm 2: movdqa <h7_stack=80(%rsp),>r=%xmm4
movdqa 80(%rsp),%xmm4

# qhasm: 2x r += r
# asm 1: paddq <r=reg128#5,<r=reg128#5
# asm 2: paddq <r=%xmm4,<r=%xmm4
paddq %xmm4,%xmm4

# qhasm: 2x r *= h8_19_stack
# asm 1: pmuludq <h8_19_stack=stack128#29,<r=reg128#5
# asm 2: pmuludq <h8_19_stack=448(%rsp),<r=%xmm4
pmuludq 448(%rsp),%xmm4

# qhasm: 2x m5 += r
# asm 1: paddq <r=reg128#5,<m5=reg128#15
# asm 2: paddq <r=%xmm4,<m5=%xmm14
paddq %xmm4,%xmm14

# qhasm: 2x r = h4_2 * h6_19_stack
# asm 1: vpmuludq <h6_19_stack=stack128#5,<h4_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <h6_19_stack=64(%rsp),<h4_2=%xmm2,>r=%xmm4
vpmuludq 64(%rsp),%xmm2,%xmm4

# qhasm: 2x m0 += r
# asm 1: paddq <r=reg128#5,<m0=reg128#4
# asm 2: paddq <r=%xmm4,<m0=%xmm3
paddq %xmm4,%xmm3

# qhasm: r = h5_2_stack
# asm 1: movdqa <h5_2_stack=stack128#2,>r=reg128#5
# asm 2: movdqa <h5_2_stack=16(%rsp),>r=%xmm4
movdqa 16(%rsp),%xmm4

# qhasm: 2x r *= h6_19_stack
# asm 1: pmuludq <h6_19_stack=stack128#5,<r=reg128#5
# asm 2: pmuludq <h6_19_stack=64(%rsp),<r=%xmm4
pmuludq 64(%rsp),%xmm4

# qhasm: 2x m1 += r
# asm 1: paddq <r=reg128#5,<m1=reg128#12
# asm 2: paddq <r=%xmm4,<m1=%xmm11
paddq %xmm4,%xmm11

# qhasm: r = h5_2_stack
# asm 1: movdqa <h5_2_stack=stack128#2,>r=reg128#5
# asm 2: movdqa <h5_2_stack=16(%rsp),>r=%xmm4
movdqa 16(%rsp),%xmm4

# qhasm: 2x r *= h7_38_stack
# asm 1: pmuludq <h7_38_stack=stack128#7,<r=reg128#5
# asm 2: pmuludq <h7_38_stack=96(%rsp),<r=%xmm4
pmuludq 96(%rsp),%xmm4

# qhasm: 2x m2 += r
# asm 1: paddq <r=reg128#5,<m2=reg128#13
# asm 2: paddq <r=%xmm4,<m2=%xmm12
paddq %xmm4,%xmm12

# qhasm: r = h6_stack
# asm 1: movdqa <h6_stack=stack128#4,>r=reg128#5
# asm 2: movdqa <h6_stack=48(%rsp),>r=%xmm4
movdqa 48(%rsp),%xmm4

# qhasm: 2x r *= h7_38_stack
# asm 1: pmuludq <h7_38_stack=stack128#7,<r=reg128#5
# asm 2: pmuludq <h7_38_stack=96(%rsp),<r=%xmm4
pmuludq 96(%rsp),%xmm4

# qhasm: 2x m3 += r
# asm 1: paddq <r=reg128#5,<m3=reg128#10
# asm 2: paddq <r=%xmm4,<m3=%xmm9
paddq %xmm4,%xmm9

# qhasm: 2x r = h4_2 * h5_stack
# asm 1: vpmuludq <h5_stack=stack128#1,<h4_2=reg128#3,>r=reg128#3
# asm 2: vpmuludq <h5_stack=0(%rsp),<h4_2=%xmm2,>r=%xmm2
vpmuludq 0(%rsp),%xmm2,%xmm2

# qhasm: 2x m9 += r
# asm 1: paddq <r=reg128#3,<m9=reg128#11
# asm 2: paddq <r=%xmm2,<m9=%xmm10
paddq %xmm2,%xmm10

# qhasm: r = h5_38_stack
# asm 1: movdqa <h5_38_stack=stack128#3,>r=reg128#3
# asm 2: movdqa <h5_38_stack=32(%rsp),>r=%xmm2
movdqa 32(%rsp),%xmm2

# qhasm: 2x r *= h5_stack
# asm 1: pmuludq <h5_stack=stack128#1,<r=reg128#3
# asm 2: pmuludq <h5_stack=0(%rsp),<r=%xmm2
pmuludq 0(%rsp),%xmm2

# qhasm: 2x m0 += r
# asm 1: paddq <r=reg128#3,<m0=reg128#4
# asm 2: paddq <r=%xmm2,<m0=%xmm3
paddq %xmm2,%xmm3

# qhasm: r = h6_19_stack
# asm 1: movdqa <h6_19_stack=stack128#5,>r=reg128#3
# asm 2: movdqa <h6_19_stack=64(%rsp),>r=%xmm2
movdqa 64(%rsp),%xmm2

# qhasm: 2x r *= h6_stack
# asm 1: pmuludq <h6_stack=stack128#4,<r=reg128#3
# asm 2: pmuludq <h6_stack=48(%rsp),<r=%xmm2
pmuludq 48(%rsp),%xmm2

# qhasm: 2x m2 += r
# asm 1: paddq <r=reg128#3,<m2=reg128#13
# asm 2: paddq <r=%xmm2,<m2=%xmm12
paddq %xmm2,%xmm12

# qhasm: r = h7_38_stack
# asm 1: movdqa <h7_38_stack=stack128#7,>r=reg128#3
# asm 2: movdqa <h7_38_stack=96(%rsp),>r=%xmm2
movdqa 96(%rsp),%xmm2

# qhasm: 2x r *= h7_stack
# asm 1: pmuludq <h7_stack=stack128#6,<r=reg128#3
# asm 2: pmuludq <h7_stack=80(%rsp),<r=%xmm2
pmuludq 80(%rsp),%xmm2

# qhasm: 2x m4 += r
# asm 1: paddq <r=reg128#3,<m4=reg128#2
# asm 2: paddq <r=%xmm2,<m4=%xmm1
paddq %xmm2,%xmm1

# qhasm: r = h8_19_stack
# asm 1: movdqa <h8_19_stack=stack128#29,>r=reg128#3
# asm 2: movdqa <h8_19_stack=448(%rsp),>r=%xmm2
movdqa 448(%rsp),%xmm2

# qhasm: 2x r *= h8_stack
# asm 1: pmuludq <h8_stack=stack128#8,<r=reg128#3
# asm 2: pmuludq <h8_stack=112(%rsp),<r=%xmm2
pmuludq 112(%rsp),%xmm2

# qhasm: 2x m6 += r
# asm 1: paddq <r=reg128#3,<m6=reg128#6
# asm 2: paddq <r=%xmm2,<m6=%xmm5
paddq %xmm2,%xmm5

# qhasm: 2x carry0 = m0 unsigned>>= 26
# asm 1: vpsrlq $26,<m0=reg128#4,>carry0=reg128#3
# asm 2: vpsrlq $26,<m0=%xmm3,>carry0=%xmm2
vpsrlq $26,%xmm3,%xmm2

# qhasm: 2x m1 += carry0
# asm 1: paddq <carry0=reg128#3,<m1=reg128#12
# asm 2: paddq <carry0=%xmm2,<m1=%xmm11
paddq %xmm2,%xmm11

# qhasm: m0 &= mem128[ m26 ]
# asm 1: pand m26,<m0=reg128#4
# asm 2: pand m26,<m0=%xmm3
pand m26(%rip),%xmm3

# qhasm: 2x carry5 = m5 unsigned>>= 25
# asm 1: vpsrlq $25,<m5=reg128#15,>carry5=reg128#3
# asm 2: vpsrlq $25,<m5=%xmm14,>carry5=%xmm2
vpsrlq $25,%xmm14,%xmm2

# qhasm: 2x m6 += carry5
# asm 1: paddq <carry5=reg128#3,<m6=reg128#6
# asm 2: paddq <carry5=%xmm2,<m6=%xmm5
paddq %xmm2,%xmm5

# qhasm: m5 &= mem128[ m25 ]
# asm 1: pand m25,<m5=reg128#15
# asm 2: pand m25,<m5=%xmm14
pand m25(%rip),%xmm14

# qhasm: 2x carry1 = m1 unsigned>>= 25
# asm 1: vpsrlq $25,<m1=reg128#12,>carry1=reg128#3
# asm 2: vpsrlq $25,<m1=%xmm11,>carry1=%xmm2
vpsrlq $25,%xmm11,%xmm2

# qhasm: 2x m2 += carry1
# asm 1: paddq <carry1=reg128#3,<m2=reg128#13
# asm 2: paddq <carry1=%xmm2,<m2=%xmm12
paddq %xmm2,%xmm12

# qhasm: m1 &= mem128[ m25 ]
# asm 1: pand m25,<m1=reg128#12
# asm 2: pand m25,<m1=%xmm11
pand m25(%rip),%xmm11

# qhasm: 2x carry6 = m6 unsigned>>= 26
# asm 1: vpsrlq $26,<m6=reg128#6,>carry6=reg128#3
# asm 2: vpsrlq $26,<m6=%xmm5,>carry6=%xmm2
vpsrlq $26,%xmm5,%xmm2

# qhasm: 2x m7 += carry6
# asm 1: paddq <carry6=reg128#3,<m7=reg128#1
# asm 2: paddq <carry6=%xmm2,<m7=%xmm0
paddq %xmm2,%xmm0

# qhasm: m6 &= mem128[ m26 ]
# asm 1: pand m26,<m6=reg128#6
# asm 2: pand m26,<m6=%xmm5
pand m26(%rip),%xmm5

# qhasm: 2x carry2 = m2 unsigned>>= 26
# asm 1: vpsrlq $26,<m2=reg128#13,>carry2=reg128#3
# asm 2: vpsrlq $26,<m2=%xmm12,>carry2=%xmm2
vpsrlq $26,%xmm12,%xmm2

# qhasm: 2x m3 += carry2
# asm 1: paddq <carry2=reg128#3,<m3=reg128#10
# asm 2: paddq <carry2=%xmm2,<m3=%xmm9
paddq %xmm2,%xmm9

# qhasm: m2 &= mem128[ m26 ]
# asm 1: pand m26,<m2=reg128#13
# asm 2: pand m26,<m2=%xmm12
pand m26(%rip),%xmm12

# qhasm: 2x carry7 = m7 unsigned>>= 25
# asm 1: vpsrlq $25,<m7=reg128#1,>carry7=reg128#3
# asm 2: vpsrlq $25,<m7=%xmm0,>carry7=%xmm2
vpsrlq $25,%xmm0,%xmm2

# qhasm: 2x m8 += carry7
# asm 1: paddq <carry7=reg128#3,<m8=reg128#9
# asm 2: paddq <carry7=%xmm2,<m8=%xmm8
paddq %xmm2,%xmm8

# qhasm: m7 &= mem128[ m25 ]
# asm 1: pand m25,<m7=reg128#1
# asm 2: pand m25,<m7=%xmm0
pand m25(%rip),%xmm0

# qhasm: 2x carry3 = m3 unsigned>>= 25
# asm 1: vpsrlq $25,<m3=reg128#10,>carry3=reg128#3
# asm 2: vpsrlq $25,<m3=%xmm9,>carry3=%xmm2
vpsrlq $25,%xmm9,%xmm2

# qhasm: 2x m4 += carry3
# asm 1: paddq <carry3=reg128#3,<m4=reg128#2
# asm 2: paddq <carry3=%xmm2,<m4=%xmm1
paddq %xmm2,%xmm1

# qhasm: m3 &= mem128[ m25 ]
# asm 1: pand m25,<m3=reg128#10
# asm 2: pand m25,<m3=%xmm9
pand m25(%rip),%xmm9

# qhasm: 2x carry8 = m8 unsigned>>= 26
# asm 1: vpsrlq $26,<m8=reg128#9,>carry8=reg128#3
# asm 2: vpsrlq $26,<m8=%xmm8,>carry8=%xmm2
vpsrlq $26,%xmm8,%xmm2

# qhasm: 2x m9 += carry8
# asm 1: paddq <carry8=reg128#3,<m9=reg128#11
# asm 2: paddq <carry8=%xmm2,<m9=%xmm10
paddq %xmm2,%xmm10

# qhasm: m8 &= mem128[ m26 ]
# asm 1: pand m26,<m8=reg128#9
# asm 2: pand m26,<m8=%xmm8
pand m26(%rip),%xmm8

# qhasm: 2x carry4 = m4 unsigned>>= 26
# asm 1: vpsrlq $26,<m4=reg128#2,>carry4=reg128#3
# asm 2: vpsrlq $26,<m4=%xmm1,>carry4=%xmm2
vpsrlq $26,%xmm1,%xmm2

# qhasm: 2x m5 += carry4
# asm 1: paddq <carry4=reg128#3,<m5=reg128#15
# asm 2: paddq <carry4=%xmm2,<m5=%xmm14
paddq %xmm2,%xmm14

# qhasm: m4 &= mem128[ m26 ]
# asm 1: pand m26,<m4=reg128#2
# asm 2: pand m26,<m4=%xmm1
pand m26(%rip),%xmm1

# qhasm: 2x carry9 = m9 unsigned>>= 25
# asm 1: vpsrlq $25,<m9=reg128#11,>carry9=reg128#3
# asm 2: vpsrlq $25,<m9=%xmm10,>carry9=%xmm2
vpsrlq $25,%xmm10,%xmm2

# qhasm: 2x r0 = carry9 << 4
# asm 1: vpsllq $4,<carry9=reg128#3,>r0=reg128#5
# asm 2: vpsllq $4,<carry9=%xmm2,>r0=%xmm4
vpsllq $4,%xmm2,%xmm4

# qhasm: 2x m0 += carry9
# asm 1: paddq <carry9=reg128#3,<m0=reg128#4
# asm 2: paddq <carry9=%xmm2,<m0=%xmm3
paddq %xmm2,%xmm3

# qhasm: 2x carry9 <<= 1
# asm 1: psllq $1,<carry9=reg128#3
# asm 2: psllq $1,<carry9=%xmm2
psllq $1,%xmm2

# qhasm: 2x r0 += carry9
# asm 1: paddq <carry9=reg128#3,<r0=reg128#5
# asm 2: paddq <carry9=%xmm2,<r0=%xmm4
paddq %xmm2,%xmm4

# qhasm: 2x m0 += r0
# asm 1: paddq <r0=reg128#5,<m0=reg128#4
# asm 2: paddq <r0=%xmm4,<m0=%xmm3
paddq %xmm4,%xmm3

# qhasm: m9 &= mem128[ m25 ]
# asm 1: pand m25,<m9=reg128#11
# asm 2: pand m25,<m9=%xmm10
pand m25(%rip),%xmm10

# qhasm: 2x carry5 = m5 unsigned>>= 25
# asm 1: vpsrlq $25,<m5=reg128#15,>carry5=reg128#3
# asm 2: vpsrlq $25,<m5=%xmm14,>carry5=%xmm2
vpsrlq $25,%xmm14,%xmm2

# qhasm: 2x m6 += carry5
# asm 1: paddq <carry5=reg128#3,<m6=reg128#6
# asm 2: paddq <carry5=%xmm2,<m6=%xmm5
paddq %xmm2,%xmm5

# qhasm: m5 &= mem128[ m25 ]
# asm 1: pand m25,<m5=reg128#15
# asm 2: pand m25,<m5=%xmm14
pand m25(%rip),%xmm14

# qhasm: 2x carry0 = m0 unsigned>>= 26
# asm 1: vpsrlq $26,<m0=reg128#4,>carry0=reg128#3
# asm 2: vpsrlq $26,<m0=%xmm3,>carry0=%xmm2
vpsrlq $26,%xmm3,%xmm2

# qhasm: 2x m1 += carry0
# asm 1: paddq <carry0=reg128#3,<m1=reg128#12
# asm 2: paddq <carry0=%xmm2,<m1=%xmm11
paddq %xmm2,%xmm11

# qhasm: m0 &= mem128[ m26 ]
# asm 1: pand m26,<m0=reg128#4
# asm 2: pand m26,<m0=%xmm3
pand m26(%rip),%xmm3

# qhasm: r = unpack_high(m0, m1)
# asm 1: vpunpckhqdq <m1=reg128#12,<m0=reg128#4,>r=reg128#3
# asm 2: vpunpckhqdq <m1=%xmm11,<m0=%xmm3,>r=%xmm2
vpunpckhqdq %xmm11,%xmm3,%xmm2

# qhasm: x3_0 = r
# asm 1: movdqa <r=reg128#3,>x3_0=stack128#1
# asm 2: movdqa <r=%xmm2,>x3_0=0(%rsp)
movdqa %xmm2,0(%rsp)

# qhasm: m0 = shuffle dwords of m0 by 0
# asm 1: pshufd $0,<m0=reg128#4,>m0=reg128#3
# asm 2: pshufd $0,<m0=%xmm3,>m0=%xmm2
pshufd $0,%xmm3,%xmm2

# qhasm: m1 = shuffle dwords of m1 by 0
# asm 1: pshufd $0,<m1=reg128#12,>m1=reg128#4
# asm 2: pshufd $0,<m1=%xmm11,>m1=%xmm3
pshufd $0,%xmm11,%xmm3

# qhasm: 2x h_01 = m0 * p01_stack
# asm 1: vpmuludq <p01_stack=stack128#11,<m0=reg128#3,>h_01=reg128#5
# asm 2: vpmuludq <p01_stack=160(%rsp),<m0=%xmm2,>h_01=%xmm4
vpmuludq 160(%rsp),%xmm2,%xmm4

# qhasm: 2x r = m1 * p90_38_1_stack
# asm 1: vpmuludq <p90_38_1_stack=stack128#28,<m1=reg128#4,>r=reg128#7
# asm 2: vpmuludq <p90_38_1_stack=432(%rsp),<m1=%xmm3,>r=%xmm6
vpmuludq 432(%rsp),%xmm3,%xmm6

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#7,<h_01=reg128#5
# asm 2: paddq <r=%xmm6,<h_01=%xmm4
paddq %xmm6,%xmm4

# qhasm: 2x h_23 = m0 * p23_stack
# asm 1: vpmuludq <p23_stack=stack128#12,<m0=reg128#3,>h_23=reg128#7
# asm 2: vpmuludq <p23_stack=176(%rsp),<m0=%xmm2,>h_23=%xmm6
vpmuludq 176(%rsp),%xmm2,%xmm6

# qhasm: 2x r = m1 * p12_2_1_stack
# asm 1: vpmuludq <p12_2_1_stack=stack128#20,<m1=reg128#4,>r=reg128#8
# asm 2: vpmuludq <p12_2_1_stack=304(%rsp),<m1=%xmm3,>r=%xmm7
vpmuludq 304(%rsp),%xmm3,%xmm7

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#8,<h_23=reg128#7
# asm 2: paddq <r=%xmm7,<h_23=%xmm6
paddq %xmm7,%xmm6

# qhasm: 2x h_45 = m0 * p45_stack
# asm 1: vpmuludq <p45_stack=stack128#14,<m0=reg128#3,>h_45=reg128#8
# asm 2: vpmuludq <p45_stack=208(%rsp),<m0=%xmm2,>h_45=%xmm7
vpmuludq 208(%rsp),%xmm2,%xmm7

# qhasm: 2x r = m1 * p34_2_1_stack
# asm 1: vpmuludq <p34_2_1_stack=stack128#22,<m1=reg128#4,>r=reg128#12
# asm 2: vpmuludq <p34_2_1_stack=336(%rsp),<m1=%xmm3,>r=%xmm11
vpmuludq 336(%rsp),%xmm3,%xmm11

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#12,<h_45=reg128#8
# asm 2: paddq <r=%xmm11,<h_45=%xmm7
paddq %xmm11,%xmm7

# qhasm: 2x h_67 = m0 * p67_stack
# asm 1: vpmuludq <p67_stack=stack128#16,<m0=reg128#3,>h_67=reg128#12
# asm 2: vpmuludq <p67_stack=240(%rsp),<m0=%xmm2,>h_67=%xmm11
vpmuludq 240(%rsp),%xmm2,%xmm11

# qhasm: 2x r = m1 * p56_2_1_stack
# asm 1: vpmuludq <p56_2_1_stack=stack128#24,<m1=reg128#4,>r=reg128#14
# asm 2: vpmuludq <p56_2_1_stack=368(%rsp),<m1=%xmm3,>r=%xmm13
vpmuludq 368(%rsp),%xmm3,%xmm13

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#14,<h_67=reg128#12
# asm 2: paddq <r=%xmm13,<h_67=%xmm11
paddq %xmm13,%xmm11

# qhasm: 2x h_89 = m0 * p89_stack
# asm 1: vpmuludq <p89_stack=stack128#18,<m0=reg128#3,>h_89=reg128#3
# asm 2: vpmuludq <p89_stack=272(%rsp),<m0=%xmm2,>h_89=%xmm2
vpmuludq 272(%rsp),%xmm2,%xmm2

# qhasm: 2x r = m1 * p78_2_1_stack
# asm 1: vpmuludq <p78_2_1_stack=stack128#26,<m1=reg128#4,>r=reg128#4
# asm 2: vpmuludq <p78_2_1_stack=400(%rsp),<m1=%xmm3,>r=%xmm3
vpmuludq 400(%rsp),%xmm3,%xmm3

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#4,<h_89=reg128#3
# asm 2: paddq <r=%xmm3,<h_89=%xmm2
paddq %xmm3,%xmm2

# qhasm: r = unpack_high(m2, m3)
# asm 1: vpunpckhqdq <m3=reg128#10,<m2=reg128#13,>r=reg128#4
# asm 2: vpunpckhqdq <m3=%xmm9,<m2=%xmm12,>r=%xmm3
vpunpckhqdq %xmm9,%xmm12,%xmm3

# qhasm: x3_2 = r
# asm 1: movdqa <r=reg128#4,>x3_2=stack128#2
# asm 2: movdqa <r=%xmm3,>x3_2=16(%rsp)
movdqa %xmm3,16(%rsp)

# qhasm: m2 = shuffle dwords of m2 by 0
# asm 1: pshufd $0,<m2=reg128#13,>m2=reg128#4
# asm 2: pshufd $0,<m2=%xmm12,>m2=%xmm3
pshufd $0,%xmm12,%xmm3

# qhasm: m3 = shuffle dwords of m3 by 0
# asm 1: pshufd $0,<m3=reg128#10,>m3=reg128#10
# asm 2: pshufd $0,<m3=%xmm9,>m3=%xmm9
pshufd $0,%xmm9,%xmm9

# qhasm: 2x r = m2 * p89_19_19_stack
# asm 1: vpmuludq <p89_19_19_stack=stack128#19,<m2=reg128#4,>r=reg128#13
# asm 2: vpmuludq <p89_19_19_stack=288(%rsp),<m2=%xmm3,>r=%xmm12
vpmuludq 288(%rsp),%xmm3,%xmm12

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#13,<h_01=reg128#5
# asm 2: paddq <r=%xmm12,<h_01=%xmm4
paddq %xmm12,%xmm4

# qhasm: 2x r = m3 * p78_38_19_stack
# asm 1: vpmuludq <p78_38_19_stack=stack128#27,<m3=reg128#10,>r=reg128#13
# asm 2: vpmuludq <p78_38_19_stack=416(%rsp),<m3=%xmm9,>r=%xmm12
vpmuludq 416(%rsp),%xmm9,%xmm12

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#13,<h_01=reg128#5
# asm 2: paddq <r=%xmm12,<h_01=%xmm4
paddq %xmm12,%xmm4

# qhasm: 2x r = m2 * p01_stack
# asm 1: vpmuludq <p01_stack=stack128#11,<m2=reg128#4,>r=reg128#13
# asm 2: vpmuludq <p01_stack=160(%rsp),<m2=%xmm3,>r=%xmm12
vpmuludq 160(%rsp),%xmm3,%xmm12

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#13,<h_23=reg128#7
# asm 2: paddq <r=%xmm12,<h_23=%xmm6
paddq %xmm12,%xmm6

# qhasm: 2x r = m3 * p90_38_1_stack
# asm 1: vpmuludq <p90_38_1_stack=stack128#28,<m3=reg128#10,>r=reg128#13
# asm 2: vpmuludq <p90_38_1_stack=432(%rsp),<m3=%xmm9,>r=%xmm12
vpmuludq 432(%rsp),%xmm9,%xmm12

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#13,<h_23=reg128#7
# asm 2: paddq <r=%xmm12,<h_23=%xmm6
paddq %xmm12,%xmm6

# qhasm: 2x r = m2 * p23_stack
# asm 1: vpmuludq <p23_stack=stack128#12,<m2=reg128#4,>r=reg128#13
# asm 2: vpmuludq <p23_stack=176(%rsp),<m2=%xmm3,>r=%xmm12
vpmuludq 176(%rsp),%xmm3,%xmm12

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#13,<h_45=reg128#8
# asm 2: paddq <r=%xmm12,<h_45=%xmm7
paddq %xmm12,%xmm7

# qhasm: 2x r = m3 * p12_2_1_stack
# asm 1: vpmuludq <p12_2_1_stack=stack128#20,<m3=reg128#10,>r=reg128#13
# asm 2: vpmuludq <p12_2_1_stack=304(%rsp),<m3=%xmm9,>r=%xmm12
vpmuludq 304(%rsp),%xmm9,%xmm12

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#13,<h_45=reg128#8
# asm 2: paddq <r=%xmm12,<h_45=%xmm7
paddq %xmm12,%xmm7

# qhasm: 2x r = m2 * p45_stack
# asm 1: vpmuludq <p45_stack=stack128#14,<m2=reg128#4,>r=reg128#13
# asm 2: vpmuludq <p45_stack=208(%rsp),<m2=%xmm3,>r=%xmm12
vpmuludq 208(%rsp),%xmm3,%xmm12

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#13,<h_67=reg128#12
# asm 2: paddq <r=%xmm12,<h_67=%xmm11
paddq %xmm12,%xmm11

# qhasm: 2x r = m3 * p34_2_1_stack
# asm 1: vpmuludq <p34_2_1_stack=stack128#22,<m3=reg128#10,>r=reg128#13
# asm 2: vpmuludq <p34_2_1_stack=336(%rsp),<m3=%xmm9,>r=%xmm12
vpmuludq 336(%rsp),%xmm9,%xmm12

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#13,<h_67=reg128#12
# asm 2: paddq <r=%xmm12,<h_67=%xmm11
paddq %xmm12,%xmm11

# qhasm: 2x r = m2 * p67_stack
# asm 1: vpmuludq <p67_stack=stack128#16,<m2=reg128#4,>r=reg128#4
# asm 2: vpmuludq <p67_stack=240(%rsp),<m2=%xmm3,>r=%xmm3
vpmuludq 240(%rsp),%xmm3,%xmm3

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#4,<h_89=reg128#3
# asm 2: paddq <r=%xmm3,<h_89=%xmm2
paddq %xmm3,%xmm2

# qhasm: 2x r = m3 * p56_2_1_stack
# asm 1: vpmuludq <p56_2_1_stack=stack128#24,<m3=reg128#10,>r=reg128#4
# asm 2: vpmuludq <p56_2_1_stack=368(%rsp),<m3=%xmm9,>r=%xmm3
vpmuludq 368(%rsp),%xmm9,%xmm3

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#4,<h_89=reg128#3
# asm 2: paddq <r=%xmm3,<h_89=%xmm2
paddq %xmm3,%xmm2

# qhasm: r = unpack_high(m4, m5)
# asm 1: vpunpckhqdq <m5=reg128#15,<m4=reg128#2,>r=reg128#4
# asm 2: vpunpckhqdq <m5=%xmm14,<m4=%xmm1,>r=%xmm3
vpunpckhqdq %xmm14,%xmm1,%xmm3

# qhasm: x3_4 = r
# asm 1: movdqa <r=reg128#4,>x3_4=stack128#3
# asm 2: movdqa <r=%xmm3,>x3_4=32(%rsp)
movdqa %xmm3,32(%rsp)

# qhasm: m4 = shuffle dwords of m4 by 0
# asm 1: pshufd $0,<m4=reg128#2,>m4=reg128#2
# asm 2: pshufd $0,<m4=%xmm1,>m4=%xmm1
pshufd $0,%xmm1,%xmm1

# qhasm: m5 = shuffle dwords of m5 by 0
# asm 1: pshufd $0,<m5=reg128#15,>m5=reg128#4
# asm 2: pshufd $0,<m5=%xmm14,>m5=%xmm3
pshufd $0,%xmm14,%xmm3

# qhasm: 2x r = m4 * p67_19_19_stack
# asm 1: vpmuludq <p67_19_19_stack=stack128#17,<m4=reg128#2,>r=reg128#10
# asm 2: vpmuludq <p67_19_19_stack=256(%rsp),<m4=%xmm1,>r=%xmm9
vpmuludq 256(%rsp),%xmm1,%xmm9

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#10,<h_01=reg128#5
# asm 2: paddq <r=%xmm9,<h_01=%xmm4
paddq %xmm9,%xmm4

# qhasm: 2x r = m5 * p56_38_19_stack
# asm 1: vpmuludq <p56_38_19_stack=stack128#25,<m5=reg128#4,>r=reg128#10
# asm 2: vpmuludq <p56_38_19_stack=384(%rsp),<m5=%xmm3,>r=%xmm9
vpmuludq 384(%rsp),%xmm3,%xmm9

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#10,<h_01=reg128#5
# asm 2: paddq <r=%xmm9,<h_01=%xmm4
paddq %xmm9,%xmm4

# qhasm: 2x r = m4 * p89_19_19_stack
# asm 1: vpmuludq <p89_19_19_stack=stack128#19,<m4=reg128#2,>r=reg128#10
# asm 2: vpmuludq <p89_19_19_stack=288(%rsp),<m4=%xmm1,>r=%xmm9
vpmuludq 288(%rsp),%xmm1,%xmm9

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#10,<h_23=reg128#7
# asm 2: paddq <r=%xmm9,<h_23=%xmm6
paddq %xmm9,%xmm6

# qhasm: 2x r = m5 * p78_38_19_stack
# asm 1: vpmuludq <p78_38_19_stack=stack128#27,<m5=reg128#4,>r=reg128#10
# asm 2: vpmuludq <p78_38_19_stack=416(%rsp),<m5=%xmm3,>r=%xmm9
vpmuludq 416(%rsp),%xmm3,%xmm9

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#10,<h_23=reg128#7
# asm 2: paddq <r=%xmm9,<h_23=%xmm6
paddq %xmm9,%xmm6

# qhasm: 2x r = m4 * p01_stack
# asm 1: vpmuludq <p01_stack=stack128#11,<m4=reg128#2,>r=reg128#10
# asm 2: vpmuludq <p01_stack=160(%rsp),<m4=%xmm1,>r=%xmm9
vpmuludq 160(%rsp),%xmm1,%xmm9

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#10,<h_45=reg128#8
# asm 2: paddq <r=%xmm9,<h_45=%xmm7
paddq %xmm9,%xmm7

# qhasm: 2x r = m5 * p90_38_1_stack
# asm 1: vpmuludq <p90_38_1_stack=stack128#28,<m5=reg128#4,>r=reg128#10
# asm 2: vpmuludq <p90_38_1_stack=432(%rsp),<m5=%xmm3,>r=%xmm9
vpmuludq 432(%rsp),%xmm3,%xmm9

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#10,<h_45=reg128#8
# asm 2: paddq <r=%xmm9,<h_45=%xmm7
paddq %xmm9,%xmm7

# qhasm: 2x r = m4 * p23_stack
# asm 1: vpmuludq <p23_stack=stack128#12,<m4=reg128#2,>r=reg128#10
# asm 2: vpmuludq <p23_stack=176(%rsp),<m4=%xmm1,>r=%xmm9
vpmuludq 176(%rsp),%xmm1,%xmm9

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#10,<h_67=reg128#12
# asm 2: paddq <r=%xmm9,<h_67=%xmm11
paddq %xmm9,%xmm11

# qhasm: 2x r = m5 * p12_2_1_stack
# asm 1: vpmuludq <p12_2_1_stack=stack128#20,<m5=reg128#4,>r=reg128#10
# asm 2: vpmuludq <p12_2_1_stack=304(%rsp),<m5=%xmm3,>r=%xmm9
vpmuludq 304(%rsp),%xmm3,%xmm9

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#10,<h_67=reg128#12
# asm 2: paddq <r=%xmm9,<h_67=%xmm11
paddq %xmm9,%xmm11

# qhasm: 2x r = m4 * p45_stack
# asm 1: vpmuludq <p45_stack=stack128#14,<m4=reg128#2,>r=reg128#2
# asm 2: vpmuludq <p45_stack=208(%rsp),<m4=%xmm1,>r=%xmm1
vpmuludq 208(%rsp),%xmm1,%xmm1

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#2,<h_89=reg128#3
# asm 2: paddq <r=%xmm1,<h_89=%xmm2
paddq %xmm1,%xmm2

# qhasm: 2x r = m5 * p34_2_1_stack
# asm 1: vpmuludq <p34_2_1_stack=stack128#22,<m5=reg128#4,>r=reg128#2
# asm 2: vpmuludq <p34_2_1_stack=336(%rsp),<m5=%xmm3,>r=%xmm1
vpmuludq 336(%rsp),%xmm3,%xmm1

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#2,<h_89=reg128#3
# asm 2: paddq <r=%xmm1,<h_89=%xmm2
paddq %xmm1,%xmm2

# qhasm: r = unpack_high(m6, m7)
# asm 1: vpunpckhqdq <m7=reg128#1,<m6=reg128#6,>r=reg128#2
# asm 2: vpunpckhqdq <m7=%xmm0,<m6=%xmm5,>r=%xmm1
vpunpckhqdq %xmm0,%xmm5,%xmm1

# qhasm: x3_6 = r
# asm 1: movdqa <r=reg128#2,>x3_6=stack128#4
# asm 2: movdqa <r=%xmm1,>x3_6=48(%rsp)
movdqa %xmm1,48(%rsp)

# qhasm: m6 = shuffle dwords of m6 by 0
# asm 1: pshufd $0,<m6=reg128#6,>m6=reg128#2
# asm 2: pshufd $0,<m6=%xmm5,>m6=%xmm1
pshufd $0,%xmm5,%xmm1

# qhasm: m7 = shuffle dwords of m7 by 0
# asm 1: pshufd $0,<m7=reg128#1,>m7=reg128#1
# asm 2: pshufd $0,<m7=%xmm0,>m7=%xmm0
pshufd $0,%xmm0,%xmm0

# qhasm: 2x r = m6 * p45_19_19_stack
# asm 1: vpmuludq <p45_19_19_stack=stack128#15,<m6=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p45_19_19_stack=224(%rsp),<m6=%xmm1,>r=%xmm3
vpmuludq 224(%rsp),%xmm1,%xmm3

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#4,<h_01=reg128#5
# asm 2: paddq <r=%xmm3,<h_01=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r = m7 * p34_38_19_stack
# asm 1: vpmuludq <p34_38_19_stack=stack128#23,<m7=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p34_38_19_stack=352(%rsp),<m7=%xmm0,>r=%xmm3
vpmuludq 352(%rsp),%xmm0,%xmm3

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#4,<h_01=reg128#5
# asm 2: paddq <r=%xmm3,<h_01=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r = m6 * p67_19_19_stack
# asm 1: vpmuludq <p67_19_19_stack=stack128#17,<m6=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p67_19_19_stack=256(%rsp),<m6=%xmm1,>r=%xmm3
vpmuludq 256(%rsp),%xmm1,%xmm3

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#4,<h_23=reg128#7
# asm 2: paddq <r=%xmm3,<h_23=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r = m7 * p56_38_19_stack
# asm 1: vpmuludq <p56_38_19_stack=stack128#25,<m7=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p56_38_19_stack=384(%rsp),<m7=%xmm0,>r=%xmm3
vpmuludq 384(%rsp),%xmm0,%xmm3

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#4,<h_23=reg128#7
# asm 2: paddq <r=%xmm3,<h_23=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r = m6 * p89_19_19_stack
# asm 1: vpmuludq <p89_19_19_stack=stack128#19,<m6=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p89_19_19_stack=288(%rsp),<m6=%xmm1,>r=%xmm3
vpmuludq 288(%rsp),%xmm1,%xmm3

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#4,<h_45=reg128#8
# asm 2: paddq <r=%xmm3,<h_45=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x r = m7 * p78_38_19_stack
# asm 1: vpmuludq <p78_38_19_stack=stack128#27,<m7=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p78_38_19_stack=416(%rsp),<m7=%xmm0,>r=%xmm3
vpmuludq 416(%rsp),%xmm0,%xmm3

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#4,<h_45=reg128#8
# asm 2: paddq <r=%xmm3,<h_45=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x r = m6 * p01_stack
# asm 1: vpmuludq <p01_stack=stack128#11,<m6=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p01_stack=160(%rsp),<m6=%xmm1,>r=%xmm3
vpmuludq 160(%rsp),%xmm1,%xmm3

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#4,<h_67=reg128#12
# asm 2: paddq <r=%xmm3,<h_67=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r = m7 * p90_38_1_stack
# asm 1: vpmuludq <p90_38_1_stack=stack128#28,<m7=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p90_38_1_stack=432(%rsp),<m7=%xmm0,>r=%xmm3
vpmuludq 432(%rsp),%xmm0,%xmm3

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#4,<h_67=reg128#12
# asm 2: paddq <r=%xmm3,<h_67=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r = m6 * p23_stack
# asm 1: vpmuludq <p23_stack=stack128#12,<m6=reg128#2,>r=reg128#2
# asm 2: vpmuludq <p23_stack=176(%rsp),<m6=%xmm1,>r=%xmm1
vpmuludq 176(%rsp),%xmm1,%xmm1

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#2,<h_89=reg128#3
# asm 2: paddq <r=%xmm1,<h_89=%xmm2
paddq %xmm1,%xmm2

# qhasm: 2x r = m7 * p12_2_1_stack
# asm 1: vpmuludq <p12_2_1_stack=stack128#20,<m7=reg128#1,>r=reg128#1
# asm 2: vpmuludq <p12_2_1_stack=304(%rsp),<m7=%xmm0,>r=%xmm0
vpmuludq 304(%rsp),%xmm0,%xmm0

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#1,<h_89=reg128#3
# asm 2: paddq <r=%xmm0,<h_89=%xmm2
paddq %xmm0,%xmm2

# qhasm: r = unpack_high(m8, m9)
# asm 1: vpunpckhqdq <m9=reg128#11,<m8=reg128#9,>r=reg128#1
# asm 2: vpunpckhqdq <m9=%xmm10,<m8=%xmm8,>r=%xmm0
vpunpckhqdq %xmm10,%xmm8,%xmm0

# qhasm: x3_8 = r
# asm 1: movdqa <r=reg128#1,>x3_8=stack128#5
# asm 2: movdqa <r=%xmm0,>x3_8=64(%rsp)
movdqa %xmm0,64(%rsp)

# qhasm: m8 = shuffle dwords of m8 by 0
# asm 1: pshufd $0,<m8=reg128#9,>m8=reg128#1
# asm 2: pshufd $0,<m8=%xmm8,>m8=%xmm0
pshufd $0,%xmm8,%xmm0

# qhasm: m9 = shuffle dwords of m9 by 0
# asm 1: pshufd $0,<m9=reg128#11,>m9=reg128#2
# asm 2: pshufd $0,<m9=%xmm10,>m9=%xmm1
pshufd $0,%xmm10,%xmm1

# qhasm: 2x r = m8 * p23_19_19_stack
# asm 1: vpmuludq <p23_19_19_stack=stack128#13,<m8=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p23_19_19_stack=192(%rsp),<m8=%xmm0,>r=%xmm3
vpmuludq 192(%rsp),%xmm0,%xmm3

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#4,<h_01=reg128#5
# asm 2: paddq <r=%xmm3,<h_01=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r = m9 * p12_38_19_stack
# asm 1: vpmuludq <p12_38_19_stack=stack128#21,<m9=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p12_38_19_stack=320(%rsp),<m9=%xmm1,>r=%xmm3
vpmuludq 320(%rsp),%xmm1,%xmm3

# qhasm: 2x h_01 += r
# asm 1: paddq <r=reg128#4,<h_01=reg128#5
# asm 2: paddq <r=%xmm3,<h_01=%xmm4
paddq %xmm3,%xmm4

# qhasm: 2x r = m8 * p45_19_19_stack
# asm 1: vpmuludq <p45_19_19_stack=stack128#15,<m8=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p45_19_19_stack=224(%rsp),<m8=%xmm0,>r=%xmm3
vpmuludq 224(%rsp),%xmm0,%xmm3

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#4,<h_23=reg128#7
# asm 2: paddq <r=%xmm3,<h_23=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r = m9 * p34_38_19_stack
# asm 1: vpmuludq <p34_38_19_stack=stack128#23,<m9=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p34_38_19_stack=352(%rsp),<m9=%xmm1,>r=%xmm3
vpmuludq 352(%rsp),%xmm1,%xmm3

# qhasm: 2x h_23 += r
# asm 1: paddq <r=reg128#4,<h_23=reg128#7
# asm 2: paddq <r=%xmm3,<h_23=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r = m8 * p67_19_19_stack
# asm 1: vpmuludq <p67_19_19_stack=stack128#17,<m8=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p67_19_19_stack=256(%rsp),<m8=%xmm0,>r=%xmm3
vpmuludq 256(%rsp),%xmm0,%xmm3

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#4,<h_45=reg128#8
# asm 2: paddq <r=%xmm3,<h_45=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x r = m9 * p56_38_19_stack
# asm 1: vpmuludq <p56_38_19_stack=stack128#25,<m9=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p56_38_19_stack=384(%rsp),<m9=%xmm1,>r=%xmm3
vpmuludq 384(%rsp),%xmm1,%xmm3

# qhasm: 2x h_45 += r
# asm 1: paddq <r=reg128#4,<h_45=reg128#8
# asm 2: paddq <r=%xmm3,<h_45=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x r = m8 * p89_19_19_stack
# asm 1: vpmuludq <p89_19_19_stack=stack128#19,<m8=reg128#1,>r=reg128#4
# asm 2: vpmuludq <p89_19_19_stack=288(%rsp),<m8=%xmm0,>r=%xmm3
vpmuludq 288(%rsp),%xmm0,%xmm3

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#4,<h_67=reg128#12
# asm 2: paddq <r=%xmm3,<h_67=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r = m9 * p78_38_19_stack
# asm 1: vpmuludq <p78_38_19_stack=stack128#27,<m9=reg128#2,>r=reg128#4
# asm 2: vpmuludq <p78_38_19_stack=416(%rsp),<m9=%xmm1,>r=%xmm3
vpmuludq 416(%rsp),%xmm1,%xmm3

# qhasm: 2x h_67 += r
# asm 1: paddq <r=reg128#4,<h_67=reg128#12
# asm 2: paddq <r=%xmm3,<h_67=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r = m8 * p01_stack
# asm 1: vpmuludq <p01_stack=stack128#11,<m8=reg128#1,>r=reg128#1
# asm 2: vpmuludq <p01_stack=160(%rsp),<m8=%xmm0,>r=%xmm0
vpmuludq 160(%rsp),%xmm0,%xmm0

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#1,<h_89=reg128#3
# asm 2: paddq <r=%xmm0,<h_89=%xmm2
paddq %xmm0,%xmm2

# qhasm: 2x r = m9 * p90_38_1_stack
# asm 1: vpmuludq <p90_38_1_stack=stack128#28,<m9=reg128#2,>r=reg128#1
# asm 2: vpmuludq <p90_38_1_stack=432(%rsp),<m9=%xmm1,>r=%xmm0
vpmuludq 432(%rsp),%xmm1,%xmm0

# qhasm: 2x h_89 += r
# asm 1: paddq <r=reg128#1,<h_89=reg128#3
# asm 2: paddq <r=%xmm0,<h_89=%xmm2
paddq %xmm0,%xmm2

# qhasm: z3_0 = h_01
# asm 1: movdqa <h_01=reg128#5,>z3_0=stack128#6
# asm 2: movdqa <h_01=%xmm4,>z3_0=80(%rsp)
movdqa %xmm4,80(%rsp)

# qhasm: z3_2 = h_23
# asm 1: movdqa <h_23=reg128#7,>z3_2=stack128#7
# asm 2: movdqa <h_23=%xmm6,>z3_2=96(%rsp)
movdqa %xmm6,96(%rsp)

# qhasm: z3_4 = h_45
# asm 1: movdqa <h_45=reg128#8,>z3_4=stack128#8
# asm 2: movdqa <h_45=%xmm7,>z3_4=112(%rsp)
movdqa %xmm7,112(%rsp)

# qhasm: z3_6 = h_67
# asm 1: movdqa <h_67=reg128#12,>z3_6=stack128#29
# asm 2: movdqa <h_67=%xmm11,>z3_6=448(%rsp)
movdqa %xmm11,448(%rsp)

# qhasm: z3_8 = h_89
# asm 1: movdqa <h_89=reg128#3,>z3_8=stack128#32
# asm 2: movdqa <h_89=%xmm2,>z3_8=496(%rsp)
movdqa %xmm2,496(%rsp)

# qhasm: f0 = f0_stack
# asm 1: movdqa <f0_stack=stack128#10,>f0=reg128#1
# asm 2: movdqa <f0_stack=144(%rsp),>f0=%xmm0
movdqa 144(%rsp),%xmm0

# qhasm: 2x h0 = f0 * f0
# asm 1: vpmuludq <f0=reg128#1,<f0=reg128#1,>h0=reg128#2
# asm 2: vpmuludq <f0=%xmm0,<f0=%xmm0,>h0=%xmm1
vpmuludq %xmm0,%xmm0,%xmm1

# qhasm: 2x f0 += f0
# asm 1: paddq <f0=reg128#1,<f0=reg128#1
# asm 2: paddq <f0=%xmm0,<f0=%xmm0
paddq %xmm0,%xmm0

# qhasm: f1 = f1_stack
# asm 1: movdqa <f1_stack=stack128#9,>f1=reg128#3
# asm 2: movdqa <f1_stack=128(%rsp),>f1=%xmm2
movdqa 128(%rsp),%xmm2

# qhasm: 2x h1 = f0 * f1
# asm 1: vpmuludq <f1=reg128#3,<f0=reg128#1,>h1=reg128#4
# asm 2: vpmuludq <f1=%xmm2,<f0=%xmm0,>h1=%xmm3
vpmuludq %xmm2,%xmm0,%xmm3

# qhasm: f2 = f2_stack
# asm 1: movdqa <f2_stack=stack128#31,>f2=reg128#5
# asm 2: movdqa <f2_stack=480(%rsp),>f2=%xmm4
movdqa 480(%rsp),%xmm4

# qhasm: 2x h2 = f0 * f2
# asm 1: vpmuludq <f2=reg128#5,<f0=reg128#1,>h2=reg128#6
# asm 2: vpmuludq <f2=%xmm4,<f0=%xmm0,>h2=%xmm5
vpmuludq %xmm4,%xmm0,%xmm5

# qhasm: f3 = f3_stack
# asm 1: movdqa <f3_stack=stack128#30,>f3=reg128#7
# asm 2: movdqa <f3_stack=464(%rsp),>f3=%xmm6
movdqa 464(%rsp),%xmm6

# qhasm: 2x h3 = f0 * f3
# asm 1: vpmuludq <f3=reg128#7,<f0=reg128#1,>h3=reg128#8
# asm 2: vpmuludq <f3=%xmm6,<f0=%xmm0,>h3=%xmm7
vpmuludq %xmm6,%xmm0,%xmm7

# qhasm: f4 = f4_stack
# asm 1: movdqa <f4_stack=stack128#34,>f4=reg128#9
# asm 2: movdqa <f4_stack=528(%rsp),>f4=%xmm8
movdqa 528(%rsp),%xmm8

# qhasm: 2x h4 = f0 * f4
# asm 1: vpmuludq <f4=reg128#9,<f0=reg128#1,>h4=reg128#10
# asm 2: vpmuludq <f4=%xmm8,<f0=%xmm0,>h4=%xmm9
vpmuludq %xmm8,%xmm0,%xmm9

# qhasm: 2x h5 = f0 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#33,<f0=reg128#1,>h5=reg128#11
# asm 2: vpmuludq <f5_stack=512(%rsp),<f0=%xmm0,>h5=%xmm10
vpmuludq 512(%rsp),%xmm0,%xmm10

# qhasm: 2x h6 = f0 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<f0=reg128#1,>h6=reg128#12
# asm 2: vpmuludq <f6_stack=592(%rsp),<f0=%xmm0,>h6=%xmm11
vpmuludq 592(%rsp),%xmm0,%xmm11

# qhasm: 2x h7 = f0 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#37,<f0=reg128#1,>h7=reg128#13
# asm 2: vpmuludq <f7_stack=576(%rsp),<f0=%xmm0,>h7=%xmm12
vpmuludq 576(%rsp),%xmm0,%xmm12

# qhasm: 2x h8 = f0 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<f0=reg128#1,>h8=reg128#14
# asm 2: vpmuludq <f8_stack=624(%rsp),<f0=%xmm0,>h8=%xmm13
vpmuludq 624(%rsp),%xmm0,%xmm13

# qhasm: f9 = f9_stack
# asm 1: movdqa <f9_stack=stack128#43,>f9=reg128#15
# asm 2: movdqa <f9_stack=672(%rsp),>f9=%xmm14
movdqa 672(%rsp),%xmm14

# qhasm: 2x h9 = f0 * f9
# asm 1: vpmuludq <f9=reg128#15,<f0=reg128#1,>h9=reg128#1
# asm 2: vpmuludq <f9=%xmm14,<f0=%xmm0,>h9=%xmm0
vpmuludq %xmm14,%xmm0,%xmm0

# qhasm: 2x f9_38 = f9 * mem128[ v38_38 ]
# asm 1: vpmuludq v38_38,<f9=reg128#15,>f9_38=reg128#16
# asm 2: vpmuludq v38_38,<f9=%xmm14,>f9_38=%xmm15
vpmuludq v38_38(%rip),%xmm14,%xmm15

# qhasm: 2x r = f9 * f9_38
# asm 1: vpmuludq <f9_38=reg128#16,<f9=reg128#15,>r=reg128#15
# asm 2: vpmuludq <f9_38=%xmm15,<f9=%xmm14,>r=%xmm14
vpmuludq %xmm15,%xmm14,%xmm14

# qhasm: 2x h8 += r
# asm 1: paddq <r=reg128#15,<h8=reg128#14
# asm 2: paddq <r=%xmm14,<h8=%xmm13
paddq %xmm14,%xmm13

# qhasm: 2x f3_2 = f3 + f3
# asm 1: vpaddq <f3=reg128#7,<f3=reg128#7,>f3_2=reg128#15
# asm 2: vpaddq <f3=%xmm6,<f3=%xmm6,>f3_2=%xmm14
vpaddq %xmm6,%xmm6,%xmm14

# qhasm: 2x r = f3 * f3_2
# asm 1: vpmuludq <f3_2=reg128#15,<f3=reg128#7,>r=reg128#7
# asm 2: vpmuludq <f3_2=%xmm14,<f3=%xmm6,>r=%xmm6
vpmuludq %xmm14,%xmm6,%xmm6

# qhasm: 2x h6 += r
# asm 1: paddq <r=reg128#7,<h6=reg128#12
# asm 2: paddq <r=%xmm6,<h6=%xmm11
paddq %xmm6,%xmm11

# qhasm: 2x f1_2 = f1 + f1
# asm 1: vpaddq <f1=reg128#3,<f1=reg128#3,>f1_2=reg128#7
# asm 2: vpaddq <f1=%xmm2,<f1=%xmm2,>f1_2=%xmm6
vpaddq %xmm2,%xmm2,%xmm6

# qhasm: 2x r = f1 * f1_2
# asm 1: vpmuludq <f1_2=reg128#7,<f1=reg128#3,>r=reg128#3
# asm 2: vpmuludq <f1_2=%xmm6,<f1=%xmm2,>r=%xmm2
vpmuludq %xmm6,%xmm2,%xmm2

# qhasm: 2x h2 += r
# asm 1: paddq <r=reg128#3,<h2=reg128#6
# asm 2: paddq <r=%xmm2,<h2=%xmm5
paddq %xmm2,%xmm5

# qhasm: 2x r = f1_2 * f9_38
# asm 1: vpmuludq <f9_38=reg128#16,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f9_38=%xmm15,<f1_2=%xmm6,>r=%xmm2
vpmuludq %xmm15,%xmm6,%xmm2

# qhasm: 2x h0 += r
# asm 1: paddq <r=reg128#3,<h0=reg128#2
# asm 2: paddq <r=%xmm2,<h0=%xmm1
paddq %xmm2,%xmm1

# qhasm: 2x r = f2 * f9_38
# asm 1: vpmuludq <f9_38=reg128#16,<f2=reg128#5,>r=reg128#3
# asm 2: vpmuludq <f9_38=%xmm15,<f2=%xmm4,>r=%xmm2
vpmuludq %xmm15,%xmm4,%xmm2

# qhasm: 2x h1 += r
# asm 1: paddq <r=reg128#3,<h1=reg128#4
# asm 2: paddq <r=%xmm2,<h1=%xmm3
paddq %xmm2,%xmm3

# qhasm: 2x r = f1_2 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#35,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f5_2_stack=544(%rsp),<f1_2=%xmm6,>r=%xmm2
vpmuludq 544(%rsp),%xmm6,%xmm2

# qhasm: 2x h6 += r
# asm 1: paddq <r=reg128#3,<h6=reg128#12
# asm 2: paddq <r=%xmm2,<h6=%xmm11
paddq %xmm2,%xmm11

# qhasm: 2x r = f1_2 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f6_stack=592(%rsp),<f1_2=%xmm6,>r=%xmm2
vpmuludq 592(%rsp),%xmm6,%xmm2

# qhasm: 2x h7 += r
# asm 1: paddq <r=reg128#3,<h7=reg128#13
# asm 2: paddq <r=%xmm2,<h7=%xmm12
paddq %xmm2,%xmm12

# qhasm: 2x r = f1_2 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#41,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f7_2_stack=640(%rsp),<f1_2=%xmm6,>r=%xmm2
vpmuludq 640(%rsp),%xmm6,%xmm2

# qhasm: 2x h8 += r
# asm 1: paddq <r=reg128#3,<h8=reg128#14
# asm 2: paddq <r=%xmm2,<h8=%xmm13
paddq %xmm2,%xmm13

# qhasm: 2x r = f1_2 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f8_stack=624(%rsp),<f1_2=%xmm6,>r=%xmm2
vpmuludq 624(%rsp),%xmm6,%xmm2

# qhasm: 2x h9 += r
# asm 1: paddq <r=reg128#3,<h9=reg128#1
# asm 2: paddq <r=%xmm2,<h9=%xmm0
paddq %xmm2,%xmm0

# qhasm: 2x r = f1_2 * f2
# asm 1: vpmuludq <f2=reg128#5,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f2=%xmm4,<f1_2=%xmm6,>r=%xmm2
vpmuludq %xmm4,%xmm6,%xmm2

# qhasm: 2x h3 += r
# asm 1: paddq <r=reg128#3,<h3=reg128#8
# asm 2: paddq <r=%xmm2,<h3=%xmm7
paddq %xmm2,%xmm7

# qhasm: 2x r = f1_2 * f3_2
# asm 1: vpmuludq <f3_2=reg128#15,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f3_2=%xmm14,<f1_2=%xmm6,>r=%xmm2
vpmuludq %xmm14,%xmm6,%xmm2

# qhasm: 2x h4 += r
# asm 1: paddq <r=reg128#3,<h4=reg128#10
# asm 2: paddq <r=%xmm2,<h4=%xmm9
paddq %xmm2,%xmm9

# qhasm: 2x r = f1_2 * f4
# asm 1: vpmuludq <f4=reg128#9,<f1_2=reg128#7,>r=reg128#3
# asm 2: vpmuludq <f4=%xmm8,<f1_2=%xmm6,>r=%xmm2
vpmuludq %xmm8,%xmm6,%xmm2

# qhasm: 2x h5 += r
# asm 1: paddq <r=reg128#3,<h5=reg128#11
# asm 2: paddq <r=%xmm2,<h5=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x r = f3_2 * f9_38
# asm 1: vpmuludq <f9_38=reg128#16,<f3_2=reg128#15,>r=reg128#3
# asm 2: vpmuludq <f9_38=%xmm15,<f3_2=%xmm14,>r=%xmm2
vpmuludq %xmm15,%xmm14,%xmm2

# qhasm: 2x h2 += r
# asm 1: paddq <r=reg128#3,<h2=reg128#6
# asm 2: paddq <r=%xmm2,<h2=%xmm5
paddq %xmm2,%xmm5

# qhasm: 2x r = f4 * f9_38
# asm 1: vpmuludq <f9_38=reg128#16,<f4=reg128#9,>r=reg128#3
# asm 2: vpmuludq <f9_38=%xmm15,<f4=%xmm8,>r=%xmm2
vpmuludq %xmm15,%xmm8,%xmm2

# qhasm: 2x h3 += r
# asm 1: paddq <r=reg128#3,<h3=reg128#8
# asm 2: paddq <r=%xmm2,<h3=%xmm7
paddq %xmm2,%xmm7

# qhasm: 2x r = f2 * f2
# asm 1: vpmuludq <f2=reg128#5,<f2=reg128#5,>r=reg128#3
# asm 2: vpmuludq <f2=%xmm4,<f2=%xmm4,>r=%xmm2
vpmuludq %xmm4,%xmm4,%xmm2

# qhasm: 2x h4 += r
# asm 1: paddq <r=reg128#3,<h4=reg128#10
# asm 2: paddq <r=%xmm2,<h4=%xmm9
paddq %xmm2,%xmm9

# qhasm: 2x r = f2 * f3_2
# asm 1: vpmuludq <f3_2=reg128#15,<f2=reg128#5,>r=reg128#3
# asm 2: vpmuludq <f3_2=%xmm14,<f2=%xmm4,>r=%xmm2
vpmuludq %xmm14,%xmm4,%xmm2

# qhasm: 2x h5 += r
# asm 1: paddq <r=reg128#3,<h5=reg128#11
# asm 2: paddq <r=%xmm2,<h5=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x f2_2 = f2 + f2
# asm 1: vpaddq <f2=reg128#5,<f2=reg128#5,>f2_2=reg128#3
# asm 2: vpaddq <f2=%xmm4,<f2=%xmm4,>f2_2=%xmm2
vpaddq %xmm4,%xmm4,%xmm2

# qhasm: 2x r = f2_2 * f4
# asm 1: vpmuludq <f4=reg128#9,<f2_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <f4=%xmm8,<f2_2=%xmm2,>r=%xmm4
vpmuludq %xmm8,%xmm2,%xmm4

# qhasm: 2x h6 += r
# asm 1: paddq <r=reg128#5,<h6=reg128#12
# asm 2: paddq <r=%xmm4,<h6=%xmm11
paddq %xmm4,%xmm11

# qhasm: 2x r = f2_2 * f8_19_stack
# asm 1: vpmuludq <f8_19_stack=stack128#44,<f2_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <f8_19_stack=688(%rsp),<f2_2=%xmm2,>r=%xmm4
vpmuludq 688(%rsp),%xmm2,%xmm4

# qhasm: 2x h0 += r
# asm 1: paddq <r=reg128#5,<h0=reg128#2
# asm 2: paddq <r=%xmm4,<h0=%xmm1
paddq %xmm4,%xmm1

# qhasm: 2x r = f3_2 * f8_19_stack
# asm 1: vpmuludq <f8_19_stack=stack128#44,<f3_2=reg128#15,>r=reg128#5
# asm 2: vpmuludq <f8_19_stack=688(%rsp),<f3_2=%xmm14,>r=%xmm4
vpmuludq 688(%rsp),%xmm14,%xmm4

# qhasm: 2x h1 += r
# asm 1: paddq <r=reg128#5,<h1=reg128#4
# asm 2: paddq <r=%xmm4,<h1=%xmm3
paddq %xmm4,%xmm3

# qhasm: 2x r = f2_2 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#33,<f2_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <f5_stack=512(%rsp),<f2_2=%xmm2,>r=%xmm4
vpmuludq 512(%rsp),%xmm2,%xmm4

# qhasm: 2x h7 += r
# asm 1: paddq <r=reg128#5,<h7=reg128#13
# asm 2: paddq <r=%xmm4,<h7=%xmm12
paddq %xmm4,%xmm12

# qhasm: 2x r = f2_2 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<f2_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <f6_stack=592(%rsp),<f2_2=%xmm2,>r=%xmm4
vpmuludq 592(%rsp),%xmm2,%xmm4

# qhasm: 2x h8 += r
# asm 1: paddq <r=reg128#5,<h8=reg128#14
# asm 2: paddq <r=%xmm4,<h8=%xmm13
paddq %xmm4,%xmm13

# qhasm: 2x r = f2_2 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#37,<f2_2=reg128#3,>r=reg128#3
# asm 2: vpmuludq <f7_stack=576(%rsp),<f2_2=%xmm2,>r=%xmm2
vpmuludq 576(%rsp),%xmm2,%xmm2

# qhasm: 2x h9 += r
# asm 1: paddq <r=reg128#3,<h9=reg128#1
# asm 2: paddq <r=%xmm2,<h9=%xmm0
paddq %xmm2,%xmm0

# qhasm: 2x r = f4 * f7_38_stack
# asm 1: vpmuludq <f7_38_stack=stack128#42,<f4=reg128#9,>r=reg128#3
# asm 2: vpmuludq <f7_38_stack=656(%rsp),<f4=%xmm8,>r=%xmm2
vpmuludq 656(%rsp),%xmm8,%xmm2

# qhasm: 2x h1 += r
# asm 1: paddq <r=reg128#3,<h1=reg128#4
# asm 2: paddq <r=%xmm2,<h1=%xmm3
paddq %xmm2,%xmm3

# qhasm: 2x r = f3_2 * f4
# asm 1: vpmuludq <f4=reg128#9,<f3_2=reg128#15,>r=reg128#3
# asm 2: vpmuludq <f4=%xmm8,<f3_2=%xmm14,>r=%xmm2
vpmuludq %xmm8,%xmm14,%xmm2

# qhasm: 2x h7 += r
# asm 1: paddq <r=reg128#3,<h7=reg128#13
# asm 2: paddq <r=%xmm2,<h7=%xmm12
paddq %xmm2,%xmm12

# qhasm: 2x r = f4 * f4
# asm 1: vpmuludq <f4=reg128#9,<f4=reg128#9,>r=reg128#3
# asm 2: vpmuludq <f4=%xmm8,<f4=%xmm8,>r=%xmm2
vpmuludq %xmm8,%xmm8,%xmm2

# qhasm: 2x h8 += r
# asm 1: paddq <r=reg128#3,<h8=reg128#14
# asm 2: paddq <r=%xmm2,<h8=%xmm13
paddq %xmm2,%xmm13

# qhasm: 2x f4_2 = f4 + f4
# asm 1: vpaddq <f4=reg128#9,<f4=reg128#9,>f4_2=reg128#3
# asm 2: vpaddq <f4=%xmm8,<f4=%xmm8,>f4_2=%xmm2
vpaddq %xmm8,%xmm8,%xmm2

# qhasm: 2x r = f4_2 * f8_19_stack
# asm 1: vpmuludq <f8_19_stack=stack128#44,<f4_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <f8_19_stack=688(%rsp),<f4_2=%xmm2,>r=%xmm4
vpmuludq 688(%rsp),%xmm2,%xmm4

# qhasm: 2x h2 += r
# asm 1: paddq <r=reg128#5,<h2=reg128#6
# asm 2: paddq <r=%xmm4,<h2=%xmm5
paddq %xmm4,%xmm5

# qhasm: 2x r = f9_38 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#35,<f9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <f5_2_stack=544(%rsp),<f9_38=%xmm15,>r=%xmm4
vpmuludq 544(%rsp),%xmm15,%xmm4

# qhasm: 2x h4 += r
# asm 1: paddq <r=reg128#5,<h4=reg128#10
# asm 2: paddq <r=%xmm4,<h4=%xmm9
paddq %xmm4,%xmm9

# qhasm: 2x r = f9_38 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<f9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <f6_stack=592(%rsp),<f9_38=%xmm15,>r=%xmm4
vpmuludq 592(%rsp),%xmm15,%xmm4

# qhasm: 2x h5 += r
# asm 1: paddq <r=reg128#5,<h5=reg128#11
# asm 2: paddq <r=%xmm4,<h5=%xmm10
paddq %xmm4,%xmm10

# qhasm: 2x r = f3_2 * f7_38_stack
# asm 1: vpmuludq <f7_38_stack=stack128#42,<f3_2=reg128#15,>r=reg128#5
# asm 2: vpmuludq <f7_38_stack=656(%rsp),<f3_2=%xmm14,>r=%xmm4
vpmuludq 656(%rsp),%xmm14,%xmm4

# qhasm: 2x h0 += r
# asm 1: paddq <r=reg128#5,<h0=reg128#2
# asm 2: paddq <r=%xmm4,<h0=%xmm1
paddq %xmm4,%xmm1

# qhasm: r = f5_2_stack
# asm 1: movdqa <f5_2_stack=stack128#35,>r=reg128#5
# asm 2: movdqa <f5_2_stack=544(%rsp),>r=%xmm4
movdqa 544(%rsp),%xmm4

# qhasm: 2x r *= f8_19_stack
# asm 1: pmuludq <f8_19_stack=stack128#44,<r=reg128#5
# asm 2: pmuludq <f8_19_stack=688(%rsp),<r=%xmm4
pmuludq 688(%rsp),%xmm4

# qhasm: 2x h3 += r
# asm 1: paddq <r=reg128#5,<h3=reg128#8
# asm 2: paddq <r=%xmm4,<h3=%xmm7
paddq %xmm4,%xmm7

# qhasm: 2x r = f3_2 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#35,<f3_2=reg128#15,>r=reg128#5
# asm 2: vpmuludq <f5_2_stack=544(%rsp),<f3_2=%xmm14,>r=%xmm4
vpmuludq 544(%rsp),%xmm14,%xmm4

# qhasm: 2x h8 += r
# asm 1: paddq <r=reg128#5,<h8=reg128#14
# asm 2: paddq <r=%xmm4,<h8=%xmm13
paddq %xmm4,%xmm13

# qhasm: 2x r = f3_2 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#38,<f3_2=reg128#15,>r=reg128#5
# asm 2: vpmuludq <f6_stack=592(%rsp),<f3_2=%xmm14,>r=%xmm4
vpmuludq 592(%rsp),%xmm14,%xmm4

# qhasm: 2x h9 += r
# asm 1: paddq <r=reg128#5,<h9=reg128#1
# asm 2: paddq <r=%xmm4,<h9=%xmm0
paddq %xmm4,%xmm0

# qhasm: 2x r = f9_38 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#41,<f9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <f7_2_stack=640(%rsp),<f9_38=%xmm15,>r=%xmm4
vpmuludq 640(%rsp),%xmm15,%xmm4

# qhasm: 2x h6 += r
# asm 1: paddq <r=reg128#5,<h6=reg128#12
# asm 2: paddq <r=%xmm4,<h6=%xmm11
paddq %xmm4,%xmm11

# qhasm: 2x r = f9_38 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#40,<f9_38=reg128#16,>r=reg128#5
# asm 2: vpmuludq <f8_stack=624(%rsp),<f9_38=%xmm15,>r=%xmm4
vpmuludq 624(%rsp),%xmm15,%xmm4

# qhasm: 2x h7 += r
# asm 1: paddq <r=reg128#5,<h7=reg128#13
# asm 2: paddq <r=%xmm4,<h7=%xmm12
paddq %xmm4,%xmm12

# qhasm: r = f6_stack
# asm 1: movdqa <f6_stack=stack128#38,>r=reg128#5
# asm 2: movdqa <f6_stack=592(%rsp),>r=%xmm4
movdqa 592(%rsp),%xmm4

# qhasm: 2x r += r
# asm 1: paddq <r=reg128#5,<r=reg128#5
# asm 2: paddq <r=%xmm4,<r=%xmm4
paddq %xmm4,%xmm4

# qhasm: 2x r *= f8_19_stack
# asm 1: pmuludq <f8_19_stack=stack128#44,<r=reg128#5
# asm 2: pmuludq <f8_19_stack=688(%rsp),<r=%xmm4
pmuludq 688(%rsp),%xmm4

# qhasm: 2x h4 += r
# asm 1: paddq <r=reg128#5,<h4=reg128#10
# asm 2: paddq <r=%xmm4,<h4=%xmm9
paddq %xmm4,%xmm9

# qhasm: 2x r = f4_2 * f6_19_stack
# asm 1: vpmuludq <f6_19_stack=stack128#39,<f4_2=reg128#3,>r=reg128#5
# asm 2: vpmuludq <f6_19_stack=608(%rsp),<f4_2=%xmm2,>r=%xmm4
vpmuludq 608(%rsp),%xmm2,%xmm4

# qhasm: 2x h0 += r
# asm 1: paddq <r=reg128#5,<h0=reg128#2
# asm 2: paddq <r=%xmm4,<h0=%xmm1
paddq %xmm4,%xmm1

# qhasm: r = f5_2_stack
# asm 1: movdqa <f5_2_stack=stack128#35,>r=reg128#5
# asm 2: movdqa <f5_2_stack=544(%rsp),>r=%xmm4
movdqa 544(%rsp),%xmm4

# qhasm: 2x r *= f6_19_stack
# asm 1: pmuludq <f6_19_stack=stack128#39,<r=reg128#5
# asm 2: pmuludq <f6_19_stack=608(%rsp),<r=%xmm4
pmuludq 608(%rsp),%xmm4

# qhasm: 2x h1 += r
# asm 1: paddq <r=reg128#5,<h1=reg128#4
# asm 2: paddq <r=%xmm4,<h1=%xmm3
paddq %xmm4,%xmm3

# qhasm: r = f5_2_stack
# asm 1: movdqa <f5_2_stack=stack128#35,>r=reg128#5
# asm 2: movdqa <f5_2_stack=544(%rsp),>r=%xmm4
movdqa 544(%rsp),%xmm4

# qhasm: 2x r *= f7_38_stack
# asm 1: pmuludq <f7_38_stack=stack128#42,<r=reg128#5
# asm 2: pmuludq <f7_38_stack=656(%rsp),<r=%xmm4
pmuludq 656(%rsp),%xmm4

# qhasm: 2x h2 += r
# asm 1: paddq <r=reg128#5,<h2=reg128#6
# asm 2: paddq <r=%xmm4,<h2=%xmm5
paddq %xmm4,%xmm5

# qhasm: r = f6_stack
# asm 1: movdqa <f6_stack=stack128#38,>r=reg128#5
# asm 2: movdqa <f6_stack=592(%rsp),>r=%xmm4
movdqa 592(%rsp),%xmm4

# qhasm: 2x r *= f7_38_stack
# asm 1: pmuludq <f7_38_stack=stack128#42,<r=reg128#5
# asm 2: pmuludq <f7_38_stack=656(%rsp),<r=%xmm4
pmuludq 656(%rsp),%xmm4

# qhasm: 2x h3 += r
# asm 1: paddq <r=reg128#5,<h3=reg128#8
# asm 2: paddq <r=%xmm4,<h3=%xmm7
paddq %xmm4,%xmm7

# qhasm: r = f7_2_stack
# asm 1: movdqa <f7_2_stack=stack128#41,>r=reg128#5
# asm 2: movdqa <f7_2_stack=640(%rsp),>r=%xmm4
movdqa 640(%rsp),%xmm4

# qhasm: 2x r *= f8_19_stack
# asm 1: pmuludq <f8_19_stack=stack128#44,<r=reg128#5
# asm 2: pmuludq <f8_19_stack=688(%rsp),<r=%xmm4
pmuludq 688(%rsp),%xmm4

# qhasm: 2x h5 += r
# asm 1: paddq <r=reg128#5,<h5=reg128#11
# asm 2: paddq <r=%xmm4,<h5=%xmm10
paddq %xmm4,%xmm10

# qhasm: 2x r = f4_2 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#33,<f4_2=reg128#3,>r=reg128#3
# asm 2: vpmuludq <f5_stack=512(%rsp),<f4_2=%xmm2,>r=%xmm2
vpmuludq 512(%rsp),%xmm2,%xmm2

# qhasm: 2x h9 += r
# asm 1: paddq <r=reg128#3,<h9=reg128#1
# asm 2: paddq <r=%xmm2,<h9=%xmm0
paddq %xmm2,%xmm0

# qhasm: r = f5_38_stack
# asm 1: movdqa <f5_38_stack=stack128#36,>r=reg128#3
# asm 2: movdqa <f5_38_stack=560(%rsp),>r=%xmm2
movdqa 560(%rsp),%xmm2

# qhasm: 2x r *= f5_stack
# asm 1: pmuludq <f5_stack=stack128#33,<r=reg128#3
# asm 2: pmuludq <f5_stack=512(%rsp),<r=%xmm2
pmuludq 512(%rsp),%xmm2

# qhasm: 2x h0 += r
# asm 1: paddq <r=reg128#3,<h0=reg128#2
# asm 2: paddq <r=%xmm2,<h0=%xmm1
paddq %xmm2,%xmm1

# qhasm: r = f6_19_stack
# asm 1: movdqa <f6_19_stack=stack128#39,>r=reg128#3
# asm 2: movdqa <f6_19_stack=608(%rsp),>r=%xmm2
movdqa 608(%rsp),%xmm2

# qhasm: 2x r *= f6_stack
# asm 1: pmuludq <f6_stack=stack128#38,<r=reg128#3
# asm 2: pmuludq <f6_stack=592(%rsp),<r=%xmm2
pmuludq 592(%rsp),%xmm2

# qhasm: 2x h2 += r
# asm 1: paddq <r=reg128#3,<h2=reg128#6
# asm 2: paddq <r=%xmm2,<h2=%xmm5
paddq %xmm2,%xmm5

# qhasm: r = f7_38_stack
# asm 1: movdqa <f7_38_stack=stack128#42,>r=reg128#3
# asm 2: movdqa <f7_38_stack=656(%rsp),>r=%xmm2
movdqa 656(%rsp),%xmm2

# qhasm: 2x r *= f7_stack
# asm 1: pmuludq <f7_stack=stack128#37,<r=reg128#3
# asm 2: pmuludq <f7_stack=576(%rsp),<r=%xmm2
pmuludq 576(%rsp),%xmm2

# qhasm: 2x h4 += r
# asm 1: paddq <r=reg128#3,<h4=reg128#10
# asm 2: paddq <r=%xmm2,<h4=%xmm9
paddq %xmm2,%xmm9

# qhasm: r = f8_19_stack
# asm 1: movdqa <f8_19_stack=stack128#44,>r=reg128#3
# asm 2: movdqa <f8_19_stack=688(%rsp),>r=%xmm2
movdqa 688(%rsp),%xmm2

# qhasm: 2x r *= f8_stack
# asm 1: pmuludq <f8_stack=stack128#40,<r=reg128#3
# asm 2: pmuludq <f8_stack=624(%rsp),<r=%xmm2
pmuludq 624(%rsp),%xmm2

# qhasm: 2x h6 += r
# asm 1: paddq <r=reg128#3,<h6=reg128#12
# asm 2: paddq <r=%xmm2,<h6=%xmm11
paddq %xmm2,%xmm11

# qhasm: 2x carry0 = h0 unsigned>>= 26
# asm 1: vpsrlq $26,<h0=reg128#2,>carry0=reg128#3
# asm 2: vpsrlq $26,<h0=%xmm1,>carry0=%xmm2
vpsrlq $26,%xmm1,%xmm2

# qhasm: 2x h1 += carry0
# asm 1: paddq <carry0=reg128#3,<h1=reg128#4
# asm 2: paddq <carry0=%xmm2,<h1=%xmm3
paddq %xmm2,%xmm3

# qhasm: h0 &= mem128[ m26 ]
# asm 1: pand m26,<h0=reg128#2
# asm 2: pand m26,<h0=%xmm1
pand m26(%rip),%xmm1

# qhasm: 2x carry5 = h5 unsigned>>= 25
# asm 1: vpsrlq $25,<h5=reg128#11,>carry5=reg128#3
# asm 2: vpsrlq $25,<h5=%xmm10,>carry5=%xmm2
vpsrlq $25,%xmm10,%xmm2

# qhasm: 2x h6 += carry5
# asm 1: paddq <carry5=reg128#3,<h6=reg128#12
# asm 2: paddq <carry5=%xmm2,<h6=%xmm11
paddq %xmm2,%xmm11

# qhasm: h5 &= mem128[ m25 ]
# asm 1: pand m25,<h5=reg128#11
# asm 2: pand m25,<h5=%xmm10
pand m25(%rip),%xmm10

# qhasm: 2x carry1 = h1 unsigned>>= 25
# asm 1: vpsrlq $25,<h1=reg128#4,>carry1=reg128#3
# asm 2: vpsrlq $25,<h1=%xmm3,>carry1=%xmm2
vpsrlq $25,%xmm3,%xmm2

# qhasm: 2x h2 += carry1
# asm 1: paddq <carry1=reg128#3,<h2=reg128#6
# asm 2: paddq <carry1=%xmm2,<h2=%xmm5
paddq %xmm2,%xmm5

# qhasm: h1 &= mem128[ m25 ]
# asm 1: pand m25,<h1=reg128#4
# asm 2: pand m25,<h1=%xmm3
pand m25(%rip),%xmm3

# qhasm: 2x carry6 = h6 unsigned>>= 26
# asm 1: vpsrlq $26,<h6=reg128#12,>carry6=reg128#3
# asm 2: vpsrlq $26,<h6=%xmm11,>carry6=%xmm2
vpsrlq $26,%xmm11,%xmm2

# qhasm: 2x h7 += carry6
# asm 1: paddq <carry6=reg128#3,<h7=reg128#13
# asm 2: paddq <carry6=%xmm2,<h7=%xmm12
paddq %xmm2,%xmm12

# qhasm: h6 &= mem128[ m26 ]
# asm 1: pand m26,<h6=reg128#12
# asm 2: pand m26,<h6=%xmm11
pand m26(%rip),%xmm11

# qhasm: 2x carry2 = h2 unsigned>>= 26
# asm 1: vpsrlq $26,<h2=reg128#6,>carry2=reg128#3
# asm 2: vpsrlq $26,<h2=%xmm5,>carry2=%xmm2
vpsrlq $26,%xmm5,%xmm2

# qhasm: 2x h3 += carry2
# asm 1: paddq <carry2=reg128#3,<h3=reg128#8
# asm 2: paddq <carry2=%xmm2,<h3=%xmm7
paddq %xmm2,%xmm7

# qhasm: h2 &= mem128[ m26 ]
# asm 1: pand m26,<h2=reg128#6
# asm 2: pand m26,<h2=%xmm5
pand m26(%rip),%xmm5

# qhasm: 2x carry7 = h7 unsigned>>= 25
# asm 1: vpsrlq $25,<h7=reg128#13,>carry7=reg128#3
# asm 2: vpsrlq $25,<h7=%xmm12,>carry7=%xmm2
vpsrlq $25,%xmm12,%xmm2

# qhasm: 2x h8 += carry7
# asm 1: paddq <carry7=reg128#3,<h8=reg128#14
# asm 2: paddq <carry7=%xmm2,<h8=%xmm13
paddq %xmm2,%xmm13

# qhasm: h7 &= mem128[ m25 ]
# asm 1: pand m25,<h7=reg128#13
# asm 2: pand m25,<h7=%xmm12
pand m25(%rip),%xmm12

# qhasm: 2x carry3 = h3 unsigned>>= 25
# asm 1: vpsrlq $25,<h3=reg128#8,>carry3=reg128#3
# asm 2: vpsrlq $25,<h3=%xmm7,>carry3=%xmm2
vpsrlq $25,%xmm7,%xmm2

# qhasm: 2x h4 += carry3
# asm 1: paddq <carry3=reg128#3,<h4=reg128#10
# asm 2: paddq <carry3=%xmm2,<h4=%xmm9
paddq %xmm2,%xmm9

# qhasm: h3 &= mem128[ m25 ]
# asm 1: pand m25,<h3=reg128#8
# asm 2: pand m25,<h3=%xmm7
pand m25(%rip),%xmm7

# qhasm: 2x carry8 = h8 unsigned>>= 26
# asm 1: vpsrlq $26,<h8=reg128#14,>carry8=reg128#3
# asm 2: vpsrlq $26,<h8=%xmm13,>carry8=%xmm2
vpsrlq $26,%xmm13,%xmm2

# qhasm: 2x h9 += carry8
# asm 1: paddq <carry8=reg128#3,<h9=reg128#1
# asm 2: paddq <carry8=%xmm2,<h9=%xmm0
paddq %xmm2,%xmm0

# qhasm: h8 &= mem128[ m26 ]
# asm 1: pand m26,<h8=reg128#14
# asm 2: pand m26,<h8=%xmm13
pand m26(%rip),%xmm13

# qhasm: 2x carry4 = h4 unsigned>>= 26
# asm 1: vpsrlq $26,<h4=reg128#10,>carry4=reg128#3
# asm 2: vpsrlq $26,<h4=%xmm9,>carry4=%xmm2
vpsrlq $26,%xmm9,%xmm2

# qhasm: 2x h5 += carry4
# asm 1: paddq <carry4=reg128#3,<h5=reg128#11
# asm 2: paddq <carry4=%xmm2,<h5=%xmm10
paddq %xmm2,%xmm10

# qhasm: h4 &= mem128[ m26 ]
# asm 1: pand m26,<h4=reg128#10
# asm 2: pand m26,<h4=%xmm9
pand m26(%rip),%xmm9

# qhasm: 2x carry9 = h9 unsigned>>= 25
# asm 1: vpsrlq $25,<h9=reg128#1,>carry9=reg128#3
# asm 2: vpsrlq $25,<h9=%xmm0,>carry9=%xmm2
vpsrlq $25,%xmm0,%xmm2

# qhasm: 2x r0 = carry9 << 4
# asm 1: vpsllq $4,<carry9=reg128#3,>r0=reg128#5
# asm 2: vpsllq $4,<carry9=%xmm2,>r0=%xmm4
vpsllq $4,%xmm2,%xmm4

# qhasm: 2x h0 += carry9
# asm 1: paddq <carry9=reg128#3,<h0=reg128#2
# asm 2: paddq <carry9=%xmm2,<h0=%xmm1
paddq %xmm2,%xmm1

# qhasm: 2x carry9 <<= 1
# asm 1: psllq $1,<carry9=reg128#3
# asm 2: psllq $1,<carry9=%xmm2
psllq $1,%xmm2

# qhasm: 2x r0 += carry9
# asm 1: paddq <carry9=reg128#3,<r0=reg128#5
# asm 2: paddq <carry9=%xmm2,<r0=%xmm4
paddq %xmm2,%xmm4

# qhasm: 2x h0 += r0
# asm 1: paddq <r0=reg128#5,<h0=reg128#2
# asm 2: paddq <r0=%xmm4,<h0=%xmm1
paddq %xmm4,%xmm1

# qhasm: h9 &= mem128[ m25 ]
# asm 1: pand m25,<h9=reg128#1
# asm 2: pand m25,<h9=%xmm0
pand m25(%rip),%xmm0

# qhasm: 2x carry5 = h5 unsigned>>= 25
# asm 1: vpsrlq $25,<h5=reg128#11,>carry5=reg128#3
# asm 2: vpsrlq $25,<h5=%xmm10,>carry5=%xmm2
vpsrlq $25,%xmm10,%xmm2

# qhasm: 2x h6 += carry5
# asm 1: paddq <carry5=reg128#3,<h6=reg128#12
# asm 2: paddq <carry5=%xmm2,<h6=%xmm11
paddq %xmm2,%xmm11

# qhasm: h5 &= mem128[ m25 ]
# asm 1: pand m25,<h5=reg128#11
# asm 2: pand m25,<h5=%xmm10
pand m25(%rip),%xmm10

# qhasm: 2x carry0 = h0 unsigned>>= 26
# asm 1: vpsrlq $26,<h0=reg128#2,>carry0=reg128#3
# asm 2: vpsrlq $26,<h0=%xmm1,>carry0=%xmm2
vpsrlq $26,%xmm1,%xmm2

# qhasm: 2x h1 += carry0
# asm 1: paddq <carry0=reg128#3,<h1=reg128#4
# asm 2: paddq <carry0=%xmm2,<h1=%xmm3
paddq %xmm2,%xmm3

# qhasm: h0 &= mem128[ m26 ]
# asm 1: pand m26,<h0=reg128#2
# asm 2: pand m26,<h0=%xmm1
pand m26(%rip),%xmm1

# qhasm: f0 = unpack_high(h0, h1)
# asm 1: vpunpckhqdq <h1=reg128#4,<h0=reg128#2,>f0=reg128#3
# asm 2: vpunpckhqdq <h1=%xmm3,<h0=%xmm1,>f0=%xmm2
vpunpckhqdq %xmm3,%xmm1,%xmm2

# qhasm: h0 = unpack_low(h0, h1)
# asm 1: vpunpcklqdq <h1=reg128#4,<h0=reg128#2,>h0=reg128#2
# asm 2: vpunpcklqdq <h1=%xmm3,<h0=%xmm1,>h0=%xmm1
vpunpcklqdq %xmm3,%xmm1,%xmm1

# qhasm: t0_0 = h0
# asm 1: movdqa <h0=reg128#2,>t0_0=stack128#30
# asm 2: movdqa <h0=%xmm1,>t0_0=464(%rsp)
movdqa %xmm1,464(%rsp)

# qhasm: 2x h1 = f0 + mem128[ subc0 ]
# asm 1: vpaddq subc0,<f0=reg128#3,>h1=reg128#4
# asm 2: vpaddq subc0,<f0=%xmm2,>h1=%xmm3
vpaddq subc0(%rip),%xmm2,%xmm3

# qhasm: 2x h1 -= h0
# asm 1: psubq <h0=reg128#2,<h1=reg128#4
# asm 2: psubq <h0=%xmm1,<h1=%xmm3
psubq %xmm1,%xmm3

# qhasm: f1 = unpack_high(f0, h1)
# asm 1: vpunpckhqdq <h1=reg128#4,<f0=reg128#3,>f1=reg128#2
# asm 2: vpunpckhqdq <h1=%xmm3,<f0=%xmm2,>f1=%xmm1
vpunpckhqdq %xmm3,%xmm2,%xmm1

# qhasm: f0 = unpack_low(f0, h1)
# asm 1: vpunpcklqdq <h1=reg128#4,<f0=reg128#3,>f0=reg128#3
# asm 2: vpunpcklqdq <h1=%xmm3,<f0=%xmm2,>f0=%xmm2
vpunpcklqdq %xmm3,%xmm2,%xmm2

# qhasm: f0_stack = f0
# asm 1: movdqa <f0=reg128#3,>f0_stack=stack128#31
# asm 2: movdqa <f0=%xmm2,>f0_stack=480(%rsp)
movdqa %xmm2,480(%rsp)

# qhasm: f1_stack = f1
# asm 1: movdqa <f1=reg128#2,>f1_stack=stack128#33
# asm 2: movdqa <f1=%xmm1,>f1_stack=512(%rsp)
movdqa %xmm1,512(%rsp)

# qhasm: 2x f1 <<= 1
# asm 1: psllq $1,<f1=reg128#2
# asm 2: psllq $1,<f1=%xmm1
psllq $1,%xmm1

# qhasm: f1_2_stack = f1
# asm 1: movdqa <f1=reg128#2,>f1_2_stack=stack128#34
# asm 2: movdqa <f1=%xmm1,>f1_2_stack=528(%rsp)
movdqa %xmm1,528(%rsp)

# qhasm: 2x h1 *= mem128[ v121666_121666 ]
# asm 1: pmuludq v121666_121666,<h1=reg128#4
# asm 2: pmuludq v121666_121666,<h1=%xmm3
pmuludq v121666_121666(%rip),%xmm3

# qhasm: r = z3_0
# asm 1: movdqa <z3_0=stack128#6,>r=reg128#2
# asm 2: movdqa <z3_0=80(%rsp),>r=%xmm1
movdqa 80(%rsp),%xmm1

# qhasm: g0 = unpack_low(h1, r)
# asm 1: vpunpcklqdq <r=reg128#2,<h1=reg128#4,>g0=reg128#3
# asm 2: vpunpcklqdq <r=%xmm1,<h1=%xmm3,>g0=%xmm2
vpunpcklqdq %xmm1,%xmm3,%xmm2

# qhasm: g1 = unpack_high(h1, r)
# asm 1: vpunpckhqdq <r=reg128#2,<h1=reg128#4,>g1=reg128#2
# asm 2: vpunpckhqdq <r=%xmm1,<h1=%xmm3,>g1=%xmm1
vpunpckhqdq %xmm1,%xmm3,%xmm1

# qhasm: f2 = unpack_high(h2, h3)
# asm 1: vpunpckhqdq <h3=reg128#8,<h2=reg128#6,>f2=reg128#4
# asm 2: vpunpckhqdq <h3=%xmm7,<h2=%xmm5,>f2=%xmm3
vpunpckhqdq %xmm7,%xmm5,%xmm3

# qhasm: h2 = unpack_low(h2, h3)
# asm 1: vpunpcklqdq <h3=reg128#8,<h2=reg128#6,>h2=reg128#5
# asm 2: vpunpcklqdq <h3=%xmm7,<h2=%xmm5,>h2=%xmm4
vpunpcklqdq %xmm7,%xmm5,%xmm4

# qhasm: t0_2 = h2
# asm 1: movdqa <h2=reg128#5,>t0_2=stack128#35
# asm 2: movdqa <h2=%xmm4,>t0_2=544(%rsp)
movdqa %xmm4,544(%rsp)

# qhasm: 2x h3 = f2 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<f2=reg128#4,>h3=reg128#6
# asm 2: vpaddq subc2,<f2=%xmm3,>h3=%xmm5
vpaddq subc2(%rip),%xmm3,%xmm5

# qhasm: 2x h3 -= h2
# asm 1: psubq <h2=reg128#5,<h3=reg128#6
# asm 2: psubq <h2=%xmm4,<h3=%xmm5
psubq %xmm4,%xmm5

# qhasm: f3 = unpack_high(f2, h3)
# asm 1: vpunpckhqdq <h3=reg128#6,<f2=reg128#4,>f3=reg128#5
# asm 2: vpunpckhqdq <h3=%xmm5,<f2=%xmm3,>f3=%xmm4
vpunpckhqdq %xmm5,%xmm3,%xmm4

# qhasm: f2 = unpack_low(f2, h3)
# asm 1: vpunpcklqdq <h3=reg128#6,<f2=reg128#4,>f2=reg128#4
# asm 2: vpunpcklqdq <h3=%xmm5,<f2=%xmm3,>f2=%xmm3
vpunpcklqdq %xmm5,%xmm3,%xmm3

# qhasm: f2_stack = f2
# asm 1: movdqa <f2=reg128#4,>f2_stack=stack128#36
# asm 2: movdqa <f2=%xmm3,>f2_stack=560(%rsp)
movdqa %xmm3,560(%rsp)

# qhasm: f3_stack = f3
# asm 1: movdqa <f3=reg128#5,>f3_stack=stack128#37
# asm 2: movdqa <f3=%xmm4,>f3_stack=576(%rsp)
movdqa %xmm4,576(%rsp)

# qhasm: 2x f3 <<= 1
# asm 1: psllq $1,<f3=reg128#5
# asm 2: psllq $1,<f3=%xmm4
psllq $1,%xmm4

# qhasm: f3_2_stack = f3
# asm 1: movdqa <f3=reg128#5,>f3_2_stack=stack128#38
# asm 2: movdqa <f3=%xmm4,>f3_2_stack=592(%rsp)
movdqa %xmm4,592(%rsp)

# qhasm: 2x h3 *= mem128[ v121666_121666 ]
# asm 1: pmuludq v121666_121666,<h3=reg128#6
# asm 2: pmuludq v121666_121666,<h3=%xmm5
pmuludq v121666_121666(%rip),%xmm5

# qhasm: r = z3_2
# asm 1: movdqa <z3_2=stack128#7,>r=reg128#4
# asm 2: movdqa <z3_2=96(%rsp),>r=%xmm3
movdqa 96(%rsp),%xmm3

# qhasm: g2 = unpack_low(h3, r)
# asm 1: vpunpcklqdq <r=reg128#4,<h3=reg128#6,>g2=reg128#5
# asm 2: vpunpcklqdq <r=%xmm3,<h3=%xmm5,>g2=%xmm4
vpunpcklqdq %xmm3,%xmm5,%xmm4

# qhasm: g3 = unpack_high(h3, r)
# asm 1: vpunpckhqdq <r=reg128#4,<h3=reg128#6,>g3=reg128#4
# asm 2: vpunpckhqdq <r=%xmm3,<h3=%xmm5,>g3=%xmm3
vpunpckhqdq %xmm3,%xmm5,%xmm3

# qhasm: f4 = unpack_high(h4, h5)
# asm 1: vpunpckhqdq <h5=reg128#11,<h4=reg128#10,>f4=reg128#6
# asm 2: vpunpckhqdq <h5=%xmm10,<h4=%xmm9,>f4=%xmm5
vpunpckhqdq %xmm10,%xmm9,%xmm5

# qhasm: h4 = unpack_low(h4, h5)
# asm 1: vpunpcklqdq <h5=reg128#11,<h4=reg128#10,>h4=reg128#7
# asm 2: vpunpcklqdq <h5=%xmm10,<h4=%xmm9,>h4=%xmm6
vpunpcklqdq %xmm10,%xmm9,%xmm6

# qhasm: t0_4 = h4
# asm 1: movdqa <h4=reg128#7,>t0_4=stack128#39
# asm 2: movdqa <h4=%xmm6,>t0_4=608(%rsp)
movdqa %xmm6,608(%rsp)

# qhasm: 2x h5 = f4 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<f4=reg128#6,>h5=reg128#8
# asm 2: vpaddq subc2,<f4=%xmm5,>h5=%xmm7
vpaddq subc2(%rip),%xmm5,%xmm7

# qhasm: 2x h5 -= h4
# asm 1: psubq <h4=reg128#7,<h5=reg128#8
# asm 2: psubq <h4=%xmm6,<h5=%xmm7
psubq %xmm6,%xmm7

# qhasm: f5 = unpack_high(f4, h5)
# asm 1: vpunpckhqdq <h5=reg128#8,<f4=reg128#6,>f5=reg128#7
# asm 2: vpunpckhqdq <h5=%xmm7,<f4=%xmm5,>f5=%xmm6
vpunpckhqdq %xmm7,%xmm5,%xmm6

# qhasm: f4 = unpack_low(f4, h5)
# asm 1: vpunpcklqdq <h5=reg128#8,<f4=reg128#6,>f4=reg128#6
# asm 2: vpunpcklqdq <h5=%xmm7,<f4=%xmm5,>f4=%xmm5
vpunpcklqdq %xmm7,%xmm5,%xmm5

# qhasm: f4_stack = f4
# asm 1: movdqa <f4=reg128#6,>f4_stack=stack128#40
# asm 2: movdqa <f4=%xmm5,>f4_stack=624(%rsp)
movdqa %xmm5,624(%rsp)

# qhasm: f5_stack = f5
# asm 1: movdqa <f5=reg128#7,>f5_stack=stack128#41
# asm 2: movdqa <f5=%xmm6,>f5_stack=640(%rsp)
movdqa %xmm6,640(%rsp)

# qhasm: 2x f5 <<= 1
# asm 1: psllq $1,<f5=reg128#7
# asm 2: psllq $1,<f5=%xmm6
psllq $1,%xmm6

# qhasm: f5_2_stack = f5
# asm 1: movdqa <f5=reg128#7,>f5_2_stack=stack128#42
# asm 2: movdqa <f5=%xmm6,>f5_2_stack=656(%rsp)
movdqa %xmm6,656(%rsp)

# qhasm: 2x h5 *= mem128[ v121666_121666 ]
# asm 1: pmuludq v121666_121666,<h5=reg128#8
# asm 2: pmuludq v121666_121666,<h5=%xmm7
pmuludq v121666_121666(%rip),%xmm7

# qhasm: r = z3_4
# asm 1: movdqa <z3_4=stack128#8,>r=reg128#6
# asm 2: movdqa <z3_4=112(%rsp),>r=%xmm5
movdqa 112(%rsp),%xmm5

# qhasm: g4 = unpack_low(h5, r)
# asm 1: vpunpcklqdq <r=reg128#6,<h5=reg128#8,>g4=reg128#7
# asm 2: vpunpcklqdq <r=%xmm5,<h5=%xmm7,>g4=%xmm6
vpunpcklqdq %xmm5,%xmm7,%xmm6

# qhasm: g5 = unpack_high(h5, r)
# asm 1: vpunpckhqdq <r=reg128#6,<h5=reg128#8,>g5=reg128#6
# asm 2: vpunpckhqdq <r=%xmm5,<h5=%xmm7,>g5=%xmm5
vpunpckhqdq %xmm5,%xmm7,%xmm5

# qhasm: f6 = unpack_high(h6, h7)
# asm 1: vpunpckhqdq <h7=reg128#13,<h6=reg128#12,>f6=reg128#8
# asm 2: vpunpckhqdq <h7=%xmm12,<h6=%xmm11,>f6=%xmm7
vpunpckhqdq %xmm12,%xmm11,%xmm7

# qhasm: h6 = unpack_low(h6, h7)
# asm 1: vpunpcklqdq <h7=reg128#13,<h6=reg128#12,>h6=reg128#9
# asm 2: vpunpcklqdq <h7=%xmm12,<h6=%xmm11,>h6=%xmm8
vpunpcklqdq %xmm12,%xmm11,%xmm8

# qhasm: t0_6 = h6
# asm 1: movdqa <h6=reg128#9,>t0_6=stack128#43
# asm 2: movdqa <h6=%xmm8,>t0_6=672(%rsp)
movdqa %xmm8,672(%rsp)

# qhasm: 2x h7 = f6 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<f6=reg128#8,>h7=reg128#10
# asm 2: vpaddq subc2,<f6=%xmm7,>h7=%xmm9
vpaddq subc2(%rip),%xmm7,%xmm9

# qhasm: 2x h7 -= h6
# asm 1: psubq <h6=reg128#9,<h7=reg128#10
# asm 2: psubq <h6=%xmm8,<h7=%xmm9
psubq %xmm8,%xmm9

# qhasm: f7 = unpack_high(f6, h7)
# asm 1: vpunpckhqdq <h7=reg128#10,<f6=reg128#8,>f7=reg128#9
# asm 2: vpunpckhqdq <h7=%xmm9,<f6=%xmm7,>f7=%xmm8
vpunpckhqdq %xmm9,%xmm7,%xmm8

# qhasm: f6 = unpack_low(f6, h7)
# asm 1: vpunpcklqdq <h7=reg128#10,<f6=reg128#8,>f6=reg128#8
# asm 2: vpunpcklqdq <h7=%xmm9,<f6=%xmm7,>f6=%xmm7
vpunpcklqdq %xmm9,%xmm7,%xmm7

# qhasm: f6_stack = f6
# asm 1: movdqa <f6=reg128#8,>f6_stack=stack128#44
# asm 2: movdqa <f6=%xmm7,>f6_stack=688(%rsp)
movdqa %xmm7,688(%rsp)

# qhasm: f7_stack = f7
# asm 1: movdqa <f7=reg128#9,>f7_stack=stack128#45
# asm 2: movdqa <f7=%xmm8,>f7_stack=704(%rsp)
movdqa %xmm8,704(%rsp)

# qhasm: 2x f7 <<= 1
# asm 1: psllq $1,<f7=reg128#9
# asm 2: psllq $1,<f7=%xmm8
psllq $1,%xmm8

# qhasm: f7_2_stack = f7
# asm 1: movdqa <f7=reg128#9,>f7_2_stack=stack128#46
# asm 2: movdqa <f7=%xmm8,>f7_2_stack=720(%rsp)
movdqa %xmm8,720(%rsp)

# qhasm: 2x h7 *= mem128[ v121666_121666 ]
# asm 1: pmuludq v121666_121666,<h7=reg128#10
# asm 2: pmuludq v121666_121666,<h7=%xmm9
pmuludq v121666_121666(%rip),%xmm9

# qhasm: r = z3_6
# asm 1: movdqa <z3_6=stack128#29,>r=reg128#8
# asm 2: movdqa <z3_6=448(%rsp),>r=%xmm7
movdqa 448(%rsp),%xmm7

# qhasm: g6 = unpack_low(h7, r)
# asm 1: vpunpcklqdq <r=reg128#8,<h7=reg128#10,>g6=reg128#9
# asm 2: vpunpcklqdq <r=%xmm7,<h7=%xmm9,>g6=%xmm8
vpunpcklqdq %xmm7,%xmm9,%xmm8

# qhasm: g7 = unpack_high(h7, r)
# asm 1: vpunpckhqdq <r=reg128#8,<h7=reg128#10,>g7=reg128#8
# asm 2: vpunpckhqdq <r=%xmm7,<h7=%xmm9,>g7=%xmm7
vpunpckhqdq %xmm7,%xmm9,%xmm7

# qhasm: f8 = unpack_high(h8, h9)
# asm 1: vpunpckhqdq <h9=reg128#1,<h8=reg128#14,>f8=reg128#10
# asm 2: vpunpckhqdq <h9=%xmm0,<h8=%xmm13,>f8=%xmm9
vpunpckhqdq %xmm0,%xmm13,%xmm9

# qhasm: h8 = unpack_low(h8, h9)
# asm 1: vpunpcklqdq <h9=reg128#1,<h8=reg128#14,>h8=reg128#1
# asm 2: vpunpcklqdq <h9=%xmm0,<h8=%xmm13,>h8=%xmm0
vpunpcklqdq %xmm0,%xmm13,%xmm0

# qhasm: t0_8 = h8
# asm 1: movdqa <h8=reg128#1,>t0_8=stack128#29
# asm 2: movdqa <h8=%xmm0,>t0_8=448(%rsp)
movdqa %xmm0,448(%rsp)

# qhasm: 2x h9 = f8 + mem128[ subc2 ]
# asm 1: vpaddq subc2,<f8=reg128#10,>h9=reg128#11
# asm 2: vpaddq subc2,<f8=%xmm9,>h9=%xmm10
vpaddq subc2(%rip),%xmm9,%xmm10

# qhasm: 2x h9 -= h8
# asm 1: psubq <h8=reg128#1,<h9=reg128#11
# asm 2: psubq <h8=%xmm0,<h9=%xmm10
psubq %xmm0,%xmm10

# qhasm: f9 = unpack_high(f8, h9)
# asm 1: vpunpckhqdq <h9=reg128#11,<f8=reg128#10,>f9=reg128#1
# asm 2: vpunpckhqdq <h9=%xmm10,<f8=%xmm9,>f9=%xmm0
vpunpckhqdq %xmm10,%xmm9,%xmm0

# qhasm: f8 = unpack_low(f8, h9)
# asm 1: vpunpcklqdq <h9=reg128#11,<f8=reg128#10,>f8=reg128#10
# asm 2: vpunpcklqdq <h9=%xmm10,<f8=%xmm9,>f8=%xmm9
vpunpcklqdq %xmm10,%xmm9,%xmm9

# qhasm: f8_stack = f8
# asm 1: movdqa <f8=reg128#10,>f8_stack=stack128#47
# asm 2: movdqa <f8=%xmm9,>f8_stack=736(%rsp)
movdqa %xmm9,736(%rsp)

# qhasm: f9_stack = f9
# asm 1: movdqa <f9=reg128#1,>f9_stack=stack128#48
# asm 2: movdqa <f9=%xmm0,>f9_stack=752(%rsp)
movdqa %xmm0,752(%rsp)

# qhasm: 2x f9 <<= 1
# asm 1: psllq $1,<f9=reg128#1
# asm 2: psllq $1,<f9=%xmm0
psllq $1,%xmm0

# qhasm: f9_2_stack = f9
# asm 1: movdqa <f9=reg128#1,>f9_2_stack=stack128#49
# asm 2: movdqa <f9=%xmm0,>f9_2_stack=768(%rsp)
movdqa %xmm0,768(%rsp)

# qhasm: 2x h9 *= mem128[ v121666_121666 ]
# asm 1: pmuludq v121666_121666,<h9=reg128#11
# asm 2: pmuludq v121666_121666,<h9=%xmm10
pmuludq v121666_121666(%rip),%xmm10

# qhasm: r = z3_8
# asm 1: movdqa <z3_8=stack128#32,>r=reg128#1
# asm 2: movdqa <z3_8=496(%rsp),>r=%xmm0
movdqa 496(%rsp),%xmm0

# qhasm: g8 = unpack_low(h9, r)
# asm 1: vpunpcklqdq <r=reg128#1,<h9=reg128#11,>g8=reg128#10
# asm 2: vpunpcklqdq <r=%xmm0,<h9=%xmm10,>g8=%xmm9
vpunpcklqdq %xmm0,%xmm10,%xmm9

# qhasm: g9 = unpack_high(h9, r)
# asm 1: vpunpckhqdq <r=reg128#1,<h9=reg128#11,>g9=reg128#1
# asm 2: vpunpckhqdq <r=%xmm0,<h9=%xmm10,>g9=%xmm0
vpunpckhqdq %xmm0,%xmm10,%xmm0

# qhasm: 2x carry0 = g0 unsigned>>= 26
# asm 1: vpsrlq $26,<g0=reg128#3,>carry0=reg128#11
# asm 2: vpsrlq $26,<g0=%xmm2,>carry0=%xmm10
vpsrlq $26,%xmm2,%xmm10

# qhasm: 2x g1 += carry0
# asm 1: paddq <carry0=reg128#11,<g1=reg128#2
# asm 2: paddq <carry0=%xmm10,<g1=%xmm1
paddq %xmm10,%xmm1

# qhasm: g0 &= mem128[ m26 ]
# asm 1: pand m26,<g0=reg128#3
# asm 2: pand m26,<g0=%xmm2
pand m26(%rip),%xmm2

# qhasm: 2x carry5 = g5 unsigned>>= 25
# asm 1: vpsrlq $25,<g5=reg128#6,>carry5=reg128#11
# asm 2: vpsrlq $25,<g5=%xmm5,>carry5=%xmm10
vpsrlq $25,%xmm5,%xmm10

# qhasm: 2x g6 += carry5
# asm 1: paddq <carry5=reg128#11,<g6=reg128#9
# asm 2: paddq <carry5=%xmm10,<g6=%xmm8
paddq %xmm10,%xmm8

# qhasm: g5 &= mem128[ m25 ]
# asm 1: pand m25,<g5=reg128#6
# asm 2: pand m25,<g5=%xmm5
pand m25(%rip),%xmm5

# qhasm: 2x carry1 = g1 unsigned>>= 25
# asm 1: vpsrlq $25,<g1=reg128#2,>carry1=reg128#11
# asm 2: vpsrlq $25,<g1=%xmm1,>carry1=%xmm10
vpsrlq $25,%xmm1,%xmm10

# qhasm: 2x g2 += carry1
# asm 1: paddq <carry1=reg128#11,<g2=reg128#5
# asm 2: paddq <carry1=%xmm10,<g2=%xmm4
paddq %xmm10,%xmm4

# qhasm: g1 &= mem128[ m25 ]
# asm 1: pand m25,<g1=reg128#2
# asm 2: pand m25,<g1=%xmm1
pand m25(%rip),%xmm1

# qhasm: 2x carry6 = g6 unsigned>>= 26
# asm 1: vpsrlq $26,<g6=reg128#9,>carry6=reg128#11
# asm 2: vpsrlq $26,<g6=%xmm8,>carry6=%xmm10
vpsrlq $26,%xmm8,%xmm10

# qhasm: 2x g7 += carry6
# asm 1: paddq <carry6=reg128#11,<g7=reg128#8
# asm 2: paddq <carry6=%xmm10,<g7=%xmm7
paddq %xmm10,%xmm7

# qhasm: g6 &= mem128[ m26 ]
# asm 1: pand m26,<g6=reg128#9
# asm 2: pand m26,<g6=%xmm8
pand m26(%rip),%xmm8

# qhasm: 2x carry2 = g2 unsigned>>= 26
# asm 1: vpsrlq $26,<g2=reg128#5,>carry2=reg128#11
# asm 2: vpsrlq $26,<g2=%xmm4,>carry2=%xmm10
vpsrlq $26,%xmm4,%xmm10

# qhasm: 2x g3 += carry2
# asm 1: paddq <carry2=reg128#11,<g3=reg128#4
# asm 2: paddq <carry2=%xmm10,<g3=%xmm3
paddq %xmm10,%xmm3

# qhasm: g2 &= mem128[ m26 ]
# asm 1: pand m26,<g2=reg128#5
# asm 2: pand m26,<g2=%xmm4
pand m26(%rip),%xmm4

# qhasm: 2x carry7 = g7 unsigned>>= 25
# asm 1: vpsrlq $25,<g7=reg128#8,>carry7=reg128#11
# asm 2: vpsrlq $25,<g7=%xmm7,>carry7=%xmm10
vpsrlq $25,%xmm7,%xmm10

# qhasm: 2x g8 += carry7
# asm 1: paddq <carry7=reg128#11,<g8=reg128#10
# asm 2: paddq <carry7=%xmm10,<g8=%xmm9
paddq %xmm10,%xmm9

# qhasm: g7 &= mem128[ m25 ]
# asm 1: pand m25,<g7=reg128#8
# asm 2: pand m25,<g7=%xmm7
pand m25(%rip),%xmm7

# qhasm: 2x carry3 = g3 unsigned>>= 25
# asm 1: vpsrlq $25,<g3=reg128#4,>carry3=reg128#11
# asm 2: vpsrlq $25,<g3=%xmm3,>carry3=%xmm10
vpsrlq $25,%xmm3,%xmm10

# qhasm: 2x g4 += carry3
# asm 1: paddq <carry3=reg128#11,<g4=reg128#7
# asm 2: paddq <carry3=%xmm10,<g4=%xmm6
paddq %xmm10,%xmm6

# qhasm: g3 &= mem128[ m25 ]
# asm 1: pand m25,<g3=reg128#4
# asm 2: pand m25,<g3=%xmm3
pand m25(%rip),%xmm3

# qhasm: 2x carry8 = g8 unsigned>>= 26
# asm 1: vpsrlq $26,<g8=reg128#10,>carry8=reg128#11
# asm 2: vpsrlq $26,<g8=%xmm9,>carry8=%xmm10
vpsrlq $26,%xmm9,%xmm10

# qhasm: 2x g9 += carry8
# asm 1: paddq <carry8=reg128#11,<g9=reg128#1
# asm 2: paddq <carry8=%xmm10,<g9=%xmm0
paddq %xmm10,%xmm0

# qhasm: g8 &= mem128[ m26 ]
# asm 1: pand m26,<g8=reg128#10
# asm 2: pand m26,<g8=%xmm9
pand m26(%rip),%xmm9

# qhasm: 2x carry4 = g4 unsigned>>= 26
# asm 1: vpsrlq $26,<g4=reg128#7,>carry4=reg128#11
# asm 2: vpsrlq $26,<g4=%xmm6,>carry4=%xmm10
vpsrlq $26,%xmm6,%xmm10

# qhasm: 2x g5 += carry4
# asm 1: paddq <carry4=reg128#11,<g5=reg128#6
# asm 2: paddq <carry4=%xmm10,<g5=%xmm5
paddq %xmm10,%xmm5

# qhasm: g4 &= mem128[ m26 ]
# asm 1: pand m26,<g4=reg128#7
# asm 2: pand m26,<g4=%xmm6
pand m26(%rip),%xmm6

# qhasm: 2x carry9 = g9 unsigned>>= 25
# asm 1: vpsrlq $25,<g9=reg128#1,>carry9=reg128#11
# asm 2: vpsrlq $25,<g9=%xmm0,>carry9=%xmm10
vpsrlq $25,%xmm0,%xmm10

# qhasm: 2x r0 = carry9 << 4
# asm 1: vpsllq $4,<carry9=reg128#11,>r0=reg128#12
# asm 2: vpsllq $4,<carry9=%xmm10,>r0=%xmm11
vpsllq $4,%xmm10,%xmm11

# qhasm: 2x g0 += carry9
# asm 1: paddq <carry9=reg128#11,<g0=reg128#3
# asm 2: paddq <carry9=%xmm10,<g0=%xmm2
paddq %xmm10,%xmm2

# qhasm: 2x carry9 <<= 1
# asm 1: psllq $1,<carry9=reg128#11
# asm 2: psllq $1,<carry9=%xmm10
psllq $1,%xmm10

# qhasm: 2x r0 += carry9
# asm 1: paddq <carry9=reg128#11,<r0=reg128#12
# asm 2: paddq <carry9=%xmm10,<r0=%xmm11
paddq %xmm10,%xmm11

# qhasm: 2x g0 += r0
# asm 1: paddq <r0=reg128#12,<g0=reg128#3
# asm 2: paddq <r0=%xmm11,<g0=%xmm2
paddq %xmm11,%xmm2

# qhasm: g9 &= mem128[ m25 ]
# asm 1: pand m25,<g9=reg128#1
# asm 2: pand m25,<g9=%xmm0
pand m25(%rip),%xmm0

# qhasm: 2x carry5 = g5 unsigned>>= 25
# asm 1: vpsrlq $25,<g5=reg128#6,>carry5=reg128#11
# asm 2: vpsrlq $25,<g5=%xmm5,>carry5=%xmm10
vpsrlq $25,%xmm5,%xmm10

# qhasm: 2x g6 += carry5
# asm 1: paddq <carry5=reg128#11,<g6=reg128#9
# asm 2: paddq <carry5=%xmm10,<g6=%xmm8
paddq %xmm10,%xmm8

# qhasm: g5 &= mem128[ m25 ]
# asm 1: pand m25,<g5=reg128#6
# asm 2: pand m25,<g5=%xmm5
pand m25(%rip),%xmm5

# qhasm: 2x carry0 = g0 unsigned>>= 26
# asm 1: vpsrlq $26,<g0=reg128#3,>carry0=reg128#11
# asm 2: vpsrlq $26,<g0=%xmm2,>carry0=%xmm10
vpsrlq $26,%xmm2,%xmm10

# qhasm: 2x g1 += carry0
# asm 1: paddq <carry0=reg128#11,<g1=reg128#2
# asm 2: paddq <carry0=%xmm10,<g1=%xmm1
paddq %xmm10,%xmm1

# qhasm: g0 &= mem128[ m26 ]
# asm 1: pand m26,<g0=reg128#3
# asm 2: pand m26,<g0=%xmm2
pand m26(%rip),%xmm2

# qhasm: r = unpack_high(g0, g1)
# asm 1: vpunpckhqdq <g1=reg128#2,<g0=reg128#3,>r=reg128#11
# asm 2: vpunpckhqdq <g1=%xmm1,<g0=%xmm2,>r=%xmm10
vpunpckhqdq %xmm1,%xmm2,%xmm10

# qhasm: z3_0 = r
# asm 1: movdqa <r=reg128#11,>z3_0=stack128#6
# asm 2: movdqa <r=%xmm10,>z3_0=80(%rsp)
movdqa %xmm10,80(%rsp)

# qhasm: h1 = unpack_low(g0, g1)
# asm 1: vpunpcklqdq <g1=reg128#2,<g0=reg128#3,>h1=reg128#2
# asm 2: vpunpcklqdq <g1=%xmm1,<g0=%xmm2,>h1=%xmm1
vpunpcklqdq %xmm1,%xmm2,%xmm1

# qhasm: r = unpack_high(g2, g3)
# asm 1: vpunpckhqdq <g3=reg128#4,<g2=reg128#5,>r=reg128#3
# asm 2: vpunpckhqdq <g3=%xmm3,<g2=%xmm4,>r=%xmm2
vpunpckhqdq %xmm3,%xmm4,%xmm2

# qhasm: z3_2 = r
# asm 1: movdqa <r=reg128#3,>z3_2=stack128#7
# asm 2: movdqa <r=%xmm2,>z3_2=96(%rsp)
movdqa %xmm2,96(%rsp)

# qhasm: h3 = unpack_low(g2, g3)
# asm 1: vpunpcklqdq <g3=reg128#4,<g2=reg128#5,>h3=reg128#3
# asm 2: vpunpcklqdq <g3=%xmm3,<g2=%xmm4,>h3=%xmm2
vpunpcklqdq %xmm3,%xmm4,%xmm2

# qhasm: r = unpack_high(g4, g5)
# asm 1: vpunpckhqdq <g5=reg128#6,<g4=reg128#7,>r=reg128#4
# asm 2: vpunpckhqdq <g5=%xmm5,<g4=%xmm6,>r=%xmm3
vpunpckhqdq %xmm5,%xmm6,%xmm3

# qhasm: z3_4 = r
# asm 1: movdqa <r=reg128#4,>z3_4=stack128#8
# asm 2: movdqa <r=%xmm3,>z3_4=112(%rsp)
movdqa %xmm3,112(%rsp)

# qhasm: h5 = unpack_low(g4, g5)
# asm 1: vpunpcklqdq <g5=reg128#6,<g4=reg128#7,>h5=reg128#4
# asm 2: vpunpcklqdq <g5=%xmm5,<g4=%xmm6,>h5=%xmm3
vpunpcklqdq %xmm5,%xmm6,%xmm3

# qhasm: r = unpack_high(g6, g7)
# asm 1: vpunpckhqdq <g7=reg128#8,<g6=reg128#9,>r=reg128#5
# asm 2: vpunpckhqdq <g7=%xmm7,<g6=%xmm8,>r=%xmm4
vpunpckhqdq %xmm7,%xmm8,%xmm4

# qhasm: z3_6 = r
# asm 1: movdqa <r=reg128#5,>z3_6=stack128#9
# asm 2: movdqa <r=%xmm4,>z3_6=128(%rsp)
movdqa %xmm4,128(%rsp)

# qhasm: h7 = unpack_low(g6, g7)
# asm 1: vpunpcklqdq <g7=reg128#8,<g6=reg128#9,>h7=reg128#5
# asm 2: vpunpcklqdq <g7=%xmm7,<g6=%xmm8,>h7=%xmm4
vpunpcklqdq %xmm7,%xmm8,%xmm4

# qhasm: r = unpack_high(g8, g9)
# asm 1: vpunpckhqdq <g9=reg128#1,<g8=reg128#10,>r=reg128#6
# asm 2: vpunpckhqdq <g9=%xmm0,<g8=%xmm9,>r=%xmm5
vpunpckhqdq %xmm0,%xmm9,%xmm5

# qhasm: z3_8 = r
# asm 1: movdqa <r=reg128#6,>z3_8=stack128#10
# asm 2: movdqa <r=%xmm5,>z3_8=144(%rsp)
movdqa %xmm5,144(%rsp)

# qhasm: h9 = unpack_low(g8, g9)
# asm 1: vpunpcklqdq <g9=reg128#1,<g8=reg128#10,>h9=reg128#1
# asm 2: vpunpcklqdq <g9=%xmm0,<g8=%xmm9,>h9=%xmm0
vpunpcklqdq %xmm0,%xmm9,%xmm0

# qhasm: h0 = t0_0
# asm 1: movdqa <t0_0=stack128#30,>h0=reg128#6
# asm 2: movdqa <t0_0=464(%rsp),>h0=%xmm5
movdqa 464(%rsp),%xmm5

# qhasm: 2x h1 += h0
# asm 1: paddq <h0=reg128#6,<h1=reg128#2
# asm 2: paddq <h0=%xmm5,<h1=%xmm1
paddq %xmm5,%xmm1

# qhasm: g0 = unpack_low(h0, h1)
# asm 1: vpunpcklqdq <h1=reg128#2,<h0=reg128#6,>g0=reg128#7
# asm 2: vpunpcklqdq <h1=%xmm1,<h0=%xmm5,>g0=%xmm6
vpunpcklqdq %xmm1,%xmm5,%xmm6

# qhasm: g1 = unpack_high(h0, h1)
# asm 1: vpunpckhqdq <h1=reg128#2,<h0=reg128#6,>g1=reg128#2
# asm 2: vpunpckhqdq <h1=%xmm1,<h0=%xmm5,>g1=%xmm1
vpunpckhqdq %xmm1,%xmm5,%xmm1

# qhasm: 2x m1 = g0 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#33,<g0=reg128#7,>m1=reg128#6
# asm 2: vpmuludq <f1_stack=512(%rsp),<g0=%xmm6,>m1=%xmm5
vpmuludq 512(%rsp),%xmm6,%xmm5

# qhasm: 2x r = g1 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g1=reg128#2,>r=reg128#8
# asm 2: vpmuludq <f0_stack=480(%rsp),<g1=%xmm1,>r=%xmm7
vpmuludq 480(%rsp),%xmm1,%xmm7

# qhasm: 2x m1 += r
# asm 1: paddq <r=reg128#8,<m1=reg128#6
# asm 2: paddq <r=%xmm7,<m1=%xmm5
paddq %xmm7,%xmm5

# qhasm: 2x m2 = g0 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g0=reg128#7,>m2=reg128#8
# asm 2: vpmuludq <f2_stack=560(%rsp),<g0=%xmm6,>m2=%xmm7
vpmuludq 560(%rsp),%xmm6,%xmm7

# qhasm: 2x r = g1 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#34,<g1=reg128#2,>r=reg128#9
# asm 2: vpmuludq <f1_2_stack=528(%rsp),<g1=%xmm1,>r=%xmm8
vpmuludq 528(%rsp),%xmm1,%xmm8

# qhasm: 2x m2 += r
# asm 1: paddq <r=reg128#9,<m2=reg128#8
# asm 2: paddq <r=%xmm8,<m2=%xmm7
paddq %xmm8,%xmm7

# qhasm: 2x m3 = g0 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#37,<g0=reg128#7,>m3=reg128#9
# asm 2: vpmuludq <f3_stack=576(%rsp),<g0=%xmm6,>m3=%xmm8
vpmuludq 576(%rsp),%xmm6,%xmm8

# qhasm: 2x r = g1 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g1=reg128#2,>r=reg128#10
# asm 2: vpmuludq <f2_stack=560(%rsp),<g1=%xmm1,>r=%xmm9
vpmuludq 560(%rsp),%xmm1,%xmm9

# qhasm: 2x m3 += r
# asm 1: paddq <r=reg128#10,<m3=reg128#9
# asm 2: paddq <r=%xmm9,<m3=%xmm8
paddq %xmm9,%xmm8

# qhasm: 2x m4 = g0 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g0=reg128#7,>m4=reg128#10
# asm 2: vpmuludq <f4_stack=624(%rsp),<g0=%xmm6,>m4=%xmm9
vpmuludq 624(%rsp),%xmm6,%xmm9

# qhasm: 2x r = g1 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#38,<g1=reg128#2,>r=reg128#11
# asm 2: vpmuludq <f3_2_stack=592(%rsp),<g1=%xmm1,>r=%xmm10
vpmuludq 592(%rsp),%xmm1,%xmm10

# qhasm: 2x m4 += r
# asm 1: paddq <r=reg128#11,<m4=reg128#10
# asm 2: paddq <r=%xmm10,<m4=%xmm9
paddq %xmm10,%xmm9

# qhasm: 2x m5 = g0 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#41,<g0=reg128#7,>m5=reg128#11
# asm 2: vpmuludq <f5_stack=640(%rsp),<g0=%xmm6,>m5=%xmm10
vpmuludq 640(%rsp),%xmm6,%xmm10

# qhasm: 2x r = g1 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g1=reg128#2,>r=reg128#12
# asm 2: vpmuludq <f4_stack=624(%rsp),<g1=%xmm1,>r=%xmm11
vpmuludq 624(%rsp),%xmm1,%xmm11

# qhasm: 2x m5 += r
# asm 1: paddq <r=reg128#12,<m5=reg128#11
# asm 2: paddq <r=%xmm11,<m5=%xmm10
paddq %xmm11,%xmm10

# qhasm: 2x m6 = g0 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g0=reg128#7,>m6=reg128#12
# asm 2: vpmuludq <f6_stack=688(%rsp),<g0=%xmm6,>m6=%xmm11
vpmuludq 688(%rsp),%xmm6,%xmm11

# qhasm: 2x r = g1 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#42,<g1=reg128#2,>r=reg128#13
# asm 2: vpmuludq <f5_2_stack=656(%rsp),<g1=%xmm1,>r=%xmm12
vpmuludq 656(%rsp),%xmm1,%xmm12

# qhasm: 2x m6 += r
# asm 1: paddq <r=reg128#13,<m6=reg128#12
# asm 2: paddq <r=%xmm12,<m6=%xmm11
paddq %xmm12,%xmm11

# qhasm: 2x m7 = g0 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#45,<g0=reg128#7,>m7=reg128#13
# asm 2: vpmuludq <f7_stack=704(%rsp),<g0=%xmm6,>m7=%xmm12
vpmuludq 704(%rsp),%xmm6,%xmm12

# qhasm: 2x r = g1 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g1=reg128#2,>r=reg128#14
# asm 2: vpmuludq <f6_stack=688(%rsp),<g1=%xmm1,>r=%xmm13
vpmuludq 688(%rsp),%xmm1,%xmm13

# qhasm: 2x m7 += r
# asm 1: paddq <r=reg128#14,<m7=reg128#13
# asm 2: paddq <r=%xmm13,<m7=%xmm12
paddq %xmm13,%xmm12

# qhasm: 2x m8 = g0 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g0=reg128#7,>m8=reg128#14
# asm 2: vpmuludq <f8_stack=736(%rsp),<g0=%xmm6,>m8=%xmm13
vpmuludq 736(%rsp),%xmm6,%xmm13

# qhasm: 2x r = g1 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#46,<g1=reg128#2,>r=reg128#15
# asm 2: vpmuludq <f7_2_stack=720(%rsp),<g1=%xmm1,>r=%xmm14
vpmuludq 720(%rsp),%xmm1,%xmm14

# qhasm: 2x m8 += r
# asm 1: paddq <r=reg128#15,<m8=reg128#14
# asm 2: paddq <r=%xmm14,<m8=%xmm13
paddq %xmm14,%xmm13

# qhasm: 2x m9 = g0 * f9_stack
# asm 1: vpmuludq <f9_stack=stack128#48,<g0=reg128#7,>m9=reg128#15
# asm 2: vpmuludq <f9_stack=752(%rsp),<g0=%xmm6,>m9=%xmm14
vpmuludq 752(%rsp),%xmm6,%xmm14

# qhasm: 2x r = g1 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g1=reg128#2,>r=reg128#16
# asm 2: vpmuludq <f8_stack=736(%rsp),<g1=%xmm1,>r=%xmm15
vpmuludq 736(%rsp),%xmm1,%xmm15

# qhasm: 2x m9 += r
# asm 1: paddq <r=reg128#16,<m9=reg128#15
# asm 2: paddq <r=%xmm15,<m9=%xmm14
paddq %xmm15,%xmm14

# qhasm: 2x m0 = g0 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g0=reg128#7,>m0=reg128#7
# asm 2: vpmuludq <f0_stack=480(%rsp),<g0=%xmm6,>m0=%xmm6
vpmuludq 480(%rsp),%xmm6,%xmm6

# qhasm: 2x g1 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g1=reg128#2
# asm 2: pmuludq v19_19,<g1=%xmm1
pmuludq v19_19(%rip),%xmm1

# qhasm: 2x r = g1 * f9_2_stack
# asm 1: vpmuludq <f9_2_stack=stack128#49,<g1=reg128#2,>r=reg128#2
# asm 2: vpmuludq <f9_2_stack=768(%rsp),<g1=%xmm1,>r=%xmm1
vpmuludq 768(%rsp),%xmm1,%xmm1

# qhasm: 2x m0 += r
# asm 1: paddq <r=reg128#2,<m0=reg128#7
# asm 2: paddq <r=%xmm1,<m0=%xmm6
paddq %xmm1,%xmm6

# qhasm: h2 = t0_2
# asm 1: movdqa <t0_2=stack128#35,>h2=reg128#2
# asm 2: movdqa <t0_2=544(%rsp),>h2=%xmm1
movdqa 544(%rsp),%xmm1

# qhasm: 2x h3 += h2
# asm 1: paddq <h2=reg128#2,<h3=reg128#3
# asm 2: paddq <h2=%xmm1,<h3=%xmm2
paddq %xmm1,%xmm2

# qhasm: g2 = unpack_low(h2, h3)
# asm 1: vpunpcklqdq <h3=reg128#3,<h2=reg128#2,>g2=reg128#16
# asm 2: vpunpcklqdq <h3=%xmm2,<h2=%xmm1,>g2=%xmm15
vpunpcklqdq %xmm2,%xmm1,%xmm15

# qhasm: g3 = unpack_high(h2, h3)
# asm 1: vpunpckhqdq <h3=reg128#3,<h2=reg128#2,>g3=reg128#2
# asm 2: vpunpckhqdq <h3=%xmm2,<h2=%xmm1,>g3=%xmm1
vpunpckhqdq %xmm2,%xmm1,%xmm1

# qhasm: 2x r2 = g2 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f0_stack=480(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 480(%rsp),%xmm15,%xmm2

# qhasm: 2x m2 += r2
# asm 1: paddq <r2=reg128#3,<m2=reg128#8
# asm 2: paddq <r2=%xmm2,<m2=%xmm7
paddq %xmm2,%xmm7

# qhasm: 2x r2 = g2 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#33,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f1_stack=512(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 512(%rsp),%xmm15,%xmm2

# qhasm: 2x m3 += r2
# asm 1: paddq <r2=reg128#3,<m3=reg128#9
# asm 2: paddq <r2=%xmm2,<m3=%xmm8
paddq %xmm2,%xmm8

# qhasm: 2x r2 = g2 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f2_stack=560(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 560(%rsp),%xmm15,%xmm2

# qhasm: 2x m4 += r2
# asm 1: paddq <r2=reg128#3,<m4=reg128#10
# asm 2: paddq <r2=%xmm2,<m4=%xmm9
paddq %xmm2,%xmm9

# qhasm: 2x r2 = g2 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#37,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f3_stack=576(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 576(%rsp),%xmm15,%xmm2

# qhasm: 2x m5 += r2
# asm 1: paddq <r2=reg128#3,<m5=reg128#11
# asm 2: paddq <r2=%xmm2,<m5=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x r2 = g2 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f4_stack=624(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 624(%rsp),%xmm15,%xmm2

# qhasm: 2x m6 += r2
# asm 1: paddq <r2=reg128#3,<m6=reg128#12
# asm 2: paddq <r2=%xmm2,<m6=%xmm11
paddq %xmm2,%xmm11

# qhasm: 2x r2 = g2 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#41,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f5_stack=640(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 640(%rsp),%xmm15,%xmm2

# qhasm: 2x m7 += r2
# asm 1: paddq <r2=reg128#3,<m7=reg128#13
# asm 2: paddq <r2=%xmm2,<m7=%xmm12
paddq %xmm2,%xmm12

# qhasm: 2x r2 = g2 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f6_stack=688(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 688(%rsp),%xmm15,%xmm2

# qhasm: 2x m8 += r2
# asm 1: paddq <r2=reg128#3,<m8=reg128#14
# asm 2: paddq <r2=%xmm2,<m8=%xmm13
paddq %xmm2,%xmm13

# qhasm: 2x r2 = g2 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#45,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f7_stack=704(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 704(%rsp),%xmm15,%xmm2

# qhasm: 2x m9 += r2
# asm 1: paddq <r2=reg128#3,<m9=reg128#15
# asm 2: paddq <r2=%xmm2,<m9=%xmm14
paddq %xmm2,%xmm14

# qhasm: 2x g2 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g2=reg128#16
# asm 2: pmuludq v19_19,<g2=%xmm15
pmuludq v19_19(%rip),%xmm15

# qhasm: 2x r2 = g2 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g2=reg128#16,>r2=reg128#3
# asm 2: vpmuludq <f8_stack=736(%rsp),<g2=%xmm15,>r2=%xmm2
vpmuludq 736(%rsp),%xmm15,%xmm2

# qhasm: 2x m0 += r2
# asm 1: paddq <r2=reg128#3,<m0=reg128#7
# asm 2: paddq <r2=%xmm2,<m0=%xmm6
paddq %xmm2,%xmm6

# qhasm: 2x g2 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#48,<g2=reg128#16
# asm 2: pmuludq <f9_stack=752(%rsp),<g2=%xmm15
pmuludq 752(%rsp),%xmm15

# qhasm: 2x m1 += g2
# asm 1: paddq <g2=reg128#16,<m1=reg128#6
# asm 2: paddq <g2=%xmm15,<m1=%xmm5
paddq %xmm15,%xmm5

# qhasm: 2x r3 = g3 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f0_stack=480(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 480(%rsp),%xmm1,%xmm2

# qhasm: 2x m3 += r3
# asm 1: paddq <r3=reg128#3,<m3=reg128#9
# asm 2: paddq <r3=%xmm2,<m3=%xmm8
paddq %xmm2,%xmm8

# qhasm: 2x r3 = g3 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#34,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f1_2_stack=528(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 528(%rsp),%xmm1,%xmm2

# qhasm: 2x m4 += r3
# asm 1: paddq <r3=reg128#3,<m4=reg128#10
# asm 2: paddq <r3=%xmm2,<m4=%xmm9
paddq %xmm2,%xmm9

# qhasm: 2x r3 = g3 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f2_stack=560(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 560(%rsp),%xmm1,%xmm2

# qhasm: 2x m5 += r3
# asm 1: paddq <r3=reg128#3,<m5=reg128#11
# asm 2: paddq <r3=%xmm2,<m5=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x r3 = g3 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#38,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f3_2_stack=592(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 592(%rsp),%xmm1,%xmm2

# qhasm: 2x m6 += r3
# asm 1: paddq <r3=reg128#3,<m6=reg128#12
# asm 2: paddq <r3=%xmm2,<m6=%xmm11
paddq %xmm2,%xmm11

# qhasm: 2x r3 = g3 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f4_stack=624(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 624(%rsp),%xmm1,%xmm2

# qhasm: 2x m7 += r3
# asm 1: paddq <r3=reg128#3,<m7=reg128#13
# asm 2: paddq <r3=%xmm2,<m7=%xmm12
paddq %xmm2,%xmm12

# qhasm: 2x r3 = g3 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#42,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f5_2_stack=656(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 656(%rsp),%xmm1,%xmm2

# qhasm: 2x m8 += r3
# asm 1: paddq <r3=reg128#3,<m8=reg128#14
# asm 2: paddq <r3=%xmm2,<m8=%xmm13
paddq %xmm2,%xmm13

# qhasm: 2x r3 = g3 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f6_stack=688(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 688(%rsp),%xmm1,%xmm2

# qhasm: 2x m9 += r3
# asm 1: paddq <r3=reg128#3,<m9=reg128#15
# asm 2: paddq <r3=%xmm2,<m9=%xmm14
paddq %xmm2,%xmm14

# qhasm: 2x g3 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g3=reg128#2
# asm 2: pmuludq v19_19,<g3=%xmm1
pmuludq v19_19(%rip),%xmm1

# qhasm: 2x r3 = g3 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#46,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f7_2_stack=720(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 720(%rsp),%xmm1,%xmm2

# qhasm: 2x m0 += r3
# asm 1: paddq <r3=reg128#3,<m0=reg128#7
# asm 2: paddq <r3=%xmm2,<m0=%xmm6
paddq %xmm2,%xmm6

# qhasm: 2x r3 = g3 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g3=reg128#2,>r3=reg128#3
# asm 2: vpmuludq <f8_stack=736(%rsp),<g3=%xmm1,>r3=%xmm2
vpmuludq 736(%rsp),%xmm1,%xmm2

# qhasm: 2x m1 += r3
# asm 1: paddq <r3=reg128#3,<m1=reg128#6
# asm 2: paddq <r3=%xmm2,<m1=%xmm5
paddq %xmm2,%xmm5

# qhasm: 2x g3 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#49,<g3=reg128#2
# asm 2: pmuludq <f9_2_stack=768(%rsp),<g3=%xmm1
pmuludq 768(%rsp),%xmm1

# qhasm: 2x m2 += g3
# asm 1: paddq <g3=reg128#2,<m2=reg128#8
# asm 2: paddq <g3=%xmm1,<m2=%xmm7
paddq %xmm1,%xmm7

# qhasm: h4 = t0_4
# asm 1: movdqa <t0_4=stack128#39,>h4=reg128#2
# asm 2: movdqa <t0_4=608(%rsp),>h4=%xmm1
movdqa 608(%rsp),%xmm1

# qhasm: 2x h5 += h4
# asm 1: paddq <h4=reg128#2,<h5=reg128#4
# asm 2: paddq <h4=%xmm1,<h5=%xmm3
paddq %xmm1,%xmm3

# qhasm: g4 = unpack_low(h4, h5)
# asm 1: vpunpcklqdq <h5=reg128#4,<h4=reg128#2,>g4=reg128#3
# asm 2: vpunpcklqdq <h5=%xmm3,<h4=%xmm1,>g4=%xmm2
vpunpcklqdq %xmm3,%xmm1,%xmm2

# qhasm: g5 = unpack_high(h4, h5)
# asm 1: vpunpckhqdq <h5=reg128#4,<h4=reg128#2,>g5=reg128#2
# asm 2: vpunpckhqdq <h5=%xmm3,<h4=%xmm1,>g5=%xmm1
vpunpckhqdq %xmm3,%xmm1,%xmm1

# qhasm: 2x r4 = g4 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f0_stack=480(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 480(%rsp),%xmm2,%xmm3

# qhasm: 2x m4 += r4
# asm 1: paddq <r4=reg128#4,<m4=reg128#10
# asm 2: paddq <r4=%xmm3,<m4=%xmm9
paddq %xmm3,%xmm9

# qhasm: 2x r4 = g4 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#33,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f1_stack=512(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 512(%rsp),%xmm2,%xmm3

# qhasm: 2x m5 += r4
# asm 1: paddq <r4=reg128#4,<m5=reg128#11
# asm 2: paddq <r4=%xmm3,<m5=%xmm10
paddq %xmm3,%xmm10

# qhasm: 2x r4 = g4 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f2_stack=560(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 560(%rsp),%xmm2,%xmm3

# qhasm: 2x m6 += r4
# asm 1: paddq <r4=reg128#4,<m6=reg128#12
# asm 2: paddq <r4=%xmm3,<m6=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r4 = g4 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#37,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f3_stack=576(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 576(%rsp),%xmm2,%xmm3

# qhasm: 2x m7 += r4
# asm 1: paddq <r4=reg128#4,<m7=reg128#13
# asm 2: paddq <r4=%xmm3,<m7=%xmm12
paddq %xmm3,%xmm12

# qhasm: 2x r4 = g4 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f4_stack=624(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 624(%rsp),%xmm2,%xmm3

# qhasm: 2x m8 += r4
# asm 1: paddq <r4=reg128#4,<m8=reg128#14
# asm 2: paddq <r4=%xmm3,<m8=%xmm13
paddq %xmm3,%xmm13

# qhasm: 2x r4 = g4 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#41,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f5_stack=640(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 640(%rsp),%xmm2,%xmm3

# qhasm: 2x m9 += r4
# asm 1: paddq <r4=reg128#4,<m9=reg128#15
# asm 2: paddq <r4=%xmm3,<m9=%xmm14
paddq %xmm3,%xmm14

# qhasm: 2x g4 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g4=reg128#3
# asm 2: pmuludq v19_19,<g4=%xmm2
pmuludq v19_19(%rip),%xmm2

# qhasm: 2x r4 = g4 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f6_stack=688(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 688(%rsp),%xmm2,%xmm3

# qhasm: 2x m0 += r4
# asm 1: paddq <r4=reg128#4,<m0=reg128#7
# asm 2: paddq <r4=%xmm3,<m0=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r4 = g4 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#45,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f7_stack=704(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 704(%rsp),%xmm2,%xmm3

# qhasm: 2x m1 += r4
# asm 1: paddq <r4=reg128#4,<m1=reg128#6
# asm 2: paddq <r4=%xmm3,<m1=%xmm5
paddq %xmm3,%xmm5

# qhasm: 2x r4 = g4 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g4=reg128#3,>r4=reg128#4
# asm 2: vpmuludq <f8_stack=736(%rsp),<g4=%xmm2,>r4=%xmm3
vpmuludq 736(%rsp),%xmm2,%xmm3

# qhasm: 2x m2 += r4
# asm 1: paddq <r4=reg128#4,<m2=reg128#8
# asm 2: paddq <r4=%xmm3,<m2=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x g4 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#48,<g4=reg128#3
# asm 2: pmuludq <f9_stack=752(%rsp),<g4=%xmm2
pmuludq 752(%rsp),%xmm2

# qhasm: 2x m3 += g4
# asm 1: paddq <g4=reg128#3,<m3=reg128#9
# asm 2: paddq <g4=%xmm2,<m3=%xmm8
paddq %xmm2,%xmm8

# qhasm: 2x r5 = g5 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f0_stack=480(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 480(%rsp),%xmm1,%xmm2

# qhasm: 2x m5 += r5
# asm 1: paddq <r5=reg128#3,<m5=reg128#11
# asm 2: paddq <r5=%xmm2,<m5=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x r5 = g5 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#34,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f1_2_stack=528(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 528(%rsp),%xmm1,%xmm2

# qhasm: 2x m6 += r5
# asm 1: paddq <r5=reg128#3,<m6=reg128#12
# asm 2: paddq <r5=%xmm2,<m6=%xmm11
paddq %xmm2,%xmm11

# qhasm: 2x r5 = g5 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f2_stack=560(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 560(%rsp),%xmm1,%xmm2

# qhasm: 2x m7 += r5
# asm 1: paddq <r5=reg128#3,<m7=reg128#13
# asm 2: paddq <r5=%xmm2,<m7=%xmm12
paddq %xmm2,%xmm12

# qhasm: 2x r5 = g5 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#38,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f3_2_stack=592(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 592(%rsp),%xmm1,%xmm2

# qhasm: 2x m8 += r5
# asm 1: paddq <r5=reg128#3,<m8=reg128#14
# asm 2: paddq <r5=%xmm2,<m8=%xmm13
paddq %xmm2,%xmm13

# qhasm: 2x r5 = g5 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f4_stack=624(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 624(%rsp),%xmm1,%xmm2

# qhasm: 2x m9 += r5
# asm 1: paddq <r5=reg128#3,<m9=reg128#15
# asm 2: paddq <r5=%xmm2,<m9=%xmm14
paddq %xmm2,%xmm14

# qhasm: 2x g5 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g5=reg128#2
# asm 2: pmuludq v19_19,<g5=%xmm1
pmuludq v19_19(%rip),%xmm1

# qhasm: 2x r5 = g5 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#42,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f5_2_stack=656(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 656(%rsp),%xmm1,%xmm2

# qhasm: 2x m0 += r5
# asm 1: paddq <r5=reg128#3,<m0=reg128#7
# asm 2: paddq <r5=%xmm2,<m0=%xmm6
paddq %xmm2,%xmm6

# qhasm: 2x r5 = g5 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f6_stack=688(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 688(%rsp),%xmm1,%xmm2

# qhasm: 2x m1 += r5
# asm 1: paddq <r5=reg128#3,<m1=reg128#6
# asm 2: paddq <r5=%xmm2,<m1=%xmm5
paddq %xmm2,%xmm5

# qhasm: 2x r5 = g5 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#46,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f7_2_stack=720(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 720(%rsp),%xmm1,%xmm2

# qhasm: 2x m2 += r5
# asm 1: paddq <r5=reg128#3,<m2=reg128#8
# asm 2: paddq <r5=%xmm2,<m2=%xmm7
paddq %xmm2,%xmm7

# qhasm: 2x r5 = g5 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g5=reg128#2,>r5=reg128#3
# asm 2: vpmuludq <f8_stack=736(%rsp),<g5=%xmm1,>r5=%xmm2
vpmuludq 736(%rsp),%xmm1,%xmm2

# qhasm: 2x m3 += r5
# asm 1: paddq <r5=reg128#3,<m3=reg128#9
# asm 2: paddq <r5=%xmm2,<m3=%xmm8
paddq %xmm2,%xmm8

# qhasm: 2x g5 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#49,<g5=reg128#2
# asm 2: pmuludq <f9_2_stack=768(%rsp),<g5=%xmm1
pmuludq 768(%rsp),%xmm1

# qhasm: 2x m4 += g5
# asm 1: paddq <g5=reg128#2,<m4=reg128#10
# asm 2: paddq <g5=%xmm1,<m4=%xmm9
paddq %xmm1,%xmm9

# qhasm: h6 = t0_6
# asm 1: movdqa <t0_6=stack128#43,>h6=reg128#2
# asm 2: movdqa <t0_6=672(%rsp),>h6=%xmm1
movdqa 672(%rsp),%xmm1

# qhasm: 2x h7 += h6
# asm 1: paddq <h6=reg128#2,<h7=reg128#5
# asm 2: paddq <h6=%xmm1,<h7=%xmm4
paddq %xmm1,%xmm4

# qhasm: g6 = unpack_low(h6, h7)
# asm 1: vpunpcklqdq <h7=reg128#5,<h6=reg128#2,>g6=reg128#3
# asm 2: vpunpcklqdq <h7=%xmm4,<h6=%xmm1,>g6=%xmm2
vpunpcklqdq %xmm4,%xmm1,%xmm2

# qhasm: g7 = unpack_high(h6, h7)
# asm 1: vpunpckhqdq <h7=reg128#5,<h6=reg128#2,>g7=reg128#2
# asm 2: vpunpckhqdq <h7=%xmm4,<h6=%xmm1,>g7=%xmm1
vpunpckhqdq %xmm4,%xmm1,%xmm1

# qhasm: 2x r6 = g6 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f0_stack=480(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 480(%rsp),%xmm2,%xmm3

# qhasm: 2x m6 += r6
# asm 1: paddq <r6=reg128#4,<m6=reg128#12
# asm 2: paddq <r6=%xmm3,<m6=%xmm11
paddq %xmm3,%xmm11

# qhasm: 2x r6 = g6 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#33,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f1_stack=512(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 512(%rsp),%xmm2,%xmm3

# qhasm: 2x m7 += r6
# asm 1: paddq <r6=reg128#4,<m7=reg128#13
# asm 2: paddq <r6=%xmm3,<m7=%xmm12
paddq %xmm3,%xmm12

# qhasm: 2x r6 = g6 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f2_stack=560(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 560(%rsp),%xmm2,%xmm3

# qhasm: 2x m8 += r6
# asm 1: paddq <r6=reg128#4,<m8=reg128#14
# asm 2: paddq <r6=%xmm3,<m8=%xmm13
paddq %xmm3,%xmm13

# qhasm: 2x r6 = g6 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#37,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f3_stack=576(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 576(%rsp),%xmm2,%xmm3

# qhasm: 2x m9 += r6
# asm 1: paddq <r6=reg128#4,<m9=reg128#15
# asm 2: paddq <r6=%xmm3,<m9=%xmm14
paddq %xmm3,%xmm14

# qhasm: 2x g6 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g6=reg128#3
# asm 2: pmuludq v19_19,<g6=%xmm2
pmuludq v19_19(%rip),%xmm2

# qhasm: 2x r6 = g6 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f4_stack=624(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 624(%rsp),%xmm2,%xmm3

# qhasm: 2x m0 += r6
# asm 1: paddq <r6=reg128#4,<m0=reg128#7
# asm 2: paddq <r6=%xmm3,<m0=%xmm6
paddq %xmm3,%xmm6

# qhasm: 2x r6 = g6 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#41,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f5_stack=640(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 640(%rsp),%xmm2,%xmm3

# qhasm: 2x m1 += r6
# asm 1: paddq <r6=reg128#4,<m1=reg128#6
# asm 2: paddq <r6=%xmm3,<m1=%xmm5
paddq %xmm3,%xmm5

# qhasm: 2x r6 = g6 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f6_stack=688(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 688(%rsp),%xmm2,%xmm3

# qhasm: 2x m2 += r6
# asm 1: paddq <r6=reg128#4,<m2=reg128#8
# asm 2: paddq <r6=%xmm3,<m2=%xmm7
paddq %xmm3,%xmm7

# qhasm: 2x r6 = g6 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#45,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f7_stack=704(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 704(%rsp),%xmm2,%xmm3

# qhasm: 2x m3 += r6
# asm 1: paddq <r6=reg128#4,<m3=reg128#9
# asm 2: paddq <r6=%xmm3,<m3=%xmm8
paddq %xmm3,%xmm8

# qhasm: 2x r6 = g6 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g6=reg128#3,>r6=reg128#4
# asm 2: vpmuludq <f8_stack=736(%rsp),<g6=%xmm2,>r6=%xmm3
vpmuludq 736(%rsp),%xmm2,%xmm3

# qhasm: 2x m4 += r6
# asm 1: paddq <r6=reg128#4,<m4=reg128#10
# asm 2: paddq <r6=%xmm3,<m4=%xmm9
paddq %xmm3,%xmm9

# qhasm: 2x g6 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#48,<g6=reg128#3
# asm 2: pmuludq <f9_stack=752(%rsp),<g6=%xmm2
pmuludq 752(%rsp),%xmm2

# qhasm: 2x m5 += g6
# asm 1: paddq <g6=reg128#3,<m5=reg128#11
# asm 2: paddq <g6=%xmm2,<m5=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x r7 = g7 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f0_stack=480(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 480(%rsp),%xmm1,%xmm2

# qhasm: 2x m7 += r7
# asm 1: paddq <r7=reg128#3,<m7=reg128#13
# asm 2: paddq <r7=%xmm2,<m7=%xmm12
paddq %xmm2,%xmm12

# qhasm: 2x r7 = g7 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#34,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f1_2_stack=528(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 528(%rsp),%xmm1,%xmm2

# qhasm: 2x m8 += r7
# asm 1: paddq <r7=reg128#3,<m8=reg128#14
# asm 2: paddq <r7=%xmm2,<m8=%xmm13
paddq %xmm2,%xmm13

# qhasm: 2x r7 = g7 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f2_stack=560(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 560(%rsp),%xmm1,%xmm2

# qhasm: 2x m9 += r7
# asm 1: paddq <r7=reg128#3,<m9=reg128#15
# asm 2: paddq <r7=%xmm2,<m9=%xmm14
paddq %xmm2,%xmm14

# qhasm: 2x g7 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g7=reg128#2
# asm 2: pmuludq v19_19,<g7=%xmm1
pmuludq v19_19(%rip),%xmm1

# qhasm: 2x r7 = g7 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#38,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f3_2_stack=592(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 592(%rsp),%xmm1,%xmm2

# qhasm: 2x m0 += r7
# asm 1: paddq <r7=reg128#3,<m0=reg128#7
# asm 2: paddq <r7=%xmm2,<m0=%xmm6
paddq %xmm2,%xmm6

# qhasm: 2x r7 = g7 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f4_stack=624(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 624(%rsp),%xmm1,%xmm2

# qhasm: 2x m1 += r7
# asm 1: paddq <r7=reg128#3,<m1=reg128#6
# asm 2: paddq <r7=%xmm2,<m1=%xmm5
paddq %xmm2,%xmm5

# qhasm: 2x r7 = g7 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#42,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f5_2_stack=656(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 656(%rsp),%xmm1,%xmm2

# qhasm: 2x m2 += r7
# asm 1: paddq <r7=reg128#3,<m2=reg128#8
# asm 2: paddq <r7=%xmm2,<m2=%xmm7
paddq %xmm2,%xmm7

# qhasm: 2x r7 = g7 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f6_stack=688(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 688(%rsp),%xmm1,%xmm2

# qhasm: 2x m3 += r7
# asm 1: paddq <r7=reg128#3,<m3=reg128#9
# asm 2: paddq <r7=%xmm2,<m3=%xmm8
paddq %xmm2,%xmm8

# qhasm: 2x r7 = g7 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#46,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f7_2_stack=720(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 720(%rsp),%xmm1,%xmm2

# qhasm: 2x m4 += r7
# asm 1: paddq <r7=reg128#3,<m4=reg128#10
# asm 2: paddq <r7=%xmm2,<m4=%xmm9
paddq %xmm2,%xmm9

# qhasm: 2x r7 = g7 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g7=reg128#2,>r7=reg128#3
# asm 2: vpmuludq <f8_stack=736(%rsp),<g7=%xmm1,>r7=%xmm2
vpmuludq 736(%rsp),%xmm1,%xmm2

# qhasm: 2x m5 += r7
# asm 1: paddq <r7=reg128#3,<m5=reg128#11
# asm 2: paddq <r7=%xmm2,<m5=%xmm10
paddq %xmm2,%xmm10

# qhasm: 2x g7 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#49,<g7=reg128#2
# asm 2: pmuludq <f9_2_stack=768(%rsp),<g7=%xmm1
pmuludq 768(%rsp),%xmm1

# qhasm: 2x m6 += g7
# asm 1: paddq <g7=reg128#2,<m6=reg128#12
# asm 2: paddq <g7=%xmm1,<m6=%xmm11
paddq %xmm1,%xmm11

# qhasm: h8 = t0_8
# asm 1: movdqa <t0_8=stack128#29,>h8=reg128#2
# asm 2: movdqa <t0_8=448(%rsp),>h8=%xmm1
movdqa 448(%rsp),%xmm1

# qhasm: 2x h9 += h8
# asm 1: paddq <h8=reg128#2,<h9=reg128#1
# asm 2: paddq <h8=%xmm1,<h9=%xmm0
paddq %xmm1,%xmm0

# qhasm: g8 = unpack_low(h8, h9)
# asm 1: vpunpcklqdq <h9=reg128#1,<h8=reg128#2,>g8=reg128#3
# asm 2: vpunpcklqdq <h9=%xmm0,<h8=%xmm1,>g8=%xmm2
vpunpcklqdq %xmm0,%xmm1,%xmm2

# qhasm: g9 = unpack_high(h8, h9)
# asm 1: vpunpckhqdq <h9=reg128#1,<h8=reg128#2,>g9=reg128#1
# asm 2: vpunpckhqdq <h9=%xmm0,<h8=%xmm1,>g9=%xmm0
vpunpckhqdq %xmm0,%xmm1,%xmm0

# qhasm: 2x r8 = g8 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f0_stack=480(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 480(%rsp),%xmm2,%xmm1

# qhasm: 2x m8 += r8
# asm 1: paddq <r8=reg128#2,<m8=reg128#14
# asm 2: paddq <r8=%xmm1,<m8=%xmm13
paddq %xmm1,%xmm13

# qhasm: 2x r8 = g8 * f1_stack
# asm 1: vpmuludq <f1_stack=stack128#33,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f1_stack=512(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 512(%rsp),%xmm2,%xmm1

# qhasm: 2x m9 += r8
# asm 1: paddq <r8=reg128#2,<m9=reg128#15
# asm 2: paddq <r8=%xmm1,<m9=%xmm14
paddq %xmm1,%xmm14

# qhasm: 2x g8 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g8=reg128#3
# asm 2: pmuludq v19_19,<g8=%xmm2
pmuludq v19_19(%rip),%xmm2

# qhasm: 2x r8 = g8 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f2_stack=560(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 560(%rsp),%xmm2,%xmm1

# qhasm: 2x m0 += r8
# asm 1: paddq <r8=reg128#2,<m0=reg128#7
# asm 2: paddq <r8=%xmm1,<m0=%xmm6
paddq %xmm1,%xmm6

# qhasm: 2x r8 = g8 * f3_stack
# asm 1: vpmuludq <f3_stack=stack128#37,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f3_stack=576(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 576(%rsp),%xmm2,%xmm1

# qhasm: 2x m1 += r8
# asm 1: paddq <r8=reg128#2,<m1=reg128#6
# asm 2: paddq <r8=%xmm1,<m1=%xmm5
paddq %xmm1,%xmm5

# qhasm: 2x r8 = g8 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f4_stack=624(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 624(%rsp),%xmm2,%xmm1

# qhasm: 2x m2 += r8
# asm 1: paddq <r8=reg128#2,<m2=reg128#8
# asm 2: paddq <r8=%xmm1,<m2=%xmm7
paddq %xmm1,%xmm7

# qhasm: 2x r8 = g8 * f5_stack
# asm 1: vpmuludq <f5_stack=stack128#41,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f5_stack=640(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 640(%rsp),%xmm2,%xmm1

# qhasm: 2x m3 += r8
# asm 1: paddq <r8=reg128#2,<m3=reg128#9
# asm 2: paddq <r8=%xmm1,<m3=%xmm8
paddq %xmm1,%xmm8

# qhasm: 2x r8 = g8 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f6_stack=688(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 688(%rsp),%xmm2,%xmm1

# qhasm: 2x m4 += r8
# asm 1: paddq <r8=reg128#2,<m4=reg128#10
# asm 2: paddq <r8=%xmm1,<m4=%xmm9
paddq %xmm1,%xmm9

# qhasm: 2x r8 = g8 * f7_stack
# asm 1: vpmuludq <f7_stack=stack128#45,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f7_stack=704(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 704(%rsp),%xmm2,%xmm1

# qhasm: 2x m5 += r8
# asm 1: paddq <r8=reg128#2,<m5=reg128#11
# asm 2: paddq <r8=%xmm1,<m5=%xmm10
paddq %xmm1,%xmm10

# qhasm: 2x r8 = g8 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g8=reg128#3,>r8=reg128#2
# asm 2: vpmuludq <f8_stack=736(%rsp),<g8=%xmm2,>r8=%xmm1
vpmuludq 736(%rsp),%xmm2,%xmm1

# qhasm: 2x m6 += r8
# asm 1: paddq <r8=reg128#2,<m6=reg128#12
# asm 2: paddq <r8=%xmm1,<m6=%xmm11
paddq %xmm1,%xmm11

# qhasm: 2x g8 *= f9_stack
# asm 1: pmuludq <f9_stack=stack128#48,<g8=reg128#3
# asm 2: pmuludq <f9_stack=752(%rsp),<g8=%xmm2
pmuludq 752(%rsp),%xmm2

# qhasm: 2x m7 += g8
# asm 1: paddq <g8=reg128#3,<m7=reg128#13
# asm 2: paddq <g8=%xmm2,<m7=%xmm12
paddq %xmm2,%xmm12

# qhasm: 2x r9 = g9 * f0_stack
# asm 1: vpmuludq <f0_stack=stack128#31,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f0_stack=480(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 480(%rsp),%xmm0,%xmm1

# qhasm: 2x m9 += r9
# asm 1: paddq <r9=reg128#2,<m9=reg128#15
# asm 2: paddq <r9=%xmm1,<m9=%xmm14
paddq %xmm1,%xmm14

# qhasm: 2x g9 *= mem128[ v19_19 ]
# asm 1: pmuludq v19_19,<g9=reg128#1
# asm 2: pmuludq v19_19,<g9=%xmm0
pmuludq v19_19(%rip),%xmm0

# qhasm: 2x r9 = g9 * f1_2_stack
# asm 1: vpmuludq <f1_2_stack=stack128#34,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f1_2_stack=528(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 528(%rsp),%xmm0,%xmm1

# qhasm: 2x m0 += r9
# asm 1: paddq <r9=reg128#2,<m0=reg128#7
# asm 2: paddq <r9=%xmm1,<m0=%xmm6
paddq %xmm1,%xmm6

# qhasm: 2x r9 = g9 * f2_stack
# asm 1: vpmuludq <f2_stack=stack128#36,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f2_stack=560(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 560(%rsp),%xmm0,%xmm1

# qhasm: 2x m1 += r9
# asm 1: paddq <r9=reg128#2,<m1=reg128#6
# asm 2: paddq <r9=%xmm1,<m1=%xmm5
paddq %xmm1,%xmm5

# qhasm: 2x r9 = g9 * f3_2_stack
# asm 1: vpmuludq <f3_2_stack=stack128#38,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f3_2_stack=592(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 592(%rsp),%xmm0,%xmm1

# qhasm: 2x m2 += r9
# asm 1: paddq <r9=reg128#2,<m2=reg128#8
# asm 2: paddq <r9=%xmm1,<m2=%xmm7
paddq %xmm1,%xmm7

# qhasm: 2x r9 = g9 * f4_stack
# asm 1: vpmuludq <f4_stack=stack128#40,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f4_stack=624(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 624(%rsp),%xmm0,%xmm1

# qhasm: 2x m3 += r9
# asm 1: paddq <r9=reg128#2,<m3=reg128#9
# asm 2: paddq <r9=%xmm1,<m3=%xmm8
paddq %xmm1,%xmm8

# qhasm: 2x r9 = g9 * f5_2_stack
# asm 1: vpmuludq <f5_2_stack=stack128#42,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f5_2_stack=656(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 656(%rsp),%xmm0,%xmm1

# qhasm: 2x m4 += r9
# asm 1: paddq <r9=reg128#2,<m4=reg128#10
# asm 2: paddq <r9=%xmm1,<m4=%xmm9
paddq %xmm1,%xmm9

# qhasm: 2x r9 = g9 * f6_stack
# asm 1: vpmuludq <f6_stack=stack128#44,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f6_stack=688(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 688(%rsp),%xmm0,%xmm1

# qhasm: 2x m5 += r9
# asm 1: paddq <r9=reg128#2,<m5=reg128#11
# asm 2: paddq <r9=%xmm1,<m5=%xmm10
paddq %xmm1,%xmm10

# qhasm: 2x r9 = g9 * f7_2_stack
# asm 1: vpmuludq <f7_2_stack=stack128#46,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f7_2_stack=720(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 720(%rsp),%xmm0,%xmm1

# qhasm: 2x m6 += r9
# asm 1: paddq <r9=reg128#2,<m6=reg128#12
# asm 2: paddq <r9=%xmm1,<m6=%xmm11
paddq %xmm1,%xmm11

# qhasm: 2x r9 = g9 * f8_stack
# asm 1: vpmuludq <f8_stack=stack128#47,<g9=reg128#1,>r9=reg128#2
# asm 2: vpmuludq <f8_stack=736(%rsp),<g9=%xmm0,>r9=%xmm1
vpmuludq 736(%rsp),%xmm0,%xmm1

# qhasm: 2x m7 += r9
# asm 1: paddq <r9=reg128#2,<m7=reg128#13
# asm 2: paddq <r9=%xmm1,<m7=%xmm12
paddq %xmm1,%xmm12

# qhasm: 2x g9 *= f9_2_stack
# asm 1: pmuludq <f9_2_stack=stack128#49,<g9=reg128#1
# asm 2: pmuludq <f9_2_stack=768(%rsp),<g9=%xmm0
pmuludq 768(%rsp),%xmm0

# qhasm: 2x m8 += g9
# asm 1: paddq <g9=reg128#1,<m8=reg128#14
# asm 2: paddq <g9=%xmm0,<m8=%xmm13
paddq %xmm0,%xmm13

# qhasm: 2x carry0 = m0 unsigned>>= 26
# asm 1: vpsrlq $26,<m0=reg128#7,>carry0=reg128#1
# asm 2: vpsrlq $26,<m0=%xmm6,>carry0=%xmm0
vpsrlq $26,%xmm6,%xmm0

# qhasm: 2x m1 += carry0
# asm 1: paddq <carry0=reg128#1,<m1=reg128#6
# asm 2: paddq <carry0=%xmm0,<m1=%xmm5
paddq %xmm0,%xmm5

# qhasm: m0 &= mem128[ m26 ]
# asm 1: pand m26,<m0=reg128#7
# asm 2: pand m26,<m0=%xmm6
pand m26(%rip),%xmm6

# qhasm: 2x carry5 = m5 unsigned>>= 25
# asm 1: vpsrlq $25,<m5=reg128#11,>carry5=reg128#1
# asm 2: vpsrlq $25,<m5=%xmm10,>carry5=%xmm0
vpsrlq $25,%xmm10,%xmm0

# qhasm: 2x m6 += carry5
# asm 1: paddq <carry5=reg128#1,<m6=reg128#12
# asm 2: paddq <carry5=%xmm0,<m6=%xmm11
paddq %xmm0,%xmm11

# qhasm: m5 &= mem128[ m25 ]
# asm 1: pand m25,<m5=reg128#11
# asm 2: pand m25,<m5=%xmm10
pand m25(%rip),%xmm10

# qhasm: 2x carry1 = m1 unsigned>>= 25
# asm 1: vpsrlq $25,<m1=reg128#6,>carry1=reg128#1
# asm 2: vpsrlq $25,<m1=%xmm5,>carry1=%xmm0
vpsrlq $25,%xmm5,%xmm0

# qhasm: 2x m2 += carry1
# asm 1: paddq <carry1=reg128#1,<m2=reg128#8
# asm 2: paddq <carry1=%xmm0,<m2=%xmm7
paddq %xmm0,%xmm7

# qhasm: m1 &= mem128[ m25 ]
# asm 1: pand m25,<m1=reg128#6
# asm 2: pand m25,<m1=%xmm5
pand m25(%rip),%xmm5

# qhasm: 2x carry6 = m6 unsigned>>= 26
# asm 1: vpsrlq $26,<m6=reg128#12,>carry6=reg128#1
# asm 2: vpsrlq $26,<m6=%xmm11,>carry6=%xmm0
vpsrlq $26,%xmm11,%xmm0

# qhasm: 2x m7 += carry6
# asm 1: paddq <carry6=reg128#1,<m7=reg128#13
# asm 2: paddq <carry6=%xmm0,<m7=%xmm12
paddq %xmm0,%xmm12

# qhasm: m6 &= mem128[ m26 ]
# asm 1: pand m26,<m6=reg128#12
# asm 2: pand m26,<m6=%xmm11
pand m26(%rip),%xmm11

# qhasm: 2x carry2 = m2 unsigned>>= 26
# asm 1: vpsrlq $26,<m2=reg128#8,>carry2=reg128#1
# asm 2: vpsrlq $26,<m2=%xmm7,>carry2=%xmm0
vpsrlq $26,%xmm7,%xmm0

# qhasm: 2x m3 += carry2
# asm 1: paddq <carry2=reg128#1,<m3=reg128#9
# asm 2: paddq <carry2=%xmm0,<m3=%xmm8
paddq %xmm0,%xmm8

# qhasm: m2 &= mem128[ m26 ]
# asm 1: pand m26,<m2=reg128#8
# asm 2: pand m26,<m2=%xmm7
pand m26(%rip),%xmm7

# qhasm: 2x carry7 = m7 unsigned>>= 25
# asm 1: vpsrlq $25,<m7=reg128#13,>carry7=reg128#1
# asm 2: vpsrlq $25,<m7=%xmm12,>carry7=%xmm0
vpsrlq $25,%xmm12,%xmm0

# qhasm: 2x m8 += carry7
# asm 1: paddq <carry7=reg128#1,<m8=reg128#14
# asm 2: paddq <carry7=%xmm0,<m8=%xmm13
paddq %xmm0,%xmm13

# qhasm: m7 &= mem128[ m25 ]
# asm 1: pand m25,<m7=reg128#13
# asm 2: pand m25,<m7=%xmm12
pand m25(%rip),%xmm12

# qhasm: 2x carry3 = m3 unsigned>>= 25
# asm 1: vpsrlq $25,<m3=reg128#9,>carry3=reg128#1
# asm 2: vpsrlq $25,<m3=%xmm8,>carry3=%xmm0
vpsrlq $25,%xmm8,%xmm0

# qhasm: 2x m4 += carry3
# asm 1: paddq <carry3=reg128#1,<m4=reg128#10
# asm 2: paddq <carry3=%xmm0,<m4=%xmm9
paddq %xmm0,%xmm9

# qhasm: m3 &= mem128[ m25 ]
# asm 1: pand m25,<m3=reg128#9
# asm 2: pand m25,<m3=%xmm8
pand m25(%rip),%xmm8

# qhasm: 2x carry8 = m8 unsigned>>= 26
# asm 1: vpsrlq $26,<m8=reg128#14,>carry8=reg128#1
# asm 2: vpsrlq $26,<m8=%xmm13,>carry8=%xmm0
vpsrlq $26,%xmm13,%xmm0

# qhasm: 2x m9 += carry8
# asm 1: paddq <carry8=reg128#1,<m9=reg128#15
# asm 2: paddq <carry8=%xmm0,<m9=%xmm14
paddq %xmm0,%xmm14

# qhasm: m8 &= mem128[ m26 ]
# asm 1: pand m26,<m8=reg128#14
# asm 2: pand m26,<m8=%xmm13
pand m26(%rip),%xmm13

# qhasm: 2x carry4 = m4 unsigned>>= 26
# asm 1: vpsrlq $26,<m4=reg128#10,>carry4=reg128#1
# asm 2: vpsrlq $26,<m4=%xmm9,>carry4=%xmm0
vpsrlq $26,%xmm9,%xmm0

# qhasm: 2x m5 += carry4
# asm 1: paddq <carry4=reg128#1,<m5=reg128#11
# asm 2: paddq <carry4=%xmm0,<m5=%xmm10
paddq %xmm0,%xmm10

# qhasm: m4 &= mem128[ m26 ]
# asm 1: pand m26,<m4=reg128#10
# asm 2: pand m26,<m4=%xmm9
pand m26(%rip),%xmm9

# qhasm: 2x carry9 = m9 unsigned>>= 25
# asm 1: vpsrlq $25,<m9=reg128#15,>carry9=reg128#1
# asm 2: vpsrlq $25,<m9=%xmm14,>carry9=%xmm0
vpsrlq $25,%xmm14,%xmm0

# qhasm: 2x r0 = carry9 << 4
# asm 1: vpsllq $4,<carry9=reg128#1,>r0=reg128#2
# asm 2: vpsllq $4,<carry9=%xmm0,>r0=%xmm1
vpsllq $4,%xmm0,%xmm1

# qhasm: 2x m0 += carry9
# asm 1: paddq <carry9=reg128#1,<m0=reg128#7
# asm 2: paddq <carry9=%xmm0,<m0=%xmm6
paddq %xmm0,%xmm6

# qhasm: 2x carry9 <<= 1
# asm 1: psllq $1,<carry9=reg128#1
# asm 2: psllq $1,<carry9=%xmm0
psllq $1,%xmm0

# qhasm: 2x r0 += carry9
# asm 1: paddq <carry9=reg128#1,<r0=reg128#2
# asm 2: paddq <carry9=%xmm0,<r0=%xmm1
paddq %xmm0,%xmm1

# qhasm: 2x m0 += r0
# asm 1: paddq <r0=reg128#2,<m0=reg128#7
# asm 2: paddq <r0=%xmm1,<m0=%xmm6
paddq %xmm1,%xmm6

# qhasm: m9 &= mem128[ m25 ]
# asm 1: pand m25,<m9=reg128#15
# asm 2: pand m25,<m9=%xmm14
pand m25(%rip),%xmm14

# qhasm: 2x carry5 = m5 unsigned>>= 25
# asm 1: vpsrlq $25,<m5=reg128#11,>carry5=reg128#1
# asm 2: vpsrlq $25,<m5=%xmm10,>carry5=%xmm0
vpsrlq $25,%xmm10,%xmm0

# qhasm: 2x m6 += carry5
# asm 1: paddq <carry5=reg128#1,<m6=reg128#12
# asm 2: paddq <carry5=%xmm0,<m6=%xmm11
paddq %xmm0,%xmm11

# qhasm: m5 &= mem128[ m25 ]
# asm 1: pand m25,<m5=reg128#11
# asm 2: pand m25,<m5=%xmm10
pand m25(%rip),%xmm10

# qhasm: 2x carry0 = m0 unsigned>>= 26
# asm 1: vpsrlq $26,<m0=reg128#7,>carry0=reg128#1
# asm 2: vpsrlq $26,<m0=%xmm6,>carry0=%xmm0
vpsrlq $26,%xmm6,%xmm0

# qhasm: 2x m1 += carry0
# asm 1: paddq <carry0=reg128#1,<m1=reg128#6
# asm 2: paddq <carry0=%xmm0,<m1=%xmm5
paddq %xmm0,%xmm5

# qhasm: m0 &= mem128[ m26 ]
# asm 1: pand m26,<m0=reg128#7
# asm 2: pand m26,<m0=%xmm6
pand m26(%rip),%xmm6

# qhasm: f1 = unpack_high( m0, m1 )
# asm 1: vpunpckhqdq <m1=reg128#6,<m0=reg128#7,>f1=reg128#2
# asm 2: vpunpckhqdq <m1=%xmm5,<m0=%xmm6,>f1=%xmm1
vpunpckhqdq %xmm5,%xmm6,%xmm1

# qhasm: f0 = unpack_low( m0, m1 )
# asm 1: vpunpcklqdq <m1=reg128#6,<m0=reg128#7,>f0=reg128#1
# asm 2: vpunpcklqdq <m1=%xmm5,<m0=%xmm6,>f0=%xmm0
vpunpcklqdq %xmm5,%xmm6,%xmm0

# qhasm: f3 = unpack_high( m2, m3 )
# asm 1: vpunpckhqdq <m3=reg128#9,<m2=reg128#8,>f3=reg128#4
# asm 2: vpunpckhqdq <m3=%xmm8,<m2=%xmm7,>f3=%xmm3
vpunpckhqdq %xmm8,%xmm7,%xmm3

# qhasm: f2 = unpack_low( m2, m3 )
# asm 1: vpunpcklqdq <m3=reg128#9,<m2=reg128#8,>f2=reg128#3
# asm 2: vpunpcklqdq <m3=%xmm8,<m2=%xmm7,>f2=%xmm2
vpunpcklqdq %xmm8,%xmm7,%xmm2

# qhasm: f5 = unpack_high( m4, m5 )
# asm 1: vpunpckhqdq <m5=reg128#11,<m4=reg128#10,>f5=reg128#6
# asm 2: vpunpckhqdq <m5=%xmm10,<m4=%xmm9,>f5=%xmm5
vpunpckhqdq %xmm10,%xmm9,%xmm5

# qhasm: f4 = unpack_low( m4, m5 )
# asm 1: vpunpcklqdq <m5=reg128#11,<m4=reg128#10,>f4=reg128#5
# asm 2: vpunpcklqdq <m5=%xmm10,<m4=%xmm9,>f4=%xmm4
vpunpcklqdq %xmm10,%xmm9,%xmm4

# qhasm: f7 = unpack_high( m6, m7 )
# asm 1: vpunpckhqdq <m7=reg128#13,<m6=reg128#12,>f7=reg128#8
# asm 2: vpunpckhqdq <m7=%xmm12,<m6=%xmm11,>f7=%xmm7
vpunpckhqdq %xmm12,%xmm11,%xmm7

# qhasm: f6 = unpack_low( m6, m7 )
# asm 1: vpunpcklqdq <m7=reg128#13,<m6=reg128#12,>f6=reg128#7
# asm 2: vpunpcklqdq <m7=%xmm12,<m6=%xmm11,>f6=%xmm6
vpunpcklqdq %xmm12,%xmm11,%xmm6

# qhasm: f9 = unpack_high( m8, m9 )
# asm 1: vpunpckhqdq <m9=reg128#15,<m8=reg128#14,>f9=reg128#10
# asm 2: vpunpckhqdq <m9=%xmm14,<m8=%xmm13,>f9=%xmm9
vpunpckhqdq %xmm14,%xmm13,%xmm9

# qhasm: f8 = unpack_low( m8, m9 )
# asm 1: vpunpcklqdq <m9=reg128#15,<m8=reg128#14,>f8=reg128#9
# asm 2: vpunpcklqdq <m9=%xmm14,<m8=%xmm13,>f8=%xmm8
vpunpcklqdq %xmm14,%xmm13,%xmm8

# qhasm: =? pos - 0
# asm 1: cmp  $0,<pos=int64#3
# asm 2: cmp  $0,<pos=%rdx
cmp  $0,%rdx
# comment:fp stack unchanged by jump

# qhasm: goto loop if !=
jne ._loop

# qhasm: mem128[input_0 + 160] = f1
# asm 1: movdqu   <f1=reg128#2,160(<input_0=int64#1)
# asm 2: movdqu   <f1=%xmm1,160(<input_0=%rdi)
movdqu   %xmm1,160(%rdi)

# qhasm: mem128[input_0 + 80] = f0
# asm 1: movdqu   <f0=reg128#1,80(<input_0=int64#1)
# asm 2: movdqu   <f0=%xmm0,80(<input_0=%rdi)
movdqu   %xmm0,80(%rdi)

# qhasm: mem128[input_0 + 176] = f3
# asm 1: movdqu   <f3=reg128#4,176(<input_0=int64#1)
# asm 2: movdqu   <f3=%xmm3,176(<input_0=%rdi)
movdqu   %xmm3,176(%rdi)

# qhasm: mem128[input_0 + 96] = f2
# asm 1: movdqu   <f2=reg128#3,96(<input_0=int64#1)
# asm 2: movdqu   <f2=%xmm2,96(<input_0=%rdi)
movdqu   %xmm2,96(%rdi)

# qhasm: mem128[input_0 + 192] = f5
# asm 1: movdqu   <f5=reg128#6,192(<input_0=int64#1)
# asm 2: movdqu   <f5=%xmm5,192(<input_0=%rdi)
movdqu   %xmm5,192(%rdi)

# qhasm: mem128[input_0 + 112] = f4
# asm 1: movdqu   <f4=reg128#5,112(<input_0=int64#1)
# asm 2: movdqu   <f4=%xmm4,112(<input_0=%rdi)
movdqu   %xmm4,112(%rdi)

# qhasm: mem128[input_0 + 208] = f7
# asm 1: movdqu   <f7=reg128#8,208(<input_0=int64#1)
# asm 2: movdqu   <f7=%xmm7,208(<input_0=%rdi)
movdqu   %xmm7,208(%rdi)

# qhasm: mem128[input_0 + 128] = f6
# asm 1: movdqu   <f6=reg128#7,128(<input_0=int64#1)
# asm 2: movdqu   <f6=%xmm6,128(<input_0=%rdi)
movdqu   %xmm6,128(%rdi)

# qhasm: mem128[input_0 + 224] = f9
# asm 1: movdqu   <f9=reg128#10,224(<input_0=int64#1)
# asm 2: movdqu   <f9=%xmm9,224(<input_0=%rdi)
movdqu   %xmm9,224(%rdi)

# qhasm: mem128[input_0 + 144] = f8
# asm 1: movdqu   <f8=reg128#9,144(<input_0=int64#1)
# asm 2: movdqu   <f8=%xmm8,144(<input_0=%rdi)
movdqu   %xmm8,144(%rdi)

# qhasm: caller_r11 = r11_stack
# asm 1: movq <r11_stack=stack64#1,>caller_r11=int64#9
# asm 2: movq <r11_stack=1824(%rsp),>caller_r11=%r11
movq 1824(%rsp),%r11

# qhasm: caller_r12 = r12_stack
# asm 1: movq <r12_stack=stack64#2,>caller_r12=int64#10
# asm 2: movq <r12_stack=1832(%rsp),>caller_r12=%r12
movq 1832(%rsp),%r12

# qhasm: caller_r13 = r13_stack
# asm 1: movq <r13_stack=stack64#3,>caller_r13=int64#11
# asm 2: movq <r13_stack=1840(%rsp),>caller_r13=%r13
movq 1840(%rsp),%r13

# qhasm: caller_r14 = r14_stack
# asm 1: movq <r14_stack=stack64#4,>caller_r14=int64#12
# asm 2: movq <r14_stack=1848(%rsp),>caller_r14=%r14
movq 1848(%rsp),%r14

# qhasm: return
add %r11,%rsp
ret
