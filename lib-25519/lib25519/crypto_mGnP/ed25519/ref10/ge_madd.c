// linker define ge_madd
// linker use fe_add
// linker use fe_sub
// linker use fe_mul

#include "ge.h"

/*
r = p + q
*/

void ge_madd(ge_p1p1 *r,const ge_p3 *p,const ge_precomp *q)
{
  fe t0;
#include "ge_madd.h"
}
