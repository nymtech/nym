// linker define hh_p1
// linker define hh_p2
// linker define hh_p3
// linker define hh_xor
// linker define sub_p1
// linker define sub_p2
// linker define sub_p3
// linker define swap_c
// linker define swap_mask
// linker define sh2526
// linker define sh2625
// linker define vecmask2526
// linker define vecmask2625
// linker define vec19
// linker define vec38
// linker define vecmask25
// linker define vecmask26
// linker define vecmask32
// linker define mask51

#include "consts_namespace.h"
#include "crypto_uint32.h"
#include "crypto_uint64.h"

const crypto_uint32 hh_p1[]       = { 0x0,0x0,0x7FFFFDB,0x3FFFFFF,0x0,0x0,0x7FFFFDB,0x3FFFFFF };
const crypto_uint32 hh_p2[]       = { 0x0,0x0,0x3FFFFFF,0x7FFFFFF,0x0,0x0,0x3FFFFFF,0x7FFFFFF };
const crypto_uint32 hh_p3[]       = { 0x0,0x0,0x7FFFFFF,0x3FFFFFF,0x0,0x0,0x7FFFFFF,0x3FFFFFF };
const crypto_uint32 hh_xor[]      = { 0,0,-1,-1,0,0,-1,-1 };
const crypto_uint32 sub_p1[]      = { 0x7FFFFDA,0x3FFFFFE,0x0,0x0,0x0,0x0,0x0,0x0 };
const crypto_uint32 sub_p2[]      = { 0x3FFFFFE,0x7FFFFFE,0x0,0x0,0x0,0x0,0x0,0x0 };
const crypto_uint32 sub_p3[]      = { 0x7FFFFFE,0x3FFFFFE,0x0,0x0,0x0,0x0,0x0,0x0 };
const crypto_uint32 swap_c[]      = { 0,1,2,3,4,5,6,7 };
const crypto_uint32 swap_mask[]   = { 7,7,7,7,7,7,7,7 };
const crypto_uint32 sh2526[]      = { 25,26,25,26,25,26,25,26 };
const crypto_uint32 sh2625[]      = { 26,25,26,25,26,25,26,25 };
const crypto_uint32 vecmask2526[] = { 0x1FFFFFF,0x3FFFFFF,0x1FFFFFF,0x3FFFFFF,0x1FFFFFF,0x3FFFFFF,0x1FFFFFF,0x3FFFFFF };
const crypto_uint32 vecmask2625[] = { 0x3FFFFFF,0x1FFFFFF,0x3FFFFFF,0x1FFFFFF,0x3FFFFFF,0x1FFFFFF,0x3FFFFFF,0x1FFFFFF };
const crypto_uint64 vec19[]       = { 19,19,19,19 };
const crypto_uint64 vec38[]       = { 38,38,38,38 };
const crypto_uint64 vecmask25[]   = { 0x1FFFFFF,0x1FFFFFF,0x1FFFFFF,0x1FFFFFF };
const crypto_uint64 vecmask26[]   = { 0x3FFFFFF,0x3FFFFFF,0x3FFFFFF,0x3FFFFFF };
const crypto_uint64 vecmask32[]   = { 0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF,0xFFFFFFFF };
const crypto_uint64 mask51[] 	   = { 0x7FFFFFFFFFFFF };
