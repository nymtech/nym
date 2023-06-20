#ifndef cpucycles_h
#define cpucycles_h

extern long long (*cpucycles)(void) __attribute__((visibility("default")));;
extern long long cpucycles_init(void) __attribute__((visibility("default")));;
extern const char *cpucycles_implementation(void) __attribute__((visibility("default")));;
extern long long cpucycles_persecond(void) __attribute__((visibility("default")));;

#endif
