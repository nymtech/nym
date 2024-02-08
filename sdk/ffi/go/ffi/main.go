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

	str, err := bindings.GetSelfAddress()
	if err != nil {
		fmt.Println("Error:", err)
		return
	}
	fmt.Println("response from selfaddr:")
	fmt.Println("String:", str)

	time.Sleep(30 * time.Second)
}
