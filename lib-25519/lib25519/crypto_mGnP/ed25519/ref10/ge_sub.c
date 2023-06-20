// linker define ge_sub
// linker use fe_add fe_sub fe_mul

#include "ge.h"

/*
r = p - q
*/

void ge_sub(ge_p1p1 *r,const ge_p3 *p,const ge_cached *q)
{
  fe t0;
#include "ge_sub.h"
}
