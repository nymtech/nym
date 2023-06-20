// linker define hh_p1
// linker define hh_p2
// linker define hh_p3
// linker define hh_xor1
// linker define hh_xor2
// linker define sub_p1
// linker define sub_p2
// linker define sub_p3
// linker define swap_c
// linker define swap_mask
// linker define vecmask29d
// linker define vec19
// linker define vec1216
// linker define vecmask23
// linker define vecmask29
// linker define vecmask32
// linker define mask51

#include "consts_namespace.h"
#include "crypto_uint32.h"
#include "crypto_uint64.h"

const crypto_uint32 hh_p1[]      = { 0x0,0x0,0x5FFFFFC8,0x5FFFFFFE,0x0,0x0,0x5FFFFFC8,0x5FFFFFFE };
const crypto_uint32 hh_p2[]      = { 0x0,0x0,0x5FFFFFFE,0x5FFFFFFE,0x0,0x0,0x5FFFFFFE,0x5FFFFFFE };
const crypto_uint32 hh_p3[]      = { 0x0,0x0,0x17FFFFE,0x0,0x0,0x0,0x17FFFFE,0x0 };
const crypto_uint32 hh_xor1[]    = { 0,0,-1,-1,0,0,-1,-1 };
const crypto_uint32 hh_xor2[]    = { 0,0,-1,0,0,0,-1,0 };
const crypto_uint32 sub_p1[]     = { 0x3FFFFFDA,0x3FFFFFFE,0x0,0x0,0x0,0x0,0x0,0x0 };
const crypto_uint32 sub_p2[]     = { 0x3FFFFFFE,0x3FFFFFFE,0x0,0x0,0x0,0x0,0x0,0x0 };
const crypto_uint32 sub_p3[]     = { 0xFFFFFE,0x0,0x0,0x0,0x0,0x0,0x0,0x0 };
const crypto_uint32 swap_c[]     = { 0,1,2,3,4,5,6,7 };
const crypto_uint32 swap_mask[]  = { 7,7,7,7,7,7,7,7 };
const crypto_uint32 vecmask29d[] = { 0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF };
const crypto_uint64 vec19[]      = { 19,19,19,19 };
const crypto_uint64 vec1216[]    = { 1216,1216,1216,1216 };
const crypto_uint64 vecmask23[]  = { 0x7FFFFF,0x7FFFFF,0x7FFFFF,0x7FFFFF };
const crypto_uint64 vecmask29[]  = { 0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF };
const crypto_uint64 vecmask32[]  = { 0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF };
const crypto_uint64 mask51[] 	   = { 0x7FFFFFFFFFFFF };
