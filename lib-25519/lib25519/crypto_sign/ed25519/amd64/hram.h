#ifndef HRAM_H
#define HRAM_H

#define get_hram CRYPTO_NAMESPACE(get_hram)

extern void get_hram(unsigned char *hram, const unsigned char *sm, const unsigned char *pk, unsigned char *playground, unsigned long long smlen);

#endif
