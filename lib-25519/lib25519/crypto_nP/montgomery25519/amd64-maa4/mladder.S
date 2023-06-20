// linker define mladder
// linker use mask63

/* Assembly for Montgomery ladder. */

#define mask63 CRYPTO_SHARED_NAMESPACE(mask63)

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(mladder)
.globl CRYPTO_SHARED_NAMESPACE(mladder)
_CRYPTO_SHARED_NAMESPACE(mladder):
CRYPTO_SHARED_NAMESPACE(mladder):

movq 	%rsp,%r11
andq    $-32,%rsp
subq 	$376,%rsp

movq 	%r11,0(%rsp)
movq 	%r12,8(%rsp)
movq 	%r13,16(%rsp)
movq 	%r14,24(%rsp)
movq 	%r15,32(%rsp)
movq 	%rbx,40(%rsp)
movq 	%rbp,48(%rsp)
movq 	%rdi,56(%rsp)

// X1 ← XP,X3 ← XP
movq	0(%rsi),%r8
movq	%r8,72(%rsp)
movq	%r8,168(%rsp)
movq	8(%rsi),%r8
movq	%r8,80(%rsp)
movq	%r8,176(%rsp)
movq	16(%rsi),%r8
movq	%r8,88(%rsp)
movq	%r8,184(%rsp)
movq	24(%rsi),%r8
movq	%r8,96(%rsp)
movq	%r8,192(%rsp)   

// X2 ← 1
movq	$1,104(%rsp)
movq	$0,112(%rsp)
movq	$0,120(%rsp)
movq	$0,128(%rsp) 	 

// Z2 ← 0
movq	$0,136(%rsp)
movq	$0,144(%rsp)
movq	$0,152(%rsp)
movq	$0,160(%rsp)	 

// Z3 ← 1
movq	$1,200(%rsp)
movq	$0,208(%rsp)
movq	$0,216(%rsp)
movq	$0,224(%rsp)    

movq    $31,240(%rsp)
movb	$6,232(%rsp)
movb    $0,234(%rsp)
movq    %rdx,64(%rsp)

movq    %rdx,%rax

// Montgomery ladder loop

.L1:

addq    240(%rsp),%rax
movb    0(%rax),%r14b
movb    %r14b,236(%rsp)

.L2:

/* 
 * Montgomery ladder step
 *
 * Reduction ideas for addition and subtraction are taken from the 64-bit implementation 
 * "amd64-64" of the work "https://link.springer.com/article/10.1007/s13389-012-0027-1"
 *
 * T1 ← X2 + Z2
 * T2 ← X2 - Z2
 * T3 ← X3 + Z3
 * T4 ← X3 - Z3
 * Z3 ← T2 · T3
 * X3 ← T1 · T4
 *
 * bit ← n[i]
 * select ← bit ⊕ prevbit
 * prevbit ← bit
 * CSelect(T1,T3,select): if (select == 1) {T1 = T3}
 * CSelect(T2,T4,select): if (select == 1) {T2 = T4}
 *
 * T2 ← T2^2
 * T1 ← T1^2
 * T3 ← X3 + Z3
 * Z3 ← X3 - Z3
 * Z3 ← Z3^2
 * X3 ← T3^2
 * T3 ← T1 - T2
 * T4 ← ((A + 2)/4) · T3
 * T4 ← T4 + T2
 * X2 ← T1 · T2
 * Z2 ← T3 · T4
 * Z3 ← Z3 · X1
 *
 */

// X2
movq    104(%rsp),%r8  
movq    112(%rsp),%r9
movq    120(%rsp),%r10
movq    128(%rsp),%r11

// copy X2
movq    %r8,%rax	
movq    %r9,%rbx
movq    %r10,%rbp
movq    %r11,%rsi

// T1 ← X2 + Z2
addq    136(%rsp),%r8
adcq    144(%rsp),%r9
adcq    152(%rsp),%r10
adcq    160(%rsp),%r11

movq    $0,%rdi
movq    $38,%rcx
cmovae  %rdi,%rcx

addq    %rcx,%r8
adcq    %rdi,%r9
adcq    %rdi,%r10
adcq    %rdi,%r11

cmovc   %rcx,%rdi
addq    %rdi,%r8

movq    %r8,248(%rsp)
movq    %r9,256(%rsp)
movq    %r10,264(%rsp)
movq    %r11,272(%rsp)

// T2 ← X2 - Z2
subq    136(%rsp),%rax
sbbq    144(%rsp),%rbx
sbbq    152(%rsp),%rbp
sbbq    160(%rsp),%rsi

movq    $0,%rdi
movq    $38,%rcx
cmovae  %rdi,%rcx

subq    %rcx,%rax
sbbq    %rdi,%rbx
sbbq    %rdi,%rbp
sbbq    %rdi,%rsi

cmovc   %rcx,%rdi
subq    %rdi,%rax

movq    %rax,280(%rsp)
movq    %rbx,288(%rsp)
movq    %rbp,296(%rsp)
movq    %rsi,304(%rsp)

// X3
movq    168(%rsp),%r8
movq    176(%rsp),%r9
movq    184(%rsp),%r10
movq    192(%rsp),%r11

// copy X3 
movq    %r8,%rax
movq    %r9,%rbx
movq    %r10,%rbp
movq    %r11,%rsi

// T3 ← X3 + Z3
addq    200(%rsp),%r8
adcq    208(%rsp),%r9
adcq    216(%rsp),%r10
adcq    224(%rsp),%r11

movq    $0,%rdi
movq    $38,%rcx
cmovae  %rdi,%rcx

addq    %rcx,%r8
adcq    %rdi,%r9
adcq    %rdi,%r10
adcq    %rdi,%r11

cmovc   %rcx,%rdi
addq    %rdi,%r8

movq    %r8,312(%rsp)
movq    %r9,320(%rsp)
movq    %r10,328(%rsp)
movq    %r11,336(%rsp)

// T4 ← X3 - Z3
subq    200(%rsp),%rax
sbbq    208(%rsp),%rbx
sbbq    216(%rsp),%rbp
sbbq    224(%rsp),%rsi

movq    $0,%rdi
movq    $38,%rcx
cmovae  %rdi,%rcx

subq    %rcx,%rax
sbbq    %rdi,%rbx
sbbq    %rdi,%rbp
sbbq    %rdi,%rsi

cmovc   %rcx,%rdi
subq    %rdi,%rax

movq    %rax,344(%rsp)
movq    %rbx,352(%rsp)
movq    %rbp,360(%rsp)
movq    %rsi,368(%rsp)

// Z3 ← T2 · T3
movq    288(%rsp),%rax
mulq    336(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    296(%rsp),%rax
mulq    328(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    304(%rsp),%rax
mulq    320(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    296(%rsp),%rax
mulq    336(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    304(%rsp),%rax
mulq    328(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    304(%rsp),%rax
mulq    336(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    280(%rsp),%rax
mulq    336(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    288(%rsp),%rax
mulq    328(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    296(%rsp),%rax
mulq    320(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    304(%rsp),%rax
mulq    312(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    280(%rsp),%rax
mulq    312(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    280(%rsp),%rax
mulq    320(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    288(%rsp),%rax
mulq    312(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    280(%rsp),%rax
mulq    328(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    288(%rsp),%rax
mulq    320(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    296(%rsp),%rax
mulq    312(%rsp)
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

movq    %r8,200(%rsp)
movq    %r10,208(%rsp)
movq    %r12,216(%rsp)
movq    %r14,224(%rsp)

// X3 ← T1 · T4
movq    256(%rsp),%rax
mulq    368(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    264(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    272(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    264(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    272(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    272(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    248(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    256(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    264(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    272(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    248(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    248(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    256(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    248(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    256(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    264(%rsp),%rax
mulq    344(%rsp)
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

movq    %r8,168(%rsp)
movq    %r10,176(%rsp)
movq    %r12,184(%rsp)
movq    %r14,192(%rsp)

movb	232(%rsp),%cl
movb	236(%rsp),%bl
shrb    %cl,%bl
andb    $1,%bl
movb    %bl,%cl
xorb    234(%rsp),%bl
movb    %cl,234(%rsp)

cmpb    $1,%bl 

// CSelect(T1,T3,select)
movq    248(%rsp),%r8
movq    256(%rsp),%r9
movq    264(%rsp),%r10
movq    272(%rsp),%r11

movq    312(%rsp),%r12
movq    320(%rsp),%r13
movq    328(%rsp),%r14
movq    336(%rsp),%r15

cmove   %r12,%r8
cmove   %r13,%r9
cmove   %r14,%r10
cmove   %r15,%r11

movq    %r8,248(%rsp)
movq    %r9,256(%rsp)
movq    %r10,264(%rsp)
movq    %r11,272(%rsp)

// CSelect(T2,T4,select)
movq    280(%rsp),%rbx
movq    288(%rsp),%rbp
movq    296(%rsp),%rcx
movq    304(%rsp),%rsi

movq    344(%rsp),%r12
movq    352(%rsp),%r13
movq    360(%rsp),%r14
movq    368(%rsp),%r15

cmove   %r12,%rbx
cmove   %r13,%rbp
cmove   %r14,%rcx
cmove   %r15,%rsi

// T2 ← T2^2
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
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

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

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
imul    $19,%r15,%r15
andq    mask63(%rip),%r14

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,280(%rsp)
movq    %r10,288(%rsp)
movq    %r12,296(%rsp)
movq    %r14,304(%rsp)

// T1 ← T1^2
movq    272(%rsp),%rax
mulq    272(%rsp)
movq    %rax,%r12
xorq    %r13,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    256(%rsp),%rax
mulq    272(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    264(%rsp),%rax
mulq    264(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    264(%rsp),%rax
mulq    272(%rsp)
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

movq    248(%rsp),%rax
mulq    272(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    256(%rsp),%rax
mulq    264(%rsp)
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

movq    248(%rsp),%rax
mulq    248(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    248(%rsp),%rax
mulq    256(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    248(%rsp),%rax
mulq    264(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    256(%rsp),%rax
mulq    256(%rsp)
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

movq    %r8,248(%rsp)
movq    %r10,256(%rsp)
movq    %r12,264(%rsp)
movq    %r14,272(%rsp)

// T3 ← X3 + Z3
movq    168(%rsp),%r8
movq    176(%rsp),%r9
movq    184(%rsp),%r10
movq    192(%rsp),%r11

movq    %r8,%rbx
movq    %r9,%rbp
movq    %r10,%rcx
movq    %r11,%rsi

addq    200(%rsp),%r8
adcq    208(%rsp),%r9
adcq    216(%rsp),%r10
adcq    224(%rsp),%r11

movq    $0,%rax
movq    $38,%rdx
cmovae  %rax,%rdx

addq    %rdx,%r8
adcq    %rax,%r9
adcq    %rax,%r10
adcq    %rax,%r11

cmovc   %rdx,%rax
addq    %rax,%r8

movq    %r8,168(%rsp)
movq    %r9,176(%rsp)
movq    %r10,184(%rsp)
movq    %r11,192(%rsp)

// Z3 ← X3 - Z3
subq    200(%rsp),%rbx
sbbq    208(%rsp),%rbp
sbbq    216(%rsp),%rcx
sbbq    224(%rsp),%rsi

movq    $0,%rax
movq    $38,%rdx
cmovae  %rax,%rdx

subq    %rdx,%rbx
sbbq    %rax,%rbp
sbbq    %rax,%rcx
sbbq    %rax,%rsi

cmovc   %rdx,%rax
subq    %rax,%rbx

// Z3 ← Z3^2
movq    %rsi,%rax
mulq    %rsi
movq    %rax,%r12
xorq    %r13,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    %rbp,%rax
mulq    %rsi
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

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

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
imul    $19,%r15,%r15
andq    mask63(%rip),%r14

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,200(%rsp) 
movq    %r10,208(%rsp)
movq    %r12,216(%rsp)
movq    %r14,224(%rsp)

// X3 ← T3^2
movq    192(%rsp),%rax
mulq    192(%rsp)
movq    %rax,%r12
xorq    %r13,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    176(%rsp),%rax
mulq    192(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    184(%rsp),%rax
mulq    184(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    184(%rsp),%rax
mulq    192(%rsp)
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

movq    168(%rsp),%rax
mulq    192(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    176(%rsp),%rax
mulq    184(%rsp)
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

movq    168(%rsp),%rax
mulq    168(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    168(%rsp),%rax
mulq    176(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    168(%rsp),%rax
mulq    184(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    176(%rsp),%rax
mulq    176(%rsp)
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

// update X3
movq    %r8,168(%rsp) 
movq    %r10,176(%rsp)
movq    %r12,184(%rsp)
movq    %r14,192(%rsp)

// T3 ← T1 - T2
movq    248(%rsp),%rbx
movq    256(%rsp),%rbp
movq    264(%rsp),%rcx
movq    272(%rsp),%rsi

subq    280(%rsp),%rbx
sbbq    288(%rsp),%rbp
sbbq    296(%rsp),%rcx
sbbq    304(%rsp),%rsi

movq    $0,%rax
movq    $38,%rdx
cmovae  %rax,%rdx

subq    %rdx,%rbx
sbbq    %rax,%rbp
sbbq    %rax,%rcx
sbbq    %rax,%rsi

cmovc   %rdx,%rax
subq    %rax,%rbx

movq    %rbx,312(%rsp)
movq    %rbp,320(%rsp)
movq    %rcx,328(%rsp)
movq    %rsi,336(%rsp)

// T4 ← ((A + 2)/4) · T3
movq    $121666,%rax
mulq    %rbx
movq    %rax,%r8
movq    %rdx,%r9

movq    $0,%r10
movq    $121666,%rax
mulq    %rbp
addq    %rax,%r9
adcq    %rdx,%r10

movq    $0,%r11
movq    $121666,%rax
mulq    %rcx
addq    %rax,%r10
adcq    %rdx,%r11

movq    $0,%r12
movq    $121666,%rax
mulq    %rsi
addq    %rax,%r11
adcq    %rdx,%r12

shld    $1,%r11,%r12
andq    mask63(%rip),%r11

imul    $19,%r12,%r12
addq    %r12,%r8
adcq    $0,%r9
adcq    $0,%r10
adcq    $0,%r11

// T4 ← T4 + T2
addq    280(%rsp),%r8
adcq    288(%rsp),%r9
adcq    296(%rsp),%r10
adcq    304(%rsp),%r11

movq    $0,%rax
movq    $38,%rdx
cmovae  %rax,%rdx

addq    %rdx,%r8
adcq    %rax,%r9
adcq    %rax,%r10
adcq    %rax,%r11

cmovc   %rdx,%rax
addq    %rax,%r8

movq    %r8,344(%rsp) 
movq    %r9,352(%rsp)
movq    %r10,360(%rsp)
movq    %r11,368(%rsp)

// X2 ← T1 · T2
movq    256(%rsp),%rax
mulq    304(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    264(%rsp),%rax
mulq    296(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    272(%rsp),%rax
mulq    288(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    264(%rsp),%rax
mulq    304(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    272(%rsp),%rax
mulq    296(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    272(%rsp),%rax
mulq    304(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    248(%rsp),%rax
mulq    304(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    256(%rsp),%rax
mulq    296(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    264(%rsp),%rax
mulq    288(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    272(%rsp),%rax
mulq    280(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    248(%rsp),%rax
mulq    280(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    248(%rsp),%rax
mulq    288(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    256(%rsp),%rax
mulq    280(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    248(%rsp),%rax
mulq    296(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    256(%rsp),%rax
mulq    288(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    264(%rsp),%rax
mulq    280(%rsp)
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

// update X2
movq    %r8,104(%rsp) 
movq    %r10,112(%rsp)
movq    %r12,120(%rsp)
movq    %r14,128(%rsp)

// Z2 ← T3 · T4
movq    320(%rsp),%rax
mulq    368(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    328(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    336(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    328(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    336(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    336(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    312(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    320(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    328(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    336(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    312(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    312(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    320(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    312(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    320(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    328(%rsp),%rax
mulq    344(%rsp)
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

// update Z2
movq    %r8,136(%rsp) 
movq    %r10,144(%rsp)
movq    %r12,152(%rsp)
movq    %r14,160(%rsp)

// Z3 ← Z3 · X1
movq    80(%rsp),%rax
mulq    224(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    88(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    96(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    88(%rsp),%rax
mulq    224(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    96(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    96(%rsp),%rax
mulq    224(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    72(%rsp),%rax
mulq    224(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    80(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    88(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    96(%rsp),%rax
mulq    200(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    72(%rsp),%rax
mulq    200(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    72(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    80(%rsp),%rax
mulq    200(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    72(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    80(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    88(%rsp),%rax
mulq    200(%rsp)
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

// update Z3
movq    %r8,200(%rsp) 
movq    %r10,208(%rsp)
movq    %r12,216(%rsp)
movq    %r14,224(%rsp)

movb    232(%rsp),%cl
subb    $1,%cl
movb    %cl,232(%rsp)
cmpb	$0,%cl
jge     .L2

movb    $7,232(%rsp)
movq    64(%rsp),%rax
movq    240(%rsp),%r15
subq    $1,%r15
movq    %r15,240(%rsp)
cmpq	$0,%r15
jge     .L1

movq    56(%rsp),%rdi

movq    104(%rsp),%r8 
movq    112(%rsp),%r9
movq    120(%rsp),%r10
movq    128(%rsp),%r11

// store final value of X2
movq    %r8,0(%rdi) 
movq    %r9,8(%rdi)
movq    %r10,16(%rdi)
movq    %r11,24(%rdi)

movq    136(%rsp),%r8 
movq    144(%rsp),%r9
movq    152(%rsp),%r10
movq    160(%rsp),%r11

// store final value of Z2
movq    %r8,32(%rdi) 
movq    %r9,40(%rdi)
movq    %r10,48(%rdi)
movq    %r11,56(%rdi)

movq 	 0(%rsp),%r11
movq 	 8(%rsp),%r12
movq 	16(%rsp),%r13
movq 	24(%rsp),%r14
movq 	32(%rsp),%r15
movq 	40(%rsp),%rbx
movq 	48(%rsp),%rbp

movq 	%r11,%rsp

ret
