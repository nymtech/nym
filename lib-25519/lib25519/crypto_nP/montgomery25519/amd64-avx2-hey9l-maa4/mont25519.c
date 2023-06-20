#include "randombytes.h"
#include "crypto_nP.h"
#include "fe25519.h"
#include "crypto_uint64_vec4x1.h"

#define mladder CRYPTO_SHARED_NAMESPACE(mladder)
extern void mladder(crypto_uint64_vec4x1 *,crypto_uint64_vec4x1 *,const unsigned char *);

void crypto_nP(unsigned char *r,
                      const unsigned char *s,
                      const unsigned char *p)
{
  unsigned char e[32];
  int i;
  
  crypto_uint64_vec4x1 a[10] = {{0}};
  crypto_uint64_vec4x1 b[10] = {{0}};
  fe25519_9l u,v;
  fe25519 z[2];  
  
  for(i=0;i<32;i++) e[i] = s[i];
  e[0] &= 248;
  e[31] &= 127;
  e[31] |= 64;
  
  fe25519_unpack(z,p);
  fe25519_to_9l(&u,z);  

  b[0][0] = b[0][3] = a[0][2] = 1; a[0][1] = 486662;
	
  for (i=0;i<9;i++) {b[i][2] = u.l[i]; a[i][3] = u.l[i];}

  mladder(b,a,e);

  for (i=0;i<9;i++) {u.l[i] = b[i][0]; v.l[i] = b[i][1];}

  fe25519_from_9l(z,&u);
  fe25519_from_9l(z+1,&v);
  fe25519_invert(z+1, z+1);
  fe25519_mul(z, z, z+1);  
  fe25519_pack(r, z);
}
