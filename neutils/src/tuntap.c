/*
 * Since the rust ioctl bindings don't have all the structures and constants,
 * it's easier to just write the thing in C and link it in.
 */



#ifdef __APPLE__
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <netinet/in.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/kern_control.h>
#include <net/if_utun.h>
#include <sys/ioctl.h>
#include <sys/kern_event.h>

int tuntap_setup(u_int32_t num) {
	int err;
    int fd;
    struct sockaddr_ctl addr;
    struct ctl_info info;

    fd = socket(PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL);
    if (fd < 0) {
        return fd;
    }
    memset(&info, 0, sizeof (info));
    strncpy(info.ctl_name, UTUN_CONTROL_NAME, strlen(UTUN_CONTROL_NAME));

    err = ioctl(fd, CTLIOCGINFO, &info);
    if (err < 0) {
        close(fd);
        return err;
    }

    addr.sc_id = info.ctl_id;
    addr.sc_len = sizeof(addr);
    addr.sc_family = AF_SYSTEM;
    addr.ss_sysaddr = AF_SYS_CONTROL;
    addr.sc_unit = num + 1; // utunX where X is sc.sc_unit -1

    err = connect(fd, (struct sockaddr*)&addr, sizeof(addr));
    if (err < 0) {
        // this utun is in use
        close(fd);
        return err;
    }
    return fd;
}
#else
#include <assert.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <linux/if.h>
#include <linux/if_tun.h>
#include <sys/ioctl.h>

/**
 * fd ‒ the fd to turn into TUN or TAP.
 * name ‒ the name to use. If empty, kernel will assign something by itself.
 *   Must be buffer with capacity at least 33.
 * mode ‒ 1 = TUN, 2 = TAP.
 * packet_info ‒ if packet info should be provided, if the given value is 0 it will not prepend packet info.
 */
int tuntap_setup(int fd, unsigned char *name, int mode, int packet_info) {
	struct ifreq ifr;
	memset(&ifr, 0, sizeof ifr);
	switch (mode) {
		case 1:
			ifr.ifr_flags = IFF_TUN;
			break;
		case 2:
			ifr.ifr_flags = IFF_TAP;
			break;
		default:
			assert(0);
	}

	// If no packet info needs to be provided add corresponding flag
	if (!packet_info) {
		ifr.ifr_flags |= IFF_NO_PI;
	}

	// Leave one for terminating '\0'. No idea if it is needed, didn't find
	// it in the docs, but assuming the worst.
	strncpy(ifr.ifr_name, (char *)name, IFNAMSIZ - 1);

	int ioresult = ioctl(fd, TUNSETIFF, &ifr);
	if (ioresult < 0) {
		return ioresult;
	}
	strncpy((char *)name, ifr.ifr_name, IFNAMSIZ < 32 ? IFNAMSIZ : 32);
	name[32] = '\0';
	return 0;
}
#endif