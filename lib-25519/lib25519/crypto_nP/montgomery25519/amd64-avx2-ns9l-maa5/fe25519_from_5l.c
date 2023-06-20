// linker define fe25519_from_5l

#include "fe25519.h"

void fe25519_from_5l(fe25519 *r, const fe25519_5l *x) {

	r->l[0] = ((x->l[0] & 0x0007FFFFFFFFFFFF))       | ((x->l[1] & 0x0000000000001FFF) << 51);
	r->l[1] = ((x->l[1] & 0x0007FFFFFFFFE000) >> 13) | ((x->l[2] & 0x0000000003FFFFFF) << 38);
	r->l[2] = ((x->l[2] & 0x0007FFFFFC000000) >> 26) | ((x->l[3] & 0x0000007FFFFFFFFF) << 25);
	r->l[3] = ((x->l[3] & 0x0007FF8000000000) >> 39) | ((x->l[4] & 0x0007FFFFFFFFFFFF) << 12);  
}
