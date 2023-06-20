// linker define fe25519_mul
// linker use mask63

/* Assembly for field multiplication. */

#define mask63 CRYPTO_SHARED_NAMESPACE(mask63)

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(fe25519_mul)
.globl CRYPTO_SHARED_NAMESPACE(fe25519_mul)
_CRYPTO_SHARED_NAMESPACE(fe25519_mul):
CRYPTO_SHARED_NAMESPACE(fe25519_mul):

movq    %rsp,%r11
andq 	$-32,%rsp
subq    $64,%rsp

movq 	%r11,0(%rsp)
movq 	%r12,8(%rsp)
movq 	%r13,16(%rsp)
movq 	%r14,24(%rsp)
movq 	%r15,32(%rsp)
movq 	%rbp,40(%rsp)
movq 	%rbx,48(%rsp)
movq 	%rdi,56(%rsp)

movq    %rdx,%rdi

movq    0(%rdi),%rdx    

mulx    0(%rsi),%r8,%r9
mulx    8(%rsi),%rcx,%r10
addq    %rcx,%r9

mulx    16(%rsi),%rcx,%r11
adcq    %rcx,%r10

mulx    24(%rsi),%rcx,%r12
adcq    %rcx,%r11
adcq    $0,%r12

movq    8(%rdi),%rdx    

mulx    0(%rsi),%rax,%rbx
mulx    8(%rsi),%rcx,%rbp
addq    %rcx,%rbx

mulx    16(%rsi),%rcx,%r15
adcq    %rcx,%rbp

mulx    24(%rsi),%rcx,%r13
adcq    %rcx,%r15
adcq    $0,%r13

addq    %rax,%r9
adcq    %rbx,%r10
adcq    %rbp,%r11
adcq    %r15,%r12
adcq    $0,%r13

movq    16(%rdi),%rdx

mulx    0(%rsi),%rax,%rbx
mulx    8(%rsi),%rcx,%rbp
addq    %rcx,%rbx

mulx    16(%rsi),%rcx,%r15
adcq    %rcx,%rbp

mulx    24(%rsi),%rcx,%r14
adcq    %rcx,%r15
adcq    $0,%r14

addq    %rax,%r10
adcq    %rbx,%r11
adcq    %rbp,%r12
adcq    %r15,%r13
adcq    $0,%r14

movq    24(%rdi),%rdx

mulx    0(%rsi),%rax,%rbx
mulx    8(%rsi),%rcx,%rbp
addq    %rcx,%rbx

mulx    16(%rsi),%rcx,%r15
adcq    %rcx,%rbp

mulx    24(%rsi),%rcx,%rsi
adcq    %rcx,%r15
adcq    $0,%rsi

addq    %rax,%r11
adcq    %rbx,%r12
adcq    %rbp,%r13
adcq    %r15,%r14
adcq    $0,%rsi

movq    $38,%rdx

mulx    %r12,%r12,%rbx
mulx    %r13,%r13,%rcx
addq    %rbx,%r13

mulx    %r14,%r14,%rbx
adcq    %rcx,%r14

mulx    %rsi,%r15,%rcx
adcq    %rbx,%r15
adcq    $0,%rcx

addq    %r12,%r8
adcq    %r13,%r9
adcq    %r14,%r10
adcq    %r15,%r11
adcq    $0,%rcx

shld    $1,%r11,%rcx
andq	mask63(%rip),%r11

imul    $19,%rcx,%rcx
addq    %rcx,%r8
adcq    $0,%r9
adcq    $0,%r10
adcq    $0,%r11

movq 	56(%rsp),%rdi

movq    %r8,0(%rdi)
movq    %r9,8(%rdi)
movq    %r10,16(%rdi)
movq    %r11,24(%rdi)

movq 	 0(%rsp),%r11
movq 	 8(%rsp),%r12
movq 	16(%rsp),%r13
movq 	24(%rsp),%r14
movq 	32(%rsp),%r15
movq 	40(%rsp),%rbp
movq 	48(%rsp),%rbx

movq 	%r11,%rsp

ret
