// linker define hh1_p1
// linker define hh1_p2
// linker define hh1_p3
// linker define h2h_p1
// linker define h2h_p2
// linker define h2h_p3
// linker define vecmask29d
// linker define hh1_xor1
// linker define h2h_xor1
// linker define hh1_xor2
// linker define h2h_xor2
// linker define swap_c
// linker define swap_mask
// linker define h2h_mask
// linker define vec19
// linker define vec1216
// linker define vecmask23
// linker define vecmask29
// linker define vecmask32
// linker define a24
// linker define mask63

#include "consts_namespace.h"
#include "crypto_uint32.h"
#include "crypto_uint64.h"

const crypto_uint32 hh1_p1[]     = { 0x0,0x0,0x3FFFFFDB,0x3FFFFFFF,0x3FFFFFDB,0x3FFFFFFF,0x0,0x0 };
const crypto_uint32 hh1_p2[]     = { 0x0,0x0,0x3FFFFFFF,0x3FFFFFFF,0x3FFFFFFF,0x3FFFFFFF,0x0,0x0 };
const crypto_uint32 hh1_p3[]     = { 0x0,0x0,0xFFFFFF,0x0,0xFFFFFF,0x0,0x0,0x0 };
const crypto_uint32 h2h_p1[]     = { 0x0,0x0,0x3FFFFFDB,0x3FFFFFFF,0x0,0x0,0x3FFFFFDB,0x3FFFFFFF };
const crypto_uint32 h2h_p2[]     = { 0x0,0x0,0x3FFFFFFF,0x3FFFFFFF,0x0,0x0,0x3FFFFFFF,0x3FFFFFFF };
const crypto_uint32 h2h_p3[]     = { 0x0,0x0,0xFFFFFF,0x0,0x0,0x0,0xFFFFFF,0x0 };
const crypto_uint32 vecmask29d[] = { 0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF };
const crypto_uint32 hh1_xor1[]   = { 0,0,-1,-1,-1,-1,0,0 };
const crypto_uint32 h2h_xor1[]   = { 0,0,-1,-1,0,0,-1,-1 };
const crypto_uint32 hh1_xor2[]   = { 0,0,-1,0,-1,0,0,0 };
const crypto_uint32 h2h_xor2[]   = { 0,0,-1,0,0,0,-1,0 };
const crypto_uint32 swap_c[]     = { 0,1,2,3,4,5,6,7 };
const crypto_uint32 swap_mask[]  = { 7,7,7,7,7,7,7,7 };
const crypto_uint64 h2h_mask[]   = { 0,-1,-1,-1 };
const crypto_uint64 vec19[]      = { 19,19,19,19 };
const crypto_uint64 vec1216[]    = { 1216,1216,1216,1216 };
const crypto_uint64 vecmask23[]  = { 0x7FFFFF,0x7FFFFF,0x7FFFFF,0x7FFFFF };
const crypto_uint64 vecmask29[]  = { 0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF };
const crypto_uint64 vecmask32[]  = { 0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF };
const crypto_uint64 a24[]  	  = { 0,121666,0,0 };
const crypto_uint64 mask63[] 	  = { 0x7FFFFFFFFFFFFFFF };
