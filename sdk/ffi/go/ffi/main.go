package main

import (
	"fmt"
	"nymffi/ffi/bindings"
	"time"
)

func main() {
	fmt.Println("hello")
	bindings.InitLogging()
	bindings.InitEphemeral()
	time.Sleep(30 * time.Second)
}
