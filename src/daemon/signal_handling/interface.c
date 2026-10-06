#include <signal.h>
#include <string.h>
#include <errno.h>

extern void on_signal(int);

static int register_sighandler(int signal, void(*handler)(int)) {
	struct sigaction action;

	memset(&action, 0, sizeof(struct sigaction));
	action.sa_handler = handler;
	action.sa_flags = SA_RESTART;
	return sigaction(signal, &action, NULL);
}

char *declare_sighandlers() {
	if (register_sighandler(SIGINT, on_signal) != 0
		|| register_sighandler(SIGHUP, on_signal) != 0) {
		return strerror(errno);
	}
	return 0;
}
