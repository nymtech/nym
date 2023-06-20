// linker define fe25519_nsquare
// linker use mask51

/* Assembly for feedback field squaring. */

#define mask51 CRYPTO_SHARED_NAMESPACE(mask51)

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(fe25519_nsquare)
.globl CRYPTO_SHARED_NAMESPACE(fe25519_nsquare)
_CRYPTO_SHARED_NAMESPACE(fe25519_nsquare):
CRYPTO_SHARED_NAMESPACE(fe25519_nsquare):

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

movq   0(%rsi),%r8
movq   8(%rsi),%r9
movq   16(%rsi),%r10
movq   24(%rsi),%r11
movq   32(%rsi),%r12

movq   %r10,16(%rdi)
movq   %r11,24(%rdi)
movq   %r12,32(%rdi)

movq   %rdx,%rsi

.L:

subq   $1,%rsi

movq   %r8,%rax
mulq   %r8
movq   %rax,%r10
movq   %rdx,%r11

shlq   $1,%r8
movq   %r8,%rax
mulq   %r9
movq   %rax,%r12
movq   %rdx,%r13

movq   %r8,%rax
mulq   16(%rdi)
movq   %rax,%r14
movq   %rdx,%r15

movq   %r9,%rax
mulq   %r9
addq   %rax,%r14
adcq   %rdx,%r15

movq   %r8,%rax
mulq   24(%rdi)
movq   %rax,%rbx
movq   %rdx,%rbp

shlq   $1,%r9
movq   %r9,%rax
mulq   16(%rdi)
addq   %rax,%rbx
adcq   %rdx,%rbp

movq   %r8,%rax
mulq   32(%rdi)
movq   %rax,%r8
movq   %rdx,%rcx

movq   %r9,%rax
mulq   24(%rdi)
addq   %rax,%r8
adcq   %rdx,%rcx

movq   16(%rdi),%rax
mulq   %rax
addq   %rax,%r8
adcq   %rdx,%rcx

shld   $13,%r8,%rcx

imul   $19,24(%rdi),%rax
mulq   24(%rdi)
addq   %rax,%r12
adcq   %rdx,%r13

imul   $19,32(%rdi),%rax
movq   %rax,56(%rsp)
mulq   32(%rdi)
addq   %rax,%rbx
adcq   %rdx,%rbp

shld   $13,%rbx,%rbp

imul   $19,32(%rdi),%rax
mulq   %r9
addq   %rax,%r10
adcq   %rdx,%r11

imul   $38,24(%rdi),%rax
mulq   16(%rdi)
addq   %rax,%r10
adcq   %rdx,%r11

shld   $13,%r10,%r11

imul   $38,32(%rdi),%rax
mulq   24(%rdi)
addq   %rax,%r14
adcq   %rdx,%r15

shld   $13,%r14,%r15

movq   56(%rsp),%rax
shlq   $1,%rax
mulq   16(%rdi)
addq   %rax,%r12
adcq   %rdx,%r13

shld   $13,%r12,%r13

imul   $19,%rcx,%rcx

movq   mask51(%rip),%rdx

andq   %rdx,%r10
andq   %rdx,%r12
andq   %rdx,%r14
andq   %rdx,%rbx
andq   %rdx,%r8

addq   %rcx,%r10
addq   %r11,%r12
addq   %r13,%r14
addq   %r15,%rbx
addq   %rbp,%r8

movq   %r10,%rax
shrq   $51,%rax
addq   %r12,%rax
andq   %rdx,%r10

movq   %rax,%r9
shrq   $51,%rax
addq   %r14,%rax
andq   %rdx,%r9

movq   %rax,%r14
shrq   $51,%rax
addq   %rbx,%rax
andq   %rdx,%r14

movq   %r14,16(%rdi)

movq   %rax,%rbx
shrq   $51,%rax
addq   %r8,%rax
andq   %rdx,%rbx

movq   %rbx,24(%rdi)

movq   %rax,%r8
shrq   $51,%r8
imul   $19,%r8,%r8
addq   %r10,%r8
andq   %rdx,%rax

movq   %rax,32(%rdi)

cmpq   $0,%rsi

jne    .L

movq   %r8,0(%rdi)
movq   %r9,8(%rdi)

movq   0(%rsp),%r11
movq   8(%rsp),%r12
movq   16(%rsp),%r13
movq   24(%rsp),%r14
movq   32(%rsp),%r15
movq   40(%rsp),%rbx
movq   48(%rsp),%rbp

movq   %r11,%rsp

ret
