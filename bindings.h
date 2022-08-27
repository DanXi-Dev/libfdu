#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {

char *hello_world();

int add(int a, int b);

char *get_url(const char *url);

} // extern "C"
