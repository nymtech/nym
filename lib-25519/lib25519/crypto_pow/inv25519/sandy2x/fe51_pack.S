// linker define fe51_pack
// linker use REDMASK51

/*
   This file is the result of merging 
   amd64-51/fe25519_pack.c and amd64-51/fe25519_freeze.s.
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

# qhasm: int64 t

# qhasm: int64 s

# qhasm: int64 loop

# qhasm: int64 two51minus1

# qhasm: int64 two51minus19

# qhasm: stack64 r11_stack

# qhasm: stack64 r12_stack

# qhasm: stack64 r13_stack

# qhasm: stack64 r14_stack

# qhasm: stack64 r15_stack

# qhasm: stack64 rbx_stack

# qhasm: stack64 rbp_stack

# qhasm: enter fe51_pack
.p2align 5
.global _fe51_pack
.global fe51_pack
_fe51_pack:
fe51_pack:
mov %rsp,%r11
and $31,%r11
add $32,%r11
sub %r11,%rsp

# qhasm: r11_stack = caller_r11
# asm 1: movq <caller_r11=int64#9,>r11_stack=stack64#1
# asm 2: movq <caller_r11=%r11,>r11_stack=0(%rsp)
movq %r11,0(%rsp)

# qhasm: r12_stack = caller_r12
# asm 1: movq <caller_r12=int64#10,>r12_stack=stack64#2
# asm 2: movq <caller_r12=%r12,>r12_stack=8(%rsp)
movq %r12,8(%rsp)

# qhasm: r0 = *(uint64 *) (input_1 + 0)
# asm 1: movq   0(<input_1=int64#2),>r0=int64#3
# asm 2: movq   0(<input_1=%rsi),>r0=%rdx
movq   0(%rsi),%rdx

# qhasm: r1 = *(uint64 *) (input_1 + 8)
# asm 1: movq   8(<input_1=int64#2),>r1=int64#4
# asm 2: movq   8(<input_1=%rsi),>r1=%rcx
movq   8(%rsi),%rcx

# qhasm: r2 = *(uint64 *) (input_1 + 16)
# asm 1: movq   16(<input_1=int64#2),>r2=int64#5
# asm 2: movq   16(<input_1=%rsi),>r2=%r8
movq   16(%rsi),%r8

# qhasm: r3 = *(uint64 *) (input_1 + 24)
# asm 1: movq   24(<input_1=int64#2),>r3=int64#6
# asm 2: movq   24(<input_1=%rsi),>r3=%r9
movq   24(%rsi),%r9

# qhasm: r4 = *(uint64 *) (input_1 + 32)
# asm 1: movq   32(<input_1=int64#2),>r4=int64#2
# asm 2: movq   32(<input_1=%rsi),>r4=%rsi
movq   32(%rsi),%rsi

# qhasm: two51minus1 = *(uint64 *) &REDMASK51
# asm 1: movq REDMASK51,>two51minus1=int64#7
# asm 2: movq REDMASK51,>two51minus1=%rax
movq REDMASK51(%rip),%rax

# qhasm: two51minus19 = two51minus1 - 18
# asm 1: lea  -18(<two51minus1=int64#7),>two51minus19=int64#8
# asm 2: lea  -18(<two51minus1=%rax),>two51minus19=%r10
lea  -18(%rax),%r10

# qhasm: loop = 3
# asm 1: mov  $3,>loop=int64#9
# asm 2: mov  $3,>loop=%r11
mov  $3,%r11

# qhasm: reduceloop:
._reduceloop:

# qhasm:   t = r0
# asm 1: mov  <r0=int64#3,>t=int64#10
# asm 2: mov  <r0=%rdx,>t=%r12
mov  %rdx,%r12

# qhasm:   (uint64) t >>= 51
# asm 1: shr  $51,<t=int64#10
# asm 2: shr  $51,<t=%r12
shr  $51,%r12

# qhasm:   r0 &= two51minus1
# asm 1: and  <two51minus1=int64#7,<r0=int64#3
# asm 2: and  <two51minus1=%rax,<r0=%rdx
and  %rax,%rdx

# qhasm:   r1 += t
# asm 1: add  <t=int64#10,<r1=int64#4
# asm 2: add  <t=%r12,<r1=%rcx
add  %r12,%rcx

# qhasm:   t = r1
# asm 1: mov  <r1=int64#4,>t=int64#10
# asm 2: mov  <r1=%rcx,>t=%r12
mov  %rcx,%r12

# qhasm:   (uint64) t >>= 51
# asm 1: shr  $51,<t=int64#10
# asm 2: shr  $51,<t=%r12
shr  $51,%r12

# qhasm:   r1 &= two51minus1
# asm 1: and  <two51minus1=int64#7,<r1=int64#4
# asm 2: and  <two51minus1=%rax,<r1=%rcx
and  %rax,%rcx

# qhasm:   r2 += t
# asm 1: add  <t=int64#10,<r2=int64#5
# asm 2: add  <t=%r12,<r2=%r8
add  %r12,%r8

# qhasm:   t = r2
# asm 1: mov  <r2=int64#5,>t=int64#10
# asm 2: mov  <r2=%r8,>t=%r12
mov  %r8,%r12

# qhasm:   (uint64) t >>= 51
# asm 1: shr  $51,<t=int64#10
# asm 2: shr  $51,<t=%r12
shr  $51,%r12

# qhasm:   r2 &= two51minus1
# asm 1: and  <two51minus1=int64#7,<r2=int64#5
# asm 2: and  <two51minus1=%rax,<r2=%r8
and  %rax,%r8

# qhasm:   r3 += t
# asm 1: add  <t=int64#10,<r3=int64#6
# asm 2: add  <t=%r12,<r3=%r9
add  %r12,%r9

# qhasm:   t = r3
# asm 1: mov  <r3=int64#6,>t=int64#10
# asm 2: mov  <r3=%r9,>t=%r12
mov  %r9,%r12

# qhasm:   (uint64) t >>= 51
# asm 1: shr  $51,<t=int64#10
# asm 2: shr  $51,<t=%r12
shr  $51,%r12

# qhasm:   r3 &= two51minus1
# asm 1: and  <two51minus1=int64#7,<r3=int64#6
# asm 2: and  <two51minus1=%rax,<r3=%r9
and  %rax,%r9

# qhasm:   r4 += t
# asm 1: add  <t=int64#10,<r4=int64#2
# asm 2: add  <t=%r12,<r4=%rsi
add  %r12,%rsi

# qhasm:   t = r4
# asm 1: mov  <r4=int64#2,>t=int64#10
# asm 2: mov  <r4=%rsi,>t=%r12
mov  %rsi,%r12

# qhasm:   (uint64) t >>= 51
# asm 1: shr  $51,<t=int64#10
# asm 2: shr  $51,<t=%r12
shr  $51,%r12

# qhasm:   r4 &= two51minus1
# asm 1: and  <two51minus1=int64#7,<r4=int64#2
# asm 2: and  <two51minus1=%rax,<r4=%rsi
and  %rax,%rsi

# qhasm:   t *= 19
# asm 1: imulq  $19, <t=int64#10,<t=int64#10
# asm 2: imulq  $19, <t=%r12,<t=%r12
imulq  $19, %r12,%r12

# qhasm:   r0 += t
# asm 1: add  <t=int64#10,<r0=int64#3
# asm 2: add  <t=%r12,<r0=%rdx
add  %r12,%rdx

# qhasm:                    unsigned>? loop -= 1
# asm 1: sub  $1,<loop=int64#9
# asm 2: sub  $1,<loop=%r11
sub  $1,%r11
# comment:fp stack unchanged by jump

# qhasm: goto reduceloop if unsigned>
ja ._reduceloop

# qhasm: t = 1
# asm 1: mov  $1,>t=int64#10
# asm 2: mov  $1,>t=%r12
mov  $1,%r12

# qhasm:             signed<? r0 - two51minus19
# asm 1: cmp  <two51minus19=int64#8,<r0=int64#3
# asm 2: cmp  <two51minus19=%r10,<r0=%rdx
cmp  %r10,%rdx

# qhasm: t = loop if signed<
# asm 1: cmovl <loop=int64#9,<t=int64#10
# asm 2: cmovl <loop=%r11,<t=%r12
cmovl %r11,%r12

# qhasm:              =? r1 - two51minus1
# asm 1: cmp  <two51minus1=int64#7,<r1=int64#4
# asm 2: cmp  <two51minus1=%rax,<r1=%rcx
cmp  %rax,%rcx

# qhasm: t = loop if !=
# asm 1: cmovne <loop=int64#9,<t=int64#10
# asm 2: cmovne <loop=%r11,<t=%r12
cmovne %r11,%r12

# qhasm:              =? r2 - two51minus1
# asm 1: cmp  <two51minus1=int64#7,<r2=int64#5
# asm 2: cmp  <two51minus1=%rax,<r2=%r8
cmp  %rax,%r8

# qhasm: t = loop if !=
# asm 1: cmovne <loop=int64#9,<t=int64#10
# asm 2: cmovne <loop=%r11,<t=%r12
cmovne %r11,%r12

# qhasm:              =? r3 - two51minus1
# asm 1: cmp  <two51minus1=int64#7,<r3=int64#6
# asm 2: cmp  <two51minus1=%rax,<r3=%r9
cmp  %rax,%r9

# qhasm: t = loop if !=
# asm 1: cmovne <loop=int64#9,<t=int64#10
# asm 2: cmovne <loop=%r11,<t=%r12
cmovne %r11,%r12

# qhasm:              =? r4 - two51minus1
# asm 1: cmp  <two51minus1=int64#7,<r4=int64#2
# asm 2: cmp  <two51minus1=%rax,<r4=%rsi
cmp  %rax,%rsi

# qhasm: t = loop if !=
# asm 1: cmovne <loop=int64#9,<t=int64#10
# asm 2: cmovne <loop=%r11,<t=%r12
cmovne %r11,%r12

# qhasm: t = -t
# asm 1: neg  <t=int64#10
# asm 2: neg  <t=%r12
neg  %r12

# qhasm: two51minus1 &= t
# asm 1: and  <t=int64#10,<two51minus1=int64#7
# asm 2: and  <t=%r12,<two51minus1=%rax
and  %r12,%rax

# qhasm: two51minus19 &= t
# asm 1: and  <t=int64#10,<two51minus19=int64#8
# asm 2: and  <t=%r12,<two51minus19=%r10
and  %r12,%r10

# qhasm: r0 -= two51minus19
# asm 1: sub  <two51minus19=int64#8,<r0=int64#3
# asm 2: sub  <two51minus19=%r10,<r0=%rdx
sub  %r10,%rdx

# qhasm: r1 -= two51minus1
# asm 1: sub  <two51minus1=int64#7,<r1=int64#4
# asm 2: sub  <two51minus1=%rax,<r1=%rcx
sub  %rax,%rcx

# qhasm: r2 -= two51minus1
# asm 1: sub  <two51minus1=int64#7,<r2=int64#5
# asm 2: sub  <two51minus1=%rax,<r2=%r8
sub  %rax,%r8

# qhasm: r3 -= two51minus1
# asm 1: sub  <two51minus1=int64#7,<r3=int64#6
# asm 2: sub  <two51minus1=%rax,<r3=%r9
sub  %rax,%r9

# qhasm: r4 -= two51minus1
# asm 1: sub  <two51minus1=int64#7,<r4=int64#2
# asm 2: sub  <two51minus1=%rax,<r4=%rsi
sub  %rax,%rsi

# qhasm: t = r0
# asm 1: mov  <r0=int64#3,>t=int64#7
# asm 2: mov  <r0=%rdx,>t=%rax
mov  %rdx,%rax

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#7d
# asm 2: and  $0xFF,<t=%eax
and  $0xFF,%eax

# qhasm: *(uint8 *) (input_0 + 0) = t
# asm 1: movb   <t=int64#7b,0(<input_0=int64#1)
# asm 2: movb   <t=%al,0(<input_0=%rdi)
movb   %al,0(%rdi)

# qhasm: t = r0
# asm 1: mov  <r0=int64#3,>t=int64#7
# asm 2: mov  <r0=%rdx,>t=%rax
mov  %rdx,%rax

# qhasm: (uint64) t >>= 8
# asm 1: shr  $8,<t=int64#7
# asm 2: shr  $8,<t=%rax
shr  $8,%rax

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#7d
# asm 2: and  $0xFF,<t=%eax
and  $0xFF,%eax

# qhasm: *(uint8 *) (input_0 + 1) = t
# asm 1: movb   <t=int64#7b,1(<input_0=int64#1)
# asm 2: movb   <t=%al,1(<input_0=%rdi)
movb   %al,1(%rdi)

# qhasm: t = r0
# asm 1: mov  <r0=int64#3,>t=int64#7
# asm 2: mov  <r0=%rdx,>t=%rax
mov  %rdx,%rax

# qhasm: (uint64) t >>= 16
# asm 1: shr  $16,<t=int64#7
# asm 2: shr  $16,<t=%rax
shr  $16,%rax

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#7d
# asm 2: and  $0xFF,<t=%eax
and  $0xFF,%eax

# qhasm: *(uint8 *) (input_0 + 2) = t
# asm 1: movb   <t=int64#7b,2(<input_0=int64#1)
# asm 2: movb   <t=%al,2(<input_0=%rdi)
movb   %al,2(%rdi)

# qhasm: t = r0
# asm 1: mov  <r0=int64#3,>t=int64#7
# asm 2: mov  <r0=%rdx,>t=%rax
mov  %rdx,%rax

# qhasm: (uint64) t >>= 24
# asm 1: shr  $24,<t=int64#7
# asm 2: shr  $24,<t=%rax
shr  $24,%rax

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#7d
# asm 2: and  $0xFF,<t=%eax
and  $0xFF,%eax

# qhasm: *(uint8 *) (input_0 + 3) = t
# asm 1: movb   <t=int64#7b,3(<input_0=int64#1)
# asm 2: movb   <t=%al,3(<input_0=%rdi)
movb   %al,3(%rdi)

# qhasm: t = r0
# asm 1: mov  <r0=int64#3,>t=int64#7
# asm 2: mov  <r0=%rdx,>t=%rax
mov  %rdx,%rax

# qhasm: (uint64) t >>= 32
# asm 1: shr  $32,<t=int64#7
# asm 2: shr  $32,<t=%rax
shr  $32,%rax

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#7d
# asm 2: and  $0xFF,<t=%eax
and  $0xFF,%eax

# qhasm: *(uint8 *) (input_0 + 4) = t
# asm 1: movb   <t=int64#7b,4(<input_0=int64#1)
# asm 2: movb   <t=%al,4(<input_0=%rdi)
movb   %al,4(%rdi)

# qhasm: t = r0
# asm 1: mov  <r0=int64#3,>t=int64#7
# asm 2: mov  <r0=%rdx,>t=%rax
mov  %rdx,%rax

# qhasm: (uint64) t >>= 40
# asm 1: shr  $40,<t=int64#7
# asm 2: shr  $40,<t=%rax
shr  $40,%rax

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#7d
# asm 2: and  $0xFF,<t=%eax
and  $0xFF,%eax

# qhasm: *(uint8 *) (input_0 + 5) = t
# asm 1: movb   <t=int64#7b,5(<input_0=int64#1)
# asm 2: movb   <t=%al,5(<input_0=%rdi)
movb   %al,5(%rdi)

# qhasm: s = r0
# asm 1: mov  <r0=int64#3,>s=int64#3
# asm 2: mov  <r0=%rdx,>s=%rdx
mov  %rdx,%rdx

# qhasm: (uint64) s >>= 48
# asm 1: shr  $48,<s=int64#3
# asm 2: shr  $48,<s=%rdx
shr  $48,%rdx

# qhasm: t = r1
# asm 1: mov  <r1=int64#4,>t=int64#7
# asm 2: mov  <r1=%rcx,>t=%rax
mov  %rcx,%rax

# qhasm: t <<= 3
# asm 1: shl  $3,<t=int64#7
# asm 2: shl  $3,<t=%rax
shl  $3,%rax

# qhasm: (uint32) t &= 0xF8
# asm 1: and  $0xF8,<t=int64#7d
# asm 2: and  $0xF8,<t=%eax
and  $0xF8,%eax

# qhasm: t ^= s
# asm 1: xor  <s=int64#3,<t=int64#7
# asm 2: xor  <s=%rdx,<t=%rax
xor  %rdx,%rax

# qhasm: *(uint8 *) (input_0 + 6) = t
# asm 1: movb   <t=int64#7b,6(<input_0=int64#1)
# asm 2: movb   <t=%al,6(<input_0=%rdi)
movb   %al,6(%rdi)

# qhasm: t = r1
# asm 1: mov  <r1=int64#4,>t=int64#3
# asm 2: mov  <r1=%rcx,>t=%rdx
mov  %rcx,%rdx

# qhasm: (uint64) t >>= 5
# asm 1: shr  $5,<t=int64#3
# asm 2: shr  $5,<t=%rdx
shr  $5,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 7) = t
# asm 1: movb   <t=int64#3b,7(<input_0=int64#1)
# asm 2: movb   <t=%dl,7(<input_0=%rdi)
movb   %dl,7(%rdi)

# qhasm: t = r1
# asm 1: mov  <r1=int64#4,>t=int64#3
# asm 2: mov  <r1=%rcx,>t=%rdx
mov  %rcx,%rdx

# qhasm: (uint64) t >>= 13
# asm 1: shr  $13,<t=int64#3
# asm 2: shr  $13,<t=%rdx
shr  $13,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 8) = t
# asm 1: movb   <t=int64#3b,8(<input_0=int64#1)
# asm 2: movb   <t=%dl,8(<input_0=%rdi)
movb   %dl,8(%rdi)

# qhasm: t = r1
# asm 1: mov  <r1=int64#4,>t=int64#3
# asm 2: mov  <r1=%rcx,>t=%rdx
mov  %rcx,%rdx

# qhasm: (uint64) t >>= 21
# asm 1: shr  $21,<t=int64#3
# asm 2: shr  $21,<t=%rdx
shr  $21,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 9) = t
# asm 1: movb   <t=int64#3b,9(<input_0=int64#1)
# asm 2: movb   <t=%dl,9(<input_0=%rdi)
movb   %dl,9(%rdi)

# qhasm: t = r1
# asm 1: mov  <r1=int64#4,>t=int64#3
# asm 2: mov  <r1=%rcx,>t=%rdx
mov  %rcx,%rdx

# qhasm: (uint64) t >>= 29
# asm 1: shr  $29,<t=int64#3
# asm 2: shr  $29,<t=%rdx
shr  $29,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 10) = t
# asm 1: movb   <t=int64#3b,10(<input_0=int64#1)
# asm 2: movb   <t=%dl,10(<input_0=%rdi)
movb   %dl,10(%rdi)

# qhasm: t = r1
# asm 1: mov  <r1=int64#4,>t=int64#3
# asm 2: mov  <r1=%rcx,>t=%rdx
mov  %rcx,%rdx

# qhasm: (uint64) t >>= 37
# asm 1: shr  $37,<t=int64#3
# asm 2: shr  $37,<t=%rdx
shr  $37,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 11) = t
# asm 1: movb   <t=int64#3b,11(<input_0=int64#1)
# asm 2: movb   <t=%dl,11(<input_0=%rdi)
movb   %dl,11(%rdi)

# qhasm: s = r1
# asm 1: mov  <r1=int64#4,>s=int64#3
# asm 2: mov  <r1=%rcx,>s=%rdx
mov  %rcx,%rdx

# qhasm: (uint64) s >>= 45
# asm 1: shr  $45,<s=int64#3
# asm 2: shr  $45,<s=%rdx
shr  $45,%rdx

# qhasm: t = r2
# asm 1: mov  <r2=int64#5,>t=int64#4
# asm 2: mov  <r2=%r8,>t=%rcx
mov  %r8,%rcx

# qhasm: t <<= 6
# asm 1: shl  $6,<t=int64#4
# asm 2: shl  $6,<t=%rcx
shl  $6,%rcx

# qhasm: (uint32) t &= 0xC0
# asm 1: and  $0xC0,<t=int64#4d
# asm 2: and  $0xC0,<t=%ecx
and  $0xC0,%ecx

# qhasm: t ^= s
# asm 1: xor  <s=int64#3,<t=int64#4
# asm 2: xor  <s=%rdx,<t=%rcx
xor  %rdx,%rcx

# qhasm: *(uint8 *) (input_0 + 12) = t
# asm 1: movb   <t=int64#4b,12(<input_0=int64#1)
# asm 2: movb   <t=%cl,12(<input_0=%rdi)
movb   %cl,12(%rdi)

# qhasm: t = r2
# asm 1: mov  <r2=int64#5,>t=int64#3
# asm 2: mov  <r2=%r8,>t=%rdx
mov  %r8,%rdx

# qhasm: (uint64) t >>= 2
# asm 1: shr  $2,<t=int64#3
# asm 2: shr  $2,<t=%rdx
shr  $2,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 13) = t
# asm 1: movb   <t=int64#3b,13(<input_0=int64#1)
# asm 2: movb   <t=%dl,13(<input_0=%rdi)
movb   %dl,13(%rdi)

# qhasm: t = r2
# asm 1: mov  <r2=int64#5,>t=int64#3
# asm 2: mov  <r2=%r8,>t=%rdx
mov  %r8,%rdx

# qhasm: (uint64) t >>= 10
# asm 1: shr  $10,<t=int64#3
# asm 2: shr  $10,<t=%rdx
shr  $10,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 14) = t
# asm 1: movb   <t=int64#3b,14(<input_0=int64#1)
# asm 2: movb   <t=%dl,14(<input_0=%rdi)
movb   %dl,14(%rdi)

# qhasm: t = r2
# asm 1: mov  <r2=int64#5,>t=int64#3
# asm 2: mov  <r2=%r8,>t=%rdx
mov  %r8,%rdx

# qhasm: (uint64) t >>= 18
# asm 1: shr  $18,<t=int64#3
# asm 2: shr  $18,<t=%rdx
shr  $18,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 15) = t
# asm 1: movb   <t=int64#3b,15(<input_0=int64#1)
# asm 2: movb   <t=%dl,15(<input_0=%rdi)
movb   %dl,15(%rdi)

# qhasm: t = r2
# asm 1: mov  <r2=int64#5,>t=int64#3
# asm 2: mov  <r2=%r8,>t=%rdx
mov  %r8,%rdx

# qhasm: (uint64) t >>= 26
# asm 1: shr  $26,<t=int64#3
# asm 2: shr  $26,<t=%rdx
shr  $26,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 16) = t
# asm 1: movb   <t=int64#3b,16(<input_0=int64#1)
# asm 2: movb   <t=%dl,16(<input_0=%rdi)
movb   %dl,16(%rdi)

# qhasm: t = r2
# asm 1: mov  <r2=int64#5,>t=int64#3
# asm 2: mov  <r2=%r8,>t=%rdx
mov  %r8,%rdx

# qhasm: (uint64) t >>= 34
# asm 1: shr  $34,<t=int64#3
# asm 2: shr  $34,<t=%rdx
shr  $34,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 17) = t
# asm 1: movb   <t=int64#3b,17(<input_0=int64#1)
# asm 2: movb   <t=%dl,17(<input_0=%rdi)
movb   %dl,17(%rdi)

# qhasm: t = r2
# asm 1: mov  <r2=int64#5,>t=int64#3
# asm 2: mov  <r2=%r8,>t=%rdx
mov  %r8,%rdx

# qhasm: (uint64) t >>= 42
# asm 1: shr  $42,<t=int64#3
# asm 2: shr  $42,<t=%rdx
shr  $42,%rdx

# qhasm: *(uint8 *) (input_0 + 18) = t
# asm 1: movb   <t=int64#3b,18(<input_0=int64#1)
# asm 2: movb   <t=%dl,18(<input_0=%rdi)
movb   %dl,18(%rdi)

# qhasm: s = r2
# asm 1: mov  <r2=int64#5,>s=int64#3
# asm 2: mov  <r2=%r8,>s=%rdx
mov  %r8,%rdx

# qhasm: (uint64) s >>= 50
# asm 1: shr  $50,<s=int64#3
# asm 2: shr  $50,<s=%rdx
shr  $50,%rdx

# qhasm: t = r3
# asm 1: mov  <r3=int64#6,>t=int64#4
# asm 2: mov  <r3=%r9,>t=%rcx
mov  %r9,%rcx

# qhasm: t <<= 1
# asm 1: shl  $1,<t=int64#4
# asm 2: shl  $1,<t=%rcx
shl  $1,%rcx

# qhasm: (uint32) t &= 0xFE
# asm 1: and  $0xFE,<t=int64#4d
# asm 2: and  $0xFE,<t=%ecx
and  $0xFE,%ecx

# qhasm: t ^= s
# asm 1: xor  <s=int64#3,<t=int64#4
# asm 2: xor  <s=%rdx,<t=%rcx
xor  %rdx,%rcx

# qhasm: *(uint8 *) (input_0 + 19) = t
# asm 1: movb   <t=int64#4b,19(<input_0=int64#1)
# asm 2: movb   <t=%cl,19(<input_0=%rdi)
movb   %cl,19(%rdi)

# qhasm: t = r3
# asm 1: mov  <r3=int64#6,>t=int64#3
# asm 2: mov  <r3=%r9,>t=%rdx
mov  %r9,%rdx

# qhasm: (uint64) t >>= 7
# asm 1: shr  $7,<t=int64#3
# asm 2: shr  $7,<t=%rdx
shr  $7,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 20) = t
# asm 1: movb   <t=int64#3b,20(<input_0=int64#1)
# asm 2: movb   <t=%dl,20(<input_0=%rdi)
movb   %dl,20(%rdi)

# qhasm: t = r3
# asm 1: mov  <r3=int64#6,>t=int64#3
# asm 2: mov  <r3=%r9,>t=%rdx
mov  %r9,%rdx

# qhasm: (uint64) t >>= 15
# asm 1: shr  $15,<t=int64#3
# asm 2: shr  $15,<t=%rdx
shr  $15,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 21) = t
# asm 1: movb   <t=int64#3b,21(<input_0=int64#1)
# asm 2: movb   <t=%dl,21(<input_0=%rdi)
movb   %dl,21(%rdi)

# qhasm: t = r3
# asm 1: mov  <r3=int64#6,>t=int64#3
# asm 2: mov  <r3=%r9,>t=%rdx
mov  %r9,%rdx

# qhasm: (uint64) t >>= 23
# asm 1: shr  $23,<t=int64#3
# asm 2: shr  $23,<t=%rdx
shr  $23,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 22) = t
# asm 1: movb   <t=int64#3b,22(<input_0=int64#1)
# asm 2: movb   <t=%dl,22(<input_0=%rdi)
movb   %dl,22(%rdi)

# qhasm: t = r3
# asm 1: mov  <r3=int64#6,>t=int64#3
# asm 2: mov  <r3=%r9,>t=%rdx
mov  %r9,%rdx

# qhasm: (uint64) t >>= 31
# asm 1: shr  $31,<t=int64#3
# asm 2: shr  $31,<t=%rdx
shr  $31,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 23) = t
# asm 1: movb   <t=int64#3b,23(<input_0=int64#1)
# asm 2: movb   <t=%dl,23(<input_0=%rdi)
movb   %dl,23(%rdi)

# qhasm: t = r3
# asm 1: mov  <r3=int64#6,>t=int64#3
# asm 2: mov  <r3=%r9,>t=%rdx
mov  %r9,%rdx

# qhasm: (uint64) t >>= 39
# asm 1: shr  $39,<t=int64#3
# asm 2: shr  $39,<t=%rdx
shr  $39,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 24) = t
# asm 1: movb   <t=int64#3b,24(<input_0=int64#1)
# asm 2: movb   <t=%dl,24(<input_0=%rdi)
movb   %dl,24(%rdi)

# qhasm: s = r3
# asm 1: mov  <r3=int64#6,>s=int64#3
# asm 2: mov  <r3=%r9,>s=%rdx
mov  %r9,%rdx

# qhasm: (uint64) s >>= 47
# asm 1: shr  $47,<s=int64#3
# asm 2: shr  $47,<s=%rdx
shr  $47,%rdx

# qhasm: t = r4
# asm 1: mov  <r4=int64#2,>t=int64#4
# asm 2: mov  <r4=%rsi,>t=%rcx
mov  %rsi,%rcx

# qhasm: t <<= 4
# asm 1: shl  $4,<t=int64#4
# asm 2: shl  $4,<t=%rcx
shl  $4,%rcx

# qhasm: (uint32) t &= 0xF0
# asm 1: and  $0xF0,<t=int64#4d
# asm 2: and  $0xF0,<t=%ecx
and  $0xF0,%ecx

# qhasm: t ^= s
# asm 1: xor  <s=int64#3,<t=int64#4
# asm 2: xor  <s=%rdx,<t=%rcx
xor  %rdx,%rcx

# qhasm: *(uint8 *) (input_0 + 25) = t
# asm 1: movb   <t=int64#4b,25(<input_0=int64#1)
# asm 2: movb   <t=%cl,25(<input_0=%rdi)
movb   %cl,25(%rdi)

# qhasm: t = r4
# asm 1: mov  <r4=int64#2,>t=int64#3
# asm 2: mov  <r4=%rsi,>t=%rdx
mov  %rsi,%rdx

# qhasm: (uint64) t >>= 4
# asm 1: shr  $4,<t=int64#3
# asm 2: shr  $4,<t=%rdx
shr  $4,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 26) = t
# asm 1: movb   <t=int64#3b,26(<input_0=int64#1)
# asm 2: movb   <t=%dl,26(<input_0=%rdi)
movb   %dl,26(%rdi)

# qhasm: t = r4
# asm 1: mov  <r4=int64#2,>t=int64#3
# asm 2: mov  <r4=%rsi,>t=%rdx
mov  %rsi,%rdx

# qhasm: (uint64) t >>= 12
# asm 1: shr  $12,<t=int64#3
# asm 2: shr  $12,<t=%rdx
shr  $12,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 27) = t
# asm 1: movb   <t=int64#3b,27(<input_0=int64#1)
# asm 2: movb   <t=%dl,27(<input_0=%rdi)
movb   %dl,27(%rdi)

# qhasm: t = r4
# asm 1: mov  <r4=int64#2,>t=int64#3
# asm 2: mov  <r4=%rsi,>t=%rdx
mov  %rsi,%rdx

# qhasm: (uint64) t >>= 20
# asm 1: shr  $20,<t=int64#3
# asm 2: shr  $20,<t=%rdx
shr  $20,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 28) = t
# asm 1: movb   <t=int64#3b,28(<input_0=int64#1)
# asm 2: movb   <t=%dl,28(<input_0=%rdi)
movb   %dl,28(%rdi)

# qhasm: t = r4
# asm 1: mov  <r4=int64#2,>t=int64#3
# asm 2: mov  <r4=%rsi,>t=%rdx
mov  %rsi,%rdx

# qhasm: (uint64) t >>= 28
# asm 1: shr  $28,<t=int64#3
# asm 2: shr  $28,<t=%rdx
shr  $28,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 29) = t
# asm 1: movb   <t=int64#3b,29(<input_0=int64#1)
# asm 2: movb   <t=%dl,29(<input_0=%rdi)
movb   %dl,29(%rdi)

# qhasm: t = r4
# asm 1: mov  <r4=int64#2,>t=int64#3
# asm 2: mov  <r4=%rsi,>t=%rdx
mov  %rsi,%rdx

# qhasm: (uint64) t >>= 36
# asm 1: shr  $36,<t=int64#3
# asm 2: shr  $36,<t=%rdx
shr  $36,%rdx

# qhasm: (uint32) t &= 0xFF
# asm 1: and  $0xFF,<t=int64#3d
# asm 2: and  $0xFF,<t=%edx
and  $0xFF,%edx

# qhasm: *(uint8 *) (input_0 + 30) = t
# asm 1: movb   <t=int64#3b,30(<input_0=int64#1)
# asm 2: movb   <t=%dl,30(<input_0=%rdi)
movb   %dl,30(%rdi)

# qhasm: t = r4
# asm 1: mov  <r4=int64#2,>t=int64#2
# asm 2: mov  <r4=%rsi,>t=%rsi
mov  %rsi,%rsi

# qhasm: (uint64) t >>= 44
# asm 1: shr  $44,<t=int64#2
# asm 2: shr  $44,<t=%rsi
shr  $44,%rsi

# qhasm: *(uint8 *) (input_0 + 31) = t
# asm 1: movb   <t=int64#2b,31(<input_0=int64#1)
# asm 2: movb   <t=%sil,31(<input_0=%rdi)
movb   %sil,31(%rdi)

# qhasm: caller_r11 = r11_stack
# asm 1: movq <r11_stack=stack64#1,>caller_r11=int64#9
# asm 2: movq <r11_stack=0(%rsp),>caller_r11=%r11
movq 0(%rsp),%r11

# qhasm: caller_r12 = r12_stack
# asm 1: movq <r12_stack=stack64#2,>caller_r12=int64#10
# asm 2: movq <r12_stack=8(%rsp),>caller_r12=%r12
movq 8(%rsp),%r12

# qhasm: return 
add %r11,%rsp
ret
