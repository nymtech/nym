// linker define ge25519_double_scalarmult_precompute
// linker use upmask1 upmask2 upmask3 upmask4 upmask5 upmask6 upmask7 upmask8
// linker use pmask1 pmask2 pmask3 pmask4 pmask5 pmask6 pmask7 pmask8 pmask9 pmask10 pmask11 pmask12
// linker use EC2D0 EC2D1 EC2D2 EC2D3 mask63 vec1216 vecmask23 vecmask29

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

	movq    %r8,64(%rsp)
	movq    %r9,72(%rsp)
	movq    %r10,80(%rsp)
	movq    %r11,88(%rsp)
	
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
	
	movq    %r8,96(%rsp)
	movq    %r9,104(%rsp)
	movq    %r10,112(%rsp)
	movq    %r11,120(%rsp)
	
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
	
	movq    %r8,128(%rsp)
	movq    %r9,136(%rsp)
	movq    %r10,144(%rsp)
	movq    %r11,152(%rsp)

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
	movq    96(%rsp),%rdx
	    
	mulx    104(%rsp),%r9,%r10
	mulx    112(%rsp),%rcx,%r11
	addq    %rcx,%r10

	mulx    120(%rsp),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    104(%rsp),%rdx

	mulx    112(%rsp),%rax,%rbx
	mulx    120(%rsp),%rcx,%r13
	addq    %rcx,%rbx
	adcq    $0,%r13

	addq    %rax,%r11
	adcq    %rbx,%r12
	adcq    $0,%r13

	movq    112(%rsp),%rdx

	mulx    120(%rsp),%rax,%r14

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

	movq    96(%rsp),%rdx
	mulx    %rdx,%r8,%rax
	addq    %rax,%r9

	movq    104(%rsp),%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r10
	adcq    %rbx,%r11

	movq    112(%rsp),%rdx
	mulx    %rdx,%rax,%rbx
	adcq    %rax,%r12
	adcq    %rbx,%r13

	movq    120(%rsp),%rdx
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
	addq    64(%rsp),%r8
	adcq    72(%rsp),%r9
	adcq    80(%rsp),%r10
	adcq    88(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r8
	adcq    %rdx,%r9
	adcq    %rdx,%r10
	adcq    %rdx,%r11

	cmovc   %rax,%rdx
	addq    %rdx,%r8

	addq    160(%rsp),%r8
	adcq    168(%rsp),%r9
	adcq    176(%rsp),%r10
	adcq    184(%rsp),%r11

	movq    $0,%rdx
	movq    $38,%rax
	cmovae  %rdx,%rax

	addq    %rax,%r8
	adcq    %rdx,%r9
	adcq    %rdx,%r10
	adcq    %rdx,%r11

	cmovc   %rax,%rdx
	addq    %rdx,%r8

	movq    %r8,192(%rsp)
	movq    %r9,200(%rsp)
	movq    %r10,208(%rsp)
	movq    %r11,216(%rsp)
	
	/* p1p1 to p3 */

	// convert to 9x4 form
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
	vmovdqa   288(%rsp),%ymm8
	vmovdqa   256(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

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
	movq    EC2D0(%rip),%rdx

	mulx    96(%rdi),%r8,%r9
	mulx    104(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    112(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    120(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    EC2D1(%rip),%rdx    

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

	movq    EC2D2(%rip),%rdx

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

	movq    EC2D3(%rip),%rdx

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

	movq    %r8,96(%rdi)
	movq    %r9,104(%rdi)
	movq    %r10,112(%rdi)
	movq    %r11,120(%rdi)
	
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
	movq    320(%rsp),%rdx

	mulx    0(%rdi),%r8,%r9
	mulx    8(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    16(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    24(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    328(%rsp),%rdx    

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

	movq    336(%rsp),%rdx

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

	movq    344(%rsp),%rdx

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

	movq    %r8,320(%rsp)
	movq    %r9,328(%rsp)
	movq    %r10,336(%rsp)
	movq    %r11,344(%rsp)

	// mul
	movq    352(%rsp),%rdx

	mulx    32(%rdi),%r8,%r9
	mulx    40(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    48(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    56(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    360(%rsp),%rdx    

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

	movq    368(%rsp),%rdx

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

	movq    376(%rsp),%rdx

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

	addq 	320(%rsp),%r8
	adcq 	328(%rsp),%r9
	adcq 	336(%rsp),%r10
	adcq 	344(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,256(%rsp)
	movq   %r9,264(%rsp)
	movq   %r10,272(%rsp)
	movq   %r11,280(%rsp)

	// sub
	subq 	320(%rsp),%r12
	sbbq 	328(%rsp),%r13
	sbbq 	336(%rsp),%r14
	sbbq 	344(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,192(%rsp)
	movq   %r13,200(%rsp)
	movq   %r14,208(%rsp)
	movq   %r15,216(%rsp)

	// mul	
	movq    160(%rsp),%rdx

	mulx    96(%rdi),%r8,%r9
	mulx    104(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    112(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    120(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    168(%rsp),%rdx    

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

	movq    176(%rsp),%rdx

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

	movq    184(%rsp),%rdx

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

	movq    %r8,320(%rsp)
	movq    %r9,328(%rsp)
	movq    %r10,336(%rsp)
	movq    %r11,344(%rsp)
	
	// mul	
	movq    128(%rsp),%rdx

	mulx    64(%rdi),%r8,%r9
	mulx    72(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    80(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    88(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    136(%rsp),%rdx    

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

	movq    144(%rsp),%rdx

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

	movq    152(%rsp),%rdx

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

	addq 	320(%rsp),%r8
	adcq 	328(%rsp),%r9
	adcq 	336(%rsp),%r10
	adcq 	344(%rsp),%r11
	
	movq	$0,%rdx
	mov	$38,%rax	
	cmovae	%rdx,%rax
	
	addq	%rax,%r8
	adcq	%rdx,%r9
	adcq	%rdx,%r10
	adcq	%rdx,%r11
	
	cmovc	%rax,%rdx
	addq	%rdx,%r8
	
	movq   %r8,224(%rsp)
	movq   %r9,232(%rsp)
	movq   %r10,240(%rsp)
	movq   %r11,248(%rsp)

	// sub
	subq 	320(%rsp),%r12
	sbbq 	328(%rsp),%r13
	sbbq 	336(%rsp),%r14
	sbbq 	344(%rsp),%r15
	
	movq	$0,%rdx
	mov	$38,%rax
	cmovae	%rdx,%rax
	
	subq	%rax,%r12
	sbbq	%rdx,%r13
	sbbq	%rdx,%r14
	sbbq	%rdx,%r15
	
	cmovc	%rax,%rdx
	subq	%rdx,%r12

	movq   %r12,288(%rsp)
	movq   %r13,296(%rsp)
	movq   %r14,304(%rsp)
	movq   %r15,312(%rsp)
	
	/* p1p1 to p3 */

	// convert to 9x4 form
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
	vmovdqa   288(%rsp),%ymm8
	vmovdqa   256(%rsp),%ymm9
	vmovdqa   288(%rsp),%ymm10
	vmovdqa   256(%rsp),%ymm11

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
	movq    EC2D0(%rip),%rdx

	mulx    224(%rdi),%r8,%r9
	mulx    232(%rdi),%rcx,%r10
	addq    %rcx,%r9

	mulx    240(%rdi),%rcx,%r11
	adcq    %rcx,%r10

	mulx    248(%rdi),%rcx,%r12
	adcq    %rcx,%r11
	adcq    $0,%r12

	movq    EC2D1(%rip),%rdx    

	mulx    224(%rdi),%rax,%rbx
	mulx    232(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    240(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    248(%rdi),%rcx,%r13
	adcq    %rcx,%rsi
	adcq    $0,%r13

	addq    %rax,%r9
	adcq    %rbx,%r10
	adcq    %rbp,%r11
	adcq    %rsi,%r12
	adcq    $0,%r13

	movq    EC2D2(%rip),%rdx

	mulx    224(%rdi),%rax,%rbx
	mulx    232(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    240(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    248(%rdi),%rcx,%r14
	adcq    %rcx,%rsi
	adcq    $0,%r14

	addq    %rax,%r10
	adcq    %rbx,%r11
	adcq    %rbp,%r12
	adcq    %rsi,%r13
	adcq    $0,%r14

	movq    EC2D3(%rip),%rdx

	mulx    224(%rdi),%rax,%rbx
	mulx    232(%rdi),%rcx,%rbp
	addq    %rcx,%rbx

	mulx    240(%rdi),%rcx,%rsi
	adcq    %rcx,%rbp

	mulx    248(%rdi),%rcx,%r15
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

	movq    %r8,224(%rdi)
	movq    %r9,232(%rdi)
	movq    %r10,240(%rdi)
	movq    %r11,248(%rdi)

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
