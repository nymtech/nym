// linker define vec19
// linker define vec1216
// linker define vecmask23
// linker define vecmask29
// linker define pmask1
// linker define pmask2
// linker define pmask3
// linker define pmask4
// linker define pmask5
// linker define pmask6
// linker define pmask7
// linker define pmask8
// linker define pmask9
// linker define pmask10
// linker define pmask11
// linker define pmask12
// linker define upmask1
// linker define upmask2
// linker define upmask3
// linker define upmask4
// linker define upmask5
// linker define upmask6
// linker define upmask7
// linker define upmask8
// linker define mask63
// linker define 121666
// linker define MU0
// linker define MU1
// linker define MU2
// linker define MU3
// linker define MU4
// linker define ORDER0
// linker define ORDER1
// linker define ORDER2
// linker define ORDER3
// linker define EC2D0
// linker define EC2D1
// linker define EC2D2
// linker define EC2D3
// linker define 38

#include "crypto_uint64.h"
#include "consts_namespace.h"

const crypto_uint64 vec19[]      = { 19,19,19,19 };
const crypto_uint64 vec1216[]      = { 1216,1216,1216,1216 };
const crypto_uint64 vecmask23[]  = { 0x7FFFFF,0x7FFFFF,0x7FFFFF,0x7FFFFF };
const crypto_uint64 vecmask29[]  = { 0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF,0x1FFFFFFF };
const crypto_uint64 pmask1[]     = { 0x000000001FFFFFFF,0x000000001FFFFFFF,0x000000001FFFFFFF,0x000000001FFFFFFF };
const crypto_uint64 pmask2[]     = { 0x03FFFFFFE0000000,0x03FFFFFFE0000000,0x03FFFFFFE0000000,0x03FFFFFFE0000000 };
const crypto_uint64 pmask3[]     = { 0xFC00000000000000,0xFC00000000000000,0xFC00000000000000,0xFC00000000000000 };
const crypto_uint64 pmask4[]     = { 0x00000000007FFFFF,0x00000000007FFFFF,0x00000000007FFFFF,0x00000000007FFFFF };
const crypto_uint64 pmask5[]     = { 0x000FFFFFFF800000,0x000FFFFFFF800000,0x000FFFFFFF800000,0x000FFFFFFF800000 };
const crypto_uint64 pmask6[]     = { 0xFFF0000000000000,0xFFF0000000000000,0xFFF0000000000000,0xFFF0000000000000 };
const crypto_uint64 pmask7[]     = { 0x000000000001FFFF,0x000000000001FFFF,0x000000000001FFFF,0x000000000001FFFF };
const crypto_uint64 pmask8[]     = { 0x00003FFFFFFE0000,0x00003FFFFFFE0000,0x00003FFFFFFE0000,0x00003FFFFFFE0000 };
const crypto_uint64 pmask9[]     = { 0xFFFFC00000000000,0xFFFFC00000000000,0xFFFFC00000000000,0xFFFFC00000000000 };
const crypto_uint64 pmask10[]    = { 0x00000000000007FF,0x00000000000007FF,0x00000000000007FF,0x00000000000007FF };
const crypto_uint64 pmask11[]    = { 0x000000FFFFFFF800,0x000000FFFFFFF800,0x000000FFFFFFF800,0x000000FFFFFFF800 };
const crypto_uint64 pmask12[]    = { 0xFFFFFF0000000000,0xFFFFFF0000000000,0xFFFFFF0000000000,0xFFFFFF0000000000 };
const crypto_uint64 upmask1[]    = { 0x000000001FFFFFFF,0x000000001FFFFFFF,0x000000001FFFFFFF,0x000000001FFFFFFF };
const crypto_uint64 upmask2[]    = { 0x000000000000003F,0x000000000000003F,0x000000000000003F,0x000000000000003F };
const crypto_uint64 upmask3[]    = { 0x0000000000000FFF,0x0000000000000FFF,0x0000000000000FFF,0x0000000000000FFF };
const crypto_uint64 upmask4[]    = { 0x000000000003FFFF,0x000000000003FFFF,0x000000000003FFFF,0x000000000003FFFF };
const crypto_uint64 upmask5[]    = { 0x0000000000FFFFFF,0x0000000000FFFFFF,0x0000000000FFFFFF,0x0000000000FFFFFF };
const crypto_uint64 upmask6[]    = { 0x000000001FFFFFC0,0x000000001FFFFFC0,0x000000001FFFFFC0,0x000000001FFFFFC0 };
const crypto_uint64 upmask7[]    = { 0x000000001FFFF000,0x000000001FFFF000,0x000000001FFFF000,0x000000001FFFF000 };
const crypto_uint64 upmask8[]    = { 0x000000001FFC0000,0x000000001FFC0000,0x000000001FFC0000,0x000000001FFC0000 };
const crypto_uint64 mask63[] 	  = { 0x7FFFFFFFFFFFFFFF };

const crypto_uint64 CRYPTO_SHARED_NAMESPACE(121666) = 121666;

const crypto_uint64 CRYPTO_SHARED_NAMESPACE(MU0) = 0xED9CE5A30A2C131B;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(MU1) = 0x2106215D086329A7;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(MU2) = 0xFFFFFFFFFFFFFFEB;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(MU3) = 0xFFFFFFFFFFFFFFFF;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(MU4) = 0x000000000000000F;

const crypto_uint64 CRYPTO_SHARED_NAMESPACE(ORDER0) = 0x5812631A5CF5D3ED;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(ORDER1) = 0x14DEF9DEA2F79CD6;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(ORDER2) = 0x0000000000000000;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(ORDER3) = 0x1000000000000000;

const crypto_uint64 CRYPTO_SHARED_NAMESPACE(EC2D0) = 0xEBD69B9426B2F146;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(EC2D1) = 0x00E0149A8283B156;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(EC2D2) = 0x198E80F2EEF3D130;
const crypto_uint64 CRYPTO_SHARED_NAMESPACE(EC2D3) = 0xA406D9DC56DFFCE7;

const crypto_uint64 CRYPTO_SHARED_NAMESPACE(38) = 38;
