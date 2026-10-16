#define _GNU_SOURCE
#include <dlfcn.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <string.h>
#include <errno.h>

int bind(int fd, const struct sockaddr *address, socklen_t length) {
    static int (*real_bind)(int,const struct sockaddr *,socklen_t);
    if (!real_bind) real_bind=dlsym(RTLD_NEXT,"bind");
    if (!real_bind) { errno=ENOSYS; return -1; }
    if (address && address->sa_family==AF_INET && length>=sizeof(struct sockaddr_in)) {
        struct sockaddr_in local; memcpy(&local,address,sizeof(local));
        if (local.sin_addr.s_addr==htonl(INADDR_ANY)) local.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
        if (local.sin_addr.s_addr!=htonl(INADDR_LOOPBACK)) {errno=EACCES;return -1;}
        return real_bind(fd,(const struct sockaddr *)&local,sizeof(local));
    }
    if (address && address->sa_family==AF_INET6 && length>=sizeof(struct sockaddr_in6)) {
        struct sockaddr_in6 local; memcpy(&local,address,sizeof(local));
        if (IN6_IS_ADDR_UNSPECIFIED(&local.sin6_addr)) local.sin6_addr=in6addr_loopback;
        if (!IN6_IS_ADDR_LOOPBACK(&local.sin6_addr)) {errno=EACCES;return -1;}
        return real_bind(fd,(const struct sockaddr *)&local,sizeof(local));
    }
    return real_bind(fd,address,length);
}
