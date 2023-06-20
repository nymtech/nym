// linker define hh1_p1
// linker define hh1_p2
// linker define h2h_p1
// linker define h2h_p2
// linker define hh1_xor
// linker define h2h_xor
// linker define swap_c
// linker define swap_mask
// linker define h2h_mask
// linker define vec19
// linker define vec38
// linker define vecmask25
// linker define vecmask26
// linker define vecmask32
// linker define a24
// linker define mask63

#include "consts_namespace.h"
#include "crypto_uint32.h"
#include "crypto_uint64.h"

const crypto_uint32 hh1_p1[]     = { 0x0,0x0,0x7FFFFDB,0x3FFFFFF,0x7FFFFDB,0x3FFFFFF,0x0,0x0 };
const crypto_uint32 hh1_p2[]     = { 0x0,0x0,0x7FFFFFF,0x3FFFFFF,0x7FFFFFF,0x3FFFFFF,0x0,0x0 };
const crypto_uint32 h2h_p1[]     = { 0x0,0x0,0x7FFFFDB,0x3FFFFFF,0x0,0x0,0x7FFFFDB,0x3FFFFFF };
const crypto_uint32 h2h_p2[]     = { 0x0,0x0,0x7FFFFFF,0x3FFFFFF,0x0,0x0,0x7FFFFFF,0x3FFFFFF };
const crypto_uint32 hh1_xor[]    = { 0,0,-1,-1,-1,-1,0,0 };
const crypto_uint32 h2h_xor[]    = { 0,0,-1,-1,0,0,-1,-1 };
const crypto_uint32 swap_c[]     = { 0,1,2,3,4,5,6,7 };
const crypto_uint32 swap_mask[]  = { 7,7,7,7,7,7,7,7 };
const crypto_uint64 h2h_mask[]   = { 0,-1,-1,-1 };
const crypto_uint64 vec19[]      = { 19,19,19,19 };
const crypto_uint64 vec38[]      = { 38,38,38,38 };
const crypto_uint64 vecmask25[]  = { 0x1FFFFFF,0x1FFFFFF,0x1FFFFFF,0x1FFFFFF };
const crypto_uint64 vecmask26[]  = { 0x3FFFFFF,0x3FFFFFF,0x3FFFFFF,0x3FFFFFF };
const crypto_uint64 vecmask32[]  = { 0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF };
const crypto_uint64 a24[]  	  = { 0,121666,0,0 };
const crypto_uint64 mask63[] 	  = { 0x7FFFFFFFFFFFFFFF };
