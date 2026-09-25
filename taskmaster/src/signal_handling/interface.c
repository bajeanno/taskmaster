#include <signal.h>
#include <string.h>

extern void on_signal(int);

int register_sighandler(int signal, void(*handler)(int)) {
	struct sigaction action;

	memset(&action, 0, sizeof(struct sigaction));
	action.sa_handler = handler;
	action.sa_flags = SA_RESTART;
	sigaction(signal, &action, NULL);
	return 0;
}

int declare_sighandlers() {
	int result;
	result = register_sighandler(SIGINT, on_signal);
	if (result != 0) {
		return result;
	}
	return register_sighandler(SIGHUP, on_signal);
}
