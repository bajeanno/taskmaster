#include <stdbool.h>
#include <sys/wait.h>

bool wifexited(int status) { return WIFEXITED(status); }

int wexitstatus(int status) { return WEXITSTATUS(status); }
