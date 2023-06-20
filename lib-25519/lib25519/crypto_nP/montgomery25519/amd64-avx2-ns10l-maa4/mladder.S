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
subq 	  $960,%rsp

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
vpaddd    %ymm1,%ymm3,%ymm0

vpshufd   $68,%ymm2,%ymm1
vpshufd   $238,%ymm2,%ymm3
vpaddd    hh1_p2(%rip),%ymm1,%ymm1
vpxor     hh1_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm2

vpshufd   $68,%ymm4,%ymm1
vpshufd   $238,%ymm4,%ymm3
vpaddd    hh1_p2(%rip),%ymm1,%ymm1
vpxor     hh1_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm4

vpshufd   $68,%ymm6,%ymm1
vpshufd   $238,%ymm6,%ymm3
vpaddd    hh1_p2(%rip),%ymm1,%ymm1
vpxor     hh1_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm6

vpshufd   $68,%ymm8,%ymm1
vpshufd   $238,%ymm8,%ymm3
vpaddd    hh1_p2(%rip),%ymm1,%ymm1
vpxor     hh1_xor(%rip),%ymm3,%ymm3
vpaddd    %ymm1,%ymm3,%ymm8

// <T1,T2,T4,T3> ← Pack-N2D(<T1',T2',T4',T3'>)
vpsrlq    $32,%ymm0,%ymm11
vpsrlq    $32,%ymm2,%ymm12
vpsrlq    $32,%ymm4,%ymm13
vpsrlq    $32,%ymm6,%ymm14
vpsrlq    $32,%ymm8,%ymm15

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

// <T1',T2',T1',T2'> ← Dense-Dup(<T1',T2',T4',T3'>)
vpermq	  $68,%ymm0,%ymm0
vpermq	  $68,%ymm2,%ymm2
vpermq	  $68,%ymm4,%ymm4
vpermq	  $68,%ymm6,%ymm6
vpermq	  $68,%ymm8,%ymm8

// <T1,T2,T1,T2> ← Pack-D2N(<T1',T2',T1',T2'>)
vpsrlq    $32,%ymm0,%ymm1
vpsrlq    $32,%ymm2,%ymm3
vpsrlq    $32,%ymm4,%ymm5
vpsrlq    $32,%ymm6,%ymm7
vpsrlq    $32,%ymm8,%ymm9

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

// <T5,T6,T7,T8> ← Mul(<T1,T2,T4,T3>,<T1,T2,T1,T2>)
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

vmovdqa   %ymm0,640(%rsp)
vmovdqa   %ymm3,672(%rsp)
vmovdqa   %ymm12,704(%rsp)
vmovdqa   %ymm13,736(%rsp)
vmovdqa   %ymm7,768(%rsp)
vmovdqa   %ymm9,800(%rsp)
vmovdqa   %ymm5,832(%rsp)
vmovdqa   %ymm14,864(%rsp)
vmovdqa   %ymm6,896(%rsp)
vmovdqa   %ymm8,928(%rsp)

// <T5',T6',T7',T8'> ← Pack-N2D(<T5,T6,T7,T8>)
vpsllq    $32,%ymm3,%ymm3
vpor      %ymm3,%ymm0,%ymm0
vpsllq    $32,%ymm13,%ymm13
vpor      %ymm13,%ymm12,%ymm2
vpsllq    $32,%ymm9,%ymm9
vpor      %ymm9,%ymm7,%ymm4
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
vpblendd  $240,0(%rsi),%ymm0,%ymm10
vpblendd  $240,32(%rsi),%ymm12,%ymm11
vpblendd  $240,64(%rsi),%ymm7,%ymm13
vpblendd  $240,96(%rsi),%ymm5,%ymm14
vpblendd  $240,128(%rsi),%ymm6,%ymm15

// <T9,T10,1,X1> ← Pack-D2N(<T9',T10',1',X1'>)
vmovdqa   %ymm10,64(%rsp)
vpsrlq    $32,%ymm10,%ymm10
vmovdqa   %ymm10,96(%rsp)
vmovdqa   %ymm11,128(%rsp)
vpsrlq    $32,%ymm11,%ymm11
vmovdqa   %ymm11,160(%rsp)
vmovdqa   %ymm13,192(%rsp)
vpsrlq    $32,%ymm13,%ymm13
vmovdqa   %ymm13,224(%rsp)
vmovdqa   %ymm14,256(%rsp)
vpsrlq    $32,%ymm14,%ymm14
vmovdqa   %ymm14,288(%rsp)
vmovdqa   %ymm15,320(%rsp)
vpsrlq    $32,%ymm15,%ymm15
vmovdqa   %ymm15,352(%rsp)

// <T9,T10,T11,T12> ← Pack-D2N(<T9',T10',T11',T12'>)
vpsrlq    $32,%ymm0,%ymm3
vpsrlq    $32,%ymm12,%ymm13
vpsrlq    $32,%ymm7,%ymm9
vpsrlq    $32,%ymm5,%ymm14
vpsrlq    $32,%ymm6,%ymm8

// <0,T13,0,0> ← Unreduced-Mulc(<T9,T10,T11,T12>,<0,a24,0,0>)
// <T5,T14,T7,T8> ← Add(<0,T13,0,0>,<T5,T6,T7,T8>)
vpmuludq  a24(%rip),%ymm0,%ymm1
vpaddq    640(%rsp),%ymm1,%ymm1
vpmuludq  a24(%rip),%ymm3,%ymm2
vpaddq    672(%rsp),%ymm2,%ymm2
vpmuludq  a24(%rip),%ymm12,%ymm4
vpaddq    704(%rsp),%ymm4,%ymm4
vpmuludq  a24(%rip),%ymm9,%ymm11
vpaddq    800(%rsp),%ymm11,%ymm11
vpmuludq  a24(%rip),%ymm5,%ymm15
vpaddq    832(%rsp),%ymm15,%ymm15

vpsrlq    $26,%ymm1,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpand     vecmask26(%rip),%ymm1,%ymm1

vpsrlq    $25,%ymm11,%ymm10
vpaddq    %ymm10,%ymm15,%ymm15
vpand     vecmask25(%rip),%ymm11,%ymm11

vpsrlq    $25,%ymm2,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpand     vecmask25(%rip),%ymm2,%ymm2
vmovdqa   %ymm2,672(%rsp)

vpmuludq  a24(%rip),%ymm14,%ymm2
vpaddq    864(%rsp),%ymm2,%ymm2

vpsrlq    $26,%ymm15,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpand     vecmask26(%rip),%ymm15,%ymm15
vmovdqa   %ymm15,832(%rsp)

vpmuludq  a24(%rip),%ymm13,%ymm15
vpaddq    736(%rsp),%ymm15,%ymm15

vpsrlq    $26,%ymm4,%ymm10
vpaddq    %ymm10,%ymm15,%ymm15
vpand     vecmask26(%rip),%ymm4,%ymm4
vmovdqa   %ymm4,704(%rsp)

vpmuludq  a24(%rip),%ymm6,%ymm4
vpaddq    896(%rsp),%ymm4,%ymm4

vpsrlq    $25,%ymm2,%ymm10
vpaddq    %ymm10,%ymm4,%ymm4
vpand     vecmask25(%rip),%ymm2,%ymm2
vmovdqa   %ymm2,864(%rsp)

vpmuludq  a24(%rip),%ymm7,%ymm2
vpaddq    768(%rsp),%ymm2,%ymm2

vpsrlq    $25,%ymm15,%ymm10
vpaddq    %ymm10,%ymm2,%ymm2
vpand     vecmask25(%rip),%ymm15,%ymm15
vmovdqa   %ymm15,736(%rsp)

vpmuludq  a24(%rip),%ymm8,%ymm15
vpaddq    928(%rsp),%ymm15,%ymm15

vpsrlq    $26,%ymm4,%ymm10
vpaddq    %ymm10,%ymm15,%ymm15
vpand     vecmask26(%rip),%ymm4,%ymm4
vmovdqa   %ymm4,896(%rsp)

vpsrlq    $26,%ymm2,%ymm10
vpaddq    %ymm10,%ymm11,%ymm11
vpand     vecmask26(%rip),%ymm2,%ymm2
vmovdqa   %ymm2,768(%rsp)
vmovdqa   %ymm11,800(%rsp)

vpsrlq    $25,%ymm15,%ymm10
vpmuludq  vec19(%rip),%ymm10,%ymm10
vpaddq    %ymm10,%ymm1,%ymm1
vpand     vecmask25(%rip),%ymm15,%ymm15
vmovdqa   %ymm15,928(%rsp)
vmovdqa   %ymm1,640(%rsp)

// <*,*,T15,T16> ← Sqr(<T9,T10,T11,T12>)
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

// <T5,T14,T15,T16> ← Blend(<T5,T14,T7,T8>,<*,*,T15,T16>,0011)
vpblendd  $15,640(%rsp),%ymm0,%ymm0
vpblendd  $15,672(%rsp),%ymm1,%ymm1
vmovdqa   %ymm0,384(%rsp)
vmovdqa   %ymm1,416(%rsp)

vpblendd  $15,704(%rsp),%ymm2,%ymm2
vpblendd  $15,736(%rsp),%ymm3,%ymm3
vmovdqa   %ymm2,448(%rsp)
vmovdqa   %ymm3,480(%rsp)

vpblendd  $15,768(%rsp),%ymm4,%ymm4
vpblendd  $15,800(%rsp),%ymm5,%ymm5
vmovdqa   %ymm4,512(%rsp)
vmovdqa   %ymm5,544(%rsp)

vpblendd  $15,832(%rsp),%ymm6,%ymm6
vpblendd  $15,864(%rsp),%ymm7,%ymm7
vmovdqa   %ymm6,576(%rsp)
vmovdqa   %ymm7,608(%rsp)

vpblendd  $15,896(%rsp),%ymm8,%ymm8
vpblendd  $15,928(%rsp),%ymm9,%ymm9
vmovdqa   %ymm8,640(%rsp)
vmovdqa   %ymm9,672(%rsp)

// <X2,Z2,X3,Z3> ← Mul(<T5,T14,T15,T16>,<T9,T10,1,X1>)
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
