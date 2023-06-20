#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <linux/perf_event.h>

static struct perf_event_attr attr;
static int fdperf = -1;
static struct perf_event_mmap_page *buf = 0;

long long ticks_setup(void)
{
  if (fdperf == -1) {
    attr.type = PERF_TYPE_HARDWARE;
    attr.config = PERF_COUNT_HW_CPU_CYCLES;
    attr.exclude_kernel = 1;
    fdperf = syscall(__NR_perf_event_open,&attr,0,-1,-1,0);
    if (fdperf == -1) return 0;
    buf = mmap(NULL,sysconf(_SC_PAGESIZE),PROT_READ,MAP_SHARED,fdperf,0);
  }
  return -1;
}

long long ticks(void)
{
  long long result;
  unsigned int seq;
  long long index;
  long long offset;

  do {
    seq = buf->lock;
    asm volatile("" ::: "memory");
    index = buf->index;
    offset = buf->offset;
    asm volatile("rdpmc;shlq $32,%%rdx;orq %%rdx,%%rax"
      : "=a"(result) : "c"(index-1) : "%rdx");
    asm volatile("" ::: "memory");
  } while (buf->lock != seq);

  result += offset;
  result &= 0xffffffffffff;
  return result;
}
