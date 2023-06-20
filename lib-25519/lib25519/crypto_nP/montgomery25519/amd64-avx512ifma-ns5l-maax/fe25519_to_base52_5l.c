// linker define fe25519_to_base_52_5l

#include "fe25519.h"

void fe25519_to_base52_5l(fe25519_base52_5l *r, const fe25519 *x) {

	r->l[0] = ((x->l[0] & 0x000FFFFFFFFFFFFF));
	r->l[1] = ((x->l[0] & 0xFFF0000000000000) >> 52) | ((x->l[1] & 0x000000FFFFFFFFFF) << 12);
	r->l[2] = ((x->l[1] & 0xFFFFFF0000000000) >> 40) | ((x->l[2] & 0x000000000FFFFFFF) << 24);
	r->l[3] = ((x->l[2] & 0xFFFFFFFFF0000000) >> 28) | ((x->l[3] & 0x000000000000FFFF) << 36);
	r->l[4] = ((x->l[3] & 0x7FFFFFFFFFFF0000) >> 16);	
}
