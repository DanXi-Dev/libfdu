package main

import "runtime"

var m runtime.MemStats

func getMemoryUsage() uint64 {
	runtime.ReadMemStats(&m)
	return m.Sys
}
