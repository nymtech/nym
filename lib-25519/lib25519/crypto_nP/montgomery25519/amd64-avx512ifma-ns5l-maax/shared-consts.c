// linker define hh1_p1
// linker define hh1_p2
// linker define hh1_p3
// linker define h2h_p1
// linker define h2h_p2
// linker define h2h_p3
// linker define hh1_xor
// linker define h2h_xor
// linker define swap_c
// linker define swap_mask
// linker define h2h_mask
// linker define vec19
// linker define vec608
// linker define vecmask52
// linker define vecmask47
// linker define a24
// linker define mask63

#include "consts_namespace.h"
#include "crypto_uint32.h"
#include "crypto_uint64.h"

const crypto_uint64 hh1_p1[]     = { 0x0,0x3FFFFFFFFFFFB401,0x3FFFFFFFFFFFB401,0x0 };
const crypto_uint64 hh1_p2[]     = { 0x0,0x3FFFFFFFFFFFFC01,0x3FFFFFFFFFFFFC01,0x0 };
const crypto_uint64 hh1_p3[]     = { 0x0,0x1FFFFFFFFFFFC01,0x1FFFFFFFFFFFC01,0x0 };
const crypto_uint64 h2h_p1[]     = { 0x0,0x3FFFFFFFFFFFB401,0x0,0x3FFFFFFFFFFFB401 };
const crypto_uint64 h2h_p2[]     = { 0x0,0x3FFFFFFFFFFFFC01,0x0,0x3FFFFFFFFFFFFC01 };
const crypto_uint64 h2h_p3[]     = { 0x0,0x1FFFFFFFFFFFC01,0x0,0x1FFFFFFFFFFFC01 };
const crypto_uint64 hh1_xor[]    = { 0,-1,-1,0 };
const crypto_uint64 h2h_xor[]    = { 0,-1,0,-1 };
const crypto_uint32 swap_c[]     = { 0,1,2,3,4,5,6,7 };
const crypto_uint32 swap_mask[]  = { 7,7,7,7,7,7,7,7 };
const crypto_uint64 h2h_mask[]   = { 0,-1,-1,-1 };
const crypto_uint64 vec19[]      = { 19,19,19,19 };
const crypto_uint64 vec608[]     = { 608,608,608,608 };
const crypto_uint64 vecmask52[]  = { 0xFFFFFFFFFFFFF,0xFFFFFFFFFFFFF,0xFFFFFFFFFFFFF,0xFFFFFFFFFFFFF };
const crypto_uint64 vecmask47[]  = { 0x7FFFFFFFFFFF,0x7FFFFFFFFFFF,0x7FFFFFFFFFFF,0x7FFFFFFFFFFF };
const crypto_uint64 a24[]  	  = { 0,121666,0,0 };
const crypto_uint64 mask63[] 	  = { 0x7FFFFFFFFFFFFFFF };
