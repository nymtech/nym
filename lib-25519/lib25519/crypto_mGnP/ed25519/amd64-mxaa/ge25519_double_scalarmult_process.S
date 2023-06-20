// linker define ge25519_double_scalarmult_process
// linker use mask63

/* Assembly for double base scalar multiplication.
 * 
 * This assembly has been developed after studying the 
 * amd64-64-24k implementation of the work "High speed 
 * high security signatures" by Bernstein et al.
*/

#define mask63 CRYPTO_SHARED_NAMESPACE(mask63)

        .p2align 5
        .globl _CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process)
        .globl CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process)

_CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process):
CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process):

	movq	%rsp,%r11
	andq	$-32,%rsp
	subq  	$464,%rsp 

	movq	%r11,0(%rsp)
	movq	%r12,8(%rsp)
	movq	%r13,16(%rsp)
	movq	%r14,24(%rsp)
	movq	%r15,32(%rsp)
	movq	%rbx,40(%rsp)
	movq	%rbp,48(%rsp)

	// setneutral	
	movq	$0,%rax
	movq	$1,%rbx	

	movq	%rax,0(%rdi)
	movq	%rax,8(%rdi)
	movq	%rax,16(%rdi)
	movq	%rax,24(%rdi)
	
	movq	%rbx,32(%rdi)
	movq	%rax,40(%rdi)
	movq	%rax,48(%rdi)
	movq	%rax,56(%rdi)
	
	movq	%rbx,64(%rdi)
	movq	%rax,72(%rdi)
	movq	%rax,80(%rdi)
	movq	%rax,88(%rdi)	

	movq	%rax,96(%rdi)
	movq	%rax,104(%rdi)
	movq	%rax,112(%rdi)
	movq	%rax,120(%rdi)
	
	movq	$255,%rax
	addq	$255,%rsi
	addq	$255,%rdx
	
	movq	%rdi,56(%rsp)	
	movq	%rcx,64(%rsp)
	movq	%r8,72(%rsp)

.L1:	
	movb	0(%rsi),%r14b
	movb	0(%rdx),%r15b
	
	cmpb	$0,%r14b
	jg	.L2
	
	cmpb	$0,%r15b
	jg	.L2
	
	decq	%rsi
	decq	%rdx
	
	decq	%rax
	cmpq	$0,%rax
	
	jge	.L1
	
	cmpq	$0,%rax
	jl	.L10	
	
.L2:	
	movq	%rsi,80(%rsp)
	movq	%rdx,88(%rsp)
	movq	%rax,96(%rsp)	
	
.L3:	
	/* dbl p1p1 */

	// square
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
	
	movq    %r8,112(%rsp)
	movq    %r9,120(%rsp)
	movq    %r10,128(%rsp)
	movq    %r11,136(%rsp)
	
	// square
	movq    32(%rdi),%rdx
	    
	mulx    40(%rdi),%r9,%r10
	mulx    48(%rdi),%rcx,%r11
	addq    %rcx,%r10

	mulx    56(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    40(%rdi),%rdx

	mulx    48(%rdi),%rax,%rbx
	mulx    56(%rdi),%rcx,%r13
	addq    %rcx,%rbx
	adcq    $0,%r13

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    $0,%r13

	movq    48(%rdi),%rdx

	mulx    56(%rdi),%rax,%r14

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

	movq    32(%rdi),%rdx
	mulx    %rdx,%r8,%rax
	addq    %rax,%r9

	movq    40(%rdi),%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r10
	adcq    %rbx,%r11

	movq    48(%rdi),%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r12
	adcq    %rbx,%r13

	movq    56(%rdi),%rdx
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
	
	movq    %r8,144(%rsp)
	movq    %r9,152(%rsp)
	movq    %r10,160(%rsp)
	movq    %r11,168(%rsp)
	
	// square
	movq    64(%rdi),%rdx
	    
	mulx    72(%rdi),%r9,%r10
	mulx    80(%rdi),%rcx,%r11
	addq    %rcx,%r10

	mulx    88(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    72(%rdi),%rdx

	mulx    80(%rdi),%rax,%rbx
	mulx    88(%rdi),%rcx,%r13
	addq    %rcx,%rbx
	adcq    $0,%r13

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    $0,%r13

	movq    80(%rdi),%rdx

	mulx    88(%rdi),%rax,%r14

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

	movq    64(%rdi),%rdx
	mulx    %rdx,%r8,%rax
	addq    %rax,%r9

	movq    72(%rdi),%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r10
	adcq    %rbx,%r11

	movq    80(%rdi),%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r12
	adcq    %rbx,%r13

	movq    88(%rdi),%rdx
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

	// double
	addq    %r8,%r8
	adcq 	%r9,%r9
	adcq	%r10,%r10
	adcq 	%r11,%r11

	movq  	$0,%rdx
	movq  	$38,%rcx
	cmovae	%rdx,%rcx

	addq  	%rcx,%r8
	adcq 	%rdx,%r9
	adcq 	%rdx,%r10
	adcq 	%rdx,%r11

	cmovc 	%rcx,%rdx
	addq  	%rdx,%r8
	
	movq    %r8,176(%rsp)
	movq    %r9,184(%rsp)
	movq    %r10,192(%rsp)
	movq    %r11,200(%rsp)

	// neg
	movq    $0,%r8
	movq    $0,%r9
	movq    $0,%r10
	movq    $0,%r11

	subq    112(%rsp),%r8
	sbbq    120(%rsp),%r9
	sbbq    128(%rsp),%r10
	sbbq    136(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,112(%rsp)
	movq    %r9,120(%rsp)
	movq    %r10,128(%rsp)
	movq    %r11,136(%rsp)

	// copy
	movq    %r8,%r12
	movq    %r9,%r13
	movq    %r10,%r14
	movq    %r11,%r15
	
	// sub
	subq    144(%rsp),%r8
	sbbq    152(%rsp),%r9
	sbbq    160(%rsp),%r10
	sbbq    168(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,304(%rsp)
	movq    %r9,312(%rsp)
	movq    %r10,320(%rsp)
	movq    %r11,328(%rsp)	

	// add
	addq    144(%rsp),%r12
	adcq    152(%rsp),%r13
	adcq    160(%rsp),%r14
	adcq    168(%rsp),%r15

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r12
	adcq    %rdx,%r13
	adcq    %rdx,%r14
	adcq    %rdx,%r15

	cmovc   %rax,%rdx
	subq    %rdx,%r12

	movq    %r12,272(%rsp)
	movq    %r13,280(%rsp)
	movq    %r14,288(%rsp)
	movq    %r15,296(%rsp)

	// sub
	subq    176(%rsp),%r12
	sbbq    184(%rsp),%r13
	sbbq    192(%rsp),%r14
	sbbq    200(%rsp),%r15

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	subq    %rax,%r12
	sbbq    %rdx,%r13
	sbbq    %rdx,%r14
	sbbq    %rdx,%r15

	cmovc   %rax,%rdx
	subq    %rdx,%r12

	movq    %r12,336(%rsp)
	movq    %r13,344(%rsp)
	movq    %r14,352(%rsp)
	movq    %r15,360(%rsp)

	// add
	movq    0(%rdi),%rbx
	movq    8(%rdi),%rbp
	movq    16(%rdi),%rcx
	movq    24(%rdi),%rsi

	addq    32(%rdi),%rbx
	adcq    40(%rdi),%rbp
	adcq    48(%rdi),%rcx
	adcq    56(%rdi),%rsi

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax
	
	addq    %rax,%rbx
	adcq    %rdx,%rbp
	adcq    %rdx,%rcx
	adcq    %rdx,%rsi
	
	cmovc   %rax,%rdx
	addq    %rdx,%rbx

	// square
	movq    %rbx,%rdx
	    
	mulx    %rbp,%r9,%r10
	mulx    %rcx,%r8,%r11
	addq    %r8,%r10

	mulx    %rsi,%r8,%r12
	adcq    %r8,%r11
	adcq    $0,%r12

	movq    %rbp,%rdx

	mulx    %rcx,%rax,%r8
	mulx    %rsi,%rdx,%r13
	addq    %rdx,%r8
	adcq    $0,%r13

	addq    %rax,%r11
	adcq    %r8,%r12
	adcq    $0,%r13

	movq    %rcx,%rdx

	mulx    %rsi,%rax,%r14

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

	movq    %rbx,%rdx
	mulx    %rdx,%r8,%rax
	addq    %rax,%r9

	movq    %rbp,%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r10
	adcq    %rbx,%r11

	movq    %rcx,%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r12
	adcq    %rbx,%r13

	movq    %rsi,%rdx
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

	// add
	addq    112(%rsp),%r8
	adcq    120(%rsp),%r9
	adcq    128(%rsp),%r10
	adcq    136(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r8
	adcq    %rdx,%r9
	adcq    %rdx,%r10
	adcq    %rdx,%r11

	cmovc   %rax,%rdx
	addq    %rdx,%r8

	// sub
	subq    144(%rsp),%r8
	sbbq    152(%rsp),%r9
	sbbq    160(%rsp),%r10
	sbbq    168(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,240(%rsp)
	movq    %r9,248(%rsp)
	movq    %r10,256(%rsp)
	movq    %r11,264(%rsp)
	
	movq	80(%rsp),%rsi
	movb	0(%rsi),%r14b
	movb	%r14b,104(%rsp)
	decq	%rsi
	movq	%rsi,80(%rsp)
	movq	64(%rsp),%rdi
	
	cmpb	$0,%r14b
	jg	.L4
	jl	.L5
	je	.L6
	
.L4:	
	/* p1p1 to p3 */

	// mul
	movq    240(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,112(%rsp)
	movq    %r9,120(%rsp)
	movq    %r10,128(%rsp)
	movq    %r11,136(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,144(%rsp)
	movq    %r9,152(%rsp)
	movq    %r10,160(%rsp)
	movq    %r11,168(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,176(%rsp)
	movq    %r9,184(%rsp)
	movq    %r10,192(%rsp)
	movq    %r11,200(%rsp)

	// mul
	movq    240(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,208(%rsp)
	movq    %r9,216(%rsp)
	movq    %r10,224(%rsp)
	movq    %r11,232(%rsp)

	movb	104(%rsp),%r14b
	shrb	$1,%r14b
	movzbq	%r14b,%r14
	imul	$128,%r14,%r14
	addq	%r14,%rdi
	
	/* pnielsadd p1p1 */
	
	movq	144(%rsp),%r8
	movq	152(%rsp),%r9
	movq	160(%rsp),%r10
	movq	168(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	112(%rsp),%r8
	sbbq 	120(%rsp),%r9
	sbbq 	128(%rsp),%r10
	sbbq 	136(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,368(%rsp)
	movq   %r9,376(%rsp)
	movq   %r10,384(%rsp)
	movq   %r11,392(%rsp)
	
	// add
	addq 	112(%rsp),%r12
	adcq 	120(%rsp),%r13
	adcq 	128(%rsp),%r14
	adcq 	136(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,400(%rsp)
	movq   %r13,408(%rsp)
	movq   %r14,416(%rsp)
	movq   %r15,424(%rsp)
	
	// mul
	movq    368(%rsp),%rdx

	mulx    0(%rdi),%r8,%r9
	mulx    8(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    16(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    24(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    376(%rsp),%rdx    

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    392(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,368(%rsp)
	movq    %r9,376(%rsp)
	movq    %r10,384(%rsp)
	movq    %r11,392(%rsp)

	// mul
	movq    400(%rsp),%rdx

	mulx    32(%rdi),%r8,%r9
	mulx    40(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    48(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    56(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    408(%rsp),%rdx    

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    424(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	// add
	movq 	%r8,%r12
	movq 	%r9,%r13
	movq 	%r10,%r14
	movq 	%r11,%r15

	addq 	368(%rsp),%r8
	adcq 	376(%rsp),%r9
	adcq 	384(%rsp),%r10
	adcq 	392(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,304(%rsp)
	movq   %r9,312(%rsp)
	movq   %r10,320(%rsp)
	movq   %r11,328(%rsp)

	// sub
	subq 	368(%rsp),%r12
	sbbq 	376(%rsp),%r13
	sbbq 	384(%rsp),%r14
	sbbq 	392(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,240(%rsp)
	movq   %r13,248(%rsp)
	movq   %r14,256(%rsp)
	movq   %r15,264(%rsp)

	// mul	
	movq    208(%rsp),%rdx

	mulx    96(%rdi),%r8,%r9
	mulx    104(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    112(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    120(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    216(%rsp),%rdx    

	mulx    96(%rdi),%rax,%rbx
	mulx    104(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    112(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    120(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    224(%rsp),%rdx

	mulx    96(%rdi),%rax,%rbx
	mulx    104(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    112(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    120(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    232(%rsp),%rdx

	mulx    96(%rdi),%rax,%rbx
	mulx    104(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    112(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    120(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,368(%rsp)
	movq    %r9,376(%rsp)
	movq    %r10,384(%rsp)
	movq    %r11,392(%rsp)	

	// mul	
	movq    176(%rsp),%rdx

	mulx    64(%rdi),%r8,%r9
	mulx    72(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    80(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    88(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    184(%rsp),%rdx    

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    192(%rsp),%rdx

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    200(%rsp),%rdx

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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
	
	// double
	addq 	%r8,%r8
	adcq 	%r9,%r9
	adcq 	%r10,%r10
	adcq 	%r11,%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8	
		
	// add
	movq 	%r8,%r12
	movq 	%r9,%r13
	movq 	%r10,%r14
	movq 	%r11,%r15

	addq 	368(%rsp),%r8
	adcq 	376(%rsp),%r9
	adcq 	384(%rsp),%r10
	adcq 	392(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,272(%rsp)
	movq   %r9,280(%rsp)
	movq   %r10,288(%rsp)
	movq   %r11,296(%rsp)

	// sub
	subq 	368(%rsp),%r12
	sbbq 	376(%rsp),%r13
	sbbq 	384(%rsp),%r14
	sbbq 	392(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,336(%rsp)
	movq   %r13,344(%rsp)
	movq   %r14,352(%rsp)
	movq   %r15,360(%rsp)
	
	jmp	.L6

.L5:
	/* p1p1 to p3 */

	// mul
	movq    240(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,112(%rsp)
	movq    %r9,120(%rsp)
	movq    %r10,128(%rsp)
	movq    %r11,136(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,144(%rsp)
	movq    %r9,152(%rsp)
	movq    %r10,160(%rsp)
	movq    %r11,168(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,176(%rsp)
	movq    %r9,184(%rsp)
	movq    %r10,192(%rsp)
	movq    %r11,200(%rsp)

	// mul
	movq    240(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,208(%rsp)
	movq    %r9,216(%rsp)
	movq    %r10,224(%rsp)
	movq    %r11,232(%rsp)
	
	movb	104(%rsp),%r14b
	movb	$0,%r15b
	subb	%r14b,%r15b
	shrb	$1,%r15b
	movzbq	%r15b,%r15
	imul	$128,%r15,%r15
	addq	%r15,%rdi
	
	// neg
	movq    $0,%r8
	movq    $0,%r9
	movq    $0,%r10
	movq    $0,%r11

	subq    96(%rdi),%r8
	sbbq    104(%rdi),%r9
	sbbq    112(%rdi),%r10
	sbbq    120(%rdi),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,432(%rsp)
	movq    %r9,440(%rsp)
	movq    %r10,448(%rsp)
	movq    %r11,456(%rsp)

	/* pnielsadd p1p1 */
	
	movq	144(%rsp),%r8
	movq	152(%rsp),%r9
	movq	160(%rsp),%r10
	movq	168(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	112(%rsp),%r8
	sbbq 	120(%rsp),%r9
	sbbq 	128(%rsp),%r10
	sbbq 	136(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,368(%rsp)
	movq   %r9,376(%rsp)
	movq   %r10,384(%rsp)
	movq   %r11,392(%rsp)
	
	// add
	addq 	112(%rsp),%r12
	adcq 	120(%rsp),%r13
	adcq 	128(%rsp),%r14
	adcq 	136(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,400(%rsp)
	movq   %r13,408(%rsp)
	movq   %r14,416(%rsp)
	movq   %r15,424(%rsp)
	
	// mul
	movq    368(%rsp),%rdx

	mulx    32(%rdi),%r8,%r9
	mulx    40(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    48(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    56(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    376(%rsp),%rdx    

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    392(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,368(%rsp)
	movq    %r9,376(%rsp)
	movq    %r10,384(%rsp)
	movq    %r11,392(%rsp)

	// mul
	movq    400(%rsp),%rdx

	mulx    0(%rdi),%r8,%r9
	mulx    8(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    16(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    24(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    408(%rsp),%rdx    

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    424(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	// add
	movq 	%r8,%r12
	movq 	%r9,%r13
	movq 	%r10,%r14
	movq 	%r11,%r15

	addq 	368(%rsp),%r8
	adcq 	376(%rsp),%r9
	adcq 	384(%rsp),%r10
	adcq 	392(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,304(%rsp)
	movq   %r9,312(%rsp)
	movq   %r10,320(%rsp)
	movq   %r11,328(%rsp)

	// sub
	subq 	368(%rsp),%r12
	sbbq 	376(%rsp),%r13
	sbbq 	384(%rsp),%r14
	sbbq 	392(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,240(%rsp)
	movq   %r13,248(%rsp)
	movq   %r14,256(%rsp)
	movq   %r15,264(%rsp)

	// mul	
	movq    208(%rsp),%rdx

	mulx    432(%rsp),%r8,%r9
	mulx    440(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    448(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    456(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    216(%rsp),%rdx    

	mulx    432(%rsp),%rax,%rbx
	mulx    440(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    448(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    456(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    224(%rsp),%rdx

	mulx    432(%rsp),%rax,%rbx
	mulx    440(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    448(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    456(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    232(%rsp),%rdx

	mulx    432(%rsp),%rax,%rbx
	mulx    440(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    448(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    456(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,368(%rsp)
	movq    %r9,376(%rsp)
	movq    %r10,384(%rsp)
	movq    %r11,392(%rsp)	

	// mul	
	movq    176(%rsp),%rdx

	mulx    64(%rdi),%r8,%r9
	mulx    72(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    80(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    88(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    184(%rsp),%rdx    

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    192(%rsp),%rdx

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    200(%rsp),%rdx

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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
	
	// double
	addq 	%r8,%r8
	adcq 	%r9,%r9
	adcq 	%r10,%r10
	adcq 	%r11,%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8	
		
	// add
	movq 	%r8,%r12
	movq 	%r9,%r13
	movq 	%r10,%r14
	movq 	%r11,%r15

	addq 	368(%rsp),%r8
	adcq 	376(%rsp),%r9
	adcq 	384(%rsp),%r10
	adcq 	392(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,272(%rsp)
	movq   %r9,280(%rsp)
	movq   %r10,288(%rsp)
	movq   %r11,296(%rsp)

	// sub
	subq 	368(%rsp),%r12
	sbbq 	376(%rsp),%r13
	sbbq 	384(%rsp),%r14
	sbbq 	392(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,336(%rsp)
	movq   %r13,344(%rsp)
	movq   %r14,352(%rsp)
	movq   %r15,360(%rsp)
	
.L6:	
	movq	88(%rsp),%rsi
	movb	0(%rsi),%r14b
	movb	%r14b,104(%rsp)	
	decq	%rsi
	movq	%rsi,88(%rsp)	
	movq	72(%rsp),%rdi

	cmpb	$0,%r14b
	jg	.L7
	jl	.L8
	je	.L9
	
.L7:	
	/* p1p1 to p3 */

	// mul
	movq    240(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,112(%rsp)
	movq    %r9,120(%rsp)
	movq    %r10,128(%rsp)
	movq    %r11,136(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,144(%rsp)
	movq    %r9,152(%rsp)
	movq    %r10,160(%rsp)
	movq    %r11,168(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,176(%rsp)
	movq    %r9,184(%rsp)
	movq    %r10,192(%rsp)
	movq    %r11,200(%rsp)

	// mul
	movq    240(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,208(%rsp)
	movq    %r9,216(%rsp)
	movq    %r10,224(%rsp)
	movq    %r11,232(%rsp)

	movb	104(%rsp),%r14b
	shrb	$1,%r14b
	movzbq	%r14b,%r14
	imul	$96,%r14,%r14	
	addq	%r14,%rdi
	
	/* nielsadd p1p1 */
		
	movq	144(%rsp),%r8
	movq	152(%rsp),%r9
	movq	160(%rsp),%r10
	movq	168(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	112(%rsp),%r8
	sbbq 	120(%rsp),%r9
	sbbq 	128(%rsp),%r10
	sbbq 	136(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,368(%rsp)
	movq   %r9,376(%rsp)
	movq   %r10,384(%rsp)
	movq   %r11,392(%rsp)
	
	// add
	addq 	112(%rsp),%r12
	adcq 	120(%rsp),%r13
	adcq 	128(%rsp),%r14
	adcq 	136(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,400(%rsp)
	movq   %r13,408(%rsp)
	movq   %r14,416(%rsp)
	movq   %r15,424(%rsp)
	
	// mul
	movq    368(%rsp),%rdx

	mulx    0(%rdi),%r8,%r9
	mulx    8(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    16(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    24(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    376(%rsp),%rdx    

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    392(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,368(%rsp)
	movq    %r9,376(%rsp)
	movq    %r10,384(%rsp)
	movq    %r11,392(%rsp)

	// mul
	movq    400(%rsp),%rdx

	mulx    32(%rdi),%r8,%r9
	mulx    40(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    48(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    56(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    408(%rsp),%rdx    

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    424(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	// add
	movq 	%r8,%r12
	movq 	%r9,%r13
	movq 	%r10,%r14
	movq 	%r11,%r15

	addq 	368(%rsp),%r8
	adcq 	376(%rsp),%r9
	adcq 	384(%rsp),%r10
	adcq 	392(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,304(%rsp)
	movq   %r9,312(%rsp)
	movq   %r10,320(%rsp)
	movq   %r11,328(%rsp)

	// sub
	subq 	368(%rsp),%r12
	sbbq 	376(%rsp),%r13
	sbbq 	384(%rsp),%r14
	sbbq 	392(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,240(%rsp)
	movq   %r13,248(%rsp)
	movq   %r14,256(%rsp)
	movq   %r15,264(%rsp)

	// mul	
	movq    208(%rsp),%rdx

	mulx    64(%rdi),%r8,%r9
	mulx    72(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    80(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    88(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    216(%rsp),%rdx    

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    224(%rsp),%rdx

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    232(%rsp),%rdx

	mulx    64(%rdi),%rax,%rbx
	mulx    72(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    80(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    88(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	// double
	movq	176(%rsp),%r12
	movq	184(%rsp),%r13
	movq	192(%rsp),%r14
	movq	200(%rsp),%r15
	
	addq 	%r12,%r12
	adcq 	%r13,%r13
	adcq 	%r14,%r14
	adcq 	%r15,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq	%rdx,%r14
	adcq	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12	
		
	// sub
	movq 	%r12,%rbx
	movq 	%r13,%rcx
	movq 	%r14,%rbp
	movq 	%r15,%rsi

	subq 	%r8,%r12
	sbbq 	%r9,%r13
	sbbq 	%r10,%r14
	sbbq 	%r11,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	sbbq	%rdx,%r12
	
	movq   %r12,336(%rsp)
	movq   %r13,344(%rsp)
	movq   %r14,352(%rsp)
	movq   %r15,360(%rsp)

	// add
	addq 	%rbx,%r8
	adcq 	%rcx,%r9
	adcq 	%rbp,%r10
	adcq 	%rsi,%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	adcq	%rdx,%r8

	movq   %r8,272(%rsp)
	movq   %r9,280(%rsp)
	movq   %r10,288(%rsp)
	movq   %r11,296(%rsp)

	jmp	.L9

.L8:	
	/* p1p1 to p3 */

	// mul
	movq    240(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,112(%rsp)
	movq    %r9,120(%rsp)
	movq    %r10,128(%rsp)
	movq    %r11,136(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,144(%rsp)
	movq    %r9,152(%rsp)
	movq    %r10,160(%rsp)
	movq    %r11,168(%rsp)

	// mul
	movq    272(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,176(%rsp)
	movq    %r9,184(%rsp)
	movq    %r10,192(%rsp)
	movq    %r11,200(%rsp)

	// mul
	movq    240(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,208(%rsp)
	movq    %r9,216(%rsp)
	movq    %r10,224(%rsp)
	movq    %r11,232(%rsp)

	movb	104(%rsp),%r14b
	movb	$0,%r15b
	subb	%r14b,%r15b
	shrb	$1,%r15b
	movzbq	%r15b,%r15
	imul	$96,%r15,%r15	
	addq	%r15,%rdi
	
	// neg
	movq    $0,%r8
	movq    $0,%r9
	movq    $0,%r10
	movq    $0,%r11

	subq    64(%rdi),%r8
	sbbq    72(%rdi),%r9
	sbbq    80(%rdi),%r10
	sbbq    88(%rdi),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,432(%rsp)
	movq    %r9,440(%rsp)
	movq    %r10,448(%rsp)
	movq    %r11,456(%rsp)

	/* nielsadd p1p1 */
	
	movq	144(%rsp),%r8
	movq	152(%rsp),%r9
	movq	160(%rsp),%r10
	movq	168(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	112(%rsp),%r8
	sbbq 	120(%rsp),%r9
	sbbq 	128(%rsp),%r10
	sbbq 	136(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,368(%rsp)
	movq   %r9,376(%rsp)
	movq   %r10,384(%rsp)
	movq   %r11,392(%rsp)
	
	// add
	addq 	112(%rsp),%r12
	adcq 	120(%rsp),%r13
	adcq 	128(%rsp),%r14
	adcq 	136(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,400(%rsp)
	movq   %r13,408(%rsp)
	movq   %r14,416(%rsp)
	movq   %r15,424(%rsp)
	
	// mul
	movq    368(%rsp),%rdx

	mulx    32(%rdi),%r8,%r9
	mulx    40(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    48(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    56(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    376(%rsp),%rdx    

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    392(%rsp),%rdx

	mulx    32(%rdi),%rax,%rbx
	mulx    40(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    48(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    56(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,368(%rsp)
	movq    %r9,376(%rsp)
	movq    %r10,384(%rsp)
	movq    %r11,392(%rsp)

	// mul
	movq    400(%rsp),%rdx

	mulx    0(%rdi),%r8,%r9
	mulx    8(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    16(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    24(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    408(%rsp),%rdx    

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    424(%rsp),%rdx

	mulx    0(%rdi),%rax,%rbx
	mulx    8(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    16(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    24(%rdi),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	// add
	movq 	%r8,%r12
	movq 	%r9,%r13
	movq 	%r10,%r14
	movq 	%r11,%r15

	addq 	368(%rsp),%r8
	adcq 	376(%rsp),%r9
	adcq 	384(%rsp),%r10
	adcq 	392(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,304(%rsp)
	movq   %r9,312(%rsp)
	movq   %r10,320(%rsp)
	movq   %r11,328(%rsp)

	// sub
	subq 	368(%rsp),%r12
	sbbq 	376(%rsp),%r13
	sbbq 	384(%rsp),%r14
	sbbq 	392(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,240(%rsp)
	movq   %r13,248(%rsp)
	movq   %r14,256(%rsp)
	movq   %r15,264(%rsp)

	// mul	
	movq    432(%rsp),%rdx

	mulx    208(%rsp),%r8,%r9
	mulx    216(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    224(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    232(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    440(%rsp),%rdx    

	mulx    208(%rsp),%rax,%rbx
	mulx    216(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    224(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    232(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    448(%rsp),%rdx

	mulx    208(%rsp),%rax,%rbx
	mulx    216(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    224(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    232(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    456(%rsp),%rdx

	mulx    208(%rsp),%rax,%rbx
	mulx    216(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    224(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    232(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	// double
	movq	176(%rsp),%r12
	movq	184(%rsp),%r13
	movq	192(%rsp),%r14
	movq	200(%rsp),%r15
	
	addq 	%r12,%r12
	adcq 	%r13,%r13
	adcq 	%r14,%r14
	adcq 	%r15,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq	%rdx,%r14
	adcq	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12	
		
	// sub
	movq 	%r12,%rbx
	movq 	%r13,%rcx
	movq 	%r14,%rbp
	movq 	%r15,%rsi

	subq 	%r8,%r12
	sbbq 	%r9,%r13
	sbbq 	%r10,%r14
	sbbq 	%r11,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	sbbq	%rdx,%r12
	
	movq   %r12,336(%rsp)
	movq   %r13,344(%rsp)
	movq   %r14,352(%rsp)
	movq   %r15,360(%rsp)

	// add
	addq 	%rbx,%r8
	adcq 	%rcx,%r9
	adcq 	%rbp,%r10
	adcq 	%rsi,%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	adcq	%rdx,%r8

	movq   %r8,272(%rsp)
	movq   %r9,280(%rsp)
	movq   %r10,288(%rsp)
	movq   %r11,296(%rsp)

.L9:
	movq	56(%rsp),%rdi	
	
	/* p1p1 to p2 */	
	
	// mul
	movq    240(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    248(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    264(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	// mul
	movq    272(%rsp),%rdx

	mulx    304(%rsp),%r8,%r9
	mulx    312(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    320(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    328(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    304(%rsp),%rax,%rbx
	mulx    312(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    320(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    328(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,32(%rdi)
	movq    %r9,40(%rdi)
	movq    %r10,48(%rdi)
	movq    %r11,56(%rdi)

	// mul
	movq    272(%rsp),%rdx

	mulx    336(%rsp),%r8,%r9
	mulx    344(%rsp),%rcx,%r10
	addq    %rcx,%r9

	mulx    352(%rsp),%rcx,%r11
	adcq    %rcx,%r10

	mulx    360(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    280(%rsp),%rdx    

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    296(%rsp),%rdx

	mulx    336(%rsp),%rax,%rbx
	mulx    344(%rsp),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    352(%rsp),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    360(%rsp),%rcx,%r15
	adcq    %rcx,%rsi
	adcq    $0,%r15

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    %rbp,%r13
	adcq    %rsi,%r14
	adcq    $0,%r15

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

	movq    %r8,64(%rdi)
	movq    %r9,72(%rdi)
	movq    %r10,80(%rdi)
	movq    %r11,88(%rdi)
	
	movq	96(%rsp),%rax
	decq	%rax	
	movq	%rax,96(%rsp)	

	cmpq	$0,%rax
	
	jge	.L3
	
.L10:	

	movq 	 0(%rsp),%r11
	movq 	 8(%rsp),%r12
	movq 	16(%rsp),%r13
	movq 	24(%rsp),%r14
	movq 	32(%rsp),%r15
	movq 	40(%rsp),%rbx
	movq 	48(%rsp),%rbp

	movq 	%r11,%rsp

	ret
