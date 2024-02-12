package main

import (
	"fmt"
	"nymffi/ffi/bindings"
	"time"
)

func main() {
	fmt.Println("hello")
	bindings.InitLogging()
	err := bindings.InitEphemeral()
	if err != nil {
		fmt.Println(err)
		return
	}

	str, err2 := bindings.GetSelfAddress()
	if err2 != nil {
		fmt.Println("Error:", err2)
		return
	}
	fmt.Println("response:")
	fmt.Println(str)

	err3 := bindings.SendMessage(str, "helloworld")
	if err3 != nil {
		fmt.Println("Error:", err3)
		return
	}

	fmt.Println("end go")
	time.Sleep(30 * time.Second)
}
