// linker define ge25519_double_scalarmult_precompute
// linker use upmask1 upmask2 upmask3 upmask4 upmask5 upmask6 upmask7
// linker use pmask1 pmask2 pmask3 pmask4 pmask5 pmask6 pmask7 pmask8
// linker use EC2D0 EC2D1 EC2D2 EC2D3 mask63 vec19 vec608 vecmask47 vecmask52

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
	subq  	$416,%rsp

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
	xorq    %r13,%r13
	movq    0(%rdi),%rdx
	    
	mulx    8(%rdi),%r9,%r10

	mulx    16(%rdi),%rcx,%r11
	adcx    %rcx,%r10
	    
	mulx    24(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	movq    8(%rdi),%rdx
	xorq    %r14,%r14
	    
	mulx    16(%rdi),%rcx,%rdx
	adcx    %rcx,%r11
	adox    %rdx,%r12
	    
	movq    8(%rdi),%rdx
	mulx    24(%rdi),%rcx,%rdx
	adcx    %rcx,%r12
	adox    %rdx,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    16(%rdi),%rdx
	    
	mulx    24(%rdi),%rcx,%r14
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
	movq    0(%rdi),%rdx
	mulx    %rdx,%r8,%rdx
	adcx    %rdx,%r9

	movq    8(%rdi),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r10
	adcx    %rdx,%r11

	movq    16(%rdi),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r12
	adcx    %rdx,%r13

	movq    24(%rdi),%rdx
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

	movq    %r8,64(%rsp)
	movq    %r9,72(%rsp)
	movq    %r10,80(%rsp)
	movq    %r11,88(%rsp)
	
	// square
	xorq    %r13,%r13
	movq    32(%rdi),%rdx
	    
	mulx    40(%rdi),%r9,%r10

	mulx    48(%rdi),%rcx,%r11
	adcx    %rcx,%r10
	    
	mulx    56(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	movq    40(%rdi),%rdx
	xorq    %r14,%r14
	    
	mulx    48(%rdi),%rcx,%rdx
	adcx    %rcx,%r11
	adox    %rdx,%r12
	    
	movq    40(%rdi),%rdx
	mulx    56(%rdi),%rcx,%rdx
	adcx    %rcx,%r12
	adox    %rdx,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    48(%rdi),%rdx
	    
	mulx    56(%rdi),%rcx,%r14
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
	movq    32(%rdi),%rdx
	mulx    %rdx,%r8,%rdx
	adcx    %rdx,%r9

	movq    40(%rdi),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r10
	adcx    %rdx,%r11

	movq    48(%rdi),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r12
	adcx    %rdx,%r13

	movq    56(%rdi),%rdx
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
	
	movq    %r8,96(%rsp)
	movq    %r9,104(%rsp)
	movq    %r10,112(%rsp)
	movq    %r11,120(%rsp)
	
	// square
	xorq    %r13,%r13
	movq    64(%rdi),%rdx
	    
	mulx    72(%rdi),%r9,%r10

	mulx    80(%rdi),%rcx,%r11
	adcx    %rcx,%r10
	    
	mulx    88(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	movq    72(%rdi),%rdx
	xorq    %r14,%r14
	    
	mulx    80(%rdi),%rcx,%rdx
	adcx    %rcx,%r11
	adox    %rdx,%r12
	    
	movq    72(%rdi),%rdx
	mulx    88(%rdi),%rcx,%rdx
	adcx    %rcx,%r12
	adox    %rdx,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    80(%rdi),%rdx
	    
	mulx    88(%rdi),%rcx,%r14
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
	movq    64(%rdi),%rdx
	mulx    %rdx,%r8,%rdx
	adcx    %rdx,%r9

	movq    72(%rdi),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r10
	adcx    %rdx,%r11

	movq    80(%rdi),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r12
	adcx    %rdx,%r13

	movq    88(%rdi),%rdx
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
	xorq    %r13,%r13
	movq    96(%rsp),%rdx
	    
	mulx    104(%rsp),%r9,%r10

	mulx    112(%rsp),%rcx,%r11
	adcx    %rcx,%r10
	    
	mulx    120(%rsp),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	movq    104(%rsp),%rdx
	xorq    %r14,%r14
	    
	mulx    112(%rsp),%rcx,%rdx
	adcx    %rcx,%r11
	adox    %rdx,%r12
	    
	movq    104(%rsp),%rdx
	mulx    120(%rsp),%rcx,%rdx
	adcx    %rcx,%r12
	adox    %rdx,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    112(%rsp),%rdx
	    
	mulx    120(%rsp),%rcx,%r14
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
	movq    96(%rsp),%rdx
	mulx    %rdx,%r8,%rdx
	adcx    %rdx,%r9

	movq    104(%rsp),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r10
	adcx    %rdx,%r11

	movq    112(%rsp),%rdx
	mulx    %rdx,%rcx,%rdx
	adcx    %rcx,%r12
	adcx    %rdx,%r13

	movq    120(%rsp),%rdx
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

	// convert to 5x4 form
	vmovdqa   192(%rsp),%ymm8
	vmovdqa   224(%rsp),%ymm9
	vmovdqa   224(%rsp),%ymm10
	vmovdqa   192(%rsp),%ymm11


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

	vpand     pmask1(%rip),%ymm10,%ymm5

	vpand     pmask2(%rip),%ymm10,%ymm6
	vpsrlq    $52,%ymm6,%ymm6
	vpand     pmask3(%rip),%ymm11,%ymm7
	vpsllq    $12,%ymm7,%ymm7
	vpor      %ymm7,%ymm6,%ymm6

	vpand     pmask4(%rip),%ymm11,%ymm7
	vpsrlq    $40,%ymm7,%ymm7
	vpand     pmask5(%rip),%ymm12,%ymm8
	vpsllq    $24,%ymm8,%ymm8
	vpor      %ymm8,%ymm7,%ymm7

	vpand     pmask6(%rip),%ymm12,%ymm8
	vpsrlq    $28,%ymm8,%ymm8
	vpand     pmask7(%rip),%ymm13,%ymm9
	vpsllq    $36,%ymm9,%ymm9
	vpor      %ymm9,%ymm8,%ymm8

	vpand     pmask8(%rip),%ymm13,%ymm9
	vpsrlq    $16,%ymm9,%ymm9

	// convert to 5x4 form
	vmovdqa   288(%rsp),%ymm0
	vmovdqa   256(%rsp),%ymm1
	vmovdqa   288(%rsp),%ymm2
	vmovdqa   256(%rsp),%ymm3

	vpunpcklqdq    %ymm1,%ymm0,%ymm12
	vpunpckhqdq    %ymm1,%ymm0,%ymm13
	vpunpcklqdq    %ymm3,%ymm2,%ymm14
	vpunpckhqdq    %ymm3,%ymm2,%ymm15

	vpermq    $68,%ymm14,%ymm0
	vpblendd  $240,%ymm0,%ymm12,%ymm10
	vpermq    $68,%ymm15,%ymm0
	vpblendd  $240,%ymm0,%ymm13,%ymm11
	vpermq    $238,%ymm12,%ymm0
	vpblendd  $240,%ymm14,%ymm0,%ymm12
	vpermq    $238,%ymm13,%ymm0
	vpblendd  $240,%ymm15,%ymm0,%ymm13

	vpand     pmask1(%rip),%ymm10,%ymm0

	vpand     pmask2(%rip),%ymm10,%ymm1
	vpsrlq    $52,%ymm1,%ymm1
	vpand     pmask3(%rip),%ymm11,%ymm2
	vpsllq    $12,%ymm2,%ymm2
	vpor      %ymm2,%ymm1,%ymm1

	vpand     pmask4(%rip),%ymm11,%ymm2
	vpsrlq    $40,%ymm2,%ymm2
	vpand     pmask5(%rip),%ymm12,%ymm3
	vpsllq    $24,%ymm3,%ymm3
	vpor      %ymm3,%ymm2,%ymm2

	vpand     pmask6(%rip),%ymm12,%ymm3
	vpsrlq    $28,%ymm3,%ymm3
	vpand     pmask7(%rip),%ymm13,%ymm4
	vpsllq    $36,%ymm4,%ymm4
	vpor      %ymm4,%ymm3,%ymm3

	vpand     pmask8(%rip),%ymm13,%ymm4
	vpsrlq    $16,%ymm4,%ymm4

	// mul4x1
	vpxorq    %ymm10,%ymm10,%ymm10
	vpxorq    %ymm11,%ymm11,%ymm11
	vpxorq    %ymm12,%ymm12,%ymm12
	vpxorq    %ymm13,%ymm13,%ymm13
	vpxorq    %ymm14,%ymm14,%ymm14
	vpxorq    %ymm15,%ymm15,%ymm15
	vpxorq    %ymm16,%ymm16,%ymm16
	vpxorq    %ymm17,%ymm17,%ymm17
	vpxorq    %ymm18,%ymm18,%ymm18
	vpxorq    %ymm19,%ymm19,%ymm19

	vpxorq    %ymm25,%ymm25,%ymm25
	vpxorq    %ymm26,%ymm26,%ymm26
	vpxorq    %ymm27,%ymm27,%ymm27
	vpxorq    %ymm28,%ymm28,%ymm28
	vpxorq    %ymm29,%ymm29,%ymm29
	vpxorq    %ymm30,%ymm30,%ymm30
	vpxorq    %ymm31,%ymm31,%ymm31

	vpmadd52luq  %ymm0,%ymm5,%ymm10
	vpmadd52huq  %ymm0,%ymm5,%ymm11

	vpmadd52luq  %ymm0,%ymm6,%ymm25
	vpmadd52huq  %ymm0,%ymm6,%ymm12
	vpmadd52luq  %ymm1,%ymm5,%ymm25
	vpmadd52huq  %ymm1,%ymm5,%ymm12

	vpaddq    %ymm25,%ymm11,%ymm11

	vpmadd52luq  %ymm0,%ymm7,%ymm26
	vpmadd52huq  %ymm0,%ymm7,%ymm13
	vpmadd52luq  %ymm1,%ymm6,%ymm26
	vpmadd52huq  %ymm1,%ymm6,%ymm13
	vpmadd52luq  %ymm2,%ymm5,%ymm26
	vpmadd52huq  %ymm2,%ymm5,%ymm13

	vpaddq    %ymm26,%ymm12,%ymm12

	vpmadd52luq  %ymm0,%ymm8,%ymm27
	vpmadd52huq  %ymm0,%ymm8,%ymm14
	vpmadd52luq  %ymm1,%ymm7,%ymm27
	vpmadd52huq  %ymm1,%ymm7,%ymm14
	vpmadd52luq  %ymm2,%ymm6,%ymm27
	vpmadd52huq  %ymm2,%ymm6,%ymm14
	vpmadd52luq  %ymm3,%ymm5,%ymm27
	vpmadd52huq  %ymm3,%ymm5,%ymm14

	vpaddq    %ymm27,%ymm13,%ymm13

	vpmadd52luq  %ymm0,%ymm9,%ymm28
	vpmadd52huq  %ymm0,%ymm9,%ymm15
	vpmadd52luq  %ymm1,%ymm8,%ymm28
	vpmadd52huq  %ymm1,%ymm8,%ymm15
	vpmadd52luq  %ymm2,%ymm7,%ymm28
	vpmadd52huq  %ymm2,%ymm7,%ymm15
	vpmadd52luq  %ymm3,%ymm6,%ymm28
	vpmadd52huq  %ymm3,%ymm6,%ymm15
	vpmadd52luq  %ymm4,%ymm5,%ymm28
	vpmadd52huq  %ymm4,%ymm5,%ymm15

	vpaddq    %ymm28,%ymm14,%ymm14

	vpmadd52luq  %ymm1,%ymm9,%ymm29
	vpmadd52huq  %ymm1,%ymm9,%ymm16
	vpmadd52luq  %ymm2,%ymm8,%ymm29
	vpmadd52huq  %ymm2,%ymm8,%ymm16
	vpmadd52luq  %ymm3,%ymm7,%ymm29
	vpmadd52huq  %ymm3,%ymm7,%ymm16
	vpmadd52luq  %ymm4,%ymm6,%ymm29
	vpmadd52huq  %ymm4,%ymm6,%ymm16

	vpaddq    %ymm29,%ymm15,%ymm15

	vpmadd52luq  %ymm2,%ymm9,%ymm30
	vpmadd52huq  %ymm2,%ymm9,%ymm17
	vpmadd52luq  %ymm3,%ymm8,%ymm30
	vpmadd52huq  %ymm3,%ymm8,%ymm17
	vpmadd52luq  %ymm4,%ymm7,%ymm30
	vpmadd52huq  %ymm4,%ymm7,%ymm17

	vpaddq    %ymm30,%ymm16,%ymm16

	vpmadd52luq  %ymm3,%ymm9,%ymm31
	vpmadd52huq  %ymm3,%ymm9,%ymm18
	vpmadd52luq  %ymm4,%ymm8,%ymm31
	vpmadd52huq  %ymm4,%ymm8,%ymm18

	vpaddq    %ymm31,%ymm17,%ymm17

	vpmadd52luq  %ymm4,%ymm9,%ymm18
	vpmadd52huq  %ymm4,%ymm9,%ymm19

	vpsrlq    $52,%ymm15,%ymm22
	vpaddq    %ymm22,%ymm16,%ymm16
	vpandq    vecmask52(%rip),%ymm15,%ymm15
	vpmadd52luq  vec608(%rip),%ymm15,%ymm10
	vpmadd52huq  vec608(%rip),%ymm15,%ymm11

	vpsrlq    $52,%ymm16,%ymm22
	vpaddq    %ymm22,%ymm17,%ymm17
	vpandq    vecmask52(%rip),%ymm16,%ymm16
	vpmadd52luq  vec608(%rip),%ymm16,%ymm11
	vpmadd52huq  vec608(%rip),%ymm16,%ymm12

	vpsrlq    $52,%ymm17,%ymm22
	vpaddq    %ymm22,%ymm18,%ymm18
	vpandq    vecmask52(%rip),%ymm17,%ymm17
	vpmadd52luq  vec608(%rip),%ymm17,%ymm12
	vpmadd52huq  vec608(%rip),%ymm17,%ymm13

	vpsrlq    $52,%ymm18,%ymm22
	vpaddq    %ymm22,%ymm19,%ymm19
	vpandq    vecmask52(%rip),%ymm18,%ymm18
	vpmadd52luq  vec608(%rip),%ymm18,%ymm13
	vpmadd52huq  vec608(%rip),%ymm18,%ymm14

	vpxorq    %ymm15,%ymm15,%ymm15
	vpmadd52luq  vec608(%rip),%ymm19,%ymm14
	vpmadd52huq  vec608(%rip),%ymm19,%ymm15

	vpmadd52luq  vec608(%rip),%ymm15,%ymm10

	vpsrlq    $52,%ymm13,%ymm22
	vpaddq    %ymm22,%ymm14,%ymm14
	vpandq    vecmask52(%rip),%ymm13,%ymm13

	vpsrlq    $47,%ymm14,%ymm22
	vpandq    vecmask47(%rip),%ymm14,%ymm14
	vpmadd52luq  vec19(%rip),%ymm22,%ymm10

	vpsrlq    $52,%ymm10,%ymm22
	vpaddq    %ymm22,%ymm11,%ymm11
	vpandq    vecmask52(%rip),%ymm10,%ymm10

	vpsrlq    $52,%ymm11,%ymm22
	vpaddq    %ymm22,%ymm12,%ymm12
	vpandq    vecmask52(%rip),%ymm11,%ymm11

	vpsrlq    $52,%ymm12,%ymm22
	vpaddq    %ymm22,%ymm13,%ymm13
	vpandq    vecmask52(%rip),%ymm12,%ymm12

	vpsrlq    $52,%ymm13,%ymm22
	vpaddq    %ymm22,%ymm14,%ymm14
	vpandq    vecmask52(%rip),%ymm13,%ymm13

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm10,%ymm0
	vpand     upmask2(%rip),%ymm11,%ymm1
	vpsllq    $52,%ymm1,%ymm1
	vpor      %ymm1,%ymm0,%ymm0

	vpand     upmask3(%rip),%ymm11,%ymm1
	vpsrlq    $12,%ymm1,%ymm1
	vpand     upmask4(%rip),%ymm12,%ymm2
	vpsllq    $40,%ymm2,%ymm2
	vpor      %ymm2,%ymm1,%ymm1

	vpand     upmask5(%rip),%ymm12,%ymm2
	vpsrlq    $24,%ymm2,%ymm2
	vpand     upmask6(%rip),%ymm13,%ymm3
	vpsllq    $28,%ymm3,%ymm3
	vpor      %ymm3,%ymm2,%ymm2

	vpand     upmask7(%rip),%ymm13,%ymm3
	vpsrlq    $36,%ymm3,%ymm3
	vpand     upmask1(%rip),%ymm14,%ymm4
	vpsllq    $16,%ymm4,%ymm4
	vpor      %ymm4,%ymm3,%ymm3

	vpunpcklqdq    %ymm1,%ymm0,%ymm12
	vpunpckhqdq    %ymm1,%ymm0,%ymm13
	vpunpcklqdq    %ymm3,%ymm2,%ymm14
	vpunpckhqdq    %ymm3,%ymm2,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm0
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm1
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm2
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm3

	vmovdqa   %ymm0,64(%rsp)
	vmovdqa   %ymm1,96(%rsp)
	vmovdqa   %ymm2,128(%rsp)
	vmovdqa   %ymm3,160(%rsp)
	
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
	xorq    %r13,%r13
	movq    EC2D0(%rip),%rdx    

	mulx    96(%rdi),%r8,%r9
	mulx    104(%rdi),%rcx,%r10
	adcx    %rcx,%r9     

	mulx    112(%rdi),%rcx,%r11
	adcx    %rcx,%r10    

	mulx    120(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	xorq    %r14,%r14
	movq    EC2D1(%rip),%rdx
	   
	mulx    96(%rdi),%rcx,%rbp
	adcx    %rcx,%r9
	adox    %rbp,%r10
	    
	mulx    104(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    112(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    120(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    EC2D2(%rip),%rdx
	    
	mulx    96(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    104(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    112(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    120(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	adcx    %r15,%r14

	xorq    %rax,%rax
	movq    EC2D3(%rip),%rdx
	    
	mulx    96(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    104(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    112(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	    
	mulx    120(%rdi),%rcx,%rbp
	adcx    %rcx,%r14
	adox    %rbp,%r15			
	adcx    %rax,%r15

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
	andq    mask63(%rip),%r11

	imul    $19,%r15,%r15
	addq    %r15,%r8
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
	xorq    %r13,%r13
	movq    320(%rsp),%rdx    

	mulx    0(%rdi),%r8,%r9
	mulx    8(%rdi),%rcx,%r10
	adcx    %rcx,%r9     

	mulx    16(%rdi),%rcx,%r11
	adcx    %rcx,%r10    

	mulx    24(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	xorq    %r14,%r14
	movq    328(%rsp),%rdx
	   
	mulx    0(%rdi),%rcx,%rbp
	adcx    %rcx,%r9
	adox    %rbp,%r10
	    
	mulx    8(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    16(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    24(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    336(%rsp),%rdx
	    
	mulx    0(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    8(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    16(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    24(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	adcx    %r15,%r14

	xorq    %rax,%rax
	movq    344(%rsp),%rdx
	    
	mulx    0(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    8(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    16(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	    
	mulx    24(%rdi),%rcx,%rbp
	adcx    %rcx,%r14
	adox    %rbp,%r15			
	adcx    %rax,%r15

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
	andq    mask63(%rip),%r11

	imul    $19,%r15,%r15
	addq    %r15,%r8
	adcq    $0,%r9
	adcq    $0,%r10
	adcq    $0,%r11

	movq    %r8,320(%rsp)
	movq    %r9,328(%rsp)
	movq    %r10,336(%rsp)
	movq    %r11,344(%rsp)

	// mul
	xorq    %r13,%r13
	movq    352(%rsp),%rdx    

	mulx    32(%rdi),%r8,%r9
	mulx    40(%rdi),%rcx,%r10
	adcx    %rcx,%r9     

	mulx    48(%rdi),%rcx,%r11
	adcx    %rcx,%r10    

	mulx    56(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	xorq    %r14,%r14
	movq    360(%rsp),%rdx
	   
	mulx    32(%rdi),%rcx,%rbp
	adcx    %rcx,%r9
	adox    %rbp,%r10
	    
	mulx    40(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    48(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    56(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    368(%rsp),%rdx
	    
	mulx    32(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    40(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    48(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    56(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	adcx    %r15,%r14

	xorq    %rax,%rax
	movq    376(%rsp),%rdx
	    
	mulx    32(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    40(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    48(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	    
	mulx    56(%rdi),%rcx,%rbp
	adcx    %rcx,%r14
	adox    %rbp,%r15			
	adcx    %rax,%r15

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
	andq    mask63(%rip),%r11

	imul    $19,%r15,%r15
	addq    %r15,%r8
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
	xorq    %r13,%r13
	movq    160(%rsp),%rdx    

	mulx    96(%rdi),%r8,%r9
	mulx    104(%rdi),%rcx,%r10
	adcx    %rcx,%r9     

	mulx    112(%rdi),%rcx,%r11
	adcx    %rcx,%r10    

	mulx    120(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	xorq    %r14,%r14
	movq    168(%rsp),%rdx
	   
	mulx    96(%rdi),%rcx,%rbp
	adcx    %rcx,%r9
	adox    %rbp,%r10
	    
	mulx    104(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    112(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    120(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    176(%rsp),%rdx
	    
	mulx    96(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    104(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    112(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    120(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	adcx    %r15,%r14

	xorq    %rax,%rax
	movq    184(%rsp),%rdx
	    
	mulx    96(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    104(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    112(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	    
	mulx    120(%rdi),%rcx,%rbp
	adcx    %rcx,%r14
	adox    %rbp,%r15			
	adcx    %rax,%r15

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
	andq    mask63(%rip),%r11

	imul    $19,%r15,%r15
	addq    %r15,%r8
	adcq    $0,%r9
	adcq    $0,%r10
	adcq    $0,%r11

	movq    %r8,320(%rsp)
	movq    %r9,328(%rsp)
	movq    %r10,336(%rsp)
	movq    %r11,344(%rsp)
	
	// mul	
	xorq    %r13,%r13
	movq    128(%rsp),%rdx    

	mulx    64(%rdi),%r8,%r9
	mulx    72(%rdi),%rcx,%r10
	adcx    %rcx,%r9     

	mulx    80(%rdi),%rcx,%r11
	adcx    %rcx,%r10    

	mulx    88(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	xorq    %r14,%r14
	movq    136(%rsp),%rdx
	   
	mulx    64(%rdi),%rcx,%rbp
	adcx    %rcx,%r9
	adox    %rbp,%r10
	    
	mulx    72(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    80(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    88(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    144(%rsp),%rdx
	    
	mulx    64(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    72(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    80(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    88(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	adcx    %r15,%r14

	xorq    %rax,%rax
	movq    152(%rsp),%rdx
	    
	mulx    64(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    72(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    80(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	    
	mulx    88(%rdi),%rcx,%rbp
	adcx    %rcx,%r14
	adox    %rbp,%r15			
	adcx    %rax,%r15

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
	andq    mask63(%rip),%r11

	imul    $19,%r15,%r15
	addq    %r15,%r8
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

	// convert to 5x4 form
	vmovdqa   192(%rsp),%ymm8
	vmovdqa   224(%rsp),%ymm9
	vmovdqa   224(%rsp),%ymm10
	vmovdqa   192(%rsp),%ymm11


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

	vpand     pmask1(%rip),%ymm10,%ymm5

	vpand     pmask2(%rip),%ymm10,%ymm6
	vpsrlq    $52,%ymm6,%ymm6
	vpand     pmask3(%rip),%ymm11,%ymm7
	vpsllq    $12,%ymm7,%ymm7
	vpor      %ymm7,%ymm6,%ymm6

	vpand     pmask4(%rip),%ymm11,%ymm7
	vpsrlq    $40,%ymm7,%ymm7
	vpand     pmask5(%rip),%ymm12,%ymm8
	vpsllq    $24,%ymm8,%ymm8
	vpor      %ymm8,%ymm7,%ymm7

	vpand     pmask6(%rip),%ymm12,%ymm8
	vpsrlq    $28,%ymm8,%ymm8
	vpand     pmask7(%rip),%ymm13,%ymm9
	vpsllq    $36,%ymm9,%ymm9
	vpor      %ymm9,%ymm8,%ymm8

	vpand     pmask8(%rip),%ymm13,%ymm9
	vpsrlq    $16,%ymm9,%ymm9

	// convert to 5x4 form
	vmovdqa   288(%rsp),%ymm0
	vmovdqa   256(%rsp),%ymm1
	vmovdqa   288(%rsp),%ymm2
	vmovdqa   256(%rsp),%ymm3

	vpunpcklqdq    %ymm1,%ymm0,%ymm12
	vpunpckhqdq    %ymm1,%ymm0,%ymm13
	vpunpcklqdq    %ymm3,%ymm2,%ymm14
	vpunpckhqdq    %ymm3,%ymm2,%ymm15

	vpermq    $68,%ymm14,%ymm0
	vpblendd  $240,%ymm0,%ymm12,%ymm10
	vpermq    $68,%ymm15,%ymm0
	vpblendd  $240,%ymm0,%ymm13,%ymm11
	vpermq    $238,%ymm12,%ymm0
	vpblendd  $240,%ymm14,%ymm0,%ymm12
	vpermq    $238,%ymm13,%ymm0
	vpblendd  $240,%ymm15,%ymm0,%ymm13

	vpand     pmask1(%rip),%ymm10,%ymm0

	vpand     pmask2(%rip),%ymm10,%ymm1
	vpsrlq    $52,%ymm1,%ymm1
	vpand     pmask3(%rip),%ymm11,%ymm2
	vpsllq    $12,%ymm2,%ymm2
	vpor      %ymm2,%ymm1,%ymm1

	vpand     pmask4(%rip),%ymm11,%ymm2
	vpsrlq    $40,%ymm2,%ymm2
	vpand     pmask5(%rip),%ymm12,%ymm3
	vpsllq    $24,%ymm3,%ymm3
	vpor      %ymm3,%ymm2,%ymm2

	vpand     pmask6(%rip),%ymm12,%ymm3
	vpsrlq    $28,%ymm3,%ymm3
	vpand     pmask7(%rip),%ymm13,%ymm4
	vpsllq    $36,%ymm4,%ymm4
	vpor      %ymm4,%ymm3,%ymm3

	vpand     pmask8(%rip),%ymm13,%ymm4
	vpsrlq    $16,%ymm4,%ymm4

	// mul4x1
	vpxorq    %ymm10,%ymm10,%ymm10
	vpxorq    %ymm11,%ymm11,%ymm11
	vpxorq    %ymm12,%ymm12,%ymm12
	vpxorq    %ymm13,%ymm13,%ymm13
	vpxorq    %ymm14,%ymm14,%ymm14
	vpxorq    %ymm15,%ymm15,%ymm15
	vpxorq    %ymm16,%ymm16,%ymm16
	vpxorq    %ymm17,%ymm17,%ymm17
	vpxorq    %ymm18,%ymm18,%ymm18
	vpxorq    %ymm19,%ymm19,%ymm19

	vpxorq    %ymm25,%ymm25,%ymm25
	vpxorq    %ymm26,%ymm26,%ymm26
	vpxorq    %ymm27,%ymm27,%ymm27
	vpxorq    %ymm28,%ymm28,%ymm28
	vpxorq    %ymm29,%ymm29,%ymm29
	vpxorq    %ymm30,%ymm30,%ymm30
	vpxorq    %ymm31,%ymm31,%ymm31

	vpmadd52luq  %ymm0,%ymm5,%ymm10
	vpmadd52huq  %ymm0,%ymm5,%ymm11

	vpmadd52luq  %ymm0,%ymm6,%ymm25
	vpmadd52huq  %ymm0,%ymm6,%ymm12
	vpmadd52luq  %ymm1,%ymm5,%ymm25
	vpmadd52huq  %ymm1,%ymm5,%ymm12

	vpaddq    %ymm25,%ymm11,%ymm11

	vpmadd52luq  %ymm0,%ymm7,%ymm26
	vpmadd52huq  %ymm0,%ymm7,%ymm13
	vpmadd52luq  %ymm1,%ymm6,%ymm26
	vpmadd52huq  %ymm1,%ymm6,%ymm13
	vpmadd52luq  %ymm2,%ymm5,%ymm26
	vpmadd52huq  %ymm2,%ymm5,%ymm13

	vpaddq    %ymm26,%ymm12,%ymm12

	vpmadd52luq  %ymm0,%ymm8,%ymm27
	vpmadd52huq  %ymm0,%ymm8,%ymm14
	vpmadd52luq  %ymm1,%ymm7,%ymm27
	vpmadd52huq  %ymm1,%ymm7,%ymm14
	vpmadd52luq  %ymm2,%ymm6,%ymm27
	vpmadd52huq  %ymm2,%ymm6,%ymm14
	vpmadd52luq  %ymm3,%ymm5,%ymm27
	vpmadd52huq  %ymm3,%ymm5,%ymm14

	vpaddq    %ymm27,%ymm13,%ymm13

	vpmadd52luq  %ymm0,%ymm9,%ymm28
	vpmadd52huq  %ymm0,%ymm9,%ymm15
	vpmadd52luq  %ymm1,%ymm8,%ymm28
	vpmadd52huq  %ymm1,%ymm8,%ymm15
	vpmadd52luq  %ymm2,%ymm7,%ymm28
	vpmadd52huq  %ymm2,%ymm7,%ymm15
	vpmadd52luq  %ymm3,%ymm6,%ymm28
	vpmadd52huq  %ymm3,%ymm6,%ymm15
	vpmadd52luq  %ymm4,%ymm5,%ymm28
	vpmadd52huq  %ymm4,%ymm5,%ymm15

	vpaddq    %ymm28,%ymm14,%ymm14

	vpmadd52luq  %ymm1,%ymm9,%ymm29
	vpmadd52huq  %ymm1,%ymm9,%ymm16
	vpmadd52luq  %ymm2,%ymm8,%ymm29
	vpmadd52huq  %ymm2,%ymm8,%ymm16
	vpmadd52luq  %ymm3,%ymm7,%ymm29
	vpmadd52huq  %ymm3,%ymm7,%ymm16
	vpmadd52luq  %ymm4,%ymm6,%ymm29
	vpmadd52huq  %ymm4,%ymm6,%ymm16

	vpaddq    %ymm29,%ymm15,%ymm15

	vpmadd52luq  %ymm2,%ymm9,%ymm30
	vpmadd52huq  %ymm2,%ymm9,%ymm17
	vpmadd52luq  %ymm3,%ymm8,%ymm30
	vpmadd52huq  %ymm3,%ymm8,%ymm17
	vpmadd52luq  %ymm4,%ymm7,%ymm30
	vpmadd52huq  %ymm4,%ymm7,%ymm17

	vpaddq    %ymm30,%ymm16,%ymm16

	vpmadd52luq  %ymm3,%ymm9,%ymm31
	vpmadd52huq  %ymm3,%ymm9,%ymm18
	vpmadd52luq  %ymm4,%ymm8,%ymm31
	vpmadd52huq  %ymm4,%ymm8,%ymm18

	vpaddq    %ymm31,%ymm17,%ymm17

	vpmadd52luq  %ymm4,%ymm9,%ymm18
	vpmadd52huq  %ymm4,%ymm9,%ymm19

	vpsrlq    $52,%ymm15,%ymm22
	vpaddq    %ymm22,%ymm16,%ymm16
	vpandq    vecmask52(%rip),%ymm15,%ymm15
	vpmadd52luq  vec608(%rip),%ymm15,%ymm10
	vpmadd52huq  vec608(%rip),%ymm15,%ymm11

	vpsrlq    $52,%ymm16,%ymm22
	vpaddq    %ymm22,%ymm17,%ymm17
	vpandq    vecmask52(%rip),%ymm16,%ymm16
	vpmadd52luq  vec608(%rip),%ymm16,%ymm11
	vpmadd52huq  vec608(%rip),%ymm16,%ymm12

	vpsrlq    $52,%ymm17,%ymm22
	vpaddq    %ymm22,%ymm18,%ymm18
	vpandq    vecmask52(%rip),%ymm17,%ymm17
	vpmadd52luq  vec608(%rip),%ymm17,%ymm12
	vpmadd52huq  vec608(%rip),%ymm17,%ymm13

	vpsrlq    $52,%ymm18,%ymm22
	vpaddq    %ymm22,%ymm19,%ymm19
	vpandq    vecmask52(%rip),%ymm18,%ymm18
	vpmadd52luq  vec608(%rip),%ymm18,%ymm13
	vpmadd52huq  vec608(%rip),%ymm18,%ymm14

	vpxorq    %ymm15,%ymm15,%ymm15
	vpmadd52luq  vec608(%rip),%ymm19,%ymm14
	vpmadd52huq  vec608(%rip),%ymm19,%ymm15

	vpmadd52luq  vec608(%rip),%ymm15,%ymm10

	vpsrlq    $52,%ymm13,%ymm22
	vpaddq    %ymm22,%ymm14,%ymm14
	vpandq    vecmask52(%rip),%ymm13,%ymm13

	vpsrlq    $47,%ymm14,%ymm22
	vpandq    vecmask47(%rip),%ymm14,%ymm14
	vpmadd52luq  vec19(%rip),%ymm22,%ymm10

	vpsrlq    $52,%ymm10,%ymm22
	vpaddq    %ymm22,%ymm11,%ymm11
	vpandq    vecmask52(%rip),%ymm10,%ymm10

	vpsrlq    $52,%ymm11,%ymm22
	vpaddq    %ymm22,%ymm12,%ymm12
	vpandq    vecmask52(%rip),%ymm11,%ymm11

	vpsrlq    $52,%ymm12,%ymm22
	vpaddq    %ymm22,%ymm13,%ymm13
	vpandq    vecmask52(%rip),%ymm12,%ymm12

	vpsrlq    $52,%ymm13,%ymm22
	vpaddq    %ymm22,%ymm14,%ymm14
	vpandq    vecmask52(%rip),%ymm13,%ymm13

	// get back to 4x4 form
	vpand     upmask1(%rip),%ymm10,%ymm0
	vpand     upmask2(%rip),%ymm11,%ymm1
	vpsllq    $52,%ymm1,%ymm1
	vpor      %ymm1,%ymm0,%ymm0

	vpand     upmask3(%rip),%ymm11,%ymm1
	vpsrlq    $12,%ymm1,%ymm1
	vpand     upmask4(%rip),%ymm12,%ymm2
	vpsllq    $40,%ymm2,%ymm2
	vpor      %ymm2,%ymm1,%ymm1

	vpand     upmask5(%rip),%ymm12,%ymm2
	vpsrlq    $24,%ymm2,%ymm2
	vpand     upmask6(%rip),%ymm13,%ymm3
	vpsllq    $28,%ymm3,%ymm3
	vpor      %ymm3,%ymm2,%ymm2

	vpand     upmask7(%rip),%ymm13,%ymm3
	vpsrlq    $36,%ymm3,%ymm3
	vpand     upmask1(%rip),%ymm14,%ymm4
	vpsllq    $16,%ymm4,%ymm4
	vpor      %ymm4,%ymm3,%ymm3

	vpunpcklqdq    %ymm1,%ymm0,%ymm12
	vpunpckhqdq    %ymm1,%ymm0,%ymm13
	vpunpcklqdq    %ymm3,%ymm2,%ymm14
	vpunpckhqdq    %ymm3,%ymm2,%ymm15

	vpermq    $68,%ymm14,%ymm7
	vpblendd  $240,%ymm7,%ymm12,%ymm0
	vpermq    $68,%ymm15,%ymm7
	vpblendd  $240,%ymm7,%ymm13,%ymm1
	vpermq    $238,%ymm12,%ymm7
	vpblendd  $240,%ymm14,%ymm7,%ymm2
	vpermq    $238,%ymm13,%ymm7
	vpblendd  $240,%ymm15,%ymm7,%ymm3

	vmovdqa   %ymm0,128(%rdi)
	vmovdqa   %ymm1,160(%rdi)
	vmovdqa   %ymm2,192(%rdi)
	vmovdqa   %ymm3,224(%rdi)
	
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
	xorq    %r13,%r13
	movq    EC2D0(%rip),%rdx    

	mulx    224(%rdi),%r8,%r9
	mulx    232(%rdi),%rcx,%r10
	adcx    %rcx,%r9     

	mulx    240(%rdi),%rcx,%r11
	adcx    %rcx,%r10    

	mulx    248(%rdi),%rcx,%r12
	adcx    %rcx,%r11
	adcx    %r13,%r12

	xorq    %r14,%r14
	movq    EC2D1(%rip),%rdx
	   
	mulx    224(%rdi),%rcx,%rbp
	adcx    %rcx,%r9
	adox    %rbp,%r10
	    
	mulx    232(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    240(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    248(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	adcx    %r14,%r13

	xorq    %r15,%r15
	movq    EC2D2(%rip),%rdx
	    
	mulx    224(%rdi),%rcx,%rbp
	adcx    %rcx,%r10
	adox    %rbp,%r11
	    
	mulx    232(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    240(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    248(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	adcx    %r15,%r14

	xorq    %rax,%rax
	movq    EC2D3(%rip),%rdx
	    
	mulx    224(%rdi),%rcx,%rbp
	adcx    %rcx,%r11
	adox    %rbp,%r12
	    
	mulx    232(%rdi),%rcx,%rbp
	adcx    %rcx,%r12
	adox    %rbp,%r13
	    
	mulx    240(%rdi),%rcx,%rbp
	adcx    %rcx,%r13
	adox    %rbp,%r14
	    
	mulx    248(%rdi),%rcx,%rbp
	adcx    %rcx,%r14
	adox    %rbp,%r15			
	adcx    %rax,%r15

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
	andq    mask63(%rip),%r11

	imul    $19,%r15,%r15
	addq    %r15,%r8
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
