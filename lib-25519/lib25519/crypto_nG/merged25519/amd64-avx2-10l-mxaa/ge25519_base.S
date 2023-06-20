// linker define base
// linker use vec19
// linker use vecmask25
// linker use vecmask26
// linker use pmask1
// linker use pmask2
// linker use pmask3
// linker use pmask4
// linker use pmask5
// linker use pmask6
// linker use pmask7
// linker use pmask8
// linker use pmask9
// linker use pmask10
// linker use pmask11
// linker use pmask12
// linker use upmask1
// linker use upmask2
// linker use upmask3
// linker use upmask4
// linker use upmask5
// linker use mask63

/* Assembly for fixed base scalar multiplication. 
 * 
 * This assembly has been developed after studying the 
 * amd64-64-24k implementation of the work "High speed 
 * high security signatures" by Bernstein et al.
*/

#include "consts_namespace.h"

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(base)
.globl CRYPTO_SHARED_NAMESPACE(base)
_CRYPTO_SHARED_NAMESPACE(base):
CRYPTO_SHARED_NAMESPACE(base):

movq   %rsp,%r11
andq   $-32,%rsp
subq   $1120,%rsp

movq   %r11,0(%rsp)
movq   %r12,8(%rsp)
movq   %r13,16(%rsp)
movq   %r14,24(%rsp)
movq   %r15,32(%rsp)
movq   %rbx,40(%rsp)
movq   %rbp,48(%rsp)
movq   %rdi,56(%rsp)
movq   %rsi,64(%rsp)
movq   %rdx,72(%rsp)

/* choose t and initialize r */

movq   %rdx,%rcx
movzbl 0(%rsi),%eax
movsbq %al,%rdx
movq   $0,%rsi

imulq  $768,%rsi,%rdi
movq   %rdx,%rsi
sarq   $7,%rsi

movq   %rdx,%r8
addq   %rsi,%r8
xorq   %rsi,%r8

movq   $1,%rsi
movq   $0,%r9
movq   $0,%rax
movq   $0,%r10
movq   $1,%r11
movq   $0,%r12
movq   $0,%r13
movq   $0,%r14

cmpq   $1,%r8

movq   0(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   8(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   16(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   24(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   32(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   40(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   48(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   56(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $2,%r8
movq   96(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   104(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   112(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   120(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   128(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   136(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   144(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   152(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $3,%r8
movq   192(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   200(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   208(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   216(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   224(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   232(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   240(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   248(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $4,%r8
movq   288(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   296(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   304(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   312(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   320(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   328(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   336(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   344(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $5,%r8
movq   384(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   392(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   400(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   408(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   416(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   424(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   432(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   440(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $6,%r8
movq   480(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   488(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   496(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   504(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   512(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   520(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   528(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   536(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $7,%r8
movq   576(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   584(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   592(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   600(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   608(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   616(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   624(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   632(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $8,%r8
movq   672(%rcx,%rdi),%r15
cmove  %r15,%rsi
movq   680(%rcx,%rdi),%r15
cmove  %r15,%r9
movq   688(%rcx,%rdi),%r15
cmove  %r15,%rax
movq   696(%rcx,%rdi),%r15
cmove  %r15,%r10
movq   704(%rcx,%rdi),%r15
cmove  %r15,%r11
movq   712(%rcx,%rdi),%r15
cmove  %r15,%r12
movq   720(%rcx,%rdi),%r15
cmove  %r15,%r13
movq   728(%rcx,%rdi),%r15
cmove  %r15,%r14

cmpq   $0,%rdx
movq   %rsi,%r15
cmovl  %r11,%rsi
cmovl  %r15,%r11
movq   %r9,%r15
cmovl  %r12,%r9
cmovl  %r15,%r12
movq   %rax,%r15
cmovl  %r13,%rax
cmovl  %r15,%r13
movq   %r10,%r15
cmovl  %r14,%r10
cmovl  %r15,%r14

movq   %r11,%r15
movq   %r12,80(%rsp)
movq   %r13,88(%rsp)
movq   %r14,96(%rsp)

// sub 
subq   %rsi,%r11
sbbq   %r9,%r12
sbbq   %rax,%r13
sbbq   %r10,%r14

movq   $0,%rbx
movq   $38,%rbp
cmovae %rbx,%rbp

subq   %rbp,%r11
sbbq   %rbx,%r12
sbbq   %rbx,%r13
sbbq   %rbx,%r14

cmovc  %rbp,%rbx
subq   %rbx,%r11

movq   %r11,128(%rsp)
movq   %r12,136(%rsp)
movq   %r13,144(%rsp)
movq   %r14,152(%rsp)

// add
addq   %r15,%rsi
adcq   80(%rsp),%r9
adcq   88(%rsp),%rax
adcq   96(%rsp),%r10

movq   $0,%rbx
movq   $38,%rbp
cmovae %rbx,%rbp

addq   %rbp,%rsi
adcq   %rbx,%r9
adcq   %rbx,%rax
adcq   %rbx,%r10

cmovc  %rbp,%rbx
addq   %rbx,%rsi

movq   %rsi,160(%rsp)
movq   %r9,168(%rsp)
movq   %rax,176(%rsp)
movq   %r10,184(%rsp)

movq   $0,%rsi
movq   $0,%r9
movq   $0,%rax
movq   $0,%r10

cmpq   $1,%r8
movq   64(%rcx,%rdi),%r11
cmove  %r11,%rsi
movq   72(%rcx,%rdi),%r11
cmove  %r11,%r9
movq   80(%rcx,%rdi),%r11
cmove  %r11,%rax
movq   88(%rcx,%rdi),%r11
cmove  %r11,%r10

cmpq   $2,%r8
movq   160(%rcx,%rdi),%r11
cmove  %r11,%rsi
movq   168(%rcx,%rdi),%r11
cmove  %r11,%r9
movq   176(%rcx,%rdi),%r11
cmove  %r11,%rax
movq   184(%rcx,%rdi),%r11
cmove  %r11,%r10

cmpq   $3,%r8
movq   256(%rcx,%rdi),%r11
cmove  %r11,%rsi
movq   264(%rcx,%rdi),%r11
cmove  %r11,%r9
movq   272(%rcx,%rdi),%r11
cmove  %r11,%rax
movq   280(%rcx,%rdi),%r11
cmove  %r11,%r10

cmpq   $4,%r8
movq   352(%rcx,%rdi),%r11
cmove  %r11,%rsi
movq   360(%rcx,%rdi),%r11
cmove  %r11,%r9
movq   368(%rcx,%rdi),%r11
cmove  %r11,%rax
movq   376(%rcx,%rdi),%r11
cmove  %r11,%r10

cmpq   $5,%r8
movq   448(%rcx,%rdi),%r11
cmove  %r11,%rsi
movq   456(%rcx,%rdi),%r11
cmove  %r11,%r9
movq   464(%rcx,%rdi),%r11
cmove  %r11,%rax
movq   472(%rcx,%rdi),%r11
cmove  %r11,%r10

cmpq   $6,%r8
movq   544(%rcx,%rdi),%r11
cmove  %r11,%rsi
movq   552(%rcx,%rdi),%r11
cmove  %r11,%r9
movq   560(%rcx,%rdi),%r11
cmove  %r11,%rax
movq   568(%rcx,%rdi),%r11
cmove  %r11,%r10

cmpq   $7,%r8
movq   640(%rcx,%rdi),%r11
cmove  %r11,%rsi
movq   648(%rcx,%rdi),%r11
cmove  %r11,%r9
movq   656(%rcx,%rdi),%r11
cmove  %r11,%rax
movq   664(%rcx,%rdi),%r11
cmove  %r11,%r10

cmpq   $8,%r8
movq   736(%rcx,%rdi),%r8
cmove  %r8,%rsi
movq   744(%rcx,%rdi),%r8
cmove  %r8,%r9
movq   752(%rcx,%rdi),%r8
cmove  %r8,%rax
movq   760(%rcx,%rdi),%rdi
cmove  %rdi,%r10

movq   $0,%rdi
movq   $0,%rcx
movq   $0,%r8
movq   $0,%r11

subq   %rsi,%rdi
sbbq   %r9,%rcx
sbbq   %rax,%r8
sbbq   %r10,%r11

movq   $0,%r12
movq   $38,%r13
cmovae %r12,%r13

subq   %r13,%rdi
sbbq   %r12,%rcx
sbbq   %r12,%r8
sbbq   %r12,%r11

cmovc  %r13,%r12

subq   %r12,%rdi

cmpq   $0,%rdx
cmovl  %rdi,%rsi
cmovl  %rcx,%r9
cmovl  %r8,%rax
cmovl  %r11,%r10

movq   %rsi,224(%rsp)
movq   %r9,232(%rsp)
movq   %rax,240(%rsp)
movq   %r10,248(%rsp)

movq   $2,192(%rsp)
movq   $0,200(%rsp)
movq   $0,208(%rsp)
movq   $0,216(%rsp)

/* loop: i=1,i<64,i=i+1 */

movq    $1,80(%rsp)

.L:

/* choose t */

movq    72(%rsp),%rcx
movq    80(%rsp),%rsi
movq    64(%rsp),%rdx
movzbl  0(%rdx,%rsi),%eax
movsbq  %al,%rdx

imulq   $768,%rsi,%rdi
movq    %rdx,%rsi
sarq    $7,%rsi

movq    %rdx,%r8
addq    %rsi,%r8
xorq    %rsi,%r8

movq    $1,%rsi
movq    $0,%r9
movq    $0,%rax
movq    $0,%r10
movq    $1,%r11
movq    $0,%r12
movq    $0,%r13
movq    $0,%r14

cmpq    $1,%r8

movq    0(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    8(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    16(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    24(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    32(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    40(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    48(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    56(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $2,%r8
movq    96(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    104(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    112(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    120(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    128(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    136(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    144(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    152(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $3,%r8
movq    192(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    200(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    208(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    216(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    224(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    232(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    240(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    248(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $4,%r8
movq    288(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    296(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    304(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    312(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    320(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    328(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    336(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    344(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $5,%r8
movq    384(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    392(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    400(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    408(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    416(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    424(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    432(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    440(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $6,%r8
movq    480(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    488(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    496(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    504(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    512(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    520(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    528(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    536(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $7,%r8
movq    576(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    584(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    592(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    600(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    608(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    616(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    624(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    632(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $8,%r8
movq    672(%rcx,%rdi),%r15
cmove   %r15,%rsi
movq    680(%rcx,%rdi),%r15
cmove   %r15,%r9
movq    688(%rcx,%rdi),%r15
cmove   %r15,%rax
movq    696(%rcx,%rdi),%r15
cmove   %r15,%r10
movq    704(%rcx,%rdi),%r15
cmove   %r15,%r11
movq    712(%rcx,%rdi),%r15
cmove   %r15,%r12
movq    720(%rcx,%rdi),%r15
cmove   %r15,%r13
movq    728(%rcx,%rdi),%r15
cmove   %r15,%r14

cmpq    $0,%rdx
movq    %rsi,%r15
cmovl   %r11,%rsi
cmovl   %r15,%r11
movq    %r9,%r15
cmovl   %r12,%r9
cmovl   %r15,%r12
movq    %rax,%r15
cmovl   %r13,%rax
cmovl   %r15,%r13
movq    %r10,%r15
cmovl   %r14,%r10
cmovl   %r15,%r14

movq    %rsi,256(%rsp)
movq    %r9,264(%rsp)
movq    %rax,272(%rsp)
movq    %r10,280(%rsp)
movq    %r11,288(%rsp)
movq    %r12,296(%rsp)
movq    %r13,304(%rsp)
movq    %r14,312(%rsp)

movq    $0,%rsi
movq    $0,%r9
movq    $0,%rax
movq    $0,%r10

cmpq    $1,%r8
movq    64(%rcx,%rdi),%r11
cmove   %r11,%rsi
movq    72(%rcx,%rdi),%r11
cmove   %r11,%r9
movq    80(%rcx,%rdi),%r11
cmove   %r11,%rax
movq    88(%rcx,%rdi),%r11
cmove   %r11,%r10

cmpq    $2,%r8
movq    160(%rcx,%rdi),%r11
cmove   %r11,%rsi
movq    168(%rcx,%rdi),%r11
cmove   %r11,%r9
movq    176(%rcx,%rdi),%r11
cmove   %r11,%rax
movq    184(%rcx,%rdi),%r11
cmove   %r11,%r10

cmpq    $3,%r8
movq    256(%rcx,%rdi),%r11
cmove   %r11,%rsi
movq    264(%rcx,%rdi),%r11
cmove   %r11,%r9
movq    272(%rcx,%rdi),%r11
cmove   %r11,%rax
movq    280(%rcx,%rdi),%r11
cmove   %r11,%r10

cmpq    $4,%r8
movq    352(%rcx,%rdi),%r11
cmove   %r11,%rsi
movq    360(%rcx,%rdi),%r11
cmove   %r11,%r9
movq    368(%rcx,%rdi),%r11
cmove   %r11,%rax
movq    376(%rcx,%rdi),%r11
cmove   %r11,%r10

cmpq    $5,%r8
movq    448(%rcx,%rdi),%r11
cmove   %r11,%rsi
movq    456(%rcx,%rdi),%r11
cmove   %r11,%r9
movq    464(%rcx,%rdi),%r11
cmove   %r11,%rax
movq    472(%rcx,%rdi),%r11
cmove   %r11,%r10

cmpq    $6,%r8
movq    544(%rcx,%rdi),%r11
cmove   %r11,%rsi
movq    552(%rcx,%rdi),%r11
cmove   %r11,%r9
movq    560(%rcx,%rdi),%r11
cmove   %r11,%rax
movq    568(%rcx,%rdi),%r11
cmove   %r11,%r10

cmpq    $7,%r8
movq    640(%rcx,%rdi),%r11
cmove   %r11,%rsi
movq    648(%rcx,%rdi),%r11
cmove   %r11,%r9
movq    656(%rcx,%rdi),%r11
cmove   %r11,%rax
movq    664(%rcx,%rdi),%r11
cmove   %r11,%r10

cmpq    $8,%r8
movq    736(%rcx,%rdi),%r8
cmove   %r8,%rsi
movq    744(%rcx,%rdi),%r8
cmove   %r8,%r9
movq    752(%rcx,%rdi),%r8
cmove   %r8,%rax
movq    760(%rcx,%rdi),%rdi
cmove   %rdi,%r10

movq    $0,%rdi
movq    $0,%rcx
movq    $0,%r8
movq    $0,%r11

subq    %rsi,%rdi
sbbq    %r9,%rcx
sbbq    %rax,%r8
sbbq    %r10,%r11

movq    $0,%r12
movq    $38,%r13
cmovae  %r12,%r13

subq    %r13,%rdi
sbbq    %r12,%rcx
sbbq    %r12,%r8
sbbq    %r12,%r11

cmovc   %r13,%r12

subq    %r12,%rdi

cmpq    $0,%rdx
cmovl   %rdi,%rsi
cmovl   %rcx,%r9
cmovl   %r8,%rax
cmovl   %r11,%r10

movq    %rsi,320(%rsp)
movq    %r9,328(%rsp)
movq    %rax,336(%rsp)
movq    %r10,344(%rsp)

/* nielsadd2 */

// load
movq    160(%rsp),%rdx
movq    168(%rsp),%rcx
movq    176(%rsp),%r8
movq    184(%rsp),%r9

// copy
movq    %rdx,%rax
movq    %rcx,%r10
movq    %r8,%r11
movq    %r9,%r12

// sub
subq    128(%rsp),%rdx
sbbq    136(%rsp),%rcx
sbbq    144(%rsp),%r8
sbbq    152(%rsp),%r9

movq    $0,%r13
movq    $38,%r14
cmovae  %r13,%r14

subq    %r14,%rdx
sbbq    %r13,%rcx
sbbq    %r13,%r8
sbbq    %r13,%r9

cmovc   %r14,%r13
subq    %r13,%rdx

movq    %rdx,352(%rsp)
movq    %rcx,360(%rsp)
movq    %r8,368(%rsp)
movq    %r9,376(%rsp)

// add
addq    128(%rsp),%rax
adcq    136(%rsp),%r10
adcq    144(%rsp),%r11
adcq    152(%rsp),%r12

movq    $0,%r13
movq    $38,%r14
cmovae  %r13,%r14

addq    %r14,%rax
adcq    %r13,%r10
adcq    %r13,%r11
adcq    %r13,%r12

cmovc   %r14,%r13
addq    %r13,%rax

movq    %rax,384(%rsp)
movq    %r10,392(%rsp)
movq    %r11,400(%rsp)
movq    %r12,408(%rsp)

// mul
movq    256(%rsp),%rdx

mulx    352(%rsp),%r8,%r9
mulx    360(%rsp),%rcx,%r10
addq    %rcx,%r9

mulx    368(%rsp),%rcx,%r11
adcq    %rcx,%r10

mulx    376(%rsp),%rcx,%r12
adcq    %rcx,%r11
adcq    $0,%r12

movq    264(%rsp),%rdx    

mulx    352(%rsp),%rax,%rbx
mulx    360(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    368(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    376(%rsp),%rcx,%r13
adcq    %rcx,%rsi
adcq    $0,%r13

addq    %rax,%r9
adcq    %rbx,%r10
adcq    %rbp,%r11
adcq    %rsi,%r12
adcq    $0,%r13

movq    272(%rsp),%rdx

mulx    352(%rsp),%rax,%rbx
mulx    360(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    368(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    376(%rsp),%rcx,%r14
adcq    %rcx,%rsi
adcq    $0,%r14

addq    %rax,%r10
adcq    %rbx,%r11
adcq    %rbp,%r12
adcq    %rsi,%r13
adcq    $0,%r14

movq    280(%rsp),%rdx

mulx    352(%rsp),%rax,%rbx
mulx    360(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    368(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    376(%rsp),%rcx,%r15
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

movq    %r8,352(%rsp)
movq    %r9,360(%rsp)
movq    %r10,368(%rsp)
movq    %r11,376(%rsp)

// mul
movq    288(%rsp),%rdx

mulx    384(%rsp),%r8,%r9
mulx    392(%rsp),%rcx,%r10
addq    %rcx,%r9

mulx    400(%rsp),%rcx,%r11
adcq    %rcx,%r10

mulx    408(%rsp),%rcx,%r12
adcq    %rcx,%r11
adcq    $0,%r12

movq    296(%rsp),%rdx    

mulx    384(%rsp),%rax,%rbx
mulx    392(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    400(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    408(%rsp),%rcx,%r13
adcq    %rcx,%rsi
adcq    $0,%r13

addq    %rax,%r9
adcq    %rbx,%r10
adcq    %rbp,%r11
adcq    %rsi,%r12
adcq    $0,%r13

movq    304(%rsp),%rdx

mulx    384(%rsp),%rax,%rbx
mulx    392(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    400(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    408(%rsp),%rcx,%r14
adcq    %rcx,%rsi
adcq    $0,%r14

addq    %rax,%r10
adcq    %rbx,%r11
adcq    %rbp,%r12
adcq    %rsi,%r13
adcq    $0,%r14

movq    312(%rsp),%rdx

mulx    384(%rsp),%rax,%rbx
mulx    392(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    400(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    408(%rsp),%rcx,%r15
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

// copy
movq    %r8,%r12
movq    %r9,%r13
movq    %r10,%r14
movq    %r11,%r15

// sub
subq    352(%rsp),%r8
sbbq    360(%rsp),%r9
sbbq    368(%rsp),%r10
sbbq    376(%rsp),%r11

movq    $0,%rax
movq    $38,%rbx
cmovae  %rax,%rbx

subq    %rbx,%r8
sbbq    %rax,%r9
sbbq    %rax,%r10
sbbq    %rax,%r11

cmovc   %rbx,%rax
subq    %rax,%r8

movq    %r8,384(%rsp)
movq    %r9,392(%rsp)
movq    %r10,400(%rsp)
movq    %r11,408(%rsp)

// add
addq    352(%rsp),%r12
adcq    360(%rsp),%r13
adcq    368(%rsp),%r14
adcq    376(%rsp),%r15

movq    $0,%rax
movq    $38,%rbx
cmovae  %rax,%rbx

addq    %rbx,%r12
adcq    %rax,%r13
adcq    %rax,%r14
adcq    %rax,%r15

cmovc   %rbx,%rax
addq    %rax,%r12

movq    %r12,352(%rsp)
movq    %r13,360(%rsp)
movq    %r14,368(%rsp)
movq    %r15,376(%rsp)

// mul
movq    320(%rsp),%rdx

mulx    224(%rsp),%r8,%r9
mulx    232(%rsp),%rcx,%r10
addq    %rcx,%r9

mulx    240(%rsp),%rcx,%r11
adcq    %rcx,%r10

mulx    248(%rsp),%rcx,%r12
adcq    %rcx,%r11
adcq    $0,%r12

movq    328(%rsp),%rdx    

mulx    224(%rsp),%rax,%rbx
mulx    232(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    240(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    248(%rsp),%rcx,%r13
adcq    %rcx,%rsi
adcq    $0,%r13

addq    %rax,%r9
adcq    %rbx,%r10
adcq    %rbp,%r11
adcq    %rsi,%r12
adcq    $0,%r13

movq    336(%rsp),%rdx

mulx    224(%rsp),%rax,%rbx
mulx    232(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    240(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    248(%rsp),%rcx,%r14
adcq    %rcx,%rsi
adcq    $0,%r14

addq    %rax,%r10
adcq    %rbx,%r11
adcq    %rbp,%r12
adcq    %rsi,%r13
adcq    $0,%r14

movq    344(%rsp),%rdx

mulx    224(%rsp),%rax,%rbx
mulx    232(%rsp),%rcx,%rbp
addq    %rcx,%rbx

mulx    240(%rsp),%rcx,%rsi
adcq    %rcx,%rbp

mulx    248(%rsp),%rcx,%r15
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

movq    192(%rsp),%r12
movq    200(%rsp),%r13
movq    208(%rsp),%r14
movq    216(%rsp),%r15

// double
addq    %r12,%r12
adcq    %r13,%r13
adcq    %r14,%r14
adcq    %r15,%r15

movq    $0,%rbx
movq    $38,%rax
cmovae  %rbx,%rax

addq    %rax,%r12
adcq    %rbx,%r13
adcq    %rbx,%r14
adcq    %rbx,%r15

cmovc   %rax,%rbx
addq    %rbx,%r12

// copy
movq    %r12,%rsi
movq    %r13,%rax
movq    %r14,%rcx
movq    %r15,%rdx

// sub
subq    %r8,%r12
sbbq    %r9,%r13
sbbq    %r10,%r14
sbbq    %r11,%r15

movq    $0,%rbx
movq    $38,%rbp
cmovae  %rbx,%rbp

subq    %rbp,%r12
sbbq    %rbx,%r13
sbbq    %rbx,%r14
sbbq    %rbx,%r15

cmovc   %rbp,%rbx
subq    %rbx,%r12

movq    %r12,448(%rsp)
movq    %r13,456(%rsp)
movq    %r14,464(%rsp)
movq    %r15,472(%rsp)

// add
addq    %r8,%rsi
adcq    %r9,%rax
adcq    %r10,%rcx
adcq    %r11,%rdx

movq    $0,%rbx
movq    $38,%rbp
cmovae  %rbx,%rbp

addq    %rbp,%rsi
adcq    %rbx,%rax
adcq    %rbx,%rcx
adcq    %rbx,%rdx

cmovc   %rbp,%rbx
addq    %rbx,%rsi

movq    %rsi,416(%rsp)
movq    %rax,424(%rsp)
movq    %rcx,432(%rsp)
movq    %rdx,440(%rsp)

/* p1p1 to p3 */

// convert to 10x4 form
vmovdqa   384(%rsp),%ymm8
vmovdqa   352(%rsp),%ymm9
vmovdqa   416(%rsp),%ymm10
vmovdqa   352(%rsp),%ymm11

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
vmovdqa   448(%rsp),%ymm8
vmovdqa   416(%rsp),%ymm9
vmovdqa   448(%rsp),%ymm10
vmovdqa   384(%rsp),%ymm11

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

vmovdqa   %ymm10,128(%rsp)
vmovdqa   %ymm11,160(%rsp)
vmovdqa   %ymm12,192(%rsp)
vmovdqa   %ymm13,224(%rsp)

movq    80(%rsp),%r15
addq    $1,%r15
movq    %r15,80(%rsp)
cmpq	$63,%r15

jle     .L

/* store final value of r */

movq    56(%rsp),%rdi

movq    128(%rsp),%r8
movq    136(%rsp),%r9
movq    144(%rsp),%r10
movq    152(%rsp),%r11

movq    %r8,0(%rdi)
movq    %r9,8(%rdi)
movq    %r10,16(%rdi)
movq    %r11,24(%rdi)

movq    160(%rsp),%r8
movq    168(%rsp),%r9
movq    176(%rsp),%r10
movq    184(%rsp),%r11

movq    %r8,32(%rdi)
movq    %r9,40(%rdi)
movq    %r10,48(%rdi)
movq    %r11,56(%rdi)

movq    192(%rsp),%r8
movq    200(%rsp),%r9
movq    208(%rsp),%r10
movq    216(%rsp),%r11

movq    %r8,64(%rdi)
movq    %r9,72(%rdi)
movq    %r10,80(%rdi)
movq    %r11,88(%rdi)

movq    224(%rsp),%r8
movq    232(%rsp),%r9
movq    240(%rsp),%r10
movq    248(%rsp),%r11

movq    %r8,96(%rdi)
movq    %r9,104(%rdi)
movq    %r10,112(%rdi)
movq    %r11,120(%rdi)

movq 	 0(%rsp),%r11
movq 	 8(%rsp),%r12
movq 	16(%rsp),%r13
movq 	24(%rsp),%r14
movq 	32(%rsp),%r15
movq 	40(%rsp),%rbx
movq 	48(%rsp),%rbp

movq 	%r11,%rsp

ret
