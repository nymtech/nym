// version 20230115
// public domain
// djb

// 20230115 djb: cpucycles_version()
// 20230114 djb: improve punctuation

#ifndef cpucycles_h
#define cpucycles_h

#ifdef __cplusplus
extern "C" {
#endif

extern long long (*cpucycles)(void) __attribute__((visibility("default")));
extern const char *cpucycles_implementation(void) __attribute__((visibility("default")));
extern const char *cpucycles_version(void) __attribute__((visibility("default")));
extern long long cpucycles_persecond(void) __attribute__((visibility("default")));
extern void cpucycles_tracesetup(void) __attribute__((visibility("default")));

#ifdef __cplusplus
}
#endif

#endif
