long long ticks_setup(void)
{
  return -2;
}

long long ticks(void)
{
  unsigned long long result;
  asm volatile(".byte 15;.byte 49;shlq $32,%%rdx;orq %%rdx,%%rax"
    : "=a"(result) :: "%rdx");
  return result;
}
