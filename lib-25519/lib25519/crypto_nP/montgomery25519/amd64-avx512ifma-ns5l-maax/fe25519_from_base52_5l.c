// linker define fe25519_from_base52_5l

#include "fe25519.h"

void fe25519_from_base52_5l(fe25519 *r, const fe25519_base52_5l *x) {
  
	r->l[0] = ((x->l[0] & 0x000FFFFFFFFFFFFF))       | ((x->l[1] & 0x0000000000000FFF) << 52);
	r->l[1] = ((x->l[1] & 0x000FFFFFFFFFF000) >> 12) | ((x->l[2] & 0x0000000000FFFFFF) << 40);
	r->l[2] = ((x->l[2] & 0x000FFFFFFF000000) >> 24) | ((x->l[3] & 0x0000000FFFFFFFFF) << 28);
	r->l[3] = ((x->l[3] & 0x000FFFF000000000) >> 36) | ((x->l[4] & 0x0000FFFFFFFFFFFF) << 16);        
}
