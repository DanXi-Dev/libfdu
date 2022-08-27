package main

/*
#cgo CFLAGS: -I${SRCDIR}/../..
#cgo LDFLAGS: -L${SRCDIR}/../../target/debug -lfdu
#include "bindings.h"
*/
import "C"
import (
	"fmt"
)

// On Windows, .dll MUST be in the same directory as the executable.

func hello() string {
	ptr := C.hello_world()
	defer C.free_string(ptr)
	return C.GoString(ptr)
}

func main() {
	fmt.Println(hello())
}
