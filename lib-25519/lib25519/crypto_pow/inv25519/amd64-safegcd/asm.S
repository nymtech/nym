
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

# qhasm: int64 m

# qhasm: int64 f

# qhasm: int64 g

# qhasm: int64 u

# qhasm: int64 v

# qhasm: int64 r

# qhasm: int64 s

# qhasm: int64 uvrs

# qhasm: int64 fuv

# qhasm: int64 grs

# qhasm: int64 mnew

# qhasm: int64 z

# qhasm: int64 loop

# qhasm: int64 rax

# qhasm: int64 rdx

# qhasm: int64 t0

# qhasm: int64 t1

# qhasm: int64 t2

# qhasm: int64 h

# qhasm: int64 oldg

# qhasm: int64 i

# qhasm: int64 j

# qhasm: int64 f0

# qhasm: int64 g0

# qhasm: int64 rtimesoldv

# qhasm: int64 stimesolds

# qhasm: stack64 stack_out

# qhasm: stack256 stack_m1

# qhasm: stack64 stack_m

# qhasm: stack256 stack_fxgx

# qhasm: stack256 stack_uuss

# qhasm: stack256 stack_vvrr

# qhasm: stack256 stack_fygy

# qhasm: stack64 stack_fuv

# qhasm: stack64 stack_f

# qhasm: reg256 carryy

# qhasm: reg256 carryz

# qhasm: reg256 d0

# qhasm: reg256 d1

# qhasm: reg256 d0x19

# qhasm: reg256 d1x19

# qhasm: reg256 out0

# qhasm: reg256 FVGS0

# qhasm: reg256 GSFV0

# qhasm: reg256 out1

# qhasm: reg256 out1plus

# qhasm: stack256 stack_FVGS0

# qhasm: reg256 FVGS1

# qhasm: reg256 GSFV1

# qhasm: reg256 out2

# qhasm: reg256 out2plus

# qhasm: stack256 stack_FVGS1

# qhasm: reg256 FVGS2

# qhasm: reg256 GSFV2

# qhasm: reg256 out3

# qhasm: reg256 out3plus

# qhasm: stack256 stack_FVGS2

# qhasm: reg256 FVGS3

# qhasm: reg256 GSFV3

# qhasm: reg256 out4

# qhasm: reg256 out4plus

# qhasm: stack256 stack_FVGS3

# qhasm: reg256 FVGS4

# qhasm: reg256 GSFV4

# qhasm: reg256 out5

# qhasm: reg256 out5plus

# qhasm: stack256 stack_FVGS4

# qhasm: reg256 FVGS5

# qhasm: reg256 GSFV5

# qhasm: reg256 out6

# qhasm: reg256 out6plus

# qhasm: stack256 stack_FVGS5

# qhasm: reg256 FVGS6

# qhasm: reg256 GSFV6

# qhasm: reg256 out7

# qhasm: reg256 out7plus

# qhasm: stack256 stack_FVGS6

# qhasm: reg256 FVGS7

# qhasm: reg256 GSFV7

# qhasm: reg256 out8

# qhasm: reg256 out8plus

# qhasm: stack256 stack_FVGS7

# qhasm: reg256 FVGS8

# qhasm: reg256 GSFV8

# qhasm: reg256 out9

# qhasm: reg256 out9plus

# qhasm: stack256 stack_FVGS8

# qhasm: reg256 out8plus2

# qhasm: reg256 out10

# qhasm: reg256 ta

# qhasm: reg256 tb

# qhasm: reg256 uuss

# qhasm: reg256 uuss0

# qhasm: reg256 uuss1

# qhasm: reg256 vvrr

# qhasm: reg256 vvrr0

# qhasm: reg256 vvrr1

# qhasm: int64            _m2p62

# qhasm: stack64     stack_m2p62

# qhasm: int64             _2p20

# qhasm: stack64      stack_2p20

# qhasm: int64            _m2p41

# qhasm: stack64     stack_m2p41

# qhasm: int64            _m2p20

# qhasm: stack64     stack_m2p20

# qhasm: int64        _2p20a2p41

# qhasm: stack64 stack_2p20a2p41

# qhasm: reg256 _19x4

# qhasm: reg256 _0_19x4

# qhasm: reg256 _32767x4

# qhasm: reg256 _inv19_2p30x4

# qhasm: reg256 _2p30m1x4

# qhasm: reg256 _2p33x4

# qhasm: reg256 _2p48x4

# qhasm: reg256 _2p63x4

# qhasm: reg256 _2p63m2p33x4

# qhasm: stack256 stack_19x4

# qhasm: stack256 stack_0_19x4

# qhasm: stack256 stack_32767x4

# qhasm: stack256 stack_inv19_2p30x4

# qhasm: stack256 stack_2p30m1x4

# qhasm: stack256 stack_2p33x4

# qhasm: stack256 stack_2p48x4

# qhasm: stack256 stack_2p63x4

# qhasm: stack256 stack_2p63m2p33x4

# qhasm: enter inverse25519_skylake_asm
.p2align 5
.global _CRYPTO_SHARED_NAMESPACE(asm)
.global CRYPTO_SHARED_NAMESPACE(asm)
_CRYPTO_SHARED_NAMESPACE(asm):
CRYPTO_SHARED_NAMESPACE(asm):
mov %rsp,%r11
and $31,%r11
add $832,%r11
sub %r11,%rsp

# qhasm: new stack_m1

# qhasm: stack_out = input_1
# asm 1: movq <input_1=int64#2,>stack_out=stack64#1
# asm 2: movq <input_1=%rsi,>stack_out=704(%rsp)
movq %rsi,704(%rsp)

# qhasm: stack64 stack_r11

# qhasm: stack_r11 = caller_r11
# asm 1: movq <caller_r11=int64#9,>stack_r11=stack64#2
# asm 2: movq <caller_r11=%r11,>stack_r11=712(%rsp)
movq %r11,712(%rsp)

# qhasm: stack64 stack_r12

# qhasm: stack_r12 = caller_r12
# asm 1: movq <caller_r12=int64#10,>stack_r12=stack64#3
# asm 2: movq <caller_r12=%r12,>stack_r12=720(%rsp)
movq %r12,720(%rsp)

# qhasm: stack64 stack_r13

# qhasm: stack_r13 = caller_r13
# asm 1: movq <caller_r13=int64#11,>stack_r13=stack64#4
# asm 2: movq <caller_r13=%r13,>stack_r13=728(%rsp)
movq %r13,728(%rsp)

# qhasm: stack64 stack_r14

# qhasm: stack_r14 = caller_r14
# asm 1: movq <caller_r14=int64#12,>stack_r14=stack64#5
# asm 2: movq <caller_r14=%r14,>stack_r14=736(%rsp)
movq %r14,736(%rsp)

# qhasm: stack64 stack_r15

# qhasm: stack_r15 = caller_r15
# asm 1: movq <caller_r15=int64#13,>stack_r15=stack64#6
# asm 2: movq <caller_r15=%r15,>stack_r15=744(%rsp)
movq %r15,744(%rsp)

# qhasm: stack64 stack_rbx

# qhasm: stack_rbx = caller_rbx
# asm 1: movq <caller_rbx=int64#14,>stack_rbx=stack64#7
# asm 2: movq <caller_rbx=%rbx,>stack_rbx=752(%rsp)
movq %rbx,752(%rsp)

# qhasm: stack64 stack_rbp

# qhasm: stack_rbp = caller_rbp
# asm 1: movq <caller_rbp=int64#15,>stack_rbp=stack64#8
# asm 2: movq <caller_rbp=%rbp,>stack_rbp=760(%rsp)
movq %rbp,760(%rsp)

# qhasm: int64 a0

# qhasm: a0 = mem64[input_0 +  0]
# asm 1: movq   0(<input_0=int64#1),>a0=int64#2
# asm 2: movq   0(<input_0=%rdi),>a0=%rsi
movq   0(%rdi),%rsi

# qhasm: int64 a1

# qhasm: a1 = mem64[input_0 +  8]
# asm 1: movq   8(<input_0=int64#1),>a1=int64#4
# asm 2: movq   8(<input_0=%rdi),>a1=%rcx
movq   8(%rdi),%rcx

# qhasm: int64 a2

# qhasm: a2 = mem64[input_0 + 16]
# asm 1: movq   16(<input_0=int64#1),>a2=int64#5
# asm 2: movq   16(<input_0=%rdi),>a2=%r8
movq   16(%rdi),%r8

# qhasm: int64 a3

# qhasm: a3 = mem64[input_0 + 24]
# asm 1: movq   24(<input_0=int64#1),>a3=int64#1
# asm 2: movq   24(<input_0=%rdi),>a3=%rdi
movq   24(%rdi),%rdi

# qhasm: t0 = a3
# asm 1: mov  <a3=int64#1,>t0=int64#6
# asm 2: mov  <a3=%rdi,>t0=%r9
mov  %rdi,%r9

# qhasm: (int64) t0 >>= 63
# asm 1: sar  $63,<t0=int64#6
# asm 2: sar  $63,<t0=%r9
sar  $63,%r9

# qhasm: a3 reset bit 63
# asm 1: btr  $63,<a3=int64#1
# asm 2: btr  $63,<a3=%rdi
btr  $63,%rdi

# qhasm: t0 &= 19
# asm 1: and  $19,<t0=int64#6
# asm 2: and  $19,<t0=%r9
and  $19,%r9

# qhasm: t0 += 19
# asm 1: add  $19,<t0=int64#6
# asm 2: add  $19,<t0=%r9
add  $19,%r9

# qhasm: carry? a0 += t0
# asm 1: add  <t0=int64#6,<a0=int64#2
# asm 2: add  <t0=%r9,<a0=%rsi
add  %r9,%rsi

# qhasm: carry? a1 += 0 + carry
# asm 1: adc $0,<a1=int64#4
# asm 2: adc $0,<a1=%rcx
adc $0,%rcx

# qhasm: carry? a2 += 0 + carry
# asm 1: adc $0,<a2=int64#5
# asm 2: adc $0,<a2=%r8
adc $0,%r8

# qhasm:        a3 += 0 + carry
# asm 1: adc $0,<a3=int64#1
# asm 2: adc $0,<a3=%rdi
adc $0,%rdi

# qhasm: t0 = a3
# asm 1: mov  <a3=int64#1,>t0=int64#6
# asm 2: mov  <a3=%rdi,>t0=%r9
mov  %rdi,%r9

# qhasm: (int64) t0 >>= 63
# asm 1: sar  $63,<t0=int64#6
# asm 2: sar  $63,<t0=%r9
sar  $63,%r9

# qhasm: a3 reset bit 63
# asm 1: btr  $63,<a3=int64#1
# asm 2: btr  $63,<a3=%rdi
btr  $63,%rdi

# qhasm: t0 &= 19
# asm 1: and  $19,<t0=int64#6
# asm 2: and  $19,<t0=%r9
and  $19,%r9

# qhasm: carry? a0 += t0
# asm 1: add  <t0=int64#6,<a0=int64#2
# asm 2: add  <t0=%r9,<a0=%rsi
add  %r9,%rsi

# qhasm: carry? a1 += 0 + carry
# asm 1: adc $0,<a1=int64#4
# asm 2: adc $0,<a1=%rcx
adc $0,%rcx

# qhasm: carry? a2 += 0 + carry
# asm 1: adc $0,<a2=int64#5
# asm 2: adc $0,<a2=%r8
adc $0,%r8

# qhasm:        a3 += 0 + carry
# asm 1: adc $0,<a3=int64#1
# asm 2: adc $0,<a3=%rdi
adc $0,%rdi

# qhasm: carry? a0 -= 19
# asm 1: sub  $19,<a0=int64#2
# asm 2: sub  $19,<a0=%rsi
sub  $19,%rsi

# qhasm: carry? a1 -= 0 - carry
# asm 1: sbb  $0,<a1=int64#4
# asm 2: sbb  $0,<a1=%rcx
sbb  $0,%rcx

# qhasm: carry? a2 -= 0 - carry
# asm 1: sbb  $0,<a2=int64#5
# asm 2: sbb  $0,<a2=%r8
sbb  $0,%r8

# qhasm:        a3 -= 0 - carry
# asm 1: sbb  $0,<a3=int64#1
# asm 2: sbb  $0,<a3=%rdi
sbb  $0,%rdi

# qhasm: t0 = -1152921504606846976
# asm 1: mov  $-1152921504606846976,>t0=int64#6
# asm 2: mov  $-1152921504606846976,>t0=%r9
mov  $-1152921504606846976,%r9

# qhasm: g = a0 & ~ t0
# asm 1: andn  <a0=int64#2,<t0=int64#6,>g=int64#6
# asm 2: andn  <a0=%rsi,<t0=%r9,>g=%r9
andn  %rsi,%r9,%r9

# qhasm: d1 = mem256[ input_2 + 288 ]
# asm 1: vmovupd   288(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   288(<input_2=%rdx),>d1=%ymm0
vmovupd   288(%rdx),%ymm0

# qhasm: stack_FVGS0 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS0=stack256#2
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS0=32(%rsp)
vmovapd %ymm0,32(%rsp)

# qhasm: t0 = a0
# asm 1: mov  <a0=int64#2,>t0=int64#7
# asm 2: mov  <a0=%rsi,>t0=%rax
mov  %rsi,%rax

# qhasm: t0 &= 1073741823
# asm 1: and  $1073741823,<t0=int64#7
# asm 2: and  $1073741823,<t0=%rax
and  $1073741823,%rax

# qhasm: inplace stack_FVGS0[2] = t0
# asm 1: movq <t0=int64#7,<stack_FVGS0=stack256#2
# asm 2: movq <t0=%rax,<stack_FVGS0=48(%rsp)
movq %rax,48(%rsp)

# qhasm: d1 = mem256[ input_2 + 320 ]
# asm 1: vmovupd   320(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   320(<input_2=%rdx),>d1=%ymm0
vmovupd   320(%rdx),%ymm0

# qhasm: stack_FVGS1 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS1=stack256#3
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS1=64(%rsp)
vmovapd %ymm0,64(%rsp)

# qhasm: t0 = a0
# asm 1: mov  <a0=int64#2,>t0=int64#7
# asm 2: mov  <a0=%rsi,>t0=%rax
mov  %rsi,%rax

# qhasm: (uint64) t0 >>= 30
# asm 1: shr  $30,<t0=int64#7
# asm 2: shr  $30,<t0=%rax
shr  $30,%rax

# qhasm: t0 &= 1073741823
# asm 1: and  $1073741823,<t0=int64#7
# asm 2: and  $1073741823,<t0=%rax
and  $1073741823,%rax

# qhasm: inplace stack_FVGS1[2] = t0
# asm 1: movq <t0=int64#7,<stack_FVGS1=stack256#3
# asm 2: movq <t0=%rax,<stack_FVGS1=80(%rsp)
movq %rax,80(%rsp)

# qhasm: d1 = mem256[ input_2 + 352 ]
# asm 1: vmovupd   352(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   352(<input_2=%rdx),>d1=%ymm0
vmovupd   352(%rdx),%ymm0

# qhasm: stack_FVGS2 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS2=stack256#4
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS2=96(%rsp)
vmovapd %ymm0,96(%rsp)

# qhasm: a0 = (a1 a0) >> 60
# asm 1: shrd $60,<a1=int64#4,<a0=int64#2
# asm 2: shrd $60,<a1=%rcx,<a0=%rsi
shrd $60,%rcx,%rsi

# qhasm: a0 &= 1073741823
# asm 1: and  $1073741823,<a0=int64#2
# asm 2: and  $1073741823,<a0=%rsi
and  $1073741823,%rsi

# qhasm: inplace stack_FVGS2[2] = a0
# asm 1: movq <a0=int64#2,<stack_FVGS2=stack256#4
# asm 2: movq <a0=%rsi,<stack_FVGS2=112(%rsp)
movq %rsi,112(%rsp)

# qhasm: d1 = mem256[ input_2 + 384 ]
# asm 1: vmovupd   384(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   384(<input_2=%rdx),>d1=%ymm0
vmovupd   384(%rdx),%ymm0

# qhasm: stack_FVGS3 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS3=stack256#5
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS3=128(%rsp)
vmovapd %ymm0,128(%rsp)

# qhasm: t0 = a1
# asm 1: mov  <a1=int64#4,>t0=int64#2
# asm 2: mov  <a1=%rcx,>t0=%rsi
mov  %rcx,%rsi

# qhasm: (uint64) t0 >>= 26
# asm 1: shr  $26,<t0=int64#2
# asm 2: shr  $26,<t0=%rsi
shr  $26,%rsi

# qhasm: t0 &= 1073741823
# asm 1: and  $1073741823,<t0=int64#2
# asm 2: and  $1073741823,<t0=%rsi
and  $1073741823,%rsi

# qhasm: inplace stack_FVGS3[2] = t0
# asm 1: movq <t0=int64#2,<stack_FVGS3=stack256#5
# asm 2: movq <t0=%rsi,<stack_FVGS3=144(%rsp)
movq %rsi,144(%rsp)

# qhasm: d1 = mem256[ input_2 + 416 ]
# asm 1: vmovupd   416(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   416(<input_2=%rdx),>d1=%ymm0
vmovupd   416(%rdx),%ymm0

# qhasm: stack_FVGS4 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS4=stack256#6
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS4=160(%rsp)
vmovapd %ymm0,160(%rsp)

# qhasm: a1 = (a2 a1) >> 56
# asm 1: shrd $56,<a2=int64#5,<a1=int64#4
# asm 2: shrd $56,<a2=%r8,<a1=%rcx
shrd $56,%r8,%rcx

# qhasm: a1 &= 1073741823
# asm 1: and  $1073741823,<a1=int64#4
# asm 2: and  $1073741823,<a1=%rcx
and  $1073741823,%rcx

# qhasm: inplace stack_FVGS4[2] = a1
# asm 1: movq <a1=int64#4,<stack_FVGS4=stack256#6
# asm 2: movq <a1=%rcx,<stack_FVGS4=176(%rsp)
movq %rcx,176(%rsp)

# qhasm: d1 = mem256[ input_2 + 448 ]
# asm 1: vmovupd   448(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   448(<input_2=%rdx),>d1=%ymm0
vmovupd   448(%rdx),%ymm0

# qhasm: stack_FVGS5 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS5=stack256#7
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS5=192(%rsp)
vmovapd %ymm0,192(%rsp)

# qhasm: t0 = a2
# asm 1: mov  <a2=int64#5,>t0=int64#2
# asm 2: mov  <a2=%r8,>t0=%rsi
mov  %r8,%rsi

# qhasm: (uint64) t0 >>= 22
# asm 1: shr  $22,<t0=int64#2
# asm 2: shr  $22,<t0=%rsi
shr  $22,%rsi

# qhasm: t0 &= 1073741823
# asm 1: and  $1073741823,<t0=int64#2
# asm 2: and  $1073741823,<t0=%rsi
and  $1073741823,%rsi

# qhasm: inplace stack_FVGS5[2] = t0
# asm 1: movq <t0=int64#2,<stack_FVGS5=stack256#7
# asm 2: movq <t0=%rsi,<stack_FVGS5=208(%rsp)
movq %rsi,208(%rsp)

# qhasm: d1 = mem256[ input_2 + 480 ]
# asm 1: vmovupd   480(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   480(<input_2=%rdx),>d1=%ymm0
vmovupd   480(%rdx),%ymm0

# qhasm: stack_FVGS6 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS6=stack256#8
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS6=224(%rsp)
vmovapd %ymm0,224(%rsp)

# qhasm: a2 = (a3 a2) >> 52
# asm 1: shrd $52,<a3=int64#1,<a2=int64#5
# asm 2: shrd $52,<a3=%rdi,<a2=%r8
shrd $52,%rdi,%r8

# qhasm: a2 &= 1073741823
# asm 1: and  $1073741823,<a2=int64#5
# asm 2: and  $1073741823,<a2=%r8
and  $1073741823,%r8

# qhasm: inplace stack_FVGS6[2] = a2
# asm 1: movq <a2=int64#5,<stack_FVGS6=stack256#8
# asm 2: movq <a2=%r8,<stack_FVGS6=240(%rsp)
movq %r8,240(%rsp)

# qhasm: d1 = mem256[ input_2 + 512 ]
# asm 1: vmovupd   512(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   512(<input_2=%rdx),>d1=%ymm0
vmovupd   512(%rdx),%ymm0

# qhasm: stack_FVGS7 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS7=stack256#9
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS7=256(%rsp)
vmovapd %ymm0,256(%rsp)

# qhasm: t0 = a3
# asm 1: mov  <a3=int64#1,>t0=int64#2
# asm 2: mov  <a3=%rdi,>t0=%rsi
mov  %rdi,%rsi

# qhasm: (uint64) t0 >>= 18
# asm 1: shr  $18,<t0=int64#2
# asm 2: shr  $18,<t0=%rsi
shr  $18,%rsi

# qhasm: t0 &= 1073741823
# asm 1: and  $1073741823,<t0=int64#2
# asm 2: and  $1073741823,<t0=%rsi
and  $1073741823,%rsi

# qhasm: inplace stack_FVGS7[2] = t0
# asm 1: movq <t0=int64#2,<stack_FVGS7=stack256#9
# asm 2: movq <t0=%rsi,<stack_FVGS7=272(%rsp)
movq %rsi,272(%rsp)

# qhasm: d1 = mem256[ input_2 + 544]
# asm 1: vmovupd   544(<input_2=int64#3),>d1=reg256#1
# asm 2: vmovupd   544(<input_2=%rdx),>d1=%ymm0
vmovupd   544(%rdx),%ymm0

# qhasm: stack_FVGS8 = d1
# asm 1: vmovapd <d1=reg256#1,>stack_FVGS8=stack256#10
# asm 2: vmovapd <d1=%ymm0,>stack_FVGS8=288(%rsp)
vmovapd %ymm0,288(%rsp)

# qhasm: (uint64) a3 >>= 48
# asm 1: shr  $48,<a3=int64#1
# asm 2: shr  $48,<a3=%rdi
shr  $48,%rdi

# qhasm: inplace stack_FVGS8[2] = a3
# asm 1: movq <a3=int64#1,<stack_FVGS8=stack256#10
# asm 2: movq <a3=%rdi,<stack_FVGS8=304(%rsp)
movq %rdi,304(%rsp)

# qhasm: f = -19
# asm 1: mov  $-19,>f=int64#1
# asm 2: mov  $-19,>f=%rdi
mov  $-19,%rdi

# qhasm:                   m = 0
# asm 1: xor  >m=int64#2,>m=int64#2
# asm 2: xor  >m=%rsi,>m=%rsi
xor  %rsi,%rsi

# qhasm:                   z = -1
# asm 1: mov  $-1,>z=int64#4
# asm 2: mov  $-1,>z=%rcx
mov  $-1,%rcx

# qhasm: inplace stack_m1[0] = m
# asm 1: movq <m=int64#2,<stack_m1=stack256#1
# asm 2: movq <m=%rsi,<stack_m1=0(%rsp)
movq %rsi,0(%rsp)

# qhasm: inplace stack_m1[1] = z
# asm 1: movq <z=int64#4,<stack_m1=stack256#1
# asm 2: movq <z=%rcx,<stack_m1=8(%rsp)
movq %rcx,8(%rsp)

# qhasm:              _m2p62 = -4611686018427387904
# asm 1: mov  $-4611686018427387904,>_m2p62=int64#4
# asm 2: mov  $-4611686018427387904,>_m2p62=%rcx
mov  $-4611686018427387904,%rcx

# qhasm:         stack_m2p62 = _m2p62
# asm 1: movq <_m2p62=int64#4,>stack_m2p62=stack64#9
# asm 2: movq <_m2p62=%rcx,>stack_m2p62=768(%rsp)
movq %rcx,768(%rsp)

# qhasm:               _2p20 = 1048576
# asm 1: mov  $1048576,>_2p20=int64#4
# asm 2: mov  $1048576,>_2p20=%rcx
mov  $1048576,%rcx

# qhasm:          stack_2p20 = _2p20
# asm 1: movq <_2p20=int64#4,>stack_2p20=stack64#10
# asm 2: movq <_2p20=%rcx,>stack_2p20=776(%rsp)
movq %rcx,776(%rsp)

# qhasm:              _m2p41 = -2199023255552
# asm 1: mov  $-2199023255552,>_m2p41=int64#4
# asm 2: mov  $-2199023255552,>_m2p41=%rcx
mov  $-2199023255552,%rcx

# qhasm:         stack_m2p41 = _m2p41
# asm 1: movq <_m2p41=int64#4,>stack_m2p41=stack64#11
# asm 2: movq <_m2p41=%rcx,>stack_m2p41=784(%rsp)
movq %rcx,784(%rsp)

# qhasm:              _m2p20 = -1048576
# asm 1: mov  $-1048576,>_m2p20=int64#4
# asm 2: mov  $-1048576,>_m2p20=%rcx
mov  $-1048576,%rcx

# qhasm:         stack_m2p20 = _m2p20
# asm 1: movq <_m2p20=int64#4,>stack_m2p20=stack64#12
# asm 2: movq <_m2p20=%rcx,>stack_m2p20=792(%rsp)
movq %rcx,792(%rsp)

# qhasm:          _2p20a2p41 = 2199024304128
# asm 1: mov  $2199024304128,>_2p20a2p41=int64#5
# asm 2: mov  $2199024304128,>_2p20a2p41=%r8
mov  $2199024304128,%r8

# qhasm:     stack_2p20a2p41 = _2p20a2p41
# asm 1: movq <_2p20a2p41=int64#5,>stack_2p20a2p41=stack64#13
# asm 2: movq <_2p20a2p41=%r8,>stack_2p20a2p41=800(%rsp)
movq %r8,800(%rsp)

# qhasm:               _19x4 = mem256[ input_2 +   0 ]
# asm 1: vmovupd   0(<input_2=int64#3),>_19x4=reg256#1
# asm 2: vmovupd   0(<input_2=%rdx),>_19x4=%ymm0
vmovupd   0(%rdx),%ymm0

# qhasm:          stack_19x4 = _19x4
# asm 1: vmovapd <_19x4=reg256#1,>stack_19x4=stack256#11
# asm 2: vmovapd <_19x4=%ymm0,>stack_19x4=320(%rsp)
vmovapd %ymm0,320(%rsp)

# qhasm:             _0_19x4 = mem256[ input_2 +  32 ]
# asm 1: vmovupd   32(<input_2=int64#3),>_0_19x4=reg256#1
# asm 2: vmovupd   32(<input_2=%rdx),>_0_19x4=%ymm0
vmovupd   32(%rdx),%ymm0

# qhasm:        stack_0_19x4 = _0_19x4
# asm 1: vmovapd <_0_19x4=reg256#1,>stack_0_19x4=stack256#12
# asm 2: vmovapd <_0_19x4=%ymm0,>stack_0_19x4=352(%rsp)
vmovapd %ymm0,352(%rsp)

# qhasm:            _32767x4 = mem256[ input_2 +  64 ]
# asm 1: vmovupd   64(<input_2=int64#3),>_32767x4=reg256#1
# asm 2: vmovupd   64(<input_2=%rdx),>_32767x4=%ymm0
vmovupd   64(%rdx),%ymm0

# qhasm:       stack_32767x4 = _32767x4
# asm 1: vmovapd <_32767x4=reg256#1,>stack_32767x4=stack256#13
# asm 2: vmovapd <_32767x4=%ymm0,>stack_32767x4=384(%rsp)
vmovapd %ymm0,384(%rsp)

# qhasm:       _inv19_2p30x4 = mem256[ input_2 +  96 ]
# asm 1: vmovupd   96(<input_2=int64#3),>_inv19_2p30x4=reg256#1
# asm 2: vmovupd   96(<input_2=%rdx),>_inv19_2p30x4=%ymm0
vmovupd   96(%rdx),%ymm0

# qhasm:  stack_inv19_2p30x4 = _inv19_2p30x4
# asm 1: vmovapd <_inv19_2p30x4=reg256#1,>stack_inv19_2p30x4=stack256#14
# asm 2: vmovapd <_inv19_2p30x4=%ymm0,>stack_inv19_2p30x4=416(%rsp)
vmovapd %ymm0,416(%rsp)

# qhasm:           _2p30m1x4 = mem256[ input_2 + 128 ]
# asm 1: vmovupd   128(<input_2=int64#3),>_2p30m1x4=reg256#1
# asm 2: vmovupd   128(<input_2=%rdx),>_2p30m1x4=%ymm0
vmovupd   128(%rdx),%ymm0

# qhasm:      stack_2p30m1x4 = _2p30m1x4
# asm 1: vmovapd <_2p30m1x4=reg256#1,>stack_2p30m1x4=stack256#15
# asm 2: vmovapd <_2p30m1x4=%ymm0,>stack_2p30m1x4=448(%rsp)
vmovapd %ymm0,448(%rsp)

# qhasm:             _2p33x4 = mem256[ input_2 + 160 ]
# asm 1: vmovupd   160(<input_2=int64#3),>_2p33x4=reg256#1
# asm 2: vmovupd   160(<input_2=%rdx),>_2p33x4=%ymm0
vmovupd   160(%rdx),%ymm0

# qhasm:        stack_2p33x4 = _2p33x4
# asm 1: vmovapd <_2p33x4=reg256#1,>stack_2p33x4=stack256#16
# asm 2: vmovapd <_2p33x4=%ymm0,>stack_2p33x4=480(%rsp)
vmovapd %ymm0,480(%rsp)

# qhasm:             _2p48x4 = mem256[ input_2 + 192 ]
# asm 1: vmovupd   192(<input_2=int64#3),>_2p48x4=reg256#1
# asm 2: vmovupd   192(<input_2=%rdx),>_2p48x4=%ymm0
vmovupd   192(%rdx),%ymm0

# qhasm:        stack_2p48x4 = _2p48x4
# asm 1: vmovapd <_2p48x4=reg256#1,>stack_2p48x4=stack256#17
# asm 2: vmovapd <_2p48x4=%ymm0,>stack_2p48x4=512(%rsp)
vmovapd %ymm0,512(%rsp)

# qhasm:             _2p63x4 = mem256[ input_2 + 224 ]
# asm 1: vmovupd   224(<input_2=int64#3),>_2p63x4=reg256#1
# asm 2: vmovupd   224(<input_2=%rdx),>_2p63x4=%ymm0
vmovupd   224(%rdx),%ymm0

# qhasm:        stack_2p63x4 = _2p63x4
# asm 1: vmovapd <_2p63x4=reg256#1,>stack_2p63x4=stack256#18
# asm 2: vmovapd <_2p63x4=%ymm0,>stack_2p63x4=544(%rsp)
vmovapd %ymm0,544(%rsp)

# qhasm:        _2p63m2p33x4 = mem256[ input_2 + 256 ]
# asm 1: vmovupd   256(<input_2=int64#3),>_2p63m2p33x4=reg256#1
# asm 2: vmovupd   256(<input_2=%rdx),>_2p63m2p33x4=%ymm0
vmovupd   256(%rdx),%ymm0

# qhasm: i = 10
# asm 1: mov  $10,>i=int64#5
# asm 2: mov  $10,>i=%r8
mov  $10,%r8

# qhasm: u = 1152921504606846976
# asm 1: mov  $1152921504606846976,>u=int64#8
# asm 2: mov  $1152921504606846976,>u=%r10
mov  $1152921504606846976,%r10

# qhasm: v = 0
# asm 1: xor  >v=int64#9,>v=int64#9
# asm 2: xor  >v=%r11,>v=%r11
xor  %r11,%r11

# qhasm: s = u
# asm 1: mov  <u=int64#8,>s=int64#10
# asm 2: mov  <u=%r10,>s=%r12
mov  %r10,%r12

# qhasm: r = 0
# asm 1: xor  >r=int64#11,>r=int64#11
# asm 2: xor  >r=%r13,>r=%r13
xor  %r13,%r13

# qhasm: nop
nop

# qhasm: nop
nop

# qhasm: nop
nop

# qhasm: nop
nop

# qhasm: bigloop:
._bigloop:

# qhasm:       rax = g
# asm 1: mov  <g=int64#6,>rax=int64#7
# asm 2: mov  <g=%r9,>rax=%rax
mov  %r9,%rax

# qhasm:       (int128) rdx rax = rax * s
# asm 1: imul <s=int64#10
# asm 2: imul <s=%r12
imul %r12

# qhasm:       t2 = rax
# asm 1: mov  <rax=int64#7,>t2=int64#14
# asm 2: mov  <rax=%rax,>t2=%rbx
mov  %rax,%rbx

# qhasm:       t1 = rdx
# asm 1: mov  <rdx=int64#3,>t1=int64#15
# asm 2: mov  <rdx=%rdx,>t1=%rbp
mov  %rdx,%rbp

# qhasm:       rax = f
# asm 1: mov  <f=int64#1,>rax=int64#7
# asm 2: mov  <f=%rdi,>rax=%rax
mov  %rdi,%rax

# qhasm:       (int128) rdx rax = rax * r
# asm 1: imul <r=int64#11
# asm 2: imul <r=%r13
imul %r13

# qhasm:       carry? t2 += rax
# asm 1: add  <rax=int64#7,<t2=int64#14
# asm 2: add  <rax=%rax,<t2=%rbx
add  %rax,%rbx

# qhasm:              t1 += rdx + carry
# asm 1: adc <rdx=int64#3,<t1=int64#15
# asm 2: adc <rdx=%rdx,<t1=%rbp
adc %rdx,%rbp

# qhasm:       t2 = (t1 t2) >> 60	 
# asm 1: shrd $60,<t1=int64#15,<t2=int64#14
# asm 2: shrd $60,<t1=%rbp,<t2=%rbx
shrd $60,%rbp,%rbx

# qhasm:       rax = f
# asm 1: mov  <f=int64#1,>rax=int64#7
# asm 2: mov  <f=%rdi,>rax=%rax
mov  %rdi,%rax

# qhasm:       (int128) rdx rax = rax * u
# asm 1: imul <u=int64#8
# asm 2: imul <u=%r10
imul %r10

# qhasm:       f = rax
# asm 1: mov  <rax=int64#7,>f=int64#1
# asm 2: mov  <rax=%rax,>f=%rdi
mov  %rax,%rdi

# qhasm:       t0 = rdx
# asm 1: mov  <rdx=int64#3,>t0=int64#15
# asm 2: mov  <rdx=%rdx,>t0=%rbp
mov  %rdx,%rbp

# qhasm:       rax = g
# asm 1: mov  <g=int64#6,>rax=int64#7
# asm 2: mov  <g=%r9,>rax=%rax
mov  %r9,%rax

# qhasm:       (int128) rdx rax = rax * v
# asm 1: imul <v=int64#9
# asm 2: imul <v=%r11
imul %r11

# qhasm:       carry? f += rax
# asm 1: add  <rax=int64#7,<f=int64#1
# asm 2: add  <rax=%rax,<f=%rdi
add  %rax,%rdi

# qhasm:              t0 += rdx + carry
# asm 1: adc <rdx=int64#3,<t0=int64#15
# asm 2: adc <rdx=%rdx,<t0=%rbp
adc %rdx,%rbp

# qhasm:       f = (t0 f) >> 60
# asm 1: shrd $60,<t0=int64#15,<f=int64#1
# asm 2: shrd $60,<t0=%rbp,<f=%rdi
shrd $60,%rbp,%rdi

# qhasm: new vvrr

# qhasm: vvrr = v,vvrr[1],0,0
# asm 1: vpinsrq $0x0,<v=int64#9,<vvrr=reg256#2%128,<vvrr=reg256#2%128
# asm 2: vpinsrq $0x0,<v=%r11,<vvrr=%xmm1,<vvrr=%xmm1
vpinsrq $0x0,%r11,%xmm1,%xmm1

# qhasm: vvrr = vvrr[0],r,0,0
# asm 1: vpinsrq $0x1,<r=int64#11,<vvrr=reg256#2%128,<vvrr=reg256#2%128
# asm 2: vpinsrq $0x1,<r=%r13,<vvrr=%xmm1,<vvrr=%xmm1
vpinsrq $0x1,%r13,%xmm1,%xmm1

# qhasm:       v *= g0
# asm 1: imul  <g0=int64#13,<v=int64#9
# asm 2: imul  <g0=%r15,<v=%r11
imul  %r15,%r11

# qhasm:       g0 *= s
# asm 1: imul  <s=int64#10,<g0=int64#13
# asm 2: imul  <s=%r12,<g0=%r15
imul  %r12,%r15

# qhasm:       r *= f0
# asm 1: imul  <f0=int64#12,<r=int64#11
# asm 2: imul  <f0=%r14,<r=%r13
imul  %r14,%r13

# qhasm:       f0 *= u
# asm 1: imul  <u=int64#8,<f0=int64#12
# asm 2: imul  <u=%r10,<f0=%r14
imul  %r10,%r14

# qhasm:       f0 += v
# asm 1: add  <v=int64#9,<f0=int64#12
# asm 2: add  <v=%r11,<f0=%r14
add  %r11,%r14

# qhasm:       g0 += r
# asm 1: add  <r=int64#11,<g0=int64#13
# asm 2: add  <r=%r13,<g0=%r15
add  %r13,%r15

# qhasm:       f += f0
# asm 1: add  <f0=int64#12,<f=int64#1
# asm 2: add  <f0=%r14,<f=%rdi
add  %r14,%rdi

# qhasm:       g = t2+g0
# asm 1: lea  (<t2=int64#14,<g0=int64#13),>g=int64#3
# asm 2: lea  (<t2=%rbx,<g0=%r15),>g=%rdx
lea  (%rbx,%r15),%rdx

# qhasm: FVGS0 = stack_FVGS0
# asm 1: vmovapd <stack_FVGS0=stack256#2,>FVGS0=reg256#3
# asm 2: vmovapd <stack_FVGS0=32(%rsp),>FVGS0=%ymm2
vmovapd 32(%rsp),%ymm2

# qhasm:   fuv = f & ~ _m2p20
# asm 1: andn  <f=int64#1,<_m2p20=int64#4,>fuv=int64#6
# asm 2: andn  <f=%rdi,<_m2p20=%rcx,>fuv=%r9
andn  %rdi,%rcx,%r9

# qhasm: loop20_init:
._loop20_init:

# qhasm: new uuss

# qhasm:   grs = g & ~ _m2p20
# asm 1: andn  <g=int64#3,<_m2p20=int64#4,>grs=int64#7
# asm 2: andn  <g=%rdx,<_m2p20=%rcx,>grs=%rax
andn  %rdx,%rcx,%rax

# qhasm: uuss = u,uuss[1],0,0
# asm 1: vpinsrq $0x0,<u=int64#8,<uuss=reg256#4%128,<uuss=reg256#4%128
# asm 2: vpinsrq $0x0,<u=%r10,<uuss=%xmm3,<uuss=%xmm3
vpinsrq $0x0,%r10,%xmm3,%xmm3

# qhasm:   fuv += stack_m2p41
# asm 1: addq <stack_m2p41=stack64#11,<fuv=int64#6
# asm 2: addq <stack_m2p41=784(%rsp),<fuv=%r9
addq 784(%rsp),%r9

# qhasm:   grs += stack_m2p62
# asm 1: addq <stack_m2p62=stack64#9,<grs=int64#7
# asm 2: addq <stack_m2p62=768(%rsp),<grs=%rax
addq 768(%rsp),%rax

# qhasm: uuss = uuss[0],s,0,0
# asm 1: vpinsrq $0x1,<s=int64#10,<uuss=reg256#4%128,<uuss=reg256#4%128
# asm 2: vpinsrq $0x1,<s=%r12,<uuss=%xmm3,<uuss=%xmm3
vpinsrq $0x1,%r12,%xmm3,%xmm3

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: GSFV0 = FVGS0[1,0]
# asm 1: vpermq $0x4e,<FVGS0=reg256#3,>GSFV0=reg256#5
# asm 2: vpermq $0x4e,<FVGS0=%ymm2,>GSFV0=%ymm4
vpermq $0x4e,%ymm2,%ymm4

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm: uuss = uuss[0,0,1,1]
# asm 1: vpermq $0x50,<uuss=reg256#4,>uuss=reg256#4
# asm 2: vpermq $0x50,<uuss=%ymm3,>uuss=%ymm3
vpermq $0x50,%ymm3,%ymm3

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: vvrr = vvrr[0,0,1,1]
# asm 1: vpermq $0x50,<vvrr=reg256#2,>vvrr=reg256#2
# asm 2: vpermq $0x50,<vvrr=%ymm1,>vvrr=%ymm1
vpermq $0x50,%ymm1,%ymm1

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: _2p30m1x4 = stack_2p30m1x4
# asm 1: vmovapd <stack_2p30m1x4=stack256#15,>_2p30m1x4=reg256#6
# asm 2: vmovapd <stack_2p30m1x4=448(%rsp),>_2p30m1x4=%ymm5
vmovapd 448(%rsp),%ymm5

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm: uuss0 = uuss & _2p30m1x4
# asm 1: vpand <uuss=reg256#4,<_2p30m1x4=reg256#6,>uuss0=reg256#7
# asm 2: vpand <uuss=%ymm3,<_2p30m1x4=%ymm5,>uuss0=%ymm6
vpand %ymm3,%ymm5,%ymm6

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: vvrr0 = vvrr & _2p30m1x4
# asm 1: vpand <vvrr=reg256#2,<_2p30m1x4=reg256#6,>vvrr0=reg256#8
# asm 2: vpand <vvrr=%ymm1,<_2p30m1x4=%ymm5,>vvrr0=%ymm7
vpand %ymm1,%ymm5,%ymm7

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: 4x ta = int32 uuss0 * int32 FVGS0
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS0=reg256#3,>ta=reg256#9
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS0=%ymm2,>ta=%ymm8
vpmuldq %ymm6,%ymm2,%ymm8

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#8
# asm 2: mov  $-1,>z=%r10
mov  $-1,%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV0
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV0=reg256#5,>tb=reg256#10
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV0=%ymm4,>tb=%ymm9
vpmuldq %ymm7,%ymm4,%ymm9

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x out0 = ta + tb
# asm 1: vpaddq <ta=reg256#9,<tb=reg256#10,>out0=reg256#9
# asm 2: vpaddq <ta=%ymm8,<tb=%ymm9,>out0=%ymm8
vpaddq %ymm8,%ymm9,%ymm8

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: _inv19_2p30x4 = stack_inv19_2p30x4
# asm 1: vmovapd <stack_inv19_2p30x4=stack256#14,>_inv19_2p30x4=reg256#10
# asm 2: vmovapd <stack_inv19_2p30x4=416(%rsp),>_inv19_2p30x4=%ymm9
vmovapd 416(%rsp),%ymm9

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: _19x4 = stack_19x4
# asm 1: vmovapd <stack_19x4=stack256#11,>_19x4=reg256#11
# asm 2: vmovapd <stack_19x4=320(%rsp),>_19x4=%ymm10
vmovapd 320(%rsp),%ymm10

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: 4x d0 = int32 out0 * int32 _inv19_2p30x4
# asm 1: vpmuldq <out0=reg256#9,<_inv19_2p30x4=reg256#10,>d0=reg256#12
# asm 2: vpmuldq <out0=%ymm8,<_inv19_2p30x4=%ymm9,>d0=%ymm11
vpmuldq %ymm8,%ymm9,%ymm11

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: d0 &= _2p30m1x4
# asm 1: vpand <d0=reg256#12,<_2p30m1x4=reg256#6,<d0=reg256#12
# asm 2: vpand <d0=%ymm11,<_2p30m1x4=%ymm5,<d0=%ymm11
vpand %ymm11,%ymm5,%ymm11

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x d0x19 = int32 d0 * int32 _19x4
# asm 1: vpmuldq <d0=reg256#12,<_19x4=reg256#11,>d0x19=reg256#13
# asm 2: vpmuldq <d0=%ymm11,<_19x4=%ymm10,>d0x19=%ymm12
vpmuldq %ymm11,%ymm10,%ymm12

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x out0 -= d0x19
# asm 1: vpsubq <d0x19=reg256#13,<out0=reg256#9,<out0=reg256#9
# asm 2: vpsubq <d0x19=%ymm12,<out0=%ymm8,<out0=%ymm8
vpsubq %ymm12,%ymm8,%ymm8

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: 4x carryy = out0 + stack_2p63x4
# asm 1: vpaddq <stack_2p63x4=stack256#18,<out0=reg256#9,>carryy=reg256#9
# asm 2: vpaddq <stack_2p63x4=544(%rsp),<out0=%ymm8,>carryy=%ymm8
vpaddq 544(%rsp),%ymm8,%ymm8

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x carryy unsigned>>= 30
# asm 1: vpsrlq $30,<carryy=reg256#9,<carryy=reg256#9
# asm 2: vpsrlq $30,<carryy=%ymm8,<carryy=%ymm8
vpsrlq $30,%ymm8,%ymm8

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: 4x out8plus2 = d0 << 15
# asm 1: vpsllq $15,<d0=reg256#12,>out8plus2=reg256#12
# asm 2: vpsllq $15,<d0=%ymm11,>out8plus2=%ymm11
vpsllq $15,%ymm11,%ymm11

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: FVGS1 = stack_FVGS1
# asm 1: vmovapd <stack_FVGS1=stack256#3,>FVGS1=reg256#13
# asm 2: vmovapd <stack_FVGS1=64(%rsp),>FVGS1=%ymm12
vmovapd 64(%rsp),%ymm12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm: GSFV1 = FVGS1[1,0]
# asm 1: vpermq $0x4e,<FVGS1=reg256#13,>GSFV1=reg256#14
# asm 2: vpermq $0x4e,<FVGS1=%ymm12,>GSFV1=%ymm13
vpermq $0x4e,%ymm12,%ymm13

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x ta = int32 uuss0 * int32 FVGS1
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS1=reg256#13,>ta=reg256#15
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS1=%ymm12,>ta=%ymm14
vpmuldq %ymm6,%ymm12,%ymm14

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV1
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV1=reg256#14,>tb=reg256#16
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV1=%ymm13,>tb=%ymm15
vpmuldq %ymm7,%ymm13,%ymm15

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#8
# asm 2: mov  $-1,>z=%r10
mov  $-1,%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: 4x out1plus = ta + tb
# asm 1: vpaddq <ta=reg256#15,<tb=reg256#16,>out1plus=reg256#15
# asm 2: vpaddq <ta=%ymm14,<tb=%ymm15,>out1plus=%ymm14
vpaddq %ymm14,%ymm15,%ymm14

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x uuss1 = uuss unsigned>> 30
# asm 1: vpsrlq $30,<uuss=reg256#4,>uuss1=reg256#4
# asm 2: vpsrlq $30,<uuss=%ymm3,>uuss1=%ymm3
vpsrlq $30,%ymm3,%ymm3

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm: 4x vvrr1 = vvrr unsigned>> 30
# asm 1: vpsrlq $30,<vvrr=reg256#2,>vvrr1=reg256#2
# asm 2: vpsrlq $30,<vvrr=%ymm1,>vvrr1=%ymm1
vpsrlq $30,%ymm1,%ymm1

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x ta = int32 uuss1 * int32 FVGS0
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS0=reg256#3,>ta=reg256#3
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS0=%ymm2,>ta=%ymm2
vpmuldq %ymm3,%ymm2,%ymm2

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV0
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV0=reg256#5,>tb=reg256#5
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV0=%ymm4,>tb=%ymm4
vpmuldq %ymm1,%ymm4,%ymm4

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: 4x out1 = ta + tb
# asm 1: vpaddq <ta=reg256#3,<tb=reg256#5,>out1=reg256#3
# asm 2: vpaddq <ta=%ymm2,<tb=%ymm4,>out1=%ymm2
vpaddq %ymm2,%ymm4,%ymm2

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x out1 += out1plus
# asm 1: vpaddq <out1=reg256#3,<out1plus=reg256#15,<out1=reg256#3
# asm 2: vpaddq <out1=%ymm2,<out1plus=%ymm14,<out1=%ymm2
vpaddq %ymm2,%ymm14,%ymm2

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#8
# asm 2: mov  $-1,>z=%r10
mov  $-1,%r10

# qhasm: 4x out1 += carryy
# asm 1: vpaddq <out1=reg256#3,<carryy=reg256#9,<out1=reg256#3
# asm 2: vpaddq <out1=%ymm2,<carryy=%ymm8,<out1=%ymm2
vpaddq %ymm2,%ymm8,%ymm2

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: 4x d1 = int32 out1 * int32 _inv19_2p30x4
# asm 1: vpmuldq <out1=reg256#3,<_inv19_2p30x4=reg256#10,>d1=reg256#5
# asm 2: vpmuldq <out1=%ymm2,<_inv19_2p30x4=%ymm9,>d1=%ymm4
vpmuldq %ymm2,%ymm9,%ymm4

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: d1 &= _2p30m1x4
# asm 1: vpand <d1=reg256#5,<_2p30m1x4=reg256#6,<d1=reg256#5
# asm 2: vpand <d1=%ymm4,<_2p30m1x4=%ymm5,<d1=%ymm4
vpand %ymm4,%ymm5,%ymm4

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: 4x d1x19 = int32 d1 * int32 _19x4
# asm 1: vpmuldq <d1=reg256#5,<_19x4=reg256#11,>d1x19=reg256#9
# asm 2: vpmuldq <d1=%ymm4,<_19x4=%ymm10,>d1x19=%ymm8
vpmuldq %ymm4,%ymm10,%ymm8

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: 4x out1 -= d1x19
# asm 1: vpsubq <d1x19=reg256#9,<out1=reg256#3,<out1=reg256#3
# asm 2: vpsubq <d1x19=%ymm8,<out1=%ymm2,<out1=%ymm2
vpsubq %ymm8,%ymm2,%ymm2

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm: 4x carryy = out1 + _2p63m2p33x4 
# asm 1: vpaddq <out1=reg256#3,<_2p63m2p33x4=reg256#1,>carryy=reg256#3
# asm 2: vpaddq <out1=%ymm2,<_2p63m2p33x4=%ymm0,>carryy=%ymm2
vpaddq %ymm2,%ymm0,%ymm2

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: 4x carryy unsigned>>= 30
# asm 1: vpsrlq $30,<carryy=reg256#3,<carryy=reg256#3
# asm 2: vpsrlq $30,<carryy=%ymm2,<carryy=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x out9 = d1 << 15
# asm 1: vpsllq $15,<d1=reg256#5,>out9=reg256#5
# asm 2: vpsllq $15,<d1=%ymm4,>out9=%ymm4
vpsllq $15,%ymm4,%ymm4

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: FVGS2 = stack_FVGS2 
# asm 1: vmovapd <stack_FVGS2=stack256#4,>FVGS2=reg256#9
# asm 2: vmovapd <stack_FVGS2=96(%rsp),>FVGS2=%ymm8
vmovapd 96(%rsp),%ymm8

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: GSFV2 = FVGS2[1,0]
# asm 1: vpermq $0x4e,<FVGS2=reg256#9,>GSFV2=reg256#10
# asm 2: vpermq $0x4e,<FVGS2=%ymm8,>GSFV2=%ymm9
vpermq $0x4e,%ymm8,%ymm9

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x ta = int32 uuss1 * int32 FVGS1
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS1=reg256#13,>ta=reg256#11
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS1=%ymm12,>ta=%ymm10
vpmuldq %ymm3,%ymm12,%ymm10

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV1
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV1=reg256#14,>tb=reg256#13
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV1=%ymm13,>tb=%ymm12
vpmuldq %ymm1,%ymm13,%ymm12

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: 4x out2plus = ta + tb
# asm 1: vpaddq <ta=reg256#11,<tb=reg256#13,>out2plus=reg256#11
# asm 2: vpaddq <ta=%ymm10,<tb=%ymm12,>out2plus=%ymm10
vpaddq %ymm10,%ymm12,%ymm10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: 4x ta = int32 uuss0 * int32 FVGS2
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS2=reg256#9,>ta=reg256#13
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS2=%ymm8,>ta=%ymm12
vpmuldq %ymm6,%ymm8,%ymm12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV2
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV2=reg256#10,>tb=reg256#14
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV2=%ymm9,>tb=%ymm13
vpmuldq %ymm7,%ymm9,%ymm13

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x out2 = ta + tb
# asm 1: vpaddq <ta=reg256#13,<tb=reg256#14,>out2=reg256#13
# asm 2: vpaddq <ta=%ymm12,<tb=%ymm13,>out2=%ymm12
vpaddq %ymm12,%ymm13,%ymm12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: 4x out2 += out2plus
# asm 1: vpaddq <out2=reg256#13,<out2plus=reg256#11,<out2=reg256#13
# asm 2: vpaddq <out2=%ymm12,<out2plus=%ymm10,<out2=%ymm12
vpaddq %ymm12,%ymm10,%ymm12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm: 4x out2 += carryy
# asm 1: vpaddq <out2=reg256#13,<carryy=reg256#3,<out2=reg256#13
# asm 2: vpaddq <out2=%ymm12,<carryy=%ymm2,<out2=%ymm12
vpaddq %ymm12,%ymm2,%ymm12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: 4x carryy = out2 + _2p63m2p33x4 
# asm 1: vpaddq <out2=reg256#13,<_2p63m2p33x4=reg256#1,>carryy=reg256#3
# asm 2: vpaddq <out2=%ymm12,<_2p63m2p33x4=%ymm0,>carryy=%ymm2
vpaddq %ymm12,%ymm0,%ymm2

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: 4x carryy unsigned>>= 30
# asm 1: vpsrlq $30,<carryy=reg256#3,<carryy=reg256#3
# asm 2: vpsrlq $30,<carryy=%ymm2,<carryy=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: FVGS3 = stack_FVGS3 
# asm 1: vmovapd <stack_FVGS3=stack256#5,>FVGS3=reg256#11
# asm 2: vmovapd <stack_FVGS3=128(%rsp),>FVGS3=%ymm10
vmovapd 128(%rsp),%ymm10

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: GSFV3 = FVGS3[1,0]
# asm 1: vpermq $0x4e,<FVGS3=reg256#11,>GSFV3=reg256#14
# asm 2: vpermq $0x4e,<FVGS3=%ymm10,>GSFV3=%ymm13
vpermq $0x4e,%ymm10,%ymm13

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: 4x ta = int32 uuss1 * int32 FVGS2
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS2=reg256#9,>ta=reg256#9
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS2=%ymm8,>ta=%ymm8
vpmuldq %ymm3,%ymm8,%ymm8

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV2
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV2=reg256#10,>tb=reg256#10
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV2=%ymm9,>tb=%ymm9
vpmuldq %ymm1,%ymm9,%ymm9

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x out3plus = ta + tb
# asm 1: vpaddq <ta=reg256#9,<tb=reg256#10,>out3plus=reg256#9
# asm 2: vpaddq <ta=%ymm8,<tb=%ymm9,>out3plus=%ymm8
vpaddq %ymm8,%ymm9,%ymm8

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x ta = int32 uuss0 * int32 FVGS3
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS3=reg256#11,>ta=reg256#10
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS3=%ymm10,>ta=%ymm9
vpmuldq %ymm6,%ymm10,%ymm9

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV3
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV3=reg256#14,>tb=reg256#15
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV3=%ymm13,>tb=%ymm14
vpmuldq %ymm7,%ymm13,%ymm14

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: 4x out3 = ta + tb
# asm 1: vpaddq <ta=reg256#10,<tb=reg256#15,>out3=reg256#10
# asm 2: vpaddq <ta=%ymm9,<tb=%ymm14,>out3=%ymm9
vpaddq %ymm9,%ymm14,%ymm9

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: 4x out3 += out3plus
# asm 1: vpaddq <out3=reg256#10,<out3plus=reg256#9,<out3=reg256#10
# asm 2: vpaddq <out3=%ymm9,<out3plus=%ymm8,<out3=%ymm9
vpaddq %ymm9,%ymm8,%ymm9

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: 4x out3 += carryy
# asm 1: vpaddq <out3=reg256#10,<carryy=reg256#3,<out3=reg256#10
# asm 2: vpaddq <out3=%ymm9,<carryy=%ymm2,<out3=%ymm9
vpaddq %ymm9,%ymm2,%ymm9

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#8
# asm 2: mov  $-1,>z=%r10
mov  $-1,%r10

# qhasm: out2 &= _2p30m1x4
# asm 1: vpand <out2=reg256#13,<_2p30m1x4=reg256#6,<out2=reg256#13
# asm 2: vpand <out2=%ymm12,<_2p30m1x4=%ymm5,<out2=%ymm12
vpand %ymm12,%ymm5,%ymm12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: 4x carryy = out3 + _2p63m2p33x4 
# asm 1: vpaddq <out3=reg256#10,<_2p63m2p33x4=reg256#1,>carryy=reg256#3
# asm 2: vpaddq <out3=%ymm9,<_2p63m2p33x4=%ymm0,>carryy=%ymm2
vpaddq %ymm9,%ymm0,%ymm2

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x carryy unsigned>>= 30
# asm 1: vpsrlq $30,<carryy=reg256#3,<carryy=reg256#3
# asm 2: vpsrlq $30,<carryy=%ymm2,<carryy=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: out3 &= _2p30m1x4
# asm 1: vpand <out3=reg256#10,<_2p30m1x4=reg256#6,<out3=reg256#10
# asm 2: vpand <out3=%ymm9,<_2p30m1x4=%ymm5,<out3=%ymm9
vpand %ymm9,%ymm5,%ymm9

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: stack_FVGS1 = out3
# asm 1: vmovapd <out3=reg256#10,>stack_FVGS1=stack256#3
# asm 2: vmovapd <out3=%ymm9,>stack_FVGS1=64(%rsp)
vmovapd %ymm9,64(%rsp)

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: 4x ta = out3 << 30
# asm 1: vpsllq $30,<out3=reg256#10,>ta=reg256#9
# asm 2: vpsllq $30,<out3=%ymm9,>ta=%ymm8
vpsllq $30,%ymm9,%ymm8

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: 4x ta += out2
# asm 1: vpaddq <ta=reg256#9,<out2=reg256#13,<ta=reg256#9
# asm 2: vpaddq <ta=%ymm8,<out2=%ymm12,<ta=%ymm8
vpaddq %ymm8,%ymm12,%ymm8

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: stack_fxgx = ta
# asm 1: vmovapd <ta=reg256#9,>stack_fxgx=stack256#21
# asm 2: vmovapd <ta=%ymm8,>stack_fxgx=640(%rsp)
vmovapd %ymm8,640(%rsp)

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: FVGS4 = stack_FVGS4 
# asm 1: vmovapd <stack_FVGS4=stack256#6,>FVGS4=reg256#9
# asm 2: vmovapd <stack_FVGS4=160(%rsp),>FVGS4=%ymm8
vmovapd 160(%rsp),%ymm8

# qhasm: GSFV4 = FVGS4[1,0]
# asm 1: vpermq $0x4e,<FVGS4=reg256#9,>GSFV4=reg256#10
# asm 2: vpermq $0x4e,<FVGS4=%ymm8,>GSFV4=%ymm9
vpermq $0x4e,%ymm8,%ymm9

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x ta = int32 uuss1 * int32 FVGS3
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS3=reg256#11,>ta=reg256#11
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS3=%ymm10,>ta=%ymm10
vpmuldq %ymm3,%ymm10,%ymm10

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV3
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV3=reg256#14,>tb=reg256#14
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV3=%ymm13,>tb=%ymm13
vpmuldq %ymm1,%ymm13,%ymm13

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: 4x out4plus = ta + tb
# asm 1: vpaddq <ta=reg256#11,<tb=reg256#14,>out4plus=reg256#11
# asm 2: vpaddq <ta=%ymm10,<tb=%ymm13,>out4plus=%ymm10
vpaddq %ymm10,%ymm13,%ymm10

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm: 4x ta = int32 uuss0 * int32 FVGS4
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS4=reg256#9,>ta=reg256#14
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS4=%ymm8,>ta=%ymm13
vpmuldq %ymm6,%ymm8,%ymm13

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV4
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV4=reg256#10,>tb=reg256#15
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV4=%ymm9,>tb=%ymm14
vpmuldq %ymm7,%ymm9,%ymm14

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#8
# asm 2: mov  $-1,>z=%r10
mov  $-1,%r10

# qhasm: 4x out4 = ta + tb
# asm 1: vpaddq <ta=reg256#14,<tb=reg256#15,>out4=reg256#14
# asm 2: vpaddq <ta=%ymm13,<tb=%ymm14,>out4=%ymm13
vpaddq %ymm13,%ymm14,%ymm13

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: 4x out4 += out4plus
# asm 1: vpaddq <out4=reg256#14,<out4plus=reg256#11,<out4=reg256#14
# asm 2: vpaddq <out4=%ymm13,<out4plus=%ymm10,<out4=%ymm13
vpaddq %ymm13,%ymm10,%ymm13

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x out4 += carryy
# asm 1: vpaddq <out4=reg256#14,<carryy=reg256#3,<out4=reg256#14
# asm 2: vpaddq <out4=%ymm13,<carryy=%ymm2,<out4=%ymm13
vpaddq %ymm13,%ymm2,%ymm13

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x carryy = out4 + _2p63m2p33x4 
# asm 1: vpaddq <out4=reg256#14,<_2p63m2p33x4=reg256#1,>carryy=reg256#3
# asm 2: vpaddq <out4=%ymm13,<_2p63m2p33x4=%ymm0,>carryy=%ymm2
vpaddq %ymm13,%ymm0,%ymm2

# qhasm: 4x carryy unsigned>>= 30
# asm 1: vpsrlq $30,<carryy=reg256#3,<carryy=reg256#3
# asm 2: vpsrlq $30,<carryy=%ymm2,<carryy=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm: FVGS5 = stack_FVGS5 
# asm 1: vmovapd <stack_FVGS5=stack256#7,>FVGS5=reg256#11
# asm 2: vmovapd <stack_FVGS5=192(%rsp),>FVGS5=%ymm10
vmovapd 192(%rsp),%ymm10

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: GSFV5 = FVGS5[1,0]
# asm 1: vpermq $0x4e,<FVGS5=reg256#11,>GSFV5=reg256#15
# asm 2: vpermq $0x4e,<FVGS5=%ymm10,>GSFV5=%ymm14
vpermq $0x4e,%ymm10,%ymm14

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#8
# asm 2: mov  $-1,>z=%r10
mov  $-1,%r10

# qhasm: 4x ta = int32 uuss1 * int32 FVGS4
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS4=reg256#9,>ta=reg256#9
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS4=%ymm8,>ta=%ymm8
vpmuldq %ymm3,%ymm8,%ymm8

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV4
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV4=reg256#10,>tb=reg256#10
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV4=%ymm9,>tb=%ymm9
vpmuldq %ymm1,%ymm9,%ymm9

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x out5plus = ta + tb
# asm 1: vpaddq <ta=reg256#9,<tb=reg256#10,>out5plus=reg256#9
# asm 2: vpaddq <ta=%ymm8,<tb=%ymm9,>out5plus=%ymm8
vpaddq %ymm8,%ymm9,%ymm8

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: 4x ta = int32 uuss0 * int32 FVGS5
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS5=reg256#11,>ta=reg256#10
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS5=%ymm10,>ta=%ymm9
vpmuldq %ymm6,%ymm10,%ymm9

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV5
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV5=reg256#15,>tb=reg256#16
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV5=%ymm14,>tb=%ymm15
vpmuldq %ymm7,%ymm14,%ymm15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: 4x out5 = ta + tb
# asm 1: vpaddq <ta=reg256#10,<tb=reg256#16,>out5=reg256#10
# asm 2: vpaddq <ta=%ymm9,<tb=%ymm15,>out5=%ymm9
vpaddq %ymm9,%ymm15,%ymm9

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: 4x out5 += out5plus
# asm 1: vpaddq <out5=reg256#10,<out5plus=reg256#9,<out5=reg256#10
# asm 2: vpaddq <out5=%ymm9,<out5plus=%ymm8,<out5=%ymm9
vpaddq %ymm9,%ymm8,%ymm9

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x out5 += carryy
# asm 1: vpaddq <out5=reg256#10,<carryy=reg256#3,<out5=reg256#10
# asm 2: vpaddq <out5=%ymm9,<carryy=%ymm2,<out5=%ymm9
vpaddq %ymm9,%ymm2,%ymm9

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x carryy = out5 + _2p63m2p33x4 
# asm 1: vpaddq <out5=reg256#10,<_2p63m2p33x4=reg256#1,>carryy=reg256#3
# asm 2: vpaddq <out5=%ymm9,<_2p63m2p33x4=%ymm0,>carryy=%ymm2
vpaddq %ymm9,%ymm0,%ymm2

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: 4x carryy unsigned>>= 30
# asm 1: vpsrlq $30,<carryy=reg256#3,<carryy=reg256#3
# asm 2: vpsrlq $30,<carryy=%ymm2,<carryy=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm: out5 &= _2p30m1x4
# asm 1: vpand <out5=reg256#10,<_2p30m1x4=reg256#6,<out5=reg256#10
# asm 2: vpand <out5=%ymm9,<_2p30m1x4=%ymm5,<out5=%ymm9
vpand %ymm9,%ymm5,%ymm9

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: out4 &= _2p30m1x4
# asm 1: vpand <out4=reg256#14,<_2p30m1x4=reg256#6,<out4=reg256#14
# asm 2: vpand <out4=%ymm13,<_2p30m1x4=%ymm5,<out4=%ymm13
vpand %ymm13,%ymm5,%ymm13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: stack_FVGS2 = out4
# asm 1: vmovapd <out4=reg256#14,>stack_FVGS2=stack256#4
# asm 2: vmovapd <out4=%ymm13,>stack_FVGS2=96(%rsp)
vmovapd %ymm13,96(%rsp)

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: 4x ta = out5 << 30
# asm 1: vpsllq $30,<out5=reg256#10,>ta=reg256#9
# asm 2: vpsllq $30,<out5=%ymm9,>ta=%ymm8
vpsllq $30,%ymm9,%ymm8

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: 4x ta += out4
# asm 1: vpaddq <ta=reg256#9,<out4=reg256#14,<ta=reg256#9
# asm 2: vpaddq <ta=%ymm8,<out4=%ymm13,<ta=%ymm8
vpaddq %ymm8,%ymm13,%ymm8

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: stack_fygy = ta
# asm 1: vmovapd <ta=reg256#9,>stack_fygy=stack256#22
# asm 2: vmovapd <ta=%ymm8,>stack_fygy=672(%rsp)
vmovapd %ymm8,672(%rsp)

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: FVGS6 = stack_FVGS6 
# asm 1: vmovapd <stack_FVGS6=stack256#8,>FVGS6=reg256#9
# asm 2: vmovapd <stack_FVGS6=224(%rsp),>FVGS6=%ymm8
vmovapd 224(%rsp),%ymm8

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: GSFV6 = FVGS6[1,0]
# asm 1: vpermq $0x4e,<FVGS6=reg256#9,>GSFV6=reg256#14
# asm 2: vpermq $0x4e,<FVGS6=%ymm8,>GSFV6=%ymm13
vpermq $0x4e,%ymm8,%ymm13

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x ta = int32 uuss1 * int32 FVGS5
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS5=reg256#11,>ta=reg256#11
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS5=%ymm10,>ta=%ymm10
vpmuldq %ymm3,%ymm10,%ymm10

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV5
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV5=reg256#15,>tb=reg256#15
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV5=%ymm14,>tb=%ymm14
vpmuldq %ymm1,%ymm14,%ymm14

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: 4x out6plus = ta + tb
# asm 1: vpaddq <ta=reg256#11,<tb=reg256#15,>out6plus=reg256#11
# asm 2: vpaddq <ta=%ymm10,<tb=%ymm14,>out6plus=%ymm10
vpaddq %ymm10,%ymm14,%ymm10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: 4x ta = int32 uuss0 * int32 FVGS6
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS6=reg256#9,>ta=reg256#15
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS6=%ymm8,>ta=%ymm14
vpmuldq %ymm6,%ymm8,%ymm14

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV6
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV6=reg256#14,>tb=reg256#16
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV6=%ymm13,>tb=%ymm15
vpmuldq %ymm7,%ymm13,%ymm15

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x out6 = ta + tb
# asm 1: vpaddq <ta=reg256#15,<tb=reg256#16,>out6=reg256#15
# asm 2: vpaddq <ta=%ymm14,<tb=%ymm15,>out6=%ymm14
vpaddq %ymm14,%ymm15,%ymm14

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x out6 += out6plus
# asm 1: vpaddq <out6=reg256#15,<out6plus=reg256#11,<out6=reg256#15
# asm 2: vpaddq <out6=%ymm14,<out6plus=%ymm10,<out6=%ymm14
vpaddq %ymm14,%ymm10,%ymm14

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: 4x out6 += carryy
# asm 1: vpaddq <out6=reg256#15,<carryy=reg256#3,<out6=reg256#15
# asm 2: vpaddq <out6=%ymm14,<carryy=%ymm2,<out6=%ymm14
vpaddq %ymm14,%ymm2,%ymm14

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: 4x carryz = out6 + _2p63m2p33x4 
# asm 1: vpaddq <out6=reg256#15,<_2p63m2p33x4=reg256#1,>carryz=reg256#3
# asm 2: vpaddq <out6=%ymm14,<_2p63m2p33x4=%ymm0,>carryz=%ymm2
vpaddq %ymm14,%ymm0,%ymm2

# qhasm: 4x carryz unsigned>>= 30
# asm 1: vpsrlq $30,<carryz=reg256#3,<carryz=reg256#3
# asm 2: vpsrlq $30,<carryz=%ymm2,<carryz=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: stack_FVGS3 = out5
# asm 1: vmovapd <out5=reg256#10,>stack_FVGS3=stack256#5
# asm 2: vmovapd <out5=%ymm9,>stack_FVGS3=128(%rsp)
vmovapd %ymm9,128(%rsp)

# qhasm: FVGS7 = stack_FVGS7 
# asm 1: vmovapd <stack_FVGS7=stack256#9,>FVGS7=reg256#10
# asm 2: vmovapd <stack_FVGS7=256(%rsp),>FVGS7=%ymm9
vmovapd 256(%rsp),%ymm9

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: GSFV7 = FVGS7[1,0]
# asm 1: vpermq $0x4e,<FVGS7=reg256#10,>GSFV7=reg256#11
# asm 2: vpermq $0x4e,<FVGS7=%ymm9,>GSFV7=%ymm10
vpermq $0x4e,%ymm9,%ymm10

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x ta = int32 uuss1 * int32 FVGS6
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS6=reg256#9,>ta=reg256#9
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS6=%ymm8,>ta=%ymm8
vpmuldq %ymm3,%ymm8,%ymm8

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV6
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV6=reg256#14,>tb=reg256#14
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV6=%ymm13,>tb=%ymm13
vpmuldq %ymm1,%ymm13,%ymm13

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x out7plus = ta + tb
# asm 1: vpaddq <ta=reg256#9,<tb=reg256#14,>out7plus=reg256#9
# asm 2: vpaddq <ta=%ymm8,<tb=%ymm13,>out7plus=%ymm8
vpaddq %ymm8,%ymm13,%ymm8

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x ta = int32 uuss0 * int32 FVGS7
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS7=reg256#10,>ta=reg256#14
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS7=%ymm9,>ta=%ymm13
vpmuldq %ymm6,%ymm9,%ymm13

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV7
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV7=reg256#11,>tb=reg256#16
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV7=%ymm10,>tb=%ymm15
vpmuldq %ymm7,%ymm10,%ymm15

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: 4x out7 = ta + tb
# asm 1: vpaddq <ta=reg256#14,<tb=reg256#16,>out7=reg256#14
# asm 2: vpaddq <ta=%ymm13,<tb=%ymm15,>out7=%ymm13
vpaddq %ymm13,%ymm15,%ymm13

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm: 4x out7 += out7plus
# asm 1: vpaddq <out7=reg256#14,<out7plus=reg256#9,<out7=reg256#14
# asm 2: vpaddq <out7=%ymm13,<out7plus=%ymm8,<out7=%ymm13
vpaddq %ymm13,%ymm8,%ymm13

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x out7 += carryz
# asm 1: vpaddq <out7=reg256#14,<carryz=reg256#3,<out7=reg256#14
# asm 2: vpaddq <out7=%ymm13,<carryz=%ymm2,<out7=%ymm13
vpaddq %ymm13,%ymm2,%ymm13

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: 4x carryz = out7 + _2p63m2p33x4 
# asm 1: vpaddq <out7=reg256#14,<_2p63m2p33x4=reg256#1,>carryz=reg256#3
# asm 2: vpaddq <out7=%ymm13,<_2p63m2p33x4=%ymm0,>carryz=%ymm2
vpaddq %ymm13,%ymm0,%ymm2

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm: 4x carryz unsigned>>= 30
# asm 1: vpsrlq $30,<carryz=reg256#3,<carryz=reg256#3
# asm 2: vpsrlq $30,<carryz=%ymm2,<carryz=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: out6 &= _2p30m1x4
# asm 1: vpand <out6=reg256#15,<_2p30m1x4=reg256#6,<out6=reg256#15
# asm 2: vpand <out6=%ymm14,<_2p30m1x4=%ymm5,<out6=%ymm14
vpand %ymm14,%ymm5,%ymm14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: stack_FVGS4 = out6
# asm 1: vmovapd <out6=reg256#15,>stack_FVGS4=stack256#6
# asm 2: vmovapd <out6=%ymm14,>stack_FVGS4=160(%rsp)
vmovapd %ymm14,160(%rsp)

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: FVGS8 = stack_FVGS8 
# asm 1: vmovapd <stack_FVGS8=stack256#10,>FVGS8=reg256#9
# asm 2: vmovapd <stack_FVGS8=288(%rsp),>FVGS8=%ymm8
vmovapd 288(%rsp),%ymm8

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: GSFV8 = FVGS8[1,0]
# asm 1: vpermq $0x4e,<FVGS8=reg256#9,>GSFV8=reg256#15
# asm 2: vpermq $0x4e,<FVGS8=%ymm8,>GSFV8=%ymm14
vpermq $0x4e,%ymm8,%ymm14

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: 4x ta = int32 uuss1 * int32 FVGS7
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS7=reg256#10,>ta=reg256#10
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS7=%ymm9,>ta=%ymm9
vpmuldq %ymm3,%ymm9,%ymm9

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#8
# asm 2: mov  $-1,>z=%r10
mov  $-1,%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV7
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV7=reg256#11,>tb=reg256#11
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV7=%ymm10,>tb=%ymm10
vpmuldq %ymm1,%ymm10,%ymm10

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm: 4x out8plus = ta + tb
# asm 1: vpaddq <ta=reg256#10,<tb=reg256#11,>out8plus=reg256#10
# asm 2: vpaddq <ta=%ymm9,<tb=%ymm10,>out8plus=%ymm9
vpaddq %ymm9,%ymm10,%ymm9

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x ta = int32 uuss0 * int32 FVGS8
# asm 1: vpmuldq <uuss0=reg256#7,<FVGS8=reg256#9,>ta=reg256#7
# asm 2: vpmuldq <uuss0=%ymm6,<FVGS8=%ymm8,>ta=%ymm6
vpmuldq %ymm6,%ymm8,%ymm6

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x tb = int32 vvrr0 * int32 GSFV8
# asm 1: vpmuldq <vvrr0=reg256#8,<GSFV8=reg256#15,>tb=reg256#8
# asm 2: vpmuldq <vvrr0=%ymm7,<GSFV8=%ymm14,>tb=%ymm7
vpmuldq %ymm7,%ymm14,%ymm7

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: 4x out8 = ta + tb
# asm 1: vpaddq <ta=reg256#7,<tb=reg256#8,>out8=reg256#7
# asm 2: vpaddq <ta=%ymm6,<tb=%ymm7,>out8=%ymm6
vpaddq %ymm6,%ymm7,%ymm6

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: 4x out8 += out8plus
# asm 1: vpaddq <out8=reg256#7,<out8plus=reg256#10,<out8=reg256#7
# asm 2: vpaddq <out8=%ymm6,<out8plus=%ymm9,<out8=%ymm6
vpaddq %ymm6,%ymm9,%ymm6

# qhasm: 4x out8 += carryz
# asm 1: vpaddq <out8=reg256#7,<carryz=reg256#3,<out8=reg256#7
# asm 2: vpaddq <out8=%ymm6,<carryz=%ymm2,<out8=%ymm6
vpaddq %ymm6,%ymm2,%ymm6

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm: 4x out8 += out8plus2
# asm 1: vpaddq <out8=reg256#7,<out8plus2=reg256#12,<out8=reg256#7
# asm 2: vpaddq <out8=%ymm6,<out8plus2=%ymm11,<out8=%ymm6
vpaddq %ymm6,%ymm11,%ymm6

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm: 4x carryz = out8 + _2p63m2p33x4 
# asm 1: vpaddq <out8=reg256#7,<_2p63m2p33x4=reg256#1,>carryz=reg256#3
# asm 2: vpaddq <out8=%ymm6,<_2p63m2p33x4=%ymm0,>carryz=%ymm2
vpaddq %ymm6,%ymm0,%ymm2

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x carryz unsigned>>= 30
# asm 1: vpsrlq $30,<carryz=reg256#3,<carryz=reg256#3
# asm 2: vpsrlq $30,<carryz=%ymm2,<carryz=%ymm2
vpsrlq $30,%ymm2,%ymm2

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm: out7 &= _2p30m1x4
# asm 1: vpand <out7=reg256#14,<_2p30m1x4=reg256#6,<out7=reg256#14
# asm 2: vpand <out7=%ymm13,<_2p30m1x4=%ymm5,<out7=%ymm13
vpand %ymm13,%ymm5,%ymm13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm: stack_FVGS5 = out7
# asm 1: vmovapd <out7=reg256#14,>stack_FVGS5=stack256#7
# asm 2: vmovapd <out7=%ymm13,>stack_FVGS5=192(%rsp)
vmovapd %ymm13,192(%rsp)

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm: 4x ta = int32 uuss1 * int32 FVGS8
# asm 1: vpmuldq <uuss1=reg256#4,<FVGS8=reg256#9,>ta=reg256#4
# asm 2: vpmuldq <uuss1=%ymm3,<FVGS8=%ymm8,>ta=%ymm3
vpmuldq %ymm3,%ymm8,%ymm3

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: 4x tb = int32 vvrr1 * int32 GSFV8
# asm 1: vpmuldq <vvrr1=reg256#2,<GSFV8=reg256#15,>tb=reg256#2
# asm 2: vpmuldq <vvrr1=%ymm1,<GSFV8=%ymm14,>tb=%ymm1
vpmuldq %ymm1,%ymm14,%ymm1

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: 4x out9plus = ta + tb
# asm 1: vpaddq <ta=reg256#4,<tb=reg256#2,>out9plus=reg256#2
# asm 2: vpaddq <ta=%ymm3,<tb=%ymm1,>out9plus=%ymm1
vpaddq %ymm3,%ymm1,%ymm1

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#8
# asm 2: movq <stack_m1=8(%rsp),>z=%r10
movq 8(%rsp),%r10

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#7,>oldg=int64#9
# asm 2: mov  <grs=%rax,>oldg=%r11
mov  %rax,%r11

# qhasm: 4x out9 += out9plus
# asm 1: vpaddq <out9=reg256#5,<out9plus=reg256#2,<out9=reg256#5
# asm 2: vpaddq <out9=%ymm4,<out9plus=%ymm1,<out9=%ymm4
vpaddq %ymm4,%ymm1,%ymm4

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#7,<fuv=int64#6),>h=int64#10
# asm 2: lea  (<grs=%rax,<fuv=%r9),>h=%r12
lea  (%rax,%r9),%r12

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#7
# asm 2: test  $1,<grs=%rax
test  $1,%rax

# qhasm: 4x out9 += carryz
# asm 1: vpaddq <out9=reg256#5,<carryz=reg256#3,<out9=reg256#5
# asm 2: vpaddq <out9=%ymm4,<carryz=%ymm2,<out9=%ymm4
vpaddq %ymm4,%ymm2,%ymm4

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#8
# asm 2: cmovne <m=%rsi,<z=%r10
cmovne %rsi,%r10

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#7,<h=int64#10
# asm 2: cmove <grs=%rax,<h=%r12
cmove %rax,%r12

# qhasm: 4x out10 = out9 + _2p63m2p33x4 
# asm 1: vpaddq <out9=reg256#5,<_2p63m2p33x4=reg256#1,>out10=reg256#2
# asm 2: vpaddq <out9=%ymm4,<_2p63m2p33x4=%ymm0,>out10=%ymm1
vpaddq %ymm4,%ymm0,%ymm1

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#11
# asm 2: lea  1(<m=%rsi),>mnew=%r13
lea  1(%rsi),%r13

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#6,<grs=int64#7
# asm 2: sub  <fuv=%r9,<grs=%rax
sub  %r9,%rax

# qhasm: 4x out10 unsigned>>= 30
# asm 1: vpsrlq $30,<out10=reg256#2,<out10=reg256#2
# asm 2: vpsrlq $30,<out10=%ymm1,<out10=%ymm1
vpsrlq $30,%ymm1,%ymm1

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#7
# asm 2: sar  $1,<grs=%rax
sar  $1,%rax

# qhasm: out8 &= _2p30m1x4
# asm 1: vpand <out8=reg256#7,<_2p30m1x4=reg256#6,<out8=reg256#7
# asm 2: vpand <out8=%ymm6,<_2p30m1x4=%ymm5,<out8=%ymm6
vpand %ymm6,%ymm5,%ymm6

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#10
# asm 2: sar  $1,<h=%r12
sar  $1,%r12

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#8
# asm 2: cmp  $0,<z=%r10
cmp  $0,%r10

# qhasm: stack_FVGS6 = out8
# asm 1: vmovapd <out8=reg256#7,>stack_FVGS6=stack256#8
# asm 2: vmovapd <out8=%ymm6,>stack_FVGS6=224(%rsp)
vmovapd %ymm6,224(%rsp)

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#9,<fuv=int64#6
# asm 2: cmovge <oldg=%r11,<fuv=%r9
cmovge %r11,%r9

# qhasm: _32767x4 = stack_32767x4
# asm 1: vmovapd <stack_32767x4=stack256#13,>_32767x4=reg256#3
# asm 2: vmovapd <stack_32767x4=384(%rsp),>_32767x4=%ymm2
vmovapd 384(%rsp),%ymm2

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#10,<grs=int64#7
# asm 2: cmovl <h=%r12,<grs=%rax
cmovl %r12,%rax

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#11,<m=int64#2
# asm 2: cmovl <mnew=%r13,<m=%rsi
cmovl %r13,%rsi

# qhasm: extract_init:
._extract_init:

# qhasm:   _2p20a2p41 = stack_2p20a2p41
# asm 1: movq <stack_2p20a2p41=stack64#13,>_2p20a2p41=int64#8
# asm 2: movq <stack_2p20a2p41=800(%rsp),>_2p20a2p41=%r10
movq 800(%rsp),%r10

# qhasm:   s = grs + _2p20a2p41
# asm 1: lea  (<grs=int64#7,<_2p20a2p41=int64#8),>s=int64#9
# asm 2: lea  (<grs=%rax,<_2p20a2p41=%r10),>s=%r11
lea  (%rax,%r10),%r11

# qhasm: _2p48x4 = stack_2p48x4
# asm 1: vmovapd <stack_2p48x4=stack256#17,>_2p48x4=reg256#4
# asm 2: vmovapd <stack_2p48x4=512(%rsp),>_2p48x4=%ymm3
vmovapd 512(%rsp),%ymm3

# qhasm: 4x carryy = out10 + _2p63m2p33x4
# asm 1: vpaddq <out10=reg256#2,<_2p63m2p33x4=reg256#1,>carryy=reg256#7
# asm 2: vpaddq <out10=%ymm1,<_2p63m2p33x4=%ymm0,>carryy=%ymm6
vpaddq %ymm1,%ymm0,%ymm6

# qhasm:   (int64) s >>= 42
# asm 1: sar  $42,<s=int64#9
# asm 2: sar  $42,<s=%r11
sar  $42,%r11

# qhasm:   t2 = g
# asm 1: mov  <g=int64#3,>t2=int64#10
# asm 2: mov  <g=%rdx,>t2=%r12
mov  %rdx,%r12

# qhasm: 4x carryy unsigned>>= 15
# asm 1: vpsrlq $15,<carryy=reg256#7,<carryy=reg256#7
# asm 2: vpsrlq $15,<carryy=%ymm6,<carryy=%ymm6
vpsrlq $15,%ymm6,%ymm6

# qhasm:   g *= s  
# asm 1: imul  <s=int64#9,<g=int64#3
# asm 2: imul  <s=%r11,<g=%rdx
imul  %r11,%rdx

# qhasm:   v = fuv + _2p20a2p41
# asm 1: lea  (<fuv=int64#6,<_2p20a2p41=int64#8),>v=int64#8
# asm 2: lea  (<fuv=%r9,<_2p20a2p41=%r10),>v=%r10
lea  (%r9,%r10),%r10

# qhasm: 4x carryy -= _2p48x4
# asm 1: vpsubq <_2p48x4=reg256#4,<carryy=reg256#7,<carryy=reg256#7
# asm 2: vpsubq <_2p48x4=%ymm3,<carryy=%ymm6,<carryy=%ymm6
vpsubq %ymm3,%ymm6,%ymm6

# qhasm:   (int64) v >>= 42
# asm 1: sar  $42,<v=int64#8
# asm 2: sar  $42,<v=%r10
sar  $42,%r10

# qhasm:   t2 *= v
# asm 1: imul  <v=int64#8,<t2=int64#10
# asm 2: imul  <v=%r10,<t2=%r12
imul  %r10,%r12

# qhasm: 4x out10 -= stack_2p33x4
# asm 1: vpsubq <stack_2p33x4=stack256#16,<out10=reg256#2,<out10=reg256#2
# asm 2: vpsubq <stack_2p33x4=480(%rsp),<out10=%ymm1,<out10=%ymm1
vpsubq 480(%rsp),%ymm1,%ymm1

# qhasm:   _2p20 = stack_2p20
# asm 1: movq <stack_2p20=stack64#10,>_2p20=int64#11
# asm 2: movq <stack_2p20=776(%rsp),>_2p20=%r13
movq 776(%rsp),%r13

# qhasm:   r = grs + _2p20
# asm 1: lea  (<grs=int64#7,<_2p20=int64#11),>r=int64#7
# asm 2: lea  (<grs=%rax,<_2p20=%r13),>r=%rax
lea  (%rax,%r13),%rax

# qhasm: out9 &= _2p30m1x4
# asm 1: vpand <out9=reg256#5,<_2p30m1x4=reg256#6,<out9=reg256#5
# asm 2: vpand <out9=%ymm4,<_2p30m1x4=%ymm5,<out9=%ymm4
vpand %ymm4,%ymm5,%ymm4

# qhasm:   r <<= 22
# asm 1: shl  $22,<r=int64#7
# asm 2: shl  $22,<r=%rax
shl  $22,%rax

# qhasm:   (int64) r >>= 43
# asm 1: sar  $43,<r=int64#7
# asm 2: sar  $43,<r=%rax
sar  $43,%rax

# qhasm: out10 &= _32767x4
# asm 1: vpand <out10=reg256#2,<_32767x4=reg256#3,<out10=reg256#2
# asm 2: vpand <out10=%ymm1,<_32767x4=%ymm2,<out10=%ymm1
vpand %ymm1,%ymm2,%ymm1

# qhasm:       rax = f
# asm 1: mov  <f=int64#1,>rax=int64#12
# asm 2: mov  <f=%rdi,>rax=%r14
mov  %rdi,%r14

# qhasm:       rax *= r
# asm 1: imul  <r=int64#7,<rax=int64#12
# asm 2: imul  <r=%rax,<rax=%r14
imul  %rax,%r14

# qhasm: 4x carryy = int32 carryy * int32 stack_0_19x4
# asm 1: vpmuldq <stack_0_19x4=stack256#12,<carryy=reg256#7,>carryy=reg256#3
# asm 2: vpmuldq <stack_0_19x4=352(%rsp),<carryy=%ymm6,>carryy=%ymm2
vpmuldq 352(%rsp),%ymm6,%ymm2

# qhasm:   u = fuv + _2p20
# asm 1: lea  (<fuv=int64#6,<_2p20=int64#11),>u=int64#6
# asm 2: lea  (<fuv=%r9,<_2p20=%r13),>u=%r9
lea  (%r9,%r13),%r9

# qhasm: 4x out2 += carryy
# asm 1: vpaddq <out2=reg256#13,<carryy=reg256#3,<out2=reg256#13
# asm 2: vpaddq <out2=%ymm12,<carryy=%ymm2,<out2=%ymm12
vpaddq %ymm12,%ymm2,%ymm12

# qhasm:   u <<= 22
# asm 1: shl  $22,<u=int64#6
# asm 2: shl  $22,<u=%r9
shl  $22,%r9

# qhasm:   (int64) u >>= 43
# asm 1: sar  $43,<u=int64#6
# asm 2: sar  $43,<u=%r9
sar  $43,%r9

# qhasm:        f *= u
# asm 1: imul  <u=int64#6,<f=int64#1
# asm 2: imul  <u=%r9,<f=%rdi
imul  %r9,%rdi

# qhasm: stack_FVGS7 = out9
# asm 1: vmovapd <out9=reg256#5,>stack_FVGS7=stack256#9
# asm 2: vmovapd <out9=%ymm4,>stack_FVGS7=256(%rsp)
vmovapd %ymm4,256(%rsp)

# qhasm:        f += t2
# asm 1: add  <t2=int64#10,<f=int64#1
# asm 2: add  <t2=%r12,<f=%rdi
add  %r12,%rdi

# qhasm:        g += rax
# asm 1: add  <rax=int64#12,<g=int64#3
# asm 2: add  <rax=%r14,<g=%rdx
add  %r14,%rdx

# qhasm: stack_FVGS8 = out10
# asm 1: vmovapd <out10=reg256#2,>stack_FVGS8=stack256#10
# asm 2: vmovapd <out10=%ymm1,>stack_FVGS8=288(%rsp)
vmovapd %ymm1,288(%rsp)

# qhasm:   (int64) f >>= 20
# asm 1: sar  $20,<f=int64#1
# asm 2: sar  $20,<f=%rdi
sar  $20,%rdi

# qhasm: stack_FVGS0 = out2
# asm 1: vmovapd <out2=reg256#13,>stack_FVGS0=stack256#2
# asm 2: vmovapd <out2=%ymm12,>stack_FVGS0=32(%rsp)
vmovapd %ymm12,32(%rsp)

# qhasm:   (int64) g >>= 20
# asm 1: sar  $20,<g=int64#3
# asm 2: sar  $20,<g=%rdx
sar  $20,%rdx

# qhasm:   inplace stack_vvrr[0] = v
# asm 1: movq <v=int64#8,<stack_vvrr=stack256#20
# asm 2: movq <v=%r10,<stack_vvrr=608(%rsp)
movq %r10,608(%rsp)

# qhasm:   inplace stack_uuss[0] = u
# asm 1: movq <u=int64#6,<stack_uuss=stack256#19
# asm 2: movq <u=%r9,<stack_uuss=576(%rsp)
movq %r9,576(%rsp)

# qhasm:   inplace stack_uuss[2] = s
# asm 1: movq <s=int64#9,<stack_uuss=stack256#19
# asm 2: movq <s=%r11,<stack_uuss=592(%rsp)
movq %r11,592(%rsp)

# qhasm:   inplace stack_vvrr[2] = r
# asm 1: movq <r=int64#7,<stack_vvrr=stack256#20
# asm 2: movq <r=%rax,<stack_vvrr=624(%rsp)
movq %rax,624(%rsp)

# qhasm:   loop = 2
# asm 1: mov  $2,>loop=int64#6
# asm 2: mov  $2,>loop=%r9
mov  $2,%r9

# qhasm: loop20:
._loop20:

# qhasm:   fuv = f & ~ _m2p20
# asm 1: andn  <f=int64#1,<_m2p20=int64#4,>fuv=int64#7
# asm 2: andn  <f=%rdi,<_m2p20=%rcx,>fuv=%rax
andn  %rdi,%rcx,%rax

# qhasm:   grs = g & ~ _m2p20
# asm 1: andn  <g=int64#3,<_m2p20=int64#4,>grs=int64#8
# asm 2: andn  <g=%rdx,<_m2p20=%rcx,>grs=%r10
andn  %rdx,%rcx,%r10

# qhasm:   fuv += stack_m2p41
# asm 1: addq <stack_m2p41=stack64#11,<fuv=int64#7
# asm 2: addq <stack_m2p41=784(%rsp),<fuv=%rax
addq 784(%rsp),%rax

# qhasm:   grs += stack_m2p62
# asm 1: addq <stack_m2p62=stack64#9,<grs=int64#8
# asm 2: addq <stack_m2p62=768(%rsp),<grs=%r10
addq 768(%rsp),%r10

# qhasm: j = 2
# asm 1: mov  $2,>j=int64#9
# asm 2: mov  $2,>j=%r11
mov  $2,%r11

# qhasm: loop2:
._loop2:

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#10
# asm 2: mov  $-1,>z=%r12
mov  $-1,%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#10
# asm 2: movq <stack_m1=8(%rsp),>z=%r12
movq 8(%rsp),%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#10
# asm 2: mov  $-1,>z=%r12
mov  $-1,%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#10
# asm 2: movq <stack_m1=8(%rsp),>z=%r12
movq 8(%rsp),%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#10
# asm 2: movq <stack_m1=8(%rsp),>z=%r12
movq 8(%rsp),%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#10
# asm 2: movq <stack_m1=8(%rsp),>z=%r12
movq 8(%rsp),%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#10
# asm 2: movq <stack_m1=8(%rsp),>z=%r12
movq 8(%rsp),%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#10
# asm 2: movq <stack_m1=8(%rsp),>z=%r12
movq 8(%rsp),%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = stack_m1[1]
# asm 1: movq <stack_m1=stack256#1,>z=int64#10
# asm 2: movq <stack_m1=8(%rsp),>z=%r12
movq 8(%rsp),%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:   z = -1
# asm 1: mov  $-1,>z=int64#10
# asm 2: mov  $-1,>z=%r12
mov  $-1,%r12

# qhasm:   oldg = grs
# asm 1: mov  <grs=int64#8,>oldg=int64#11
# asm 2: mov  <grs=%r10,>oldg=%r13
mov  %r10,%r13

# qhasm:   h = grs + fuv
# asm 1: lea  (<grs=int64#8,<fuv=int64#7),>h=int64#12
# asm 2: lea  (<grs=%r10,<fuv=%rax),>h=%r14
lea  (%r10,%rax),%r14

# qhasm:               =? grs & 1
# asm 1: test  $1,<grs=int64#8
# asm 2: test  $1,<grs=%r10
test  $1,%r10

# qhasm:   z = m   if !=
# asm 1: cmovne <m=int64#2,<z=int64#10
# asm 2: cmovne <m=%rsi,<z=%r12
cmovne %rsi,%r12

# qhasm:   h = grs if  =
# asm 1: cmove <grs=int64#8,<h=int64#12
# asm 2: cmove <grs=%r10,<h=%r14
cmove %r10,%r14

# qhasm:   mnew = m + 1
# asm 1: lea  1(<m=int64#2),>mnew=int64#13
# asm 2: lea  1(<m=%rsi),>mnew=%r15
lea  1(%rsi),%r15

# qhasm:   grs -= fuv
# asm 1: sub  <fuv=int64#7,<grs=int64#8
# asm 2: sub  <fuv=%rax,<grs=%r10
sub  %rax,%r10

# qhasm:   (int64) grs >>= 1
# asm 1: sar  $1,<grs=int64#8
# asm 2: sar  $1,<grs=%r10
sar  $1,%r10

# qhasm:   (int64) h >>= 1
# asm 1: sar  $1,<h=int64#12
# asm 2: sar  $1,<h=%r14
sar  $1,%r14

# qhasm:   m = -m
# asm 1: neg  <m=int64#2
# asm 2: neg  <m=%rsi
neg  %rsi

# qhasm:                  signed<? z - 0
# asm 1: cmp  $0,<z=int64#10
# asm 2: cmp  $0,<z=%r12
cmp  $0,%r12

# qhasm:   fuv = oldg if !signed<
# asm 1: cmovge <oldg=int64#11,<fuv=int64#7
# asm 2: cmovge <oldg=%r13,<fuv=%rax
cmovge %r13,%rax

# qhasm:   grs = h    if  signed<
# asm 1: cmovl <h=int64#12,<grs=int64#8
# asm 2: cmovl <h=%r14,<grs=%r10
cmovl %r14,%r10

# qhasm:   m = mnew   if  signed<
# asm 1: cmovl <mnew=int64#13,<m=int64#2
# asm 2: cmovl <mnew=%r15,<m=%rsi
cmovl %r15,%rsi

# qhasm:  =? j -= 1
# asm 1: dec <j=int64#9
# asm 2: dec <j=%r11
dec %r11
# comment:fp stack unchanged by jump

# qhasm: goto loop2 if !=
jne ._loop2

# qhasm:   =? loop -= 1		
# asm 1: dec <loop=int64#6
# asm 2: dec <loop=%r9
dec %r9
# comment:fp stack unchanged by jump

# qhasm: goto lastloop if =
je ._lastloop

# qhasm: extract:
._extract:

# qhasm:   _2p20a2p41 = stack_2p20a2p41
# asm 1: movq <stack_2p20a2p41=stack64#13,>_2p20a2p41=int64#9
# asm 2: movq <stack_2p20a2p41=800(%rsp),>_2p20a2p41=%r11
movq 800(%rsp),%r11

# qhasm:   s = grs + _2p20a2p41
# asm 1: lea  (<grs=int64#8,<_2p20a2p41=int64#9),>s=int64#10
# asm 2: lea  (<grs=%r10,<_2p20a2p41=%r11),>s=%r12
lea  (%r10,%r11),%r12

# qhasm:   (int64) s >>= 42
# asm 1: sar  $42,<s=int64#10
# asm 2: sar  $42,<s=%r12
sar  $42,%r12

# qhasm:   t2 = g
# asm 1: mov  <g=int64#3,>t2=int64#11
# asm 2: mov  <g=%rdx,>t2=%r13
mov  %rdx,%r13

# qhasm:   g *= s  
# asm 1: imul  <s=int64#10,<g=int64#3
# asm 2: imul  <s=%r12,<g=%rdx
imul  %r12,%rdx

# qhasm:   v = fuv + _2p20a2p41
# asm 1: lea  (<fuv=int64#7,<_2p20a2p41=int64#9),>v=int64#9
# asm 2: lea  (<fuv=%rax,<_2p20a2p41=%r11),>v=%r11
lea  (%rax,%r11),%r11

# qhasm:   (int64) v >>= 42
# asm 1: sar  $42,<v=int64#9
# asm 2: sar  $42,<v=%r11
sar  $42,%r11

# qhasm:   t2 *= v
# asm 1: imul  <v=int64#9,<t2=int64#11
# asm 2: imul  <v=%r11,<t2=%r13
imul  %r11,%r13

# qhasm:   _2p20 = stack_2p20
# asm 1: movq <stack_2p20=stack64#10,>_2p20=int64#12
# asm 2: movq <stack_2p20=776(%rsp),>_2p20=%r14
movq 776(%rsp),%r14

# qhasm:   r = grs + _2p20
# asm 1: lea  (<grs=int64#8,<_2p20=int64#12),>r=int64#8
# asm 2: lea  (<grs=%r10,<_2p20=%r14),>r=%r10
lea  (%r10,%r14),%r10

# qhasm:   r <<= 22
# asm 1: shl  $22,<r=int64#8
# asm 2: shl  $22,<r=%r10
shl  $22,%r10

# qhasm:   (int64) r >>= 43
# asm 1: sar  $43,<r=int64#8
# asm 2: sar  $43,<r=%r10
sar  $43,%r10

# qhasm:       rax = f
# asm 1: mov  <f=int64#1,>rax=int64#13
# asm 2: mov  <f=%rdi,>rax=%r15
mov  %rdi,%r15

# qhasm:       rax *= r
# asm 1: imul  <r=int64#8,<rax=int64#13
# asm 2: imul  <r=%r10,<rax=%r15
imul  %r10,%r15

# qhasm:   u = fuv + _2p20
# asm 1: lea  (<fuv=int64#7,<_2p20=int64#12),>u=int64#7
# asm 2: lea  (<fuv=%rax,<_2p20=%r14),>u=%rax
lea  (%rax,%r14),%rax

# qhasm:   u <<= 22
# asm 1: shl  $22,<u=int64#7
# asm 2: shl  $22,<u=%rax
shl  $22,%rax

# qhasm:   (int64) u >>= 43
# asm 1: sar  $43,<u=int64#7
# asm 2: sar  $43,<u=%rax
sar  $43,%rax

# qhasm:        f *= u
# asm 1: imul  <u=int64#7,<f=int64#1
# asm 2: imul  <u=%rax,<f=%rdi
imul  %rax,%rdi

# qhasm:        f += t2
# asm 1: add  <t2=int64#11,<f=int64#1
# asm 2: add  <t2=%r13,<f=%rdi
add  %r13,%rdi

# qhasm:        g += rax
# asm 1: add  <rax=int64#13,<g=int64#3
# asm 2: add  <rax=%r15,<g=%rdx
add  %r15,%rdx

# qhasm:   (int64) f >>= 20
# asm 1: sar  $20,<f=int64#1
# asm 2: sar  $20,<f=%rdi
sar  $20,%rdi

# qhasm:   (int64) g >>= 20
# asm 1: sar  $20,<g=int64#3
# asm 2: sar  $20,<g=%rdx
sar  $20,%rdx

# qhasm:   t0 = stack_uuss[0]
# asm 1: movq <stack_uuss=stack256#19,>t0=int64#11
# asm 2: movq <stack_uuss=576(%rsp),>t0=%r13
movq 576(%rsp),%r13

# qhasm:   t0 *= u
# asm 1: imul  <u=int64#7,<t0=int64#11
# asm 2: imul  <u=%rax,<t0=%r13
imul  %rax,%r13

# qhasm:   t1 = stack_vvrr[2]
# asm 1: movq <stack_vvrr=stack256#20,>t1=int64#12
# asm 2: movq <stack_vvrr=624(%rsp),>t1=%r14
movq 624(%rsp),%r14

# qhasm:   t1 *= v
# asm 1: imul  <v=int64#9,<t1=int64#12
# asm 2: imul  <v=%r11,<t1=%r14
imul  %r11,%r14

# qhasm:   rtimesoldv = stack_vvrr[0]
# asm 1: movq <stack_vvrr=stack256#20,>rtimesoldv=int64#13
# asm 2: movq <stack_vvrr=608(%rsp),>rtimesoldv=%r15
movq 608(%rsp),%r15

# qhasm:   u *= rtimesoldv
# asm 1: imul  <rtimesoldv=int64#13,<u=int64#7
# asm 2: imul  <rtimesoldv=%r15,<u=%rax
imul  %r15,%rax

# qhasm:   stimesolds = stack_uuss[2]
# asm 1: movq <stack_uuss=stack256#19,>stimesolds=int64#14
# asm 2: movq <stack_uuss=592(%rsp),>stimesolds=%rbx
movq 592(%rsp),%rbx

# qhasm:   v *= stimesolds
# asm 1: imul  <stimesolds=int64#14,<v=int64#9
# asm 2: imul  <stimesolds=%rbx,<v=%r11
imul  %rbx,%r11

# qhasm:   rtimesoldv *= r
# asm 1: imul  <r=int64#8,<rtimesoldv=int64#13
# asm 2: imul  <r=%r10,<rtimesoldv=%r15
imul  %r10,%r15

# qhasm:   stimesolds *= s
# asm 1: imul  <s=int64#10,<stimesolds=int64#14
# asm 2: imul  <s=%r12,<stimesolds=%rbx
imul  %r12,%rbx

# qhasm:   r *= stack_uuss[0]
# asm 1: imulq <stack_uuss=stack256#19,<r=int64#8
# asm 2: imulq <stack_uuss=576(%rsp),<r=%r10
imulq 576(%rsp),%r10

# qhasm:   s *= stack_vvrr[2]
# asm 1: imulq <stack_vvrr=stack256#20,<s=int64#10
# asm 2: imulq <stack_vvrr=624(%rsp),<s=%r12
imulq 624(%rsp),%r12

# qhasm:   v += u
# asm 1: add  <u=int64#7,<v=int64#9
# asm 2: add  <u=%rax,<v=%r11
add  %rax,%r11

# qhasm:   u = t0 + t1
# asm 1: lea  (<t0=int64#11,<t1=int64#12),>u=int64#7
# asm 2: lea  (<t0=%r13,<t1=%r14),>u=%rax
lea  (%r13,%r14),%rax

# qhasm:   r += s
# asm 1: add  <s=int64#10,<r=int64#8
# asm 2: add  <s=%r12,<r=%r10
add  %r12,%r10

# qhasm:   s = rtimesoldv + stimesolds
# asm 1: lea  (<rtimesoldv=int64#13,<stimesolds=int64#14),>s=int64#10
# asm 2: lea  (<rtimesoldv=%r15,<stimesolds=%rbx),>s=%r12
lea  (%r15,%rbx),%r12

# qhasm: first_loop:
._first_loop:

# qhasm:   inplace stack_vvrr[0] = v
# asm 1: movq <v=int64#9,<stack_vvrr=stack256#20
# asm 2: movq <v=%r11,<stack_vvrr=608(%rsp)
movq %r11,608(%rsp)

# qhasm:   inplace stack_uuss[0] = u
# asm 1: movq <u=int64#7,<stack_uuss=stack256#19
# asm 2: movq <u=%rax,<stack_uuss=576(%rsp)
movq %rax,576(%rsp)

# qhasm:   inplace stack_uuss[2] = s
# asm 1: movq <s=int64#10,<stack_uuss=stack256#19
# asm 2: movq <s=%r12,<stack_uuss=592(%rsp)
movq %r12,592(%rsp)

# qhasm:   inplace stack_vvrr[2] = r
# asm 1: movq <r=int64#8,<stack_vvrr=stack256#20
# asm 2: movq <r=%r10,<stack_vvrr=624(%rsp)
movq %r10,624(%rsp)
# comment:fp stack unchanged by jump

# qhasm: goto loop20 
jmp ._loop20

# qhasm: lastloop:
._lastloop:

# qhasm:   _2p20a2p41 = stack_2p20a2p41
# asm 1: movq <stack_2p20a2p41=stack64#13,>_2p20a2p41=int64#1
# asm 2: movq <stack_2p20a2p41=800(%rsp),>_2p20a2p41=%rdi
movq 800(%rsp),%rdi

# qhasm:   s = grs + _2p20a2p41
# asm 1: lea  (<grs=int64#8,<_2p20a2p41=int64#1),>s=int64#3
# asm 2: lea  (<grs=%r10,<_2p20a2p41=%rdi),>s=%rdx
lea  (%r10,%rdi),%rdx

# qhasm:   (int64) s >>= 42
# asm 1: sar  $42,<s=int64#3
# asm 2: sar  $42,<s=%rdx
sar  $42,%rdx

# qhasm:   v = fuv + _2p20a2p41
# asm 1: lea  (<fuv=int64#7,<_2p20a2p41=int64#1),>v=int64#9
# asm 2: lea  (<fuv=%rax,<_2p20a2p41=%rdi),>v=%r11
lea  (%rax,%rdi),%r11

# qhasm:   (int64) v >>= 42
# asm 1: sar  $42,<v=int64#9
# asm 2: sar  $42,<v=%r11
sar  $42,%r11

# qhasm:   t1 = stack_vvrr[2]
# asm 1: movq <stack_vvrr=stack256#20,>t1=int64#1
# asm 2: movq <stack_vvrr=624(%rsp),>t1=%rdi
movq 624(%rsp),%rdi

# qhasm:   t1 *= v
# asm 1: imul  <v=int64#9,<t1=int64#1
# asm 2: imul  <v=%r11,<t1=%rdi
imul  %r11,%rdi

# qhasm:   stimesolds = stack_uuss[2]
# asm 1: movq <stack_uuss=stack256#19,>stimesolds=int64#6
# asm 2: movq <stack_uuss=592(%rsp),>stimesolds=%r9
movq 592(%rsp),%r9

# qhasm:   v *= stimesolds
# asm 1: imul  <stimesolds=int64#6,<v=int64#9
# asm 2: imul  <stimesolds=%r9,<v=%r11
imul  %r9,%r11

# qhasm:   stimesolds *= s
# asm 1: imul  <s=int64#3,<stimesolds=int64#6
# asm 2: imul  <s=%rdx,<stimesolds=%r9
imul  %rdx,%r9

# qhasm:   _2p20 = stack_2p20
# asm 1: movq <stack_2p20=stack64#10,>_2p20=int64#10
# asm 2: movq <stack_2p20=776(%rsp),>_2p20=%r12
movq 776(%rsp),%r12

# qhasm:   r = grs + _2p20
# asm 1: lea  (<grs=int64#8,<_2p20=int64#10),>r=int64#11
# asm 2: lea  (<grs=%r10,<_2p20=%r12),>r=%r13
lea  (%r10,%r12),%r13

# qhasm:   r <<= 22
# asm 1: shl  $22,<r=int64#11
# asm 2: shl  $22,<r=%r13
shl  $22,%r13

# qhasm:   (int64) r >>= 43
# asm 1: sar  $43,<r=int64#11
# asm 2: sar  $43,<r=%r13
sar  $43,%r13

# qhasm:   u = fuv + _2p20
# asm 1: lea  (<fuv=int64#7,<_2p20=int64#10),>u=int64#7
# asm 2: lea  (<fuv=%rax,<_2p20=%r12),>u=%rax
lea  (%rax,%r12),%rax

# qhasm:   u <<= 22
# asm 1: shl  $22,<u=int64#7
# asm 2: shl  $22,<u=%rax
shl  $22,%rax

# qhasm:   (int64) u >>= 43
# asm 1: sar  $43,<u=int64#7
# asm 2: sar  $43,<u=%rax
sar  $43,%rax

# qhasm:   t0 = stack_uuss[0]
# asm 1: movq <stack_uuss=stack256#19,>t0=int64#8
# asm 2: movq <stack_uuss=576(%rsp),>t0=%r10
movq 576(%rsp),%r10

# qhasm:   t0 *= u
# asm 1: imul  <u=int64#7,<t0=int64#8
# asm 2: imul  <u=%rax,<t0=%r10
imul  %rax,%r10

# qhasm:   rtimesoldv = stack_vvrr[0]
# asm 1: movq <stack_vvrr=stack256#20,>rtimesoldv=int64#10
# asm 2: movq <stack_vvrr=608(%rsp),>rtimesoldv=%r12
movq 608(%rsp),%r12

# qhasm:   u *= rtimesoldv
# asm 1: imul  <rtimesoldv=int64#10,<u=int64#7
# asm 2: imul  <rtimesoldv=%r12,<u=%rax
imul  %r12,%rax

# qhasm:   rtimesoldv *= r
# asm 1: imul  <r=int64#11,<rtimesoldv=int64#10
# asm 2: imul  <r=%r13,<rtimesoldv=%r12
imul  %r13,%r12

# qhasm:   s *= stack_vvrr[2]
# asm 1: imulq <stack_vvrr=stack256#20,<s=int64#3
# asm 2: imulq <stack_vvrr=624(%rsp),<s=%rdx
imulq 624(%rsp),%rdx

# qhasm:   r *= stack_uuss[0]
# asm 1: imulq <stack_uuss=stack256#19,<r=int64#11
# asm 2: imulq <stack_uuss=576(%rsp),<r=%r13
imulq 576(%rsp),%r13

# qhasm:   v += u
# asm 1: add  <u=int64#7,<v=int64#9
# asm 2: add  <u=%rax,<v=%r11
add  %rax,%r11

# qhasm:   u = t0 + t1
# asm 1: lea  (<t0=int64#8,<t1=int64#1),>u=int64#8
# asm 2: lea  (<t0=%r10,<t1=%rdi),>u=%r10
lea  (%r10,%rdi),%r10

# qhasm:   r += s
# asm 1: add  <s=int64#3,<r=int64#11
# asm 2: add  <s=%rdx,<r=%r13
add  %rdx,%r13

# qhasm:   s = rtimesoldv + stimesolds
# asm 1: lea  (<rtimesoldv=int64#10,<stimesolds=int64#6),>s=int64#10
# asm 2: lea  (<rtimesoldv=%r12,<stimesolds=%r9),>s=%r12
lea  (%r12,%r9),%r12

# qhasm:   f = stack_fxgx[0]
# asm 1: movq <stack_fxgx=stack256#21,>f=int64#1
# asm 2: movq <stack_fxgx=640(%rsp),>f=%rdi
movq 640(%rsp),%rdi

# qhasm:   g = stack_fxgx[2]
# asm 1: movq <stack_fxgx=stack256#21,>g=int64#6
# asm 2: movq <stack_fxgx=656(%rsp),>g=%r9
movq 656(%rsp),%r9

# qhasm:   f0 = stack_fygy[0]
# asm 1: movq <stack_fygy=stack256#22,>f0=int64#12
# asm 2: movq <stack_fygy=672(%rsp),>f0=%r14
movq 672(%rsp),%r14

# qhasm:   g0 = stack_fygy[2]
# asm 1: movq <stack_fygy=stack256#22,>g0=int64#13
# asm 2: movq <stack_fygy=688(%rsp),>g0=%r15
movq 688(%rsp),%r15

# qhasm: =? i -= 1
# asm 1: dec <i=int64#5
# asm 2: dec <i=%r8
dec %r8
# comment:fp stack unchanged by jump

# qhasm: goto bigloop if !=
jne ._bigloop

# qhasm: int64 a4

# qhasm: int64 a5

# qhasm: int64 a6

# qhasm: f0 <<= 60
# asm 1: shl  $60,<f0=int64#12
# asm 2: shl  $60,<f0=%r14
shl  $60,%r14

# qhasm: g0 <<= 60
# asm 1: shl  $60,<g0=int64#13
# asm 2: shl  $60,<g0=%r15
shl  $60,%r15

# qhasm: f += f0
# asm 1: add  <f0=int64#12,<f=int64#1
# asm 2: add  <f0=%r14,<f=%rdi
add  %r14,%rdi

# qhasm: g += g0
# asm 1: add  <g0=int64#13,<g=int64#6
# asm 2: add  <g0=%r15,<g=%r9
add  %r15,%r9

# qhasm: f *= u
# asm 1: imul  <u=int64#8,<f=int64#1
# asm 2: imul  <u=%r10,<f=%rdi
imul  %r10,%rdi

# qhasm: g *= v
# asm 1: imul  <v=int64#9,<g=int64#6
# asm 2: imul  <v=%r11,<g=%r9
imul  %r11,%r9

# qhasm: t0 = f + g
# asm 1: lea  (<f=int64#1,<g=int64#6),>t0=int64#1
# asm 2: lea  (<f=%rdi,<g=%r9),>t0=%rdi
lea  (%rdi,%r9),%rdi

# qhasm: (int64) t0 >>= 60
# asm 1: sar  $60,<t0=int64#1
# asm 2: sar  $60,<t0=%rdi
sar  $60,%rdi

# qhasm: u *= t0
# asm 1: imul  <t0=int64#1,<u=int64#8
# asm 2: imul  <t0=%rdi,<u=%r10
imul  %rdi,%r10

# qhasm: v *= t0
# asm 1: imul  <t0=int64#1,<v=int64#9
# asm 2: imul  <t0=%rdi,<v=%r11
imul  %rdi,%r11

# qhasm: cneg:
._cneg:

# qhasm: rax = stack_FVGS8[1]
# asm 1: movq <stack_FVGS8=stack256#10,>rax=int64#7
# asm 2: movq <stack_FVGS8=296(%rsp),>rax=%rax
movq 296(%rsp),%rax

# qhasm: (int128) rdx rax = rax * u
# asm 1: imul <u=int64#8
# asm 2: imul <u=%r10
imul %r10

# qhasm: a4 = rdx
# asm 1: mov  <rdx=int64#3,>a4=int64#1
# asm 2: mov  <rdx=%rdx,>a4=%rdi
mov  %rdx,%rdi

# qhasm: a3 = rax
# asm 1: mov  <rax=int64#7,>a3=int64#2
# asm 2: mov  <rax=%rax,>a3=%rsi
mov  %rax,%rsi

# qhasm: rax = stack_FVGS8[3]
# asm 1: movq <stack_FVGS8=stack256#10,>rax=int64#7
# asm 2: movq <stack_FVGS8=312(%rsp),>rax=%rax
movq 312(%rsp),%rax

# qhasm: (int128) rdx rax = rax * v
# asm 1: imul <v=int64#9
# asm 2: imul <v=%r11
imul %r11

# qhasm: carry? a3 += rax
# asm 1: add  <rax=int64#7,<a3=int64#2
# asm 2: add  <rax=%rax,<a3=%rsi
add  %rax,%rsi

# qhasm:        a4 += rdx + carry
# asm 1: adc <rdx=int64#3,<a4=int64#1
# asm 2: adc <rdx=%rdx,<a4=%rdi
adc %rdx,%rdi

# qhasm: a4 = (a4.a3) << 48
# asm 1: shld $48,<a3=int64#2,<a4=int64#1
# asm 2: shld $48,<a3=%rsi,<a4=%rdi
shld $48,%rsi,%rdi

# qhasm: a3 <<= 48
# asm 1: shl  $48,<a3=int64#2
# asm 2: shl  $48,<a3=%rsi
shl  $48,%rsi

# qhasm: rax = stack_FVGS6[1]
# asm 1: movq <stack_FVGS6=stack256#8,>rax=int64#7
# asm 2: movq <stack_FVGS6=232(%rsp),>rax=%rax
movq 232(%rsp),%rax

# qhasm: t0 = stack_FVGS7[1]
# asm 1: movq <stack_FVGS7=stack256#9,>t0=int64#3
# asm 2: movq <stack_FVGS7=264(%rsp),>t0=%rdx
movq 264(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * u
# asm 1: imul <u=int64#8
# asm 2: imul <u=%r10
imul %r10

# qhasm: t2 = rdx
# asm 1: mov  <rdx=int64#3,>t2=int64#4
# asm 2: mov  <rdx=%rdx,>t2=%rcx
mov  %rdx,%rcx

# qhasm: t1 = rax
# asm 1: mov  <rax=int64#7,>t1=int64#5
# asm 2: mov  <rax=%rax,>t1=%r8
mov  %rax,%r8

# qhasm: rax = stack_FVGS6[3]
# asm 1: movq <stack_FVGS6=stack256#8,>rax=int64#7
# asm 2: movq <stack_FVGS6=248(%rsp),>rax=%rax
movq 248(%rsp),%rax

# qhasm: t0 = stack_FVGS7[3]
# asm 1: movq <stack_FVGS7=stack256#9,>t0=int64#3
# asm 2: movq <stack_FVGS7=280(%rsp),>t0=%rdx
movq 280(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * v
# asm 1: imul <v=int64#9
# asm 2: imul <v=%r11
imul %r11

# qhasm: carry? t1 += rax
# asm 1: add  <rax=int64#7,<t1=int64#5
# asm 2: add  <rax=%rax,<t1=%r8
add  %rax,%r8

# qhasm:        t2 += rdx + carry
# asm 1: adc <rdx=int64#3,<t2=int64#4
# asm 2: adc <rdx=%rdx,<t2=%rcx
adc %rdx,%rcx

# qhasm: a2 = t1
# asm 1: mov  <t1=int64#5,>a2=int64#6
# asm 2: mov  <t1=%r8,>a2=%r9
mov  %r8,%r9

# qhasm: t1 = (t2 t1) >> 12
# asm 1: shrd $12,<t2=int64#4,<t1=int64#5
# asm 2: shrd $12,<t2=%rcx,<t1=%r8
shrd $12,%rcx,%r8

# qhasm: a2 <<= 52
# asm 1: shl  $52,<a2=int64#6
# asm 2: shl  $52,<a2=%r9
shl  $52,%r9

# qhasm: (int64) t2 >>= 12
# asm 1: sar  $12,<t2=int64#4
# asm 2: sar  $12,<t2=%rcx
sar  $12,%rcx

# qhasm: carry? a3 += t1
# asm 1: add  <t1=int64#5,<a3=int64#2
# asm 2: add  <t1=%r8,<a3=%rsi
add  %r8,%rsi

# qhasm:        a4 += t2 + carry
# asm 1: adc <t2=int64#4,<a4=int64#1
# asm 2: adc <t2=%rcx,<a4=%rdi
adc %rcx,%rdi

# qhasm: rax = stack_FVGS4[1]
# asm 1: movq <stack_FVGS4=stack256#6,>rax=int64#7
# asm 2: movq <stack_FVGS4=168(%rsp),>rax=%rax
movq 168(%rsp),%rax

# qhasm: t0 = stack_FVGS5[1]
# asm 1: movq <stack_FVGS5=stack256#7,>t0=int64#3
# asm 2: movq <stack_FVGS5=200(%rsp),>t0=%rdx
movq 200(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * u
# asm 1: imul <u=int64#8
# asm 2: imul <u=%r10
imul %r10

# qhasm: t2 = rdx
# asm 1: mov  <rdx=int64#3,>t2=int64#4
# asm 2: mov  <rdx=%rdx,>t2=%rcx
mov  %rdx,%rcx

# qhasm: t1 = rax
# asm 1: mov  <rax=int64#7,>t1=int64#5
# asm 2: mov  <rax=%rax,>t1=%r8
mov  %rax,%r8

# qhasm: rax = stack_FVGS4[3]
# asm 1: movq <stack_FVGS4=stack256#6,>rax=int64#7
# asm 2: movq <stack_FVGS4=184(%rsp),>rax=%rax
movq 184(%rsp),%rax

# qhasm: t0 = stack_FVGS5[3]
# asm 1: movq <stack_FVGS5=stack256#7,>t0=int64#3
# asm 2: movq <stack_FVGS5=216(%rsp),>t0=%rdx
movq 216(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * v
# asm 1: imul <v=int64#9
# asm 2: imul <v=%r11
imul %r11

# qhasm: carry? t1 += rax
# asm 1: add  <rax=int64#7,<t1=int64#5
# asm 2: add  <rax=%rax,<t1=%r8
add  %rax,%r8

# qhasm:        t2 += rdx + carry
# asm 1: adc <rdx=int64#3,<t2=int64#4
# asm 2: adc <rdx=%rdx,<t2=%rcx
adc %rdx,%rcx

# qhasm: a1 = t1
# asm 1: mov  <t1=int64#5,>a1=int64#10
# asm 2: mov  <t1=%r8,>a1=%r12
mov  %r8,%r12

# qhasm: t1 = (t2 t1) >> 8
# asm 1: shrd $8,<t2=int64#4,<t1=int64#5
# asm 2: shrd $8,<t2=%rcx,<t1=%r8
shrd $8,%rcx,%r8

# qhasm: a1 <<= 56
# asm 1: shl  $56,<a1=int64#10
# asm 2: shl  $56,<a1=%r12
shl  $56,%r12

# qhasm: h = t2
# asm 1: mov  <t2=int64#4,>h=int64#3
# asm 2: mov  <t2=%rcx,>h=%rdx
mov  %rcx,%rdx

# qhasm: (int64) t2 >>= 8
# asm 1: sar  $8,<t2=int64#4
# asm 2: sar  $8,<t2=%rcx
sar  $8,%rcx

# qhasm: (int64) h >>= 63
# asm 1: sar  $63,<h=int64#3
# asm 2: sar  $63,<h=%rdx
sar  $63,%rdx

# qhasm: carry? a2 += t1
# asm 1: add  <t1=int64#5,<a2=int64#6
# asm 2: add  <t1=%r8,<a2=%r9
add  %r8,%r9

# qhasm: carry? a3 += t2 + carry
# asm 1: adc <t2=int64#4,<a3=int64#2
# asm 2: adc <t2=%rcx,<a3=%rsi
adc %rcx,%rsi

# qhasm:        a4 += h + carry
# asm 1: adc <h=int64#3,<a4=int64#1
# asm 2: adc <h=%rdx,<a4=%rdi
adc %rdx,%rdi

# qhasm: rax = stack_FVGS2[1]
# asm 1: movq <stack_FVGS2=stack256#4,>rax=int64#7
# asm 2: movq <stack_FVGS2=104(%rsp),>rax=%rax
movq 104(%rsp),%rax

# qhasm: t0 = stack_FVGS3[1]
# asm 1: movq <stack_FVGS3=stack256#5,>t0=int64#3
# asm 2: movq <stack_FVGS3=136(%rsp),>t0=%rdx
movq 136(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * u
# asm 1: imul <u=int64#8
# asm 2: imul <u=%r10
imul %r10

# qhasm: t2 = rdx
# asm 1: mov  <rdx=int64#3,>t2=int64#4
# asm 2: mov  <rdx=%rdx,>t2=%rcx
mov  %rdx,%rcx

# qhasm: t1 = rax
# asm 1: mov  <rax=int64#7,>t1=int64#5
# asm 2: mov  <rax=%rax,>t1=%r8
mov  %rax,%r8

# qhasm: rax = stack_FVGS2[3]
# asm 1: movq <stack_FVGS2=stack256#4,>rax=int64#7
# asm 2: movq <stack_FVGS2=120(%rsp),>rax=%rax
movq 120(%rsp),%rax

# qhasm: t0 = stack_FVGS3[3]
# asm 1: movq <stack_FVGS3=stack256#5,>t0=int64#3
# asm 2: movq <stack_FVGS3=152(%rsp),>t0=%rdx
movq 152(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * v
# asm 1: imul <v=int64#9
# asm 2: imul <v=%r11
imul %r11

# qhasm: carry? t1 += rax
# asm 1: add  <rax=int64#7,<t1=int64#5
# asm 2: add  <rax=%rax,<t1=%r8
add  %rax,%r8

# qhasm:        t2 += rdx + carry
# asm 1: adc <rdx=int64#3,<t2=int64#4
# asm 2: adc <rdx=%rdx,<t2=%rcx
adc %rdx,%rcx

# qhasm: a0 = t1
# asm 1: mov  <t1=int64#5,>a0=int64#11
# asm 2: mov  <t1=%r8,>a0=%r13
mov  %r8,%r13

# qhasm: t1 = (t2 t1) >> 4
# asm 1: shrd $4,<t2=int64#4,<t1=int64#5
# asm 2: shrd $4,<t2=%rcx,<t1=%r8
shrd $4,%rcx,%r8

# qhasm: a0 <<= 60
# asm 1: shl  $60,<a0=int64#11
# asm 2: shl  $60,<a0=%r13
shl  $60,%r13

# qhasm: h = t2
# asm 1: mov  <t2=int64#4,>h=int64#3
# asm 2: mov  <t2=%rcx,>h=%rdx
mov  %rcx,%rdx

# qhasm: (int64) t2 >>= 4
# asm 1: sar  $4,<t2=int64#4
# asm 2: sar  $4,<t2=%rcx
sar  $4,%rcx

# qhasm: (int64) h >>= 63
# asm 1: sar  $63,<h=int64#3
# asm 2: sar  $63,<h=%rdx
sar  $63,%rdx

# qhasm: carry? a1 += t1
# asm 1: add  <t1=int64#5,<a1=int64#10
# asm 2: add  <t1=%r8,<a1=%r12
add  %r8,%r12

# qhasm: carry? a2 += t2 + carry
# asm 1: adc <t2=int64#4,<a2=int64#6
# asm 2: adc <t2=%rcx,<a2=%r9
adc %rcx,%r9

# qhasm: carry? a3 += h + carry
# asm 1: adc <h=int64#3,<a3=int64#2
# asm 2: adc <h=%rdx,<a3=%rsi
adc %rdx,%rsi

# qhasm:        a4 += h + carry
# asm 1: adc <h=int64#3,<a4=int64#1
# asm 2: adc <h=%rdx,<a4=%rdi
adc %rdx,%rdi

# qhasm: rax = stack_FVGS0[1]
# asm 1: movq <stack_FVGS0=stack256#2,>rax=int64#7
# asm 2: movq <stack_FVGS0=40(%rsp),>rax=%rax
movq 40(%rsp),%rax

# qhasm: t0 = stack_FVGS1[1]
# asm 1: movq <stack_FVGS1=stack256#3,>t0=int64#3
# asm 2: movq <stack_FVGS1=72(%rsp),>t0=%rdx
movq 72(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * u
# asm 1: imul <u=int64#8
# asm 2: imul <u=%r10
imul %r10

# qhasm: t2 = rdx
# asm 1: mov  <rdx=int64#3,>t2=int64#4
# asm 2: mov  <rdx=%rdx,>t2=%rcx
mov  %rdx,%rcx

# qhasm: t1 = rax
# asm 1: mov  <rax=int64#7,>t1=int64#5
# asm 2: mov  <rax=%rax,>t1=%r8
mov  %rax,%r8

# qhasm: rax = stack_FVGS0[3]
# asm 1: movq <stack_FVGS0=stack256#2,>rax=int64#7
# asm 2: movq <stack_FVGS0=56(%rsp),>rax=%rax
movq 56(%rsp),%rax

# qhasm: t0 = stack_FVGS1[3]
# asm 1: movq <stack_FVGS1=stack256#3,>t0=int64#3
# asm 2: movq <stack_FVGS1=88(%rsp),>t0=%rdx
movq 88(%rsp),%rdx

# qhasm: t0 <<= 30
# asm 1: shl  $30,<t0=int64#3
# asm 2: shl  $30,<t0=%rdx
shl  $30,%rdx

# qhasm: rax += t0
# asm 1: add  <t0=int64#3,<rax=int64#7
# asm 2: add  <t0=%rdx,<rax=%rax
add  %rdx,%rax

# qhasm: (int128) rdx rax = rax * v
# asm 1: imul <v=int64#9
# asm 2: imul <v=%r11
imul %r11

# qhasm: carry? t1 += rax
# asm 1: add  <rax=int64#7,<t1=int64#5
# asm 2: add  <rax=%rax,<t1=%r8
add  %rax,%r8

# qhasm:        t2 += rdx + carry
# asm 1: adc <rdx=int64#3,<t2=int64#4
# asm 2: adc <rdx=%rdx,<t2=%rcx
adc %rdx,%rcx

# qhasm: h = t2
# asm 1: mov  <t2=int64#4,>h=int64#3
# asm 2: mov  <t2=%rcx,>h=%rdx
mov  %rcx,%rdx

# qhasm: (int64) h >>= 63
# asm 1: sar  $63,<h=int64#3
# asm 2: sar  $63,<h=%rdx
sar  $63,%rdx

# qhasm: carry? a0 += t1
# asm 1: add  <t1=int64#5,<a0=int64#11
# asm 2: add  <t1=%r8,<a0=%r13
add  %r8,%r13

# qhasm: carry? a1 += t2 + carry
# asm 1: adc <t2=int64#4,<a1=int64#10
# asm 2: adc <t2=%rcx,<a1=%r12
adc %rcx,%r12

# qhasm: carry? a2 += h + carry
# asm 1: adc <h=int64#3,<a2=int64#6
# asm 2: adc <h=%rdx,<a2=%r9
adc %rdx,%r9

# qhasm: carry? a3 += h + carry
# asm 1: adc <h=int64#3,<a3=int64#2
# asm 2: adc <h=%rdx,<a3=%rsi
adc %rdx,%rsi

# qhasm:        a4 += h + carry
# asm 1: adc <h=int64#3,<a4=int64#1
# asm 2: adc <h=%rdx,<a4=%rdi
adc %rdx,%rdi

# qhasm: carry? a3 reset bit 63
# asm 1: btr  $63,<a3=int64#2
# asm 2: btr  $63,<a3=%rsi
btr  $63,%rsi

# qhasm: a4 += a4 + carry
# asm 1: adc <a4=int64#1,<a4=int64#1
# asm 2: adc <a4=%rdi,<a4=%rdi
adc %rdi,%rdi

# qhasm: h = a4
# asm 1: mov  <a4=int64#1,>h=int64#4
# asm 2: mov  <a4=%rdi,>h=%rcx
mov  %rdi,%rcx

# qhasm: rax = 19
# asm 1: mov  $19,>rax=int64#7
# asm 2: mov  $19,>rax=%rax
mov  $19,%rax

# qhasm: (int128) rdx rax = rax * a4
# asm 1: imul <a4=int64#1
# asm 2: imul <a4=%rdi
imul %rdi

# qhasm: (int64) h >>= 63
# asm 1: sar  $63,<h=int64#4
# asm 2: sar  $63,<h=%rcx
sar  $63,%rcx

# qhasm: a4 = 0
# asm 1: xor  >a4=int64#1,>a4=int64#1
# asm 2: xor  >a4=%rdi,>a4=%rdi
xor  %rdi,%rdi

# qhasm: carry?   a0 += rax
# asm 1: add  <rax=int64#7,<a0=int64#11
# asm 2: add  <rax=%rax,<a0=%r13
add  %rax,%r13

# qhasm: carry?   a1 += rdx + carry
# asm 1: adc <rdx=int64#3,<a1=int64#10
# asm 2: adc <rdx=%rdx,<a1=%r12
adc %rdx,%r12

# qhasm: carry?   a2 += h + carry
# asm 1: adc <h=int64#4,<a2=int64#6
# asm 2: adc <h=%rcx,<a2=%r9
adc %rcx,%r9

# qhasm: carry?   a3 += h + carry
# asm 1: adc <h=int64#4,<a3=int64#2
# asm 2: adc <h=%rcx,<a3=%rsi
adc %rcx,%rsi

# qhasm:          a4 += h + carry
# asm 1: adc <h=int64#4,<a4=int64#1
# asm 2: adc <h=%rcx,<a4=%rdi
adc %rcx,%rdi

# qhasm: carry? a3 reset bit 63
# asm 1: btr  $63,<a3=int64#2
# asm 2: btr  $63,<a3=%rsi
btr  $63,%rsi

# qhasm: a4 += a4 + carry
# asm 1: adc <a4=int64#1,<a4=int64#1
# asm 2: adc <a4=%rdi,<a4=%rdi
adc %rdi,%rdi

# qhasm: h = a4
# asm 1: mov  <a4=int64#1,>h=int64#4
# asm 2: mov  <a4=%rdi,>h=%rcx
mov  %rdi,%rcx

# qhasm: rax = 19
# asm 1: mov  $19,>rax=int64#7
# asm 2: mov  $19,>rax=%rax
mov  $19,%rax

# qhasm: (int128) rdx rax = rax * a4
# asm 1: imul <a4=int64#1
# asm 2: imul <a4=%rdi
imul %rdi

# qhasm: (int64) h >>= 63
# asm 1: sar  $63,<h=int64#4
# asm 2: sar  $63,<h=%rcx
sar  $63,%rcx

# qhasm: carry?   a0 += rax
# asm 1: add  <rax=int64#7,<a0=int64#11
# asm 2: add  <rax=%rax,<a0=%r13
add  %rax,%r13

# qhasm: carry?   a1 += rdx + carry
# asm 1: adc <rdx=int64#3,<a1=int64#10
# asm 2: adc <rdx=%rdx,<a1=%r12
adc %rdx,%r12

# qhasm: carry?   a2 += h + carry
# asm 1: adc <h=int64#4,<a2=int64#6
# asm 2: adc <h=%rcx,<a2=%r9
adc %rcx,%r9

# qhasm: carry?   a3 += h + carry
# asm 1: adc <h=int64#4,<a3=int64#2
# asm 2: adc <h=%rcx,<a3=%rsi
adc %rcx,%rsi

# qhasm:          a4 += h + carry
# asm 1: adc <h=int64#4,<a4=int64#1
# asm 2: adc <h=%rcx,<a4=%rdi
adc %rcx,%rdi

# qhasm:   z = 0
# asm 1: xor  >z=int64#1,>z=int64#1
# asm 2: xor  >z=%rdi,>z=%rdi
xor  %rdi,%rdi

# qhasm: a4 = -19
# asm 1: mov  $-19,>a4=int64#3
# asm 2: mov  $-19,>a4=%rdx
mov  $-19,%rdx

# qhasm: a5 = -1
# asm 1: mov  $-1,>a5=int64#4
# asm 2: mov  $-1,>a5=%rcx
mov  $-1,%rcx

# qhasm: a6 = 9223372036854775807
# asm 1: mov  $9223372036854775807,>a6=int64#5
# asm 2: mov  $9223372036854775807,>a6=%r8
mov  $9223372036854775807,%r8

# qhasm: signed<? a3 - 0
# asm 1: cmp  $0,<a3=int64#2
# asm 2: cmp  $0,<a3=%rsi
cmp  $0,%rsi

# qhasm: a4 = z if !signed<
# asm 1: cmovge <z=int64#1,<a4=int64#3
# asm 2: cmovge <z=%rdi,<a4=%rdx
cmovge %rdi,%rdx

# qhasm: a5 = z if !signed<
# asm 1: cmovge <z=int64#1,<a5=int64#4
# asm 2: cmovge <z=%rdi,<a5=%rcx
cmovge %rdi,%rcx

# qhasm: a6 = z if !signed<
# asm 1: cmovge <z=int64#1,<a6=int64#5
# asm 2: cmovge <z=%rdi,<a6=%r8
cmovge %rdi,%r8

# qhasm: carry? a0 += a4
# asm 1: add  <a4=int64#3,<a0=int64#11
# asm 2: add  <a4=%rdx,<a0=%r13
add  %rdx,%r13

# qhasm: carry? a1 += a5 + carry
# asm 1: adc <a5=int64#4,<a1=int64#10
# asm 2: adc <a5=%rcx,<a1=%r12
adc %rcx,%r12

# qhasm: carry? a2 += a5 + carry
# asm 1: adc <a5=int64#4,<a2=int64#6
# asm 2: adc <a5=%rcx,<a2=%r9
adc %rcx,%r9

# qhasm:        a3 += a6 + carry
# asm 1: adc <a6=int64#5,<a3=int64#2
# asm 2: adc <a6=%r8,<a3=%rsi
adc %r8,%rsi

# qhasm: carry?   a0 += 19
# asm 1: add  $19,<a0=int64#11
# asm 2: add  $19,<a0=%r13
add  $19,%r13

# qhasm: carry?   a1 += 0 + carry
# asm 1: adc $0,<a1=int64#10
# asm 2: adc $0,<a1=%r12
adc $0,%r12

# qhasm: carry?   a2 += 0 + carry
# asm 1: adc $0,<a2=int64#6
# asm 2: adc $0,<a2=%r9
adc $0,%r9

# qhasm:          a3 += 0 + carry
# asm 1: adc $0,<a3=int64#2
# asm 2: adc $0,<a3=%rsi
adc $0,%rsi

# qhasm: t0 = a3
# asm 1: mov  <a3=int64#2,>t0=int64#1
# asm 2: mov  <a3=%rsi,>t0=%rdi
mov  %rsi,%rdi

# qhasm: (int64) t0 >>= 63
# asm 1: sar  $63,<t0=int64#1
# asm 2: sar  $63,<t0=%rdi
sar  $63,%rdi

# qhasm: a3 reset bit 63
# asm 1: btr  $63,<a3=int64#2
# asm 2: btr  $63,<a3=%rsi
btr  $63,%rsi

# qhasm: t0 &= 19
# asm 1: and  $19,<t0=int64#1
# asm 2: and  $19,<t0=%rdi
and  $19,%rdi

# qhasm: carry? a0 += t0
# asm 1: add  <t0=int64#1,<a0=int64#11
# asm 2: add  <t0=%rdi,<a0=%r13
add  %rdi,%r13

# qhasm: carry? a1 += 0 + carry
# asm 1: adc $0,<a1=int64#10
# asm 2: adc $0,<a1=%r12
adc $0,%r12

# qhasm: carry? a2 += 0 + carry
# asm 1: adc $0,<a2=int64#6
# asm 2: adc $0,<a2=%r9
adc $0,%r9

# qhasm:        a3 += 0 + carry
# asm 1: adc $0,<a3=int64#2
# asm 2: adc $0,<a3=%rsi
adc $0,%rsi

# qhasm: carry?   a0 -= 19
# asm 1: sub  $19,<a0=int64#11
# asm 2: sub  $19,<a0=%r13
sub  $19,%r13

# qhasm: carry?   a1 -= 0 - carry
# asm 1: sbb  $0,<a1=int64#10
# asm 2: sbb  $0,<a1=%r12
sbb  $0,%r12

# qhasm: carry?   a2 -= 0 - carry
# asm 1: sbb  $0,<a2=int64#6
# asm 2: sbb  $0,<a2=%r9
sbb  $0,%r9

# qhasm: 	 a3 -= 0 - carry
# asm 1: sbb  $0,<a3=int64#2
# asm 2: sbb  $0,<a3=%rsi
sbb  $0,%rsi

# qhasm: t2 = stack_out
# asm 1: movq <stack_out=stack64#1,>t2=int64#1
# asm 2: movq <stack_out=704(%rsp),>t2=%rdi
movq 704(%rsp),%rdi

# qhasm: mem64[t2 +  0] = a0
# asm 1: movq   <a0=int64#11,0(<t2=int64#1)
# asm 2: movq   <a0=%r13,0(<t2=%rdi)
movq   %r13,0(%rdi)

# qhasm: mem64[t2 +  8] = a1
# asm 1: movq   <a1=int64#10,8(<t2=int64#1)
# asm 2: movq   <a1=%r12,8(<t2=%rdi)
movq   %r12,8(%rdi)

# qhasm: mem64[t2 + 16] = a2
# asm 1: movq   <a2=int64#6,16(<t2=int64#1)
# asm 2: movq   <a2=%r9,16(<t2=%rdi)
movq   %r9,16(%rdi)

# qhasm: mem64[t2 + 24] = a3
# asm 1: movq   <a3=int64#2,24(<t2=int64#1)
# asm 2: movq   <a3=%rsi,24(<t2=%rdi)
movq   %rsi,24(%rdi)

# qhasm: caller_r11 = stack_r11
# asm 1: movq <stack_r11=stack64#2,>caller_r11=int64#9
# asm 2: movq <stack_r11=712(%rsp),>caller_r11=%r11
movq 712(%rsp),%r11

# qhasm: caller_r12 = stack_r12
# asm 1: movq <stack_r12=stack64#3,>caller_r12=int64#10
# asm 2: movq <stack_r12=720(%rsp),>caller_r12=%r12
movq 720(%rsp),%r12

# qhasm: caller_r13 = stack_r13
# asm 1: movq <stack_r13=stack64#4,>caller_r13=int64#11
# asm 2: movq <stack_r13=728(%rsp),>caller_r13=%r13
movq 728(%rsp),%r13

# qhasm: caller_r14 = stack_r14
# asm 1: movq <stack_r14=stack64#5,>caller_r14=int64#12
# asm 2: movq <stack_r14=736(%rsp),>caller_r14=%r14
movq 736(%rsp),%r14

# qhasm: caller_r15 = stack_r15
# asm 1: movq <stack_r15=stack64#6,>caller_r15=int64#13
# asm 2: movq <stack_r15=744(%rsp),>caller_r15=%r15
movq 744(%rsp),%r15

# qhasm: caller_rbx = stack_rbx
# asm 1: movq <stack_rbx=stack64#7,>caller_rbx=int64#14
# asm 2: movq <stack_rbx=752(%rsp),>caller_rbx=%rbx
movq 752(%rsp),%rbx

# qhasm: caller_rbp = stack_rbp
# asm 1: movq <stack_rbp=stack64#8,>caller_rbp=int64#15
# asm 2: movq <stack_rbp=760(%rsp),>caller_rbp=%rbp
movq 760(%rsp),%rbp

# qhasm: return
add %r11,%rsp
ret
