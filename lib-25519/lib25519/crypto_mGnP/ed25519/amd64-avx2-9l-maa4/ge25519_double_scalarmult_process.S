// linker define ge25519_double_scalarmult_process
// linker use upmask1 upmask2 upmask3 upmask4 upmask5 upmask6 upmask7 upmask8
// linker use pmask1 pmask2 pmask3 pmask4 pmask5 pmask6 pmask7 pmask8 pmask9 pmask10 pmask11 pmask12
// linker use mask63 vec1216 vecmask23 vecmask29

/* Assembly for double base scalar multiplication.
 * 
 * This assembly has been developed after studying the 
 * amd64-64-24k implementation of the work "High speed 
 * high security signatures" by Bernstein et al.
*/

#include "consts_namespace.h"

        .p2align 5
        .globl _CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process)
        .globl CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process)

_CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process):
CRYPTO_SHARED_NAMESPACE(ge25519_double_scalarmult_process):

	movq	%rsp,%r11
	andq	$-32,%rsp
	subq  	$1536,%rsp 	

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
	
	movq    %r8,128(%rsp)
	movq    %r10,136(%rsp)
	movq    %r12,144(%rsp)
	movq    %r14,152(%rsp)
	
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
	
	movq    %r8,160(%rsp)
	movq    %r10,168(%rsp)
	movq    %r12,176(%rsp)
	movq    %r14,184(%rsp)
	
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
	
	movq    %r8,192(%rsp)
	movq    %r10,200(%rsp)
	movq    %r12,208(%rsp)
	movq    %r14,216(%rsp)

	// neg
	movq    $0,%r8
	movq    $0,%r9
	movq    $0,%r10
	movq    $0,%r11

	subq    128(%rsp),%r8
	sbbq    136(%rsp),%r9
	sbbq    144(%rsp),%r10
	sbbq    152(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,128(%rsp)
	movq    %r9,136(%rsp)
	movq    %r10,144(%rsp)
	movq    %r11,152(%rsp)

	// copy
	movq    %r8,%r12
	movq    %r9,%r13
	movq    %r10,%r14
	movq    %r11,%r15
	
	// sub
	subq    160(%rsp),%r8
	sbbq    168(%rsp),%r9
	sbbq    176(%rsp),%r10
	sbbq    184(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r9
	sbbq    %rdx,%r10
	sbbq    %rdx,%r11

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,320(%rsp)
	movq    %r9,328(%rsp)
	movq    %r10,336(%rsp)
	movq    %r11,344(%rsp)	

	// add
	addq    160(%rsp),%r12
	adcq    168(%rsp),%r13
	adcq    176(%rsp),%r14
	adcq    184(%rsp),%r15

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r12
	adcq    %rdx,%r13
	adcq    %rdx,%r14
	adcq    %rdx,%r15

	cmovc   %rax,%rdx
	subq    %rdx,%r12

	movq    %r12,288(%rsp)
	movq    %r13,296(%rsp)
	movq    %r14,304(%rsp)
	movq    %r15,312(%rsp)

	// sub
	subq    192(%rsp),%r12
	sbbq    200(%rsp),%r13
	sbbq    208(%rsp),%r14
	sbbq    216(%rsp),%r15

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	subq    %rax,%r12
	sbbq    %rdx,%r13
	sbbq    %rdx,%r14
	sbbq    %rdx,%r15

	cmovc   %rax,%rdx
	subq    %rdx,%r12

	movq    %r12,352(%rsp)
	movq    %r13,360(%rsp)
	movq    %r14,368(%rsp)
	movq    %r15,376(%rsp)

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
	addq    128(%rsp),%r8
	adcq    136(%rsp),%r10
	adcq    144(%rsp),%r12
	adcq    152(%rsp),%r14

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r8
	adcq    %rdx,%r10
	adcq    %rdx,%r12
	adcq    %rdx,%r14

	cmovc   %rax,%rdx
	addq    %rdx,%r8

	// sub
	subq    160(%rsp),%r8
	sbbq    168(%rsp),%r10
	sbbq    176(%rsp),%r12
	sbbq    184(%rsp),%r14

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	subq    %rax,%r8
	sbbq    %rdx,%r10
	sbbq    %rdx,%r12
	sbbq    %rdx,%r14

	cmovc   %rax,%rdx
	subq    %rdx,%r8

	movq    %r8,256(%rsp)
	movq    %r10,264(%rsp)
	movq    %r12,272(%rsp)
	movq    %r14,280(%rsp)
	
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

	// convert to 9x4 form
	vmovdqa   256(%rsp),%ymm8
	vmovdqa   288(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm1
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm2
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm3
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm4

	vpand     pmask1(%rip),%ymm1,%ymm10

	vpand     pmask2(%rip),%ymm1,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm1,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm2,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm2,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm2,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm3,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm5

	vpand     pmask8(%rip),%ymm3,%ymm6
	vpsrlq    $17,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm3,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm4,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm7

	vpand     pmask11(%rip),%ymm4,%ymm8
	vpsrlq    $11,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm4,%ymm9
	vpsrlq    $40,%ymm9,%ymm9

	vmovdqa   %ymm10,1248(%rsp)
	vmovdqa   %ymm11,1280(%rsp)
	vmovdqa   %ymm12,1312(%rsp)
	vmovdqa   %ymm13,1344(%rsp)
	vmovdqa   %ymm5,1376(%rsp)
	vmovdqa   %ymm6,1408(%rsp)
	vmovdqa   %ymm7,1440(%rsp)
	vmovdqa   %ymm8,1472(%rsp)
	vmovdqa   %ymm9,1504(%rsp)

	// convert to 9x4 form
	vmovdqa   352(%rsp),%ymm8
	vmovdqa   320(%rsp),%ymm9
	vmovdqa   352(%rsp),%ymm10
	vmovdqa   320(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm5
	vpunpckhqdq    %ymm9,%ymm8,%ymm6
	vpunpcklqdq    %ymm11,%ymm10,%ymm7
	vpunpckhqdq    %ymm11,%ymm10,%ymm8

	vpermq    $68,%ymm7,%ymm9
	vpblendd  $240,%ymm9,%ymm5,%ymm3
	vpermq    $68,%ymm8,%ymm9
	vpblendd  $240,%ymm9,%ymm6,%ymm4
	vpermq    $238,%ymm5,%ymm9
	vpblendd  $240,%ymm7,%ymm9,%ymm5
	vpermq    $238,%ymm6,%ymm9
	vpblendd  $240,%ymm8,%ymm9,%ymm6

	vpand     pmask1(%rip),%ymm3,%ymm10

	vpand     pmask2(%rip),%ymm3,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm3,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm4,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm4,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm4,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm5,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm0

	vpand     pmask8(%rip),%ymm5,%ymm1
	vpsrlq    $17,%ymm1,%ymm1

	vpand     pmask9(%rip),%ymm5,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm6,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm2

	vpand     pmask11(%rip),%ymm6,%ymm3
	vpsrlq    $11,%ymm3,%ymm3

	vpand     pmask12(%rip),%ymm6,%ymm4
	vpsrlq    $40,%ymm4,%ymm4

	vmovdqa   1376(%rsp),%ymm5
	vmovdqa   1408(%rsp),%ymm6
	vmovdqa   1440(%rsp),%ymm7
	vmovdqa   1472(%rsp),%ymm8
	vmovdqa   1504(%rsp),%ymm9

	// mul4x1
	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,480(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,512(%rsp)

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,544(%rsp)

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,576(%rsp)

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,608(%rsp)

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,640(%rsp)

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,672(%rsp)

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,704(%rsp)

	vpmuludq  %ymm9,%ymm4,%ymm15
	vmovdqa   %ymm15,736(%rsp)

	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm11,%ymm1,%ymm1
	vpaddq    %ymm12,%ymm2,%ymm2
	vpaddq    %ymm13,%ymm3,%ymm3
	vpaddq    1248(%rsp),%ymm5,%ymm5
	vpaddq    1280(%rsp),%ymm6,%ymm6
	vpaddq    1312(%rsp),%ymm7,%ymm7
	vpaddq    1344(%rsp),%ymm8,%ymm8

	vpmuludq  1248(%rsp),%ymm10,%ymm15
	vmovdqa   %ymm15,768(%rsp)
	vpaddq    480(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,992(%rsp)

	vpmuludq  1280(%rsp),%ymm10,%ymm15
	vpmuludq  1248(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,800(%rsp)
	vpaddq    512(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1024(%rsp)

	vpmuludq  1312(%rsp),%ymm10,%ymm15
	vpmuludq  1280(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,832(%rsp)
	vpaddq    544(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1056(%rsp)

	vpmuludq  1344(%rsp),%ymm10,%ymm15
	vpmuludq  1312(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,864(%rsp)
	vpaddq    576(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1088(%rsp)

	vpmuludq  1344(%rsp),%ymm11,%ymm15
	vpmuludq  1312(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,896(%rsp)
	vpaddq    608(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1120(%rsp)

	vpmuludq  1344(%rsp),%ymm12,%ymm15
	vpmuludq  1312(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,928(%rsp)
	vpaddq    640(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1152(%rsp)

	vpmuludq  1344(%rsp),%ymm13,%ymm15
	vmovdqa   %ymm15,960(%rsp)
	vpaddq    672(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1184(%rsp)

	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,1216(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm10

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm11

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm12

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm13

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm0

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm1

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm2

	vpmuludq  %ymm9,%ymm4,%ymm3

	vmovdqa   1216(%rsp),%ymm9

	vpsubq    992(%rsp),%ymm9,%ymm9
	vpaddq    896(%rsp),%ymm9,%ymm9
	vpsubq    1024(%rsp),%ymm10,%ymm10
	vpaddq    928(%rsp),%ymm10,%ymm10
	vpsubq    1056(%rsp),%ymm11,%ymm11
	vpaddq    960(%rsp),%ymm11,%ymm11
	vpsubq    1088(%rsp),%ymm12,%ymm12
	vpsubq    1120(%rsp),%ymm13,%ymm13
	vpaddq    480(%rsp),%ymm13,%ymm13
	vpsubq    1152(%rsp),%ymm0,%ymm0
	vpaddq    512(%rsp),%ymm0,%ymm0
	vpsubq    1184(%rsp),%ymm1,%ymm1
	vpaddq    544(%rsp),%ymm1,%ymm1
	vpsubq    704(%rsp),%ymm2,%ymm2
	vpaddq    576(%rsp),%ymm2,%ymm2
	vpsubq    736(%rsp),%ymm3,%ymm3
	vpaddq    608(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm0,%ymm14
	vpaddq    %ymm14,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0
	vpmuludq  vec1216(%rip),%ymm0,%ymm0
	vpaddq    768(%rsp),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1
	vpmuludq  vec1216(%rip),%ymm1,%ymm1
	vpaddq    800(%rsp),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2
	vpmuludq  vec1216(%rip),%ymm2,%ymm2
	vpaddq    832(%rsp),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm14
	vpaddq    640(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm3,%ymm3
	vpmuludq  vec1216(%rip),%ymm3,%ymm3
	vpaddq    864(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    672(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm4
	vpmuludq  vec1216(%rip),%ymm4,%ymm4
	vpaddq    %ymm9,%ymm4,%ymm4

	vpsrlq    $29,%ymm15,%ymm14
	vpaddq    704(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm15,%ymm5
	vpmuludq  vec1216(%rip),%ymm5,%ymm5
	vpaddq    %ymm10,%ymm5,%ymm5

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    736(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm6
	vpmuludq  vec1216(%rip),%ymm6,%ymm6
	vpaddq    %ymm11,%ymm6,%ymm6

	vpsrlq    $29,%ymm15,%ymm8
	vpand     vecmask29(%rip),%ymm15,%ymm7

	vpmuludq  vec1216(%rip),%ymm7,%ymm7
	vpaddq    %ymm12,%ymm7,%ymm7
	vpmuludq  vec1216(%rip),%ymm8,%ymm8
	vpaddq    %ymm13,%ymm8,%ymm8

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	vpsrlq    $23,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpaddq    %ymm15,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpsllq    $3,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpand     vecmask23(%rip),%ymm8,%ymm8

	vpsrlq    $29,%ymm0,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vpand     vecmask29(%rip),%ymm3,%ymm3

	vpsrlq    $29,%ymm4,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm5,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpand     vecmask29(%rip),%ymm5,%ymm5

	vpsrlq    $29,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7
	vpand     vecmask29(%rip),%ymm6,%ymm6

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm0,%ymm10
	vpand     upmask1(%rip),%ymm1,%ymm11
	vpsllq    $29,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10
	vpand     upmask2(%rip),%ymm2,%ymm11
	vpsllq    $58,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10

	vpand     upmask6(%rip),%ymm2,%ymm11
	vpsrlq    $6,%ymm11,%ymm11
	vpand     upmask1(%rip),%ymm3,%ymm12
	vpsllq    $23,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11
	vpand     upmask3(%rip),%ymm4,%ymm12
	vpsllq    $52,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11

	vpand     upmask7(%rip),%ymm4,%ymm12
	vpsrlq    $12,%ymm12,%ymm12
	vpand     upmask1(%rip),%ymm5,%ymm13
	vpsllq    $17,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12
	vpand     upmask4(%rip),%ymm6,%ymm13
	vpsllq    $46,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12

	vpand     upmask8(%rip),%ymm6,%ymm13
	vpsrlq    $18,%ymm13,%ymm13
	vpand     upmask1(%rip),%ymm7,%ymm14
	vpsllq    $11,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13
	vpand     upmask5(%rip),%ymm8,%ymm14
	vpsllq    $40,%ymm14,%ymm14
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

	vmovdqa   %ymm10,128(%rsp)
	vmovdqa   %ymm11,160(%rsp)
	vmovdqa   %ymm12,192(%rsp)
	vmovdqa   %ymm13,224(%rsp)

	movb	104(%rsp),%r14b
	shrb	$1,%r14b
	movzbq	%r14b,%r14
	imul	$128,%r14,%r14
	addq	%r14,%rdi
	
	/* pnielsadd p1p1 */
	
	movq	160(%rsp),%r8
	movq	168(%rsp),%r9
	movq	176(%rsp),%r10
	movq	184(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	128(%rsp),%r8
	sbbq 	136(%rsp),%r9
	sbbq 	144(%rsp),%r10
	sbbq 	152(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,384(%rsp)
	movq   %r9,392(%rsp)
	movq   %r10,400(%rsp)
	movq   %r11,408(%rsp)
	
	// add
	addq 	128(%rsp),%r12
	adcq 	136(%rsp),%r13
	adcq 	144(%rsp),%r14
	adcq 	152(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,416(%rsp)
	movq   %r13,424(%rsp)
	movq   %r14,432(%rsp)
	movq   %r15,440(%rsp)
	
	// mul
	movq    392(%rsp),%rax
	mulq    24(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    400(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    408(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    400(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    408(%rsp),%rax
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

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    392(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    400(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    384(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    392(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    392(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    400(%rsp),%rax
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

	movq    %r8,384(%rsp)
	movq    %r10,392(%rsp)
	movq    %r12,400(%rsp)
	movq    %r14,408(%rsp)

	// mul
	movq    424(%rsp),%rax
	mulq    56(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    432(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    440(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    432(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    440(%rsp),%rax
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

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    424(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    432(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    416(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    424(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    424(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    432(%rsp),%rax
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

	addq 	384(%rsp),%r8
	adcq 	392(%rsp),%r10
	adcq 	400(%rsp),%r12
	adcq 	408(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,320(%rsp)
	movq   %r10,328(%rsp)
	movq   %r12,336(%rsp)
	movq   %r14,344(%rsp)

	// sub
	subq 	384(%rsp),%r9
	sbbq 	392(%rsp),%r11
	sbbq 	400(%rsp),%r13
	sbbq 	408(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,256(%rsp)
	movq   %r11,264(%rsp)
	movq   %r13,272(%rsp)
	movq   %r15,280(%rsp)

	// mul	
	movq    232(%rsp),%rax
	mulq    120(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    240(%rsp),%rax
	mulq    112(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    248(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    240(%rsp),%rax
	mulq    120(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    248(%rsp),%rax
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

	movq    248(%rsp),%rax
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

	movq    224(%rsp),%rax
	mulq    120(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    232(%rsp),%rax
	mulq    112(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    240(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    248(%rsp),%rax
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

	movq    224(%rsp),%rax
	mulq    96(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    224(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    232(%rsp),%rax
	mulq    96(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    224(%rsp),%rax
	mulq    112(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    232(%rsp),%rax
	mulq    104(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    240(%rsp),%rax
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

	movq    %r8,384(%rsp)
	movq    %r10,392(%rsp)
	movq    %r12,400(%rsp)
	movq    %r14,408(%rsp)	

	// mul	
	movq    200(%rsp),%rax
	mulq    88(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    208(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    216(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    208(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    216(%rsp),%rax
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

	movq    216(%rsp),%rax
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

	movq    192(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    200(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    208(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    216(%rsp),%rax
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

	movq    192(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    192(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    200(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    192(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    200(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    208(%rsp),%rax
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

	addq 	384(%rsp),%r8
	adcq 	392(%rsp),%r10
	adcq 	400(%rsp),%r12
	adcq 	408(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,288(%rsp)
	movq   %r10,296(%rsp)
	movq   %r12,304(%rsp)
	movq   %r14,312(%rsp)

	// sub
	subq 	384(%rsp),%r9
	sbbq 	392(%rsp),%r11
	sbbq 	400(%rsp),%r13
	sbbq 	408(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,352(%rsp)
	movq   %r11,360(%rsp)
	movq   %r13,368(%rsp)
	movq   %r15,376(%rsp)
	
	jmp	.L6

.L5:	
	/* p1p1 to p3 */

	// convert to 9x4 form
	vmovdqa   256(%rsp),%ymm8
	vmovdqa   288(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm1
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm2
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm3
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm4

	vpand     pmask1(%rip),%ymm1,%ymm10

	vpand     pmask2(%rip),%ymm1,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm1,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm2,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm2,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm2,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm3,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm5

	vpand     pmask8(%rip),%ymm3,%ymm6
	vpsrlq    $17,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm3,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm4,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm7

	vpand     pmask11(%rip),%ymm4,%ymm8
	vpsrlq    $11,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm4,%ymm9
	vpsrlq    $40,%ymm9,%ymm9

	vmovdqa   %ymm10,1248(%rsp)
	vmovdqa   %ymm11,1280(%rsp)
	vmovdqa   %ymm12,1312(%rsp)
	vmovdqa   %ymm13,1344(%rsp)
	vmovdqa   %ymm5,1376(%rsp)
	vmovdqa   %ymm6,1408(%rsp)
	vmovdqa   %ymm7,1440(%rsp)
	vmovdqa   %ymm8,1472(%rsp)
	vmovdqa   %ymm9,1504(%rsp)

	// convert to 9x4 form
	vmovdqa   352(%rsp),%ymm8
	vmovdqa   320(%rsp),%ymm9
	vmovdqa   352(%rsp),%ymm10
	vmovdqa   320(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm5
	vpunpckhqdq    %ymm9,%ymm8,%ymm6
	vpunpcklqdq    %ymm11,%ymm10,%ymm7
	vpunpckhqdq    %ymm11,%ymm10,%ymm8

	vpermq    $68,%ymm7,%ymm9
	vpblendd  $240,%ymm9,%ymm5,%ymm3
	vpermq    $68,%ymm8,%ymm9
	vpblendd  $240,%ymm9,%ymm6,%ymm4
	vpermq    $238,%ymm5,%ymm9
	vpblendd  $240,%ymm7,%ymm9,%ymm5
	vpermq    $238,%ymm6,%ymm9
	vpblendd  $240,%ymm8,%ymm9,%ymm6

	vpand     pmask1(%rip),%ymm3,%ymm10

	vpand     pmask2(%rip),%ymm3,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm3,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm4,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm4,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm4,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm5,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm0

	vpand     pmask8(%rip),%ymm5,%ymm1
	vpsrlq    $17,%ymm1,%ymm1

	vpand     pmask9(%rip),%ymm5,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm6,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm2

	vpand     pmask11(%rip),%ymm6,%ymm3
	vpsrlq    $11,%ymm3,%ymm3

	vpand     pmask12(%rip),%ymm6,%ymm4
	vpsrlq    $40,%ymm4,%ymm4

	vmovdqa   1376(%rsp),%ymm5
	vmovdqa   1408(%rsp),%ymm6
	vmovdqa   1440(%rsp),%ymm7
	vmovdqa   1472(%rsp),%ymm8
	vmovdqa   1504(%rsp),%ymm9

	// mul4x1
	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,480(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,512(%rsp)

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,544(%rsp)

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,576(%rsp)

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,608(%rsp)

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,640(%rsp)

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,672(%rsp)

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,704(%rsp)

	vpmuludq  %ymm9,%ymm4,%ymm15
	vmovdqa   %ymm15,736(%rsp)

	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm11,%ymm1,%ymm1
	vpaddq    %ymm12,%ymm2,%ymm2
	vpaddq    %ymm13,%ymm3,%ymm3
	vpaddq    1248(%rsp),%ymm5,%ymm5
	vpaddq    1280(%rsp),%ymm6,%ymm6
	vpaddq    1312(%rsp),%ymm7,%ymm7
	vpaddq    1344(%rsp),%ymm8,%ymm8

	vpmuludq  1248(%rsp),%ymm10,%ymm15
	vmovdqa   %ymm15,768(%rsp)
	vpaddq    480(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,992(%rsp)

	vpmuludq  1280(%rsp),%ymm10,%ymm15
	vpmuludq  1248(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,800(%rsp)
	vpaddq    512(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1024(%rsp)

	vpmuludq  1312(%rsp),%ymm10,%ymm15
	vpmuludq  1280(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,832(%rsp)
	vpaddq    544(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1056(%rsp)

	vpmuludq  1344(%rsp),%ymm10,%ymm15
	vpmuludq  1312(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,864(%rsp)
	vpaddq    576(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1088(%rsp)

	vpmuludq  1344(%rsp),%ymm11,%ymm15
	vpmuludq  1312(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,896(%rsp)
	vpaddq    608(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1120(%rsp)

	vpmuludq  1344(%rsp),%ymm12,%ymm15
	vpmuludq  1312(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,928(%rsp)
	vpaddq    640(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1152(%rsp)

	vpmuludq  1344(%rsp),%ymm13,%ymm15
	vmovdqa   %ymm15,960(%rsp)
	vpaddq    672(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1184(%rsp)

	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,1216(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm10

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm11

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm12

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm13

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm0

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm1

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm2

	vpmuludq  %ymm9,%ymm4,%ymm3

	vmovdqa   1216(%rsp),%ymm9

	vpsubq    992(%rsp),%ymm9,%ymm9
	vpaddq    896(%rsp),%ymm9,%ymm9
	vpsubq    1024(%rsp),%ymm10,%ymm10
	vpaddq    928(%rsp),%ymm10,%ymm10
	vpsubq    1056(%rsp),%ymm11,%ymm11
	vpaddq    960(%rsp),%ymm11,%ymm11
	vpsubq    1088(%rsp),%ymm12,%ymm12
	vpsubq    1120(%rsp),%ymm13,%ymm13
	vpaddq    480(%rsp),%ymm13,%ymm13
	vpsubq    1152(%rsp),%ymm0,%ymm0
	vpaddq    512(%rsp),%ymm0,%ymm0
	vpsubq    1184(%rsp),%ymm1,%ymm1
	vpaddq    544(%rsp),%ymm1,%ymm1
	vpsubq    704(%rsp),%ymm2,%ymm2
	vpaddq    576(%rsp),%ymm2,%ymm2
	vpsubq    736(%rsp),%ymm3,%ymm3
	vpaddq    608(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm0,%ymm14
	vpaddq    %ymm14,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0
	vpmuludq  vec1216(%rip),%ymm0,%ymm0
	vpaddq    768(%rsp),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1
	vpmuludq  vec1216(%rip),%ymm1,%ymm1
	vpaddq    800(%rsp),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2
	vpmuludq  vec1216(%rip),%ymm2,%ymm2
	vpaddq    832(%rsp),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm14
	vpaddq    640(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm3,%ymm3
	vpmuludq  vec1216(%rip),%ymm3,%ymm3
	vpaddq    864(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    672(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm4
	vpmuludq  vec1216(%rip),%ymm4,%ymm4
	vpaddq    %ymm9,%ymm4,%ymm4

	vpsrlq    $29,%ymm15,%ymm14
	vpaddq    704(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm15,%ymm5
	vpmuludq  vec1216(%rip),%ymm5,%ymm5
	vpaddq    %ymm10,%ymm5,%ymm5

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    736(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm6
	vpmuludq  vec1216(%rip),%ymm6,%ymm6
	vpaddq    %ymm11,%ymm6,%ymm6

	vpsrlq    $29,%ymm15,%ymm8
	vpand     vecmask29(%rip),%ymm15,%ymm7

	vpmuludq  vec1216(%rip),%ymm7,%ymm7
	vpaddq    %ymm12,%ymm7,%ymm7
	vpmuludq  vec1216(%rip),%ymm8,%ymm8
	vpaddq    %ymm13,%ymm8,%ymm8

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	vpsrlq    $23,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpaddq    %ymm15,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpsllq    $3,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpand     vecmask23(%rip),%ymm8,%ymm8

	vpsrlq    $29,%ymm0,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vpand     vecmask29(%rip),%ymm3,%ymm3

	vpsrlq    $29,%ymm4,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm5,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpand     vecmask29(%rip),%ymm5,%ymm5

	vpsrlq    $29,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7
	vpand     vecmask29(%rip),%ymm6,%ymm6

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm0,%ymm10
	vpand     upmask1(%rip),%ymm1,%ymm11
	vpsllq    $29,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10
	vpand     upmask2(%rip),%ymm2,%ymm11
	vpsllq    $58,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10

	vpand     upmask6(%rip),%ymm2,%ymm11
	vpsrlq    $6,%ymm11,%ymm11
	vpand     upmask1(%rip),%ymm3,%ymm12
	vpsllq    $23,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11
	vpand     upmask3(%rip),%ymm4,%ymm12
	vpsllq    $52,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11

	vpand     upmask7(%rip),%ymm4,%ymm12
	vpsrlq    $12,%ymm12,%ymm12
	vpand     upmask1(%rip),%ymm5,%ymm13
	vpsllq    $17,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12
	vpand     upmask4(%rip),%ymm6,%ymm13
	vpsllq    $46,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12

	vpand     upmask8(%rip),%ymm6,%ymm13
	vpsrlq    $18,%ymm13,%ymm13
	vpand     upmask1(%rip),%ymm7,%ymm14
	vpsllq    $11,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13
	vpand     upmask5(%rip),%ymm8,%ymm14
	vpsllq    $40,%ymm14,%ymm14
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

	vmovdqa   %ymm10,128(%rsp)
	vmovdqa   %ymm11,160(%rsp)
	vmovdqa   %ymm12,192(%rsp)
	vmovdqa   %ymm13,224(%rsp)

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

	movq    %r8,448(%rsp)
	movq    %r9,456(%rsp)
	movq    %r10,464(%rsp)
	movq    %r11,472(%rsp)

	/* pnielsadd p1p1 */
	
	movq	160(%rsp),%r8
	movq	168(%rsp),%r9
	movq	176(%rsp),%r10
	movq	184(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	128(%rsp),%r8
	sbbq 	136(%rsp),%r9
	sbbq 	144(%rsp),%r10
	sbbq 	152(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,384(%rsp)
	movq   %r9,392(%rsp)
	movq   %r10,400(%rsp)
	movq   %r11,408(%rsp)
	
	// add
	addq 	128(%rsp),%r12
	adcq 	136(%rsp),%r13
	adcq 	144(%rsp),%r14
	adcq 	152(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,416(%rsp)
	movq   %r13,424(%rsp)
	movq   %r14,432(%rsp)
	movq   %r15,440(%rsp)
	
	// mul
	movq    392(%rsp),%rax
	mulq    56(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    400(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    408(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    400(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    408(%rsp),%rax
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

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    392(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    400(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    384(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    392(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    392(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    400(%rsp),%rax
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

	movq    %r8,384(%rsp)
	movq    %r10,392(%rsp)
	movq    %r12,400(%rsp)
	movq    %r14,408(%rsp)

	// mul
	movq    424(%rsp),%rax
	mulq    24(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    432(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    440(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    432(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    440(%rsp),%rax
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

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    424(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    432(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    416(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    424(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    424(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    432(%rsp),%rax
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

	// add
	movq 	%r8,%r9
	movq 	%r10,%r11
	movq 	%r12,%r13
	movq 	%r14,%r15

	addq 	384(%rsp),%r8
	adcq 	392(%rsp),%r10
	adcq 	400(%rsp),%r12
	adcq 	408(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,320(%rsp)
	movq   %r10,328(%rsp)
	movq   %r12,336(%rsp)
	movq   %r14,344(%rsp)

	// sub
	subq 	384(%rsp),%r9
	sbbq 	392(%rsp),%r11
	sbbq 	400(%rsp),%r13
	sbbq 	408(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,256(%rsp)
	movq   %r11,264(%rsp)
	movq   %r13,272(%rsp)
	movq   %r15,280(%rsp)

	// mul	
	movq    232(%rsp),%rax
	mulq    472(%rsp)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    240(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    248(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    240(%rsp),%rax
	mulq    472(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    248(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    248(%rsp),%rax
	mulq    472(%rsp)
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

	movq    224(%rsp),%rax
	mulq    472(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    232(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    240(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    248(%rsp),%rax
	mulq    448(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    224(%rsp),%rax
	mulq    448(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    224(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    232(%rsp),%rax
	mulq    448(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    224(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    232(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    240(%rsp),%rax
	mulq    448(%rsp)
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

	movq    %r8,384(%rsp)
	movq    %r10,392(%rsp)
	movq    %r12,400(%rsp)
	movq    %r14,408(%rsp)	

	// mul	
	movq    200(%rsp),%rax
	mulq    88(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    208(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    216(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    208(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    216(%rsp),%rax
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

	movq    216(%rsp),%rax
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

	movq    192(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    200(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    208(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    216(%rsp),%rax
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

	movq    192(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    192(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    200(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    192(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    200(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    208(%rsp),%rax
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

	addq 	384(%rsp),%r8
	adcq 	392(%rsp),%r10
	adcq 	400(%rsp),%r12
	adcq 	408(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,288(%rsp)
	movq   %r10,296(%rsp)
	movq   %r12,304(%rsp)
	movq   %r14,312(%rsp)

	// sub
	subq 	384(%rsp),%r9
	sbbq 	392(%rsp),%r11
	sbbq 	400(%rsp),%r13
	sbbq 	408(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,352(%rsp)
	movq   %r11,360(%rsp)
	movq   %r13,368(%rsp)
	movq   %r15,376(%rsp)
	
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

	// convert to 9x4 form
	vmovdqa   256(%rsp),%ymm8
	vmovdqa   288(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm1
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm2
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm3
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm4

	vpand     pmask1(%rip),%ymm1,%ymm10

	vpand     pmask2(%rip),%ymm1,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm1,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm2,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm2,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm2,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm3,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm5

	vpand     pmask8(%rip),%ymm3,%ymm6
	vpsrlq    $17,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm3,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm4,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm7

	vpand     pmask11(%rip),%ymm4,%ymm8
	vpsrlq    $11,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm4,%ymm9
	vpsrlq    $40,%ymm9,%ymm9

	vmovdqa   %ymm10,1248(%rsp)
	vmovdqa   %ymm11,1280(%rsp)
	vmovdqa   %ymm12,1312(%rsp)
	vmovdqa   %ymm13,1344(%rsp)
	vmovdqa   %ymm5,1376(%rsp)
	vmovdqa   %ymm6,1408(%rsp)
	vmovdqa   %ymm7,1440(%rsp)
	vmovdqa   %ymm8,1472(%rsp)
	vmovdqa   %ymm9,1504(%rsp)

	// convert to 9x4 form
	vmovdqa   352(%rsp),%ymm8
	vmovdqa   320(%rsp),%ymm9
	vmovdqa   352(%rsp),%ymm10
	vmovdqa   320(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm5
	vpunpckhqdq    %ymm9,%ymm8,%ymm6
	vpunpcklqdq    %ymm11,%ymm10,%ymm7
	vpunpckhqdq    %ymm11,%ymm10,%ymm8

	vpermq    $68,%ymm7,%ymm9
	vpblendd  $240,%ymm9,%ymm5,%ymm3
	vpermq    $68,%ymm8,%ymm9
	vpblendd  $240,%ymm9,%ymm6,%ymm4
	vpermq    $238,%ymm5,%ymm9
	vpblendd  $240,%ymm7,%ymm9,%ymm5
	vpermq    $238,%ymm6,%ymm9
	vpblendd  $240,%ymm8,%ymm9,%ymm6

	vpand     pmask1(%rip),%ymm3,%ymm10

	vpand     pmask2(%rip),%ymm3,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm3,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm4,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm4,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm4,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm5,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm0

	vpand     pmask8(%rip),%ymm5,%ymm1
	vpsrlq    $17,%ymm1,%ymm1

	vpand     pmask9(%rip),%ymm5,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm6,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm2

	vpand     pmask11(%rip),%ymm6,%ymm3
	vpsrlq    $11,%ymm3,%ymm3

	vpand     pmask12(%rip),%ymm6,%ymm4
	vpsrlq    $40,%ymm4,%ymm4

	vmovdqa   1376(%rsp),%ymm5
	vmovdqa   1408(%rsp),%ymm6
	vmovdqa   1440(%rsp),%ymm7
	vmovdqa   1472(%rsp),%ymm8
	vmovdqa   1504(%rsp),%ymm9

	// mul4x1
	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,480(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,512(%rsp)

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,544(%rsp)

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,576(%rsp)

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,608(%rsp)

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,640(%rsp)

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,672(%rsp)

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,704(%rsp)

	vpmuludq  %ymm9,%ymm4,%ymm15
	vmovdqa   %ymm15,736(%rsp)

	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm11,%ymm1,%ymm1
	vpaddq    %ymm12,%ymm2,%ymm2
	vpaddq    %ymm13,%ymm3,%ymm3
	vpaddq    1248(%rsp),%ymm5,%ymm5
	vpaddq    1280(%rsp),%ymm6,%ymm6
	vpaddq    1312(%rsp),%ymm7,%ymm7
	vpaddq    1344(%rsp),%ymm8,%ymm8

	vpmuludq  1248(%rsp),%ymm10,%ymm15
	vmovdqa   %ymm15,768(%rsp)
	vpaddq    480(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,992(%rsp)

	vpmuludq  1280(%rsp),%ymm10,%ymm15
	vpmuludq  1248(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,800(%rsp)
	vpaddq    512(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1024(%rsp)

	vpmuludq  1312(%rsp),%ymm10,%ymm15
	vpmuludq  1280(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,832(%rsp)
	vpaddq    544(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1056(%rsp)

	vpmuludq  1344(%rsp),%ymm10,%ymm15
	vpmuludq  1312(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,864(%rsp)
	vpaddq    576(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1088(%rsp)

	vpmuludq  1344(%rsp),%ymm11,%ymm15
	vpmuludq  1312(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,896(%rsp)
	vpaddq    608(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1120(%rsp)

	vpmuludq  1344(%rsp),%ymm12,%ymm15
	vpmuludq  1312(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,928(%rsp)
	vpaddq    640(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1152(%rsp)

	vpmuludq  1344(%rsp),%ymm13,%ymm15
	vmovdqa   %ymm15,960(%rsp)
	vpaddq    672(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1184(%rsp)

	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,1216(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm10

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm11

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm12

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm13

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm0

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm1

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm2

	vpmuludq  %ymm9,%ymm4,%ymm3

	vmovdqa   1216(%rsp),%ymm9

	vpsubq    992(%rsp),%ymm9,%ymm9
	vpaddq    896(%rsp),%ymm9,%ymm9
	vpsubq    1024(%rsp),%ymm10,%ymm10
	vpaddq    928(%rsp),%ymm10,%ymm10
	vpsubq    1056(%rsp),%ymm11,%ymm11
	vpaddq    960(%rsp),%ymm11,%ymm11
	vpsubq    1088(%rsp),%ymm12,%ymm12
	vpsubq    1120(%rsp),%ymm13,%ymm13
	vpaddq    480(%rsp),%ymm13,%ymm13
	vpsubq    1152(%rsp),%ymm0,%ymm0
	vpaddq    512(%rsp),%ymm0,%ymm0
	vpsubq    1184(%rsp),%ymm1,%ymm1
	vpaddq    544(%rsp),%ymm1,%ymm1
	vpsubq    704(%rsp),%ymm2,%ymm2
	vpaddq    576(%rsp),%ymm2,%ymm2
	vpsubq    736(%rsp),%ymm3,%ymm3
	vpaddq    608(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm0,%ymm14
	vpaddq    %ymm14,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0
	vpmuludq  vec1216(%rip),%ymm0,%ymm0
	vpaddq    768(%rsp),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1
	vpmuludq  vec1216(%rip),%ymm1,%ymm1
	vpaddq    800(%rsp),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2
	vpmuludq  vec1216(%rip),%ymm2,%ymm2
	vpaddq    832(%rsp),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm14
	vpaddq    640(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm3,%ymm3
	vpmuludq  vec1216(%rip),%ymm3,%ymm3
	vpaddq    864(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    672(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm4
	vpmuludq  vec1216(%rip),%ymm4,%ymm4
	vpaddq    %ymm9,%ymm4,%ymm4

	vpsrlq    $29,%ymm15,%ymm14
	vpaddq    704(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm15,%ymm5
	vpmuludq  vec1216(%rip),%ymm5,%ymm5
	vpaddq    %ymm10,%ymm5,%ymm5

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    736(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm6
	vpmuludq  vec1216(%rip),%ymm6,%ymm6
	vpaddq    %ymm11,%ymm6,%ymm6

	vpsrlq    $29,%ymm15,%ymm8
	vpand     vecmask29(%rip),%ymm15,%ymm7

	vpmuludq  vec1216(%rip),%ymm7,%ymm7
	vpaddq    %ymm12,%ymm7,%ymm7
	vpmuludq  vec1216(%rip),%ymm8,%ymm8
	vpaddq    %ymm13,%ymm8,%ymm8

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	vpsrlq    $23,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpaddq    %ymm15,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpsllq    $3,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpand     vecmask23(%rip),%ymm8,%ymm8

	vpsrlq    $29,%ymm0,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vpand     vecmask29(%rip),%ymm3,%ymm3

	vpsrlq    $29,%ymm4,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm5,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpand     vecmask29(%rip),%ymm5,%ymm5

	vpsrlq    $29,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7
	vpand     vecmask29(%rip),%ymm6,%ymm6

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm0,%ymm10
	vpand     upmask1(%rip),%ymm1,%ymm11
	vpsllq    $29,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10
	vpand     upmask2(%rip),%ymm2,%ymm11
	vpsllq    $58,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10

	vpand     upmask6(%rip),%ymm2,%ymm11
	vpsrlq    $6,%ymm11,%ymm11
	vpand     upmask1(%rip),%ymm3,%ymm12
	vpsllq    $23,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11
	vpand     upmask3(%rip),%ymm4,%ymm12
	vpsllq    $52,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11

	vpand     upmask7(%rip),%ymm4,%ymm12
	vpsrlq    $12,%ymm12,%ymm12
	vpand     upmask1(%rip),%ymm5,%ymm13
	vpsllq    $17,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12
	vpand     upmask4(%rip),%ymm6,%ymm13
	vpsllq    $46,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12

	vpand     upmask8(%rip),%ymm6,%ymm13
	vpsrlq    $18,%ymm13,%ymm13
	vpand     upmask1(%rip),%ymm7,%ymm14
	vpsllq    $11,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13
	vpand     upmask5(%rip),%ymm8,%ymm14
	vpsllq    $40,%ymm14,%ymm14
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

	vmovdqa   %ymm10,128(%rsp)
	vmovdqa   %ymm11,160(%rsp)
	vmovdqa   %ymm12,192(%rsp)
	vmovdqa   %ymm13,224(%rsp)

	movb	104(%rsp),%r14b
	shrb	$1,%r14b
	movzbq	%r14b,%r14
	imul	$96,%r14,%r14	
	addq	%r14,%rdi
	
	/* nielsadd p1p1 */
		
	movq	160(%rsp),%r8
	movq	168(%rsp),%r9
	movq	176(%rsp),%r10
	movq	184(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	128(%rsp),%r8
	sbbq 	136(%rsp),%r9
	sbbq 	144(%rsp),%r10
	sbbq 	152(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,384(%rsp)
	movq   %r9,392(%rsp)
	movq   %r10,400(%rsp)
	movq   %r11,408(%rsp)
	
	// add
	addq 	128(%rsp),%r12
	adcq 	136(%rsp),%r13
	adcq 	144(%rsp),%r14
	adcq 	152(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,416(%rsp)
	movq   %r13,424(%rsp)
	movq   %r14,432(%rsp)
	movq   %r15,440(%rsp)
	
	// mul
	movq    392(%rsp),%rax
	mulq    24(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    400(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    408(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    400(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    408(%rsp),%rax
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

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    392(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    400(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    384(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    392(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    392(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    400(%rsp),%rax
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

	movq    %r8,384(%rsp)
	movq    %r10,392(%rsp)
	movq    %r12,400(%rsp)
	movq    %r14,408(%rsp)

	// mul
	movq    424(%rsp),%rax
	mulq    56(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    432(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    440(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    432(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    440(%rsp),%rax
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

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    424(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    432(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    416(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    424(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    424(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    432(%rsp),%rax
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

	addq 	384(%rsp),%r8
	adcq 	392(%rsp),%r10
	adcq 	400(%rsp),%r12
	adcq 	408(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,320(%rsp)
	movq   %r10,328(%rsp)
	movq   %r12,336(%rsp)
	movq   %r14,344(%rsp)

	// sub
	subq 	384(%rsp),%r9
	sbbq 	392(%rsp),%r11
	sbbq 	400(%rsp),%r13
	sbbq 	408(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,256(%rsp)
	movq   %r11,264(%rsp)
	movq   %r13,272(%rsp)
	movq   %r15,280(%rsp)

	// mul	
	movq    232(%rsp),%rax
	mulq    88(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    240(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    248(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    240(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    248(%rsp),%rax
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

	movq    248(%rsp),%rax
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

	movq    224(%rsp),%rax
	mulq    88(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    232(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    240(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    248(%rsp),%rax
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

	movq    224(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    224(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    232(%rsp),%rax
	mulq    64(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    224(%rsp),%rax
	mulq    80(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    232(%rsp),%rax
	mulq    72(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    240(%rsp),%rax
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
	movq	192(%rsp),%r9
	movq	200(%rsp),%r11
	movq	208(%rsp),%r13
	movq	216(%rsp),%r15
	
	addq 	%r9,%r9
	adcq 	%r11,%r11
	adcq 	%r13,%r13
	adcq 	%r15,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r9
	adcq	%rdx,%r11
	adcq	%rdx,%r13
	adcq	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r9	
		
	// sub
	movq 	%r9,%rbx
	movq 	%r11,%rcx
	movq 	%r13,%rbp
	movq 	%r15,%rsi

	subq 	%r8,%r9
	sbbq 	%r10,%r11
	sbbq 	%r12,%r13
	sbbq 	%r14,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	sbbq	%rdx,%r9
	
	movq   %r9,352(%rsp)
	movq   %r11,360(%rsp)
	movq   %r13,368(%rsp)
	movq   %r15,376(%rsp)

	// add
	addq 	%rbx,%r8
	adcq 	%rcx,%r10
	adcq 	%rbp,%r12
	adcq 	%rsi,%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	adcq	%rdx,%r8

	movq   %r8,288(%rsp)
	movq   %r10,296(%rsp)
	movq   %r12,304(%rsp)
	movq   %r14,312(%rsp)

	jmp	.L9

.L8:	
	/* p1p1 to p3 */

	// convert to 9x4 form
	vmovdqa   256(%rsp),%ymm8
	vmovdqa   288(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm12
	vpunpckhqdq    %ymm9,%ymm8,%ymm13
	vpunpcklqdq    %ymm11,%ymm10,%ymm14
	vpunpckhqdq    %ymm11,%ymm10,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm1
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm2
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm3
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm4

	vpand     pmask1(%rip),%ymm1,%ymm10

	vpand     pmask2(%rip),%ymm1,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm1,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm2,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm2,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm2,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm3,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm5

	vpand     pmask8(%rip),%ymm3,%ymm6
	vpsrlq    $17,%ymm6,%ymm6

	vpand     pmask9(%rip),%ymm3,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm4,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm7

	vpand     pmask11(%rip),%ymm4,%ymm8
	vpsrlq    $11,%ymm8,%ymm8

	vpand     pmask12(%rip),%ymm4,%ymm9
	vpsrlq    $40,%ymm9,%ymm9

	vmovdqa   %ymm10,1248(%rsp)
	vmovdqa   %ymm11,1280(%rsp)
	vmovdqa   %ymm12,1312(%rsp)
	vmovdqa   %ymm13,1344(%rsp)
	vmovdqa   %ymm5,1376(%rsp)
	vmovdqa   %ymm6,1408(%rsp)
	vmovdqa   %ymm7,1440(%rsp)
	vmovdqa   %ymm8,1472(%rsp)
	vmovdqa   %ymm9,1504(%rsp)

	// convert to 9x4 form
	vmovdqa   352(%rsp),%ymm8
	vmovdqa   320(%rsp),%ymm9
	vmovdqa   352(%rsp),%ymm10
	vmovdqa   320(%rsp),%ymm11

	vpunpcklqdq    %ymm9,%ymm8,%ymm5
	vpunpckhqdq    %ymm9,%ymm8,%ymm6
	vpunpcklqdq    %ymm11,%ymm10,%ymm7
	vpunpckhqdq    %ymm11,%ymm10,%ymm8

	vpermq    $68,%ymm7,%ymm9
	vpblendd  $240,%ymm9,%ymm5,%ymm3
	vpermq    $68,%ymm8,%ymm9
	vpblendd  $240,%ymm9,%ymm6,%ymm4
	vpermq    $238,%ymm5,%ymm9
	vpblendd  $240,%ymm7,%ymm9,%ymm5
	vpermq    $238,%ymm6,%ymm9
	vpblendd  $240,%ymm8,%ymm9,%ymm6

	vpand     pmask1(%rip),%ymm3,%ymm10

	vpand     pmask2(%rip),%ymm3,%ymm11
	vpsrlq    $29,%ymm11,%ymm11

	vpand     pmask3(%rip),%ymm3,%ymm7
	vpsrlq    $58,%ymm7,%ymm7
	vpand     pmask4(%rip),%ymm4,%ymm9
	vpsllq    $6,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm12

	vpand     pmask5(%rip),%ymm4,%ymm13
	vpsrlq    $23,%ymm13,%ymm13

	vpand     pmask6(%rip),%ymm4,%ymm7
	vpsrlq    $52,%ymm7,%ymm7
	vpand     pmask7(%rip),%ymm5,%ymm9
	vpsllq    $12,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm0

	vpand     pmask8(%rip),%ymm5,%ymm1
	vpsrlq    $17,%ymm1,%ymm1

	vpand     pmask9(%rip),%ymm5,%ymm7
	vpsrlq    $46,%ymm7,%ymm7
	vpand     pmask10(%rip),%ymm6,%ymm9
	vpsllq    $18,%ymm9,%ymm9
	vpor      %ymm9,%ymm7,%ymm2

	vpand     pmask11(%rip),%ymm6,%ymm3
	vpsrlq    $11,%ymm3,%ymm3

	vpand     pmask12(%rip),%ymm6,%ymm4
	vpsrlq    $40,%ymm4,%ymm4

	vmovdqa   1376(%rsp),%ymm5
	vmovdqa   1408(%rsp),%ymm6
	vmovdqa   1440(%rsp),%ymm7
	vmovdqa   1472(%rsp),%ymm8
	vmovdqa   1504(%rsp),%ymm9

	// mul4x1
	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,480(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,512(%rsp)

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,544(%rsp)

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,576(%rsp)

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,608(%rsp)

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,640(%rsp)

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,672(%rsp)

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,704(%rsp)

	vpmuludq  %ymm9,%ymm4,%ymm15
	vmovdqa   %ymm15,736(%rsp)

	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm11,%ymm1,%ymm1
	vpaddq    %ymm12,%ymm2,%ymm2
	vpaddq    %ymm13,%ymm3,%ymm3
	vpaddq    1248(%rsp),%ymm5,%ymm5
	vpaddq    1280(%rsp),%ymm6,%ymm6
	vpaddq    1312(%rsp),%ymm7,%ymm7
	vpaddq    1344(%rsp),%ymm8,%ymm8

	vpmuludq  1248(%rsp),%ymm10,%ymm15
	vmovdqa   %ymm15,768(%rsp)
	vpaddq    480(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,992(%rsp)

	vpmuludq  1280(%rsp),%ymm10,%ymm15
	vpmuludq  1248(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,800(%rsp)
	vpaddq    512(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1024(%rsp)

	vpmuludq  1312(%rsp),%ymm10,%ymm15
	vpmuludq  1280(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,832(%rsp)
	vpaddq    544(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1056(%rsp)

	vpmuludq  1344(%rsp),%ymm10,%ymm15
	vpmuludq  1312(%rsp),%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1248(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,864(%rsp)
	vpaddq    576(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1088(%rsp)

	vpmuludq  1344(%rsp),%ymm11,%ymm15
	vpmuludq  1312(%rsp),%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  1280(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,896(%rsp)
	vpaddq    608(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1120(%rsp)

	vpmuludq  1344(%rsp),%ymm12,%ymm15
	vpmuludq  1312(%rsp),%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,928(%rsp)
	vpaddq    640(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1152(%rsp)

	vpmuludq  1344(%rsp),%ymm13,%ymm15
	vmovdqa   %ymm15,960(%rsp)
	vpaddq    672(%rsp),%ymm15,%ymm15
	vmovdqa   %ymm15,1184(%rsp)

	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,1216(%rsp)

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm10

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm11

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm12

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm13

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm0

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm1

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm2

	vpmuludq  %ymm9,%ymm4,%ymm3

	vmovdqa   1216(%rsp),%ymm9

	vpsubq    992(%rsp),%ymm9,%ymm9
	vpaddq    896(%rsp),%ymm9,%ymm9
	vpsubq    1024(%rsp),%ymm10,%ymm10
	vpaddq    928(%rsp),%ymm10,%ymm10
	vpsubq    1056(%rsp),%ymm11,%ymm11
	vpaddq    960(%rsp),%ymm11,%ymm11
	vpsubq    1088(%rsp),%ymm12,%ymm12
	vpsubq    1120(%rsp),%ymm13,%ymm13
	vpaddq    480(%rsp),%ymm13,%ymm13
	vpsubq    1152(%rsp),%ymm0,%ymm0
	vpaddq    512(%rsp),%ymm0,%ymm0
	vpsubq    1184(%rsp),%ymm1,%ymm1
	vpaddq    544(%rsp),%ymm1,%ymm1
	vpsubq    704(%rsp),%ymm2,%ymm2
	vpaddq    576(%rsp),%ymm2,%ymm2
	vpsubq    736(%rsp),%ymm3,%ymm3
	vpaddq    608(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm0,%ymm14
	vpaddq    %ymm14,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0
	vpmuludq  vec1216(%rip),%ymm0,%ymm0
	vpaddq    768(%rsp),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1
	vpmuludq  vec1216(%rip),%ymm1,%ymm1
	vpaddq    800(%rsp),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2
	vpmuludq  vec1216(%rip),%ymm2,%ymm2
	vpaddq    832(%rsp),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm14
	vpaddq    640(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm3,%ymm3
	vpmuludq  vec1216(%rip),%ymm3,%ymm3
	vpaddq    864(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    672(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm4
	vpmuludq  vec1216(%rip),%ymm4,%ymm4
	vpaddq    %ymm9,%ymm4,%ymm4

	vpsrlq    $29,%ymm15,%ymm14
	vpaddq    704(%rsp),%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm15,%ymm5
	vpmuludq  vec1216(%rip),%ymm5,%ymm5
	vpaddq    %ymm10,%ymm5,%ymm5

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    736(%rsp),%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm6
	vpmuludq  vec1216(%rip),%ymm6,%ymm6
	vpaddq    %ymm11,%ymm6,%ymm6

	vpsrlq    $29,%ymm15,%ymm8
	vpand     vecmask29(%rip),%ymm15,%ymm7

	vpmuludq  vec1216(%rip),%ymm7,%ymm7
	vpaddq    %ymm12,%ymm7,%ymm7
	vpmuludq  vec1216(%rip),%ymm8,%ymm8
	vpaddq    %ymm13,%ymm8,%ymm8

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	vpsrlq    $23,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpaddq    %ymm15,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpsllq    $3,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpand     vecmask23(%rip),%ymm8,%ymm8

	vpsrlq    $29,%ymm0,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vpand     vecmask29(%rip),%ymm3,%ymm3

	vpsrlq    $29,%ymm4,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm5,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpand     vecmask29(%rip),%ymm5,%ymm5

	vpsrlq    $29,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7
	vpand     vecmask29(%rip),%ymm6,%ymm6

	vpsrlq    $29,%ymm7,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm0,%ymm10
	vpand     upmask1(%rip),%ymm1,%ymm11
	vpsllq    $29,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10
	vpand     upmask2(%rip),%ymm2,%ymm11
	vpsllq    $58,%ymm11,%ymm11
	vpor      %ymm10,%ymm11,%ymm10

	vpand     upmask6(%rip),%ymm2,%ymm11
	vpsrlq    $6,%ymm11,%ymm11
	vpand     upmask1(%rip),%ymm3,%ymm12
	vpsllq    $23,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11
	vpand     upmask3(%rip),%ymm4,%ymm12
	vpsllq    $52,%ymm12,%ymm12
	vpor      %ymm11,%ymm12,%ymm11

	vpand     upmask7(%rip),%ymm4,%ymm12
	vpsrlq    $12,%ymm12,%ymm12
	vpand     upmask1(%rip),%ymm5,%ymm13
	vpsllq    $17,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12
	vpand     upmask4(%rip),%ymm6,%ymm13
	vpsllq    $46,%ymm13,%ymm13
	vpor      %ymm12,%ymm13,%ymm12

	vpand     upmask8(%rip),%ymm6,%ymm13
	vpsrlq    $18,%ymm13,%ymm13
	vpand     upmask1(%rip),%ymm7,%ymm14
	vpsllq    $11,%ymm14,%ymm14
	vpor      %ymm13,%ymm14,%ymm13
	vpand     upmask5(%rip),%ymm8,%ymm14
	vpsllq    $40,%ymm14,%ymm14
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

	vmovdqa   %ymm10,128(%rsp)
	vmovdqa   %ymm11,160(%rsp)
	vmovdqa   %ymm12,192(%rsp)
	vmovdqa   %ymm13,224(%rsp)

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

	movq    %r8,448(%rsp)
	movq    %r9,456(%rsp)
	movq    %r10,464(%rsp)
	movq    %r11,472(%rsp)

	/* nielsadd p1p1 */
	
	movq	160(%rsp),%r8
	movq	168(%rsp),%r9
	movq	176(%rsp),%r10
	movq	184(%rsp),%r11
	
	// copy
	movq	%r8,%r12
	movq	%r9,%r13
	movq	%r10,%r14
	movq	%r11,%r15			
	
	// sub
	subq 	128(%rsp),%r8
	sbbq 	136(%rsp),%r9
	sbbq 	144(%rsp),%r10
	sbbq 	152(%rsp),%r11
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r8
	sbbq	%rdx,%r9
	sbbq 	%rdx,%r10
	sbbq  	%rdx,%r11
	
	cmovc	%rax,%rdx
	subq	%rdx,%r8
	
	movq   %r8,384(%rsp)
	movq   %r9,392(%rsp)
	movq   %r10,400(%rsp)
	movq   %r11,408(%rsp)
	
	// add
	addq 	128(%rsp),%r12
	adcq 	136(%rsp),%r13
	adcq 	144(%rsp),%r14
	adcq 	152(%rsp),%r15
	
	movq 	$0,%rdx
	movq 	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r12
	adcq	%rdx,%r13
	adcq 	%rdx,%r14
	adcq  	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r12
	
	movq   %r12,416(%rsp)
	movq   %r13,424(%rsp)
	movq   %r14,432(%rsp)
	movq   %r15,440(%rsp)
	
	// mul
	movq    392(%rsp),%rax
	mulq    56(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    400(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    408(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    400(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    408(%rsp),%rax
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

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    56(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    392(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    400(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    408(%rsp),%rax
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

	movq    384(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    384(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    392(%rsp),%rax
	mulq    32(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    384(%rsp),%rax
	mulq    48(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    392(%rsp),%rax
	mulq    40(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    400(%rsp),%rax
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

	movq    %r8,384(%rsp)
	movq    %r10,392(%rsp)
	movq    %r12,400(%rsp)
	movq    %r14,408(%rsp)

	// mul
	movq    424(%rsp),%rax
	mulq    24(%rdi)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    432(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    440(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    432(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    440(%rsp),%rax
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

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    24(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    424(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    432(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    440(%rsp),%rax
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

	movq    416(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    416(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    424(%rsp),%rax
	mulq    0(%rdi)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    416(%rsp),%rax
	mulq    16(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    424(%rsp),%rax
	mulq    8(%rdi)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    432(%rsp),%rax
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

	// add
	movq 	%r8,%r9
	movq 	%r10,%r11
	movq 	%r12,%r13
	movq 	%r14,%r15

	addq 	384(%rsp),%r8
	adcq 	392(%rsp),%r10
	adcq 	400(%rsp),%r12
	adcq 	408(%rsp),%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,320(%rsp)
	movq   %r10,328(%rsp)
	movq   %r12,336(%rsp)
	movq   %r14,344(%rsp)

	// sub
	subq 	384(%rsp),%r9
	sbbq 	392(%rsp),%r11
	sbbq 	400(%rsp),%r13
	sbbq 	408(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r9

	movq   %r9,256(%rsp)
	movq   %r11,264(%rsp)
	movq   %r13,272(%rsp)
	movq   %r15,280(%rsp)

	// mul	
	movq    232(%rsp),%rax
	mulq    472(%rsp)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    240(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    248(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    240(%rsp),%rax
	mulq    472(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    248(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    248(%rsp),%rax
	mulq    472(%rsp)
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

	movq    224(%rsp),%rax
	mulq    472(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    232(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    240(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    248(%rsp),%rax
	mulq    448(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    224(%rsp),%rax
	mulq    448(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    224(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    232(%rsp),%rax
	mulq    448(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    224(%rsp),%rax
	mulq    464(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    232(%rsp),%rax
	mulq    456(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    240(%rsp),%rax
	mulq    448(%rsp)
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
	movq	192(%rsp),%r9
	movq	200(%rsp),%r11
	movq	208(%rsp),%r13
	movq	216(%rsp),%r15
	
	addq 	%r9,%r9
	adcq 	%r11,%r11
	adcq 	%r13,%r13
	adcq 	%r15,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r9
	adcq	%rdx,%r11
	adcq	%rdx,%r13
	adcq	%rdx,%r15
	
	cmovc	%rax,%rdx
	addq	%rdx,%r9	
		
	// sub
	movq 	%r9,%rbx
	movq 	%r11,%rcx
	movq 	%r13,%rbp
	movq 	%r15,%rsi

	subq 	%r8,%r9
	sbbq 	%r10,%r11
	sbbq 	%r12,%r13
	sbbq 	%r14,%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r9
	sbbq	%rdx,%r11
	sbbq	%rdx,%r13
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	sbbq	%rdx,%r9
	
	movq   %r9,352(%rsp)
	movq   %r11,360(%rsp)
	movq   %r13,368(%rsp)
	movq   %r15,376(%rsp)

	// add
	addq 	%rbx,%r8
	adcq 	%rcx,%r10
	adcq 	%rbp,%r12
	adcq 	%rsi,%r14
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r10
	adcq	%rdx,%r12
	adcq	%rdx,%r14
	
	cmovc	%rax,%rdx
	adcq	%rdx,%r8

	movq   %r8,288(%rsp)
	movq   %r10,296(%rsp)
	movq   %r12,304(%rsp)
	movq   %r14,312(%rsp)

.L9:
	movq	56(%rsp),%rdi	
	
	/* p1p1 to p2 */	
	
	// mul
	movq    264(%rsp),%rax
	mulq    376(%rsp)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    272(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    280(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    272(%rsp),%rax
	mulq    376(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    280(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    280(%rsp),%rax
	mulq    376(%rsp)
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

	movq    256(%rsp),%rax
	mulq    376(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    264(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    272(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    280(%rsp),%rax
	mulq    352(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    256(%rsp),%rax
	mulq    352(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    256(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    264(%rsp),%rax
	mulq    352(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    256(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    264(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    272(%rsp),%rax
	mulq    352(%rsp)
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

	movq    %r8,0(%rdi)
	movq    %r10,8(%rdi)
	movq    %r12,16(%rdi)
	movq    %r14,24(%rdi)

	// mul
	movq    296(%rsp),%rax
	mulq    344(%rsp)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    304(%rsp),%rax
	mulq    336(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    312(%rsp),%rax
	mulq    328(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    304(%rsp),%rax
	mulq    344(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    312(%rsp),%rax
	mulq    336(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    312(%rsp),%rax
	mulq    344(%rsp)
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

	movq    288(%rsp),%rax
	mulq    344(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    296(%rsp),%rax
	mulq    336(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    304(%rsp),%rax
	mulq    328(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    312(%rsp),%rax
	mulq    320(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    288(%rsp),%rax
	mulq    320(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    288(%rsp),%rax
	mulq    328(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    296(%rsp),%rax
	mulq    320(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rax
	mulq    336(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    296(%rsp),%rax
	mulq    328(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    304(%rsp),%rax
	mulq    320(%rsp)
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

	movq    %r8,32(%rdi)
	movq    %r10,40(%rdi)
	movq    %r12,48(%rdi)
	movq    %r14,56(%rdi)

	// mul
	movq    296(%rsp),%rax
	mulq    376(%rsp)
	movq    %rax,%r8
	xorq    %r9,%r9
	movq    %rdx,%r10
	xorq    %r11,%r11

	movq    304(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    312(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    304(%rsp),%rax
	mulq    376(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	movq    %rdx,%r12
	xorq    %r13,%r13

	movq    312(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    $38,%rax
	mulq    %r10
	imul    $38,%r11,%r11
	movq    %rax,%r10
	addq    %rdx,%r11

	movq    312(%rsp),%rax
	mulq    376(%rsp)
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

	movq    288(%rsp),%rax
	mulq    376(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    296(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    304(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    312(%rsp),%rax
	mulq    352(%rsp)
	addq    %rax,%r14
	adcq    $0,%r15
	addq    %rdx,%r8
	adcq    $0,%r9

	movq    $38,%rax
	mulq    %r8
	imul    $38,%r9,%r9
	movq    %rax,%r8
	addq    %rdx,%r9

	movq    288(%rsp),%rax
	mulq    352(%rsp)
	addq    %rax,%r8
	adcq    $0,%r9
	addq    %rdx,%r10
	adcq    $0,%r11

	movq    288(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    296(%rsp),%rax
	mulq    352(%rsp)
	addq    %rax,%r10
	adcq    $0,%r11
	addq    %rdx,%r12
	adcq    $0,%r13

	movq    288(%rsp),%rax
	mulq    368(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    296(%rsp),%rax
	mulq    360(%rsp)
	addq    %rax,%r12
	adcq    $0,%r13
	addq    %rdx,%r14
	adcq    $0,%r15

	movq    304(%rsp),%rax
	mulq    352(%rsp)
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

	movq    %r8,64(%rdi)
	movq    %r10,72(%rdi)
	movq    %r12,80(%rdi)
	movq    %r14,88(%rdi)
	
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
