#include <stdint.h>
#include <stdio.h>
#include <string.h>
#if !defined(__x86_64__)
#error "RPI0 Linux preflight probe must be compiled for x86-64"
#endif
int main(int argc,char**argv){
  if(argc!=2||strcmp(argv[1],"--self-test")||sizeof(void*)!=8){return 64;}
  const uint64_t value=0x5250493050524f42ULL;
  printf("RPI0_X86_64_LINUX_PREFLIGHT_PASS %llx\n",(unsigned long long)value);
  return 0;
}
