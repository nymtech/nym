// linker define ge25519_double_scalarmult_precompute
// linker use upmask1 upmask2 upmask3 upmask4 upmask5
// linker use pmask1 pmask2 pmask3 pmask4 pmask5 pmask6 pmask7 pmask8 pmask9 pmask10 pmask11 pmask12
// linker use EC2D0 EC2D1 EC2D2 EC2D3 mask63 vec19 vecmask25 vecmask26

/* Assembly for the precomputaion phase used in double base scalar multiplication.
 * 
 * This assembly has been developed after studying the 
 * amd64-64-24k implementation of the work "High speed 
 * high security signatures" by Bernstein et al.
*/

#include "consts_namespace.h"

	.p2align 5
	.globl _CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_precompute)
	.globl CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_precompute)
	
_CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_precompute):
CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_precompute):

	movq   %rsp,%r11
	andq   $-32,%rsp
	subq   $1120,%rsp	

	movq	%r11,0(%rsp)
	movq	%r12,8(%rsp)
	movq	%r13,16(%rsp)
	movq	%r14,24(%rsp)
	movq	%r15,32(%rsp)
	movq	%rbx,40(%rsp)
	movq	%rbp,48(%rsp)
	
	decq	%rdx
	movq	%rdx,56(%rsp)

	movq	0(%rsi),%r8
	movq	8(%rsi),%r9
	movq	16(%rsi),%r10
	movq	24(%rsi),%r11

	movq	%r8,0(%rdi)
	movq	%r9,8(%rdi)
	movq	%r10,16(%rdi)
	movq	%r11,24(%rdi)

	movq	32(%rsi),%r8
	movq	40(%rsi),%r9
	movq	48(%rsi),%r10
	movq	56(%rsi),%r11

	movq	%r8,32(%rdi)
	movq	%r9,40(%rdi)
	movq	%r10,48(%rdi)
	movq	%r11,56(%rdi)

	movq	64(%rsi),%r8
	movq	72(%rsi),%r9
	movq	80(%rsi),%r10
	movq	88(%rsi),%r11

	movq	%r8,64(%rdi)
	movq	%r9,72(%rdi)
	movq	%r10,80(%rdi)
	movq	%r11,88(%rdi)

	movq	96(%rsi),%r8
	movq	104(%rsi),%r9
	movq	112(%rsi),%r10
	movq	120(%rsi),%r11

	movq	%r8,96(%rdi)
	movq	%r9,104(%rdi)
	movq	%r10,112(%rdi)
	movq	%r11,120(%rdi)

	/* dbl p1p1 */

	// square
	movq    0(%rdi),%rbx
	movq    8(%rdi),%rbp
	movq    16(%rdi),%rcx
	movq    24(%rdi),%rsi

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
	
	movq    %r8,64(%rsp)
	movq    %r10,72(%rsp)
	movq    %r12,80(%rsp)
	movq    %r14,88(%rsp)
	
	// square
	movq    32(%rdi),%rbx
	movq    40(%rdi),%rbp
	movq    48(%rdi),%rcx
	movq    56(%rdi),%rsi

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
	
	movq    %r8,96(%rsp)
	movq    %r10,104(%rsp)
	movq    %r12,112(%rsp)
	movq    %r14,120(%rsp)
	
	// square
	movq    64(%rdi),%rbx
	movq    72(%rdi),%rbp
	movq    80(%rdi),%rcx
	movq    88(%rdi),%rsi

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

	// double
	addq    %r8,%r8
	adcq 	%r10,%r10
	adcq	%r12,%r12
	adcq 	%r14,%r14

	movq  	$0,%rdx
	movq  	$38,%rcx
	cmovae	%rdx,%rcx

	addq  	%rcx,%r8
	adcq 	%rdx,%r10
	adcq 	%rdx,%r12
	adcq 	%rdx,%r14

	cmovc 	%rcx,%rdx
	addq  	%rdx,%r8
	
	movq    %r8,128(%rsp)
	movq    %r10,136(%rsp)
	movq    %r12,144(%rsp)
	movq    %r14,152(%rsp)

	// sub
	movq    $0,%r8
	movq    $0,%r9
	movq    $0,%r10
	movq    $0,%r11

	subq    64(%rsp),%r8
	sbbq    72(%rsp),%r9
	sbbq    80(%rsp),%r10
	sbbq    88(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,64(%rsp)
	movq    %r9,72(%rsp)
	movq    %r10,80(%rsp)
	movq    %r11,88(%rsp)

	// sub
	movq    $0,%r12
	movq    $0,%r13
	movq    $0,%r14
	movq    $0,%r15

	subq    96(%rsp),%r12
	sbbq    104(%rsp),%r13
	sbbq    112(%rsp),%r14
	sbbq    120(%rsp),%r15

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	subq    %rax,%r12
	sbbq    %rdx,%r13
	sbbq    %rdx,%r14
	sbbq    %rdx,%r15

	cmovc   %rax,%rdx
	subq    %rdx,%r12

	movq    %r12,160(%rsp)
	movq    %r13,168(%rsp)
	movq    %r14,176(%rsp)
	movq    %r15,184(%rsp)

	// add
	movq    %r8,%r12
	movq    %r9,%r13
	movq    %r10,%r14
	movq    %r11,%r15

	addq    96(%rsp),%r12
	adcq    104(%rsp),%r13
	adcq    112(%rsp),%r14
	adcq    120(%rsp),%r15

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r12
	adcq    %rdx,%r13
	adcq    %rdx,%r14
	adcq    %rdx,%r15

	cmovc   %rax,%rdx
	subq    %rdx,%r12

	movq    %r12,224(%rsp)
	movq    %r13,232(%rsp)
	movq    %r14,240(%rsp)
	movq    %r15,248(%rsp)

	// sub
	subq    96(%rsp),%r8
	sbbq    104(%rsp),%r9
	sbbq    112(%rsp),%r10
	sbbq    120(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,256(%rsp)
	movq    %r9,264(%rsp)
	movq    %r10,272(%rsp)
	movq    %r11,280(%rsp)

	// sub
	subq    128(%rsp),%r12
	sbbq    136(%rsp),%r13
	sbbq    144(%rsp),%r14
	sbbq    152(%rsp),%r15

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	subq    %rax,%r12
	sbbq    %rdx,%r13
	sbbq    %rdx,%r14
	sbbq    %rdx,%r15

	cmovc   %rax,%rdx
	subq    %rdx,%r12

	movq    %r12,288(%rsp)
	movq    %r13,296(%rsp)
	movq    %r14,304(%rsp)
	movq    %r15,312(%rsp)

	// add
	movq    0(%rdi),%r8
	movq    8(%rdi),%r9
	movq    16(%rdi),%r10
	movq    24(%rdi),%r11

	addq    32(%rdi),%r8
	adcq    40(%rdi),%r9
	adcq    48(%rdi),%r10
	adcq    56(%rdi),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax
	
	addq    %rax,%r8
	adcq    %rdx,%r9
	adcq    %rdx,%r10
	adcq    %rdx,%r11
	
	cmovc   %rax,%rdx
	addq    %rdx,%r8

	movq    %r8,96(%rsp)
	movq    %r9,104(%rsp)
	movq    %r10,112(%rsp)
	movq    %r11,120(%rsp)

	// square
	movq    96(%rsp),%rbx
	movq    104(%rsp),%rbp
	movq    112(%rsp),%rcx
	movq    120(%rsp),%rsi

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

	// add
	addq    64(%rsp),%r8
	adcq    72(%rsp),%r10
	adcq    80(%rsp),%r12
	adcq    88(%rsp),%r14

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r8
	adcq    %rdx,%r10
	adcq    %rdx,%r12
	adcq    %rdx,%r14

	cmovc   %rax,%rdx
	addq    %rdx,%r8

	addq    160(%rsp),%r8
	adcq    168(%rsp),%r10
	adcq    176(%rsp),%r12
	adcq    184(%rsp),%r14

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r8
	adcq    %rdx,%r10
	adcq    %rdx,%r12
	adcq    %rdx,%r14

	cmovc   %rax,%rdx
	addq    %rdx,%r8

	movq    %r8,192(%rsp)
	movq    %r10,200(%rsp)
	movq    %r12,208(%rsp)
	movq    %r14,216(%rsp)
	
	/* p1p1 to p3 */

	// convert to 10x4 form
	vmovdqa   192(%rsp),%ymm8
	vmovdqa   224(%rsp),%ymm9
	vmovdqa   224(%rsp),%ymm10
	vmovdqa   192(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm1
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm3
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm5
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm7

	vpand     pmask1(%rip),%ymm1,%ymm0

	vpand     pmask2(%rip),%ymm1,%ymm11
	vpsrlq    $26,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm1,%ymm2
	vpsrlq    $51,%ymm2,%ymm2
	vpand     pmask4(%rip),%ymm3,%ymm9
	vpsllq    $13,%ymm9,%ymm9
	vpor      %ymm9,%ymm2,%ymm2

	vpand     pmask5(%rip),%ymm3,%ymm12
	vpsrlq    $13,%ymm12,%ymm12

	vpand     pmask6(%rip),%ymm3,%ymm4
	vpsrlq    $38,%ymm4,%ymm4

	vpand     pmask7(%rip),%ymm5,%ymm13

	vpand     pmask8(%rip),%ymm5,%ymm6
	vpsrlq    $25,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm5,%ymm14
	vpsrlq    $51,%ymm14,%ymm14
	vpand     pmask10(%rip),%ymm7,%ymm9
	vpsllq    $13,%ymm9,%ymm9
	vpor      %ymm9,%ymm14,%ymm14

	vpand     pmask11(%rip),%ymm7,%ymm8
	vpsrlq    $12,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm7,%ymm15
	vpsrlq    $38,%ymm15,%ymm15

	vmovdqa   %ymm0,480(%rsp)
	vmovdqa   %ymm11,512(%rsp)
	vmovdqa   %ymm2,544(%rsp)
	vmovdqa   %ymm12,576(%rsp)
	vmovdqa   %ymm4,608(%rsp)
	vmovdqa   %ymm13,640(%rsp)
	vmovdqa   %ymm6,672(%rsp)
	vmovdqa   %ymm14,704(%rsp)
	vmovdqa   %ymm8,736(%rsp)
	vmovdqa   %ymm15,768(%rsp)

	// convert to 10x4 form
	vmovdqa   288(%rsp),%ymm8
	vmovdqa   256(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm10
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm11
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm12
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm13

	vpand     pmask1(%rip),%ymm10,%ymm0

	vpand     pmask2(%rip),%ymm10,%ymm1
	vpsrlq    $26,%ymm1,%ymm1

	vpand     pmask3(%rip),%ymm10,%ymm2
	vpsrlq    $51,%ymm2,%ymm2
	vpand     pmask4(%rip),%ymm11,%ymm3
	vpsllq    $13,%ymm3,%ymm3
	vpor      %ymm3,%ymm2,%ymm2

	vpand     pmask5(%rip),%ymm11,%ymm3
	vpsrlq    $13,%ymm3,%ymm3

	vpand     pmask6(%rip),%ymm11,%ymm4
	vpsrlq    $38,%ymm4,%ymm4

	vpand     pmask7(%rip),%ymm12,%ymm5

	vpand     pmask8(%rip),%ymm12,%ymm6
	vpsrlq    $25,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm12,%ymm7
	vpsrlq    $51,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm13,%ymm8
	vpsllq    $13,%ymm8,%ymm8
	vpor      %ymm8,%ymm7,%ymm7

	vpand     pmask11(%rip),%ymm13,%ymm8
	vpsrlq    $12,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm13,%ymm9
	vpsrlq    $38,%ymm9,%ymm9

	vmovdqa   %ymm0,800(%rsp)
	vmovdqa   %ymm1,832(%rsp)
	vmovdqa   %ymm2,864(%rsp)
	vmovdqa   %ymm3,896(%rsp)
	vmovdqa   %ymm4,928(%rsp)
	vmovdqa   %ymm5,960(%rsp)
	vmovdqa   %ymm6,992(%rsp)
	vmovdqa   %ymm7,1024(%rsp)
	vmovdqa   %ymm8,1056(%rsp)
	vmovdqa   %ymm9,1088(%rsp)

	// mul4x1
	vmovdqa   512(%rsp),%ymm11
	vmovdqa   576(%rsp),%ymm12
	vmovdqa   640(%rsp),%ymm13
	vmovdqa   704(%rsp),%ymm14
	vmovdqa   768(%rsp),%ymm15

	vpaddq    %ymm11,%ymm11,%ymm11
	vpaddq    %ymm12,%ymm12,%ymm12
	vpaddq    %ymm13,%ymm13,%ymm13
	vpaddq    %ymm14,%ymm14,%ymm14
	vpaddq    %ymm15,%ymm15,%ymm15

	vpmuludq  vec19(%rip),%ymm1,%ymm1
	vpmuludq  vec19(%rip),%ymm2,%ymm2
	vpmuludq  vec19(%rip),%ymm3,%ymm3
	vpmuludq  vec19(%rip),%ymm4,%ymm4
	vpmuludq  vec19(%rip),%ymm5,%ymm5
	vpmuludq  vec19(%rip),%ymm6,%ymm6
	vpmuludq  vec19(%rip),%ymm7,%ymm7
	vpmuludq  vec19(%rip),%ymm8,%ymm8
	vpmuludq  vec19(%rip),%ymm9,%ymm9

	vpmuludq  %ymm15,%ymm1,%ymm0
	vpmuludq  %ymm14,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm13,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm12,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm11,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  736(%rsp),%ymm2,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  672(%rsp),%ymm4,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  608(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  544(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  768(%rsp),%ymm2,%ymm1
	vpmuludq  736(%rsp),%ymm3,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  704(%rsp),%ymm4,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  672(%rsp),%ymm5,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  640(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  608(%rsp),%ymm7,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  576(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  544(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  %ymm15,%ymm3,%ymm2
	vpmuludq  %ymm14,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm13,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm12,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  736(%rsp),%ymm4,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  672(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  608(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  832(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  768(%rsp),%ymm4,%ymm3
	vpmuludq  736(%rsp),%ymm5,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  704(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  672(%rsp),%ymm7,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  640(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  608(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  %ymm15,%ymm5,%ymm4
	vpmuludq  %ymm14,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm13,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  736(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  672(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  832(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  896(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  768(%rsp),%ymm6,%ymm5
	vpmuludq  736(%rsp),%ymm7,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  704(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  672(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  %ymm15,%ymm7,%ymm6
	vpmuludq  %ymm14,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  736(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  832(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  896(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  960(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  768(%rsp),%ymm8,%ymm7
	vpmuludq  736(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  %ymm15,%ymm9,%ymm8
	vpmuludq  832(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  896(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  960(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  1024(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vmovdqa   480(%rsp),%ymm11
	vmovdqa   512(%rsp),%ymm12
	vmovdqa   544(%rsp),%ymm13
	vmovdqa   576(%rsp),%ymm14
	vmovdqa   608(%rsp),%ymm15
	vmovdqa   640(%rsp),%ymm9

	vpmuludq  800(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  864(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  800(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  896(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  864(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  832(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  800(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  832(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  800(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  928(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  864(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  800(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  960(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  928(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  896(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  864(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  832(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  800(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  992(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  928(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  864(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  1024(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  992(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  960(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  928(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  896(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  864(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  1056(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  992(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  928(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  928(%rsp),%ymm9,%ymm9
	vpmuludq  1088(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  1056(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  1024(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  992(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  960(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9

	vmovdqa   672(%rsp),%ymm11
	vmovdqa   704(%rsp),%ymm12
	vmovdqa   736(%rsp),%ymm13
	vmovdqa   768(%rsp),%ymm14

	vpmuludq  800(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  832(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  800(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  864(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  800(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  896(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  864(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  832(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  800(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9

	vpsrlq    $26,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpand     vecmask26(%rip),%ymm8,%ymm8

	vpsrlq    $25,%ymm9,%ymm10
	vpmuludq  vec19(%rip),%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask25(%rip),%ymm9,%ymm9

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $25,%ymm1,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask25(%rip),%ymm1,%ymm1

	vpsrlq    $26,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpand     vecmask26(%rip),%ymm2,%ymm2

	vpsrlq    $25,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpand     vecmask25(%rip),%ymm3,%ymm3

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask26(%rip),%ymm4,%ymm4

	vpsrlq    $25,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask25(%rip),%ymm5,%ymm5

	vpsrlq    $26,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpand     vecmask26(%rip),%ymm6,%ymm6

	vpsrlq    $25,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpand     vecmask25(%rip),%ymm7,%ymm7

	vpsrlq    $26,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpand     vecmask26(%rip),%ymm8,%ymm8

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm0,%ymm10
	vpand     upmask2(%rip),%ymm1,%ymm11
	vpsllq    $26,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10
	vpand     upmask3(%rip),%ymm2,%ymm11
	vpsllq    $51,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10

	vpand     upmask4(%rip),%ymm2,%ymm11
	vpsrlq    $13,%ymm11,%ymm11
	vpand     upmask2(%rip),%ymm3,%ymm12
	vpsllq    $13,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11
	vpand     upmask1(%rip),%ymm4,%ymm12
	vpsllq    $38,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11

	vpand     upmask2(%rip),%ymm5,%ymm12
	vpand     upmask1(%rip),%ymm6,%ymm13
	vpsllq    $25,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12
	vpand     upmask3(%rip),%ymm7,%ymm13
	vpsllq    $51,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12

	vpand     upmask5(%rip),%ymm7,%ymm13
	vpsrlq    $13,%ymm13,%ymm13
	vpand     upmask1(%rip),%ymm8,%ymm14
	vpsllq    $12,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13
	vpand     upmask1(%rip),%ymm9,%ymm14
	vpsllq    $38,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13

	vpunpcklqdq    %ymm11,%ymm10,%ymm2
	vpunpckhqdq    %ymm11,%ymm10,%ymm3
	vpunpcklqdq    %ymm13,%ymm12,%ymm4
	vpunpckhqdq    %ymm13,%ymm12,%ymm5

	vpermq    $68,%ymm4,%ymm7
	vpblendd  $240,%ymm7,%ymm2,%ymm10
	vpermq    $68,%ymm5,%ymm7
	vpblendd  $240,%ymm7,%ymm3,%ymm11
	vpermq    $238,%ymm2,%ymm7
	vpblendd  $240,%ymm4,%ymm7,%ymm12
	vpermq    $238,%ymm3,%ymm7
	vpblendd  $240,%ymm5,%ymm7,%ymm13

	vmovdqa   %ymm10,64(%rsp)
	vmovdqa   %ymm11,96(%rsp)
	vmovdqa   %ymm12,128(%rsp)
	vmovdqa   %ymm13,160(%rsp)
	
	// Convert pre[0] to projective Niels representation	
	movq	0(%rdi),%rbx
	movq	8(%rdi),%rcx
	movq	16(%rdi),%rbp
	movq	24(%rdi),%rsi	
	
	movq	32(%rdi),%r8
	movq	40(%rdi),%r9
	movq	48(%rdi),%r10
	movq	56(%rdi),%r11
	
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	subq 	%rbx,%r8
	sbbq 	%rcx,%r9
	sbbq 	%rbp,%r10
	sbbq 	%rsi,%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax
	
	cmovae	%rdx,%rax
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,0(%rdi)
	movq   %r9,8(%rdi)
	movq   %r10,16(%rdi)
	movq   %r11,24(%rdi)
	
	addq 	%rbx,%r12
	adcq 	%rcx,%r13
	adcq 	%rbp,%r14
	adcq 	%rsi,%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax
	
	cmovae	%rdx,%rax
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,32(%rdi)
	movq   %r13,40(%rdi)
	movq   %r14,48(%rdi)
	movq   %r15,56(%rdi)
	
	// mul
	movq    EC2D1(%rip),%rax
	mulq    120(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    EC2D2(%rip),%rax
	mulq    112(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    EC2D3(%rip),%rax
	mulq    104(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    EC2D2(%rip),%rax
	mulq    120(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    EC2D3(%rip),%rax
	mulq    112(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    EC2D3(%rip),%rax
	mulq    120(%rdi)
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

	movq    EC2D0(%rip),%rax
	mulq    120(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    EC2D1(%rip),%rax
	mulq    112(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    EC2D2(%rip),%rax
	mulq    104(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    EC2D3(%rip),%rax
	mulq    96(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    EC2D0(%rip),%rax
	mulq    96(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    EC2D0(%rip),%rax
	mulq    104(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    EC2D1(%rip),%rax
	mulq    96(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    EC2D0(%rip),%rax
	mulq    112(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    EC2D1(%rip),%rax
	mulq    104(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    EC2D2(%rip),%rax
	mulq    96(%rdi)
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

	movq    %r8,96(%rdi)
	movq    %r10,104(%rdi)
	movq    %r12,112(%rdi)
	movq    %r14,120(%rdi)
	
	movq	$0,384(%rsp)
	
.L:	
	// pnielsadd_p1p1
	
	movq	96(%rsp),%r8
	movq	104(%rsp),%r9
	movq	112(%rsp),%r10
	movq	120(%rsp),%r11
	
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	subq 	64(%rsp),%r8
	sbbq 	72(%rsp),%r9
	sbbq 	80(%rsp),%r10
	sbbq 	88(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,320(%rsp)
	movq   %r9,328(%rsp)
	movq   %r10,336(%rsp)
	movq   %r11,344(%rsp)
	
	addq 	64(%rsp),%r12
	adcq 	72(%rsp),%r13
	adcq 	80(%rsp),%r14
	adcq 	88(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,352(%rsp)
	movq   %r13,360(%rsp)
	movq   %r14,368(%rsp)
	movq   %r15,376(%rsp)
	
	// mul
	movq    328(%rsp),%rax
	mulq    24(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    336(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    344(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    336(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    344(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    344(%rsp),%rax
	mulq    24(%rdi)
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

	movq    320(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    328(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    336(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    344(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    320(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    320(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    328(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    320(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    328(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    336(%rsp),%rax
	mulq    0(%rdi)
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

	movq    %r8,320(%rsp)
	movq    %r10,328(%rsp)
	movq    %r12,336(%rsp)
	movq    %r14,344(%rsp)

	// mul
	movq    360(%rsp),%rax
	mulq    56(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    368(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    376(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    368(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    376(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    376(%rsp),%rax
	mulq    56(%rdi)
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

	movq    352(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    360(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    368(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    376(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    352(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    352(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    360(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    352(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    360(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    368(%rsp),%rax
	mulq    32(%rdi)
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

	// add
	movq 	%r8,%r9
	movq 	%r10,%r11
	movq 	%r12,%r13
	movq 	%r14,%r15

	addq 	320(%rsp),%r8
	adcq 	328(%rsp),%r10
	adcq 	336(%rsp),%r12
	adcq 	344(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,256(%rsp)
	movq   %r10,264(%rsp)
	movq   %r12,272(%rsp)
	movq   %r14,280(%rsp)

	// sub
	subq 	320(%rsp),%r9
	sbbq 	328(%rsp),%r11
	sbbq 	336(%rsp),%r13
	sbbq 	344(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,192(%rsp)
	movq   %r11,200(%rsp)
	movq   %r13,208(%rsp)
	movq   %r15,216(%rsp)

	// mul	
	movq    168(%rsp),%rax
	mulq    120(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    176(%rsp),%rax
	mulq    112(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    184(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    176(%rsp),%rax
	mulq    120(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    184(%rsp),%rax
	mulq    112(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    184(%rsp),%rax
	mulq    120(%rdi)
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

	movq    160(%rsp),%rax
	mulq    120(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    168(%rsp),%rax
	mulq    112(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    176(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    184(%rsp),%rax
	mulq    96(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    160(%rsp),%rax
	mulq    96(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    160(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    168(%rsp),%rax
	mulq    96(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    160(%rsp),%rax
	mulq    112(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    168(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    176(%rsp),%rax
	mulq    96(%rdi)
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

	movq    %r8,320(%rsp)
	movq    %r10,328(%rsp)
	movq    %r12,336(%rsp)
	movq    %r14,344(%rsp)
	
	// mul	
	movq    136(%rsp),%rax
	mulq    88(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    144(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    152(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    144(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    152(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    152(%rsp),%rax
	mulq    88(%rdi)
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

	movq    128(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    136(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    144(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    152(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    128(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    128(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    136(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    128(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    136(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    144(%rsp),%rax
	mulq    64(%rdi)
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
	
	// double
	addq 	%r8,%r8
	adcq 	%r10,%r10
	adcq 	%r12,%r12
	adcq 	%r14,%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8	
		
	// add
	movq 	%r8,%r9
	movq 	%r10,%r11
	movq 	%r12,%r13
	movq 	%r14,%r15

	addq 	320(%rsp),%r8
	adcq 	328(%rsp),%r10
	adcq 	336(%rsp),%r12
	adcq 	344(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,224(%rsp)
	movq   %r10,232(%rsp)
	movq   %r12,240(%rsp)
	movq   %r14,248(%rsp)

	// sub
	subq 	320(%rsp),%r9
	sbbq 	328(%rsp),%r11
	sbbq 	336(%rsp),%r13
	sbbq 	344(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,288(%rsp)
	movq   %r11,296(%rsp)
	movq   %r13,304(%rsp)
	movq   %r15,312(%rsp)
	
	/* p1p1 to p3 */

	// convert to 10x4 form
	vmovdqa   192(%rsp),%ymm8
	vmovdqa   224(%rsp),%ymm9
	vmovdqa   224(%rsp),%ymm10
	vmovdqa   192(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm1
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm3
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm5
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm7

	vpand     pmask1(%rip),%ymm1,%ymm0

	vpand     pmask2(%rip),%ymm1,%ymm11
	vpsrlq    $26,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm1,%ymm2
	vpsrlq    $51,%ymm2,%ymm2
	vpand     pmask4(%rip),%ymm3,%ymm9
	vpsllq    $13,%ymm9,%ymm9
	vpor      %ymm9,%ymm2,%ymm2

	vpand     pmask5(%rip),%ymm3,%ymm12
	vpsrlq    $13,%ymm12,%ymm12

	vpand     pmask6(%rip),%ymm3,%ymm4
	vpsrlq    $38,%ymm4,%ymm4

	vpand     pmask7(%rip),%ymm5,%ymm13

	vpand     pmask8(%rip),%ymm5,%ymm6
	vpsrlq    $25,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm5,%ymm14
	vpsrlq    $51,%ymm14,%ymm14
	vpand     pmask10(%rip),%ymm7,%ymm9
	vpsllq    $13,%ymm9,%ymm9
	vpor      %ymm9,%ymm14,%ymm14

	vpand     pmask11(%rip),%ymm7,%ymm8
	vpsrlq    $12,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm7,%ymm15
	vpsrlq    $38,%ymm15,%ymm15

	vmovdqa   %ymm0,480(%rsp)
	vmovdqa   %ymm11,512(%rsp)
	vmovdqa   %ymm2,544(%rsp)
	vmovdqa   %ymm12,576(%rsp)
	vmovdqa   %ymm4,608(%rsp)
	vmovdqa   %ymm13,640(%rsp)
	vmovdqa   %ymm6,672(%rsp)
	vmovdqa   %ymm14,704(%rsp)
	vmovdqa   %ymm8,736(%rsp)
	vmovdqa   %ymm15,768(%rsp)

	// convert to 10x4 form
	vmovdqa   288(%rsp),%ymm8
	vmovdqa   256(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm10
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm11
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm12
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm13

	vpand     pmask1(%rip),%ymm10,%ymm0

	vpand     pmask2(%rip),%ymm10,%ymm1
	vpsrlq    $26,%ymm1,%ymm1

	vpand     pmask3(%rip),%ymm10,%ymm2
	vpsrlq    $51,%ymm2,%ymm2
	vpand     pmask4(%rip),%ymm11,%ymm3
	vpsllq    $13,%ymm3,%ymm3
	vpor      %ymm3,%ymm2,%ymm2

	vpand     pmask5(%rip),%ymm11,%ymm3
	vpsrlq    $13,%ymm3,%ymm3

	vpand     pmask6(%rip),%ymm11,%ymm4
	vpsrlq    $38,%ymm4,%ymm4

	vpand     pmask7(%rip),%ymm12,%ymm5

	vpand     pmask8(%rip),%ymm12,%ymm6
	vpsrlq    $25,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm12,%ymm7
	vpsrlq    $51,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm13,%ymm8
	vpsllq    $13,%ymm8,%ymm8
	vpor      %ymm8,%ymm7,%ymm7

	vpand     pmask11(%rip),%ymm13,%ymm8
	vpsrlq    $12,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm13,%ymm9
	vpsrlq    $38,%ymm9,%ymm9

	vmovdqa   %ymm0,800(%rsp)
	vmovdqa   %ymm1,832(%rsp)
	vmovdqa   %ymm2,864(%rsp)
	vmovdqa   %ymm3,896(%rsp)
	vmovdqa   %ymm4,928(%rsp)
	vmovdqa   %ymm5,960(%rsp)
	vmovdqa   %ymm6,992(%rsp)
	vmovdqa   %ymm7,1024(%rsp)
	vmovdqa   %ymm8,1056(%rsp)
	vmovdqa   %ymm9,1088(%rsp)

	// mul4x1
	vmovdqa   512(%rsp),%ymm11
	vmovdqa   576(%rsp),%ymm12
	vmovdqa   640(%rsp),%ymm13
	vmovdqa   704(%rsp),%ymm14
	vmovdqa   768(%rsp),%ymm15

	vpaddq    %ymm11,%ymm11,%ymm11
	vpaddq    %ymm12,%ymm12,%ymm12
	vpaddq    %ymm13,%ymm13,%ymm13
	vpaddq    %ymm14,%ymm14,%ymm14
	vpaddq    %ymm15,%ymm15,%ymm15

	vpmuludq  vec19(%rip),%ymm1,%ymm1
	vpmuludq  vec19(%rip),%ymm2,%ymm2
	vpmuludq  vec19(%rip),%ymm3,%ymm3
	vpmuludq  vec19(%rip),%ymm4,%ymm4
	vpmuludq  vec19(%rip),%ymm5,%ymm5
	vpmuludq  vec19(%rip),%ymm6,%ymm6
	vpmuludq  vec19(%rip),%ymm7,%ymm7
	vpmuludq  vec19(%rip),%ymm8,%ymm8
	vpmuludq  vec19(%rip),%ymm9,%ymm9

	vpmuludq  %ymm15,%ymm1,%ymm0
	vpmuludq  %ymm14,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm13,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm12,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm11,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  736(%rsp),%ymm2,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  672(%rsp),%ymm4,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  608(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  544(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  768(%rsp),%ymm2,%ymm1
	vpmuludq  736(%rsp),%ymm3,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  704(%rsp),%ymm4,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  672(%rsp),%ymm5,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  640(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  608(%rsp),%ymm7,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  576(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  544(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  %ymm15,%ymm3,%ymm2
	vpmuludq  %ymm14,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm13,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm12,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  736(%rsp),%ymm4,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  672(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  608(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  832(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  768(%rsp),%ymm4,%ymm3
	vpmuludq  736(%rsp),%ymm5,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  704(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  672(%rsp),%ymm7,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  640(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  608(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  %ymm15,%ymm5,%ymm4
	vpmuludq  %ymm14,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm13,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  736(%rsp),%ymm6,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  672(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  832(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  896(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  768(%rsp),%ymm6,%ymm5
	vpmuludq  736(%rsp),%ymm7,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  704(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  672(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  %ymm15,%ymm7,%ymm6
	vpmuludq  %ymm14,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  736(%rsp),%ymm8,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  832(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  896(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  960(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  768(%rsp),%ymm8,%ymm7
	vpmuludq  736(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  %ymm15,%ymm9,%ymm8
	vpmuludq  832(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  896(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  960(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  1024(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vmovdqa   480(%rsp),%ymm11
	vmovdqa   512(%rsp),%ymm12
	vmovdqa   544(%rsp),%ymm13
	vmovdqa   576(%rsp),%ymm14
	vmovdqa   608(%rsp),%ymm15
	vmovdqa   640(%rsp),%ymm9

	vpmuludq  800(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  864(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  800(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  896(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  864(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  832(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  800(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  832(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  800(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  928(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  864(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  800(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  960(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  928(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  896(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  864(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  832(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  800(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  992(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  928(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  864(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  1024(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  992(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  960(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  928(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  896(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  864(%rsp),%ymm9,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  1056(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  992(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  928(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  928(%rsp),%ymm9,%ymm9
	vpmuludq  1088(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  1056(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  1024(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  992(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  960(%rsp),%ymm15,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9

	vmovdqa   672(%rsp),%ymm11
	vmovdqa   704(%rsp),%ymm12
	vmovdqa   736(%rsp),%ymm13
	vmovdqa   768(%rsp),%ymm14

	vpmuludq  800(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  832(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  800(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  864(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  800(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  896(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  864(%rsp),%ymm12,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  832(%rsp),%ymm13,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  800(%rsp),%ymm14,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9

	vpsrlq    $26,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpand     vecmask26(%rip),%ymm8,%ymm8

	vpsrlq    $25,%ymm9,%ymm10
	vpmuludq  vec19(%rip),%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask25(%rip),%ymm9,%ymm9

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $25,%ymm1,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask25(%rip),%ymm1,%ymm1

	vpsrlq    $26,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpand     vecmask26(%rip),%ymm2,%ymm2

	vpsrlq    $25,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpand     vecmask25(%rip),%ymm3,%ymm3

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask26(%rip),%ymm4,%ymm4

	vpsrlq    $25,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask25(%rip),%ymm5,%ymm5

	vpsrlq    $26,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpand     vecmask26(%rip),%ymm6,%ymm6

	vpsrlq    $25,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpand     vecmask25(%rip),%ymm7,%ymm7

	vpsrlq    $26,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpand     vecmask26(%rip),%ymm8,%ymm8

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm0,%ymm10
	vpand     upmask2(%rip),%ymm1,%ymm11
	vpsllq    $26,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10
	vpand     upmask3(%rip),%ymm2,%ymm11
	vpsllq    $51,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10

	vpand     upmask4(%rip),%ymm2,%ymm11
	vpsrlq    $13,%ymm11,%ymm11
	vpand     upmask2(%rip),%ymm3,%ymm12
	vpsllq    $13,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11
	vpand     upmask1(%rip),%ymm4,%ymm12
	vpsllq    $38,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11

	vpand     upmask2(%rip),%ymm5,%ymm12
	vpand     upmask1(%rip),%ymm6,%ymm13
	vpsllq    $25,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12
	vpand     upmask3(%rip),%ymm7,%ymm13
	vpsllq    $51,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12

	vpand     upmask5(%rip),%ymm7,%ymm13
	vpsrlq    $13,%ymm13,%ymm13
	vpand     upmask1(%rip),%ymm8,%ymm14
	vpsllq    $12,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13
	vpand     upmask1(%rip),%ymm9,%ymm14
	vpsllq    $38,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13

	vpunpcklqdq    %ymm11,%ymm10,%ymm2
	vpunpckhqdq    %ymm11,%ymm10,%ymm3
	vpunpcklqdq    %ymm13,%ymm12,%ymm4
	vpunpckhqdq    %ymm13,%ymm12,%ymm5

	vpermq    $68,%ymm4,%ymm7
	vpblendd  $240,%ymm7,%ymm2,%ymm10
	vpermq    $68,%ymm5,%ymm7
	vpblendd  $240,%ymm7,%ymm3,%ymm11
	vpermq    $238,%ymm2,%ymm7
	vpblendd  $240,%ymm4,%ymm7,%ymm12
	vpermq    $238,%ymm3,%ymm7
	vpblendd  $240,%ymm5,%ymm7,%ymm13

	vmovdqa   %ymm10,128(%rdi)
	vmovdqa   %ymm11,160(%rdi)
	vmovdqa   %ymm12,192(%rdi)
	vmovdqa   %ymm13,224(%rdi)
	
	// Convert pre[i1] to projective Niels representation	
	movq	128(%rdi),%rbx
	movq	136(%rdi),%rcx
	movq	144(%rdi),%rbp
	movq	152(%rdi),%rsi	
	
	movq	160(%rdi),%r8
	movq	168(%rdi),%r9
	movq	176(%rdi),%r10
	movq	184(%rdi),%r11
	
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	subq 	%rbx,%r8
	sbbq 	%rcx,%r9
	sbbq 	%rbp,%r10
	sbbq 	%rsi,%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax
	
	cmovae	%rdx,%rax
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9
	
	movq   %r8,128(%rdi)
	movq   %r9,136(%rdi)
	movq   %r10,144(%rdi)
	movq   %r11,152(%rdi)
	
	addq 	%rbx,%r12
	adcq 	%rcx,%r13
	adcq 	%rbp,%r14
	adcq 	%rsi,%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax
	
	cmovae	%rdx,%rax
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,160(%rdi)
	movq   %r13,168(%rdi)
	movq   %r14,176(%rdi)
	movq   %r15,184(%rdi)
	
	// mul
	movq    EC2D1(%rip),%rax
	mulq    248(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    EC2D2(%rip),%rax
	mulq    240(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    EC2D3(%rip),%rax
	mulq    232(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    EC2D2(%rip),%rax
	mulq    248(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    EC2D3(%rip),%rax
	mulq    240(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    EC2D3(%rip),%rax
	mulq    248(%rdi)
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

	movq    EC2D0(%rip),%rax
	mulq    248(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    EC2D1(%rip),%rax
	mulq    240(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    EC2D2(%rip),%rax
	mulq    232(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    EC2D3(%rip),%rax
	mulq    224(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    EC2D0(%rip),%rax
	mulq    224(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    EC2D0(%rip),%rax
	mulq    232(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    EC2D1(%rip),%rax
	mulq    224(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    EC2D0(%rip),%rax
	mulq    240(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    EC2D1(%rip),%rax
	mulq    232(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    EC2D2(%rip),%rax
	mulq    224(%rdi)
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

	movq    %r8,224(%rdi)
	movq    %r10,232(%rdi)
	movq    %r12,240(%rdi)
	movq    %r14,248(%rdi)

	addq	$128,%rdi
	
	movq	384(%rsp),%r8
	incq	%r8	
	movq	%r8,384(%rsp)	

	cmpq	56(%rsp),%r8
	
	jl	.L
	
	movq 	 0(%rsp),%r11
	movq 	 8(%rsp),%r12
	movq 	16(%rsp),%r13
	movq 	24(%rsp),%r14
	movq 	32(%rsp),%r15
	movq 	40(%rsp),%rbx
	movq 	48(%rsp),%rbp

	movq 	%r11,%rsp

	ret
