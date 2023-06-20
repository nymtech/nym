// linker define base
// linker use vec19
// linker use vec608
// linker use vecmask47
// linker use vecmask52
// linker use pmask1
// linker use pmask2
// linker use pmask3
// linker use pmask4
// linker use pmask5
// linker use pmask6
// linker use pmask7
// linker use pmask8
// linker use upmask1
// linker use upmask2
// linker use upmask3
// linker use upmask4
// linker use upmask5
// linker use upmask6
// linker use upmask7
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
subq   $512,%rsp

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
xorq    %r13,%r13
movq    256(%rsp),%rdx    

mulx    352(%rsp),%r8,%r9
mulx    360(%rsp),%rcx,%r10
adcx    %rcx,%r9     

mulx    368(%rsp),%rcx,%r11
adcx    %rcx,%r10    

mulx    376(%rsp),%rcx,%r12
adcx    %rcx,%r11
adcx    %r13,%r12

xorq    %r14,%r14
movq    264(%rsp),%rdx
   
mulx    352(%rsp),%rcx,%rbp
adcx    %rcx,%r9
adox    %rbp,%r10
    
mulx    360(%rsp),%rcx,%rbp
adcx    %rcx,%r10
adox    %rbp,%r11
    
mulx    368(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    376(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
adcx    %r14,%r13

xorq    %r15,%r15
movq    272(%rsp),%rdx
    
mulx    352(%rsp),%rcx,%rbp
adcx    %rcx,%r10
adox    %rbp,%r11
    
mulx    360(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    368(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
    
mulx    376(%rsp),%rcx,%rbp
adcx    %rcx,%r13
adox    %rbp,%r14
adcx    %r15,%r14

xorq    %rax,%rax
movq    280(%rsp),%rdx
    
mulx    352(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    360(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
    
mulx    368(%rsp),%rcx,%rbp
adcx    %rcx,%r13
adox    %rbp,%r14
    
mulx    376(%rsp),%rcx,%rbp
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

movq    %r8,352(%rsp)
movq    %r9,360(%rsp)
movq    %r10,368(%rsp)
movq    %r11,376(%rsp)

// mul
xorq    %r13,%r13
movq    288(%rsp),%rdx    

mulx    384(%rsp),%r8,%r9
mulx    392(%rsp),%rcx,%r10
adcx    %rcx,%r9     

mulx    400(%rsp),%rcx,%r11
adcx    %rcx,%r10    

mulx    408(%rsp),%rcx,%r12
adcx    %rcx,%r11
adcx    %r13,%r12

xorq    %r14,%r14
movq    296(%rsp),%rdx
   
mulx    384(%rsp),%rcx,%rbp
adcx    %rcx,%r9
adox    %rbp,%r10
    
mulx    392(%rsp),%rcx,%rbp
adcx    %rcx,%r10
adox    %rbp,%r11
    
mulx    400(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    408(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
adcx    %r14,%r13

xorq    %r15,%r15
movq    304(%rsp),%rdx
    
mulx    384(%rsp),%rcx,%rbp
adcx    %rcx,%r10
adox    %rbp,%r11
    
mulx    392(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    400(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
    
mulx    408(%rsp),%rcx,%rbp
adcx    %rcx,%r13
adox    %rbp,%r14
adcx    %r15,%r14

xorq    %rax,%rax
movq    312(%rsp),%rdx
    
mulx    384(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    392(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
    
mulx    400(%rsp),%rcx,%rbp
adcx    %rcx,%r13
adox    %rbp,%r14
    
mulx    408(%rsp),%rcx,%rbp
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
xorq    %r13,%r13
movq    320(%rsp),%rdx    

mulx    224(%rsp),%r8,%r9
mulx    232(%rsp),%rcx,%r10
adcx    %rcx,%r9     

mulx    240(%rsp),%rcx,%r11
adcx    %rcx,%r10    

mulx    248(%rsp),%rcx,%r12
adcx    %rcx,%r11
adcx    %r13,%r12

xorq    %r14,%r14
movq    328(%rsp),%rdx
   
mulx    224(%rsp),%rcx,%rbp
adcx    %rcx,%r9
adox    %rbp,%r10
    
mulx    232(%rsp),%rcx,%rbp
adcx    %rcx,%r10
adox    %rbp,%r11
    
mulx    240(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    248(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
adcx    %r14,%r13

xorq    %r15,%r15
movq    336(%rsp),%rdx
    
mulx    224(%rsp),%rcx,%rbp
adcx    %rcx,%r10
adox    %rbp,%r11
    
mulx    232(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    240(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
    
mulx    248(%rsp),%rcx,%rbp
adcx    %rcx,%r13
adox    %rbp,%r14
adcx    %r15,%r14

xorq    %rax,%rax
movq    344(%rsp),%rdx
    
mulx    224(%rsp),%rcx,%rbp
adcx    %rcx,%r11
adox    %rbp,%r12
    
mulx    232(%rsp),%rcx,%rbp
adcx    %rcx,%r12
adox    %rbp,%r13
    
mulx    240(%rsp),%rcx,%rbp
adcx    %rcx,%r13
adox    %rbp,%r14
    
mulx    248(%rsp),%rcx,%rbp
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

// convert to 5x4 form
vmovdqa   384(%rsp),%ymm8
vmovdqa   352(%rsp),%ymm9
vmovdqa   416(%rsp),%ymm10
vmovdqa   352(%rsp),%ymm11

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
vmovdqa   448(%rsp),%ymm0
vmovdqa   416(%rsp),%ymm1
vmovdqa   448(%rsp),%ymm2
vmovdqa   384(%rsp),%ymm3

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

vmovdqa   %ymm0,128(%rsp)
vmovdqa   %ymm1,160(%rsp)
vmovdqa   %ymm2,192(%rsp)
vmovdqa   %ymm3,224(%rsp)

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
