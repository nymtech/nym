// linker define fe25519_mul
// linker use mask51

/* Assembly for field multiplication. */

#define mask51 CRYPTO_SHARED_NAMESPACE(mask51)

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(fe25519_mul)
.globl CRYPTO_SHARED_NAMESPACE(fe25519_mul)
_CRYPTO_SHARED_NAMESPACE(fe25519_mul):
CRYPTO_SHARED_NAMESPACE(fe25519_mul):

movq    %rsp,%r11
andq 	$-32,%rsp
subq    $64,%rsp

movq   %r11,0(%rsp)
movq   %r12,8(%rsp)
movq   %r13,16(%rsp)
movq   %r14,24(%rsp)
movq   %r15,32(%rsp)
movq   %rbx,40(%rsp)
movq   %rbp,48(%rsp)
movq   %rdi,56(%rsp)

movq   %rdx,%rcx

movq   0(%rsi),%rax
mulq   0(%rcx)
movq   %rax,%r8
movq   %rdx,%r9

movq   0(%rsi),%rax
mulq   8(%rcx)
movq   %rax,%r10
movq   %rdx,%r11

movq   8(%rsi),%rax
mulq   0(%rcx)
addq   %rax,%r10
adcq   %rdx,%r11

movq   0(%rsi),%rax
mulq   16(%rcx)
movq   %rax,%r12
movq   %rdx,%r13

movq   8(%rsi),%rax
mulq   8(%rcx)
addq   %rax,%r12
adcq   %rdx,%r13

movq   16(%rsi),%rax
mulq   0(%rcx)
addq   %rax,%r12
adcq   %rdx,%r13

movq   0(%rsi),%rax
mulq   24(%rcx)
movq   %rax,%r14
movq   %rdx,%r15

movq   8(%rsi),%rax
mulq   16(%rcx)
addq   %rax,%r14
adcq   %rdx,%r15

movq   16(%rsi),%rax
mulq   8(%rcx)
addq   %rax,%r14
adcq   %rdx,%r15

movq   24(%rsi),%rax
mulq   0(%rcx)
addq   %rax,%r14
adcq   %rdx,%r15

movq   0(%rsi),%rax
mulq   32(%rcx)
movq   %rax,%rbx
movq   %rdx,%rbp

movq   8(%rsi),%rax
mulq   24(%rcx)
addq   %rax,%rbx
adcq   %rdx,%rbp

movq   16(%rsi),%rax
mulq   16(%rcx)
addq   %rax,%rbx
adcq   %rdx,%rbp

movq   24(%rsi),%rax
mulq   8(%rcx)
addq   %rax,%rbx
adcq   %rdx,%rbp

movq   32(%rsi),%rax
mulq   0(%rcx)
addq   %rax,%rbx
adcq   %rdx,%rbp

shld   $13,%rbx,%rbp

imul   $19,8(%rsi),%rax
mulq   32(%rcx)
addq   %rax,%r8
adcq   %rdx,%r9

imul   $19,16(%rsi),%rax
mulq   24(%rcx)
addq   %rax,%r8
adcq   %rdx,%r9

imul   $19,24(%rsi),%rax
mulq   16(%rcx)
addq   %rax,%r8
adcq   %rdx,%r9

imul   $19,32(%rsi),%rax
mulq   8(%rcx)
addq   %rax,%r8
adcq   %rdx,%r9

shld   $13,%r8,%r9

imul   $19,16(%rsi),%rax
mulq   32(%rcx)
addq   %rax,%r10
adcq   %rdx,%r11

imul   $19,24(%rsi),%rax
mulq   24(%rcx)
addq   %rax,%r10
adcq   %rdx,%r11

imul   $19,32(%rsi),%rax
mulq   16(%rcx)
addq   %rax,%r10
adcq   %rdx,%r11

shld   $13,%r10,%r11

imul   $19,24(%rsi),%rax
mulq   32(%rcx)
addq   %rax,%r12
adcq   %rdx,%r13

imul   $19,32(%rsi),%rax
mulq   24(%rcx)
addq   %rax,%r12
adcq   %rdx,%r13

shld   $13,%r12,%r13

imul   $19,32(%rsi),%rax
mulq   32(%rcx)
addq   %rax,%r14
adcq   %rdx,%r15

shld   $13,%r14,%r15

imul   $19,%rbp,%rbp

movq   mask51(%rip),%rdx

andq   %rdx,%r8
andq   %rdx,%r10
andq   %rdx,%r12
andq   %rdx,%r14
andq   %rdx,%rbx

addq   %r9,%r10
addq   %r11,%r12
addq   %r13,%r14
addq   %r15,%rbx
addq   %rbp,%r8

movq   %r8,%rax
shrq   $51,%rax
addq   %r10,%rax
andq   %rdx,%r8

movq   %rax,%r9
shrq   $51,%rax
addq   %r12,%rax
andq   %rdx,%r9

movq   %rax,%r11
shrq   $51,%rax
addq   %r14,%rax
andq   %rdx,%r11

movq   %rax,%r13
shrq   $51,%rax
addq   %rbx,%rax
andq   %rdx,%r13

movq   %rax,%r15
shrq   $51,%rax
imul   $19,%rax ,%rax
addq   %r8,%rax
andq   %rdx,%r15

movq   %rax,0(%rdi)
movq   %r9,8(%rdi)
movq   %r11,16(%rdi)
movq   %r13,24(%rdi)
movq   %r15,32(%rdi)

movq   0(%rsp),%r11
movq   8(%rsp),%r12
movq   16(%rsp),%r13
movq   24(%rsp),%r14
movq   32(%rsp),%r15
movq   40(%rsp),%rbx
movq   48(%rsp),%rbp

movq   %r11,%rsp

ret
