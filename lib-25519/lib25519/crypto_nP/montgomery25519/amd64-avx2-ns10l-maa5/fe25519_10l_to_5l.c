// linker define fe25519_10l_to_5l

#include "fe25519.h"

void fe25519_10l_to_5l(fe25519_5l *r, const fe25519_10l *x) {
  
        r->l[0] = ((x->l[0] & 0x0000000003FFFFFF)) | ((x->l[1] & 0x0000000001FFFFFF) << 26);
        r->l[1] = ((x->l[2] & 0x0000000003FFFFFF)) | ((x->l[3] & 0x0000000001FFFFFF) << 26);
        r->l[2] = ((x->l[4] & 0x0000000003FFFFFF)) | ((x->l[5] & 0x0000000001FFFFFF) << 26);
        r->l[3] = ((x->l[6] & 0x0000000003FFFFFF)) | ((x->l[7] & 0x0000000001FFFFFF) << 26);
        r->l[4] = ((x->l[8] & 0x0000000003FFFFFF)) | ((x->l[9] & 0x0000000001FFFFFF) << 26);
}
