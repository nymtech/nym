// linker define mladder
// linker use hh_p1
// linker use hh_p2
// linker use hh_p3
// linker use hh_xor
// linker use sub_p1
// linker use sub_p2
// linker use sub_p3
// linker use swap_c
// linker use swap_mask
// linker use sh2526
// linker use sh2625
// linker use vec19
// linker use vec38
// linker use vecmask25
// linker use vecmask26
// linker use vecmask32
// linker use vecmask2526
// linker use vecmask2625

/* Assembly for Montgomery ladder. */

#include "consts_namespace.h"

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(mladder)
.globl CRYPTO_SHARED_NAMESPACE(mladder)
_CRYPTO_SHARED_NAMESPACE(mladder):
CRYPTO_SHARED_NAMESPACE(mladder):

movq 	  %rsp,%r11
andq 	  $-32,%rsp
subq 	  $864,%rsp

movq 	  %r11,0(%rsp)
movq 	  %r12,8(%rsp)
movq 	  %r13,16(%rsp)
movq 	  %r14,24(%rsp)
movq 	  %r15,32(%rsp)
movq 	  %rbx,40(%rsp)
movq 	  %rbp,48(%rsp)

// load <0,A,1,X1>
vmovdqa   0(%rsi),%ymm0
vmovdqa   32(%rsi),%ymm1
vmovdqa   64(%rsi),%ymm2
vmovdqa   96(%rsi),%ymm3
vmovdqa   128(%rsi),%ymm4
vmovdqa   160(%rsi),%ymm5
vmovdqa   192(%rsi),%ymm6
vmovdqa   224(%rsi),%ymm7
vmovdqa   256(%rsi),%ymm8
vmovdqa   288(%rsi),%ymm9

// <0',A',1',X1'> ← Pack-D2N(<0,A,1,X1>)
vpsllq    $32,%ymm5,%ymm5
vpor      %ymm5,%ymm0,%ymm0
vpsllq    $32,%ymm6,%ymm6
vpor      %ymm6,%ymm1,%ymm1
vpsllq    $32,%ymm7,%ymm7
vpor      %ymm7,%ymm2,%ymm2
vpsllq    $32,%ymm8,%ymm8
vpor      %ymm8,%ymm3,%ymm3
vpsllq    $32,%ymm9,%ymm9
vpor      %ymm9,%ymm4,%ymm4

vmovdqa   %ymm0,0(%rsi)
vmovdqa   %ymm1,32(%rsi)
vmovdqa   %ymm2,64(%rsi)
vmovdqa   %ymm3,96(%rsi)
vmovdqa   %ymm4,128(%rsi)

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
vpsllq    $32,%ymm13,%ymm13
vpor      %ymm13,%ymm0,%ymm0
vpsllq    $32,%ymm6,%ymm6
vpor      %ymm6,%ymm11,%ymm11
vpsllq    $32,%ymm14,%ymm14
vpor      %ymm14,%ymm2,%ymm2
vpsllq    $32,%ymm8,%ymm8
vpor      %ymm8,%ymm12,%ymm12
vpsllq    $32,%ymm15,%ymm15
vpor      %ymm15,%ymm4,%ymm4

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
vpermd	  %ymm11,%ymm7,%ymm11
vpermd	  %ymm2,%ymm7,%ymm2
vpermd	  %ymm12,%ymm7,%ymm12
vpermd	  %ymm4,%ymm7,%ymm4

// <T1',T2',T3',T4'> ← Dense-H-H(<X2',Z2',X3',Z3'>)
vpshufd   $68,%ymm0,%ymm1
vpshufd   $238,%ymm0,%ymm3
vpaddd    hh_p1(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm0

vpshufd   $68,%ymm11,%ymm1
vpshufd   $238,%ymm11,%ymm3
vpaddd    hh_p2(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm11

vpshufd   $68,%ymm2,%ymm1
vpshufd   $238,%ymm2,%ymm3
vpaddd    hh_p3(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm2

vpshufd   $68,%ymm12,%ymm1
vpshufd   $238,%ymm12,%ymm3
vpaddd    hh_p2(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm12

vpshufd   $68,%ymm4,%ymm1
vpshufd   $238,%ymm4,%ymm3
vpaddd    hh_p3(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm4

// <T1,T2,T3,T4> ← Pack-N2D(<T1',T2',T3',T4'>)
vpsrlq    $32,%ymm0,%ymm13
vpsrlq    $32,%ymm11,%ymm6
vpsrlq    $32,%ymm2,%ymm14
vpsrlq    $32,%ymm12,%ymm8
vpsrlq    $32,%ymm4,%ymm15

vmovdqa   %ymm0,64(%rsp)
vmovdqa   %ymm11,96(%rsp)
vmovdqa   %ymm2,128(%rsp)
vmovdqa   %ymm12,160(%rsp)
vmovdqa   %ymm4,192(%rsp)
vmovdqa   %ymm13,224(%rsp)
vmovdqa   %ymm6,256(%rsp)
vmovdqa   %ymm14,288(%rsp)
vmovdqa   %ymm8,320(%rsp)
vmovdqa   %ymm15,352(%rsp)

// <T1',T2',T2',T1'> ← Dense-Dup(<T1',T2',T3',T4'>)
vpermq	  $20,%ymm0,%ymm0
vpermq	  $20,%ymm11,%ymm1
vpermq	  $20,%ymm2,%ymm2
vpermq	  $20,%ymm12,%ymm3
vpermq	  $20,%ymm4,%ymm4

// <T1,T2,T2,T1> ← Pack-D2N(<T1',T2',T2',T1'>)
vpsrlq    $32,%ymm0,%ymm5
vpsrlq    $32,%ymm1,%ymm6
vpsrlq    $32,%ymm2,%ymm7
vpsrlq    $32,%ymm3,%ymm8
vpsrlq    $32,%ymm4,%ymm9

vmovdqa   %ymm0,384(%rsp)
vmovdqa   %ymm1,416(%rsp)
vmovdqa   %ymm2,448(%rsp)
vmovdqa   %ymm3,480(%rsp)
vmovdqa   %ymm4,512(%rsp)
vmovdqa   %ymm5,544(%rsp)
vmovdqa   %ymm6,576(%rsp)
vmovdqa   %ymm7,608(%rsp)
vmovdqa   %ymm8,640(%rsp)
vmovdqa   %ymm9,672(%rsp)

// <T5,T6,T7,T8> ← Mul(<T1,T2,T3,T4>,<T1,T2,T2,T1>)
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
vpmuludq  320(%rsp),%ymm2,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpmuludq  256(%rsp),%ymm4,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpmuludq  192(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpmuludq  128(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0

vpmuludq  352(%rsp),%ymm2,%ymm1
vpmuludq  320(%rsp),%ymm3,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  288(%rsp),%ymm4,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  256(%rsp),%ymm5,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  224(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  192(%rsp),%ymm7,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  160(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  128(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1

vpmuludq  %ymm15,%ymm3,%ymm2
vpmuludq  %ymm14,%ymm5,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  %ymm13,%ymm7,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  %ymm12,%ymm9,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  320(%rsp),%ymm4,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  256(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  192(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  416(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2

vpmuludq  352(%rsp),%ymm4,%ymm3
vpmuludq  320(%rsp),%ymm5,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  288(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  256(%rsp),%ymm7,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  224(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  192(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3

vpmuludq  %ymm15,%ymm5,%ymm4
vpmuludq  %ymm14,%ymm7,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  %ymm13,%ymm9,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  320(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  256(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  416(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  480(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4

vpmuludq  352(%rsp),%ymm6,%ymm5
vpmuludq  320(%rsp),%ymm7,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  288(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  256(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5

vpmuludq  %ymm15,%ymm7,%ymm6
vpmuludq  %ymm14,%ymm9,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  320(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  416(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  480(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  544(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6

vpmuludq  352(%rsp),%ymm8,%ymm7
vpmuludq  320(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7

vpmuludq  %ymm15,%ymm9,%ymm8
vpmuludq  416(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  480(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  544(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  608(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8

vmovdqa   64(%rsp),%ymm11
vmovdqa   96(%rsp),%ymm12
vmovdqa   128(%rsp),%ymm13
vmovdqa   160(%rsp),%ymm14
vmovdqa   192(%rsp),%ymm15
vmovdqa   224(%rsp),%ymm9

vpmuludq  384(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0

vpmuludq  448(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  384(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2

vpmuludq  480(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  448(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  416(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  384(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3

vpmuludq  416(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  384(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1

vpmuludq  512(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  448(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  384(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4

vpmuludq  544(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  512(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  480(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  448(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  416(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  384(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5

vpmuludq  576(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  512(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  448(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6

vpmuludq  608(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  576(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  544(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  512(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  480(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  448(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7

vpmuludq  640(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  576(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  512(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8

vpmuludq  512(%rsp),%ymm9,%ymm9
vpmuludq  672(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  640(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  608(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  576(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  544(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9

vmovdqa   256(%rsp),%ymm11
vmovdqa   288(%rsp),%ymm12
vmovdqa   320(%rsp),%ymm13
vmovdqa   352(%rsp),%ymm14

vpmuludq  384(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6

vpmuludq  416(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  384(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7

vpmuludq  448(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  384(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8

vpmuludq  480(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  448(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  416(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  384(%rsp),%ymm14,%ymm10
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
vpand     vecmask26(%rip),%ymm2,%ymm12

vpsrlq    $26,%ymm8,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpand     vecmask26(%rip),%ymm8,%ymm6

vpsrlq    $25,%ymm3,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpand     vecmask25(%rip),%ymm3,%ymm13

vpsrlq    $25,%ymm9,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpsllq    $1,%ymm10,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpsllq    $3,%ymm10,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpand     vecmask25(%rip),%ymm9,%ymm8

vpsrlq    $26,%ymm4,%ymm10
vpaddq    %ymm10,%ymm15,%ymm15
vpand     vecmask26(%rip),%ymm4,%ymm7

vpsrlq    $26,%ymm0,%ymm10
vpaddq    %ymm10,%ymm1,%ymm3
vpand     vecmask26(%rip),%ymm0,%ymm0

vpsrlq    $25,%ymm15,%ymm10
vpaddq    %ymm10,%ymm11,%ymm5
vpand     vecmask25(%rip),%ymm15,%ymm9

// <T5',T6',T7',T8'> ← Pack-N2D(<T5,T6,T7,T8>)
vpsllq    $32,%ymm9,%ymm9
vpor      %ymm9,%ymm0,%ymm0
vpsllq    $32,%ymm5,%ymm5
vpor      %ymm5,%ymm3,%ymm3
vpsllq    $32,%ymm14,%ymm14
vpor      %ymm14,%ymm12,%ymm12
vpsllq    $32,%ymm6,%ymm6
vpor      %ymm6,%ymm13,%ymm13
vpsllq    $32,%ymm8,%ymm8
vpor      %ymm8,%ymm7,%ymm7

// <T9',T10',T11',T12'> ← Dense-H-H(<T5',T6',T7',T8'>)
vpshufd   $68,%ymm0,%ymm1
vpshufd   $238,%ymm0,%ymm4
vpaddd    hh_p1(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm4,%ymm4
vpaddd    %ymm1,%ymm4,%ymm0

vpshufd   $68,%ymm3,%ymm1
vpshufd   $238,%ymm3,%ymm4
vpaddd    hh_p2(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm4,%ymm4
vpaddd    %ymm1,%ymm4,%ymm3

vpshufd   $68,%ymm12,%ymm1
vpshufd   $238,%ymm12,%ymm4
vpaddd    hh_p3(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm4,%ymm4
vpaddd    %ymm1,%ymm4,%ymm12

vpshufd   $68,%ymm13,%ymm1
vpshufd   $238,%ymm13,%ymm4
vpaddd    hh_p2(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm4,%ymm4
vpaddd    %ymm1,%ymm4,%ymm13

vpshufd   $68,%ymm7,%ymm1
vpshufd   $238,%ymm7,%ymm4
vpaddd    hh_p3(%rip),%ymm1,%ymm1
vpxor     hh_xor(%rip),%ymm4,%ymm4
vpaddd    %ymm1,%ymm4,%ymm7

vmovdqa   %ymm0,640(%rsp)
vmovdqa   %ymm3,672(%rsp)
vmovdqa   %ymm12,704(%rsp)
vmovdqa   %ymm13,736(%rsp)
vmovdqa   %ymm7,768(%rsp)

// <T10',T9',T12',T11'> ← Dense-Shuffle(<T9',T10',T11',T12'>)
vpshufd   $78,%ymm0,%ymm1
vpshufd   $78,%ymm3,%ymm2
vpshufd   $78,%ymm12,%ymm4
vpshufd   $78,%ymm13,%ymm5
vpshufd   $78,%ymm7,%ymm6

// <T10',A',1',X1'> ← Dense-Blend(<0',A',1',X1'>,<T10',T9',T12',T11'>,1000)
vpblendd  $252,0(%rsi),%ymm1,%ymm1
vpblendd  $252,32(%rsi),%ymm2,%ymm2
vpblendd  $252,64(%rsi),%ymm4,%ymm4
vpblendd  $252,96(%rsi),%ymm5,%ymm5
vpblendd  $252,128(%rsi),%ymm6,%ymm6

// <T10,A,1,X1> ← Pack-D2N(<T10',A',1',X1'>)
vpsrlq    $32,%ymm1,%ymm8
vpsrlq    $32,%ymm2,%ymm9
vpsrlq    $32,%ymm4,%ymm10
vpsrlq    $32,%ymm5,%ymm11
vpsrlq    $32,%ymm6,%ymm14

vmovdqa   %ymm1,64(%rsp)
vmovdqa   %ymm2,96(%rsp)
vmovdqa   %ymm4,128(%rsp)
vmovdqa   %ymm5,160(%rsp)
vmovdqa   %ymm6,192(%rsp)
vmovdqa   %ymm8,224(%rsp)
vmovdqa   %ymm9,256(%rsp)
vmovdqa   %ymm10,288(%rsp)
vmovdqa   %ymm11,320(%rsp)
vmovdqa   %ymm14,352(%rsp)

// <T9,T10,T11,T12> ← Pack-D2N(<T9',T10',T11',T12'>)
vpsrlq    $32,%ymm0,%ymm9
vpsrlq    $32,%ymm3,%ymm5
vpsrlq    $32,%ymm12,%ymm14
vpsrlq    $32,%ymm13,%ymm6
vpsrlq    $32,%ymm7,%ymm8

// <T13,T14,T15,T16> ← Sqr(<T9,T10,T11,T12>)
vmovdqa   %ymm12,384(%rsp)
vmovdqa   %ymm13,416(%rsp)
vmovdqa   %ymm7,448(%rsp)
vmovdqa   %ymm9,480(%rsp)
vmovdqa   %ymm5,512(%rsp)
vmovdqa   %ymm14,544(%rsp)
vmovdqa   %ymm6,576(%rsp)
vmovdqa   %ymm8,608(%rsp)

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
vpmuludq  480(%rsp),%ymm1,%ymm15
vpaddq    %ymm15,%ymm0,%ymm0

vpmuludq  512(%rsp),%ymm1,%ymm1
vpmuludq  %ymm3,%ymm10,%ymm15
vpaddq    %ymm15,%ymm1,%ymm1
vpmuludq  384(%rsp),%ymm8,%ymm15
vpaddq    %ymm15,%ymm1,%ymm1
vpmuludq  %ymm13,%ymm6,%ymm15
vpaddq    %ymm15,%ymm1,%ymm1
vpmuludq  448(%rsp),%ymm4,%ymm15
vpaddq    %ymm15,%ymm1,%ymm1

vpmuludq  512(%rsp),%ymm2,%ymm2
vpmuludq  384(%rsp),%ymm10,%ymm15
vpaddq    %ymm15,%ymm2,%ymm2
vpmuludq  %ymm3,%ymm11,%ymm15
vpaddq    %ymm15,%ymm2,%ymm2
vpmuludq  %ymm13,%ymm8,%ymm15
vpaddq    %ymm15,%ymm2,%ymm2
vpmuludq  %ymm7,%ymm6,%ymm15
vpaddq    %ymm15,%ymm2,%ymm2
vpmuludq  %ymm9,%ymm4,%ymm15
vpaddq    %ymm15,%ymm2,%ymm2

vpmuludq  416(%rsp),%ymm10,%ymm3
vpmuludq  384(%rsp),%ymm11,%ymm15
vpaddq    %ymm15,%ymm3,%ymm3
vpmuludq  448(%rsp),%ymm8,%ymm15
vpaddq    %ymm15,%ymm3,%ymm3
vpmuludq  %ymm9,%ymm6,%ymm15
vpaddq    %ymm15,%ymm3,%ymm3
vpmuludq  512(%rsp),%ymm4,%ymm15
vpaddq    %ymm15,%ymm3,%ymm3

vpmuludq  544(%rsp),%ymm4,%ymm4
vpmuludq  448(%rsp),%ymm10,%ymm15
vpaddq    %ymm15,%ymm4,%ymm4
vpmuludq  %ymm11,%ymm13,%ymm15
vpaddq    %ymm15,%ymm4,%ymm4
vmovdqa   384(%rsp),%ymm15
vpmuludq  %ymm15,%ymm15,%ymm15
vpaddq    %ymm15,%ymm4,%ymm4
vpmuludq  %ymm9,%ymm8,%ymm15
vpaddq    %ymm15,%ymm4,%ymm4
vpmuludq  %ymm5,%ymm6,%ymm15
vpaddq    %ymm15,%ymm4,%ymm4

vpmuludq  480(%rsp),%ymm10,%ymm5
vpmuludq  448(%rsp),%ymm11,%ymm15
vpaddq    %ymm15,%ymm5,%ymm5
vpmuludq  416(%rsp),%ymm12,%ymm15
vpaddq    %ymm15,%ymm5,%ymm5
vpmuludq  512(%rsp),%ymm8,%ymm15
vpaddq    %ymm15,%ymm5,%ymm5
vpmuludq  %ymm6,%ymm14,%ymm15
vpaddq    %ymm15,%ymm5,%ymm5

vpmuludq  576(%rsp),%ymm6,%ymm6
vpmuludq  512(%rsp),%ymm10,%ymm15
vpaddq    %ymm15,%ymm6,%ymm6
vpmuludq  %ymm11,%ymm9,%ymm15
vpaddq    %ymm15,%ymm6,%ymm6
vpmuludq  448(%rsp),%ymm12,%ymm15
vpaddq    %ymm15,%ymm6,%ymm6
vpmuludq  416(%rsp),%ymm13,%ymm15
vpaddq    %ymm15,%ymm6,%ymm6
vpmuludq  %ymm14,%ymm8,%ymm15
vpaddq    %ymm15,%ymm6,%ymm6

vpmuludq  544(%rsp),%ymm10,%ymm7
vpmuludq  512(%rsp),%ymm11,%ymm15
vpaddq    %ymm15,%ymm7,%ymm7
vpmuludq  480(%rsp),%ymm12,%ymm15
vpaddq    %ymm15,%ymm7,%ymm7
vpmuludq  448(%rsp),%ymm13,%ymm15
vpaddq    %ymm15,%ymm7,%ymm7
vpmuludq  576(%rsp),%ymm8,%ymm15
vpaddq    %ymm15,%ymm7,%ymm7

vpmuludq  608(%rsp),%ymm8,%ymm8
vpmuludq  576(%rsp),%ymm10,%ymm15
vpaddq    %ymm15,%ymm8,%ymm8
vpmuludq  %ymm11,%ymm14,%ymm15
vpaddq    %ymm15,%ymm8,%ymm8
vpmuludq  512(%rsp),%ymm12,%ymm15
vpaddq    %ymm15,%ymm8,%ymm8
vpmuludq  %ymm13,%ymm9,%ymm15
vpaddq    %ymm15,%ymm8,%ymm8
vmovdqa   448(%rsp),%ymm15
vpmuludq  %ymm15,%ymm15,%ymm15
vpaddq    %ymm15,%ymm8,%ymm8

vpmuludq  448(%rsp),%ymm9,%ymm9
vpmuludq  608(%rsp),%ymm10,%ymm15
vpaddq    %ymm15,%ymm9,%ymm9
vpmuludq  576(%rsp),%ymm11,%ymm15
vpaddq    %ymm15,%ymm9,%ymm9
vpmuludq  544(%rsp),%ymm12,%ymm15
vpaddq    %ymm15,%ymm9,%ymm9
vpmuludq  512(%rsp),%ymm13,%ymm15
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

// <T13',T14',T15',T16'> ← Pack-N2D(<T13,T14,T15,T16>)
vpsllq    $32,%ymm5,%ymm5
vpor      %ymm5,%ymm0,%ymm0
vpsllq    $32,%ymm6,%ymm6
vpor      %ymm6,%ymm1,%ymm1
vpsllq    $32,%ymm7,%ymm7
vpor      %ymm7,%ymm2,%ymm2
vpsllq    $32,%ymm8,%ymm8
vpor      %ymm8,%ymm3,%ymm3
vpsllq    $32,%ymm9,%ymm9
vpor      %ymm9,%ymm4,%ymm4

// <T14',T13',T16',T15'> ← Dense-Shuffle(<T13',T14',T15',T16'>)
vpshufd   $78,%ymm0,%ymm5
vpshufd   $78,%ymm1,%ymm6
vpshufd   $78,%ymm2,%ymm7
vpshufd   $78,%ymm3,%ymm8
vpshufd   $78,%ymm4,%ymm9

// <X2',*,*,*> ← Dense-Sub(<T13',T14',T15',T16'>,<T14',T13',T16',T15'>)
vpaddd	  sub_p1(%rip),%ymm0,%ymm10
vpaddd	  sub_p2(%rip),%ymm1,%ymm11
vpaddd	  sub_p3(%rip),%ymm2,%ymm12
vpaddd	  sub_p2(%rip),%ymm3,%ymm13
vpaddd	  sub_p3(%rip),%ymm4,%ymm14
vpsubd	  %ymm5,%ymm10,%ymm10
vpsubd	  %ymm6,%ymm11,%ymm11
vpsubd	  %ymm7,%ymm12,%ymm12
vpsubd	  %ymm8,%ymm13,%ymm13
vpsubd	  %ymm9,%ymm14,%ymm14

// <T9',T14',T15',T16'> ← Dense-Blend(<T9',T10',T11',T12'>,<T13',T14',T15',T16'>,0111)
vpblendd  $3,640(%rsp),%ymm0,%ymm0
vpblendd  $3,672(%rsp),%ymm1,%ymm1
vpblendd  $3,704(%rsp),%ymm2,%ymm2
vpblendd  $3,736(%rsp),%ymm3,%ymm3
vpblendd  $3,768(%rsp),%ymm4,%ymm4

vmovdqa   %ymm10,704(%rsp)
vmovdqa   %ymm11,736(%rsp)
vmovdqa   %ymm12,768(%rsp)
vmovdqa   %ymm13,800(%rsp)
vmovdqa   %ymm14,832(%rsp)

// <T9,T14,T15,T16> ← Pack-D2N(<T9',T14',T15',T16'>)
vpsrlq    $32,%ymm0,%ymm5
vpsrlq    $32,%ymm1,%ymm6
vpsrlq    $32,%ymm2,%ymm7
vpsrlq    $32,%ymm3,%ymm8
vpsrlq    $32,%ymm4,%ymm9

vmovdqa   %ymm0,384(%rsp)
vmovdqa   %ymm1,416(%rsp)
vmovdqa   %ymm2,448(%rsp)
vmovdqa   %ymm3,480(%rsp)
vmovdqa   %ymm4,512(%rsp)
vmovdqa   %ymm5,544(%rsp)
vmovdqa   %ymm6,576(%rsp)
vmovdqa   %ymm7,608(%rsp)
vmovdqa   %ymm8,640(%rsp)
vmovdqa   %ymm9,672(%rsp)

// <T17,T18,X3,Z3> ← Mul(<T10,A,1,X1>,<T9,T14,T15,T16>)
vmovdqa   96(%rsp),%ymm11
vmovdqa   160(%rsp),%ymm12
vmovdqa   224(%rsp),%ymm13
vmovdqa   288(%rsp),%ymm14
vmovdqa   352(%rsp),%ymm15

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
vpmuludq  320(%rsp),%ymm2,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpmuludq  256(%rsp),%ymm4,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpmuludq  192(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0
vpmuludq  128(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0

vpmuludq  352(%rsp),%ymm2,%ymm1
vpmuludq  320(%rsp),%ymm3,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  288(%rsp),%ymm4,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  256(%rsp),%ymm5,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  224(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  192(%rsp),%ymm7,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  160(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  128(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1

vpmuludq  %ymm15,%ymm3,%ymm2
vpmuludq  %ymm14,%ymm5,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  %ymm13,%ymm7,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  %ymm12,%ymm9,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  320(%rsp),%ymm4,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  256(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  192(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  416(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2

vpmuludq  352(%rsp),%ymm4,%ymm3
vpmuludq  320(%rsp),%ymm5,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  288(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  256(%rsp),%ymm7,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  224(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  192(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3

vpmuludq  %ymm15,%ymm5,%ymm4
vpmuludq  %ymm14,%ymm7,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  %ymm13,%ymm9,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  320(%rsp),%ymm6,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  256(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  416(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  480(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4

vpmuludq  352(%rsp),%ymm6,%ymm5
vpmuludq  320(%rsp),%ymm7,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  288(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  256(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5

vpmuludq  %ymm15,%ymm7,%ymm6
vpmuludq  %ymm14,%ymm9,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  320(%rsp),%ymm8,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  416(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  480(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  544(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6

vpmuludq  352(%rsp),%ymm8,%ymm7
vpmuludq  320(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7

vpmuludq  %ymm15,%ymm9,%ymm8
vpmuludq  416(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  480(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  544(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  608(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8

vmovdqa   64(%rsp),%ymm11
vmovdqa   96(%rsp),%ymm12
vmovdqa   128(%rsp),%ymm13
vmovdqa   160(%rsp),%ymm14
vmovdqa   192(%rsp),%ymm15
vmovdqa   224(%rsp),%ymm9

vpmuludq  384(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm0,%ymm0

vpmuludq  416(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpmuludq  384(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1

vpmuludq  448(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpmuludq  384(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2

vpmuludq  480(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  448(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  416(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3
vpmuludq  384(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm3,%ymm3

vpmuludq  512(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  448(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpmuludq  384(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4

vpmuludq  544(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  512(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  480(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  448(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  416(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5
vpmuludq  384(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm5,%ymm5

vpmuludq  576(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  512(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6
vpmuludq  448(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6

vpmuludq  608(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  576(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  544(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  512(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  480(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  448(%rsp),%ymm9,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7

vpmuludq  640(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  576(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  512(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8

vpmuludq  512(%rsp),%ymm9,%ymm9
vpmuludq  672(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  640(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  608(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  576(%rsp),%ymm14,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  544(%rsp),%ymm15,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9

vmovdqa   256(%rsp),%ymm11
vmovdqa   288(%rsp),%ymm12
vmovdqa   320(%rsp),%ymm13
vmovdqa   352(%rsp),%ymm14

vpmuludq  384(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm6,%ymm6

vpmuludq  416(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7
vpmuludq  384(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm7,%ymm7

vpmuludq  448(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8
vpmuludq  384(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm8,%ymm8

vpmuludq  480(%rsp),%ymm11,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  448(%rsp),%ymm12,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  416(%rsp),%ymm13,%ymm10
vpaddq    %ymm10,%ymm9,%ymm9
vpmuludq  384(%rsp),%ymm14,%ymm10
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

// <T17',T18',X3',Z3'> ← Pack-N2D(<T17,T18,X3,Z3>)
vpsllq    $32,%ymm13,%ymm13
vpor      %ymm13,%ymm0,%ymm0
vpsllq    $32,%ymm6,%ymm6
vpor      %ymm6,%ymm11,%ymm11
vpsllq    $32,%ymm14,%ymm14
vpor      %ymm14,%ymm2,%ymm2
vpsllq    $32,%ymm8,%ymm8
vpor      %ymm8,%ymm12,%ymm12
vpsllq    $32,%ymm15,%ymm15
vpor      %ymm15,%ymm4,%ymm4

// <T19',*,*,*> ← Dense-Add(<T17',T18',X3',Z3'>,<T17',T18',X3',Z3'>)
vpaddd	  %ymm0,%ymm0,%ymm1
vpaddd	  %ymm11,%ymm11,%ymm3
vpaddd	  %ymm2,%ymm2,%ymm5
vpaddd	  %ymm12,%ymm12,%ymm6
vpaddd	  %ymm4,%ymm4,%ymm7

// <*,T19',*,*> ← Dense-Shuffle(<T19',*,*,*>)
vpshufd   $78,%ymm1,%ymm1
vpshufd   $78,%ymm3,%ymm3
vpshufd   $78,%ymm5,%ymm5
vpshufd   $78,%ymm6,%ymm6
vpshufd   $78,%ymm7,%ymm7

// <*,Z2',*,*> ← Dense-Add(<T17',T18',X3',Z3'>,<*,T19',*,*>)
vpaddd	  %ymm0,%ymm1,%ymm1
vpaddd	  %ymm11,%ymm3,%ymm3
vpaddd	  %ymm2,%ymm5,%ymm5
vpaddd	  %ymm12,%ymm6,%ymm6
vpaddd	  %ymm4,%ymm7,%ymm7

// <X2',Z2',*,*> ← Dense-Blend(<X2',*,*,*>,<*,Z2',*,*>,0100)
vpblendd  $3,704(%rsp),%ymm1,%ymm1
vpblendd  $3,736(%rsp),%ymm3,%ymm3
vpblendd  $3,768(%rsp),%ymm5,%ymm5
vpblendd  $3,800(%rsp),%ymm6,%ymm6
vpblendd  $3,832(%rsp),%ymm7,%ymm7

// <X2',Z2',X3',Z3'> ← Dense-Blend(<X2',Z2',*,*>,<T17',T18',X3',Z3'>,0011)
vpblendd  $15,%ymm1,%ymm0,%ymm0
vpblendd  $15,%ymm3,%ymm11,%ymm11
vpblendd  $15,%ymm5,%ymm2,%ymm2
vpblendd  $15,%ymm6,%ymm12,%ymm12
vpblendd  $15,%ymm7,%ymm4,%ymm4

// <X2',Z2',X3',Z3'> ← Reduce(<X2',Z2',X3',Z3'>)
vpsrlvd   sh2625(%rip),%ymm0,%ymm15
vpaddd    %ymm15,%ymm11,%ymm11
vpand     vecmask2625(%rip),%ymm0,%ymm0

vpsrlvd   sh2526(%rip),%ymm11,%ymm15
vpaddd    %ymm15,%ymm2,%ymm2
vpand     vecmask2526(%rip),%ymm11,%ymm11

vpsrlvd   sh2625(%rip),%ymm2,%ymm15
vpaddd    %ymm15,%ymm12,%ymm12
vpand     vecmask2625(%rip),%ymm2,%ymm2

vpsrlvd   sh2526(%rip),%ymm12,%ymm15
vpaddd    %ymm15,%ymm4,%ymm4
vpand     vecmask2526(%rip),%ymm12,%ymm12

vpsrld    $26,%ymm4,%ymm15
vpsllq    $32,%ymm15,%ymm15
vpaddd    %ymm15,%ymm0,%ymm0
vpsrlq    $57,%ymm4,%ymm15
vpmuludq  vec19(%rip),%ymm15,%ymm15
vpaddd    %ymm15,%ymm0,%ymm0
vpand     vecmask2625(%rip),%ymm4,%ymm4

subb      $1,%cl
cmpb	  $0,%cl
jge       .L2

movb	  $7,%cl
subq      $1,%r15
cmpq	  $0,%r15
jge       .L1

// <X2,Z2,X3,Z3> ← Pack-D2N(<X2',Z2',X3',Z3'>)
vpsrlq    $32,%ymm0,%ymm13
vpand     vecmask32(%rip),%ymm0,%ymm0
vpsrlq    $32,%ymm11,%ymm6
vpand     vecmask32(%rip),%ymm11,%ymm11
vpsrlq    $32,%ymm2,%ymm14
vpand     vecmask32(%rip),%ymm2,%ymm2
vpsrlq    $32,%ymm12,%ymm8
vpand     vecmask32(%rip),%ymm12,%ymm12
vpsrlq    $32,%ymm4,%ymm15
vpand     vecmask32(%rip),%ymm4,%ymm4

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

// <X2,Z2,X3,Z3> ← Reduce(<X2,Z2,X3,Z3>)
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
