#include <sys/wait.h>
#include <stdbool.h>

bool wifexited(int status) {
	return WIFEXITED(status);
}