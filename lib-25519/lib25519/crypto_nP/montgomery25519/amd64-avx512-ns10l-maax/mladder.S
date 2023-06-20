// linker define mladder
// linker use hh1_p1
// linker use hh1_p2
// linker use h2h_p1
// linker use h2h_p2
// linker use hh1_xor
// linker use h2h_xor
// linker use swap_c
// linker use swap_mask
// linker use h2h_mask
// linker use vec19
// linker use vec38
// linker use vecmask25
// linker use vecmask26
// linker use vecmask32
// linker use a24

/* Assembly for Montgomery ladder. */

#include "consts_namespace.h"

	.p2align 5
	.globl _CRYPTO_SHARED_NAMESPACE(mladder)
	.globl CRYPTO_SHARED_NAMESPACE(mladder)
	
_CRYPTO_SHARED_NAMESPACE(mladder):
CRYPTO_SHARED_NAMESPACE(mladder):

	movq 	  %rsp,%r11
	andq 	  $-32,%rsp
	subq 	  $448,%rsp	

	movq 	  %r11,0(%rsp)
	movq 	  %r12,8(%rsp)
	movq 	  %r13,16(%rsp)
	movq 	  %r14,24(%rsp)
	movq 	  %r15,32(%rsp)
	movq 	  %rbx,40(%rsp)
	movq 	  %rbp,48(%rsp)

	// load <0,0,1,X1>
	vmovdqa   0(%rsi),%ymm0
	vmovdqa   32(%rsi),%ymm11
	vmovdqa   64(%rsi),%ymm2
	vmovdqa   96(%rsi),%ymm12
	vmovdqa   128(%rsi),%ymm4
	vmovdqa   160(%rsi),%ymm13
	vmovdqa   192(%rsi),%ymm6
	vmovdqa   224(%rsi),%ymm14
	vmovdqa   256(%rsi),%ymm8
	vmovdqa   288(%rsi),%ymm15

	// <0',0',1',X1'> ← Pack-D2N(<0,0,1,X1>)
	vpsllq    $32,%ymm11,%ymm11
	vpor      %ymm11,%ymm0,%ymm0
	vpsllq    $32,%ymm12,%ymm12
	vpor      %ymm12,%ymm2,%ymm2
	vpsllq    $32,%ymm13,%ymm13
	vpor      %ymm13,%ymm4,%ymm4
	vpsllq    $32,%ymm14,%ymm14
	vpor      %ymm14,%ymm6,%ymm6
	vpsllq    $32,%ymm15,%ymm15
	vpor      %ymm15,%ymm8,%ymm8

	vmovdqa   %ymm0,0(%rsi)
	vmovdqa   %ymm2,32(%rsi)
	vmovdqa   %ymm4,64(%rsi)
	vmovdqa   %ymm6,96(%rsi)
	vmovdqa   %ymm8,128(%rsi)

	// load <X2,Z2,X3,Z3>
	vmovdqa   0(%rdi),%ymm0
	vmovdqa   32(%rdi),%ymm11
	vmovdqa   64(%rdi),%ymm2
	vmovdqa   96(%rdi),%ymm12
	vmovdqa   128(%rdi),%ymm4
	vmovdqa   160(%rdi),%ymm13
	vmovdqa   192(%rdi),%ymm6
	vmovdqa   224(%rdi),%ymm14
	vmovdqa   256(%rdi),%ymm8
	vmovdqa   288(%rdi),%ymm15

	// <X2',Z2',X3',Z3'> ← Pack-D2N(<X2,Z2,X3,Z3>)
	vpsllq    $32,%ymm11,%ymm11
	vpor      %ymm11,%ymm0,%ymm0
	vpsllq    $32,%ymm12,%ymm12
	vpor      %ymm12,%ymm2,%ymm2
	vpsllq    $32,%ymm13,%ymm13
	vpor      %ymm13,%ymm4,%ymm4
	vpsllq    $32,%ymm14,%ymm14
	vpor      %ymm14,%ymm6,%ymm6
	vpsllq    $32,%ymm15,%ymm15
	vpor      %ymm15,%ymm8,%ymm8

	movq      $31,%r15
	movq	  $6,%rcx

	movb      $0,%r8b
	movq      %rdx,%rax

.L1:
	addq      %r15,%rax
	movb      0(%rax),%r14b
	movq      %rdx,%rax

.L2:
	movb	  %r14b,%bl
	shrb      %cl,%bl
	andb      $1,%bl
	movb      %bl,%r9b
	xorb      %r8b,%bl
	movb      %r9b,%r8b

	// <X2',Z2',X3',Z3'> ← Dense-Swap(<X2',Z2',X3',Z3'>,b)
	movzbl    %bl,%ebx
	imul	  $4,%ebx,%ebx
	movl      %ebx,56(%rsp)
	vpbroadcastd 56(%rsp),%ymm7
	vpaddd	  swap_c(%rip),%ymm7,%ymm7
	vpand     swap_mask(%rip),%ymm7,%ymm7

	vpermd	  %ymm0,%ymm7,%ymm0
	vpermd	  %ymm2,%ymm7,%ymm2
	vpermd	  %ymm4,%ymm7,%ymm4
	vpermd	  %ymm6,%ymm7,%ymm6
	vpermd	  %ymm8,%ymm7,%ymm8

	// <T1',T2',T4',T3'> ← Dense-H-H1(<X2',Z2',X3',Z3'>)
	vpshufd   $68,%ymm0,%ymm1
	vpshufd   $238,%ymm0,%ymm3
	vpaddd    hh1_p1(%rip),%ymm1,%ymm1
	vpxor     hh1_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm16

	vpshufd   $68,%ymm2,%ymm1
	vpshufd   $238,%ymm2,%ymm3
	vpaddd    hh1_p2(%rip),%ymm1,%ymm1
	vpxor     hh1_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm18

	vpshufd   $68,%ymm4,%ymm1
	vpshufd   $238,%ymm4,%ymm3
	vpaddd    hh1_p2(%rip),%ymm1,%ymm1
	vpxor     hh1_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm20

	vpshufd   $68,%ymm6,%ymm1
	vpshufd   $238,%ymm6,%ymm3
	vpaddd    hh1_p2(%rip),%ymm1,%ymm1
	vpxor     hh1_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm22

	vpshufd   $68,%ymm8,%ymm1
	vpshufd   $238,%ymm8,%ymm3
	vpaddd    hh1_p2(%rip),%ymm1,%ymm1
	vpxor     hh1_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm24

	// <T1,T2,T4,T3> ← Pack-N2D(<T1',T2',T4',T3'>)
	vpsrlq    $32,%ymm16,%ymm17
	vpsrlq    $32,%ymm18,%ymm19
	vpsrlq    $32,%ymm20,%ymm21
	vpsrlq    $32,%ymm22,%ymm23
	vpsrlq    $32,%ymm24,%ymm25
	
	// <T1',T2',T1',T2'> ← Dense-Dup(<T1',T2',T4',T3'>)
	vpermq	  $68,%ymm16,%ymm26
	vpermq	  $68,%ymm18,%ymm28
	vpermq	  $68,%ymm20,%ymm30
	vpermq	  $68,%ymm22,%ymm6
	vpermq	  $68,%ymm24,%ymm8

	// <T1,T2,T1,T2> ← Pack-D2N(<T1',T2',T1',T2'>)
	vpsrlq    $32,%ymm26,%ymm27
	vpsrlq    $32,%ymm28,%ymm29
	vpsrlq    $32,%ymm30,%ymm31
	vpsrlq    $32,%ymm6,%ymm7
	vpsrlq    $32,%ymm8,%ymm9

	vmovdqa   %ymm6,64(%rsp)
	vmovdqa   %ymm7,96(%rsp)
	vmovdqa   %ymm8,128(%rsp)
	vmovdqa   %ymm9,160(%rsp)

	// <T5,T6,T7,T8> ← Mul(<T1,T2,T4,T3>,<T1,T2,T1,T2>)
	vpaddq    %ymm17,%ymm17,%ymm11
	vpaddq    %ymm19,%ymm19,%ymm12
	vpaddq    %ymm21,%ymm21,%ymm13
	vpaddq    %ymm23,%ymm23,%ymm14
	vpaddq    %ymm25,%ymm25,%ymm15

	vpmuludq  vec19(%rip),%ymm27,%ymm1
	vpmuludq  vec19(%rip),%ymm28,%ymm2
	vpmuludq  vec19(%rip),%ymm29,%ymm3
	vpmuludq  vec19(%rip),%ymm30,%ymm4
	vpmuludq  vec19(%rip),%ymm31,%ymm5
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
	vpmuludq  %ymm24,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm22,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm20,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm18,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  %ymm25,%ymm2,%ymm1
	vpmuludq  %ymm24,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm23,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm22,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm21,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm20,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm19,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm18,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  %ymm15,%ymm3,%ymm2
	vpmuludq  %ymm14,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm13,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm12,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm24,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm22,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm20,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm27,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  %ymm25,%ymm4,%ymm3
	vpmuludq  %ymm24,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm23,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm22,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm21,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm20,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  %ymm15,%ymm5,%ymm4
	vpmuludq  %ymm14,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm13,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm24,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm22,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm27,%ymm12,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm29,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  %ymm25,%ymm6,%ymm5
	vpmuludq  %ymm24,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm23,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm22,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  %ymm15,%ymm7,%ymm6
	vpmuludq  %ymm14,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm24,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm27,%ymm13,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm29,%ymm12,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm31,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  %ymm25,%ymm8,%ymm7
	vpmuludq  %ymm24,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  %ymm15,%ymm9,%ymm8
	vpmuludq  %ymm27,%ymm14,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm29,%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm31,%ymm12,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  96(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  %ymm26,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  %ymm28,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm26,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  %ymm29,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm28,%ymm17,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm27,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm26,%ymm19,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  %ymm27,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm26,%ymm17,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  %ymm30,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm28,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm26,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  %ymm31,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm30,%ymm17,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm29,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm28,%ymm19,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm27,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm26,%ymm21,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  64(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm30,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm28,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  96(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  64(%rsp),%ymm17,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm31,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm30,%ymm19,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm29,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm28,%ymm21,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  128(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  64(%rsp),%ymm18,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm30,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  %ymm30,%ymm21,%ymm21
	vpmuludq  160(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  128(%rsp),%ymm17,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  96(%rsp),%ymm18,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  64(%rsp),%ymm19,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  %ymm31,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm9

	vpmuludq  %ymm26,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  %ymm27,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm26,%ymm23,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  %ymm28,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm26,%ymm24,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  %ymm29,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  %ymm28,%ymm23,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  %ymm27,%ymm24,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  %ymm26,%ymm25,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9

	vpsrlq    $25,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask25(%rip),%ymm5,%ymm15

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $26,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpand     vecmask26(%rip),%ymm6,%ymm11

	vpsrlq    $25,%ymm1,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask25(%rip),%ymm1,%ymm1

	vpsrlq    $25,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpand     vecmask25(%rip),%ymm7,%ymm14

	vpsrlq    $26,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpandq    vecmask26(%rip),%ymm2,%ymm28

	vpsrlq    $26,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpand     vecmask26(%rip),%ymm8,%ymm6

	vpsrlq    $25,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpandq    vecmask25(%rip),%ymm3,%ymm29

	vpsrlq    $25,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpsllq    $1,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpsllq    $3,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask25(%rip),%ymm9,%ymm8

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm15,%ymm15
	vpandq    vecmask26(%rip),%ymm4,%ymm30

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm27
	vpandq    vecmask26(%rip),%ymm0,%ymm26

	vpsrlq    $25,%ymm15,%ymm10
	vpaddq    %ymm10,%ymm11,%ymm5
	vpandq    vecmask25(%rip),%ymm15,%ymm31

	vmovdqa   %ymm5,64(%rsp)
	vmovdqa   %ymm14,96(%rsp)
	vmovdqa   %ymm6,128(%rsp)
	vmovdqa   %ymm8,160(%rsp)

	// <T5',T6',T7',T8'> ← Pack-N2D(<T5,T6,T7,T8>)
	vpsllq    $32,%ymm27,%ymm3
	vporq     %ymm3,%ymm26,%ymm0
	vpsllq    $32,%ymm29,%ymm13
	vporq     %ymm13,%ymm28,%ymm2
	vpsllq    $32,%ymm31,%ymm9
	vporq     %ymm9,%ymm30,%ymm4
	vpsllq    $32,%ymm8,%ymm8
	vpor      %ymm8,%ymm6,%ymm8
	vpsllq    $32,%ymm14,%ymm14
	vpor      %ymm14,%ymm5,%ymm6

	// <T9',T10',T11',T12'> ← Dense-H2-H(<T5',T6',T7',T8'>)
	vpshufd   $68,%ymm0,%ymm1
	vpand     h2h_mask(%rip),%ymm1,%ymm1
	vpshufd   $238,%ymm0,%ymm3
	vpaddd    h2h_p1(%rip),%ymm1,%ymm1
	vpxor     h2h_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm0

	vpshufd   $68,%ymm2,%ymm1
	vpand     h2h_mask(%rip),%ymm1,%ymm1
	vpshufd   $238,%ymm2,%ymm3
	vpaddd    h2h_p2(%rip),%ymm1,%ymm1
	vpxor     h2h_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm12

	vpshufd   $68,%ymm4,%ymm1
	vpand     h2h_mask(%rip),%ymm1,%ymm1
	vpshufd   $238,%ymm4,%ymm3
	vpaddd    h2h_p2(%rip),%ymm1,%ymm1
	vpxor     h2h_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm7

	vpshufd   $68,%ymm6,%ymm1
	vpand     h2h_mask(%rip),%ymm1,%ymm1
	vpshufd   $238,%ymm6,%ymm3
	vpaddd    h2h_p2(%rip),%ymm1,%ymm1
	vpxor     h2h_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm5

	vpshufd   $68,%ymm8,%ymm1
	vpand     h2h_mask(%rip),%ymm1,%ymm1
	vpshufd   $238,%ymm8,%ymm3
	vpaddd    h2h_p2(%rip),%ymm1,%ymm1
	vpxor     h2h_xor(%rip),%ymm3,%ymm3
	vpaddd    %ymm1,%ymm3,%ymm6

	// <T9',T10',1',X1'> ← Blend(<0',0',1',X1'>,<T9',T10',T11',T12'>,1100)
	movl      $240,%ebx
	kmovw     %ebx,%k1	
	vpblendmd  0(%rsi),%ymm0,%ymm16{%k1}
	vpblendmd  32(%rsi),%ymm12,%ymm18{%k1}
	vpblendmd  64(%rsi),%ymm7,%ymm20{%k1}
	vpblendmd  96(%rsi),%ymm5,%ymm22{%k1}
	vpblendmd  128(%rsi),%ymm6,%ymm24{%k1}

	// <T9,T10,1,X1> ← Pack-D2N(<T9',T10',1',X1'>)
	vpsrlq    $32,%ymm16,%ymm17
	vpsrlq    $32,%ymm18,%ymm19
	vpsrlq    $32,%ymm20,%ymm21
	vpsrlq    $32,%ymm22,%ymm23
	vpsrlq    $32,%ymm24,%ymm25

	// <T9,T10,T11,T12> ← Pack-D2N(<T9',T10',T11',T12'>)
	vpsrlq    $32,%ymm0,%ymm3
	vpsrlq    $32,%ymm12,%ymm13
	vpsrlq    $32,%ymm7,%ymm9
	vpsrlq    $32,%ymm5,%ymm14
	vpsrlq    $32,%ymm6,%ymm8

	// <0,T13,0,0> ← Unreduced-Mulc(<T9,T10,T11,T12>,<0,a24,0,0>)
	// <T5,T14,T7,T8> ← Add(<0,T13,0,0>,<T5,T6,T7,T8>)
	vpmuludq  a24(%rip),%ymm0,%ymm1
	vpaddq    %ymm26,%ymm1,%ymm1
	vpmuludq  a24(%rip),%ymm3,%ymm2
	vpaddq    %ymm27,%ymm2,%ymm2
	vpmuludq  a24(%rip),%ymm12,%ymm4
	vpaddq    %ymm28,%ymm4,%ymm4
	vpmuludq  a24(%rip),%ymm9,%ymm11
	vpaddq    %ymm31,%ymm11,%ymm11
	vpmuludq  a24(%rip),%ymm5,%ymm15
	vpaddq    64(%rsp),%ymm15,%ymm15

	vpsrlq    $26,%ymm1,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask26(%rip),%ymm1,%ymm1

	vpsrlq    $25,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm15,%ymm15
	vpand     vecmask25(%rip),%ymm11,%ymm11

	vpsrlq    $25,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpandq    vecmask25(%rip),%ymm2,%ymm27

	vpmuludq  a24(%rip),%ymm14,%ymm2
	vpaddq    96(%rsp),%ymm2,%ymm2

	vpsrlq    $26,%ymm15,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask26(%rip),%ymm15,%ymm15
	vmovdqa   %ymm15,64(%rsp)

	vpmuludq  a24(%rip),%ymm13,%ymm15
	vpaddq    %ymm29,%ymm15,%ymm15

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm15,%ymm15
	vpandq    vecmask26(%rip),%ymm4,%ymm28

	vpmuludq  a24(%rip),%ymm6,%ymm4
	vpaddq    128(%rsp),%ymm4,%ymm4

	vpsrlq    $25,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpand     vecmask25(%rip),%ymm2,%ymm2
	vmovdqa   %ymm2,96(%rsp)

	vpmuludq  a24(%rip),%ymm7,%ymm2
	vpaddq    %ymm30,%ymm2,%ymm2

	vpsrlq    $25,%ymm15,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpandq    vecmask25(%rip),%ymm15,%ymm29

	vpmuludq  a24(%rip),%ymm8,%ymm15
	vpaddq    160(%rsp),%ymm15,%ymm15

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm15,%ymm15
	vpand     vecmask26(%rip),%ymm4,%ymm4
	vmovdqa   %ymm4,128(%rsp)

	vpsrlq    $26,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm11,%ymm31
	vpandq    vecmask26(%rip),%ymm2,%ymm30

	vpsrlq    $25,%ymm15,%ymm10
	vpmuludq  vec19(%rip),%ymm10,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm26
	vpand     vecmask25(%rip),%ymm15,%ymm15
	vmovdqa   %ymm15,160(%rsp)

	// <*,*,T15,T16> ← Sqr(<T9,T10,T11,T12>)
	vmovdqa   %ymm12,192(%rsp)
	vmovdqa   %ymm13,224(%rsp)
	vmovdqa   %ymm7,256(%rsp)
	vmovdqa   %ymm9,288(%rsp)
	vmovdqa   %ymm5,320(%rsp)
	vmovdqa   %ymm14,352(%rsp)
	vmovdqa   %ymm6,384(%rsp)
	vmovdqa   %ymm8,416(%rsp)

	vpmuludq  vec19(%rip),%ymm5,%ymm2
	vpmuludq  vec19(%rip),%ymm6,%ymm6
	vpmuludq  vec38(%rip),%ymm9,%ymm1
	vpmuludq  vec38(%rip),%ymm14,%ymm4
	vpmuludq  vec38(%rip),%ymm8,%ymm8

	vpaddq    %ymm0,%ymm0,%ymm10
	vpaddq    %ymm3,%ymm3,%ymm11
	vpaddq    %ymm12,%ymm12,%ymm12
	vpaddq    %ymm13,%ymm13,%ymm13
	vpaddq    %ymm7,%ymm7,%ymm7
	vpaddq    %ymm9,%ymm9,%ymm9
	vpaddq    %ymm5,%ymm5,%ymm5
	vpaddq    %ymm14,%ymm14,%ymm14

	vpmuludq  %ymm0,%ymm0,%ymm0
	vpmuludq  %ymm11,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpmuludq  %ymm12,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpmuludq  %ymm13,%ymm4,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpmuludq  %ymm7,%ymm2,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpmuludq  288(%rsp),%ymm1,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0

	vpmuludq  320(%rsp),%ymm1,%ymm1
	vpmuludq  %ymm3,%ymm10,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1
	vpmuludq  192(%rsp),%ymm8,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1
	vpmuludq  %ymm13,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1
	vpmuludq  256(%rsp),%ymm4,%ymm15
	vpaddq    %ymm15,%ymm1,%ymm1

	vpmuludq  320(%rsp),%ymm2,%ymm2
	vpmuludq  192(%rsp),%ymm10,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpmuludq  %ymm3,%ymm11,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpmuludq  %ymm13,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpmuludq  %ymm7,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2
	vpmuludq  %ymm9,%ymm4,%ymm15
	vpaddq    %ymm15,%ymm2,%ymm2

	vpmuludq  224(%rsp),%ymm10,%ymm3
	vpmuludq  192(%rsp),%ymm11,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3
	vpmuludq  256(%rsp),%ymm8,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3
	vpmuludq  %ymm9,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3
	vpmuludq  320(%rsp),%ymm4,%ymm15
	vpaddq    %ymm15,%ymm3,%ymm3

	vpmuludq  352(%rsp),%ymm4,%ymm4
	vpmuludq  256(%rsp),%ymm10,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vpmuludq  %ymm11,%ymm13,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vmovdqa   192(%rsp),%ymm15
	vpmuludq  %ymm15,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vpmuludq  %ymm9,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4
	vpmuludq  %ymm5,%ymm6,%ymm15
	vpaddq    %ymm15,%ymm4,%ymm4

	vpmuludq  288(%rsp),%ymm10,%ymm5
	vpmuludq  256(%rsp),%ymm11,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5
	vpmuludq  224(%rsp),%ymm12,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5
	vpmuludq  320(%rsp),%ymm8,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5
	vpmuludq  %ymm6,%ymm14,%ymm15
	vpaddq    %ymm15,%ymm5,%ymm5

	vpmuludq  384(%rsp),%ymm6,%ymm6
	vpmuludq  320(%rsp),%ymm10,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpmuludq  %ymm11,%ymm9,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpmuludq  256(%rsp),%ymm12,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpmuludq  224(%rsp),%ymm13,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6
	vpmuludq  %ymm14,%ymm8,%ymm15
	vpaddq    %ymm15,%ymm6,%ymm6

	vpmuludq  352(%rsp),%ymm10,%ymm7
	vpmuludq  320(%rsp),%ymm11,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7
	vpmuludq  288(%rsp),%ymm12,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7
	vpmuludq  256(%rsp),%ymm13,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7
	vpmuludq  384(%rsp),%ymm8,%ymm15
	vpaddq    %ymm15,%ymm7,%ymm7

	vpmuludq  416(%rsp),%ymm8,%ymm8
	vpmuludq  384(%rsp),%ymm10,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpmuludq  %ymm11,%ymm14,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpmuludq  320(%rsp),%ymm12,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vpmuludq  %ymm13,%ymm9,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8
	vmovdqa   256(%rsp),%ymm15
	vpmuludq  %ymm15,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm8,%ymm8

	vpmuludq  256(%rsp),%ymm9,%ymm9
	vpmuludq  416(%rsp),%ymm10,%ymm15
	vpaddq    %ymm15,%ymm9,%ymm9
	vpmuludq  384(%rsp),%ymm11,%ymm15
	vpaddq    %ymm15,%ymm9,%ymm9
	vpmuludq  352(%rsp),%ymm12,%ymm15
	vpaddq    %ymm15,%ymm9,%ymm9
	vpmuludq  320(%rsp),%ymm13,%ymm15
	vpaddq    %ymm15,%ymm9,%ymm9

	vpsrlq    $25,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask25(%rip),%ymm5,%ymm5

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $26,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpand     vecmask26(%rip),%ymm6,%ymm6

	vpsrlq    $25,%ymm1,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask25(%rip),%ymm1,%ymm1

	vpsrlq    $25,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpand     vecmask25(%rip),%ymm7,%ymm7

	vpsrlq    $26,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpand     vecmask26(%rip),%ymm2,%ymm2

	vpsrlq    $26,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpand     vecmask26(%rip),%ymm8,%ymm8

	vpsrlq    $25,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpand     vecmask25(%rip),%ymm3,%ymm3

	vpsrlq    $25,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpsllq    $1,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpsllq    $3,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask25(%rip),%ymm9,%ymm9

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask26(%rip),%ymm4,%ymm4

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $25,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask25(%rip),%ymm5,%ymm5

	// <T5,T14,T15,T16> ← Blend(<T5,T14,T7,T8>,<*,*,T15,T16>,0011)
	movl      $15,%ebx
	kmovw     %ebx,%k1
	vpblendmd %ymm26,%ymm0,%ymm26{%k1}
	vpblendmd %ymm27,%ymm1,%ymm27{%k1}

	vpblendmd %ymm28,%ymm2,%ymm28{%k1}
	vpblendmd %ymm29,%ymm3,%ymm29{%k1}

	vpblendmd %ymm30,%ymm4,%ymm30{%k1}
	vpblendmd %ymm31,%ymm5,%ymm31{%k1}

	vpblendmd 64(%rsp),%ymm6,%ymm6{%k1}
	vpblendmd 96(%rsp),%ymm7,%ymm7{%k1}
	vmovdqa   %ymm6,64(%rsp)
	vmovdqa   %ymm7,96(%rsp)

	vpblendmd 128(%rsp),%ymm8,%ymm8{%k1}
	vpblendmd 160(%rsp),%ymm9,%ymm9{%k1}
	vmovdqa   %ymm8,128(%rsp)
	vmovdqa   %ymm9,160(%rsp)

	// <X2,Z2,X3,Z3> ← Mul(<T5,T14,T15,T16>,<T6,T10,1,X1>)
	vpaddq    %ymm17,%ymm17,%ymm11
	vpaddq    %ymm19,%ymm19,%ymm12
	vpaddq    %ymm21,%ymm21,%ymm13
	vpaddq    %ymm23,%ymm23,%ymm14
	vpaddq    %ymm25,%ymm25,%ymm15

	vpmuludq  vec19(%rip),%ymm27,%ymm1
	vpmuludq  vec19(%rip),%ymm28,%ymm2
	vpmuludq  vec19(%rip),%ymm29,%ymm3
	vpmuludq  vec19(%rip),%ymm30,%ymm4
	vpmuludq  vec19(%rip),%ymm31,%ymm5
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
	vpmuludq  %ymm24,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm22,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm20,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpmuludq  %ymm18,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  %ymm25,%ymm2,%ymm1
	vpmuludq  %ymm24,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm23,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm22,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm21,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm20,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm19,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm18,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  %ymm15,%ymm3,%ymm2
	vpmuludq  %ymm14,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm13,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm12,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm24,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm22,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm20,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm27,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  %ymm25,%ymm4,%ymm3
	vpmuludq  %ymm24,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm23,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm22,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm21,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm20,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  %ymm15,%ymm5,%ymm4
	vpmuludq  %ymm14,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm13,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm24,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm22,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm27,%ymm12,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm29,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  %ymm25,%ymm6,%ymm5
	vpmuludq  %ymm24,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm23,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm22,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  %ymm15,%ymm7,%ymm6
	vpmuludq  %ymm14,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm24,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm27,%ymm13,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm29,%ymm12,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm31,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  %ymm25,%ymm8,%ymm7
	vpmuludq  %ymm24,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  %ymm15,%ymm9,%ymm8
	vpmuludq  %ymm27,%ymm14,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm29,%ymm13,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm31,%ymm12,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  96(%rsp),%ymm11,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  %ymm26,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0

	vpmuludq  %ymm28,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpmuludq  %ymm26,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2

	vpmuludq  %ymm29,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm28,%ymm17,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm27,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpmuludq  %ymm26,%ymm19,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3

	vpmuludq  %ymm27,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpmuludq  %ymm26,%ymm17,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1

	vpmuludq  %ymm30,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm28,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpmuludq  %ymm26,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4

	vpmuludq  %ymm31,%ymm16,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm30,%ymm17,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm29,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm28,%ymm19,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm27,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpmuludq  %ymm26,%ymm21,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5

	vpmuludq  64(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm30,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpmuludq  %ymm28,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  96(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  64(%rsp),%ymm17,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm31,%ymm18,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm30,%ymm19,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm29,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm28,%ymm21,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  128(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  64(%rsp),%ymm18,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm30,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  %ymm30,%ymm21,%ymm21
	vpmuludq  160(%rsp),%ymm16,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  128(%rsp),%ymm17,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  96(%rsp),%ymm18,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  64(%rsp),%ymm19,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm21
	vpmuludq  %ymm31,%ymm20,%ymm10
	vpaddq    %ymm10,%ymm21,%ymm9

	vpmuludq  %ymm26,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6

	vpmuludq  %ymm27,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpmuludq  %ymm26,%ymm23,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7

	vpmuludq  %ymm28,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpmuludq  %ymm26,%ymm24,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8

	vpmuludq  %ymm29,%ymm22,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  %ymm28,%ymm23,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  %ymm27,%ymm24,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9
	vpmuludq  %ymm26,%ymm25,%ymm10
	vpaddq    %ymm10,%ymm9,%ymm9

	vpsrlq    $25,%ymm5,%ymm10	
	vpaddq    %ymm10,%ymm6,%ymm6		
	vpand     vecmask25(%rip),%ymm5,%ymm5

	vpsrlq    $26,%ymm0,%ymm10	
	vpaddq    %ymm10,%ymm1,%ymm1		
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $26,%ymm6,%ymm10	
	vpaddq    %ymm10,%ymm7,%ymm7		
	vpand     vecmask26(%rip),%ymm6,%ymm6

	vpsrlq    $25,%ymm1,%ymm10	
	vpaddq    %ymm10,%ymm2,%ymm2		
	vpand     vecmask25(%rip),%ymm1,%ymm1

	vpsrlq    $25,%ymm7,%ymm10	
	vpaddq    %ymm10,%ymm8,%ymm8		
	vpand     vecmask25(%rip),%ymm7,%ymm14

	vpsrlq    $26,%ymm2,%ymm10	
	vpaddq    %ymm10,%ymm3,%ymm3		
	vpand     vecmask26(%rip),%ymm2,%ymm2

	vpsrlq    $26,%ymm8,%ymm10	
	vpaddq    %ymm10,%ymm9,%ymm9		
	vpand     vecmask26(%rip),%ymm8,%ymm8

	vpsrlq    $25,%ymm3,%ymm10	
	vpaddq    %ymm10,%ymm4,%ymm4		
	vpand     vecmask25(%rip),%ymm3,%ymm12

	vpsrlq    $25,%ymm9,%ymm10
	vpmuludq  vec19(%rip),%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask25(%rip),%ymm9,%ymm15

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask26(%rip),%ymm4,%ymm4

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm11
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $25,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask25(%rip),%ymm5,%ymm13

	// <X2',Z2',X3',Z3'> ← Pack-N2D(<X2,Z2,X3,Z3>)
	vpsllq    $32,%ymm11,%ymm11
	vpor      %ymm11,%ymm0,%ymm0
	vpsllq    $32,%ymm12,%ymm12
	vpor      %ymm12,%ymm2,%ymm2
	vpsllq    $32,%ymm13,%ymm13
	vpor      %ymm13,%ymm4,%ymm4
	vpsllq    $32,%ymm14,%ymm14
	vpor      %ymm14,%ymm6,%ymm6
	vpsllq    $32,%ymm15,%ymm15
	vpor      %ymm15,%ymm8,%ymm8

	subb      $1,%cl
	cmpb	  $0,%cl
	jge       .L2

	movb	  $7,%cl
	subq      $1,%r15
	cmpq	  $0,%r15
	jge       .L1

	// <X2,Z2,X3,Z3> ← Pack-D2N(<X2',Z2',X3',Z3'>)
	vpsrlq    $32,%ymm0,%ymm11
	vpand     vecmask32(%rip),%ymm0,%ymm0
	vpsrlq    $32,%ymm2,%ymm12
	vpand     vecmask32(%rip),%ymm2,%ymm2
	vpsrlq    $32,%ymm4,%ymm13
	vpand     vecmask32(%rip),%ymm4,%ymm4
	vpsrlq    $32,%ymm6,%ymm14
	vpand     vecmask32(%rip),%ymm6,%ymm6
	vpsrlq    $32,%ymm8,%ymm15
	vpand     vecmask32(%rip),%ymm8,%ymm8

	// <X2,Z2,X3,Z3> ← Reduce(<X2,Z2,X3,Z3>)
	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm11,%ymm11
	vpand     vecmask26(%rip),%ymm0,%ymm0

	vpsrlq    $25,%ymm11,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask25(%rip),%ymm11,%ymm11

	vpsrlq    $26,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm12,%ymm12
	vpand     vecmask26(%rip),%ymm2,%ymm2

	vpsrlq    $25,%ymm12,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpand     vecmask25(%rip),%ymm12,%ymm12

	vpsrlq    $26,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm13,%ymm13
	vpand     vecmask26(%rip),%ymm4,%ymm4

	vpsrlq    $25,%ymm13,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask25(%rip),%ymm13,%ymm13

	vpsrlq    $26,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm14,%ymm14
	vpand     vecmask26(%rip),%ymm6,%ymm6

	vpsrlq    $25,%ymm14,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpand     vecmask25(%rip),%ymm14,%ymm14

	vpsrlq    $26,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm15,%ymm15
	vpand     vecmask26(%rip),%ymm8,%ymm8

	vpsrlq    $25,%ymm15,%ymm10
	vpmuludq  vec19(%rip),%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask25(%rip),%ymm15,%ymm15

	vpsrlq    $26,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm11,%ymm11
	vpand     vecmask26(%rip),%ymm0,%ymm0

	// store <X2,Z2,X3,Z3>
	vmovdqa   %ymm0,0(%rdi)
	vmovdqa   %ymm11,32(%rdi)
	vmovdqa   %ymm2,64(%rdi)
	vmovdqa   %ymm12,96(%rdi)
	vmovdqa   %ymm4,128(%rdi)
	vmovdqa   %ymm13,160(%rdi)
	vmovdqa   %ymm6,192(%rdi)
	vmovdqa   %ymm14,224(%rdi)
	vmovdqa   %ymm8,256(%rdi)
	vmovdqa   %ymm15,288(%rdi)

	movq 	  0(%rsp),%r11
	movq 	  8(%rsp),%r12
	movq 	  16(%rsp),%r13
	movq 	  24(%rsp),%r14
	movq 	  32(%rsp),%r15
	movq 	  40(%rsp),%rbx
	movq 	  48(%rsp),%rbp

	movq 	  %r11,%rsp

	ret
