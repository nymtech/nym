// linker define ge_msub
// linker use fe_add fe_sub fe_mul

#include "ge.h"

/*
r = p - q
*/

void ge_msub(ge_p1p1 *r,const ge_p3 *p,const ge_precomp *q)
{
  fe t0;
#include "ge_msub.h"
}
