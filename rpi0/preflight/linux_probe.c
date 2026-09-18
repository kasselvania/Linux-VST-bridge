#include <stdint.h>
#include <stdio.h>
#include <string.h>
int main(int argc,char**argv){
  if(argc!=2||strcmp(argv[1],"--self-test")||sizeof(void*)!=8){return 64;}
  const uint64_t value=0x5250493050524f42ULL;
  printf("RPI0_X86_64_LINUX_PREFLIGHT_PASS %llx\n",(unsigned long long)value);
  return 0;
}
