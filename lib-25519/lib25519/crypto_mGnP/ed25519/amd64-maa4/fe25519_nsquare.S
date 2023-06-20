// linker define fe25519_nsquare
// linker use mask63

/* Assembly for feedback field squaring. */

#define mask63 CRYPTO_SHARED_NAMESPACE(mask63)

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(fe25519_nsquare)
.globl CRYPTO_SHARED_NAMESPACE(fe25519_nsquare)
_CRYPTO_SHARED_NAMESPACE(fe25519_nsquare):
CRYPTO_SHARED_NAMESPACE(fe25519_nsquare):

movq    %rsp,%r11
andq 	$-32,%rsp
subq    $64,%rsp

movq    %r11,0(%rsp)
movq    %r12,8(%rsp)
movq    %r13,16(%rsp)
movq    %r14,24(%rsp)
movq    %r15,32(%rsp)
movq    %rbx,40(%rsp)
movq    %rbp,48(%rsp)
movq    %rdi,56(%rsp)

movq    0(%rsi),%rbx
movq    8(%rsi),%rbp
movq    16(%rsi),%rcx
movq    24(%rsi),%rsi

movq    %rdx,%rdi

.L:

movq    %rsi,%rax
mulq    %rsi
movq    %rax,%r12
movq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    %rbp,%rax
mulq    %rsi
movq    %rax,%r8
movq    $0,%r9
movq    %rdx,%r10
movq    $0,%r11

addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    %rcx,%rax
mulq    %rcx
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    %rcx,%rax
mulq    %rsi
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

movq    %rbx,%rax
mulq    %rsi
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    %rbp,%rax
mulq    %rcx
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

movq    %rbx,%rax
mulq    %rbx
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    %rbx,%rax
mulq    %rbp
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    %rbx,%rax
mulq    %rcx
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    %rbp,%rax
mulq    %rbp
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    %r10,%rbp
addq    %r9,%rbp
adcq    $0,%r11

movq    %r12,%rcx
addq    %r11,%rcx
adcq    $0,%r13

movq    %r14,%rsi
addq    %r13,%rsi
adcq    $0,%r15

shld    $1,%rsi,%r15
imul    $19,%r15,%rbx
andq    mask63(%rip),%rsi

addq    %r8,%rbx
adcq    $0,%rbp
adcq    $0,%rcx
adcq    $0,%rsi

subq    $1,%rdi
cmpq    $0,%rdi

jne     .L

movq    56(%rsp),%rdi

movq    %rbx,0(%rdi)
movq    %rbp,8(%rdi)
movq    %rcx,16(%rdi)
movq    %rsi,24(%rdi)

movq    0(%rsp),%r11
movq    8(%rsp),%r12
movq    16(%rsp),%r13
movq    24(%rsp),%r14
movq    32(%rsp),%r15
movq    40(%rsp),%rbx
movq    48(%rsp),%rbp

movq    %r11,%rsp

ret
