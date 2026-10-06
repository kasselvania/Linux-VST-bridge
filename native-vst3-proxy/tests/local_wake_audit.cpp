// Exact test-only auditor exception for prepared local wakes. No DSP claim.
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <dlfcn.h>
#include <sys/eventfd.h>
#include <unistd.h>
namespace {
void require(bool value,const char* why){if(!value){std::fprintf(stderr,"FAIL: %s\n",why);std::exit(1);}}
}
int main(){
 auto begin=reinterpret_cast<void(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_begin"));
 auto end=reinterpret_cast<uint64_t(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_end"));
 auto wakes=reinterpret_cast<uint64_t(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_local_wakes"));
 require(begin&&end&&wakes,"auditor exports");
 int fd=eventfd(0,EFD_NONBLOCK|EFD_CLOEXEC);require(fd>=0,"prepare wake");
 uint64_t value=1;
 begin();auto result=write(fd,&value,sizeof(value));auto effects=end();
 require(result==8&&effects==0&&wakes()==1,"prepared local wake is attributed separately");
 value=2;
 begin();result=write(fd,&value,sizeof(value));effects=end();
 require(result==8&&effects==1&&wakes()==0,"unbounded/coalesced counter write is not the permitted callback primitive");
 int pipefd[2]{};require(pipe(pipefd)==0,"prepare ordinary pipe");value=1;
 begin();result=write(pipefd[1],&value,sizeof(value));effects=end();
 require(result==8&&effects==1&&wakes()==0,"ordinary descriptor writes remain forbidden");
 begin();auto late=eventfd(0,EFD_NONBLOCK|EFD_CLOEXEC);effects=end();
 require(late>=0&&effects==1,"eventfd creation inside callback remains forbidden");
 begin();result=write(late,&value,sizeof(value));effects=end();
 require(result==8&&effects==1&&wakes()==0,"callback-created descriptor cannot register a legitimate wake");
 require(close(fd)==0&&dup2(pipefd[1],fd)==fd,"reuse retired wake identity as an ordinary descriptor");
 begin();result=write(fd,&value,sizeof(value));effects=end();
 require(result==8&&effects==1&&wakes()==0,"closed/reused descriptor cannot retain wake authority");
 require(close(fd)==0&&close(late)==0&&close(pipefd[0])==0&&close(pipefd[1])==0,"retire descriptors");
 std::puts("Exact prepared local wake attribution PASS");
}
