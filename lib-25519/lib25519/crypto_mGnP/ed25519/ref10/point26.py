#!/usr/bin/env python3

q = 2**255 - 19
l = 2**252 + 27742317777372353535851937790883648493

def expmod(b,e,m):
  if e == 0: return 1
  t = expmod(b,e//2,m)**2 % m
  if e & 1: t = (t*b) % m
  return t

def inv(x):
  return expmod(x,q-2,q)

d = -121665 * inv(121666)
I = expmod(2,(q-1)//4,q)

def xrecover(y):
  xx = (y*y-1) * inv(d*y*y+1)
  x = expmod(xx,(q+3)//8,q)
  if (x*x - xx) % q != 0: x = (x*I) % q
  if x % 2 != 0: x = q-x
  return x

def isoncurve(P):
  x,y = P
  return (-x*x+y*y-1-d*x**2*y**2) % q == 0

By = 4 * inv(5)
Bx = xrecover(By)
B = Bx%q,By%q

assert isoncurve(B)

def edwards(P,Q):
  x1,y1 = P
  x2,y2 = Q
  x3 = (x1*y2+x2*y1) * inv(1+d*x1*x2*y1*y2)
  y3 = (y1*y2+x1*x2) * inv(1-d*x1*x2*y1*y2)
  return x3%q,y3%q

def scalarmult(P,e):
  assert e >= 0
  if e == 0: return 0,1
  Q = scalarmult(P,e//2)
  Q = edwards(Q,Q)
  if e & 1: Q = edwards(P,Q)
  return Q

assert scalarmult(B,l) == (0,1)

Cy = 26 # minimum integer >1 giving C on curve and order l
Cx = xrecover(Cy)
C = Cx%q,Cy%q
assert isoncurve(C)
assert scalarmult(C,l) == (0,1)

# sanity checks:
# C is not a small multiple of B
# namely (0,1) or (+-...,4/5) or (+-...,-985469/985549) or ...
# and B is not a small multiple of C
# namely (0,1) or (+-...,10) or (+-...,-596219233/596219333) or ...
# and similarly for other small relations: e.g., 2B = 3C
# could do more serious discrete-log check

def hassmallrelation(A,B):
  jAyset = set()
  jA = 0,1
  for j in range(1,1000):
    jA = edwards(jA,A)
    jAyset.add(jA[1])
  jB = 0,1
  for j in range(1,1000):
    jB = edwards(jB,B)
    if jB[1] in jAyset: return True
  return False

testy = 333922976373947162567639162515495974004 * inv(333933611346170188668120256727948534525)
testx = xrecover(testy)
test = testx%q,testy%q
assert hassmallrelation(test,B)
assert not hassmallrelation(C,B)

def radix255(x):
  x = x % q
  if x + x > q: x -= q
  x = [x,0,0,0,0,0,0,0,0,0]
  bits = [26,25,26,25,26,25,26,25,26,25]
  for i in range(9):
    carry = (x[i] + 2**(bits[i]-1)) // 2**bits[i]
    x[i] -= carry * 2**bits[i]
    x[i + 1] += carry
  result = ""
  for i in range(9):
    result = result+str(x[i])+","
  result = result+str(x[9])
  return result

print('static const fe point26_x = {')
print('//',C[0])
print(radix255(C[0]))
print('} ;')
print('static const fe point26_y = {')
print('//',C[1])
print(radix255(C[1]))
print('} ;')
