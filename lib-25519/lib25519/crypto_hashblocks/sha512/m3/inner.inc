typedef unsigned long long uint64;

#define SHR(x,c) ((x) >> (c))
#define ROTR(x,c) (((x) >> (c)) | ((x) << (64 - (c))))
#define sigma0(x) (ROTR(x, 1) ^ ROTR(x, 8) ^ SHR(x,7))
#define sigma1(x) (ROTR(x,19) ^ ROTR(x,61) ^ SHR(x,6))
#
#define flip(x) ROTR(x,32)

#define Ch(x,y,z) (z ^ (x & (y ^ z)))
#define Maj(x,y,z) ((x & (y ^ z)) ^ (y & z))
#define Sigma0(x) (ROTR(x,28) ^ ROTR(x,34) ^ ROTR(x,39))
#define Sigma1(x) (ROTR(x,14) ^ ROTR(x,18) ^ ROTR(x,41))

#define reverse(x) __builtin_bswap64(x)
#define load64(x) *(uint64 *) (x)
#define store64(x,y) *(uint64 *) (x) = y
#define eq(x,y) (x == y)
#define ge(x,y) (x >= y)

// qhasm: ptr input_0
// qhasm: ptr input_1
// qhasm: int32 input_2
// qhasm: ptr input_3
// qhasm: spill64 r0_spill
// qhasm: spill64 r1_spill
// qhasm: spill64 r2_spill
// qhasm: spill64 r3_spill
// qhasm: spill64 r4_spill
// qhasm: spill64 r5_spill
// qhasm: spill64 r6_spill
// qhasm: spill64 r7_spill
// qhasm: spill64 w0_spill
// qhasm: spill64 w1_spill
// qhasm: spill64 w2_spill
// qhasm: spill64 w3_spill
// qhasm: spill64 w4_spill
// qhasm: spill64 w5_spill
// qhasm: spill64 w6_spill
// qhasm: spill64 w7_spill
// qhasm: stack64 w8_stack
// qhasm: stack64 state0
// qhasm: stack64 state1
// qhasm: stack64 state2
// qhasm: stack64 state3
// qhasm: stack64 state4
// qhasm: stack64 state5
// qhasm: stack64 state6
// qhasm: stack64 state7
// qhasm: int64 r0
// qhasm: int64 r1
// qhasm: int64 r2
// qhasm: int64 r3
// qhasm: int64 r4
// qhasm: int64 r5
// qhasm: int64 r6
// qhasm: int64 r7
// qhasm: int64 w0
// qhasm: int64 w1
// qhasm: int64 w2
// qhasm: int64 w3
// qhasm: int64 w4
// qhasm: int64 w5
// qhasm: int64 w6
// qhasm: int64 w7
// qhasm: stack64 w0_next
// qhasm: stack64 w1_next
// qhasm: stack64 w2_next
// qhasm: stack64 w3_next
// qhasm: stack64 w4_next
// qhasm: stack64 w5_next
// qhasm: stack64 w6_next
// qhasm: stack64 w7_next
// qhasm: int64 w8
// qhasm: int64 w9
// qhasm: int64 w10
// qhasm: int64 w11
// qhasm: int64 w12
// qhasm: int64 w13
// qhasm: int64 w14
// qhasm: int64 w15
// qhasm: int32 i
// qhasm: ptr statebytes
// qhasm: ptr in
// qhasm: int32 inlen
// qhasm: ptr constants
// qhasm: stackptr in_stack
// qhasm: stackptr statebytes_stack
// qhasm: stack32 inlen_stack
// qhasm: stack32 i_stack
// qhasm: pushenter inner
int inner(unsigned char *p0,unsigned char *p1,unsigned int i2,unsigned char *p3)
{
int equal;
int unsignedge;
unsigned char *p2;
unsigned int i0,i1,i3,i4,i5,i6,i7,i8,i9;
uint64 u0,u1,u2,u3,u4,u5,u6,u7;
uint64 m0,m1,m2,m3,m4,m5,m6,m7,m8,m9,m10,m11,m12,m13,m14,m15,m16,m17,m18;
unsigned char *n0,*n1,*n2;
unsigned int o0,o1,o2;
uint64 d0,d1,d2,d3,d4,d5,d6,d7,d8,d9,d10,d11,d12,d13,d14,d15,d16;
// qhasm: statebytes_stack = input_0
// asm 1: >statebytes_stack=stackptr#1 = <input_0=ptr#1;
// asm 2: >statebytes_stack=n0 = <input_0=p0;
n0 = p0;
// qhasm: in_stack = input_1
// asm 1: >in_stack=stackptr#2 = <input_1=ptr#2;
// asm 2: >in_stack=n1 = <input_1=p1;
n1 = p1;
// qhasm: inlen = input_2 - 128
// asm 1: >inlen=int32#1 = <input_2=int32#3 - 128;
// asm 2: >inlen=i0 = <input_2=i2 - 128;
i0 = i2 - 128;
// qhasm: inlen_stack = inlen
// asm 1: >inlen_stack=stack32#1 = <inlen=int32#1;
// asm 2: >inlen_stack=o0 = <inlen=i0;
o0 = i0;
// qhasm: r0 = flip mem64[input_0]
// asm 1: >r0=int64#1 = flip(load64(<input_0=ptr#1));
// asm 2: >r0=u0 = flip(load64(<input_0=p0));
u0 = flip(load64(p0));
// qhasm: r1 = flip mem64[input_0+8]
// asm 1: >r1=int64#2 = flip(load64(<input_0=ptr#1+8));
// asm 2: >r1=u1 = flip(load64(<input_0=p0+8));
u1 = flip(load64(p0+8));
// qhasm: r2 = flip mem64[input_0+16]
// asm 1: >r2=int64#3 = flip(load64(<input_0=ptr#1+16));
// asm 2: >r2=u2 = flip(load64(<input_0=p0+16));
u2 = flip(load64(p0+16));
// qhasm: r3 = flip mem64[input_0+24]
// asm 1: >r3=int64#4 = flip(load64(<input_0=ptr#1+24));
// asm 2: >r3=u3 = flip(load64(<input_0=p0+24));
u3 = flip(load64(p0+24));
// qhasm: r0 = reverse flip r0
// asm 1: >r0=int64#1 = reverse(flip(<r0=int64#1));
// asm 2: >r0=u0 = reverse(flip(<r0=u0));
u0 = reverse(flip(u0));
// qhasm: r1 = reverse flip r1
// asm 1: >r1=int64#2 = reverse(flip(<r1=int64#2));
// asm 2: >r1=u1 = reverse(flip(<r1=u1));
u1 = reverse(flip(u1));
// qhasm: r2 = reverse flip r2
// asm 1: >r2=int64#3 = reverse(flip(<r2=int64#3));
// asm 2: >r2=u2 = reverse(flip(<r2=u2));
u2 = reverse(flip(u2));
// qhasm: r3 = reverse flip r3
// asm 1: >r3=int64#4 = reverse(flip(<r3=int64#4));
// asm 2: >r3=u3 = reverse(flip(<r3=u3));
u3 = reverse(flip(u3));
// qhasm: state0 = r0
// asm 1: >state0=stack64#1 = <r0=int64#1;
// asm 2: >state0=m0 = <r0=u0;
m0 = u0;
// qhasm: state1 = r1
// asm 1: >state1=stack64#2 = <r1=int64#2;
// asm 2: >state1=m1 = <r1=u1;
m1 = u1;
// qhasm: state2 = r2
// asm 1: >state2=stack64#3 = <r2=int64#3;
// asm 2: >state2=m2 = <r2=u2;
m2 = u2;
// qhasm: state3 = r3
// asm 1: >state3=stack64#4 = <r3=int64#4;
// asm 2: >state3=m3 = <r3=u3;
m3 = u3;
// qhasm: r0_spill = r0
// asm 1: >r0_spill=spill64#1 = <r0=int64#1;
// asm 2: >r0_spill=d0 = <r0=u0;
d0 = u0;
// qhasm: r1_spill = r1
// asm 1: >r1_spill=spill64#2 = <r1=int64#2;
// asm 2: >r1_spill=d1 = <r1=u1;
d1 = u1;
// qhasm: r2_spill = r2
// asm 1: >r2_spill=spill64#3 = <r2=int64#3;
// asm 2: >r2_spill=d2 = <r2=u2;
d2 = u2;
// qhasm: r3_spill = r3
// asm 1: >r3_spill=spill64#4 = <r3=int64#4;
// asm 2: >r3_spill=d3 = <r3=u3;
d3 = u3;
// qhasm: r4 = flip mem64[input_0+32]
// asm 1: >r4=int64#1 = flip(load64(<input_0=ptr#1+32));
// asm 2: >r4=u0 = flip(load64(<input_0=p0+32));
u0 = flip(load64(p0+32));
// qhasm: r5 = flip mem64[input_0+40]
// asm 1: >r5=int64#2 = flip(load64(<input_0=ptr#1+40));
// asm 2: >r5=u1 = flip(load64(<input_0=p0+40));
u1 = flip(load64(p0+40));
// qhasm: r6 = flip mem64[input_0+48]
// asm 1: >r6=int64#3 = flip(load64(<input_0=ptr#1+48));
// asm 2: >r6=u2 = flip(load64(<input_0=p0+48));
u2 = flip(load64(p0+48));
// qhasm: r7 = flip mem64[input_0+56]
// asm 1: >r7=int64#4 = flip(load64(<input_0=ptr#1+56));
// asm 2: >r7=u3 = flip(load64(<input_0=p0+56));
u3 = flip(load64(p0+56));
// qhasm: r4 = reverse flip r4
// asm 1: >r4=int64#1 = reverse(flip(<r4=int64#1));
// asm 2: >r4=u0 = reverse(flip(<r4=u0));
u0 = reverse(flip(u0));
// qhasm: r5 = reverse flip r5
// asm 1: >r5=int64#2 = reverse(flip(<r5=int64#2));
// asm 2: >r5=u1 = reverse(flip(<r5=u1));
u1 = reverse(flip(u1));
// qhasm: r6 = reverse flip r6
// asm 1: >r6=int64#3 = reverse(flip(<r6=int64#3));
// asm 2: >r6=u2 = reverse(flip(<r6=u2));
u2 = reverse(flip(u2));
// qhasm: r7 = reverse flip r7
// asm 1: >r7=int64#4 = reverse(flip(<r7=int64#4));
// asm 2: >r7=u3 = reverse(flip(<r7=u3));
u3 = reverse(flip(u3));
// qhasm: state4 = r4
// asm 1: >state4=stack64#5 = <r4=int64#1;
// asm 2: >state4=m4 = <r4=u0;
m4 = u0;
// qhasm: state5 = r5
// asm 1: >state5=stack64#6 = <r5=int64#2;
// asm 2: >state5=m5 = <r5=u1;
m5 = u1;
// qhasm: state6 = r6
// asm 1: >state6=stack64#7 = <r6=int64#3;
// asm 2: >state6=m6 = <r6=u2;
m6 = u2;
// qhasm: state7 = r7
// asm 1: >state7=stack64#8 = <r7=int64#4;
// asm 2: >state7=m7 = <r7=u3;
m7 = u3;
// qhasm: r4_spill = r4
// asm 1: >r4_spill=spill64#5 = <r4=int64#1;
// asm 2: >r4_spill=d4 = <r4=u0;
d4 = u0;
// qhasm: r5_spill = r5
// asm 1: >r5_spill=spill64#6 = <r5=int64#2;
// asm 2: >r5_spill=d5 = <r5=u1;
d5 = u1;
// qhasm: r6_spill = r6
// asm 1: >r6_spill=spill64#7 = <r6=int64#3;
// asm 2: >r6_spill=d6 = <r6=u2;
d6 = u2;
// qhasm: r7_spill = r7
// asm 1: >r7_spill=spill64#8 = <r7=int64#4;
// asm 2: >r7_spill=d7 = <r7=u3;
d7 = u3;
// qhasm: constants = input_3
// asm 1: >constants=ptr#1 = <input_3=ptr#4;
// asm 2: >constants=p0 = <input_3=p3;
p0 = p3;
// qhasm: mainloop:
label_mainloop:
// qhasm:   in = in_stack
// asm 1: >in=ptr#2 = <in_stack=stackptr#2;
// asm 2: >in=p1 = <in_stack=n1;
p1 = n1;
// qhasm:   w0 = flip mem64[in]; in += 8
// asm 1: >w0=int64#1 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w0=u0 = flip(load64(<in=p1)); <in=p1 += 8;
u0 = flip(load64(p1)); p1 += 8;
// qhasm:   w1 = flip mem64[in]; in += 8
// asm 1: >w1=int64#2 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w1=u1 = flip(load64(<in=p1)); <in=p1 += 8;
u1 = flip(load64(p1)); p1 += 8;
// qhasm:   w2 = flip mem64[in]; in += 8
// asm 1: >w2=int64#3 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w2=u2 = flip(load64(<in=p1)); <in=p1 += 8;
u2 = flip(load64(p1)); p1 += 8;
// qhasm:   w3 = flip mem64[in]; in += 8
// asm 1: >w3=int64#4 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w3=u3 = flip(load64(<in=p1)); <in=p1 += 8;
u3 = flip(load64(p1)); p1 += 8;
// qhasm:   w0 = reverse flip w0
// asm 1: >w0=int64#1 = reverse(flip(<w0=int64#1));
// asm 2: >w0=u0 = reverse(flip(<w0=u0));
u0 = reverse(flip(u0));
// qhasm:   w1 = reverse flip w1
// asm 1: >w1=int64#2 = reverse(flip(<w1=int64#2));
// asm 2: >w1=u1 = reverse(flip(<w1=u1));
u1 = reverse(flip(u1));
// qhasm:   w2 = reverse flip w2
// asm 1: >w2=int64#3 = reverse(flip(<w2=int64#3));
// asm 2: >w2=u2 = reverse(flip(<w2=u2));
u2 = reverse(flip(u2));
// qhasm:   w3 = reverse flip w3
// asm 1: >w3=int64#4 = reverse(flip(<w3=int64#4));
// asm 2: >w3=u3 = reverse(flip(<w3=u3));
u3 = reverse(flip(u3));
// qhasm:   w0_spill = w0
// asm 1: >w0_spill=spill64#9 = <w0=int64#1;
// asm 2: >w0_spill=d8 = <w0=u0;
d8 = u0;
// qhasm:   w1_spill = w1
// asm 1: >w1_spill=spill64#10 = <w1=int64#2;
// asm 2: >w1_spill=d9 = <w1=u1;
d9 = u1;
// qhasm:   w2_spill = w2
// asm 1: >w2_spill=spill64#11 = <w2=int64#3;
// asm 2: >w2_spill=d10 = <w2=u2;
d10 = u2;
// qhasm:   w3_spill = w3
// asm 1: >w3_spill=spill64#12 = <w3=int64#4;
// asm 2: >w3_spill=d11 = <w3=u3;
d11 = u3;
// qhasm:   w4 = flip mem64[in]; in += 8
// asm 1: >w4=int64#1 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w4=u0 = flip(load64(<in=p1)); <in=p1 += 8;
u0 = flip(load64(p1)); p1 += 8;
// qhasm:   w5 = flip mem64[in]; in += 8
// asm 1: >w5=int64#2 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w5=u1 = flip(load64(<in=p1)); <in=p1 += 8;
u1 = flip(load64(p1)); p1 += 8;
// qhasm:   w6 = flip mem64[in]; in += 8
// asm 1: >w6=int64#3 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w6=u2 = flip(load64(<in=p1)); <in=p1 += 8;
u2 = flip(load64(p1)); p1 += 8;
// qhasm:   w7 = flip mem64[in]; in += 8
// asm 1: >w7=int64#4 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w7=u3 = flip(load64(<in=p1)); <in=p1 += 8;
u3 = flip(load64(p1)); p1 += 8;
// qhasm:   w4 = reverse flip w4
// asm 1: >w4=int64#1 = reverse(flip(<w4=int64#1));
// asm 2: >w4=u0 = reverse(flip(<w4=u0));
u0 = reverse(flip(u0));
// qhasm:   w5 = reverse flip w5
// asm 1: >w5=int64#2 = reverse(flip(<w5=int64#2));
// asm 2: >w5=u1 = reverse(flip(<w5=u1));
u1 = reverse(flip(u1));
// qhasm:   w6 = reverse flip w6
// asm 1: >w6=int64#3 = reverse(flip(<w6=int64#3));
// asm 2: >w6=u2 = reverse(flip(<w6=u2));
u2 = reverse(flip(u2));
// qhasm:   w7 = reverse flip w7
// asm 1: >w7=int64#4 = reverse(flip(<w7=int64#4));
// asm 2: >w7=u3 = reverse(flip(<w7=u3));
u3 = reverse(flip(u3));
// qhasm:   w4_spill = w4
// asm 1: >w4_spill=spill64#13 = <w4=int64#1;
// asm 2: >w4_spill=d12 = <w4=u0;
d12 = u0;
// qhasm:   w5_spill = w5
// asm 1: >w5_spill=spill64#14 = <w5=int64#2;
// asm 2: >w5_spill=d13 = <w5=u1;
d13 = u1;
// qhasm:   w6_spill = w6
// asm 1: >w6_spill=spill64#15 = <w6=int64#3;
// asm 2: >w6_spill=d14 = <w6=u2;
d14 = u2;
// qhasm:   w7_spill = w7
// asm 1: >w7_spill=spill64#16 = <w7=int64#4;
// asm 2: >w7_spill=d15 = <w7=u3;
d15 = u3;
// qhasm:   w8 = flip mem64[in]; in += 8
// asm 1: >w8=int64#1 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w8=u0 = flip(load64(<in=p1)); <in=p1 += 8;
u0 = flip(load64(p1)); p1 += 8;
// qhasm:   w9 = flip mem64[in]; in += 8
// asm 1: >w9=int64#2 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w9=u1 = flip(load64(<in=p1)); <in=p1 += 8;
u1 = flip(load64(p1)); p1 += 8;
// qhasm:   w10 = flip mem64[in]; in += 8
// asm 1: >w10=int64#3 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w10=u2 = flip(load64(<in=p1)); <in=p1 += 8;
u2 = flip(load64(p1)); p1 += 8;
// qhasm:   w11 = flip mem64[in]; in += 8
// asm 1: >w11=int64#4 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w11=u3 = flip(load64(<in=p1)); <in=p1 += 8;
u3 = flip(load64(p1)); p1 += 8;
// qhasm:   w8 = reverse flip w8
// asm 1: >w8=int64#1 = reverse(flip(<w8=int64#1));
// asm 2: >w8=u0 = reverse(flip(<w8=u0));
u0 = reverse(flip(u0));
// qhasm:   w9 = reverse flip w9
// asm 1: >w9=int64#2 = reverse(flip(<w9=int64#2));
// asm 2: >w9=u1 = reverse(flip(<w9=u1));
u1 = reverse(flip(u1));
// qhasm:   w10 = reverse flip w10
// asm 1: >w10=int64#3 = reverse(flip(<w10=int64#3));
// asm 2: >w10=u2 = reverse(flip(<w10=u2));
u2 = reverse(flip(u2));
// qhasm:   w11 = reverse flip w11
// asm 1: >w11=int64#4 = reverse(flip(<w11=int64#4));
// asm 2: >w11=u3 = reverse(flip(<w11=u3));
u3 = reverse(flip(u3));
// qhasm:   w0_next = w8
// asm 1: >w0_next=stack64#9 = <w8=int64#1;
// asm 2: >w0_next=m8 = <w8=u0;
m8 = u0;
// qhasm:   w1_next = w9
// asm 1: >w1_next=stack64#10 = <w9=int64#2;
// asm 2: >w1_next=m9 = <w9=u1;
m9 = u1;
// qhasm:   w2_next = w10
// asm 1: >w2_next=stack64#11 = <w10=int64#3;
// asm 2: >w2_next=m10 = <w10=u2;
m10 = u2;
// qhasm:   w3_next = w11
// asm 1: >w3_next=stack64#12 = <w11=int64#4;
// asm 2: >w3_next=m11 = <w11=u3;
m11 = u3;
// qhasm:   w12 = flip mem64[in]; in += 8
// asm 1: >w12=int64#1 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w12=u0 = flip(load64(<in=p1)); <in=p1 += 8;
u0 = flip(load64(p1)); p1 += 8;
// qhasm:   w13 = flip mem64[in]; in += 8
// asm 1: >w13=int64#2 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w13=u1 = flip(load64(<in=p1)); <in=p1 += 8;
u1 = flip(load64(p1)); p1 += 8;
// qhasm:   w14 = flip mem64[in]; in += 8
// asm 1: >w14=int64#3 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w14=u2 = flip(load64(<in=p1)); <in=p1 += 8;
u2 = flip(load64(p1)); p1 += 8;
// qhasm:   w15 = flip mem64[in]; in += 8
// asm 1: >w15=int64#4 = flip(load64(<in=ptr#2)); <in=ptr#2 += 8;
// asm 2: >w15=u3 = flip(load64(<in=p1)); <in=p1 += 8;
u3 = flip(load64(p1)); p1 += 8;
// qhasm:   w12 = reverse flip w12
// asm 1: >w12=int64#1 = reverse(flip(<w12=int64#1));
// asm 2: >w12=u0 = reverse(flip(<w12=u0));
u0 = reverse(flip(u0));
// qhasm:   w13 = reverse flip w13
// asm 1: >w13=int64#2 = reverse(flip(<w13=int64#2));
// asm 2: >w13=u1 = reverse(flip(<w13=u1));
u1 = reverse(flip(u1));
// qhasm:   w14 = reverse flip w14
// asm 1: >w14=int64#3 = reverse(flip(<w14=int64#3));
// asm 2: >w14=u2 = reverse(flip(<w14=u2));
u2 = reverse(flip(u2));
// qhasm:   w15 = reverse flip w15
// asm 1: >w15=int64#4 = reverse(flip(<w15=int64#4));
// asm 2: >w15=u3 = reverse(flip(<w15=u3));
u3 = reverse(flip(u3));
// qhasm:   w4_next = w12
// asm 1: >w4_next=stack64#13 = <w12=int64#1;
// asm 2: >w4_next=m12 = <w12=u0;
m12 = u0;
// qhasm:   w5_next = w13
// asm 1: >w5_next=stack64#14 = <w13=int64#2;
// asm 2: >w5_next=m13 = <w13=u1;
m13 = u1;
// qhasm:   w6_next = w14
// asm 1: >w6_next=stack64#15 = <w14=int64#3;
// asm 2: >w6_next=m14 = <w14=u2;
m14 = u2;
// qhasm:   w7_next = w15
// asm 1: >w7_next=stack64#16 = <w15=int64#4;
// asm 2: >w7_next=m15 = <w15=u3;
m15 = u3;
// qhasm:   in_stack = in
// asm 1: >in_stack=stackptr#2 = <in=ptr#2;
// asm 2: >in_stack=n1 = <in=p1;
n1 = p1;
// qhasm:   i = 80 simple
// asm 1: >i=int32#1 = 80;
// asm 2: >i=i0 = 80;
i0 = 80;
// qhasm:   i_stack = i
// asm 1: >i_stack=stack32#2 = <i=int32#1;
// asm 2: >i_stack=o1 = <i=i0;
o1 = i0;
// qhasm:   innerloop:
label_innerloop:
// qhasm:     assign 0 to r0_spill
// qhasm:     assign 1 to r1_spill
// qhasm:     assign 2 to r2_spill
// qhasm:     assign 3 to r3_spill
// qhasm:     assign 4 to r4_spill
// qhasm:     assign 5 to r5_spill
// qhasm:     assign 6 to r6_spill
// qhasm:     assign 7 to r7_spill
// qhasm:       r3 = r3_spill
// asm 1: >r3=int64#1 = <r3_spill=spill64#4;
// asm 2: >r3=u0 = <r3_spill=d3;
u0 = d3;
// qhasm:       r4 = r4_spill
// asm 1: >r4=int64#2 = <r4_spill=spill64#5;
// asm 2: >r4=u1 = <r4_spill=d4;
u1 = d4;
// qhasm:       r5 = r5_spill
// asm 1: >r5=int64#3 = <r5_spill=spill64#6;
// asm 2: >r5=u2 = <r5_spill=d5;
u2 = d5;
// qhasm:       r6 = r6_spill
// asm 1: >r6=int64#4 = <r6_spill=spill64#7;
// asm 2: >r6=u3 = <r6_spill=d6;
u3 = d6;
// qhasm:       r7 = r7_spill
// asm 1: >r7=int64#5 = <r7_spill=spill64#8;
// asm 2: >r7=u4 = <r7_spill=d7;
u4 = d7;
// qhasm:       w0 = w0_spill
// asm 1: >w0=int64#6 = <w0_spill=spill64#9;
// asm 2: >w0=u5 = <w0_spill=d8;
u5 = d8;
// qhasm:     r7 += w0 + mem64[constants] + Sigma1(r4) + Ch(r4,r5,r6); constants += 8
// asm 1: <r7=int64#5 += <w0=int64#6 + load64(<constants=ptr#1) + Sigma1(<r4=int64#2) + Ch(<r4=int64#2,<r5=int64#3,<r6=int64#4);
// asm 2: <r7=u4 += <w0=u5 + load64(<constants=p0) + Sigma1(<r4=u1) + Ch(<r4=u1,<r5=u2,<r6=u3);
u4 += u5 + load64(p0) + Sigma1(u1) + Ch(u1,u2,u3);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r3 += r7
// asm 1: <r3=int64#1 += <r7=int64#5;
// asm 2: <r3=u0 += <r7=u4;
u0 += u4;
// qhasm:       r3_spill = r3
// asm 1: >r3_spill=spill64#4 = <r3=int64#1;
// asm 2: >r3_spill=d3 = <r3=u0;
d3 = u0;
// qhasm:       r0 = r0_spill
// asm 1: >r0=int64#2 = <r0_spill=spill64#1;
// asm 2: >r0=u1 = <r0_spill=d0;
u1 = d0;
// qhasm:       r1 = r1_spill
// asm 1: >r1=int64#3 = <r1_spill=spill64#2;
// asm 2: >r1=u2 = <r1_spill=d1;
u2 = d1;
// qhasm:       r2 = r2_spill
// asm 1: >r2=int64#4 = <r2_spill=spill64#3;
// asm 2: >r2=u3 = <r2_spill=d2;
u3 = d2;
// qhasm:     r7 += Sigma0(r0) + Maj(r0,r1,r2)
// asm 1: <r7=int64#5 += Sigma0(<r0=int64#2) + Maj(<r0=int64#2,<r1=int64#3,<r2=int64#4);
// asm 2: <r7=u4 += Sigma0(<r0=u1) + Maj(<r0=u1,<r1=u2,<r2=u3);
u4 += Sigma0(u1) + Maj(u1,u2,u3);
// qhasm:       r7_spill = r7
// asm 1: >r7_spill=spill64#8 = <r7=int64#5;
// asm 2: >r7_spill=d7 = <r7=u4;
d7 = u4;
// qhasm:       r4 = r4_spill
// asm 1: >r4=int64#2 = <r4_spill=spill64#5;
// asm 2: >r4=u1 = <r4_spill=d4;
u1 = d4;
// qhasm:       r5 = r5_spill
// asm 1: >r5=int64#3 = <r5_spill=spill64#6;
// asm 2: >r5=u2 = <r5_spill=d5;
u2 = d5;
// qhasm:       r6 = r6_spill
// asm 1: >r6=int64#5 = <r6_spill=spill64#7;
// asm 2: >r6=u4 = <r6_spill=d6;
u4 = d6;
// qhasm:       w1 = w1_spill
// asm 1: >w1=int64#6 = <w1_spill=spill64#10;
// asm 2: >w1=u5 = <w1_spill=d9;
u5 = d9;
// qhasm:     r6 += w1 + mem64[constants] + Sigma1(r3) + Ch(r3,r4,r5); constants += 8
// asm 1: <r6=int64#5 += <w1=int64#6 + load64(<constants=ptr#1) + Sigma1(<r3=int64#1) + Ch(<r3=int64#1,<r4=int64#2,<r5=int64#3);
// asm 2: <r6=u4 += <w1=u5 + load64(<constants=p0) + Sigma1(<r3=u0) + Ch(<r3=u0,<r4=u1,<r5=u2);
u4 += u5 + load64(p0) + Sigma1(u0) + Ch(u0,u1,u2);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r2 += r6
// asm 1: <r2=int64#4 += <r6=int64#5;
// asm 2: <r2=u3 += <r6=u4;
u3 += u4;
// qhasm:       r2_spill = r2
// asm 1: >r2_spill=spill64#3 = <r2=int64#4;
// asm 2: >r2_spill=d2 = <r2=u3;
d2 = u3;
// qhasm:       r7 = r7_spill
// asm 1: >r7=int64#1 = <r7_spill=spill64#8;
// asm 2: >r7=u0 = <r7_spill=d7;
u0 = d7;
// qhasm:       r0 = r0_spill
// asm 1: >r0=int64#2 = <r0_spill=spill64#1;
// asm 2: >r0=u1 = <r0_spill=d0;
u1 = d0;
// qhasm:       r1 = r1_spill
// asm 1: >r1=int64#3 = <r1_spill=spill64#2;
// asm 2: >r1=u2 = <r1_spill=d1;
u2 = d1;
// qhasm:     r6 += Sigma0(r7) + Maj(r7,r0,r1)
// asm 1: <r6=int64#5 += Sigma0(<r7=int64#1) + Maj(<r7=int64#1,<r0=int64#2,<r1=int64#3);
// asm 2: <r6=u4 += Sigma0(<r7=u0) + Maj(<r7=u0,<r0=u1,<r1=u2);
u4 += Sigma0(u0) + Maj(u0,u1,u2);
// qhasm:       r6_spill = r6
// asm 1: >r6_spill=spill64#7 = <r6=int64#5;
// asm 2: >r6_spill=d6 = <r6=u4;
d6 = u4;
// qhasm:     assign 0 to r0_spill
// qhasm:     assign 1 to r1_spill
// qhasm:     assign 2 to r2_spill
// qhasm:     assign 3 to r3_spill
// qhasm:     assign 4 to r4_spill
// qhasm:     assign 5 to r5_spill
// qhasm:     assign 6 to r6_spill
// qhasm:     assign 7 to r7_spill
// qhasm:       r3 = r3_spill
// asm 1: >r3=int64#1 = <r3_spill=spill64#4;
// asm 2: >r3=u0 = <r3_spill=d3;
u0 = d3;
// qhasm:       r4 = r4_spill
// asm 1: >r4=int64#2 = <r4_spill=spill64#5;
// asm 2: >r4=u1 = <r4_spill=d4;
u1 = d4;
// qhasm:       r5 = r5_spill
// asm 1: >r5=int64#5 = <r5_spill=spill64#6;
// asm 2: >r5=u4 = <r5_spill=d5;
u4 = d5;
// qhasm:       w2 = w2_spill
// asm 1: >w2=int64#6 = <w2_spill=spill64#11;
// asm 2: >w2=u5 = <w2_spill=d10;
u5 = d10;
// qhasm:     r5 += w2 + mem64[constants] + Sigma1(r2) + Ch(r2,r3,r4); constants += 8
// asm 1: <r5=int64#5 += <w2=int64#6 + load64(<constants=ptr#1) + Sigma1(<r2=int64#4) + Ch(<r2=int64#4,<r3=int64#1,<r4=int64#2);
// asm 2: <r5=u4 += <w2=u5 + load64(<constants=p0) + Sigma1(<r2=u3) + Ch(<r2=u3,<r3=u0,<r4=u1);
u4 += u5 + load64(p0) + Sigma1(u3) + Ch(u3,u0,u1);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r1 += r5
// asm 1: <r1=int64#3 += <r5=int64#5;
// asm 2: <r1=u2 += <r5=u4;
u2 += u4;
// qhasm:       r1_spill = r1
// asm 1: >r1_spill=spill64#2 = <r1=int64#3;
// asm 2: >r1_spill=d1 = <r1=u2;
d1 = u2;
// qhasm:       r6 = r6_spill
// asm 1: >r6=int64#1 = <r6_spill=spill64#7;
// asm 2: >r6=u0 = <r6_spill=d6;
u0 = d6;
// qhasm:       r7 = r7_spill
// asm 1: >r7=int64#2 = <r7_spill=spill64#8;
// asm 2: >r7=u1 = <r7_spill=d7;
u1 = d7;
// qhasm:       r0 = r0_spill
// asm 1: >r0=int64#4 = <r0_spill=spill64#1;
// asm 2: >r0=u3 = <r0_spill=d0;
u3 = d0;
// qhasm:     r5 += Sigma0(r6) + Maj(r6,r7,r0)
// asm 1: <r5=int64#5 += Sigma0(<r6=int64#1) + Maj(<r6=int64#1,<r7=int64#2,<r0=int64#4);
// asm 2: <r5=u4 += Sigma0(<r6=u0) + Maj(<r6=u0,<r7=u1,<r0=u3);
u4 += Sigma0(u0) + Maj(u0,u1,u3);
// qhasm:       r5_spill = r5
// asm 1: >r5_spill=spill64#6 = <r5=int64#5;
// asm 2: >r5_spill=d5 = <r5=u4;
d5 = u4;
// qhasm:       r2 = r2_spill
// asm 1: >r2=int64#1 = <r2_spill=spill64#3;
// asm 2: >r2=u0 = <r2_spill=d2;
u0 = d2;
// qhasm:       r3 = r3_spill
// asm 1: >r3=int64#2 = <r3_spill=spill64#4;
// asm 2: >r3=u1 = <r3_spill=d3;
u1 = d3;
// qhasm:       r4 = r4_spill
// asm 1: >r4=int64#5 = <r4_spill=spill64#5;
// asm 2: >r4=u4 = <r4_spill=d4;
u4 = d4;
// qhasm:       w3 = w3_spill
// asm 1: >w3=int64#6 = <w3_spill=spill64#12;
// asm 2: >w3=u5 = <w3_spill=d11;
u5 = d11;
// qhasm:     r4 += w3 + mem64[constants] + Sigma1(r1) + Ch(r1,r2,r3); constants += 8
// asm 1: <r4=int64#5 += <w3=int64#6 + load64(<constants=ptr#1) + Sigma1(<r1=int64#3) + Ch(<r1=int64#3,<r2=int64#1,<r3=int64#2);
// asm 2: <r4=u4 += <w3=u5 + load64(<constants=p0) + Sigma1(<r1=u2) + Ch(<r1=u2,<r2=u0,<r3=u1);
u4 += u5 + load64(p0) + Sigma1(u2) + Ch(u2,u0,u1);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r0 += r4
// asm 1: <r0=int64#4 += <r4=int64#5;
// asm 2: <r0=u3 += <r4=u4;
u3 += u4;
// qhasm:       r0_spill = r0
// asm 1: >r0_spill=spill64#1 = <r0=int64#4;
// asm 2: >r0_spill=d0 = <r0=u3;
d0 = u3;
// qhasm:       r5 = r5_spill
// asm 1: >r5=int64#1 = <r5_spill=spill64#6;
// asm 2: >r5=u0 = <r5_spill=d5;
u0 = d5;
// qhasm:       r6 = r6_spill
// asm 1: >r6=int64#2 = <r6_spill=spill64#7;
// asm 2: >r6=u1 = <r6_spill=d6;
u1 = d6;
// qhasm:       r7 = r7_spill
// asm 1: >r7=int64#3 = <r7_spill=spill64#8;
// asm 2: >r7=u2 = <r7_spill=d7;
u2 = d7;
// qhasm:     r4 += Sigma0(r5) + Maj(r5,r6,r7)
// asm 1: <r4=int64#5 += Sigma0(<r5=int64#1) + Maj(<r5=int64#1,<r6=int64#2,<r7=int64#3);
// asm 2: <r4=u4 += Sigma0(<r5=u0) + Maj(<r5=u0,<r6=u1,<r7=u2);
u4 += Sigma0(u0) + Maj(u0,u1,u2);
// qhasm:       r4_spill = r4
// asm 1: >r4_spill=spill64#5 = <r4=int64#5;
// asm 2: >r4_spill=d4 = <r4=u4;
d4 = u4;
// qhasm:     assign 0 to r0_spill
// qhasm:     assign 1 to r1_spill
// qhasm:     assign 2 to r2_spill
// qhasm:     assign 3 to r3_spill
// qhasm:     assign 4 to r4_spill
// qhasm:     assign 5 to r5_spill
// qhasm:     assign 6 to r6_spill
// qhasm:     assign 7 to r7_spill
// qhasm:       r1 = r1_spill
// asm 1: >r1=int64#1 = <r1_spill=spill64#2;
// asm 2: >r1=u0 = <r1_spill=d1;
u0 = d1;
// qhasm:       r2 = r2_spill
// asm 1: >r2=int64#2 = <r2_spill=spill64#3;
// asm 2: >r2=u1 = <r2_spill=d2;
u1 = d2;
// qhasm:       r3 = r3_spill
// asm 1: >r3=int64#5 = <r3_spill=spill64#4;
// asm 2: >r3=u4 = <r3_spill=d3;
u4 = d3;
// qhasm:       w4 = w4_spill
// asm 1: >w4=int64#6 = <w4_spill=spill64#13;
// asm 2: >w4=u5 = <w4_spill=d12;
u5 = d12;
// qhasm:     r3 += w4 + mem64[constants] + Sigma1(r0) + Ch(r0,r1,r2); constants += 8
// asm 1: <r3=int64#5 += <w4=int64#6 + load64(<constants=ptr#1) + Sigma1(<r0=int64#4) + Ch(<r0=int64#4,<r1=int64#1,<r2=int64#2);
// asm 2: <r3=u4 += <w4=u5 + load64(<constants=p0) + Sigma1(<r0=u3) + Ch(<r0=u3,<r1=u0,<r2=u1);
u4 += u5 + load64(p0) + Sigma1(u3) + Ch(u3,u0,u1);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r7 += r3
// asm 1: <r7=int64#3 += <r3=int64#5;
// asm 2: <r7=u2 += <r3=u4;
u2 += u4;
// qhasm:       r7_spill = r7
// asm 1: >r7_spill=spill64#8 = <r7=int64#3;
// asm 2: >r7_spill=d7 = <r7=u2;
d7 = u2;
// qhasm:       r4 = r4_spill
// asm 1: >r4=int64#1 = <r4_spill=spill64#5;
// asm 2: >r4=u0 = <r4_spill=d4;
u0 = d4;
// qhasm:       r5 = r5_spill
// asm 1: >r5=int64#2 = <r5_spill=spill64#6;
// asm 2: >r5=u1 = <r5_spill=d5;
u1 = d5;
// qhasm:       r6 = r6_spill
// asm 1: >r6=int64#4 = <r6_spill=spill64#7;
// asm 2: >r6=u3 = <r6_spill=d6;
u3 = d6;
// qhasm:     r3 += Sigma0(r4) + Maj(r4,r5,r6)
// asm 1: <r3=int64#5 += Sigma0(<r4=int64#1) + Maj(<r4=int64#1,<r5=int64#2,<r6=int64#4);
// asm 2: <r3=u4 += Sigma0(<r4=u0) + Maj(<r4=u0,<r5=u1,<r6=u3);
u4 += Sigma0(u0) + Maj(u0,u1,u3);
// qhasm:       r3_spill = r3
// asm 1: >r3_spill=spill64#4 = <r3=int64#5;
// asm 2: >r3_spill=d3 = <r3=u4;
d3 = u4;
// qhasm:       r0 = r0_spill
// asm 1: >r0=int64#1 = <r0_spill=spill64#1;
// asm 2: >r0=u0 = <r0_spill=d0;
u0 = d0;
// qhasm:       r1 = r1_spill
// asm 1: >r1=int64#2 = <r1_spill=spill64#2;
// asm 2: >r1=u1 = <r1_spill=d1;
u1 = d1;
// qhasm:       r2 = r2_spill
// asm 1: >r2=int64#5 = <r2_spill=spill64#3;
// asm 2: >r2=u4 = <r2_spill=d2;
u4 = d2;
// qhasm:       w5 = w5_spill
// asm 1: >w5=int64#6 = <w5_spill=spill64#14;
// asm 2: >w5=u5 = <w5_spill=d13;
u5 = d13;
// qhasm:     r2 += w5 + mem64[constants] + Sigma1(r7) + Ch(r7,r0,r1); constants += 8
// asm 1: <r2=int64#5 += <w5=int64#6 + load64(<constants=ptr#1) + Sigma1(<r7=int64#3) + Ch(<r7=int64#3,<r0=int64#1,<r1=int64#2);
// asm 2: <r2=u4 += <w5=u5 + load64(<constants=p0) + Sigma1(<r7=u2) + Ch(<r7=u2,<r0=u0,<r1=u1);
u4 += u5 + load64(p0) + Sigma1(u2) + Ch(u2,u0,u1);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r6 += r2
// asm 1: <r6=int64#4 += <r2=int64#5;
// asm 2: <r6=u3 += <r2=u4;
u3 += u4;
// qhasm:       r6_spill = r6
// asm 1: >r6_spill=spill64#7 = <r6=int64#4;
// asm 2: >r6_spill=d6 = <r6=u3;
d6 = u3;
// qhasm:       r3 = r3_spill
// asm 1: >r3=int64#1 = <r3_spill=spill64#4;
// asm 2: >r3=u0 = <r3_spill=d3;
u0 = d3;
// qhasm:       r4 = r4_spill
// asm 1: >r4=int64#2 = <r4_spill=spill64#5;
// asm 2: >r4=u1 = <r4_spill=d4;
u1 = d4;
// qhasm:       r5 = r5_spill
// asm 1: >r5=int64#3 = <r5_spill=spill64#6;
// asm 2: >r5=u2 = <r5_spill=d5;
u2 = d5;
// qhasm:     r2 += Sigma0(r3) + Maj(r3,r4,r5)
// asm 1: <r2=int64#5 += Sigma0(<r3=int64#1) + Maj(<r3=int64#1,<r4=int64#2,<r5=int64#3);
// asm 2: <r2=u4 += Sigma0(<r3=u0) + Maj(<r3=u0,<r4=u1,<r5=u2);
u4 += Sigma0(u0) + Maj(u0,u1,u2);
// qhasm:       r2_spill = r2
// asm 1: >r2_spill=spill64#3 = <r2=int64#5;
// asm 2: >r2_spill=d2 = <r2=u4;
d2 = u4;
// qhasm:     assign 0 to r0_spill
// qhasm:     assign 1 to r1_spill
// qhasm:     assign 2 to r2_spill
// qhasm:     assign 3 to r3_spill
// qhasm:     assign 4 to r4_spill
// qhasm:     assign 5 to r5_spill
// qhasm:     assign 6 to r6_spill
// qhasm:     assign 7 to r7_spill
// qhasm:       r7 = r7_spill
// asm 1: >r7=int64#1 = <r7_spill=spill64#8;
// asm 2: >r7=u0 = <r7_spill=d7;
u0 = d7;
// qhasm:       r0 = r0_spill
// asm 1: >r0=int64#2 = <r0_spill=spill64#1;
// asm 2: >r0=u1 = <r0_spill=d0;
u1 = d0;
// qhasm:       r1 = r1_spill
// asm 1: >r1=int64#5 = <r1_spill=spill64#2;
// asm 2: >r1=u4 = <r1_spill=d1;
u4 = d1;
// qhasm:       w6 = w6_spill
// asm 1: >w6=int64#6 = <w6_spill=spill64#15;
// asm 2: >w6=u5 = <w6_spill=d14;
u5 = d14;
// qhasm:     r1 += w6 + mem64[constants] + Sigma1(r6) + Ch(r6,r7,r0); constants += 8
// asm 1: <r1=int64#5 += <w6=int64#6 + load64(<constants=ptr#1) + Sigma1(<r6=int64#4) + Ch(<r6=int64#4,<r7=int64#1,<r0=int64#2);
// asm 2: <r1=u4 += <w6=u5 + load64(<constants=p0) + Sigma1(<r6=u3) + Ch(<r6=u3,<r7=u0,<r0=u1);
u4 += u5 + load64(p0) + Sigma1(u3) + Ch(u3,u0,u1);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r5 += r1
// asm 1: <r5=int64#3 += <r1=int64#5;
// asm 2: <r5=u2 += <r1=u4;
u2 += u4;
// qhasm:       r5_spill = r5
// asm 1: >r5_spill=spill64#6 = <r5=int64#3;
// asm 2: >r5_spill=d5 = <r5=u2;
d5 = u2;
// qhasm:       r2 = r2_spill
// asm 1: >r2=int64#1 = <r2_spill=spill64#3;
// asm 2: >r2=u0 = <r2_spill=d2;
u0 = d2;
// qhasm:       r3 = r3_spill
// asm 1: >r3=int64#2 = <r3_spill=spill64#4;
// asm 2: >r3=u1 = <r3_spill=d3;
u1 = d3;
// qhasm:       r4 = r4_spill
// asm 1: >r4=int64#4 = <r4_spill=spill64#5;
// asm 2: >r4=u3 = <r4_spill=d4;
u3 = d4;
// qhasm:     r1 += Sigma0(r2) + Maj(r2,r3,r4)
// asm 1: <r1=int64#5 += Sigma0(<r2=int64#1) + Maj(<r2=int64#1,<r3=int64#2,<r4=int64#4);
// asm 2: <r1=u4 += Sigma0(<r2=u0) + Maj(<r2=u0,<r3=u1,<r4=u3);
u4 += Sigma0(u0) + Maj(u0,u1,u3);
// qhasm:       r1_spill = r1
// asm 1: >r1_spill=spill64#2 = <r1=int64#5;
// asm 2: >r1_spill=d1 = <r1=u4;
d1 = u4;
// qhasm:       r6 = r6_spill
// asm 1: >r6=int64#1 = <r6_spill=spill64#7;
// asm 2: >r6=u0 = <r6_spill=d6;
u0 = d6;
// qhasm:       r7 = r7_spill
// asm 1: >r7=int64#2 = <r7_spill=spill64#8;
// asm 2: >r7=u1 = <r7_spill=d7;
u1 = d7;
// qhasm:       r0 = r0_spill
// asm 1: >r0=int64#5 = <r0_spill=spill64#1;
// asm 2: >r0=u4 = <r0_spill=d0;
u4 = d0;
// qhasm:       w7 = w7_spill
// asm 1: >w7=int64#6 = <w7_spill=spill64#16;
// asm 2: >w7=u5 = <w7_spill=d15;
u5 = d15;
// qhasm:     r0 += w7 + mem64[constants] + Sigma1(r5) + Ch(r5,r6,r7); constants += 8
// asm 1: <r0=int64#5 += <w7=int64#6 + load64(<constants=ptr#1) + Sigma1(<r5=int64#3) + Ch(<r5=int64#3,<r6=int64#1,<r7=int64#2);
// asm 2: <r0=u4 += <w7=u5 + load64(<constants=p0) + Sigma1(<r5=u2) + Ch(<r5=u2,<r6=u0,<r7=u1);
u4 += u5 + load64(p0) + Sigma1(u2) + Ch(u2,u0,u1);
// asm 1: <constants=ptr#1 += 8;
// asm 2: <constants=p0 += 8;
p0 += 8;
// qhasm:     r4 += r0
// asm 1: <r4=int64#4 += <r0=int64#5;
// asm 2: <r4=u3 += <r0=u4;
u3 += u4;
// qhasm:       r4_spill = r4
// asm 1: >r4_spill=spill64#5 = <r4=int64#4;
// asm 2: >r4_spill=d4 = <r4=u3;
d4 = u3;
// qhasm:       r1 = r1_spill
// asm 1: >r1=int64#1 = <r1_spill=spill64#2;
// asm 2: >r1=u0 = <r1_spill=d1;
u0 = d1;
// qhasm:       r2 = r2_spill
// asm 1: >r2=int64#2 = <r2_spill=spill64#3;
// asm 2: >r2=u1 = <r2_spill=d2;
u1 = d2;
// qhasm:       r3 = r3_spill
// asm 1: >r3=int64#3 = <r3_spill=spill64#4;
// asm 2: >r3=u2 = <r3_spill=d3;
u2 = d3;
// qhasm:     r0 += Sigma0(r1) + Maj(r1,r2,r3)
// asm 1: <r0=int64#5 += Sigma0(<r1=int64#1) + Maj(<r1=int64#1,<r2=int64#2,<r3=int64#3);
// asm 2: <r0=u4 += Sigma0(<r1=u0) + Maj(<r1=u0,<r2=u1,<r3=u2);
u4 += Sigma0(u0) + Maj(u0,u1,u2);
// qhasm:       r0_spill = r0
// asm 1: >r0_spill=spill64#1 = <r0=int64#5;
// asm 2: >r0_spill=d0 = <r0=u4;
d0 = u4;
// qhasm:     assign 8 to w0_spill
// qhasm:     assign 9 to w1_spill
// qhasm:     assign 10 to w2_spill
// qhasm:     assign 11 to w3_spill
// qhasm:     assign 12 to w4_spill
// qhasm:     assign 13 to w5_spill
// qhasm:     assign 14 to w6_spill
// qhasm:     assign 15 to w7_spill
// qhasm:     i = i_stack
// asm 1: >i=int32#1 = <i_stack=stack32#2;
// asm 2: >i=i0 = <i_stack=o1;
i0 = o1;
// qhasm:                          =? i -= 8
// asm 1: equal = eq(<i=int32#1,8); unsignedge = ge(<i=int32#1,8); <i=int32#1 -= 8;
// asm 2: equal = eq(<i=i0,8); unsignedge = ge(<i=i0,8); <i=i0 -= 8;
equal = eq(i0,8); unsignedge = ge(i0,8); i0 -= 8;
// qhasm:     goto endinnerloop if =
if (equal) goto label_endinnerloop;
// qhasm:     i_stack = i
// asm 1: >i_stack=stack32#2 = <i=int32#1;
// asm 2: >i_stack=o1 = <i=i0;
o1 = i0;
// qhasm:                     =? i - 8
// asm 1: equal = eq(<i=int32#1,8);
// asm 2: equal = eq(<i=i0,8);
equal = eq(i0,8);
// qhasm:     goto nearend if =
if (equal) goto label_nearend;
// qhasm:       w8 = w0_spill
// asm 1: >w8=int64#1 = <w0_spill=spill64#9;
// asm 2: >w8=u0 = <w0_spill=d8;
u0 = d8;
// qhasm:       w9 = w1_spill
// asm 1: >w9=int64#2 = <w1_spill=spill64#10;
// asm 2: >w9=u1 = <w1_spill=d9;
u1 = d9;
// qhasm:       w6 = w6_next
// asm 1: >w6=int64#3 = <w6_next=stack64#15;
// asm 2: >w6=u2 = <w6_next=m14;
u2 = m14;
// qhasm:       w1 = w1_next
// asm 1: >w1=int64#4 = <w1_next=stack64#10;
// asm 2: >w1=u3 = <w1_next=m9;
u3 = m9;
// qhasm:       w8  += sigma1(w6)
// asm 1: <w8=int64#1 += sigma1(<w6=int64#3);
// asm 2: <w8=u0 += sigma1(<w6=u2);
u0 += sigma1(u2);
// qhasm:       w8  += sigma0(w9)
// asm 1: <w8=int64#1 += sigma0(<w9=int64#2);
// asm 2: <w8=u0 += sigma0(<w9=u1);
u0 += sigma0(u1);
// qhasm:       w8  += w1
// asm 1: <w8=int64#1 += <w1=int64#4;
// asm 2: <w8=u0 += <w1=u3;
u0 += u3;
// qhasm:         w1_spill = w1
// asm 1: >w1_spill=spill64#10 = <w1=int64#4;
// asm 2: >w1_spill=d9 = <w1=u3;
d9 = u3;
// qhasm:       w7 = w7_next
// asm 1: >w7=int64#4 = <w7_next=stack64#16;
// asm 2: >w7=u3 = <w7_next=m15;
u3 = m15;
// qhasm:         w8_stack = w8
// asm 1: >w8_stack=stack64#16 = <w8=int64#1;
// asm 2: >w8_stack=m15 = <w8=u0;
m15 = u0;
// qhasm:       w9  += sigma1(w7)
// asm 1: <w9=int64#2 += sigma1(<w7=int64#4);
// asm 2: <w9=u1 += sigma1(<w7=u3);
u1 += sigma1(u3);
// qhasm:       w10 = w2_spill
// asm 1: >w10=int64#5 = <w2_spill=spill64#11;
// asm 2: >w10=u4 = <w2_spill=d10;
u4 = d10;
// qhasm:       w9  += sigma0(w10)
// asm 1: <w9=int64#2 += sigma0(<w10=int64#5);
// asm 2: <w9=u1 += sigma0(<w10=u4);
u1 += sigma0(u4);
// qhasm:       w2 = w2_next
// asm 1: >w2=int64#6 = <w2_next=stack64#11;
// asm 2: >w2=u5 = <w2_next=m10;
u5 = m10;
// qhasm:       w9  += w2
// asm 1: <w9=int64#2 += <w2=int64#6;
// asm 2: <w9=u1 += <w2=u5;
u1 += u5;
// qhasm:         w2_spill = w2
// asm 1: >w2_spill=spill64#11 = <w2=int64#6;
// asm 2: >w2_spill=d10 = <w2=u5;
d10 = u5;
// qhasm:         w1_next = w9
// asm 1: >w1_next=stack64#10 = <w9=int64#2;
// asm 2: >w1_next=m9 = <w9=u1;
m9 = u1;
// qhasm:       w10 += sigma1(w8)
// asm 1: <w10=int64#5 += sigma1(<w8=int64#1);
// asm 2: <w10=u4 += sigma1(<w8=u0);
u4 += sigma1(u0);
// qhasm:       w11 = w3_spill
// asm 1: >w11=int64#1 = <w3_spill=spill64#12;
// asm 2: >w11=u0 = <w3_spill=d11;
u0 = d11;
// qhasm:       w10 += sigma0(w11)
// asm 1: <w10=int64#5 += sigma0(<w11=int64#1);
// asm 2: <w10=u4 += sigma0(<w11=u0);
u4 += sigma0(u0);
// qhasm:       w3 = w3_next
// asm 1: >w3=int64#6 = <w3_next=stack64#12;
// asm 2: >w3=u5 = <w3_next=m11;
u5 = m11;
// qhasm:       w10 += w3
// asm 1: <w10=int64#5 += <w3=int64#6;
// asm 2: <w10=u4 += <w3=u5;
u4 += u5;
// qhasm:         w3_spill = w3
// asm 1: >w3_spill=spill64#12 = <w3=int64#6;
// asm 2: >w3_spill=d11 = <w3=u5;
d11 = u5;
// qhasm:         w2_next = w10
// asm 1: >w2_next=stack64#11 = <w10=int64#5;
// asm 2: >w2_next=m10 = <w10=u4;
m10 = u4;
// qhasm:       w11 += sigma1(w9)
// asm 1: <w11=int64#1 += sigma1(<w9=int64#2);
// asm 2: <w11=u0 += sigma1(<w9=u1);
u0 += sigma1(u1);
// qhasm:       w12 = w4_spill
// asm 1: >w12=int64#2 = <w4_spill=spill64#13;
// asm 2: >w12=u1 = <w4_spill=d12;
u1 = d12;
// qhasm:       w11 += sigma0(w12)
// asm 1: <w11=int64#1 += sigma0(<w12=int64#2);
// asm 2: <w11=u0 += sigma0(<w12=u1);
u0 += sigma0(u1);
// qhasm:       w4 = w4_next
// asm 1: >w4=int64#6 = <w4_next=stack64#13;
// asm 2: >w4=u5 = <w4_next=m12;
u5 = m12;
// qhasm:       w11 += w4
// asm 1: <w11=int64#1 += <w4=int64#6;
// asm 2: <w11=u0 += <w4=u5;
u0 += u5;
// qhasm:         w4_spill = w4
// asm 1: >w4_spill=spill64#13 = <w4=int64#6;
// asm 2: >w4_spill=d12 = <w4=u5;
d12 = u5;
// qhasm:         w3_next = w11
// asm 1: >w3_next=stack64#12 = <w11=int64#1;
// asm 2: >w3_next=m11 = <w11=u0;
m11 = u0;
// qhasm:       w12 += sigma1(w10)
// asm 1: <w12=int64#2 += sigma1(<w10=int64#5);
// asm 2: <w12=u1 += sigma1(<w10=u4);
u1 += sigma1(u4);
// qhasm:       w13 = w5_spill
// asm 1: >w13=int64#5 = <w5_spill=spill64#14;
// asm 2: >w13=u4 = <w5_spill=d13;
u4 = d13;
// qhasm:       w12 += sigma0(w13)
// asm 1: <w12=int64#2 += sigma0(<w13=int64#5);
// asm 2: <w12=u1 += sigma0(<w13=u4);
u1 += sigma0(u4);
// qhasm:       w5 = w5_next
// asm 1: >w5=int64#6 = <w5_next=stack64#14;
// asm 2: >w5=u5 = <w5_next=m13;
u5 = m13;
// qhasm:       w12 += w5
// asm 1: <w12=int64#2 += <w5=int64#6;
// asm 2: <w12=u1 += <w5=u5;
u1 += u5;
// qhasm:         w5_spill = w5
// asm 1: >w5_spill=spill64#14 = <w5=int64#6;
// asm 2: >w5_spill=d13 = <w5=u5;
d13 = u5;
// qhasm:         w4_next = w12
// asm 1: >w4_next=stack64#13 = <w12=int64#2;
// asm 2: >w4_next=m12 = <w12=u1;
m12 = u1;
// qhasm:       w13 += sigma1(w11)
// asm 1: <w13=int64#5 += sigma1(<w11=int64#1);
// asm 2: <w13=u4 += sigma1(<w11=u0);
u4 += sigma1(u0);
// qhasm:       w14 = w6_spill
// asm 1: >w14=int64#1 = <w6_spill=spill64#15;
// asm 2: >w14=u0 = <w6_spill=d14;
u0 = d14;
// qhasm:         w6_spill = w6
// asm 1: >w6_spill=spill64#15 = <w6=int64#3;
// asm 2: >w6_spill=d14 = <w6=u2;
d14 = u2;
// qhasm:       w13 += sigma0(w14)
// asm 1: <w13=int64#5 += sigma0(<w14=int64#1);
// asm 2: <w13=u4 += sigma0(<w14=u0);
u4 += sigma0(u0);
// qhasm:       w13 += w6
// asm 1: <w13=int64#5 += <w6=int64#3;
// asm 2: <w13=u4 += <w6=u2;
u4 += u2;
// qhasm:         w5_next = w13
// asm 1: >w5_next=stack64#14 = <w13=int64#5;
// asm 2: >w5_next=m13 = <w13=u4;
m13 = u4;
// qhasm:       w14 += sigma1(w12)
// asm 1: <w14=int64#1 += sigma1(<w12=int64#2);
// asm 2: <w14=u0 += sigma1(<w12=u1);
u0 += sigma1(u1);
// qhasm:       w15 = w7_spill
// asm 1: >w15=int64#2 = <w7_spill=spill64#16;
// asm 2: >w15=u1 = <w7_spill=d15;
u1 = d15;
// qhasm:         w7_spill = w7
// asm 1: >w7_spill=spill64#16 = <w7=int64#4;
// asm 2: >w7_spill=d15 = <w7=u3;
d15 = u3;
// qhasm:       w14 += sigma0(w15)
// asm 1: <w14=int64#1 += sigma0(<w15=int64#2);
// asm 2: <w14=u0 += sigma0(<w15=u1);
u0 += sigma0(u1);
// qhasm:       w14 += w7
// asm 1: <w14=int64#1 += <w7=int64#4;
// asm 2: <w14=u0 += <w7=u3;
u0 += u3;
// qhasm:         w6_next = w14
// asm 1: >w6_next=stack64#15 = <w14=int64#1;
// asm 2: >w6_next=m14 = <w14=u0;
m14 = u0;
// qhasm:       w15 += sigma1(w13)
// asm 1: <w15=int64#2 += sigma1(<w13=int64#5);
// asm 2: <w15=u1 += sigma1(<w13=u4);
u1 += sigma1(u4);
// qhasm:       w0 = w0_next
// asm 1: >w0=int64#1 = <w0_next=stack64#9;
// asm 2: >w0=u0 = <w0_next=m8;
u0 = m8;
// qhasm:         w8 = w8_stack
// asm 1: >w8=int64#3 = <w8_stack=stack64#16;
// asm 2: >w8=u2 = <w8_stack=m15;
u2 = m15;
// qhasm:         w0_next = w8
// asm 1: >w0_next=stack64#9 = <w8=int64#3;
// asm 2: >w0_next=m8 = <w8=u2;
m8 = u2;
// qhasm:       w15 += sigma0(w0)
// asm 1: <w15=int64#2 += sigma0(<w0=int64#1);
// asm 2: <w15=u1 += sigma0(<w0=u0);
u1 += sigma0(u0);
// qhasm:       w15 += w8
// asm 1: <w15=int64#2 += <w8=int64#3;
// asm 2: <w15=u1 += <w8=u2;
u1 += u2;
// qhasm:         w7_next = w15
// asm 1: >w7_next=stack64#16 = <w15=int64#2;
// asm 2: >w7_next=m15 = <w15=u1;
m15 = u1;
// qhasm:         w0_spill = w0
// asm 1: >w0_spill=spill64#9 = <w0=int64#1;
// asm 2: >w0_spill=d8 = <w0=u0;
d8 = u0;
// qhasm:     goto innerloop
goto label_innerloop;
// qhasm:     nearend:
label_nearend:
// qhasm:       w0 = w0_next
// asm 1: >w0=int64#1 = <w0_next=stack64#9;
// asm 2: >w0=u0 = <w0_next=m8;
u0 = m8;
// qhasm:       w1 = w1_next
// asm 1: >w1=int64#2 = <w1_next=stack64#10;
// asm 2: >w1=u1 = <w1_next=m9;
u1 = m9;
// qhasm:       w2 = w2_next
// asm 1: >w2=int64#3 = <w2_next=stack64#11;
// asm 2: >w2=u2 = <w2_next=m10;
u2 = m10;
// qhasm:       w3 = w3_next
// asm 1: >w3=int64#4 = <w3_next=stack64#12;
// asm 2: >w3=u3 = <w3_next=m11;
u3 = m11;
// qhasm:       w0_spill = w0
// asm 1: >w0_spill=spill64#9 = <w0=int64#1;
// asm 2: >w0_spill=d8 = <w0=u0;
d8 = u0;
// qhasm:       w1_spill = w1
// asm 1: >w1_spill=spill64#10 = <w1=int64#2;
// asm 2: >w1_spill=d9 = <w1=u1;
d9 = u1;
// qhasm:       w2_spill = w2
// asm 1: >w2_spill=spill64#11 = <w2=int64#3;
// asm 2: >w2_spill=d10 = <w2=u2;
d10 = u2;
// qhasm:       w3_spill = w3
// asm 1: >w3_spill=spill64#12 = <w3=int64#4;
// asm 2: >w3_spill=d11 = <w3=u3;
d11 = u3;
// qhasm:       w4 = w4_next
// asm 1: >w4=int64#1 = <w4_next=stack64#13;
// asm 2: >w4=u0 = <w4_next=m12;
u0 = m12;
// qhasm:       w5 = w5_next
// asm 1: >w5=int64#2 = <w5_next=stack64#14;
// asm 2: >w5=u1 = <w5_next=m13;
u1 = m13;
// qhasm:       w6 = w6_next
// asm 1: >w6=int64#3 = <w6_next=stack64#15;
// asm 2: >w6=u2 = <w6_next=m14;
u2 = m14;
// qhasm:       w7 = w7_next
// asm 1: >w7=int64#4 = <w7_next=stack64#16;
// asm 2: >w7=u3 = <w7_next=m15;
u3 = m15;
// qhasm:       w4_spill = w4
// asm 1: >w4_spill=spill64#13 = <w4=int64#1;
// asm 2: >w4_spill=d12 = <w4=u0;
d12 = u0;
// qhasm:       w5_spill = w5
// asm 1: >w5_spill=spill64#14 = <w5=int64#2;
// asm 2: >w5_spill=d13 = <w5=u1;
d13 = u1;
// qhasm:       w6_spill = w6
// asm 1: >w6_spill=spill64#15 = <w6=int64#3;
// asm 2: >w6_spill=d14 = <w6=u2;
d14 = u2;
// qhasm:       w7_spill = w7
// asm 1: >w7_spill=spill64#16 = <w7=int64#4;
// asm 2: >w7_spill=d15 = <w7=u3;
d15 = u3;
// qhasm:     goto innerloop
goto label_innerloop;
// qhasm:   endinnerloop:
label_endinnerloop:
// qhasm:   constants -= 640
// asm 1: <constants=ptr#1 -= 640;
// asm 2: <constants=p0 -= 640;
p0 -= 640;
// qhasm:   r0 = r0_spill
// asm 1: >r0=int64#1 = <r0_spill=spill64#1;
// asm 2: >r0=u0 = <r0_spill=d0;
u0 = d0;
// qhasm:   r1 = r1_spill
// asm 1: >r1=int64#2 = <r1_spill=spill64#2;
// asm 2: >r1=u1 = <r1_spill=d1;
u1 = d1;
// qhasm:   r2 = r2_spill
// asm 1: >r2=int64#3 = <r2_spill=spill64#3;
// asm 2: >r2=u2 = <r2_spill=d2;
u2 = d2;
// qhasm:   r3 = r3_spill
// asm 1: >r3=int64#4 = <r3_spill=spill64#4;
// asm 2: >r3=u3 = <r3_spill=d3;
u3 = d3;
// qhasm:   r0 += state0
// asm 1: <r0=int64#1 += <state0=stack64#1;
// asm 2: <r0=u0 += <state0=m0;
u0 += m0;
// qhasm:   r1 += state1
// asm 1: <r1=int64#2 += <state1=stack64#2;
// asm 2: <r1=u1 += <state1=m1;
u1 += m1;
// qhasm:   r2 += state2
// asm 1: <r2=int64#3 += <state2=stack64#3;
// asm 2: <r2=u2 += <state2=m2;
u2 += m2;
// qhasm:   r3 += state3
// asm 1: <r3=int64#4 += <state3=stack64#4;
// asm 2: <r3=u3 += <state3=m3;
u3 += m3;
// qhasm:   state0 = r0
// asm 1: >state0=stack64#1 = <r0=int64#1;
// asm 2: >state0=m0 = <r0=u0;
m0 = u0;
// qhasm:   state1 = r1
// asm 1: >state1=stack64#2 = <r1=int64#2;
// asm 2: >state1=m1 = <r1=u1;
m1 = u1;
// qhasm:   state2 = r2
// asm 1: >state2=stack64#3 = <r2=int64#3;
// asm 2: >state2=m2 = <r2=u2;
m2 = u2;
// qhasm:   state3 = r3
// asm 1: >state3=stack64#4 = <r3=int64#4;
// asm 2: >state3=m3 = <r3=u3;
m3 = u3;
// qhasm:   r0_spill = r0
// asm 1: >r0_spill=spill64#1 = <r0=int64#1;
// asm 2: >r0_spill=d0 = <r0=u0;
d0 = u0;
// qhasm:   r1_spill = r1
// asm 1: >r1_spill=spill64#2 = <r1=int64#2;
// asm 2: >r1_spill=d1 = <r1=u1;
d1 = u1;
// qhasm:   r2_spill = r2
// asm 1: >r2_spill=spill64#3 = <r2=int64#3;
// asm 2: >r2_spill=d2 = <r2=u2;
d2 = u2;
// qhasm:   r3_spill = r3
// asm 1: >r3_spill=spill64#4 = <r3=int64#4;
// asm 2: >r3_spill=d3 = <r3=u3;
d3 = u3;
// qhasm:   r4 = r4_spill
// asm 1: >r4=int64#1 = <r4_spill=spill64#5;
// asm 2: >r4=u0 = <r4_spill=d4;
u0 = d4;
// qhasm:   r5 = r5_spill
// asm 1: >r5=int64#2 = <r5_spill=spill64#6;
// asm 2: >r5=u1 = <r5_spill=d5;
u1 = d5;
// qhasm:   r6 = r6_spill
// asm 1: >r6=int64#3 = <r6_spill=spill64#7;
// asm 2: >r6=u2 = <r6_spill=d6;
u2 = d6;
// qhasm:   r7 = r7_spill
// asm 1: >r7=int64#4 = <r7_spill=spill64#8;
// asm 2: >r7=u3 = <r7_spill=d7;
u3 = d7;
// qhasm:   r4 += state4
// asm 1: <r4=int64#1 += <state4=stack64#5;
// asm 2: <r4=u0 += <state4=m4;
u0 += m4;
// qhasm:   r5 += state5
// asm 1: <r5=int64#2 += <state5=stack64#6;
// asm 2: <r5=u1 += <state5=m5;
u1 += m5;
// qhasm:   r6 += state6
// asm 1: <r6=int64#3 += <state6=stack64#7;
// asm 2: <r6=u2 += <state6=m6;
u2 += m6;
// qhasm:   r7 += state7
// asm 1: <r7=int64#4 += <state7=stack64#8;
// asm 2: <r7=u3 += <state7=m7;
u3 += m7;
// qhasm:   state4 = r4
// asm 1: >state4=stack64#5 = <r4=int64#1;
// asm 2: >state4=m4 = <r4=u0;
m4 = u0;
// qhasm:   state5 = r5
// asm 1: >state5=stack64#6 = <r5=int64#2;
// asm 2: >state5=m5 = <r5=u1;
m5 = u1;
// qhasm:   state6 = r6
// asm 1: >state6=stack64#7 = <r6=int64#3;
// asm 2: >state6=m6 = <r6=u2;
m6 = u2;
// qhasm:   state7 = r7
// asm 1: >state7=stack64#8 = <r7=int64#4;
// asm 2: >state7=m7 = <r7=u3;
m7 = u3;
// qhasm:   r4_spill = r4
// asm 1: >r4_spill=spill64#5 = <r4=int64#1;
// asm 2: >r4_spill=d4 = <r4=u0;
d4 = u0;
// qhasm:   r5_spill = r5
// asm 1: >r5_spill=spill64#6 = <r5=int64#2;
// asm 2: >r5_spill=d5 = <r5=u1;
d5 = u1;
// qhasm:   r6_spill = r6
// asm 1: >r6_spill=spill64#7 = <r6=int64#3;
// asm 2: >r6_spill=d6 = <r6=u2;
d6 = u2;
// qhasm:   r7_spill = r7
// asm 1: >r7_spill=spill64#8 = <r7=int64#4;
// asm 2: >r7_spill=d7 = <r7=u3;
d7 = u3;
// qhasm:   inlen = inlen_stack
// asm 1: >inlen=int32#1 = <inlen_stack=stack32#1;
// asm 2: >inlen=i0 = <inlen_stack=o0;
i0 = o0;
// qhasm:                    unsigned>=? inlen -= 128
// asm 1: equal = eq(<inlen=int32#1,128); unsignedge = ge(<inlen=int32#1,128); <inlen=int32#1 -= 128;
// asm 2: equal = eq(<inlen=i0,128); unsignedge = ge(<inlen=i0,128); <inlen=i0 -= 128;
equal = eq(i0,128); unsignedge = ge(i0,128); i0 -= 128;
// qhasm:   inlen_stack = inlen
// asm 1: >inlen_stack=stack32#1 = <inlen=int32#1;
// asm 2: >inlen_stack=o0 = <inlen=i0;
o0 = i0;
// qhasm:   goto mainloop if unsigned>=
if (unsignedge) goto label_mainloop;
// qhasm: endmainloop:
label_endmainloop:
// qhasm: statebytes = statebytes_stack
// asm 1: >statebytes=ptr#1 = <statebytes_stack=stackptr#1;
// asm 2: >statebytes=p0 = <statebytes_stack=n0;
p0 = n0;
// qhasm: r0 = state0
// asm 1: >r0=int64#1 = <state0=stack64#1;
// asm 2: >r0=u0 = <state0=m0;
u0 = m0;
// qhasm: r1 = state1
// asm 1: >r1=int64#2 = <state1=stack64#2;
// asm 2: >r1=u1 = <state1=m1;
u1 = m1;
// qhasm: r2 = state2
// asm 1: >r2=int64#3 = <state2=stack64#3;
// asm 2: >r2=u2 = <state2=m2;
u2 = m2;
// qhasm: r3 = state3
// asm 1: >r3=int64#4 = <state3=stack64#4;
// asm 2: >r3=u3 = <state3=m3;
u3 = m3;
// qhasm: r0 = reverse flip r0
// asm 1: >r0=int64#1 = reverse(flip(<r0=int64#1));
// asm 2: >r0=u0 = reverse(flip(<r0=u0));
u0 = reverse(flip(u0));
// qhasm: r1 = reverse flip r1
// asm 1: >r1=int64#2 = reverse(flip(<r1=int64#2));
// asm 2: >r1=u1 = reverse(flip(<r1=u1));
u1 = reverse(flip(u1));
// qhasm: r2 = reverse flip r2
// asm 1: >r2=int64#3 = reverse(flip(<r2=int64#3));
// asm 2: >r2=u2 = reverse(flip(<r2=u2));
u2 = reverse(flip(u2));
// qhasm: r3 = reverse flip r3
// asm 1: >r3=int64#4 = reverse(flip(<r3=int64#4));
// asm 2: >r3=u3 = reverse(flip(<r3=u3));
u3 = reverse(flip(u3));
// qhasm: mem64[statebytes] = flip r0
// asm 1: store64(<statebytes=ptr#1,flip(<r0=int64#1));
// asm 2: store64(<statebytes=p0,flip(<r0=u0));
store64(p0,flip(u0));
// qhasm: mem64[statebytes+8] = flip r1
// asm 1: store64(<statebytes=ptr#1+8,flip(<r1=int64#2));
// asm 2: store64(<statebytes=p0+8,flip(<r1=u1));
store64(p0+8,flip(u1));
// qhasm: mem64[statebytes+16] = flip r2
// asm 1: store64(<statebytes=ptr#1+16,flip(<r2=int64#3));
// asm 2: store64(<statebytes=p0+16,flip(<r2=u2));
store64(p0+16,flip(u2));
// qhasm: mem64[statebytes+24] = flip r3
// asm 1: store64(<statebytes=ptr#1+24,flip(<r3=int64#4));
// asm 2: store64(<statebytes=p0+24,flip(<r3=u3));
store64(p0+24,flip(u3));
// qhasm: r4 = state4
// asm 1: >r4=int64#1 = <state4=stack64#5;
// asm 2: >r4=u0 = <state4=m4;
u0 = m4;
// qhasm: r5 = state5
// asm 1: >r5=int64#2 = <state5=stack64#6;
// asm 2: >r5=u1 = <state5=m5;
u1 = m5;
// qhasm: r6 = state6
// asm 1: >r6=int64#3 = <state6=stack64#7;
// asm 2: >r6=u2 = <state6=m6;
u2 = m6;
// qhasm: r7 = state7
// asm 1: >r7=int64#4 = <state7=stack64#8;
// asm 2: >r7=u3 = <state7=m7;
u3 = m7;
// qhasm: r4 = reverse flip r4
// asm 1: >r4=int64#1 = reverse(flip(<r4=int64#1));
// asm 2: >r4=u0 = reverse(flip(<r4=u0));
u0 = reverse(flip(u0));
// qhasm: r5 = reverse flip r5
// asm 1: >r5=int64#2 = reverse(flip(<r5=int64#2));
// asm 2: >r5=u1 = reverse(flip(<r5=u1));
u1 = reverse(flip(u1));
// qhasm: r6 = reverse flip r6
// asm 1: >r6=int64#3 = reverse(flip(<r6=int64#3));
// asm 2: >r6=u2 = reverse(flip(<r6=u2));
u2 = reverse(flip(u2));
// qhasm: r7 = reverse flip r7
// asm 1: >r7=int64#4 = reverse(flip(<r7=int64#4));
// asm 2: >r7=u3 = reverse(flip(<r7=u3));
u3 = reverse(flip(u3));
// qhasm: mem64[statebytes+32] = flip r4
// asm 1: store64(<statebytes=ptr#1+32,flip(<r4=int64#1));
// asm 2: store64(<statebytes=p0+32,flip(<r4=u0));
store64(p0+32,flip(u0));
// qhasm: mem64[statebytes+40] = flip r5
// asm 1: store64(<statebytes=ptr#1+40,flip(<r5=int64#2));
// asm 2: store64(<statebytes=p0+40,flip(<r5=u1));
store64(p0+40,flip(u1));
// qhasm: mem64[statebytes+48] = flip r6
// asm 1: store64(<statebytes=ptr#1+48,flip(<r6=int64#3));
// asm 2: store64(<statebytes=p0+48,flip(<r6=u2));
store64(p0+48,flip(u2));
// qhasm: mem64[statebytes+56] = flip r7
// asm 1: store64(<statebytes=ptr#1+56,flip(<r7=int64#4));
// asm 2: store64(<statebytes=p0+56,flip(<r7=u3));
store64(p0+56,flip(u3));
// qhasm: inlen += 128
// asm 1: <inlen=int32#1 += 128;
// asm 2: <inlen=i0 += 128;
i0 += 128;
// qhasm: popreturn inlen
return i0;
}
