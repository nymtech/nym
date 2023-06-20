// linker define mladder
// linker use hh1_p1
// linker use hh1_p2
// linker use hh1_p3
// linker use h2h_p1
// linker use h2h_p2
// linker use h2h_p3
// linker use hh1_xor
// linker use h2h_xor
// linker use swap_c
// linker use swap_mask
// linker use h2h_mask
// linker use vec19
// linker use vec608
// linker use vecmask52
// linker use vecmask47
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
subq 	  $64,%rsp

movq 	  %r11,0(%rsp)
movq 	  %r12,8(%rsp)
movq 	  %r13,16(%rsp)
movq 	  %r14,24(%rsp)
movq 	  %r15,32(%rsp)
movq 	  %rbx,40(%rsp)
movq 	  %rbp,48(%rsp)

// load <X2,Z2,X3,Z3>
vmovdqa   0(%rdi),%ymm10
vmovdqa   32(%rdi),%ymm11
vmovdqa   64(%rdi),%ymm12
vmovdqa   96(%rdi),%ymm13
vmovdqa   128(%rdi),%ymm14

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

// <X2,Z2,X3,Z3> ← Swap(<X2,Z2,X3,Z3>,b)
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
vpermd	  %ymm14,%ymm7,%ymm14

// <T1,T2,T4,T3> ← H-H1(<X2,Z2,X3,Z3>)
vpshufd   $68,%ymm10,%ymm5 
vpshufd   $238,%ymm10,%ymm6
vpaddq    hh1_p1(%rip),%ymm5,%ymm5
vpxor     hh1_xor(%rip),%ymm6,%ymm6
vpaddq    %ymm5,%ymm6,%ymm0

vpshufd   $68,%ymm11,%ymm5
vpshufd   $238,%ymm11,%ymm6
vpaddq    hh1_p2(%rip),%ymm5,%ymm5
vpxor     hh1_xor(%rip),%ymm6,%ymm6
vpaddq    %ymm5,%ymm6,%ymm1

vpshufd   $68,%ymm12,%ymm5
vpshufd   $238,%ymm12,%ymm6
vpaddq    hh1_p2(%rip),%ymm5,%ymm5
vpxor     hh1_xor(%rip),%ymm6,%ymm6
vpaddq    %ymm5,%ymm6,%ymm2

vpshufd   $68,%ymm13,%ymm5
vpshufd   $238,%ymm13,%ymm6
vpaddq    hh1_p2(%rip),%ymm5,%ymm5
vpxor     hh1_xor(%rip),%ymm6,%ymm6
vpaddq    %ymm5,%ymm6,%ymm3

vpshufd   $68,%ymm14,%ymm5
vpshufd   $238,%ymm14,%ymm6
vpaddq    hh1_p3(%rip),%ymm5,%ymm5
vpxor     hh1_xor(%rip),%ymm6,%ymm6
vpaddq    %ymm5,%ymm6,%ymm4

vpsrlq    $52,%ymm3,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpandq    vecmask52(%rip),%ymm3,%ymm3

vpsrlq    $47,%ymm4,%ymm10
vpandq    vecmask47(%rip),%ymm4,%ymm4
vpmadd52luq  vec19(%rip),%ymm10,%ymm0

vpsrlq    $52,%ymm0,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpandq    vecmask52(%rip),%ymm0,%ymm0

vpsrlq    $52,%ymm1,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpandq    vecmask52(%rip),%ymm1,%ymm1

vpsrlq    $52,%ymm2,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpandq    vecmask52(%rip),%ymm2,%ymm2

vpsrlq    $52,%ymm3,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpandq    vecmask52(%rip),%ymm3,%ymm3

// <T1,T2,T1,T2> ← Dup(<T1,T2,T4,T3>)
vpermq	  $68,%ymm0,%ymm5
vpermq	  $68,%ymm1,%ymm6
vpermq	  $68,%ymm2,%ymm7
vpermq	  $68,%ymm3,%ymm8
vpermq	  $68,%ymm4,%ymm9

// <T5,T6,T7,T8> ← Mul(<T1,T2,T4,T3>,<T1,T2,T1,T2>)
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

vpmadd52luq  vec608(%rip),%ymm19,%ymm14

// <T9,T10,T11,T12> ← H2-H(<T5,T6,T7,T8>)
vpshufd   $68,%ymm10,%ymm1
vpand     h2h_mask(%rip),%ymm1,%ymm1
vpshufd   $238,%ymm10,%ymm3
vpaddq    h2h_p1(%rip),%ymm1,%ymm1
vpxor     h2h_xor(%rip),%ymm3,%ymm3
vpaddq    %ymm1,%ymm3,%ymm5

vpshufd   $68,%ymm11,%ymm1
vpand     h2h_mask(%rip),%ymm1,%ymm1
vpshufd   $238,%ymm11,%ymm3
vpaddq    h2h_p2(%rip),%ymm1,%ymm1
vpxor     h2h_xor(%rip),%ymm3,%ymm3
vpaddq    %ymm1,%ymm3,%ymm6

vpshufd   $68,%ymm12,%ymm1
vpand     h2h_mask(%rip),%ymm1,%ymm1
vpshufd   $238,%ymm12,%ymm3
vpaddq    h2h_p2(%rip),%ymm1,%ymm1
vpxor     h2h_xor(%rip),%ymm3,%ymm3
vpaddq    %ymm1,%ymm3,%ymm7

vpshufd   $68,%ymm13,%ymm1
vpand     h2h_mask(%rip),%ymm1,%ymm1
vpshufd   $238,%ymm13,%ymm3
vpaddq    h2h_p2(%rip),%ymm1,%ymm1
vpxor     h2h_xor(%rip),%ymm3,%ymm3
vpaddq    %ymm1,%ymm3,%ymm8

vpshufd   $68,%ymm14,%ymm1
vpand     h2h_mask(%rip),%ymm1,%ymm1
vpshufd   $238,%ymm14,%ymm3
vpaddq    h2h_p3(%rip),%ymm1,%ymm1
vpxor     h2h_xor(%rip),%ymm3,%ymm3
vpaddq    %ymm1,%ymm3,%ymm9

vpsrlq    $52,%ymm8,%ymm21
vpaddq    %ymm21,%ymm9,%ymm9
vpandq    vecmask52(%rip),%ymm8,%ymm8

vpsrlq    $47,%ymm9,%ymm21
vpandq    vecmask47(%rip),%ymm9,%ymm9
vpmadd52luq  vec19(%rip),%ymm21,%ymm5

vpsrlq    $52,%ymm5,%ymm21
vpaddq    %ymm21,%ymm6,%ymm6
vpandq    vecmask52(%rip),%ymm5,%ymm5

vpsrlq    $52,%ymm6,%ymm21
vpaddq    %ymm21,%ymm7,%ymm7
vpandq    vecmask52(%rip),%ymm6,%ymm6

vpsrlq    $52,%ymm7,%ymm21
vpaddq    %ymm21,%ymm8,%ymm8
vpandq    vecmask52(%rip),%ymm7,%ymm7

vpsrlq    $52,%ymm8,%ymm21
vpaddq    %ymm21,%ymm9,%ymm9
vpandq    vecmask52(%rip),%ymm8,%ymm8

// <T9,T10,1,X1> ← Blend(<0,0,1,X1>,<T9,T10,T11,T12>,1100)
vpblendd  $240,0(%rsi),%ymm5,%ymm0
vpblendd  $240,32(%rsi),%ymm6,%ymm1
vpblendd  $240,64(%rsi),%ymm7,%ymm2
vpblendd  $240,96(%rsi),%ymm8,%ymm3
vpblendd  $240,128(%rsi),%ymm9,%ymm4

// <0,T13,0,0> ← Unreduced-Mulc(<T9,T10,T11,T12>,<0,a24,0,0>)
// <T5,T14,T7,T8> ← Add(<0,T13,0,0>,<T5,T6,T7,T8>)
vpxorq    %ymm15,%ymm15,%ymm15
vpxorq    %ymm21,%ymm21,%ymm21
vpxorq    %ymm22,%ymm22,%ymm22
vpxorq    %ymm23,%ymm23,%ymm23

vpmadd52luq  a24(%rip),%ymm5,%ymm10
vpmadd52huq  a24(%rip),%ymm5,%ymm11

vpmadd52luq  a24(%rip),%ymm6,%ymm21
vpmadd52huq  a24(%rip),%ymm6,%ymm12

vpaddq    %ymm21,%ymm11,%ymm11

vpmadd52luq  a24(%rip),%ymm7,%ymm22
vpmadd52huq  a24(%rip),%ymm7,%ymm13

vpaddq    %ymm22,%ymm12,%ymm12

vpmadd52luq  a24(%rip),%ymm8,%ymm23
vpmadd52huq  a24(%rip),%ymm8,%ymm14

vpaddq    %ymm23,%ymm13,%ymm13

vpmadd52luq  a24(%rip),%ymm9,%ymm14
vpmadd52huq  a24(%rip),%ymm9,%ymm15

vpmadd52luq  vec608(%rip),%ymm15,%ymm10

vpsrlq    $52,%ymm13,%ymm21
vpaddq    %ymm21,%ymm14,%ymm14
vpandq    vecmask52(%rip),%ymm13,%ymm13

vpsrlq    $47,%ymm14,%ymm21
vpandq    vecmask47(%rip),%ymm14,%ymm14
vpmadd52luq  vec19(%rip),%ymm21,%ymm10

vpsrlq    $52,%ymm10,%ymm21
vpaddq    %ymm21,%ymm11,%ymm11
vpandq    vecmask52(%rip),%ymm10,%ymm10

vpsrlq    $52,%ymm11,%ymm21
vpaddq    %ymm21,%ymm12,%ymm12
vpandq    vecmask52(%rip),%ymm11,%ymm11

vpsrlq    $52,%ymm12,%ymm21
vpaddq    %ymm21,%ymm13,%ymm13
vpandq    vecmask52(%rip),%ymm12,%ymm12

vpsrlq    $52,%ymm13,%ymm21
vpaddq    %ymm21,%ymm14,%ymm14
vpandq    vecmask52(%rip),%ymm13,%ymm13

// <*,*,T15,T16> ← Sqr(<T9,T10,T11,T12>)
vpxorq    %ymm22,%ymm22,%ymm22
vpxorq    %ymm23,%ymm23,%ymm23
vpxorq    %ymm24,%ymm24,%ymm24
vpxorq    %ymm25,%ymm25,%ymm25
vpxorq    %ymm26,%ymm26,%ymm26
vpxorq    %ymm27,%ymm27,%ymm27
vpxorq    %ymm28,%ymm28,%ymm28
vpxorq    %ymm29,%ymm29,%ymm29
vpxorq    %ymm30,%ymm30,%ymm30
vpxorq    %ymm31,%ymm31,%ymm31

vpmadd52luq  %ymm5,%ymm5,%ymm22
vpmadd52huq  %ymm5,%ymm5,%ymm23

vpxorq       %ymm21,%ymm21,%ymm21
vpmadd52luq  %ymm5,%ymm6,%ymm21
vpmadd52huq  %ymm5,%ymm6,%ymm24
vpaddq       %ymm21,%ymm21,%ymm21
vpaddq       %ymm21,%ymm23,%ymm23
vpaddq       %ymm24,%ymm24,%ymm24

vpxorq       %ymm21,%ymm21,%ymm21
vpmadd52luq  %ymm5,%ymm7,%ymm21
vpmadd52huq  %ymm5,%ymm7,%ymm25
vpaddq       %ymm21,%ymm21,%ymm21
vpaddq       %ymm21,%ymm24,%ymm24
vpaddq       %ymm25,%ymm25,%ymm25
vpmadd52luq  %ymm6,%ymm6,%ymm24
vpmadd52huq  %ymm6,%ymm6,%ymm25

vpxorq       %ymm21,%ymm21,%ymm21
vpmadd52luq  %ymm5,%ymm8,%ymm21
vpmadd52huq  %ymm5,%ymm8,%ymm26
vpmadd52luq  %ymm6,%ymm7,%ymm21
vpmadd52huq  %ymm6,%ymm7,%ymm26
vpaddq       %ymm21,%ymm21,%ymm21
vpaddq       %ymm21,%ymm25,%ymm25
vpaddq       %ymm26,%ymm26,%ymm26

vpxorq       %ymm21,%ymm21,%ymm21
vpmadd52luq  %ymm5,%ymm9,%ymm21
vpmadd52huq  %ymm5,%ymm9,%ymm27
vpmadd52luq  %ymm6,%ymm8,%ymm21
vpmadd52huq  %ymm6,%ymm8,%ymm27
vpaddq       %ymm21,%ymm21,%ymm21
vpaddq       %ymm21,%ymm26,%ymm26
vpaddq       %ymm27,%ymm27,%ymm27
vpmadd52luq  %ymm7,%ymm7,%ymm26
vpmadd52huq  %ymm7,%ymm7,%ymm27

vpxorq       %ymm21,%ymm21,%ymm21
vpmadd52luq  %ymm6,%ymm9,%ymm21
vpmadd52huq  %ymm6,%ymm9,%ymm28
vpmadd52luq  %ymm7,%ymm8,%ymm21
vpmadd52huq  %ymm7,%ymm8,%ymm28
vpaddq       %ymm21,%ymm21,%ymm21
vpaddq       %ymm21,%ymm27,%ymm27
vpaddq       %ymm28,%ymm28,%ymm28

vpxorq       %ymm21,%ymm21,%ymm21
vpmadd52luq  %ymm7,%ymm9,%ymm21
vpmadd52huq  %ymm7,%ymm9,%ymm29
vpaddq       %ymm21,%ymm21,%ymm21
vpaddq       %ymm21,%ymm28,%ymm28
vpaddq       %ymm29,%ymm29,%ymm29
vpmadd52luq  %ymm8,%ymm8,%ymm28
vpmadd52huq  %ymm8,%ymm8,%ymm29

vpxorq       %ymm21,%ymm21,%ymm21
vpmadd52luq  %ymm8,%ymm9,%ymm21
vpmadd52huq  %ymm8,%ymm9,%ymm30
vpaddq       %ymm21,%ymm21,%ymm21
vpaddq       %ymm21,%ymm29,%ymm29
vpaddq       %ymm30,%ymm30,%ymm30

vpmadd52luq  %ymm9,%ymm9,%ymm30
vpmadd52huq  %ymm9,%ymm9,%ymm31

vpsrlq    $52,%ymm27,%ymm21
vpaddq    %ymm21,%ymm28,%ymm28
vpandq    vecmask52(%rip),%ymm27,%ymm27
vpmadd52luq  vec608(%rip),%ymm27,%ymm22
vpmadd52huq  vec608(%rip),%ymm27,%ymm23

vpsrlq    $52,%ymm28,%ymm21
vpaddq    %ymm21,%ymm29,%ymm29
vpandq    vecmask52(%rip),%ymm28,%ymm28
vpmadd52luq  vec608(%rip),%ymm28,%ymm23
vpmadd52huq  vec608(%rip),%ymm28,%ymm24

vpsrlq    $52,%ymm29,%ymm21
vpaddq    %ymm21,%ymm30,%ymm30
vpandq    vecmask52(%rip),%ymm29,%ymm29
vpmadd52luq  vec608(%rip),%ymm29,%ymm24
vpmadd52huq  vec608(%rip),%ymm29,%ymm25

vpsrlq    $52,%ymm30,%ymm21
vpaddq    %ymm21,%ymm31,%ymm31
vpandq    vecmask52(%rip),%ymm30,%ymm30
vpmadd52luq  vec608(%rip),%ymm30,%ymm25
vpmadd52huq  vec608(%rip),%ymm30,%ymm26

vpmadd52luq  vec608(%rip),%ymm31,%ymm26

vpsrlq    $52,%ymm25,%ymm21
vpaddq    %ymm21,%ymm26,%ymm26
vpandq    vecmask52(%rip),%ymm25,%ymm25

vpsrlq    $47,%ymm26,%ymm21
vpandq    vecmask47(%rip),%ymm26,%ymm26
vpmadd52luq  vec19(%rip),%ymm21,%ymm22

vpsrlq    $52,%ymm22,%ymm21
vpaddq    %ymm21,%ymm23,%ymm23
vpandq    vecmask52(%rip),%ymm22,%ymm5

vpsrlq    $52,%ymm23,%ymm21
vpaddq    %ymm21,%ymm24,%ymm24
vpandq    vecmask52(%rip),%ymm23,%ymm6

vpsrlq    $52,%ymm24,%ymm21
vpaddq    %ymm21,%ymm25,%ymm25
vpandq    vecmask52(%rip),%ymm24,%ymm7

vpsrlq    $52,%ymm25,%ymm21
vpaddq    %ymm21,%ymm26,%ymm9
vpandq    vecmask52(%rip),%ymm25,%ymm8

// <T5,T14,T15,T16> ← Blend(<T5,T14,T7,T8>,<*,*,T15,T16>,0011)
vpblendd  $15,%ymm10,%ymm5,%ymm5
vpblendd  $15,%ymm11,%ymm6,%ymm6
vpblendd  $15,%ymm12,%ymm7,%ymm7
vpblendd  $15,%ymm13,%ymm8,%ymm8
vpblendd  $15,%ymm14,%ymm9,%ymm9

// <X2,Z2,X3,Z3> ← Mul(<T5,T14,T15,T16>,<T9,T10,1,X1>)
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

vpmadd52luq  vec608(%rip),%ymm19,%ymm14

subb      $1,%cl
cmpb      $0,%cl
jge       .L2

movb      $7,%cl
subq      $1,%r15
cmpq      $0,%r15
jge       .L1

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

// store <X2,Z2,X3,Z3>
vmovdqa   %ymm10,0(%rdi)
vmovdqa   %ymm11,32(%rdi)
vmovdqa   %ymm12,64(%rdi)
vmovdqa   %ymm13,96(%rdi)
vmovdqa   %ymm14,128(%rdi)

movq 	  0(%rsp), %r11
movq 	  8(%rsp), %r12
movq 	  16(%rsp),%r13
movq 	  24(%rsp),%r14
movq 	  32(%rsp),%r15
movq 	  40(%rsp),%rbx
movq 	  48(%rsp),%rbp

movq 	  %r11,%rsp

ret
