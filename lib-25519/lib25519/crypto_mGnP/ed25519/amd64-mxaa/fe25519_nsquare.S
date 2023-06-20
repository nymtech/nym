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
subq    $56,%rsp

movq 	%r11,0(%rsp)
movq 	%r12,8(%rsp)
movq 	%r13,16(%rsp)
movq 	%r14,24(%rsp)
movq 	%r15,32(%rsp)
movq 	%rbp,40(%rsp)
movq 	%rbx,48(%rsp)

movq 	 0(%rsi),%r8	
movq 	 8(%rsi),%r9	
movq 	16(%rsi),%r10	
movq 	24(%rsi),%r11

movq 	%r8,0(%rdi)
movq 	%r9,8(%rdi)
movq 	%r10,16(%rdi)
movq 	%r11,24(%rdi)

mov  	%rdx,%rbp

.L:

movq    0(%rdi),%rdx
    
mulx    8(%rdi),%r9,%r10
mulx    16(%rdi),%rcx,%r11
addq    %rcx,%r10

mulx    24(%rdi),%rcx,%r12
adcq    %rcx,%r11
adcq    $0,%r12

movq    8(%rdi),%rdx

mulx    16(%rdi),%rax,%rbx
mulx    24(%rdi),%rcx,%r13
addq    %rcx,%rbx
adcq    $0,%r13

addq    %rax,%r11
adcq    %rbx,%r12
adcq    $0,%r13

movq    16(%rdi),%rdx

mulx    24(%rdi),%rax,%r14

addq    %rax,%r13
adcq    $0,%r14

movq    $0,%r15
shld    $1,%r14,%r15
shld    $1,%r13,%r14
shld    $1,%r12,%r13
shld    $1,%r11,%r12
shld    $1,%r10,%r11
shld    $1,%r9,%r10
shlq    $1,%r9

movq    0(%rdi),%rdx
mulx    %rdx,%r8,%rax
addq    %rax,%r9

movq    8(%rdi),%rdx
mulx    %rdx,%rax,%rbx
adcq    %rax,%r10
adcq    %rbx,%r11

movq    16(%rdi),%rdx
mulx    %rdx,%rax,%rbx
adcq    %rax,%r12
adcq    %rbx,%r13

movq    24(%rdi),%rdx
mulx    %rdx,%rax,%rbx
adcq    %rax,%r14
adcq    %rbx,%r15

movq    $38,%rdx

mulx    %r12,%r12,%rbx
mulx    %r13,%r13,%rcx
addq    %rbx,%r13

mulx    %r14,%r14,%rbx
adcq    %rcx,%r14

mulx    %r15,%r15,%rcx
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

movq    %r8,0(%rdi)
movq    %r9,8(%rdi)
movq    %r10,16(%rdi)
movq    %r11,24(%rdi)

subq  	$1,%rbp
cmpq    $0,%rbp

jne     .L

movq 	 0(%rsp),%r11
movq 	 8(%rsp),%r12
movq 	16(%rsp),%r13
movq 	24(%rsp),%r14
movq 	32(%rsp),%r15
movq 	40(%rsp),%rbp
movq 	48(%rsp),%rbx

movq 	%r11,%rsp

ret
