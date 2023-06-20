// linker define base
// linker use vec1216
// linker use vecmask23
// linker use vecmask29
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
// linker use upmask6
// linker use upmask7
// linker use upmask8
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
subq   $1536,%rsp

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

// convert to 9x4 form
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
vmovdqa   448(%rsp),%ymm8
vmovdqa   416(%rsp),%ymm9
vmovdqa   448(%rsp),%ymm10
vmovdqa   384(%rsp),%ymm11

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
