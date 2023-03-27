// version 20230105
// public domain
// djb

#ifndef cpucycles_internal_h
#define cpucycles_internal_h

extern long long cpucycles_init(void);
extern long long cpucycles_microseconds(void);
extern int cpucycles_works(long long (*)(void));

// return values from ticks_setup():
#define cpucycles_SKIP (0)
#define cpucycles_CYCLECOUNTER (-1)
#define cpucycles_MAYBECYCLECOUNTER (-2)
#define cpucycles_FINDMULTIPLIER (-3)
#define cpucycles_EXTEND32 (-32)
// and positive values mean known ticks/second

#endif
