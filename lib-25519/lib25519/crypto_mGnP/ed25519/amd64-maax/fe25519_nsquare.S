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
movq    16(%rsi),%rax
movq    24(%rsi),%rsi

movq    %rdx,%rdi

.L:

xorq    %r13,%r13
movq    %rbx,%rdx
    
mulx    %rbp,%r9,%r10

mulx    %rax,%rcx,%r11
adcx    %rcx,%r10
    
mulx    %rsi,%rcx,%r12
adcx    %rcx,%r11
adcx    %r13,%r12

movq    %rbp,%rdx
xorq    %r14,%r14
    
mulx    %rax,%rcx,%rdx
adcx    %rcx,%r11
adox    %rdx,%r12
    
movq    %rbp,%rdx
mulx    %rsi,%rcx,%rdx
adcx    %rcx,%r12
adox    %rdx,%r13
adcx    %r14,%r13

xorq    %r15,%r15
movq    %rax,%rdx
    
mulx    %rsi,%rcx,%r14
adcx    %rcx,%r13
adcx    %r15,%r14

shld    $1,%r14,%r15
shld    $1,%r13,%r14
shld    $1,%r12,%r13
shld    $1,%r11,%r12
shld    $1,%r10,%r11
shld    $1,%r9,%r10
shlq    $1,%r9
     
xorq    %rdx,%rdx
movq    %rbx,%rdx
mulx    %rdx,%r8,%rdx
adcx    %rdx,%r9

movq    %rbp,%rdx
mulx    %rdx,%rcx,%rdx
adcx    %rcx,%r10
adcx    %rdx,%r11

movq    %rax,%rdx
mulx    %rdx,%rcx,%rdx
adcx    %rcx,%r12
adcx    %rdx,%r13

movq    %rsi,%rdx
mulx    %rdx,%rcx,%rdx
adcx    %rcx,%r14
adcx    %rdx,%r15

xorq    %rbx,%rbx
movq    $38,%rdx    		

mulx    %r12,%r12,%rbp
adcx    %r8,%r12
adox    %r9,%rbp

mulx    %r13,%rcx,%rax
adcx    %rcx,%rbp
adox    %r10,%rax

mulx    %r14,%rcx,%rsi
adcx    %rcx,%rax
adox    %r11,%rsi

mulx    %r15,%rcx,%r15
adcx    %rcx,%rsi
adox    %rbx,%r15
adcx    %rbx,%r15

shld    $1,%rsi,%r15
andq	mask63(%rip),%rsi

imul   $19,%r15,%rbx
addq   %r12,%rbx
adcq   $0,%rbp
adcq   $0,%rax
adcq   $0,%rsi

subq    $1,%rdi
cmpq    $0,%rdi

jne     .L

movq    56(%rsp),%rdi

movq    %rbx,0(%rdi)
movq    %rbp,8(%rdi)
movq    %rax,16(%rdi)
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
