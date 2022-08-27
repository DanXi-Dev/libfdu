package main

import (
	"fmt"
	"math"
	"testing"
)

func TestHello(t *testing.T) {
	if hello() != "hello world" {
		t.Fail()
	}
}

func TestMemoryLeak(t *testing.T) {
	start := getMemoryUsage()
	for i := 0; i < 10000000; i++ {
		hello()
	}
	end := getMemoryUsage()
	usage := int(math.Abs(float64(end - start)))
	fmt.Printf("Memory usage: %d bytes", usage)
}
