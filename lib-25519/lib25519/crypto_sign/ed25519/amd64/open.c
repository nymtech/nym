#include <string.h>
#include "crypto_sign.h"
#include "crypto_verify_32.h"
#include "crypto_hash_sha512.h"
#include "crypto_mGnP_ed25519.h"

int crypto_sign_open(
    unsigned char *m,long long *mlen,
    const unsigned char *sm,long long smlen,
    const unsigned char *pk
    )
{
  unsigned char Acopy[32];
  unsigned char Rcopy[32];
  unsigned char Scopy[32];
  unsigned char hram[64];
  unsigned char Rcheck[33];

  if (smlen < 64) goto badsig;
  if (sm[63] & 224) goto badsig;

  memmove(Acopy,pk,32);
  memmove(Rcopy,sm,32);
  memmove(Scopy,sm+32,32);

  memmove(m,sm,smlen);
  memmove(m+32,Acopy,32);
  crypto_hash_sha512(hram,m,smlen);
  crypto_mGnP_ed25519(Rcheck,Scopy,hram,Acopy);
  if (Rcheck[32] != 1) goto badsig;

  if (crypto_verify_32(Rcopy,Rcheck) == 0) {
    memmove(m,m+64,smlen-64);
    memset(m+smlen-64,0,64);
    *mlen = smlen-64;
    return 0;
  }

badsig:
  *mlen = (unsigned long long) -1;
  memset(m,0,smlen);
  return -1;
}
