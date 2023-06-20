// linker define fe25519_square
// linker use mask63

/* Assembly for field squaring. */

#define mask63 CRYPTO_SHARED_NAMESPACE(mask63)

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(fe25519_square)
.globl CRYPTO_SHARED_NAMESPACE(fe25519_square)
_CRYPTO_SHARED_NAMESPACE(fe25519_square):
CRYPTO_SHARED_NAMESPACE(fe25519_square):

movq    %rsp,%r11
andq 	$-32,%rsp
subq    $56,%rsp

movq    %r11,0(%rsp)
movq    %r12,8(%rsp)
movq    %r13,16(%rsp)
movq    %r14,24(%rsp)
movq    %r15,32(%rsp)
movq    %rbx,40(%rsp)
movq    %rbp,48(%rsp)

movq    24(%rsi),%rax
mulq    24(%rsi)
movq    %rax,%r12
movq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    8(%rsi),%rax
mulq    24(%rsi)
movq    %rax,%r8
movq    $0,%r9
movq    %rdx,%r10
movq    $0,%r11

addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    16(%rsi),%rax
mulq    16(%rsi)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    16(%rsi),%rax
mulq    24(%rsi)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    0(%rsi),%rax
mulq    24(%rsi)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    8(%rsi),%rax
mulq    16(%rsi)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    0(%rsi),%rax
mulq    0(%rsi)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    0(%rsi),%rax
mulq    8(%rsi)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    0(%rsi),%rax
mulq    16(%rsi)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    8(%rsi),%rax
mulq    8(%rsi)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,0(%rdi)
movq    %r10,8(%rdi)
movq    %r12,16(%rdi)
movq    %r14,24(%rdi)

movq    0(%rsp),%r11
movq    8(%rsp),%r12
movq    16(%rsp),%r13
movq    24(%rsp),%r14
movq    32(%rsp),%r15
movq    40(%rsp),%rbx
movq    48(%rsp),%rbp

movq    %r11,%rsp

ret
