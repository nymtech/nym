// linker define fe51_nsquare
// linker use REDMASK51

/*
   This file is adapted from amd64-51/fe25519_square.s:
   Adding loop to perform n squares.
*/

#include "fe51_namespace.h"
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

# qhasm: int64 r0

# qhasm: int64 r1

# qhasm: int64 r2

# qhasm: int64 r3

# qhasm: int64 r4

# qhasm: int64 x0

# qhasm: int64 x1

# qhasm: int64 x2

# qhasm: int64 x3

# qhasm: int64 x4

# qhasm: stack64 x119_stack

# qhasm: stack64 x219_stack

# qhasm: stack64 x319_stack

# qhasm: stack64 x419_stack

# qhasm: int64 squarer01

# qhasm: int64 squarer11

# qhasm: int64 squarer21

# qhasm: int64 squarer31

# qhasm: int64 squarer41

# qhasm: int64 squarerax

# qhasm: int64 squarerdx

# qhasm: int64 squaret

# qhasm: int64 squareredmask

# qhasm: int64 iters

# qhasm: stack64 r11_stack

# qhasm: stack64 r12_stack

# qhasm: stack64 r13_stack

# qhasm: stack64 r14_stack

# qhasm: stack64 r15_stack

# qhasm: stack64 rbx_stack

# qhasm: stack64 rbp_stack

# qhasm: enter fe51_nsquare
.p2align 5
.global _fe51_nsquare
.global fe51_nsquare
_fe51_nsquare:
fe51_nsquare:
mov %rsp,%r11
and $31,%r11
add $64,%r11
sub %r11,%rsp

# qhasm: r11_stack = caller_r11
# asm 1: movq <caller_r11=int64#9,>r11_stack=stack64#1
# asm 2: movq <caller_r11=%r11,>r11_stack=0(%rsp)
movq %r11,0(%rsp)

# qhasm: r12_stack = caller_r12
# asm 1: movq <caller_r12=int64#10,>r12_stack=stack64#2
# asm 2: movq <caller_r12=%r12,>r12_stack=8(%rsp)
movq %r12,8(%rsp)

# qhasm: r13_stack = caller_r13
# asm 1: movq <caller_r13=int64#11,>r13_stack=stack64#3
# asm 2: movq <caller_r13=%r13,>r13_stack=16(%rsp)
movq %r13,16(%rsp)

# qhasm: r14_stack = caller_r14
# asm 1: movq <caller_r14=int64#12,>r14_stack=stack64#4
# asm 2: movq <caller_r14=%r14,>r14_stack=24(%rsp)
movq %r14,24(%rsp)

# qhasm: r15_stack = caller_r15
# asm 1: movq <caller_r15=int64#13,>r15_stack=stack64#5
# asm 2: movq <caller_r15=%r15,>r15_stack=32(%rsp)
movq %r15,32(%rsp)

# qhasm: rbx_stack = caller_rbx
# asm 1: movq <caller_rbx=int64#14,>rbx_stack=stack64#6
# asm 2: movq <caller_rbx=%rbx,>rbx_stack=40(%rsp)
movq %rbx,40(%rsp)

# qhasm: rbp_stack = caller_rbp
# asm 1: movq <caller_rbp=int64#15,>rbp_stack=stack64#7
# asm 2: movq <caller_rbp=%rbp,>rbp_stack=48(%rsp)
movq %rbp,48(%rsp)

# qhasm: x0 = *(uint64 *)(input_1 + 0) 
# asm 1: movq   0(<input_1=int64#2),>x0=int64#4
# asm 2: movq   0(<input_1=%rsi),>x0=%rcx
movq   0(%rsi),%rcx

# qhasm: x1 = *(uint64 *)(input_1 + 8) 
# asm 1: movq   8(<input_1=int64#2),>x1=int64#5
# asm 2: movq   8(<input_1=%rsi),>x1=%r8
movq   8(%rsi),%r8

# qhasm: x2 = *(uint64 *)(input_1 + 16) 
# asm 1: movq   16(<input_1=int64#2),>x2=int64#6
# asm 2: movq   16(<input_1=%rsi),>x2=%r9
movq   16(%rsi),%r9

# qhasm: x3 = *(uint64 *)(input_1 + 24) 
# asm 1: movq   24(<input_1=int64#2),>x3=int64#7
# asm 2: movq   24(<input_1=%rsi),>x3=%rax
movq   24(%rsi),%rax

# qhasm: x4 = *(uint64 *)(input_1 + 32) 
# asm 1: movq   32(<input_1=int64#2),>x4=int64#2
# asm 2: movq   32(<input_1=%rsi),>x4=%rsi
movq   32(%rsi),%rsi

# qhasm: *(uint64 *)(input_0 + 16) = x2 
# asm 1: movq   <x2=int64#6,16(<input_0=int64#1)
# asm 2: movq   <x2=%r9,16(<input_0=%rdi)
movq   %r9,16(%rdi)

# qhasm: *(uint64 *)(input_0 + 24) = x3
# asm 1: movq   <x3=int64#7,24(<input_0=int64#1)
# asm 2: movq   <x3=%rax,24(<input_0=%rdi)
movq   %rax,24(%rdi)

# qhasm: *(uint64 *)(input_0 + 32) = x4
# asm 1: movq   <x4=int64#2,32(<input_0=int64#1)
# asm 2: movq   <x4=%rsi,32(<input_0=%rdi)
movq   %rsi,32(%rdi)

# qhasm: iters = input_2
# asm 1: mov  <input_2=int64#3,>iters=int64#2
# asm 2: mov  <input_2=%rdx,>iters=%rsi
mov  %rdx,%rsi

# qhasm: loop:
._loop:

# qhasm:   iters -= 1
# asm 1: sub  $1,<iters=int64#2
# asm 2: sub  $1,<iters=%rsi
sub  $1,%rsi

# qhasm:   squarerax = x0
# asm 1: mov  <x0=int64#4,>squarerax=int64#7
# asm 2: mov  <x0=%rcx,>squarerax=%rax
mov  %rcx,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * x0
# asm 1: mul  <x0=int64#4
# asm 2: mul  <x0=%rcx
mul  %rcx

# qhasm:   x0 += x0
# asm 1: add  <x0=int64#4,<x0=int64#4
# asm 2: add  <x0=%rcx,<x0=%rcx
add  %rcx,%rcx

# qhasm:   r0 = squarerax
# asm 1: mov  <squarerax=int64#7,>r0=int64#6
# asm 2: mov  <squarerax=%rax,>r0=%r9
mov  %rax,%r9

# qhasm:   squarer01 = squarerdx
# asm 1: mov  <squarerdx=int64#3,>squarer01=int64#8
# asm 2: mov  <squarerdx=%rdx,>squarer01=%r10
mov  %rdx,%r10

# qhasm:   squarerax = x0
# asm 1: mov  <x0=int64#4,>squarerax=int64#7
# asm 2: mov  <x0=%rcx,>squarerax=%rax
mov  %rcx,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * x1
# asm 1: mul  <x1=int64#5
# asm 2: mul  <x1=%r8
mul  %r8

# qhasm:   r1 = squarerax
# asm 1: mov  <squarerax=int64#7,>r1=int64#9
# asm 2: mov  <squarerax=%rax,>r1=%r11
mov  %rax,%r11

# qhasm:   squarer11 = squarerdx
# asm 1: mov  <squarerdx=int64#3,>squarer11=int64#10
# asm 2: mov  <squarerdx=%rdx,>squarer11=%r12
mov  %rdx,%r12

# qhasm:   squarerax = x0
# asm 1: mov  <x0=int64#4,>squarerax=int64#7
# asm 2: mov  <x0=%rcx,>squarerax=%rax
mov  %rcx,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 16)
# asm 1: mulq  16(<input_0=int64#1)
# asm 2: mulq  16(<input_0=%rdi)
mulq  16(%rdi)

# qhasm:   r2 = squarerax
# asm 1: mov  <squarerax=int64#7,>r2=int64#11
# asm 2: mov  <squarerax=%rax,>r2=%r13
mov  %rax,%r13

# qhasm:   squarer21 = squarerdx
# asm 1: mov  <squarerdx=int64#3,>squarer21=int64#12
# asm 2: mov  <squarerdx=%rdx,>squarer21=%r14
mov  %rdx,%r14

# qhasm:   squarerax = x0
# asm 1: mov  <x0=int64#4,>squarerax=int64#7
# asm 2: mov  <x0=%rcx,>squarerax=%rax
mov  %rcx,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 24)
# asm 1: mulq  24(<input_0=int64#1)
# asm 2: mulq  24(<input_0=%rdi)
mulq  24(%rdi)

# qhasm:   r3 = squarerax
# asm 1: mov  <squarerax=int64#7,>r3=int64#13
# asm 2: mov  <squarerax=%rax,>r3=%r15
mov  %rax,%r15

# qhasm:   squarer31 = squarerdx
# asm 1: mov  <squarerdx=int64#3,>squarer31=int64#14
# asm 2: mov  <squarerdx=%rdx,>squarer31=%rbx
mov  %rdx,%rbx

# qhasm:   squarerax = x0
# asm 1: mov  <x0=int64#4,>squarerax=int64#7
# asm 2: mov  <x0=%rcx,>squarerax=%rax
mov  %rcx,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 32)
# asm 1: mulq  32(<input_0=int64#1)
# asm 2: mulq  32(<input_0=%rdi)
mulq  32(%rdi)

# qhasm:   r4 = squarerax
# asm 1: mov  <squarerax=int64#7,>r4=int64#4
# asm 2: mov  <squarerax=%rax,>r4=%rcx
mov  %rax,%rcx

# qhasm:   squarer41 = squarerdx
# asm 1: mov  <squarerdx=int64#3,>squarer41=int64#15
# asm 2: mov  <squarerdx=%rdx,>squarer41=%rbp
mov  %rdx,%rbp

# qhasm:   squarerax = x1
# asm 1: mov  <x1=int64#5,>squarerax=int64#7
# asm 2: mov  <x1=%r8,>squarerax=%rax
mov  %r8,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * x1
# asm 1: mul  <x1=int64#5
# asm 2: mul  <x1=%r8
mul  %r8

# qhasm:   x1 += x1
# asm 1: add  <x1=int64#5,<x1=int64#5
# asm 2: add  <x1=%r8,<x1=%r8
add  %r8,%r8

# qhasm:   carry? r2 += squarerax
# asm 1: add  <squarerax=int64#7,<r2=int64#11
# asm 2: add  <squarerax=%rax,<r2=%r13
add  %rax,%r13

# qhasm:   squarer21 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer21=int64#12
# asm 2: adc <squarerdx=%rdx,<squarer21=%r14
adc %rdx,%r14

# qhasm:   squarerax = x1
# asm 1: mov  <x1=int64#5,>squarerax=int64#7
# asm 2: mov  <x1=%r8,>squarerax=%rax
mov  %r8,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 16)
# asm 1: mulq  16(<input_0=int64#1)
# asm 2: mulq  16(<input_0=%rdi)
mulq  16(%rdi)

# qhasm:   carry? r3 += squarerax
# asm 1: add  <squarerax=int64#7,<r3=int64#13
# asm 2: add  <squarerax=%rax,<r3=%r15
add  %rax,%r15

# qhasm:   squarer31 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer31=int64#14
# asm 2: adc <squarerdx=%rdx,<squarer31=%rbx
adc %rdx,%rbx

# qhasm:   squarerax = x1
# asm 1: mov  <x1=int64#5,>squarerax=int64#7
# asm 2: mov  <x1=%r8,>squarerax=%rax
mov  %r8,%rax

# qhasm:   x1 *= 19
# asm 1: imulq  $19, <x1=int64#5,<x1=int64#5
# asm 2: imulq  $19, <x1=%r8,<x1=%r8
imulq  $19, %r8,%r8

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 24)
# asm 1: mulq  24(<input_0=int64#1)
# asm 2: mulq  24(<input_0=%rdi)
mulq  24(%rdi)

# qhasm:   carry? r4 += squarerax
# asm 1: add  <squarerax=int64#7,<r4=int64#4
# asm 2: add  <squarerax=%rax,<r4=%rcx
add  %rax,%rcx

# qhasm:   squarer41 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer41=int64#15
# asm 2: adc <squarerdx=%rdx,<squarer41=%rbp
adc %rdx,%rbp

# qhasm:   squarerax = x1
# asm 1: mov  <x1=int64#5,>squarerax=int64#7
# asm 2: mov  <x1=%r8,>squarerax=%rax
mov  %r8,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 32)
# asm 1: mulq  32(<input_0=int64#1)
# asm 2: mulq  32(<input_0=%rdi)
mulq  32(%rdi)

# qhasm:   carry? r0 += squarerax
# asm 1: add  <squarerax=int64#7,<r0=int64#6
# asm 2: add  <squarerax=%rax,<r0=%r9
add  %rax,%r9

# qhasm:   squarer01 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer01=int64#8
# asm 2: adc <squarerdx=%rdx,<squarer01=%r10
adc %rdx,%r10

# qhasm:   squarerax = *(uint64 *)(input_0 + 16)
# asm 1: movq   16(<input_0=int64#1),>squarerax=int64#7
# asm 2: movq   16(<input_0=%rdi),>squarerax=%rax
movq   16(%rdi),%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 16)
# asm 1: mulq  16(<input_0=int64#1)
# asm 2: mulq  16(<input_0=%rdi)
mulq  16(%rdi)

# qhasm:   carry? r4 += squarerax
# asm 1: add  <squarerax=int64#7,<r4=int64#4
# asm 2: add  <squarerax=%rax,<r4=%rcx
add  %rax,%rcx

# qhasm:   squarer41 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer41=int64#15
# asm 2: adc <squarerdx=%rdx,<squarer41=%rbp
adc %rdx,%rbp

# qhasm:   squarer41 = (squarer41.r4) << 13
# asm 1: shld $13,<r4=int64#4,<squarer41=int64#15
# asm 2: shld $13,<r4=%rcx,<squarer41=%rbp
shld $13,%rcx,%rbp

# qhasm:   squarerax = *(uint64 *)(input_0 + 16)
# asm 1: movq   16(<input_0=int64#1),>squarerax=int64#7
# asm 2: movq   16(<input_0=%rdi),>squarerax=%rax
movq   16(%rdi),%rax

# qhasm:   squarerax *= 38
# asm 1: imulq  $38, <squarerax=int64#7,<squarerax=int64#7
# asm 2: imulq  $38, <squarerax=%rax,<squarerax=%rax
imulq  $38, %rax,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 24)
# asm 1: mulq  24(<input_0=int64#1)
# asm 2: mulq  24(<input_0=%rdi)
mulq  24(%rdi)

# qhasm:   carry? r0 += squarerax
# asm 1: add  <squarerax=int64#7,<r0=int64#6
# asm 2: add  <squarerax=%rax,<r0=%r9
add  %rax,%r9

# qhasm:   squarer01 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer01=int64#8
# asm 2: adc <squarerdx=%rdx,<squarer01=%r10
adc %rdx,%r10

# qhasm:   squarer01 = (squarer01.r0) << 13
# asm 1: shld $13,<r0=int64#6,<squarer01=int64#8
# asm 2: shld $13,<r0=%r9,<squarer01=%r10
shld $13,%r9,%r10

# qhasm:   squarerax = *(uint64 *)(input_0 + 16)
# asm 1: movq   16(<input_0=int64#1),>squarerax=int64#7
# asm 2: movq   16(<input_0=%rdi),>squarerax=%rax
movq   16(%rdi),%rax

# qhasm:   squarerax *= 38
# asm 1: imulq  $38, <squarerax=int64#7,<squarerax=int64#7
# asm 2: imulq  $38, <squarerax=%rax,<squarerax=%rax
imulq  $38, %rax,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 32)
# asm 1: mulq  32(<input_0=int64#1)
# asm 2: mulq  32(<input_0=%rdi)
mulq  32(%rdi)

# qhasm:   carry? r1 += squarerax
# asm 1: add  <squarerax=int64#7,<r1=int64#9
# asm 2: add  <squarerax=%rax,<r1=%r11
add  %rax,%r11

# qhasm:   squarer11 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer11=int64#10
# asm 2: adc <squarerdx=%rdx,<squarer11=%r12
adc %rdx,%r12

# qhasm:   squarerax = *(uint64 *)(input_0 + 24)
# asm 1: movq   24(<input_0=int64#1),>squarerax=int64#7
# asm 2: movq   24(<input_0=%rdi),>squarerax=%rax
movq   24(%rdi),%rax

# qhasm:   squarerax *= 19
# asm 1: imulq  $19, <squarerax=int64#7,<squarerax=int64#7
# asm 2: imulq  $19, <squarerax=%rax,<squarerax=%rax
imulq  $19, %rax,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 24)
# asm 1: mulq  24(<input_0=int64#1)
# asm 2: mulq  24(<input_0=%rdi)
mulq  24(%rdi)

# qhasm:   carry? r1 += squarerax
# asm 1: add  <squarerax=int64#7,<r1=int64#9
# asm 2: add  <squarerax=%rax,<r1=%r11
add  %rax,%r11

# qhasm:   squarer11 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer11=int64#10
# asm 2: adc <squarerdx=%rdx,<squarer11=%r12
adc %rdx,%r12

# qhasm:   squarer11 = (squarer11.r1) << 13
# asm 1: shld $13,<r1=int64#9,<squarer11=int64#10
# asm 2: shld $13,<r1=%r11,<squarer11=%r12
shld $13,%r11,%r12

# qhasm:   squarerax = *(uint64 *)(input_0 + 24)
# asm 1: movq   24(<input_0=int64#1),>squarerax=int64#7
# asm 2: movq   24(<input_0=%rdi),>squarerax=%rax
movq   24(%rdi),%rax

# qhasm:   squarerax *= 38
# asm 1: imulq  $38, <squarerax=int64#7,<squarerax=int64#7
# asm 2: imulq  $38, <squarerax=%rax,<squarerax=%rax
imulq  $38, %rax,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 32)
# asm 1: mulq  32(<input_0=int64#1)
# asm 2: mulq  32(<input_0=%rdi)
mulq  32(%rdi)

# qhasm:   carry? r2 += squarerax
# asm 1: add  <squarerax=int64#7,<r2=int64#11
# asm 2: add  <squarerax=%rax,<r2=%r13
add  %rax,%r13

# qhasm:   squarer21 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer21=int64#12
# asm 2: adc <squarerdx=%rdx,<squarer21=%r14
adc %rdx,%r14

# qhasm:   squarer21 = (squarer21.r2) << 13
# asm 1: shld $13,<r2=int64#11,<squarer21=int64#12
# asm 2: shld $13,<r2=%r13,<squarer21=%r14
shld $13,%r13,%r14

# qhasm:   squarerax = *(uint64 *)(input_0 + 32)
# asm 1: movq   32(<input_0=int64#1),>squarerax=int64#7
# asm 2: movq   32(<input_0=%rdi),>squarerax=%rax
movq   32(%rdi),%rax

# qhasm:   squarerax *= 19
# asm 1: imulq  $19, <squarerax=int64#7,<squarerax=int64#7
# asm 2: imulq  $19, <squarerax=%rax,<squarerax=%rax
imulq  $19, %rax,%rax

# qhasm:   (uint128) squarerdx squarerax = squarerax * *(uint64 *)(input_0 + 32)
# asm 1: mulq  32(<input_0=int64#1)
# asm 2: mulq  32(<input_0=%rdi)
mulq  32(%rdi)

# qhasm:   carry? r3 += squarerax
# asm 1: add  <squarerax=int64#7,<r3=int64#13
# asm 2: add  <squarerax=%rax,<r3=%r15
add  %rax,%r15

# qhasm:   squarer31 += squarerdx + carry
# asm 1: adc <squarerdx=int64#3,<squarer31=int64#14
# asm 2: adc <squarerdx=%rdx,<squarer31=%rbx
adc %rdx,%rbx

# qhasm:   squarer31 = (squarer31.r3) << 13
# asm 1: shld $13,<r3=int64#13,<squarer31=int64#14
# asm 2: shld $13,<r3=%r15,<squarer31=%rbx
shld $13,%r15,%rbx

# qhasm:   squareredmask = *(uint64 *) &REDMASK51
# asm 1: movq REDMASK51,>squareredmask=int64#3
# asm 2: movq REDMASK51,>squareredmask=%rdx
movq REDMASK51(%rip),%rdx

# qhasm:   r4 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r4=int64#4
# asm 2: and  <squareredmask=%rdx,<r4=%rcx
and  %rdx,%rcx

# qhasm:   r4 += squarer31
# asm 1: add  <squarer31=int64#14,<r4=int64#4
# asm 2: add  <squarer31=%rbx,<r4=%rcx
add  %rbx,%rcx

# qhasm:   r0 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r0=int64#6
# asm 2: and  <squareredmask=%rdx,<r0=%r9
and  %rdx,%r9

# qhasm:   r1 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r1=int64#9
# asm 2: and  <squareredmask=%rdx,<r1=%r11
and  %rdx,%r11

# qhasm:   r1 += squarer01
# asm 1: add  <squarer01=int64#8,<r1=int64#9
# asm 2: add  <squarer01=%r10,<r1=%r11
add  %r10,%r11

# qhasm:   r2 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r2=int64#11
# asm 2: and  <squareredmask=%rdx,<r2=%r13
and  %rdx,%r13

# qhasm:   r2 += squarer11
# asm 1: add  <squarer11=int64#10,<r2=int64#11
# asm 2: add  <squarer11=%r12,<r2=%r13
add  %r12,%r13

# qhasm:   r3 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r3=int64#13
# asm 2: and  <squareredmask=%rdx,<r3=%r15
and  %rdx,%r15

# qhasm:   r3 += squarer21
# asm 1: add  <squarer21=int64#12,<r3=int64#13
# asm 2: add  <squarer21=%r14,<r3=%r15
add  %r14,%r15

# qhasm:   squarer41 *= 19
# asm 1: imulq  $19, <squarer41=int64#15,<squarer41=int64#15
# asm 2: imulq  $19, <squarer41=%rbp,<squarer41=%rbp
imulq  $19, %rbp,%rbp

# qhasm:   squaret = r0 + squarer41
# asm 1: lea  (<r0=int64#6,<squarer41=int64#15),>squaret=int64#6
# asm 2: lea  (<r0=%r9,<squarer41=%rbp),>squaret=%r9
lea  (%r9,%rbp),%r9

# qhasm:   r0 = squaret 
# asm 1: mov  <squaret=int64#6,>r0=int64#7
# asm 2: mov  <squaret=%r9,>r0=%rax
mov  %r9,%rax

# qhasm:   (uint64) squaret >>= 51
# asm 1: shr  $51,<squaret=int64#6
# asm 2: shr  $51,<squaret=%r9
shr  $51,%r9

# qhasm:   squaret += r1
# asm 1: add  <r1=int64#9,<squaret=int64#6
# asm 2: add  <r1=%r11,<squaret=%r9
add  %r11,%r9

# qhasm:   r0 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r0=int64#7
# asm 2: and  <squareredmask=%rdx,<r0=%rax
and  %rdx,%rax

# qhasm:   x1 = squaret
# asm 1: mov  <squaret=int64#6,>x1=int64#5
# asm 2: mov  <squaret=%r9,>x1=%r8
mov  %r9,%r8

# qhasm:   (uint64) squaret >>= 51
# asm 1: shr  $51,<squaret=int64#6
# asm 2: shr  $51,<squaret=%r9
shr  $51,%r9

# qhasm:   squaret += r2
# asm 1: add  <r2=int64#11,<squaret=int64#6
# asm 2: add  <r2=%r13,<squaret=%r9
add  %r13,%r9

# qhasm:   x1 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<x1=int64#5
# asm 2: and  <squareredmask=%rdx,<x1=%r8
and  %rdx,%r8

# qhasm:   r2 = squaret
# asm 1: mov  <squaret=int64#6,>r2=int64#8
# asm 2: mov  <squaret=%r9,>r2=%r10
mov  %r9,%r10

# qhasm:   (uint64) squaret >>= 51
# asm 1: shr  $51,<squaret=int64#6
# asm 2: shr  $51,<squaret=%r9
shr  $51,%r9

# qhasm:   squaret += r3
# asm 1: add  <r3=int64#13,<squaret=int64#6
# asm 2: add  <r3=%r15,<squaret=%r9
add  %r15,%r9

# qhasm:   r2 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r2=int64#8
# asm 2: and  <squareredmask=%rdx,<r2=%r10
and  %rdx,%r10

# qhasm:   *(uint64 *)(input_0 + 16) = r2
# asm 1: movq   <r2=int64#8,16(<input_0=int64#1)
# asm 2: movq   <r2=%r10,16(<input_0=%rdi)
movq   %r10,16(%rdi)

# qhasm:   r3 = squaret
# asm 1: mov  <squaret=int64#6,>r3=int64#8
# asm 2: mov  <squaret=%r9,>r3=%r10
mov  %r9,%r10

# qhasm:   (uint64) squaret >>= 51
# asm 1: shr  $51,<squaret=int64#6
# asm 2: shr  $51,<squaret=%r9
shr  $51,%r9

# qhasm:   squaret += r4
# asm 1: add  <r4=int64#4,<squaret=int64#6
# asm 2: add  <r4=%rcx,<squaret=%r9
add  %rcx,%r9

# qhasm:   r3 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r3=int64#8
# asm 2: and  <squareredmask=%rdx,<r3=%r10
and  %rdx,%r10

# qhasm:   *(uint64 *)(input_0 + 24) = r3
# asm 1: movq   <r3=int64#8,24(<input_0=int64#1)
# asm 2: movq   <r3=%r10,24(<input_0=%rdi)
movq   %r10,24(%rdi)

# qhasm:   r4 = squaret
# asm 1: mov  <squaret=int64#6,>r4=int64#8
# asm 2: mov  <squaret=%r9,>r4=%r10
mov  %r9,%r10

# qhasm:   (uint64) squaret >>= 51
# asm 1: shr  $51,<squaret=int64#6
# asm 2: shr  $51,<squaret=%r9
shr  $51,%r9

# qhasm:   squaret *= 19
# asm 1: imulq  $19, <squaret=int64#6,<squaret=int64#6
# asm 2: imulq  $19, <squaret=%r9,<squaret=%r9
imulq  $19, %r9,%r9

# qhasm:   x0 = r0 + squaret
# asm 1: lea  (<r0=int64#7,<squaret=int64#6),>x0=int64#4
# asm 2: lea  (<r0=%rax,<squaret=%r9),>x0=%rcx
lea  (%rax,%r9),%rcx

# qhasm:   r4 &= squareredmask
# asm 1: and  <squareredmask=int64#3,<r4=int64#8
# asm 2: and  <squareredmask=%rdx,<r4=%r10
and  %rdx,%r10

# qhasm:   *(uint64 *)(input_0 + 32) = r4
# asm 1: movq   <r4=int64#8,32(<input_0=int64#1)
# asm 2: movq   <r4=%r10,32(<input_0=%rdi)
movq   %r10,32(%rdi)

# qhasm: =? iters - 0
# asm 1: cmp  $0,<iters=int64#2
# asm 2: cmp  $0,<iters=%rsi
cmp  $0,%rsi
# comment:fp stack unchanged by jump

# qhasm: goto loop if !=
jne ._loop

# qhasm: *(uint64 *)(input_0 + 0) = x0
# asm 1: movq   <x0=int64#4,0(<input_0=int64#1)
# asm 2: movq   <x0=%rcx,0(<input_0=%rdi)
movq   %rcx,0(%rdi)

# qhasm: *(uint64 *)(input_0 + 8) = x1
# asm 1: movq   <x1=int64#5,8(<input_0=int64#1)
# asm 2: movq   <x1=%r8,8(<input_0=%rdi)
movq   %r8,8(%rdi)

# qhasm: caller_r11 = r11_stack
# asm 1: movq <r11_stack=stack64#1,>caller_r11=int64#9
# asm 2: movq <r11_stack=0(%rsp),>caller_r11=%r11
movq 0(%rsp),%r11

# qhasm: caller_r12 = r12_stack
# asm 1: movq <r12_stack=stack64#2,>caller_r12=int64#10
# asm 2: movq <r12_stack=8(%rsp),>caller_r12=%r12
movq 8(%rsp),%r12

# qhasm: caller_r13 = r13_stack
# asm 1: movq <r13_stack=stack64#3,>caller_r13=int64#11
# asm 2: movq <r13_stack=16(%rsp),>caller_r13=%r13
movq 16(%rsp),%r13

# qhasm: caller_r14 = r14_stack
# asm 1: movq <r14_stack=stack64#4,>caller_r14=int64#12
# asm 2: movq <r14_stack=24(%rsp),>caller_r14=%r14
movq 24(%rsp),%r14

# qhasm: caller_r15 = r15_stack
# asm 1: movq <r15_stack=stack64#5,>caller_r15=int64#13
# asm 2: movq <r15_stack=32(%rsp),>caller_r15=%r15
movq 32(%rsp),%r15

# qhasm: caller_rbx = rbx_stack
# asm 1: movq <rbx_stack=stack64#6,>caller_rbx=int64#14
# asm 2: movq <rbx_stack=40(%rsp),>caller_rbx=%rbx
movq 40(%rsp),%rbx

# qhasm: caller_rbp = rbp_stack
# asm 1: movq <rbp_stack=stack64#7,>caller_rbp=int64#15
# asm 2: movq <rbp_stack=48(%rsp),>caller_rbp=%rbp
movq 48(%rsp),%rbp

# qhasm: return
add %r11,%rsp
ret
