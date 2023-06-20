// linker define ge_precomp_0
// linker use fe_0
// linker use fe_1

#include "ge.h"

void ge_precomp_0(ge_precomp *h)
{
  fe_1(h->yplusx);
  fe_1(h->yminusx);
  fe_0(h->xy2d);
}
