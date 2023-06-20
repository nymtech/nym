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

xorq    %r13,%r13
movq    0(%rsi),%rdx
    
mulx    8(%rsi),%r9,%r10

mulx    16(%rsi),%rcx,%r11
adcx    %rcx,%r10
    
mulx    24(%rsi),%rcx,%r12
adcx    %rcx,%r11
adcx    %r13,%r12

movq    8(%rsi),%rdx
xorq    %r14,%r14
    
mulx    16(%rsi),%rcx,%rdx
adcx    %rcx,%r11
adox    %rdx,%r12
    
movq    8(%rsi),%rdx
mulx    24(%rsi),%rcx,%rdx
adcx    %rcx,%r12
adox    %rdx,%r13
adcx    %r14,%r13

xorq    %r15,%r15
movq    16(%rsi),%rdx
    
mulx    24(%rsi),%rcx,%r14
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
movq    0(%rsi),%rdx
mulx    %rdx,%r8,%rdx
adcx    %rdx,%r9

movq    8(%rsi),%rdx
mulx    %rdx,%rcx,%rdx
adcx    %rcx,%r10
adcx    %rdx,%r11

movq    16(%rsi),%rdx
mulx    %rdx,%rcx,%rdx
adcx    %rcx,%r12
adcx    %rdx,%r13

movq    24(%rsi),%rdx
mulx    %rdx,%rcx,%rdx
adcx    %rcx,%r14
adcx    %rdx,%r15

xorq    %rbp,%rbp
movq    $38,%rdx

mulx    %r12,%rax,%r12 
adcx    %rax,%r8
adox    %r12,%r9

mulx    %r13,%rcx,%r13
adcx    %rcx,%r9
adox    %r13,%r10

mulx    %r14,%rcx,%r14
adcx    %rcx,%r10
adox    %r14,%r11

mulx    %r15,%rcx,%r15
adcx    %rcx,%r11
adox    %rbp,%r15
adcx    %rbp,%r15

shld    $1,%r11,%r15
andq	mask63(%rip),%r11

imul    $19,%r15,%r15
addq    %r15,%r8
adcq    $0,%r9
adcq    $0,%r10
adcq    $0,%r11

movq    %r8,0(%rdi)
movq    %r9,8(%rdi)
movq    %r10,16(%rdi)
movq    %r11,24(%rdi)

movq    0(%rsp),%r11
movq    8(%rsp),%r12
movq    16(%rsp),%r13
movq    24(%rsp),%r14
movq    32(%rsp),%r15
movq    40(%rsp),%rbx
movq    48(%rsp),%rbp

movq    %r11,%rsp

ret
