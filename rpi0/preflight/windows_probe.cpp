#include <cstdint>
#include <cstdio>
#include <cstring>
int main(int argc,char**argv){
  if(argc!=2||std::strcmp(argv[1],"--self-test")||sizeof(void*)!=8)return 64;
  constexpr std::uint64_t value=0x5250493057494e36ULL;
  std::printf("RPI0_X86_64_WINDOWS_PREFLIGHT_PASS %llx\n",static_cast<unsigned long long>(value));
  return 0;
}
