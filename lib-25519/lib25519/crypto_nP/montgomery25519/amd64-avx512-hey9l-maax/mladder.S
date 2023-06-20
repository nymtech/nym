// linker define mladder
// linker use hh_p1
// linker use hh_p2
// linker use hh_p3
// linker use hh_xor1
// linker use hh_xor2
// linker use sub_p1
// linker use sub_p2
// linker use sub_p3
// linker use swap_c
// linker use swap_mask
// linker use vec19
// linker use vec1216
// linker use vecmask23
// linker use vecmask29
// linker use vecmask29d
// linker use vecmask32

/* Assembly for Montgomery ladder. */

#include "consts_namespace.h"

	.p2align 5
	.globl _CRYPTO_SHARED_NAMESPACE(mladder)
	.globl CRYPTO_SHARED_NAMESPACE(mladder)

_CRYPTO_SHARED_NAMESPACE(mladder):
CRYPTO_SHARED_NAMESPACE(mladder):

	movq 	  %rsp,%r11
	andq 	  $-32,%rsp
	subq 	  $608,%rsp

	movq 	  %r11,0(%rsp)
	movq 	  %r12,8(%rsp)
	movq 	  %r13,16(%rsp)
	movq 	  %r14,24(%rsp)
	movq 	  %r15,32(%rsp)
	movq 	  %rbx,40(%rsp)
	movq 	  %rbp,48(%rsp)

	// load <0,0,1,X1>
	vmovdqa   0(%rsi),%ymm0
	vmovdqa   32(%rsi),%ymm1
	vmovdqa   64(%rsi),%ymm2
	vmovdqa   96(%rsi),%ymm3
	vmovdqa   128(%rsi),%ymm4
	vmovdqa   160(%rsi),%ymm5
	vmovdqa   192(%rsi),%ymm6
	vmovdqa   224(%rsi),%ymm7
	vmovdqa   256(%rsi),%ymm8

	// <0',0',1',X1'> ← Pack-D2N(<0,0,1,X1>)
	vpsllq    $32,%ymm4,%ymm4
	vpor      %ymm4,%ymm0,%ymm0
	vpsllq    $32,%ymm5,%ymm5
	vpor      %ymm5,%ymm1,%ymm1
	vpsllq    $32,%ymm6,%ymm6
	vpor      %ymm6,%ymm2,%ymm2
	vpsllq    $32,%ymm7,%ymm7
	vpor      %ymm7,%ymm3,%ymm3

	vmovdqa   %ymm0,0(%rsi)
	vmovdqa   %ymm1,32(%rsi)
	vmovdqa   %ymm2,64(%rsi)
	vmovdqa   %ymm3,96(%rsi)
	vmovdqa   %ymm8,128(%rsi)

	// load <X2,Z2,X3,Z3>
	vmovdqa   0(%rdi),%ymm10
	vmovdqa   32(%rdi),%ymm11
	vmovdqa   64(%rdi),%ymm12
	vmovdqa   96(%rdi),%ymm13
	vmovdqa   128(%rdi),%ymm0
	vmovdqa   160(%rdi),%ymm1
	vmovdqa   192(%rdi),%ymm2
	vmovdqa   224(%rdi),%ymm3
	vmovdqa   256(%rdi),%ymm4

	// <X2',Z2',X3',Z3'> ← Pack-D2N(<X2,Z2,X3,Z3>)
	vpsllq    $32,%ymm0,%ymm0
	vpor      %ymm0,%ymm10,%ymm10
	vpsllq    $32,%ymm1,%ymm1
	vpor      %ymm1,%ymm11,%ymm11
	vpsllq    $32,%ymm2,%ymm2
	vpor      %ymm2,%ymm12,%ymm12
	vpsllq    $32,%ymm3,%ymm3
	vpor      %ymm3,%ymm13,%ymm13

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

	vpermd	  %ymm10,%ymm7,%ymm10
	vpermd	  %ymm11,%ymm7,%ymm11
	vpermd	  %ymm12,%ymm7,%ymm12
	vpermd	  %ymm13,%ymm7,%ymm13
	vpermd	  %ymm4,%ymm7,%ymm4

	// <T1',T2',T3',T4'> ← Dense-H-H(<X2',Z2',X3',Z3'>)
	vpshufd   $68,%ymm10,%ymm5
	vpshufd   $238,%ymm10,%ymm6
	vpaddd    hh_p1(%rip),%ymm5,%ymm5
	vpxor     hh_xor1(%rip),%ymm6,%ymm6
	vpaddd    %ymm5,%ymm6,%ymm10

	vpshufd   $68,%ymm11,%ymm5
	vpshufd   $238,%ymm11,%ymm6
	vpaddd    hh_p2(%rip),%ymm5,%ymm5
	vpxor     hh_xor1(%rip),%ymm6,%ymm6
	vpaddd    %ymm5,%ymm6,%ymm11

	vpshufd   $68,%ymm12,%ymm5
	vpshufd  $238,%ymm12,%ymm6
	vpaddd    hh_p2(%rip),%ymm5,%ymm5
	vpxor     hh_xor1(%rip),%ymm6,%ymm6
	vpaddd    %ymm5,%ymm6,%ymm12

	vpshufd   $68,%ymm13,%ymm5
	vpshufd   $238,%ymm13,%ymm6
	vpaddd    hh_p2(%rip),%ymm5,%ymm5
	vpxor     hh_xor1(%rip),%ymm6,%ymm6
	vpaddd    %ymm5,%ymm6,%ymm13

	vpshufd   $68,%ymm4,%ymm5
	vpshufd   $238,%ymm4,%ymm6
	vpaddd    hh_p3(%rip),%ymm5,%ymm5
	vpxor     hh_xor2(%rip),%ymm6,%ymm6
	vpaddd    %ymm5,%ymm6,%ymm4

	vpsrld    $29,%ymm10,%ymm15
	vpaddd    %ymm15,%ymm11,%ymm11
	vpand     vecmask29d(%rip),%ymm10,%ymm10

	vpsrld    $29,%ymm11,%ymm15
	vpaddd    %ymm15,%ymm12,%ymm12
	vpandq    vecmask29d(%rip),%ymm11,%ymm17

	vpsrld    $29,%ymm12,%ymm15
	vpaddd    %ymm15,%ymm13,%ymm13
	vpandq    vecmask29d(%rip),%ymm12,%ymm18

	vpsrld    $29,%ymm13,%ymm15
	vpsllq    $32,%ymm15,%ymm15
	vpaddd    %ymm15,%ymm10,%ymm10
	vpsrlq    $61,%ymm13,%ymm15
	vpaddd    %ymm15,%ymm4,%ymm4
	vpandq    vecmask29d(%rip),%ymm13,%ymm19

	vpsrlq    $23,%ymm4,%ymm15
	vpmuludq  vec19(%rip),%ymm15,%ymm15
	vpaddd    %ymm15,%ymm10,%ymm16
	vpand     vecmask23(%rip),%ymm4,%ymm9

	// <T1,T2,T3,T4> ← Pack-N2D(<T1',T2',T3',T4'>)
	vpsrlq    $32,%ymm16,%ymm5
	vpsrlq    $32,%ymm17,%ymm6
	vpsrlq    $32,%ymm18,%ymm7
	vpsrlq    $32,%ymm19,%ymm8

	// <T1',T2',T2',T1'> ← Dense-Dup(<T1',T2',T3',T4'>)
	vpermq	  $20,%ymm16,%ymm10
	vpermq	  $20,%ymm17,%ymm11
	vpermq	  $20,%ymm18,%ymm12
	vpermq	  $20,%ymm19,%ymm13
	vpermq	  $20,%ymm9,%ymm4

	// <T1,T2,T2,T1> ← Pack-D2N(<T1',T2',T2',T1'>)
	vpsrlq    $32,%ymm10,%ymm0
	vpsrlq    $32,%ymm11,%ymm1
	vpsrlq    $32,%ymm12,%ymm2
	vpsrlq    $32,%ymm13,%ymm3

	// <T5,T6,T7,T8> ← Mul(<T1,T2,T3,T4>,<T1,T2,T2,T1>)
	vpmuludq  %ymm5,%ymm0,%ymm20

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm21

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm22

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm23

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm24

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm25

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm26

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm27

	vpmuludq  %ymm9,%ymm4,%ymm28

	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm11,%ymm1,%ymm1
	vpaddq    %ymm12,%ymm2,%ymm2
	vpaddq    %ymm13,%ymm3,%ymm3
	vpaddq    %ymm16,%ymm5,%ymm5
	vpaddq    %ymm17,%ymm6,%ymm6
	vpaddq    %ymm18,%ymm7,%ymm7
	vpaddq    %ymm19,%ymm8,%ymm8

	vpmuludq  %ymm16,%ymm10,%ymm29
	vpaddq    %ymm20,%ymm29,%ymm15
	vmovdqa   %ymm15,192(%rsp)

	vpmuludq  %ymm17,%ymm10,%ymm15
	vpmuludq  %ymm16,%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm30
	vpaddq    %ymm21,%ymm30,%ymm15
	vmovdqa   %ymm15,224(%rsp)

	vpmuludq  %ymm18,%ymm10,%ymm15
	vpmuludq  %ymm17,%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm16,%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm31
	vpaddq    %ymm22,%ymm31,%ymm15
	vmovdqa   %ymm15,256(%rsp)

	vpmuludq  %ymm19,%ymm10,%ymm15
	vpmuludq  %ymm18,%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm17,%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm16,%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,64(%rsp)
	vpaddq    %ymm23,%ymm15,%ymm15
	vmovdqa   %ymm15,288(%rsp)

	vpmuludq  %ymm19,%ymm11,%ymm15
	vpmuludq  %ymm18,%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm17,%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,96(%rsp)
	vpaddq    %ymm24,%ymm15,%ymm15
	vmovdqa   %ymm15,320(%rsp)

	vpmuludq  %ymm19,%ymm12,%ymm15
	vpmuludq  %ymm18,%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,128(%rsp)
	vpaddq    %ymm25,%ymm15,%ymm15
	vmovdqa   %ymm15,352(%rsp)

	vpmuludq  %ymm19,%ymm13,%ymm15
	vmovdqa   %ymm15,160(%rsp)
	vpaddq    %ymm26,%ymm15,%ymm15
	vmovdqa   %ymm15,384(%rsp)

	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,416(%rsp)

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

	vmovdqa   416(%rsp),%ymm9

	vpsubq    192(%rsp),%ymm9,%ymm9
	vpaddq    96(%rsp),%ymm9,%ymm9
	vpsubq    224(%rsp),%ymm10,%ymm10
	vpaddq    128(%rsp),%ymm10,%ymm10
	vpsubq    256(%rsp),%ymm11,%ymm11
	vpaddq    160(%rsp),%ymm11,%ymm11
	vpsubq    288(%rsp),%ymm12,%ymm12
	vpsubq    320(%rsp),%ymm13,%ymm13
	vpaddq    %ymm20,%ymm13,%ymm13
	vpsubq    352(%rsp),%ymm0,%ymm0
	vpaddq    %ymm21,%ymm0,%ymm0
	vpsubq    384(%rsp),%ymm1,%ymm1
	vpaddq    %ymm22,%ymm1,%ymm1
	vpsubq    %ymm27,%ymm2,%ymm2
	vpaddq    %ymm23,%ymm2,%ymm2
	vpsubq    %ymm28,%ymm3,%ymm3
	vpaddq    %ymm24,%ymm3,%ymm3

	vpsrlq    $29,%ymm0,%ymm14
	vpaddq    %ymm14,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0
	vpmuludq  vec1216(%rip),%ymm0,%ymm0
	vpaddq    %ymm29,%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1
	vpmuludq  vec1216(%rip),%ymm1,%ymm1
	vpaddq    %ymm30,%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2
	vpmuludq  vec1216(%rip),%ymm2,%ymm2
	vpaddq    %ymm31,%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm14
	vpaddq    %ymm25,%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm3,%ymm3
	vpmuludq  vec1216(%rip),%ymm3,%ymm3
	vpaddq    64(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    %ymm26,%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm4
	vpmuludq  vec1216(%rip),%ymm4,%ymm4
	vpaddq    %ymm9,%ymm4,%ymm4

	vpsrlq    $29,%ymm15,%ymm14
	vpaddq    %ymm27,%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm15,%ymm5
	vpmuludq  vec1216(%rip),%ymm5,%ymm5
	vpaddq    %ymm10,%ymm5,%ymm5

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    %ymm28,%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm6
	vpmuludq  vec1216(%rip),%ymm6,%ymm6
	vpaddq    %ymm11,%ymm6,%ymm6

	vpsrlq    $29,%ymm15,%ymm8
	vpand     vecmask29(%rip),%ymm15,%ymm7

	vpmuludq  vec1216(%rip),%ymm7,%ymm7
	vpaddq    %ymm12,%ymm7,%ymm7
	vpmuludq  vec1216(%rip),%ymm8,%ymm8
	vpaddq    %ymm13,%ymm8,%ymm8

	vpsrlq    $29,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	vpsrlq    $29,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask29(%rip),%ymm5,%ymm5

	vpsrlq    $29,%ymm1,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1

	vpsrlq    $29,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpand     vecmask29(%rip),%ymm6,%ymm6

	vpsrlq    $29,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2

	vpsrlq    $29,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	vpsrlq    $29,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm4
	vpand     vecmask29(%rip),%ymm3,%ymm3

	vpsrlq    $23,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm10,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpsllq    $3,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask23(%rip),%ymm8,%ymm8

	vpsrlq    $29,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	// <T5',T6',T7',T8'> ← Pack-N2D(<T5,T6,T7,T8>)
	vpsllq    $32,%ymm4,%ymm4
	vpor      %ymm4,%ymm0,%ymm0
	vpsllq    $32,%ymm5,%ymm5
	vpor      %ymm5,%ymm1,%ymm1
	vpsllq    $32,%ymm6,%ymm6
	vpor      %ymm6,%ymm2,%ymm2
	vpsllq    $32,%ymm7,%ymm7
	vpor      %ymm7,%ymm3,%ymm3

	// <T9',T10',T11',T12'> ← Dense-H-H(<T5',T6',T7',T8'>)
	vpshufd   $68,%ymm0,%ymm9
	vpshufd   $238,%ymm0,%ymm10
	vpaddd    hh_p1(%rip),%ymm9,%ymm9
	vpxor     hh_xor1(%rip),%ymm10,%ymm10
	vpaddd    %ymm9,%ymm10,%ymm0

	vpshufd   $68,%ymm1,%ymm9
	vpshufd   $238,%ymm1,%ymm10
	vpaddd    hh_p2(%rip),%ymm9,%ymm9
	vpxor     hh_xor1(%rip),%ymm10,%ymm10
	vpaddd    %ymm9,%ymm10,%ymm1

	vpshufd   $68,%ymm2,%ymm9
	vpshufd   $238,%ymm2,%ymm10
	vpaddd    hh_p2(%rip),%ymm9,%ymm9
	vpxor     hh_xor1(%rip),%ymm10,%ymm10
	vpaddd    %ymm9,%ymm10,%ymm2

	vpshufd   $68,%ymm3,%ymm9
	vpshufd   $238,%ymm3,%ymm10
	vpaddd    hh_p2(%rip),%ymm9,%ymm9
	vpxor     hh_xor1(%rip),%ymm10,%ymm10
	vpaddd    %ymm9,%ymm10,%ymm3

	vpshufd   $68,%ymm8,%ymm9
	vpshufd   $238,%ymm8,%ymm10
	vpaddd    hh_p3(%rip),%ymm9,%ymm9
	vpxor     hh_xor2(%rip),%ymm10,%ymm10
	vpaddd    %ymm9,%ymm10,%ymm8

	vpsrld    $29,%ymm0,%ymm15
	vpaddd    %ymm15,%ymm1,%ymm1
	vpand     vecmask29d(%rip),%ymm0,%ymm0

	vpsrld    $29,%ymm1,%ymm15
	vpaddd    %ymm15,%ymm2,%ymm2
	vpandq    vecmask29d(%rip),%ymm1,%ymm21

	vpsrld    $29,%ymm2,%ymm15
	vpaddd    %ymm15,%ymm3,%ymm3
	vpandq    vecmask29d(%rip),%ymm2,%ymm22

	vpsrld    $29,%ymm3,%ymm15
	vpsllq    $32,%ymm15,%ymm15
	vpaddd    %ymm15,%ymm0,%ymm0
	vpsrlq    $61,%ymm3,%ymm15
	vpaddd    %ymm15,%ymm8,%ymm8
	vpandq    vecmask29d(%rip),%ymm3,%ymm23

	vpsrlq    $23,%ymm8,%ymm15
	vpmuludq  vec19(%rip),%ymm15,%ymm15
	vpaddd    %ymm15,%ymm0,%ymm20
	vpandq    vecmask23(%rip),%ymm8,%ymm24

	// <T10',T9',T12',T11'> ← Dense-Shuffle(<T9',T10',T11',T12'>)
	vpshufd   $78,%ymm20,%ymm4
	vpshufd   $78,%ymm21,%ymm5
	vpshufd   $78,%ymm22,%ymm6
	vpshufd   $78,%ymm23,%ymm7
	vpshufd   $78,%ymm24,%ymm9

	// <T10',A',1',X1'> ← Dense-Blend(<0',A',1',X1'>,<T10',T9',T12',T11'>,1000)
	movl      $252,%ebx
	kmovw     %ebx,%k1	
	vpblendmd 0(%rsi),%ymm4,%ymm16{%k1}
	vpblendmd 32(%rsi),%ymm5,%ymm17{%k1}
	vpblendmd 64(%rsi),%ymm6,%ymm18{%k1}
	vpblendmd 96(%rsi),%ymm7,%ymm19{%k1}
	vpblendmd 128(%rsi),%ymm9,%ymm9{%k1}

	// <T10,A,1,X1> ← Pack-D2N(<T10',A',1',X1'>)
	vpsrlq    $32,%ymm16,%ymm10
	vpsrlq    $32,%ymm17,%ymm11
	vpsrlq    $32,%ymm18,%ymm12
	vpsrlq    $32,%ymm19,%ymm13

	vmovdqa   %ymm10,192(%rsp)
	vmovdqa   %ymm11,224(%rsp)
	vmovdqa   %ymm12,256(%rsp)
	vmovdqa   %ymm13,288(%rsp)
	vmovdqa   %ymm9,320(%rsp)

	// <T9,T10,T11,T12> ← Pack-D2N(<T9',T10',T11',T12'>)
	vpsrlq    $32,%ymm20,%ymm4
	vpsrlq    $32,%ymm21,%ymm5
	vpsrlq    $32,%ymm22,%ymm6
	vpsrlq    $32,%ymm23,%ymm7

	// <T13,T14,T15,T16> ← Sqr(<T9,T10,T11,T12>)
	vpmuludq  %ymm21,%ymm24,%ymm15
	vpmuludq  %ymm22,%ymm7,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm23,%ymm6,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm4,%ymm5,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpsllq    $1,%ymm15,%ymm9

	vpmuludq  %ymm22,%ymm24,%ymm15
	vpmuludq  %ymm23,%ymm7,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm4,%ymm6,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpsllq    $1,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm5,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm10

	vpsrlq    $29,%ymm9,%ymm14
	vpaddq    %ymm14,%ymm10,%ymm10
	vpand     vecmask29(%rip),%ymm9,%ymm9

	vpmuludq  vec1216(%rip),%ymm9,%ymm9
	vpmuludq  %ymm20,%ymm20,%ymm15
	vpaddq    %ymm15,%ymm9,%ymm9

	vpmuludq  %ymm23,%ymm24,%ymm15
	vpmuludq  %ymm4,%ymm7,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm6,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpsllq    $1,%ymm15,%ymm11

	vpsrlq    $29,%ymm10,%ymm14
	vpaddq    %ymm14,%ymm11,%ymm11
	vpand     vecmask29(%rip),%ymm10,%ymm10

	vpmuludq  vec1216(%rip),%ymm10,%ymm10
	vpmuludq  %ymm20,%ymm21,%ymm15
	vpsllq    $1,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm10,%ymm10
	vpmuludq  %ymm4,%ymm24,%ymm15
	vpmuludq  %ymm5,%ymm7,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpsllq    $1,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm6,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm12

	vpsrlq    $29,%ymm11,%ymm14
	vpaddq    %ymm14,%ymm12,%ymm12
	vpand     vecmask29(%rip),%ymm11,%ymm11
	vpmuludq  vec1216(%rip),%ymm11,%ymm11
	vpmuludq  %ymm20,%ymm22,%ymm15
	vpsllq    $1,%ymm15,%ymm15
	vpmuludq  %ymm21,%ymm21,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm11,%ymm11
	vpmuludq  %ymm5,%ymm24,%ymm15
	vpmuludq  %ymm6,%ymm7,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpsllq    $1,%ymm15,%ymm13

	vpsrlq    $29,%ymm12,%ymm14
	vpaddq    %ymm14,%ymm13,%ymm13
	vpand     vecmask29(%rip),%ymm12,%ymm12

	vpmuludq  vec1216(%rip),%ymm12,%ymm12
	vpmuludq  %ymm20,%ymm23,%ymm15
	vpmuludq  %ymm21,%ymm22,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpsllq    $1,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm12,%ymm12

	vmovdqa   %ymm11,64(%rsp)

	vpmuludq  %ymm6,%ymm24,%ymm15
	vpsllq    $1,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm7,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm14

	vpsrlq    $29,%ymm13,%ymm15
	vpaddq    %ymm15,%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm13,%ymm13

	vpmuludq  vec1216(%rip),%ymm13,%ymm13
	vpmuludq  %ymm20,%ymm4,%ymm15
	vpmuludq  %ymm21,%ymm23,%ymm11
	vpaddq    %ymm11,%ymm15,%ymm15
	vpsllq    $1,%ymm15,%ymm15
	vpmuludq  %ymm22,%ymm22,%ymm11
	vpaddq    %ymm11,%ymm15,%ymm15
	vpaddq    %ymm15,%ymm13,%ymm13

	vmovdqa   %ymm12,96(%rsp)

	vpmuludq  %ymm7,%ymm24,%ymm15
	vpsllq    $1,%ymm15,%ymm15

	vpsrlq    $29,%ymm14,%ymm11
	vpaddq    %ymm11,%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm14

	vpmuludq  vec1216(%rip),%ymm14,%ymm14
	vpmuludq  %ymm20,%ymm5,%ymm12
	vpmuludq  %ymm21,%ymm4,%ymm11
	vpaddq    %ymm11,%ymm12,%ymm12
	vpmuludq  %ymm22,%ymm23,%ymm11
	vpaddq    %ymm11,%ymm12,%ymm12
	vpsllq    $1,%ymm12,%ymm12
	vpaddq    %ymm12,%ymm14,%ymm14

	vmovdqa   %ymm13,128(%rsp)

	vpmuludq  %ymm24,%ymm24,%ymm11

	vpsrlq    $29,%ymm15,%ymm12
	vpaddq    %ymm12,%ymm11,%ymm11
	vpand     vecmask29(%rip),%ymm15,%ymm15

	vpmuludq  vec1216(%rip),%ymm15,%ymm15
	vpmuludq  %ymm20,%ymm6,%ymm12
	vpmuludq  %ymm21,%ymm5,%ymm13
	vpaddq    %ymm12,%ymm13,%ymm13
	vpmuludq  %ymm22,%ymm4,%ymm12
	vpaddq    %ymm12,%ymm13,%ymm13
	vpsllq    $1,%ymm13,%ymm13
	vpmuludq  %ymm23,%ymm23,%ymm12
	vpaddq    %ymm12,%ymm13,%ymm13
	vpaddq    %ymm13,%ymm15,%ymm15

	vmovdqa   %ymm14,160(%rsp)

	vpsrlq    $29,%ymm11,%ymm12
	vpand     vecmask29(%rip),%ymm11,%ymm11

	vpmuludq  vec1216(%rip),%ymm11,%ymm11
	vpmuludq  %ymm20,%ymm7,%ymm13
	vpmuludq  %ymm21,%ymm6,%ymm14
	vpaddq    %ymm13,%ymm14,%ymm14
	vpmuludq  %ymm22,%ymm5,%ymm13
	vpaddq    %ymm13,%ymm14,%ymm14
	vpmuludq  %ymm23,%ymm4,%ymm13
	vpaddq    %ymm13,%ymm14,%ymm14
	vpsllq    $1,%ymm14,%ymm14
	vpaddq    %ymm14,%ymm11,%ymm11

	vpmuludq  vec1216(%rip),%ymm12,%ymm12
	vpmuludq  %ymm20,%ymm24,%ymm13
	vpmuludq  %ymm21,%ymm7,%ymm14
	vpaddq    %ymm13,%ymm14,%ymm14
	vpmuludq  %ymm22,%ymm6,%ymm13
	vpaddq    %ymm13,%ymm14,%ymm14
	vpmuludq  %ymm23,%ymm5,%ymm13
	vpaddq    %ymm13,%ymm14,%ymm14
	vpsllq    $1,%ymm14,%ymm14
	vpmuludq  %ymm4,%ymm4,%ymm13
	vpaddq    %ymm13,%ymm14,%ymm14
	vpaddq    %ymm14,%ymm12,%ymm12

	vmovdqa   128(%rsp),%ymm4
	vpsrlq    $29,%ymm4,%ymm5
	vpaddq    160(%rsp),%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm9,%ymm14
	vpaddq    %ymm14,%ymm10,%ymm1
	vpand     vecmask29(%rip),%ymm9,%ymm0

	vpsrlq    $29,%ymm5,%ymm6
	vpaddq    %ymm15,%ymm6,%ymm6
	vpand     vecmask29(%rip),%ymm5,%ymm5

	vpsrlq    $29,%ymm1,%ymm14
	vpaddq    64(%rsp),%ymm14,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1

	vpsrlq    $29,%ymm6,%ymm7
	vpaddq    %ymm11,%ymm7,%ymm7
	vpand     vecmask29(%rip),%ymm6,%ymm6

	vpsrlq    $29,%ymm2,%ymm14
	vpaddq    96(%rsp),%ymm14,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2

	vpsrlq    $29,%ymm7,%ymm8
	vpaddq    %ymm12,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	vpsrlq    $29,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm4,%ymm4
	vpand     vecmask29(%rip),%ymm3,%ymm3

	vpsrlq    $23,%ymm8,%ymm14
	vpaddq    %ymm14,%ymm0,%ymm0
	vpsllq    $1,%ymm14,%ymm14
	vpaddq    %ymm14,%ymm0,%ymm0
	vpsllq    $3,%ymm14,%ymm14
	vpaddq    %ymm14,%ymm0,%ymm0
	vpand     vecmask23(%rip),%ymm8,%ymm8

	vpsrlq    $29,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm0,%ymm14
	vpaddq    %ymm14,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	// <T13',T14',T15',T16'> ← Pack-N2D(<T13,T14,T15,T16>)
	vpsllq    $32,%ymm4,%ymm4
	vpor      %ymm4,%ymm0,%ymm10
	vpsllq    $32,%ymm5,%ymm5
	vpor      %ymm5,%ymm1,%ymm11
	vpsllq    $32,%ymm6,%ymm6
	vpor      %ymm6,%ymm2,%ymm12
	vpsllq    $32,%ymm7,%ymm7
	vpor      %ymm7,%ymm3,%ymm13

	// <T14',T13',T16',T15'> ← Dense-Shuffle(<T13',T14',T15',T16'>)
	vpshufd   $78,%ymm10,%ymm0
	vpshufd   $78,%ymm11,%ymm1
	vpshufd   $78,%ymm12,%ymm2
	vpshufd   $78,%ymm13,%ymm3
	vpshufd   $78,%ymm8,%ymm4

	// <X2',*,*,*> ← Dense-Sub(<T13',T14',T15',T16'>,<T14',T13',T16',T15'>)
	vpaddd	  sub_p1(%rip),%ymm10,%ymm5
	vpaddd	  sub_p2(%rip),%ymm11,%ymm6
	vpaddd	  sub_p2(%rip),%ymm12,%ymm7
	vpaddd	  sub_p2(%rip),%ymm13,%ymm9
	vpaddd	  sub_p3(%rip),%ymm8,%ymm14
	vpsubd	  %ymm0,%ymm5,%ymm5
	vpsubd	  %ymm1,%ymm6,%ymm6
	vpsubd	  %ymm2,%ymm7,%ymm7
	vpsubd	  %ymm3,%ymm9,%ymm9
	vpsubd	  %ymm4,%ymm14,%ymm14

	vmovdqa   %ymm5,448(%rsp)
	vmovdqa   %ymm6,480(%rsp)
	vmovdqa   %ymm7,512(%rsp)
	vmovdqa   %ymm9,544(%rsp)
	vmovdqa   %ymm14,576(%rsp)

	// <T9',T14',T15',T16'> ← Dense-Blend(<T9',T10',T11',T12'>,<T13',T14',T15',T16'>,0111)
	movl      $3,%ebx
	kmovw     %ebx,%k1
	vpblendmd %ymm20,%ymm10,%ymm10{%k1}
	vpblendmd %ymm21,%ymm11,%ymm11{%k1}
	vpblendmd %ymm22,%ymm12,%ymm12{%k1}
	vpblendmd %ymm23,%ymm13,%ymm13{%k1}
	vpblendmd %ymm24,%ymm8,%ymm4{%k1}

	// <T9,T14,T15,T16> ← Pack-D2N(<T9',T14',T15',T16'>)
	vpsrlq    $32,%ymm10,%ymm0
	vpsrlq    $32,%ymm11,%ymm1
	vpsrlq    $32,%ymm12,%ymm2
	vpsrlq    $32,%ymm13,%ymm3

	// <T17,T18,X3,Z3> ← Mul(<T10,A,1,X1>,<T9,T14,T15,T16>)
	vmovdqa   192(%rsp),%ymm5
	vmovdqa   224(%rsp),%ymm6
	vmovdqa   256(%rsp),%ymm7
	vmovdqa   288(%rsp),%ymm8
	vmovdqa   320(%rsp),%ymm9

	vpmuludq  %ymm5,%ymm0,%ymm20

	vpmuludq  %ymm6,%ymm0,%ymm15
	vpmuludq  %ymm5,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm21

	vpmuludq  %ymm7,%ymm0,%ymm15
	vpmuludq  %ymm6,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm22

	vpmuludq  %ymm8,%ymm0,%ymm15
	vpmuludq  %ymm7,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm23

	vpmuludq  %ymm9,%ymm0,%ymm15
	vpmuludq  %ymm8,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm5,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm24

	vpmuludq  %ymm9,%ymm1,%ymm15
	vpmuludq  %ymm8,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm6,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm25

	vpmuludq  %ymm9,%ymm2,%ymm15
	vpmuludq  %ymm8,%ymm3,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm7,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm26

	vpmuludq  %ymm9,%ymm3,%ymm15
	vpmuludq  %ymm8,%ymm4,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm27

	vpmuludq  %ymm9,%ymm4,%ymm28

	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm11,%ymm1,%ymm1
	vpaddq    %ymm12,%ymm2,%ymm2
	vpaddq    %ymm13,%ymm3,%ymm3
	vpaddq    %ymm16,%ymm5,%ymm5
	vpaddq    %ymm17,%ymm6,%ymm6
	vpaddq    %ymm18,%ymm7,%ymm7
	vpaddq    %ymm19,%ymm8,%ymm8

	vpmuludq  %ymm16,%ymm10,%ymm29
	vpaddq    %ymm20,%ymm29,%ymm15
	vmovdqa   %ymm15,192(%rsp)

	vpmuludq  %ymm17,%ymm10,%ymm15
	vpmuludq  %ymm16,%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm30
	vpaddq    %ymm21,%ymm30,%ymm15
	vmovdqa   %ymm15,224(%rsp)

	vpmuludq  %ymm18,%ymm10,%ymm15
	vpmuludq  %ymm17,%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm16,%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm31
	vpaddq    %ymm22,%ymm31,%ymm15
	vmovdqa   %ymm15,256(%rsp)

	vpmuludq  %ymm19,%ymm10,%ymm15
	vpmuludq  %ymm18,%ymm11,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm17,%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm16,%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,64(%rsp)
	vpaddq    %ymm23,%ymm15,%ymm15
	vmovdqa   %ymm15,288(%rsp)

	vpmuludq  %ymm19,%ymm11,%ymm15
	vpmuludq  %ymm18,%ymm12,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vpmuludq  %ymm17,%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,96(%rsp)
	vpaddq    %ymm24,%ymm15,%ymm15
	vmovdqa   %ymm15,320(%rsp)

	vpmuludq  %ymm19,%ymm12,%ymm15
	vpmuludq  %ymm18,%ymm13,%ymm14
	vpaddq    %ymm14,%ymm15,%ymm15
	vmovdqa   %ymm15,128(%rsp)
	vpaddq    %ymm25,%ymm15,%ymm15
	vmovdqa   %ymm15,352(%rsp)

	vpmuludq  %ymm19,%ymm13,%ymm15
	vmovdqa   %ymm15,160(%rsp)
	vpaddq    %ymm26,%ymm15,%ymm15
	vmovdqa   %ymm15,384(%rsp)

	vpmuludq  %ymm5,%ymm0,%ymm15
	vmovdqa   %ymm15,416(%rsp)

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

	vmovdqa   416(%rsp),%ymm9

	vpsubq    192(%rsp),%ymm9,%ymm9
	vpaddq    96(%rsp),%ymm9,%ymm9
	vpsubq    224(%rsp),%ymm10,%ymm10
	vpaddq    128(%rsp),%ymm10,%ymm10
	vpsubq    256(%rsp),%ymm11,%ymm11
	vpaddq    160(%rsp),%ymm11,%ymm11
	vpsubq    288(%rsp),%ymm12,%ymm12
	vpsubq    320(%rsp),%ymm13,%ymm13
	vpaddq    %ymm20,%ymm13,%ymm13
	vpsubq    352(%rsp),%ymm0,%ymm0
	vpaddq    %ymm21,%ymm0,%ymm0
	vpsubq    384(%rsp),%ymm1,%ymm1
	vpaddq    %ymm22,%ymm1,%ymm1
	vpsubq    %ymm27,%ymm2,%ymm2
	vpaddq    %ymm23,%ymm2,%ymm2
	vpsubq    %ymm28,%ymm3,%ymm3
	vpaddq    %ymm24,%ymm3,%ymm3

	vpsrlq    $29,%ymm0,%ymm14
	vpaddq    %ymm14,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0
	vpmuludq  vec1216(%rip),%ymm0,%ymm0
	vpaddq    %ymm29,%ymm0,%ymm0

	vpsrlq    $29,%ymm1,%ymm14
	vpaddq    %ymm14,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1
	vpmuludq  vec1216(%rip),%ymm1,%ymm1
	vpaddq    %ymm30,%ymm1,%ymm1

	vpsrlq    $29,%ymm2,%ymm14
	vpaddq    %ymm14,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2
	vpmuludq  vec1216(%rip),%ymm2,%ymm2
	vpaddq    %ymm31,%ymm2,%ymm2

	vpsrlq    $29,%ymm3,%ymm14
	vpaddq    %ymm25,%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm3,%ymm3
	vpmuludq  vec1216(%rip),%ymm3,%ymm3
	vpaddq    64(%rsp),%ymm3,%ymm3

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    %ymm26,%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm4
	vpmuludq  vec1216(%rip),%ymm4,%ymm4
	vpaddq    %ymm9,%ymm4,%ymm4

	vpsrlq    $29,%ymm15,%ymm14
	vpaddq    %ymm27,%ymm14,%ymm14
	vpand     vecmask29(%rip),%ymm15,%ymm5
	vpmuludq  vec1216(%rip),%ymm5,%ymm5
	vpaddq    %ymm10,%ymm5,%ymm5

	vpsrlq    $29,%ymm14,%ymm15
	vpaddq    %ymm28,%ymm15,%ymm15
	vpand     vecmask29(%rip),%ymm14,%ymm6
	vpmuludq  vec1216(%rip),%ymm6,%ymm6
	vpaddq    %ymm11,%ymm6,%ymm6

	vpsrlq    $29,%ymm15,%ymm8
	vpand     vecmask29(%rip),%ymm15,%ymm7

	vpmuludq  vec1216(%rip),%ymm7,%ymm7
	vpaddq    %ymm12,%ymm7,%ymm7
	vpmuludq  vec1216(%rip),%ymm8,%ymm8
	vpaddq    %ymm13,%ymm8,%ymm8

	vpsrlq    $29,%ymm4,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	vpsrlq    $29,%ymm5,%ymm10
	vpaddq    %ymm10,%ymm6,%ymm6
	vpand     vecmask29(%rip),%ymm5,%ymm5

	vpsrlq    $29,%ymm1,%ymm10
	vpaddq    %ymm10,%ymm2,%ymm2
	vpand     vecmask29(%rip),%ymm1,%ymm1

	vpsrlq    $29,%ymm6,%ymm10
	vpaddq    %ymm10,%ymm7,%ymm7
	vpand     vecmask29(%rip),%ymm6,%ymm6

	vpsrlq    $29,%ymm2,%ymm10
	vpaddq    %ymm10,%ymm3,%ymm3
	vpand     vecmask29(%rip),%ymm2,%ymm2

	vpsrlq    $29,%ymm7,%ymm10
	vpaddq    %ymm10,%ymm8,%ymm8
	vpand     vecmask29(%rip),%ymm7,%ymm7

	vpsrlq    $29,%ymm3,%ymm10
	vpaddq    %ymm10,%ymm4,%ymm9
	vpand     vecmask29(%rip),%ymm3,%ymm3

	vpsrlq    $23,%ymm8,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpaddq    %ymm10,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpsllq    $3,%ymm10,%ymm10
	vpaddq    %ymm10,%ymm0,%ymm0
	vpand     vecmask23(%rip),%ymm8,%ymm4

	vpsrlq    $29,%ymm9,%ymm10
	vpaddq    %ymm10,%ymm5,%ymm5
	vpand     vecmask29(%rip),%ymm9,%ymm9

	vpsrlq    $29,%ymm0,%ymm10
	vpaddq    %ymm10,%ymm1,%ymm1
	vpand     vecmask29(%rip),%ymm0,%ymm0

	// <T17',T18',X3',Z3'> ← Pack-N2D(<T17,T18,X3,Z3>)
	vpsllq    $32,%ymm9,%ymm9
	vpor      %ymm9,%ymm0,%ymm10
	vpsllq    $32,%ymm5,%ymm5
	vpor      %ymm5,%ymm1,%ymm11
	vpsllq    $32,%ymm6,%ymm6
	vpor      %ymm6,%ymm2,%ymm12
	vpsllq    $32,%ymm7,%ymm7
	vpor      %ymm7,%ymm3,%ymm13

	// <T19',*,*,*> ← Dense-Add(<T17',T18',X3',Z3'>,<T17',T18',X3',Z3'>)
	vpaddd	  %ymm10,%ymm10,%ymm0
	vpaddd	  %ymm11,%ymm11,%ymm1
	vpaddd	  %ymm12,%ymm12,%ymm2
	vpaddd	  %ymm13,%ymm13,%ymm3
	vpaddd	  %ymm4,%ymm4,%ymm5

	// <*,T19',*,*> ← Dense-Shuffle(<T19',*,*,*>)
	vpshufd   $78,%ymm0,%ymm0 
	vpshufd   $78,%ymm1,%ymm1
	vpshufd   $78,%ymm2,%ymm2
	vpshufd   $78,%ymm3,%ymm3
	vpshufd   $78,%ymm5,%ymm5

	// <*,Z2',*,*> ← Dense-Add(<T17',T18',X3',Z3'>,<*,T19',*,*>)
	vpaddd	  %ymm10,%ymm0,%ymm0
	vpaddd	  %ymm11,%ymm1,%ymm1
	vpaddd	  %ymm12,%ymm2,%ymm2
	vpaddd	  %ymm13,%ymm3,%ymm3
	vpaddd	  %ymm4,%ymm5,%ymm5

	// <X2',Z2',*,*> ← Dense-Blend(<X2',*,*,*>,<*,Z2',*,*>,0100)
	vpblendd  $3,448(%rsp),%ymm0,%ymm0
	vpblendd  $3,480(%rsp),%ymm1,%ymm1
	vpblendd  $3,512(%rsp),%ymm2,%ymm2
	vpblendd  $3,544(%rsp),%ymm3,%ymm3
	vpblendd  $3,576(%rsp),%ymm5,%ymm5

	// <X2',Z2',X3',Z3'> ← Dense-Blend(<X2',Z2',*,*>,<T17',T18',X3',Z3'>,0011)
	vpblendd  $15,%ymm0,%ymm10,%ymm10
	vpblendd  $15,%ymm1,%ymm11,%ymm11
	vpblendd  $15,%ymm2,%ymm12,%ymm12
	vpblendd  $15,%ymm3,%ymm13,%ymm13
	vpblendd  $15,%ymm5,%ymm4,%ymm4

	subb      $1,%cl
	cmpb	  $0,%cl
	jge       .L2

	movb	  $7,%cl
	subq      $1,%r15
	cmpq	  $0,%r15
	jge       .L1

	// <X2,Z2,X3,Z3> ← Pack-D2N(<X2',Z2',X3',Z3'>)
	vpsrlq    $32,%ymm10,%ymm0
	vpand     vecmask32(%rip),%ymm10,%ymm10
	vpsrlq    $32,%ymm11,%ymm1
	vpand     vecmask32(%rip),%ymm11,%ymm11
	vpsrlq    $32,%ymm12,%ymm2
	vpand     vecmask32(%rip),%ymm12,%ymm12
	vpsrlq    $32,%ymm13,%ymm3
	vpand     vecmask32(%rip),%ymm13,%ymm13

	// <X2,Z2,X3,Z3> ← Reduce(<X2,Z2,X3,Z3>)
	vpsrlq    $29,%ymm10,%ymm15
	vpaddq    %ymm15,%ymm11,%ymm11
	vpand     vecmask29(%rip),%ymm10,%ymm10

	vpsrlq    $29,%ymm11,%ymm15
	vpaddq    %ymm15,%ymm12,%ymm12
	vpand     vecmask29(%rip),%ymm11,%ymm11

	vpsrlq    $29,%ymm12,%ymm15
	vpaddq    %ymm15,%ymm13,%ymm13
	vpand     vecmask29(%rip),%ymm12,%ymm12

	vpsrlq    $29,%ymm13,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpand     vecmask29(%rip),%ymm13,%ymm13

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

	vpsrlq    $23,%ymm4,%ymm15
	vpmuludq  vec19(%rip),%ymm15,%ymm15
	vpaddq    %ymm15,%ymm10,%ymm10
	vpand     vecmask23(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm10,%ymm15
	vpaddq    %ymm15,%ymm11,%ymm11
	vpand     vecmask29(%rip),%ymm10,%ymm10

	// <X2,Z2,X3,Z3> ← Reduce(<X2,Z2,X3,Z3>)
	vpsrlq    $29,%ymm11,%ymm15
	vpaddq    %ymm15,%ymm12,%ymm12
	vpand     vecmask29(%rip),%ymm11,%ymm11

	vpsrlq    $29,%ymm12,%ymm15
	vpaddq    %ymm15,%ymm13,%ymm13
	vpand     vecmask29(%rip),%ymm12,%ymm12

	vpsrlq    $29,%ymm13,%ymm15
	vpaddq    %ymm15,%ymm0,%ymm0
	vpand     vecmask29(%rip),%ymm13,%ymm13

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

	vpsrlq    $23,%ymm4,%ymm15
	vpmuludq  vec19(%rip),%ymm15,%ymm15
	vpaddq    %ymm15,%ymm10,%ymm10
	vpand     vecmask23(%rip),%ymm4,%ymm4

	vpsrlq    $29,%ymm10,%ymm15
	vpaddq    %ymm15,%ymm11,%ymm11
	vpand     vecmask29(%rip),%ymm10,%ymm10

	// store <X2,Z2,X3,Z3>
	vmovdqa   %ymm10,0(%rdi)
	vmovdqa   %ymm11,32(%rdi)
	vmovdqa   %ymm12,64(%rdi)
	vmovdqa   %ymm13,96(%rdi)
	vmovdqa   %ymm0,128(%rdi)
	vmovdqa   %ymm1,160(%rdi)
	vmovdqa   %ymm2,192(%rdi)
	vmovdqa   %ymm3,224(%rdi)
	vmovdqa   %ymm4,256(%rdi)

	movq 	  0(%rsp),%r11
	movq 	  8(%rsp),%r12
	movq 	  16(%rsp),%r13
	movq 	  24(%rsp),%r14
	movq 	  32(%rsp),%r15
	movq 	  40(%rsp),%rbx
	movq 	  48(%rsp),%rbp

	movq 	  %r11,%rsp

	ret
