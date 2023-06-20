#ifndef base_h
#define base_h

#define base CRYPTO_SHARED_NAMESPACE(base)

#include "ge.h"

/* base[i][j] = (j+1)*256^i*B */
extern const ge_precomp base[32][8];

#endif
