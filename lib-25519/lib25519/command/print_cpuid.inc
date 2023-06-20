static void print_cpuid(void)
{
  unsigned int cpuid[32];
  lib25519_cpuid(cpuid,32);
  printf("cpuid");
  for (long long j = 0;j < 32;++j)
    printf(" %08x",cpuid[j]);
  printf("\n");
}
