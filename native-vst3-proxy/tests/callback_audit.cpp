// Test-only LD_PRELOAD instrumentation. Enabled on the actual SDK host's audio
// thread only during process/setProcessing; transport-worker effects stay out.
#include <cstddef>
#include <cstdint>
#include <array>
#include <atomic>
#include <cstring>
#include <cstdio>
#include <dlfcn.h>
#include <fcntl.h>
#include <poll.h>
#include <pthread.h>
#include <stdarg.h>
#include <sys/select.h>
#include <sys/socket.h>
#include <sys/eventfd.h>
#include <time.h>
#include <unistd.h>
namespace {
thread_local bool enabled __attribute__((tls_model("initial-exec"))) = false;
thread_local uint64_t effects __attribute__((tls_model("initial-exec"))) = 0;
thread_local uint64_t local_wakes __attribute__((tls_model("initial-exec"))) = 0;
std::array<std::atomic<bool>,4096> prepared_wakes{};
void hit() {
  if (enabled)
    ++effects;
}
template <class T> T next(const char *name) {
  return reinterpret_cast<T>(dlsym(RTLD_NEXT, name));
}
} // namespace
extern "C" void ap3_audit_begin() {
  effects = 0;
  local_wakes = 0;
  enabled = true;
}
extern "C" uint64_t ap3_audit_end() {
  enabled = false;
  return effects;
}
extern "C" uint64_t ap3_audit_local_wakes() { return local_wakes; }
// glibc allocator entry points avoid recursive dlsym during process startup.
extern "C" void *__libc_malloc(size_t);
extern "C" void __libc_free(void *);
extern "C" void *__libc_calloc(size_t, size_t);
extern "C" void *__libc_realloc(void *, size_t);
extern "C" void *__libc_memalign(size_t, size_t);
extern "C" void *malloc(size_t n) {
  hit();
  return __libc_malloc(n);
}
extern "C" void free(void *p) {
  hit();
  __libc_free(p);
}
extern "C" void *calloc(size_t n, size_t s) {
  hit();
  return __libc_calloc(n, s);
}
extern "C" void *realloc(void *p, size_t n) {
  hit();
  return __libc_realloc(p, n);
}
extern "C" void *aligned_alloc(size_t a, size_t n) {
  hit();
  return __libc_memalign(a, n);
}
extern "C" int posix_memalign(void **p, size_t a, size_t n) {
  hit();
  if (a < sizeof(void *) || (a & (a - 1)))
    return 22;
  *p = __libc_memalign(a, n);
  return *p ? 0 : 12;
}
#define WRAP(ret, name, args, call)                                            \
  namespace {                                                                  \
  auto real_##name = next<ret(*) args>(#name);                                 \
  }                                                                            \
  extern "C" ret name args {                                                   \
    hit();                                                                     \
    return real_##name call;                                                   \
  }
WRAP(ssize_t, read, (int f, void *p, size_t n), (f, p, n))
WRAP(ssize_t, recv, (int f, void *p, size_t n, int v), (f, p, n, v))
WRAP(ssize_t, send, (int f, const void *p, size_t n, int v), (f, p, n, v))
WRAP(int, connect, (int f, const sockaddr *p, socklen_t n), (f, p, n))
WRAP(int, socket, (int a, int b, int c), (a, b, c))
WRAP(int, poll, (pollfd * p, nfds_t n, int t), (p, n, t))
WRAP(int, select, (int n, fd_set *a, fd_set *b, fd_set *c, timeval *t),
     (n, a, b, c, t))
WRAP(int, nanosleep, (const timespec *a, timespec *b), (a, b))
WRAP(int, clock_nanosleep, (clockid_t c, int f, const timespec *a, timespec *b),
     (c, f, a, b))
WRAP(int, pthread_mutex_lock, (pthread_mutex_t * p), (p))
WRAP(int, pthread_cond_wait, (pthread_cond_t * p, pthread_mutex_t *m), (p, m))
WRAP(int, pthread_cond_timedwait,
     (pthread_cond_t * p, pthread_mutex_t *m, const timespec *t), (p, m, t))
WRAP(int, pthread_join, (pthread_t t, void **r), (t, r))
WRAP(int, pthread_create,
     (pthread_t * t, const pthread_attr_t *a, void *(*f)(void *), void *p),
     (t, a, f, p))
namespace {
auto real_eventfd=next<int(*)(unsigned int,int)>("eventfd");
auto real_write=next<ssize_t(*)(int,const void*,size_t)>("write");
auto real_close=next<int(*)(int)>("close");
auto real_open = next<int (*)(const char *, int, ...)>("open");
auto real_open64 = next<int (*)(const char *, int, ...)>("open64");
auto real_vfprintf = next<int (*)(FILE *, const char *, va_list)>("vfprintf");
} // namespace
extern "C" int eventfd(unsigned int count,int flags) {
  hit();
  auto fd=real_eventfd(count,flags);
  if(fd>=0&&size_t(fd)<prepared_wakes.size())prepared_wakes[size_t(fd)].store(
      !enabled&&count==0&&(flags&(EFD_NONBLOCK|EFD_CLOEXEC))==(EFD_NONBLOCK|EFD_CLOEXEC),std::memory_order_release);
  return fd;
}
extern "C" ssize_t write(int fd,const void* bytes,size_t size) {
  uint64_t value=0;
  const bool prepared=fd>=0&&size_t(fd)<prepared_wakes.size()&&
      prepared_wakes[size_t(fd)].load(std::memory_order_acquire);
  if(prepared&&bytes&&size==sizeof(value))std::memcpy(&value,bytes,sizeof(value));
  if(prepared&&size==sizeof(value)&&value==1){if(enabled)++local_wakes;}
  else hit();
  return real_write(fd,bytes,size);
}
extern "C" int close(int fd) {
  hit();
  if(fd>=0&&size_t(fd)<prepared_wakes.size())prepared_wakes[size_t(fd)].store(false,std::memory_order_release);
  return real_close(fd);
}
extern "C" int open(const char *p, int f, ...) {
  hit();
  mode_t m = 0;
  if (f & O_CREAT) {
    va_list a;
    va_start(a, f);
    m = va_arg(a, int);
    va_end(a);
  }
  return real_open(p, f, m);
}
extern "C" int open64(const char *p, int f, ...) {
  hit();
  mode_t m = 0;
  if (f & O_CREAT) {
    va_list a;
    va_start(a, f);
    m = va_arg(a, int);
    va_end(a);
  }
  return real_open64(p, f, m);
}
extern "C" int fprintf(FILE *f, const char *p, ...) {
  hit();
  va_list a;
  va_start(a, p);
  int r = real_vfprintf(f, p, a);
  va_end(a);
  return r;
}
