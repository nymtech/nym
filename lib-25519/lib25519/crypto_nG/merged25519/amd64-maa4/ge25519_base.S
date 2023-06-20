// linker define base
// linker use mask63

/* Assembly for fixed base scalar multiplication. 
 * 
 * This assembly has been developed after studying the 
 * amd64-64-24k implementation of the work "High speed 
 * high security signatures" by Bernstein et al.
*/

#define mask63 CRYPTO_SHARED_NAMESPACE(mask63)

.p2align 5
.globl _CRYPTO_SHARED_NAMESPACE(base)
.globl CRYPTO_SHARED_NAMESPACE(base)
_CRYPTO_SHARED_NAMESPACE(base):
CRYPTO_SHARED_NAMESPACE(base):

movq   %rsp,%r11
andq   $-32,%rsp
subq   $464,%rsp

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

movq   %r11,104(%rsp)
movq   %r12,112(%rsp)
movq   %r13,120(%rsp)
movq   %r14,128(%rsp)

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

movq   %rsi,136(%rsp)
movq   %r9,144(%rsp)
movq   %rax,152(%rsp)
movq   %r10,160(%rsp)

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

movq   %rsi,200(%rsp)
movq   %r9,208(%rsp)
movq   %rax,216(%rsp)
movq   %r10,224(%rsp)

movq   $2,168(%rsp)
movq   $0,176(%rsp)
movq   $0,184(%rsp)
movq   $0,192(%rsp)

/* loop: i=1,i<64,i=i+1 */

movq    $1,232(%rsp)

.L:

/* choose t */

movq    72(%rsp),%rcx
movq    232(%rsp),%rsi
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

movq    %rsi,240(%rsp)
movq    %r9,248(%rsp)
movq    %rax,256(%rsp)
movq    %r10,264(%rsp)
movq    %r11,272(%rsp)
movq    %r12,280(%rsp)
movq    %r13,288(%rsp)
movq    %r14,296(%rsp)

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

movq    %rsi,304(%rsp)
movq    %r9,312(%rsp)
movq    %rax,320(%rsp)
movq    %r10,328(%rsp)

/* nielsadd2 */

// load
movq    136(%rsp),%rdx
movq    144(%rsp),%rcx
movq    152(%rsp),%r8
movq    160(%rsp),%r9

// copy
movq    %rdx,%rax
movq    %rcx,%r10
movq    %r8,%r11
movq    %r9,%r12

// sub
subq    104(%rsp),%rdx
sbbq    112(%rsp),%rcx
sbbq    120(%rsp),%r8
sbbq    128(%rsp),%r9

movq    $0,%r13
movq    $38,%r14
cmovae  %r13,%r14

subq    %r14,%rdx
sbbq    %r13,%rcx
sbbq    %r13,%r8
sbbq    %r13,%r9

cmovc   %r14,%r13
subq    %r13,%rdx

movq    %rdx,336(%rsp)
movq    %rcx,344(%rsp)
movq    %r8,352(%rsp)
movq    %r9,360(%rsp)

// add
addq    104(%rsp),%rax
adcq    112(%rsp),%r10
adcq    120(%rsp),%r11
adcq    128(%rsp),%r12

movq    $0,%r13
movq    $38,%r14
cmovae  %r13,%r14

addq    %r14,%rax
adcq    %r13,%r10
adcq    %r13,%r11
adcq    %r13,%r12

cmovc   %r14,%r13
addq    %r13,%rax

movq    %rax,368(%rsp)
movq    %r10,376(%rsp)
movq    %r11,384(%rsp)
movq    %r12,392(%rsp)

// mul
movq    248(%rsp),%rax
mulq    360(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    256(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    264(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    256(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    264(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    264(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    240(%rsp),%rax
mulq    360(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    248(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    256(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    264(%rsp),%rax
mulq    336(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    240(%rsp),%rax
mulq    336(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    240(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    248(%rsp),%rax
mulq    336(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    240(%rsp),%rax
mulq    352(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    248(%rsp),%rax
mulq    344(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    256(%rsp),%rax
mulq    336(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,336(%rsp)
movq    %r10,344(%rsp)
movq    %r12,352(%rsp)
movq    %r14,360(%rsp)

// mul
movq    280(%rsp),%rax
mulq    392(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    288(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    296(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    288(%rsp),%rax
mulq    392(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    296(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    296(%rsp),%rax
mulq    392(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    272(%rsp),%rax
mulq    392(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    280(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    288(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    296(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    272(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    272(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    280(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    272(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    280(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    288(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

// copy
movq    %r8,%r9
movq    %r10,%r11
movq    %r12,%r13
movq    %r14,%r15

// sub
subq    336(%rsp),%r8
sbbq    344(%rsp),%r10
sbbq    352(%rsp),%r12
sbbq    360(%rsp),%r14

movq    $0,%rax
movq    $38,%rbx
cmovae  %rax,%rbx

subq    %rbx,%r8
sbbq    %rax,%r10
sbbq    %rax,%r12
sbbq    %rax,%r14

cmovc   %rbx,%rax
subq    %rax,%r8

movq    %r8,368(%rsp)
movq    %r10,376(%rsp)
movq    %r12,384(%rsp)
movq    %r14,392(%rsp)

// add
addq    336(%rsp),%r9
adcq    344(%rsp),%r11
adcq    352(%rsp),%r13
adcq    360(%rsp),%r15

movq    $0,%rax
movq    $38,%rbx
cmovae  %rax,%rbx

addq    %rbx,%r9
adcq    %rax,%r11
adcq    %rax,%r13
adcq    %rax,%r15

cmovc   %rbx,%rax
addq    %rax,%r9

movq    %r9,336(%rsp)
movq    %r11,344(%rsp)
movq    %r13,352(%rsp)
movq    %r15,360(%rsp)

// mul
movq    312(%rsp),%rax
mulq    224(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    320(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    328(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    320(%rsp),%rax
mulq    224(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    328(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    328(%rsp),%rax
mulq    224(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    304(%rsp),%rax
mulq    224(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    312(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    320(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    328(%rsp),%rax
mulq    200(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    304(%rsp),%rax
mulq    200(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    304(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    312(%rsp),%rax
mulq    200(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    304(%rsp),%rax
mulq    216(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    312(%rsp),%rax
mulq    208(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    320(%rsp),%rax
mulq    200(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    168(%rsp),%r9
movq    176(%rsp),%r11
movq    184(%rsp),%r13
movq    192(%rsp),%r15

// double
addq    %r9,%r9
adcq    %r11,%r11
adcq    %r13,%r13
adcq    %r15,%r15

movq    $0,%rbx
movq    $38,%rax
cmovae  %rbx,%rax

addq    %rax,%r9
adcq    %rbx,%r11
adcq    %rbx,%r13
adcq    %rbx,%r15

cmovc   %rax,%rbx
addq    %rbx,%r9

// copy
movq    %r9,%rsi
movq    %r11,%rax
movq    %r13,%rcx
movq    %r15,%rdx

// sub
subq    %r8,%r9
sbbq    %r10,%r11
sbbq    %r12,%r13
sbbq    %r14,%r15

movq    $0,%rbx
movq    $38,%rbp
cmovae  %rbx,%rbp

subq    %rbp,%r9
sbbq    %rbx,%r11
sbbq    %rbx,%r13
sbbq    %rbx,%r15

cmovc   %rbp,%rbx
subq    %rbx,%r9

movq    %r9,432(%rsp)
movq    %r11,440(%rsp)
movq    %r13,448(%rsp)
movq    %r15,456(%rsp)

// add
addq    %r8,%rsi
adcq    %r10,%rax
adcq    %r12,%rcx
adcq    %r14,%rdx

movq    $0,%rbx
movq    $38,%rbp
cmovae  %rbx,%rbp

addq    %rbp,%rsi
adcq    %rbx,%rax
adcq    %rbx,%rcx
adcq    %rbx,%rdx

cmovc   %rbp,%rbx
addq    %rbx,%rsi

movq    %rsi,400(%rsp)
movq    %rax,408(%rsp)
movq    %rcx,416(%rsp)
movq    %rdx,424(%rsp)

/* p1p1 to p3 */

// mul
movq    376(%rsp),%rax
mulq    456(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    384(%rsp),%rax
mulq    448(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    392(%rsp),%rax
mulq    440(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    384(%rsp),%rax
mulq    456(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    392(%rsp),%rax
mulq    448(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    392(%rsp),%rax
mulq    456(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    368(%rsp),%rax
mulq    456(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    376(%rsp),%rax
mulq    448(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    384(%rsp),%rax
mulq    440(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    392(%rsp),%rax
mulq    432(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    368(%rsp),%rax
mulq    432(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    368(%rsp),%rax
mulq    440(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    376(%rsp),%rax
mulq    432(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    368(%rsp),%rax
mulq    448(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    376(%rsp),%rax
mulq    440(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    384(%rsp),%rax
mulq    432(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,104(%rsp)
movq    %r10,112(%rsp)
movq    %r12,120(%rsp)
movq    %r14,128(%rsp)

// mul
movq    344(%rsp),%rax
mulq    424(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    352(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    360(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    352(%rsp),%rax
mulq    424(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    360(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    360(%rsp),%rax
mulq    424(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    336(%rsp),%rax
mulq    424(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    344(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    352(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    360(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    336(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    336(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    344(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    336(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    344(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    352(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,136(%rsp)
movq    %r10,144(%rsp)
movq    %r12,152(%rsp)
movq    %r14,160(%rsp)

// mul
movq    440(%rsp),%rax
mulq    424(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    448(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    456(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    448(%rsp),%rax
mulq    424(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    456(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    456(%rsp),%rax
mulq    424(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    432(%rsp),%rax
mulq    424(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    440(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    448(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    456(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    432(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    432(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    440(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    432(%rsp),%rax
mulq    416(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    440(%rsp),%rax
mulq    408(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    448(%rsp),%rax
mulq    400(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,168(%rsp)
movq    %r10,176(%rsp)
movq    %r12,184(%rsp)
movq    %r14,192(%rsp)

// mul
movq    344(%rsp),%rax
mulq    392(%rsp)
movq    %rax,%r8
xorq    %r9,%r9
movq    %rdx,%r10
xorq    %r11,%r11

movq    352(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    360(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    352(%rsp),%rax
mulq    392(%rsp)
addq    %rax,%r10
adcq    $0,%r11
movq    %rdx,%r12
xorq    %r13,%r13

movq    360(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %r10
imul    $38,%r11,%r11
movq    %rax,%r10
addq    %rdx,%r11

movq    360(%rsp),%rax
mulq    392(%rsp)
addq    %rax,%r12
adcq    $0,%r13

movq    $38,%rax
mulq    %rdx
movq    %rax,%r14
movq    %rdx,%r15

movq    $38,%rax
mulq    %r12
imul    $38,%r13,%r13
movq    %rax,%r12
addq    %rdx,%r13

movq    336(%rsp),%rax
mulq    392(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    344(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    352(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    360(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r14
adcq    $0,%r15
addq    %rdx,%r8
adcq    $0,%r9

movq    $38,%rax
mulq    %r8
imul    $38,%r9,%r9
movq    %rax,%r8
addq    %rdx,%r9

movq    336(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r8
adcq    $0,%r9
addq    %rdx,%r10
adcq    $0,%r11

movq    336(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    344(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r10
adcq    $0,%r11
addq    %rdx,%r12
adcq    $0,%r13

movq    336(%rsp),%rax
mulq    384(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    344(%rsp),%rax
mulq    376(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

movq    352(%rsp),%rax
mulq    368(%rsp)
addq    %rax,%r12
adcq    $0,%r13
addq    %rdx,%r14
adcq    $0,%r15

addq    %r9,%r10
adcq    $0,%r11

addq    %r11,%r12
adcq    $0,%r13

addq    %r13,%r14
adcq    $0,%r15

shld    $1,%r14,%r15
andq    mask63(%rip),%r14
imul    $19,%r15,%r15

addq    %r15,%r8
adcq    $0,%r10
adcq    $0,%r12
adcq    $0,%r14

movq    %r8,200(%rsp)
movq    %r10,208(%rsp)
movq    %r12,216(%rsp)
movq    %r14,224(%rsp)

movq    232(%rsp),%r15
addq    $1,%r15
movq    %r15,232(%rsp)
cmpq	$63,%r15

jle     .L

/* store final value of r */

movq    56(%rsp),%rdi

movq    104(%rsp),%r8
movq    112(%rsp),%r9
movq    120(%rsp),%r10
movq    128(%rsp),%r11

movq    %r8,0(%rdi)
movq    %r9,8(%rdi)
movq    %r10,16(%rdi)
movq    %r11,24(%rdi)

movq    136(%rsp),%r8
movq    144(%rsp),%r9
movq    152(%rsp),%r10
movq    160(%rsp),%r11

movq    %r8,32(%rdi)
movq    %r9,40(%rdi)
movq    %r10,48(%rdi)
movq    %r11,56(%rdi)

movq    168(%rsp),%r8
movq    176(%rsp),%r9
movq    184(%rsp),%r10
movq    192(%rsp),%r11

movq    %r8,64(%rdi)
movq    %r9,72(%rdi)
movq    %r10,80(%rdi)
movq    %r11,88(%rdi)

movq    200(%rsp),%r8
movq    208(%rsp),%r9
movq    216(%rsp),%r10
movq    224(%rsp),%r11

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
