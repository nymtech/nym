#ifndef shared_data_h
#define shared_data_h

#include "ge25519.h"

// warning: these constants are not encapsulated

#define P_WINDOWSIZE 5
#define P_MULTIPLES (1<<(P_WINDOWSIZE-2))
#define G_WINDOWSIZE 7
#define G_MULTIPLES (1<<(G_WINDOWSIZE-2))
#define G_multiples CRYPTO_SHARED_NAMESPACE(G_multiples)

extern const ge25519_niels G_multiples[G_MULTIPLES];

#endif
