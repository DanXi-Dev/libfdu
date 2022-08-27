#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {

char *hello_world();

void free_string(char *s);

int add(int a, int b);

char *get_url(const char *url);

} // extern "C"
